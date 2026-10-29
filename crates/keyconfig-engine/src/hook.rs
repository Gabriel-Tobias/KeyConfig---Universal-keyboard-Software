//! Hook global de teclado: intercepta cada tecla e aplica o perfil ativo.
//!
//! O callback roda na thread que instalou o hook, dentro do loop de mensagens,
//! então o estado fica num `thread_local` sem precisar de locks. Ele precisa ser
//! rápido: se demorar demais o Windows remove o hook sem avisar. Por isso
//! macros, textos e comandos vão para o [`worker`](crate::worker).

use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::Arc;

use keyconfig_core::{Action, KeyCode, KeyOutput, Profile};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_UP, WH_KEYBOARD_LL,
};

use crate::input::{self, INJECT_TAG};
use crate::worker;

#[derive(Clone)]
enum Held {
    Up,
    /// Foi pressionada sem mapeamento; a soltura também passa direto.
    Passed,
    /// Foi pressionada com este mapeamento; a soltura usa o mesmo, mesmo que o perfil mude.
    Mapped(Arc<Action>),
}

enum Decision {
    Pass,
    Block,
    Act(Arc<Action>, bool),
}

struct Engine {
    mappings: Vec<Option<Arc<Action>>>,
    held: Vec<Held>,
    by_scan: [Option<KeyCode>; 512],
    by_vk: [Option<KeyCode>; 256],
}

impl Engine {
    fn new() -> Self {
        let mut by_scan = [None; 512];
        let mut by_vk = [None; 256];
        for &key in KeyCode::ALL {
            match key.output() {
                KeyOutput::Scan(s) => by_scan[s as usize] = Some(key),
                KeyOutput::Ext(s) => by_scan[0x100 | s as usize] = Some(key),
                KeyOutput::Vk(v, _) => by_vk[v as usize] = Some(key),
            }
        }
        Engine {
            mappings: vec![None; KeyCode::ALL.len()],
            held: vec![Held::Up; KeyCode::ALL.len()],
            by_scan,
            by_vk,
        }
    }

    fn identify(&self, kb: &KBDLLHOOKSTRUCT) -> Option<KeyCode> {
        if let Some(key) = self.by_vk.get(kb.vkCode as usize).copied().flatten() {
            return Some(key);
        }
        // Scancodes acima de 0xFF são teclas "fantasma" (ex.: o Ctrl falso do AltGr).
        if kb.scanCode > 0xFF {
            return None;
        }
        let ext = if kb.flags & LLKHF_EXTENDED != 0 { 0x100 } else { 0 };
        self.by_scan[ext | kb.scanCode as usize]
    }

    fn decide(&mut self, kb: &KBDLLHOOKSTRUCT) -> Decision {
        let Some(key) = self.identify(kb) else {
            return Decision::Pass;
        };
        let up = kb.flags & LLKHF_UP != 0;
        let i = key.index();

        let action = if up {
            match std::mem::replace(&mut self.held[i], Held::Up) {
                Held::Passed => return Decision::Pass,
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
            Some(action) => Decision::Act(action, up),
        }
    }
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
    });
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
            let decision = ENGINE.with_borrow_mut(|engine| engine.decide(kb));
            match decision {
                Decision::Pass => {}
                Decision::Block => return 1,
                Decision::Act(action, up) => {
                    perform(action, up);
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn perform(action: Arc<Action>, up: bool) {
    match &*action {
        Action::Key { to } => input::send(&[input::key(*to, up)]),
        Action::Disabled => {}
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
