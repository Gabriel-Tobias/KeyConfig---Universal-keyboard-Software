//! Thread que executa macros, textos e comandos fora do hook.
//!
//! O hook não pode esperar (pausas de macro o fariam ser removido pelo Windows),
//! então ele só enfileira o trabalho aqui. A fila também garante que duas
//! macros nunca se misturem.

use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use keyconfig_core::{tr, Action, KeyCode, MacroStep};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Com::{
    CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, SW_SHOWNORMAL, WM_APP};

use crate::input;
use crate::util::wide;

/// Avisa a janela principal que um comando falhou; o texto fica em [`take_error`].
pub const WM_WORKER_ERROR: u32 = WM_APP + 2;

static QUEUE: OnceLock<Sender<Arc<Action>>> = OnceLock::new();
static LAST_ERROR: Mutex<String> = Mutex::new(String::new());

pub fn start(hwnd: HWND) {
    let (tx, rx) = mpsc::channel::<Arc<Action>>();
    if QUEUE.set(tx).is_err() {
        return;
    }
    // HWND não é Send; o valor só é usado para PostMessage, que aceita qualquer thread.
    let hwnd = hwnd as usize;
    std::thread::spawn(move || {
        // ShellExecute precisa de COM inicializado na thread.
        unsafe { CoInitializeEx(null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32) };
        for action in rx {
            if let Err(e) = run(&action) {
                *LAST_ERROR.lock().unwrap_or_else(|p| p.into_inner()) = e;
                unsafe { PostMessageW(hwnd as HWND, WM_WORKER_ERROR, 0, 0) };
            }
        }
    });
}

pub fn submit(action: Arc<Action>) {
    if let Some(queue) = QUEUE.get() {
        let _ = queue.send(action);
    }
}

pub fn take_error() -> String {
    std::mem::take(&mut *LAST_ERROR.lock().unwrap_or_else(|p| p.into_inner()))
}

fn run(action: &Action) -> Result<(), String> {
    match action {
        Action::Macro { steps } => run_macro(steps),
        Action::Text { text } => {
            let mut inputs = Vec::new();
            input::text(text, &mut inputs);
            input::send(&inputs);
        }
        Action::Run { command, args } => return run_command(command, args),
        Action::Key { .. } | Action::Disabled | Action::Shortcut { .. } | Action::DeadKey { .. } => {}
    }
    Ok(())
}

fn run_macro(steps: &[MacroStep]) {
    // Teclas que a macro apertou e não soltou; soltamos no final para nada ficar preso.
    let mut pressed: Vec<KeyCode> = Vec::new();
    for step in steps {
        match step {
            MacroStep::Tap(key) => input::send(&[input::key(*key, false), input::key(*key, true)]),
            MacroStep::Down(key) => {
                input::send(&[input::key(*key, false)]);
                if !pressed.contains(key) {
                    pressed.push(*key);
                }
            }
            MacroStep::Up(key) => {
                input::send(&[input::key(*key, true)]);
                pressed.retain(|k| k != key);
            }
            MacroStep::DelayMs(ms) => std::thread::sleep(Duration::from_millis(*ms as u64)),
            MacroStep::Text(text) => {
                let mut inputs = Vec::new();
                input::text(text, &mut inputs);
                input::send(&inputs);
            }
        }
    }
    let release: Vec<_> = pressed.iter().rev().map(|&k| input::key(k, true)).collect();
    input::send(&release);
}

fn run_command(command: &str, args: &str) -> Result<(), String> {
    let args = (!args.is_empty()).then(|| wide(args));
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            wide(command).as_ptr(),
            args.as_ref().map_or(null(), |a| a.as_ptr()),
            null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize > 32 {
        Ok(())
    } else {
        Err(format!("{} \"{command}\"", tr("não foi possível abrir", "could not open", "no se pudo abrir")))
    }
}
