//! Envio de teclas e texto ao Windows via `SendInput`.

use std::mem::size_of;

use keyconfig_core::{KeyCode, KeyOutput};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE,
    MAPVK_VK_TO_VSC,
};

/// Marca as teclas que nós mesmos enviamos, para o hook não processá-las de novo ("KCFG").
pub const INJECT_TAG: usize = 0x4B43_4647;

pub fn key(code: KeyCode, up: bool) -> INPUT {
    let (vk, scan, mut flags) = match code.output() {
        KeyOutput::Scan(s) => (0, s as u16, KEYEVENTF_SCANCODE),
        KeyOutput::Ext(s) => (0, s as u16, KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY),
        KeyOutput::Vk(v, ext) => {
            let scan = unsafe { MapVirtualKeyW(v as u32, MAPVK_VK_TO_VSC) } as u16;
            (v as u16, scan, if ext { KEYEVENTF_EXTENDEDKEY } else { 0 })
        }
    };
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    raw(vk, scan, flags)
}

/// Adiciona a `out` os eventos que digitam `text`, sem depender do idioma do teclado.
pub fn text(text: &str, out: &mut Vec<INPUT>) {
    for c in text.chars() {
        // Caracteres de controle como Unicode não funcionam na maioria dos programas.
        let control = match c {
            '\n' => Some(KeyCode::Enter),
            '\t' => Some(KeyCode::Tab),
            '\r' => continue,
            _ => None,
        };
        if let Some(code) = control {
            out.extend([key(code, false), key(code, true)]);
            continue;
        }
        let mut units = [0u16; 2];
        for &unit in c.encode_utf16(&mut units).iter() {
            out.push(raw(0, unit, KEYEVENTF_UNICODE));
            out.push(raw(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
        }
    }
}

pub fn send(inputs: &[INPUT]) {
    if !inputs.is_empty() {
        unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32) };
    }
}

fn raw(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECT_TAG,
            },
        },
    }
}
