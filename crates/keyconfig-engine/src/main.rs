//! Motor do KeyConfig: fica na bandeja aplicando o perfil ativo a todas as teclas.

// Em debug mantém o console para ver os logs; em release não abre janela nenhuma.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod configurator;
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

use keyconfig_core::{i18n, ipc, store, tr, UiLang};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, POINT,
    WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, ChangeWindowMessageFilterEx, CreatePopupMenu, MSGFLT_ALLOW, MF_DEFAULT, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, KillTimer, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer,
    TrackPopupMenu, TranslateMessage, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG,
    SW_SHOWNORMAL, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_CONTEXTMENU, WM_DESTROY,
    WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
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

const TRAY_RETRY_TIMER: usize = 1;

thread_local! {
    static ELEVATED: Cell<bool> = const { Cell::new(false) };
    static PROFILE_NAME: RefCell<String> = const { RefCell::new(String::new()) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static RELOAD_REQUEST: Cell<u32> = const { Cell::new(0) };
    static SHOW_REQUEST: Cell<u32> = const { Cell::new(0) };
}

fn main() {
    let saved_language = store::Store::open()
        .and_then(|s| s.load_settings())
        .ok()
        .and_then(|(settings, _)| settings.ui_language);
    apply_language(saved_language);

    let relaunched = std::env::args().any(|a| a == elevation::RELAUNCH_ARG);
    let Some(_instance) = single_instance(relaunched) else {
        // A instância aberta mostra o ícone (pode ter falhado no logon) e avisa por balão.
        if ask_running_instance_to_show() {
            return;
        }
        message_box(APP, tr("O KeyConfig já está rodando. Procure o ícone na bandeja do sistema.", "KeyConfig is already running. Look for its icon in the system tray.", "KeyConfig ya se está ejecutando. Busca su icono en la bandeja del sistema."), false);
        return;
    };

    ELEVATED.set(elevation::current_is_elevated());

    let hwnd = match create_window() {
        Ok(hwnd) => hwnd,
        Err(e) => {
            message_box(APP, &format!("{}: {e}", tr("Não foi possível iniciar", "Could not start", "No se pudo iniciar")), true);
            return;
        }
    };
    if !tray::create(hwnd, APP) {
        unsafe { SetTimer(hwnd, TRAY_RETRY_TIMER, 2000, None) };
    }
    worker::start(hwnd);
    reload_profile(false);

    if let Err(e) = hook::install() {
        message_box(APP, &format!("{}: {e}", tr("Não foi possível instalar o hook de teclado", "Could not install the keyboard hook", "No se pudo instalar el hook de teclado")), true);
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

/// Avisa a instância que já roda para mostrar o ícone. `false` se ela não foi encontrada.
fn ask_running_instance_to_show() -> bool {
    unsafe {
        let hwnd = FindWindowW(wide(ipc::ENGINE_WINDOW_CLASS).as_ptr(), null());
        if hwnd.is_null() {
            return false;
        }
        let msg = RegisterWindowMessageW(wide(ipc::SHOW_MESSAGE).as_ptr());
        PostMessageW(hwnd, msg, 0, 0) != 0
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

        // Elevado, o Windows bloqueia mensagens vindas de processos comuns (configurador,
        // outra cópia do motor e o próprio Explorer); liberamos só estas.
        let reload = RegisterWindowMessageW(wide(ipc::RELOAD_MESSAGE).as_ptr());
        RELOAD_REQUEST.set(reload);
        let show = RegisterWindowMessageW(wide(ipc::SHOW_MESSAGE).as_ptr());
        SHOW_REQUEST.set(show);
        for msg in [reload, show, TASKBAR_CREATED.get()] {
            ChangeWindowMessageFilterEx(hwnd, msg, MSGFLT_ALLOW, null_mut());
        }
        Ok(hwnd)
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        tray::WM_TRAY => {
            match (lparam & 0xFFFF) as u32 {
                WM_LBUTTONUP => configurator::open(),
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
            tray::notify(Notice::Error, tr("Erro ao executar ação", "Error running action", "Error al ejecutar la acción"), &worker::take_error());
            0
        }
        WM_DESTROY => {
            tray::remove();
            PostQuitMessage(0);
            0
        }
        _ if msg != 0 && msg == TASKBAR_CREATED.get() => {
            if tray::show() {
                KillTimer(hwnd, TRAY_RETRY_TIMER);
            }
            0
        }
        _ if msg != 0 && msg == SHOW_REQUEST.get() => {
            if tray::show() {
                KillTimer(hwnd, TRAY_RETRY_TIMER);
            }
            tray::notify(
                Notice::Info,
                APP,
                tr("O KeyConfig já está rodando.", "KeyConfig is already running.", "KeyConfig ya se está ejecutando."),
            );
            0
        }
        WM_TIMER if wparam == TRAY_RETRY_TIMER => {
            // No logon a bandeja pode demorar a aceitar ícones; tenta até conseguir.
            if tray::show() {
                KillTimer(hwnd, TRAY_RETRY_TIMER);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Usa o idioma escolhido no configurador ou, se não houver, o do Windows.
fn apply_language(saved: Option<UiLang>) {
    let lang = saved.unwrap_or_else(|| {
        UiLang::from_windows_langid(unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() })
    });
    i18n::set(lang);
}

fn reload_profile(manual: bool) {
    match store::load_active() {
        Ok(loaded) => {
            // O configurador pede recarga também quando o idioma do app muda.
            apply_language(loaded.settings.ui_language);
            hook::set_profile(&loaded.profile);
            let name = loaded.profile.name.clone();
            tray::set_tip(&format!("{APP} — {}: {name}", tr("perfil", "profile", "perfil")));
            if loaded.created_defaults {
                tray::notify(
                    Notice::Info,
                    tr("KeyConfig está ativo", "KeyConfig is running", "KeyConfig está activo"),
                    tr(
                        "Clique com o botão direito no ícone para ver as opções e abrir a pasta dos perfis.",
                        "Right-click the icon to see the options and open the profiles folder.",
                        "Haz clic derecho en el icono para ver las opciones y abrir la carpeta de perfiles.",
                    ),
                );
            } else if manual {
                tray::notify(
                    Notice::Info,
                    tr("Perfil recarregado", "Profile reloaded", "Perfil recargado"),
                    &format!("{}: {name}", tr("Perfil ativo", "Active profile", "Perfil activo")),
                );
            }
            PROFILE_NAME.set(name);
        }
        // Mantém os mapeamentos anteriores para o teclado não ficar num estado estranho.
        Err(e) => tray::notify(Notice::Error, tr("Erro ao carregar o perfil", "Error loading the profile", "Error al cargar el perfil"), &e.to_string()),
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
        tr("Iniciar com o Windows (como administrador)", "Start with Windows (as administrator)", "Iniciar con Windows (como administrador)")
    } else {
        tr("Iniciar com o Windows", "Start with Windows", "Iniciar con Windows")
    };
    let profile = PROFILE_NAME.with_borrow(|n| format!("{}: {n}", tr("Perfil", "Profile", "Perfil")));

    let menu = CreatePopupMenu();
    let item = |flags, id, text: &str| {
        AppendMenuW(menu, flags, id, wide(text).as_ptr());
    };
    item(MF_STRING | MF_DEFAULT, ID_CONFIGURATOR, tr("Abrir configurador", "Open configurator", "Abrir configurador"));
    item(MF_STRING | MF_GRAYED, 0, &profile);
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    item(MF_STRING, ID_RELOAD, tr("Recarregar perfil", "Reload profile", "Recargar perfil"));
    item(MF_STRING, ID_OPEN_FOLDER, tr("Abrir pasta de configuração", "Open configuration folder", "Abrir carpeta de configuración"));
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    item(
        MF_STRING | if autostart_on { MF_CHECKED } else { 0 },
        ID_AUTOSTART,
        autostart_label,
    );
    if !elevated {
        item(MF_STRING, ID_RUN_AS_ADMIN, tr("Executar como administrador", "Run as administrator", "Ejecutar como administrador"));
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    item(MF_STRING, ID_EXIT, tr("Sair", "Quit", "Salir"));

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
        ID_CONFIGURATOR => configurator::open(),
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
            tray::notify(Notice::Info, APP, tr("O KeyConfig não vai mais iniciar com o Windows.", "KeyConfig will no longer start with Windows.", "KeyConfig ya no se iniciará con Windows."))
        }
        Ok(()) if elevated => tray::notify(
            Notice::Info,
            APP,
            tr(
                "O KeyConfig vai iniciar com o Windows como administrador, funcionando em todos os programas.",
                "KeyConfig will start with Windows as administrator, working in every program.",
                "KeyConfig se iniciará con Windows como administrador y funcionará en todos los programas.",
            ),
        ),
        Ok(()) => tray::notify(
            Notice::Info,
            APP,
            tr(
                "O KeyConfig vai iniciar com o Windows. Para funcionar em programas de administrador, \
                 use \"Executar como administrador\" e ative esta opção de novo.",
                "KeyConfig will start with Windows. To work in administrator programs, \
                 use \"Run as administrator\" and turn this option on again.",
                "KeyConfig se iniciará con Windows. Para que funcione en programas de administrador, \
                 usa \"Ejecutar como administrador\" y vuelve a activar esta opción.",
            ),
        ),
        Err(e) => tray::notify(Notice::Error, tr("Não foi possível alterar a inicialização", "Could not change the startup setting", "No se pudo cambiar el inicio"), &e),
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
