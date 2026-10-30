//! Hook global de teclado: intercepta cada tecla e aplica o perfil ativo.
//!
//! O callback roda na thread que instalou o hook, dentro do loop de mensagens,
//! então o estado fica num `thread_local` sem precisar de locks. Ele precisa ser
//! rápido: se demorar demais o Windows remove o hook sem avisar. Por isso
//! macros, textos e comandos vão para o [`worker`](crate::worker).

use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::Arc;

use keyconfig_core::{accent, Action, KeyCode, KeyLookup, Profile};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetKeyState};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_UP, WH_KEYBOARD_LL,
};

use crate::input::{self, INJECT_TAG};
use crate::worker;

const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const VK_CAPITAL: i32 = 0x14;
const VK_SPACE: u32 = 0x20;

#[derive(Clone)]
enum Held {
    Up,
    /// Foi pressionada sem mapeamento; a soltura também passa direto.
    Passed,
    /// Foi pressionada com este mapeamento; a soltura usa o mesmo, mesmo que o perfil mude.
    Mapped(Arc<Action>),
    /// Foi consumida por um acento (`~` + `a`); a soltura é descartada.
    Swallowed,
}

enum Decision {
    Pass,
    Block,
    Act(Arc<Action>, bool),
    /// Bloqueia a tecla e digita este caractere no lugar.
    Type(char),
}

struct Outcome {
    /// Acento pendente que não combinou com esta tecla e precisa ser digitado antes dela.
    prefix: Option<char>,
    key: Option<KeyCode>,
    decision: Decision,
}

struct Engine {
    mappings: Vec<Option<Arc<Action>>>,
    held: Vec<Held>,
    lookup: KeyLookup,
    /// Acento de uma tecla morta esperando a próxima letra.
    pending_accent: Option<char>,
    /// Configurador em foco: as teclas passam sem remapear, para ele ver as físicas.
    paused: bool,
}

impl Engine {
    fn new() -> Self {
        Engine {
            mappings: vec![None; KeyCode::ALL.len()],
            held: vec![Held::Up; KeyCode::ALL.len()],
            lookup: KeyLookup::new(),
            pending_accent: None,
            paused: false,
        }
    }

    fn decide(&mut self, kb: &KBDLLHOOKSTRUCT) -> Outcome {
        let key = self
            .lookup
            .identify(kb.vkCode, kb.scanCode, kb.flags & LLKHF_EXTENDED != 0);
        let up = kb.flags & LLKHF_UP != 0;

        if self.paused {
            // Uma tecla mapeada apertada antes da pausa ainda precisa soltar o que enviou.
            let releasing = up
                && key.is_some_and(|k| matches!(self.held[k.index()], Held::Mapped(_) | Held::Swallowed));
            if !releasing {
                if let Some(k) = key {
                    self.held[k.index()] = if up { Held::Up } else { Held::Passed };
                }
                self.pending_accent = None;
                return Outcome { prefix: None, key, decision: Decision::Pass };
            }
        }

        // Repetição automática de uma tecla segurada não consome o acento.
        let repeat = key.is_some_and(|k| !matches!(self.held[k.index()], Held::Up));

        let mut prefix = None;
        if !up && !repeat && !is_modifier(kb.vkCode) {
            if let Some(accent) = self.pending_accent.take() {
                match combine(accent, kb.vkCode) {
                    Some(c) => {
                        if let Some(k) = key {
                            self.held[k.index()] = Held::Swallowed;
                        }
                        return Outcome { prefix: None, key, decision: Decision::Type(c) };
                    }
                    None => prefix = Some(accent),
                }
            }
        }

        let decision = match key {
            Some(k) => self.decide_key(k, up),
            None => Decision::Pass,
        };
        Outcome { prefix, key, decision }
    }

    fn decide_key(&mut self, key: KeyCode, up: bool) -> Decision {
        let i = key.index();
        let action = if up {
            match std::mem::replace(&mut self.held[i], Held::Up) {
                Held::Passed => return Decision::Pass,
                Held::Swallowed => return Decision::Block,
                Held::Mapped(action) => Some(action),
                Held::Up => self.mappings[i].clone(),
            }
        } else {
            let repeat = matches!(self.held[i], Held::Mapped(_));
            let action = self.mappings[i].clone();
            self.held[i] = action.clone().map_or(Held::Passed, Held::Mapped);
            // Segurar a tecla gera repetições; só a troca simples de tecla deve repeti-las.
            if repeat && action.as_deref().is_some_and(|a| !matches!(a, Action::Key { .. })) {
                return Decision::Block;
            }
            action
        };

        match action {
            None => Decision::Pass,
            Some(action) => {
                if let Action::DeadKey { accent } = *action {
                    if !up {
                        self.pending_accent = Some(accent);
                    }
                    return Decision::Block;
                }
                Decision::Act(action, up)
            }
        }
    }
}

fn is_modifier(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0x14 | 0x5B | 0x5C | 0xA0..=0xA5)
}

/// Combina o acento com a tecla pela letra que ela gera no idioma atual do Windows.
fn combine(accent: char, vk: u32) -> Option<char> {
    let down = |vk: i32| unsafe { GetAsyncKeyState(vk) } < 0;
    if vk == VK_SPACE {
        return Some(accent);
    }
    if down(VK_CONTROL) || down(VK_MENU) || !(0x41..=0x5A).contains(&vk) {
        return None;
    }
    let caps = unsafe { GetKeyState(VK_CAPITAL) } & 1 != 0;
    let letter = char::from_u32(vk)?;
    let base = if down(VK_SHIFT) != caps { letter } else { letter.to_ascii_lowercase() };
    accent::compose(accent, base)
}

thread_local! {
    static ENGINE: RefCell<Engine> = RefCell::new(Engine::new());
    static HOOK: RefCell<HHOOK> = const { RefCell::new(null_mut()) };
}

pub fn set_profile(profile: &Profile) {
    ENGINE.with_borrow_mut(|engine| {
        engine.mappings.iter_mut().for_each(|m| *m = None);
        for (&key, action) in &profile.mappings {
            engine.mappings[key.index()] = Some(Arc::new(action.clone()));
        }
        engine.pending_accent = None;
    });
}

pub fn set_paused(paused: bool) {
    ENGINE.with_borrow_mut(|engine| engine.paused = paused);
}

pub fn install() -> Result<(), String> {
    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(null()), 0)
    };
    if hook.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    HOOK.set(hook);
    Ok(())
}

pub fn uninstall() {
    let hook = HOOK.replace(null_mut());
    if !hook.is_null() {
        unsafe { UnhookWindowsHookEx(hook) };
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        // Teclas injetadas (por nós ou por outros programas) passam direto.
        if kb.flags & LLKHF_INJECTED == 0 && kb.dwExtraInfo != INJECT_TAG {
            // O empréstimo termina antes do SendInput, que pode reentrar neste callback.
            let outcome = ENGINE.with_borrow_mut(|engine| engine.decide(kb));
            if let Some(accent) = outcome.prefix {
                type_char(accent);
            }
            match outcome.decision {
                // Depois de digitar o acento, reenviamos a tecla para ela não chegar antes dele.
                Decision::Pass => {
                    if let (Some(_), Some(key)) = (outcome.prefix, outcome.key) {
                        input::send(&[input::key(key, false)]);
                        return 1;
                    }
                }
                Decision::Block => return 1,
                Decision::Act(action, up) => {
                    perform(action, up);
                    return 1;
                }
                Decision::Type(c) => {
                    type_char(c);
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn type_char(c: char) {
    let mut inputs = Vec::with_capacity(4);
    input::text(c.encode_utf8(&mut [0; 4]), &mut inputs);
    input::send(&inputs);
}

fn perform(action: Arc<Action>, up: bool) {
    match &*action {
        Action::Key { to } => input::send(&[input::key(*to, up)]),
        Action::Disabled | Action::DeadKey { .. } => {}
        // Aperta na ordem e solta na ordem inversa, como uma pessoa faria.
        Action::Shortcut { keys } if up => {
            let inputs: Vec<_> = keys.iter().rev().map(|&k| input::key(k, true)).collect();
            input::send(&inputs);
        }
        Action::Shortcut { keys } => {
            let inputs: Vec<_> = keys.iter().map(|&k| input::key(k, false)).collect();
            input::send(&inputs);
        }
        Action::Macro { .. } | Action::Text { .. } | Action::Run { .. } => {
            if !up {
                worker::submit(action);
            }
        }
    }
}
