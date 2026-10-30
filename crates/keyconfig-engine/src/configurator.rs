//! Abre o configurador, ou traz para a frente o que já estiver aberto.

use std::path::Path;
use std::ptr::null_mut;

use keyconfig_core::{ipc, tr};
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, ASFW_ANY, SW_RESTORE,
};

use crate::tray::{self, Notice};

pub fn open() {
    // Ainda estamos tratando o clique no ícone, então o Windows deixa trazer janelas para a frente.
    if let Some(hwnd) = find_window() {
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd);
        }
        return;
    }

    // Repassa esse direito ao configurador, senão o Windows abre a janela dele atrás das outras.
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let path = std::env::current_exe().map(|exe| exe.with_file_name(ipc::CONFIG_EXE));
    let launched = path.as_ref().is_ok_and(|p| std::process::Command::new(p).spawn().is_ok());
    if !launched {
        tray::notify(
            Notice::Error,
            "KeyConfig",
            &format!(
                "{} ({})",
                tr(
                    "Não encontrei o configurador na pasta do KeyConfig",
                    "Could not find the configurator in the KeyConfig folder",
                    "No se encontró el configurador en la carpeta de KeyConfig",
                ),
                ipc::CONFIG_EXE
            ),
        );
    }
}

/// Janela visível de um processo `keyconfig-config.exe`.
fn find_window() -> Option<HWND> {
    let mut found: HWND = null_mut();
    unsafe { EnumWindows(Some(check_window), &mut found as *mut HWND as LPARAM) };
    (!found.is_null()).then_some(found)
}

unsafe extern "system" fn check_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        return 1;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) != 0;
    CloseHandle(process);
    if !ok {
        return 1;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let is_configurator = Path::new(&path)
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(ipc::CONFIG_EXE));
    if is_configurator {
        *(lparam as *mut HWND) = hwnd;
        return 0; // Achou: para de procurar.
    }
    1
}
