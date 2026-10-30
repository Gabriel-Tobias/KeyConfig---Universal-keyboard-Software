//! Ícone na bandeja do sistema e notificações (balões/toasts).

use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO,
    NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, LoadIconW, LoadImageW, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR,
    SM_CXSMICON, SM_CYSMICON, WM_APP,
};

use crate::util::copy_wide;

/// Mensagem que o Windows manda para a nossa janela quando clicam no ícone.
pub const WM_TRAY: u32 = WM_APP + 1;

pub enum Notice {
    Info,
    Warning,
    Error,
}

thread_local! {
    static DATA: RefCell<Option<NOTIFYICONDATAW>> = const { RefCell::new(None) };
}

pub fn create(hwnd: HWND, tip: &str) {
    let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = load_icon();
    copy_wide(&mut data.szTip, tip);
    DATA.set(Some(data));
    show();
}

/// Adiciona o ícone. Também é chamado quando o Explorer reinicia e some com ele.
pub fn show() {
    with_data(|data| {
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        unsafe { Shell_NotifyIconW(NIM_ADD, data) };
    });
}

pub fn set_tip(tip: &str) {
    with_data(|data| {
        data.uFlags = NIF_TIP;
        copy_wide(&mut data.szTip, tip);
        unsafe { Shell_NotifyIconW(NIM_MODIFY, data) };
    });
}

pub fn notify(kind: Notice, title: &str, text: &str) {
    with_data(|data| {
        data.uFlags = NIF_INFO;
        copy_wide(&mut data.szInfoTitle, title);
        copy_wide(&mut data.szInfo, text);
        data.dwInfoFlags = match kind {
            Notice::Info => NIIF_INFO,
            Notice::Warning => NIIF_WARNING,
            Notice::Error => NIIF_ERROR,
        };
        unsafe { Shell_NotifyIconW(NIM_MODIFY, data) };
    });
}

pub fn remove() {
    if let Some(data) = DATA.take() {
        unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
    }
}

/// Ícone do KeyConfig embutido no executável (recurso 1), no tamanho de ícone pequeno
/// do sistema; se faltar, usa o genérico do Windows.
fn load_icon() -> HICON {
    unsafe {
        let icon = LoadImageW(
            GetModuleHandleW(null()),
            1 as PCWSTR,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_DEFAULTCOLOR,
        );
        if icon.is_null() {
            LoadIconW(null_mut(), IDI_APPLICATION)
        } else {
            icon as HICON
        }
    }
}

fn with_data(f: impl FnOnce(&mut NOTIFYICONDATAW)) {
    DATA.with_borrow_mut(|data| {
        if let Some(data) = data {
            f(data);
        }
    });
}
