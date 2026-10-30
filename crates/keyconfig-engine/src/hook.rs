//! Hook global de teclado: intercepta cada tecla e aplica o perfil ativo.
//!
//! O callback roda na thread que instalou o hook, dentro do loop de mensagens,
//! então o estado fica num `thread_local` sem precisar de locks. Ele precisa ser
//! rápido: se demorar demais o Windows remove o hook sem avisar. Por isso
//! macros, textos e comandos vão para o [`worker`](crate::worker).

use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::Arc;

use keyconfig_core::{accent, Action, KeyCode, KeyLookup, Mods, Profile, Trigger};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetKeyState};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, KillTimer, SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK,
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_UP, WH_KEYBOARD_LL,
};

use crate::input::{self, INJECT_TAG};
use crate::worker;

const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const VK_CAPITAL: i32 = 0x14;
const VK_SPACE: u32 = 0x20;

/// Quanto o primeiro toque espera pelo segundo antes de valer como toque simples.
const DOUBLE_TAP_MS: u32 = 300;

/// Ação em andamento numa tecla segurada.
#[derive(Clone)]
struct Active {
    action: Arc<Action>,
    /// Modificadores do gatilho que soltamos ao apertar e voltamos ao soltar.
    lifted: u8,
}

#[derive(Clone)]
enum Held {
    Up,
    /// Foi pressionada sem mapeamento; a soltura também passa direto.
    Passed,
    /// Foi pressionada com este mapeamento; a soltura usa o mesmo, mesmo que o perfil mude.
    Mapped(Active),
    /// Foi consumida por um acento (`~` + `a`); a soltura é descartada.
    Swallowed,
    /// Primeiro toque de um possível toque duplo, ainda sem decisão.
    Pending,
}

/// Uma ação para executar depois que o hook decide.
struct Fire {
    active: Active,
    up: bool,
    /// Modificadores do gatilho, para as ações que rodam no worker soltarem.
    combo: Mods,
}

enum Decision {
    Pass,
    Block,
    Act(Fire),
    /// Bloqueia a tecla e digita este caractere no lugar.
    Type(char),
}

struct Outcome {
    /// Toque duplo que não se completou: vale o toque simples, antes desta tecla.
    flushed: Vec<Fire>,
    /// Acento pendente que não combinou com esta tecla e precisa ser digitado antes dela.
    prefix: Option<char>,
    key: Option<KeyCode>,
    decision: Decision,
}

struct PendingTap {
    key: KeyCode,
    mods: Mods,
    released: bool,
    timer: usize,
}

struct Engine {
    mappings: Vec<Option<Arc<Action>>>,
    /// Ações com gatilho especial (modificadores e/ou toque duplo), por tecla.
    combos: Vec<Vec<(Trigger, Arc<Action>)>>,
    held: Vec<Held>,
    lookup: KeyLookup,
    /// Acento de uma tecla morta esperando a próxima letra.
    pending_accent: Option<char>,
    pending_tap: Option<PendingTap>,
    /// Configurador em foco: as teclas passam sem remapear, para ele ver as físicas.
    paused: bool,
}

impl Engine {
    fn new() -> Self {
        Engine {
            mappings: vec![None; KeyCode::ALL.len()],
            combos: vec![Vec::new(); KeyCode::ALL.len()],
            held: vec![Held::Up; KeyCode::ALL.len()],
            lookup: KeyLookup::new(),
            pending_accent: None,
            pending_tap: None,
            paused: false,
        }
    }

    fn decide(&mut self, kb: &KBDLLHOOKSTRUCT) -> Outcome {
        let key = self
            .lookup
            .identify(kb.vkCode, kb.scanCode, kb.flags & LLKHF_EXTENDED != 0);
        let up = kb.flags & LLKHF_UP != 0;
        if let Some(k) = key {
            input::track_physical(k, up);
        }

        let mut flushed = Vec::new();
        if self.paused {
            // Uma tecla mapeada apertada antes da pausa ainda precisa soltar o que enviou.
            let releasing = up
                && key.is_some_and(|k| matches!(self.held[k.index()], Held::Mapped(_) | Held::Swallowed | Held::Pending));
            if !releasing {
                self.flush_tap(&mut flushed);
                if let Some(k) = key {
                    self.held[k.index()] = if up { Held::Up } else { Held::Passed };
                }
                self.pending_accent = None;
                return Outcome { flushed, prefix: None, key, decision: Decision::Pass };
            }
        }

        // Repetição automática de uma tecla segurada não consome o acento.
        let repeat = key.is_some_and(|k| !matches!(self.held[k.index()], Held::Up));

        // O segundo toque do toque duplo é decidido antes de mexer no acento pendente.
        let second_tap = !up
            && self
                .pending_tap
                .as_ref()
                .is_some_and(|p| Some(p.key) == key && p.mods == input::mods_down());
        // Qualquer outra tecla apertada encerra a espera: vale o toque simples, antes dela.
        if !up && !repeat && !second_tap {
            self.flush_tap(&mut flushed);
        }

        let mut prefix = None;
        if !up && !repeat && !second_tap && !is_modifier(kb.vkCode) {
            if let Some(accent) = self.pending_accent.take() {
                match combine(accent, kb.vkCode) {
                    Some(c) => {
                        if let Some(k) = key {
                            self.held[k.index()] = Held::Swallowed;
                        }
                        return Outcome { flushed, prefix: None, key, decision: Decision::Type(c) };
                    }
                    None => prefix = Some(accent),
                }
            }
        }

        let decision = match key {
            Some(k) => self.decide_key(k, up),
            None => Decision::Pass,
        };
        Outcome { flushed, prefix, key, decision }
    }

    fn decide_key(&mut self, key: KeyCode, up: bool) -> Decision {
        let i = key.index();
        if up {
            return match std::mem::replace(&mut self.held[i], Held::Up) {
                Held::Passed => Decision::Pass,
                Held::Swallowed => Decision::Block,
                Held::Pending => {
                    if let Some(p) = &mut self.pending_tap {
                        p.released = true;
                    }
                    Decision::Block
                }
                Held::Mapped(active) => Decision::Act(Fire { active, up: true, combo: Mods::NONE }),
                Held::Up => match &self.mappings[i] {
                    Some(action) => self.act(Active { action: action.clone(), lifted: 0 }, true, Mods::NONE),
                    None => Decision::Pass,
                },
            };
        }

        match &self.held[i] {
            Held::Up => {}
            Held::Passed => return Decision::Pass,
            Held::Swallowed | Held::Pending => return Decision::Block,
            // Segurar a tecla gera repetições; só a troca simples de tecla deve repeti-las.
            Held::Mapped(active) => {
                return match *active.action {
                    Action::Key { .. } => {
                        let active = Active { action: active.action.clone(), lifted: 0 };
                        Decision::Act(Fire { active, up: false, combo: Mods::NONE })
                    }
                    _ => Decision::Block,
                };
            }
        }

        let mods = input::mods_down();
        let double = Trigger { mods, double_tap: true };
        if let Some(p) = self.pending_tap.take() {
            kill_timer(p.timer);
            if let Some(action) = self.combo(key, double) {
                return self.start(key, action, mods);
            }
        }
        if self.combo(key, double).is_some() {
            let timer = unsafe { SetTimer(null_mut(), 0, DOUBLE_TAP_MS, Some(tap_timeout)) };
            self.pending_tap = Some(PendingTap { key, mods, released: false, timer });
            self.held[i] = Held::Pending;
            return Decision::Block;
        }
        match self.single(key, mods) {
            Some((action, combo)) => self.start(key, action, combo),
            None => {
                self.held[i] = Held::Passed;
                Decision::Pass
            }
        }
    }

    fn combo(&self, key: KeyCode, trigger: Trigger) -> Option<Arc<Action>> {
        self.combos[key.index()].iter().find(|(t, _)| *t == trigger).map(|(_, a)| a.clone())
    }

    /// Ação do toque simples: o combo com estes modificadores ou, se não houver, o mapeamento comum.
    fn single(&self, key: KeyCode, mods: Mods) -> Option<(Arc<Action>, Mods)> {
        if !mods.is_empty() {
            if let Some(action) = self.combo(key, Trigger { mods, double_tap: false }) {
                return Some((action, mods));
            }
        }
        self.mappings[key.index()].clone().map(|a| (a, Mods::NONE))
    }

    /// Aperta a ação na tecla. `combo` são os modificadores do gatilho, que a ação não deve receber.
    fn start(&mut self, key: KeyCode, action: Arc<Action>, combo: Mods) -> Decision {
        let lifted = match *action {
            Action::Key { .. } | Action::Shortcut { .. } => input::held_sides(combo),
            _ => 0,
        };
        let active = Active { action, lifted };
        self.held[key.index()] = Held::Mapped(active.clone());
        self.act(active, false, combo)
    }

    fn act(&mut self, active: Active, up: bool, combo: Mods) -> Decision {
        if let Action::DeadKey { accent } = *active.action {
            if !up {
                self.pending_accent = Some(accent);
            }
            return Decision::Block;
        }
        Decision::Act(Fire { active, up, combo })
    }

    /// O toque duplo não veio: executa o toque simples da tecla que estava esperando.
    fn flush_tap(&mut self, out: &mut Vec<Fire>) {
        let Some(p) = self.pending_tap.take() else { return };
        kill_timer(p.timer);
        // Sem nada mapeado, a própria tecla é reenviada.
        let (action, combo) = self
            .single(p.key, p.mods)
            .unwrap_or_else(|| (Arc::new(Action::Key { to: p.key }), Mods::NONE));
        if let Decision::Act(fire) = self.start(p.key, action, combo) {
            out.push(fire);
        }
        if p.released {
            if let Held::Mapped(active) = std::mem::replace(&mut self.held[p.key.index()], Held::Up) {
                if let Decision::Act(fire) = self.act(active, true, Mods::NONE) {
                    out.push(fire);
                }
            }
        }
    }
}

fn kill_timer(timer: usize) {
    unsafe { KillTimer(null_mut(), timer) };
}

/// Passou o tempo do toque duplo sem o segundo toque.
unsafe extern "system" fn tap_timeout(_: HWND, _: u32, _: usize, _: u32) {
    let mut fires = Vec::new();
    ENGINE.with_borrow_mut(|engine| engine.flush_tap(&mut fires));
    fires.into_iter().for_each(perform);
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
    let mut fires = Vec::new();
    ENGINE.with_borrow_mut(|engine| {
        // A tecla esperando o segundo toque resolve com o perfil antigo.
        engine.flush_tap(&mut fires);
        engine.mappings.iter_mut().for_each(|m| *m = None);
        for (&key, action) in &profile.mappings {
            engine.mappings[key.index()] = Some(Arc::new(action.clone()));
        }
        engine.combos.iter_mut().for_each(Vec::clear);
        for combo in &profile.combos {
            engine.combos[combo.key.index()].push((combo.trigger(), Arc::new(combo.action.clone())));
        }
        engine.pending_accent = None;
    });
    fires.into_iter().for_each(perform);
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
            outcome.flushed.into_iter().for_each(perform);
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
                Decision::Act(fire) => {
                    perform(fire);
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

fn perform(fire: Fire) {
    let Fire { active: Active { action, lifted }, up, combo } = fire;
    if !up {
        input::release(lifted);
    }
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
                worker::submit(action, combo);
            }
        }
    }
    if up {
        input::restore(lifted);
    }
}
