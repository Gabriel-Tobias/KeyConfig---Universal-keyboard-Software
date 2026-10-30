//! Uma janela só: se o configurador já estiver aberto, a nova cópia traz a
//! antiga para a frente e fecha.

use std::path::Path;
use std::ptr::{null, null_mut};
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, BOOL, ERROR_ALREADY_EXISTS, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow,
    ShowWindow, SW_RESTORE,
};

/// `true` se esta é a única cópia aberta.
pub fn acquire() -> bool {
    let name: Vec<u16> = "Local\\KeyConfigConfigurator\0".encode_utf16().collect();
    let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
    // Handle nulo (ex.: a outra cópia roda como admin) também quer dizer que já existe uma.
    let first = !handle.is_null() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS;
    if !first && !handle.is_null() {
        unsafe { CloseHandle(handle) };
    }
    // Em caso de sucesso o mutex fica aberto até o processo fechar.
    first
}

/// Traz a janela da outra cópia para a frente. Espera um pouco, porque ela
/// pode ter acabado de abrir e ainda não ter janela visível.
pub fn activate_existing() {
    for _ in 0..40 {
        if let Some(hwnd) = find_other_window() {
            unsafe {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                SetForegroundWindow(hwnd);
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn find_other_window() -> Option<HWND> {
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
    if pid == GetCurrentProcessId() {
        return 1;
    }
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
    let same_app = std::env::current_exe().ok().and_then(|e| e.file_name().map(|n| n.to_owned())).is_some_and(|me| {
        Path::new(&path).file_name().is_some_and(|n| n.eq_ignore_ascii_case(&me))
    });
    if same_app {
        *(lparam as *mut HWND) = hwnd;
        return 0; // Achou: para de procurar.
    }
    1
}
