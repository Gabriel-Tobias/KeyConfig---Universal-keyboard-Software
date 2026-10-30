//! Motor do KeyConfig: fica na bandeja aplicando o perfil ativo a todas as teclas.

// Em debug mantém o console para ver os logs; em release não abre janela nenhuma.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod elevation;
mod hook;
mod input;
mod tray;
mod util;
mod worker;

use std::cell::{Cell, RefCell};
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use keyconfig_core::{ipc, store};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, POINT,
    WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, AppendMenuW, ChangeWindowMessageFilterEx, ASFW_ANY, CreatePopupMenu, MSGFLT_ALLOW, MF_DEFAULT, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, MF_CHECKED,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, SW_SHOWNORMAL, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};

use crate::tray::Notice;
use crate::util::{message_box, wide};

const APP: &str = "KeyConfig";

const ID_RELOAD: usize = 1;
const ID_OPEN_FOLDER: usize = 2;
const ID_AUTOSTART: usize = 3;
const ID_RUN_AS_ADMIN: usize = 4;
const ID_EXIT: usize = 5;
const ID_CONFIGURATOR: usize = 6;

thread_local! {
    static ELEVATED: Cell<bool> = const { Cell::new(false) };
    static PROFILE_NAME: RefCell<String> = const { RefCell::new(String::new()) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static RELOAD_REQUEST: Cell<u32> = const { Cell::new(0) };
}

fn main() {
    let relaunched = std::env::args().any(|a| a == elevation::RELAUNCH_ARG);
    let Some(_instance) = single_instance(relaunched) else {
        message_box(APP, "O KeyConfig já está rodando. Procure o ícone na bandeja do sistema.", false);
        return;
    };

    ELEVATED.set(elevation::current_is_elevated());

    let hwnd = match create_window() {
        Ok(hwnd) => hwnd,
        Err(e) => {
            message_box(APP, &format!("Não foi possível iniciar: {e}"), true);
            return;
        }
    };
    tray::create(hwnd, APP);
    worker::start(hwnd);
    reload_profile(false);

    if let Err(e) = hook::install() {
        message_box(APP, &format!("Não foi possível instalar o hook de teclado: {e}"), true);
        return;
    }
    elevation::watch_foreground(ELEVATED.get());

    unsafe {
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    hook::uninstall();
}

/// Garante uma instância só. Ao reabrir como admin, espera a instância antiga fechar.
fn single_instance(relaunched: bool) -> Option<HANDLE> {
    let name = wide(r"Local\KeyConfigEngine");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        let err = unsafe { GetLastError() };
        if !handle.is_null() {
            if err != ERROR_ALREADY_EXISTS {
                return Some(handle);
            }
            unsafe { CloseHandle(handle) };
        }
        // Handle nulo (ex.: ACCESS_DENIED, mutex de uma instância elevada) também conta como ocupado.
        if !relaunched || Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn create_window() -> Result<HWND, String> {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = wide(ipc::ENGINE_WINDOW_CLASS);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&wc);
        TASKBAR_CREATED.set(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()));

        // Janela invisível comum (não message-only) para receber o aviso "TaskbarCreated".
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide(APP).as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }

        // Elevado, o Windows bloqueia mensagens vindas do configurador comum; liberamos só esta.
        let reload = RegisterWindowMessageW(wide(ipc::RELOAD_MESSAGE).as_ptr());
        RELOAD_REQUEST.set(reload);
        ChangeWindowMessageFilterEx(hwnd, reload, MSGFLT_ALLOW, null_mut());
        Ok(hwnd)
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        tray::WM_TRAY => {
            match (lparam & 0xFFFF) as u32 {
                WM_LBUTTONUP => open_configurator(),
                WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(hwnd),
                _ => {}
            }
            0
        }
        _ if msg != 0 && msg == RELOAD_REQUEST.get() => {
            reload_profile(false);
            0
        }
        worker::WM_WORKER_ERROR => {
            tray::notify(Notice::Error, "Erro ao executar ação", &worker::take_error());
            0
        }
        WM_DESTROY => {
            tray::remove();
            PostQuitMessage(0);
            0
        }
        _ if msg != 0 && msg == TASKBAR_CREATED.get() => {
            tray::show();
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn reload_profile(manual: bool) {
    match store::load_active() {
        Ok(loaded) => {
            hook::set_profile(&loaded.profile);
            let name = loaded.profile.name.clone();
            tray::set_tip(&format!("{APP} — perfil: {name}"));
            if loaded.created_defaults {
                tray::notify(
                    Notice::Info,
                    "KeyConfig está ativo",
                    "Clique com o botão direito no ícone para ver as opções e abrir a pasta dos perfis.",
                );
            } else if manual {
                tray::notify(Notice::Info, "Perfil recarregado", &format!("Perfil ativo: {name}"));
            }
            PROFILE_NAME.set(name);
        }
        // Mantém os mapeamentos anteriores para o teclado não ficar num estado estranho.
        Err(e) => tray::notify(Notice::Error, "Erro ao carregar o perfil", &e.to_string()),
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let elevated = ELEVATED.get();
    let autostart = autostart::status();
    let autostart_on = if elevated {
        autostart == autostart::Status::Admin
    } else {
        autostart != autostart::Status::Off
    };
    let autostart_label = if elevated {
        "Iniciar com o Windows (como administrador)"
    } else {
        "Iniciar com o Windows"
    };
    let profile = PROFILE_NAME.with_borrow(|n| format!("Perfil: {n}"));

    let menu = CreatePopupMenu();
    let item = |flags, id, text: &str| {
        AppendMenuW(menu, flags, id, wide(text).as_ptr());
    };
    item(MF_STRING | MF_DEFAULT, ID_CONFIGURATOR, "Abrir configurador");
    item(MF_STRING | MF_GRAYED, 0, &profile);
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    item(MF_STRING, ID_RELOAD, "Recarregar perfil");
    item(MF_STRING, ID_OPEN_FOLDER, "Abrir pasta de configuração");
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    item(
        MF_STRING | if autostart_on { MF_CHECKED } else { 0 },
        ID_AUTOSTART,
        autostart_label,
    );
    if !elevated {
        item(MF_STRING, ID_RUN_AS_ADMIN, "Executar como administrador");
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    item(MF_STRING, ID_EXIT, "Sair");

    // Sem isso o menu não fecha ao clicar fora dele.
    SetForegroundWindow(hwnd);
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
        pt.x,
        pt.y,
        0,
        hwnd,
        null(),
    ) as usize;
    DestroyMenu(menu);
    PostMessageW(hwnd, WM_NULL, 0, 0);

    match cmd {
        ID_CONFIGURATOR => open_configurator(),
        ID_RELOAD => reload_profile(true),
        ID_OPEN_FOLDER => open_config_folder(),
        ID_AUTOSTART => toggle_autostart(autostart_on, elevated),
        ID_RUN_AS_ADMIN => {
            if elevation::relaunch_as_admin() {
                DestroyWindow(hwnd);
            }
        }
        ID_EXIT => {
            DestroyWindow(hwnd);
        }
        _ => {}
    }
}

fn toggle_autostart(currently_on: bool, elevated: bool) {
    let result = if currently_on {
        autostart::disable()
    } else {
        autostart::enable(elevated)
    };
    match result {
        Ok(()) if currently_on => {
            tray::notify(Notice::Info, APP, "O KeyConfig não vai mais iniciar com o Windows.")
        }
        Ok(()) if elevated => tray::notify(
            Notice::Info,
            APP,
            "O KeyConfig vai iniciar com o Windows como administrador, funcionando em todos os programas.",
        ),
        Ok(()) => tray::notify(
            Notice::Info,
            APP,
            "O KeyConfig vai iniciar com o Windows. Para funcionar em programas de administrador, \
             use \"Executar como administrador\" e ative esta opção de novo.",
        ),
        Err(e) => tray::notify(Notice::Error, "Não foi possível alterar a inicialização", &e),
    }
}

fn open_configurator() {
    // O clique no ícone dá ao motor o direito de trazer janelas para a frente; repassamos
    // ao configurador, senão o Windows abre a janela dele atrás das outras.
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let path = std::env::current_exe().map(|exe| exe.with_file_name(ipc::CONFIG_EXE));
    let launched = path.as_ref().is_ok_and(|p| std::process::Command::new(p).spawn().is_ok());
    if !launched {
        tray::notify(
            Notice::Error,
            APP,
            &format!("Não encontrei o configurador ({}) na pasta do KeyConfig.", ipc::CONFIG_EXE),
        );
    }
}

fn open_config_folder() {
    match store::config_dir() {
        Ok(dir) => unsafe {
            ShellExecuteW(
                null_mut(),
                wide("open").as_ptr(),
                wide(&dir.to_string_lossy()).as_ptr(),
                null(),
                null(),
                SW_SHOWNORMAL,
            );
        },
        Err(e) => tray::notify(Notice::Error, APP, &e.to_string()),
    }
}
