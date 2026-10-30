//! Mostra no desenho as teclas físicas apertadas enquanto o configurador está em foco.
//!
//! Lê as mensagens de teclado da própria janela, que trazem o scancode físico.
//! Não dá para usar um hook de teclado aqui: o Windows deixa de chamá-lo
//! justamente quando a janela do próprio processo está em foco. O motor pausa os
//! remapeamentos enquanto o configurador está em foco, então o que chega é
//! sempre a tecla física.

use std::sync::{Mutex, OnceLock};

use keyconfig_core::{ipc, KeyCode, KeyLookup};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageExtraInfo, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

struct State {
    held: Vec<KeyCode>,
    last: Option<KeyCode>,
    /// Aumenta a cada tecla apertada, para saber se houve uma nova desde um momento.
    presses: u64,
}

static STATE: Mutex<State> = Mutex::new(State { held: Vec::new(), last: None, presses: 0 });
static LOOKUP: OnceLock<KeyLookup> = OnceLock::new();

pub struct Snapshot {
    pub held: Vec<KeyCode>,
    pub last: Option<KeyCode>,
    pub presses: u64,
}

pub fn install(cc: &eframe::CreationContext<'_>) {
    LOOKUP.get_or_init(KeyLookup::new);
    let Some(hwnd) = window_handle(cc) else { return };
    unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0) };
}

fn window_handle(cc: &eframe::CreationContext<'_>) -> Option<HWND> {
    match cc.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as HWND),
        _ => None,
    }
}

pub fn snapshot() -> Snapshot {
    let state = STATE.lock().unwrap_or_else(|p| p.into_inner());
    Snapshot {
        held: state.held.clone(),
        last: state.last,
        presses: state.presses,
    }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
            if GetMessageExtraInfo() as usize != ipc::INJECT_TAG =>
        {
            let scan = ((lparam >> 16) & 0xFF) as u32;
            let extended = (lparam >> 24) & 1 != 0;
            let key = LOOKUP.get().and_then(|l| l.identify(wparam as u32, scan, extended));
            if let Some(key) = key {
                let mut state = STATE.lock().unwrap_or_else(|p| p.into_inner());
                if matches!(msg, WM_KEYUP | WM_SYSKEYUP) {
                    state.held.retain(|&k| k != key);
                } else if !state.held.contains(&key) {
                    state.held.push(key);
                    state.last = Some(key);
                    state.presses += 1;
                }
            }
        }
        // Ao perder o foco a soltura vai para outra janela; apaga tudo para nada ficar aceso.
        WM_KILLFOCUS => STATE.lock().unwrap_or_else(|p| p.into_inner()).held.clear(),
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}
