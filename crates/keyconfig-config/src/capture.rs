//! Mostra no desenho as teclas físicas apertadas enquanto o configurador está em foco.
//!
//! Usa um hook de teclado só de leitura: nunca bloqueia nem altera nada. Teclas
//! injetadas (como as que o motor envia ao remapear) são ignoradas, então o
//! desenho sempre mostra a tecla física, não o resultado do remapeamento.

use std::ptr::{null, null_mut};
use std::sync::{Mutex, OnceLock};

use eframe::egui;
use keyconfig_core::{KeyCode, KeyLookup};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetWindowThreadProcessId, SetWindowsHookExW, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_UP, WH_KEYBOARD_LL,
};

#[derive(Default)]
struct State {
    held: Vec<KeyCode>,
    last: Option<KeyCode>,
    /// Aumenta a cada tecla apertada, para saber se houve uma nova desde um momento.
    presses: u64,
}

static STATE: Mutex<State> = Mutex::new(State { held: Vec::new(), last: None, presses: 0 });
static LOOKUP: OnceLock<KeyLookup> = OnceLock::new();
static CTX: OnceLock<egui::Context> = OnceLock::new();

pub struct Snapshot {
    pub held: Vec<KeyCode>,
    pub last: Option<KeyCode>,
    pub presses: u64,
}

/// Instala o hook na thread atual, que precisa ter loop de mensagens (a do eframe tem).
pub fn install(ctx: &egui::Context) {
    let _ = CTX.set(ctx.clone());
    LOOKUP.get_or_init(KeyLookup::new);
    unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(null()), 0) };
}

pub fn snapshot() -> Snapshot {
    let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    Snapshot {
        held: state.held.clone(),
        last: state.last,
        presses: state.presses,
    }
}

fn we_are_focused() -> bool {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
        pid == GetCurrentProcessId()
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let key = LOOKUP
            .get()
            .and_then(|l| l.identify(kb.vkCode, kb.scanCode, kb.flags & LLKHF_EXTENDED != 0));
        if let (Some(key), true) = (key, kb.flags & LLKHF_INJECTED == 0) {
            let up = kb.flags & LLKHF_UP != 0;
            let mut changed = false;
            if let Ok(mut state) = STATE.lock() {
                // Soltar sempre limpa, mesmo fora de foco, para nada ficar "aceso" preso.
                if up {
                    let before = state.held.len();
                    state.held.retain(|&k| k != key);
                    changed = state.held.len() != before;
                } else if !state.held.contains(&key) && we_are_focused() {
                    state.held.push(key);
                    state.last = Some(key);
                    state.presses += 1;
                    changed = true;
                }
            }
            if changed {
                if let Some(ctx) = CTX.get() {
                    ctx.request_repaint();
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}
