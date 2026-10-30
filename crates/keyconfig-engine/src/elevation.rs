//! Detecção de privilégios de administrador.
//!
//! O Windows não deixa um processo comum mexer no teclado de janelas elevadas.
//! Quando o KeyConfig roda sem admin, avisamos o usuário na primeira vez que
//! cada programa elevado ganha o foco.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::mem::{size_of, zeroed};
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, HANDLE, HWND};
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, EVENT_SYSTEM_FOREGROUND, SW_SHOWNORMAL, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS,
};

use keyconfig_core::ipc;

use crate::hook;
use crate::tray::{self, Notice};
use crate::util::wide;

/// Argumento passado ao reabrir como admin, para a nova instância esperar a antiga fechar.
pub const RELAUNCH_ARG: &str = "--relaunched";

pub fn current_is_elevated() -> bool {
    unsafe { token_is_elevated(GetCurrentProcess()) }.unwrap_or(false)
}

/// Reabre o KeyConfig pedindo permissão de administrador (UAC).
/// Retorna `false` se o usuário cancelou.
pub fn relaunch_as_admin() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("runas").as_ptr(),
            wide(&exe.to_string_lossy()).as_ptr(),
            wide(RELAUNCH_ARG).as_ptr(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    result as isize > 32
}

thread_local! {
    static WARNED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static SELF_ELEVATED: Cell<bool> = const { Cell::new(false) };
}

/// Passa a observar a janela em foco: pausa os remapeamentos quando é o
/// configurador e, se o motor não for admin, avisa sobre programas elevados.
pub fn watch_foreground(self_elevated: bool) {
    SELF_ELEVATED.set(self_elevated);
    unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            null_mut(),
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
    }
}

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 {
        return;
    }
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        hook::set_paused(false);
        return;
    }
    let name = exe_name(process);
    let configurator = name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(ipc::CONFIG_EXE));
    hook::set_paused(configurator);
    let elevated = !SELF_ELEVATED.get() && process_is_elevated(process);
    CloseHandle(process);

    let Some(name) = name.filter(|_| elevated) else { return };
    if WARNED.with_borrow_mut(|warned| warned.insert(name.to_lowercase())) {
        tray::notify(
            Notice::Warning,
            "KeyConfig sem acesso a este programa",
            &format!(
                "\"{name}\" está rodando como administrador, então seus mapeamentos não funcionam nele. \
                 Clique com o botão direito no ícone do KeyConfig e escolha \"Executar como administrador\"."
            ),
        );
    }
}

unsafe fn process_is_elevated(process: HANDLE) -> bool {
    match token_is_elevated(process) {
        Some(elevated) => elevated,
        // Sem admin não conseguimos abrir o token de um processo elevado.
        None => GetLastError() == ERROR_ACCESS_DENIED,
    }
}

unsafe fn token_is_elevated(process: HANDLE) -> Option<bool> {
    let mut token: HANDLE = null_mut();
    if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
        return None;
    }
    let mut elevation: TOKEN_ELEVATION = zeroed();
    let mut len = 0;
    let ok = GetTokenInformation(
        token,
        TokenElevation,
        &mut elevation as *mut _ as *mut _,
        size_of::<TOKEN_ELEVATION>() as u32,
        &mut len,
    );
    CloseHandle(token);
    (ok != 0).then_some(elevation.TokenIsElevated != 0)
}

unsafe fn exe_name(process: HANDLE) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}
