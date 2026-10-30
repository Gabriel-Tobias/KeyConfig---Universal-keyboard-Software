//! Envio de teclas e texto ao Windows via `SendInput`.

use std::mem::size_of;
use std::sync::atomic::{AtomicU8, Ordering};

use keyconfig_core::{KeyCode, KeyOutput, Modifier, Mods};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
    KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC,
};

/// Marca as teclas que nós mesmos enviamos, para o hook não processá-las de novo.
pub use keyconfig_core::ipc::INJECT_TAG;

/// Os oito modificadores, um por bit nas máscaras abaixo: virtual-key, tecla e tipo.
const SIDES: [(i32, KeyCode, Modifier); 8] = [
    (0xA2, KeyCode::ControlLeft, Modifier::Ctrl),
    (0xA3, KeyCode::ControlRight, Modifier::Ctrl),
    (0xA0, KeyCode::ShiftLeft, Modifier::Shift),
    (0xA1, KeyCode::ShiftRight, Modifier::Shift),
    (0xA4, KeyCode::AltLeft, Modifier::Alt),
    (0xA5, KeyCode::AltRight, Modifier::Alt),
    (0x5B, KeyCode::MetaLeft, Modifier::Win),
    (0x5C, KeyCode::MetaRight, Modifier::Win),
];
/// Alt e Win: soltos sozinhos, abrem o menu da janela ou o Iniciar.
const MENU_SIDES: u8 = 0xF0;
/// Tecla sem uso que, enviada entre o aperto e a soltura de Alt/Win, impede o menu de abrir.
const MASK_VK: u16 = 0xE8;
const VK_SNAPSHOT: u8 = 0x2C;

/// Modificadores que o usuário segura de verdade (o hook atualiza a cada evento físico).
static PHYSICAL: AtomicU8 = AtomicU8::new(0);

pub fn track_physical(code: KeyCode, up: bool) {
    if let Some(i) = SIDES.iter().position(|s| s.1 == code) {
        if up {
            PHYSICAL.fetch_and(!(1 << i), Ordering::Relaxed);
        } else {
            PHYSICAL.fetch_or(1 << i, Ordering::Relaxed);
        }
    }
}

/// Modificadores ativos agora para o Windows (inclui os de teclas remapeadas).
pub fn mods_down() -> Mods {
    let mut mods = Mods::NONE;
    for (vk, _, m) in SIDES {
        if unsafe { GetAsyncKeyState(vk) } < 0 {
            mods.set(m, true);
        }
    }
    mods
}

/// Quais lados destes modificadores estão apertados agora.
pub fn held_sides(mods: Mods) -> u8 {
    SIDES
        .iter()
        .enumerate()
        .filter(|(_, &(vk, _, m))| mods.has(m) && unsafe { GetAsyncKeyState(vk) } < 0)
        .fold(0, |bits, (i, _)| bits | 1 << i)
}

/// Solta os modificadores do gatilho, para a ação não sair com eles (Shift + `?` etc.).
pub fn release(sides: u8) {
    let mut inputs = Vec::new();
    if sides & MENU_SIDES != 0 {
        inputs.extend(mask());
    }
    inputs.extend(keys_of(sides).map(|k| key(k, true)));
    send(&inputs);
}

/// Aperta de novo os modificadores soltos por [`release`] que o usuário ainda segura.
pub fn restore(sides: u8) {
    let back = sides & PHYSICAL.load(Ordering::Relaxed);
    let mut inputs: Vec<_> = keys_of(back).map(|k| key(k, false)).collect();
    if back & MENU_SIDES != 0 {
        inputs.extend(mask());
    }
    send(&inputs);
}

fn keys_of(sides: u8) -> impl Iterator<Item = KeyCode> {
    SIDES.iter().enumerate().filter(move |(i, _)| sides & 1 << i != 0).map(|(_, s)| s.1)
}

fn mask() -> [INPUT; 2] {
    [raw(MASK_VK, 0, 0), raw(MASK_VK, 0, KEYEVENTF_KEYUP)]
}

pub fn key(code: KeyCode, up: bool) -> INPUT {
    let (vk, scan, mut flags) = match code.output() {
        KeyOutput::Scan(s) => (0, s as u16, KEYEVENTF_SCANCODE),
        KeyOutput::Ext(s) => (0, s as u16, KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY),
        KeyOutput::Vk(v, ext) => {
            // O Windows devolve 0x54 (Alt+SysRq) para o PrintScreen, cujo scancode é E0 37.
            // Localmente vale o virtual-key, mas a Área de Trabalho Remota repassa o scancode.
            let scan = match v {
                VK_SNAPSHOT => 0x37,
                _ => unsafe { MapVirtualKeyW(v as u32, MAPVK_VK_TO_VSC) as u16 },
            };
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
