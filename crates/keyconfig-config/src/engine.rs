//! Comunicação com o motor (`keyconfig.exe`).

use std::ptr::null;

use keyconfig_core::ipc;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, RegisterWindowMessageW};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn window() -> HWND {
    unsafe { FindWindowW(wide(ipc::ENGINE_WINDOW_CLASS).as_ptr(), null()) }
}

pub fn is_running() -> bool {
    !window().is_null()
}

/// Pede ao motor para reler o perfil ativo. Retorna `false` se ele não estiver rodando.
pub fn request_reload() -> bool {
    let hwnd = window();
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        let msg = RegisterWindowMessageW(wide(ipc::RELOAD_MESSAGE).as_ptr());
        PostMessageW(hwnd, msg, 0, 0) != 0
    }
}

/// Inicia o motor, que fica na mesma pasta do configurador.
pub fn launch() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let engine = exe.with_file_name(ipc::ENGINE_EXE);
    std::process::Command::new(&engine)
        .spawn()
        .map(drop)
        .map_err(|e| format!("{}: {e}", engine.display()))
}
