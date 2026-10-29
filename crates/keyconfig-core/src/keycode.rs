//! Teclas físicas, identificadas pela posição no teclado (padrão W3C `KeyboardEvent.code`).
//!
//! Usar a posição física em vez do caractere faz o remapeamento funcionar igual em
//! qualquer idioma: `KeyQ` é sempre a tecla à direita do Tab, seja ela "Q" ou "A".

use std::fmt;
use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Como a tecla é reconhecida e reenviada ao Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutput {
    /// Scancode simples (Set 1).
    Scan(u8),
    /// Scancode com prefixo E0 (tecla "estendida").
    Ext(u8),
    /// Teclas que o Windows só trata direito por virtual-key code; o `bool` diz se é estendida.
    Vk(u8, bool),
}

macro_rules! define_keys {
    ($($name:ident = $kind:ident($($arg:expr),*)),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum KeyCode { $($name),* }

        impl KeyCode {
            pub const ALL: &'static [KeyCode] = &[$(KeyCode::$name),*];

            pub fn name(self) -> &'static str {
                match self { $(KeyCode::$name => stringify!($name)),* }
            }

            pub fn output(self) -> KeyOutput {
                match self { $(KeyCode::$name => KeyOutput::$kind($($arg),*)),* }
            }
        }
    };
}

define_keys! {
    Escape = Scan(0x01),
    Digit1 = Scan(0x02), Digit2 = Scan(0x03), Digit3 = Scan(0x04), Digit4 = Scan(0x05),
    Digit5 = Scan(0x06), Digit6 = Scan(0x07), Digit7 = Scan(0x08), Digit8 = Scan(0x09),
    Digit9 = Scan(0x0A), Digit0 = Scan(0x0B),
    Minus = Scan(0x0C), Equal = Scan(0x0D), Backspace = Scan(0x0E), Tab = Scan(0x0F),
    KeyQ = Scan(0x10), KeyW = Scan(0x11), KeyE = Scan(0x12), KeyR = Scan(0x13), KeyT = Scan(0x14),
    KeyY = Scan(0x15), KeyU = Scan(0x16), KeyI = Scan(0x17), KeyO = Scan(0x18), KeyP = Scan(0x19),
    BracketLeft = Scan(0x1A), BracketRight = Scan(0x1B), Enter = Scan(0x1C), ControlLeft = Scan(0x1D),
    KeyA = Scan(0x1E), KeyS = Scan(0x1F), KeyD = Scan(0x20), KeyF = Scan(0x21), KeyG = Scan(0x22),
    KeyH = Scan(0x23), KeyJ = Scan(0x24), KeyK = Scan(0x25), KeyL = Scan(0x26),
    Semicolon = Scan(0x27), Quote = Scan(0x28), Backquote = Scan(0x29), ShiftLeft = Scan(0x2A),
    Backslash = Scan(0x2B),
    KeyZ = Scan(0x2C), KeyX = Scan(0x2D), KeyC = Scan(0x2E), KeyV = Scan(0x2F), KeyB = Scan(0x30),
    KeyN = Scan(0x31), KeyM = Scan(0x32),
    Comma = Scan(0x33), Period = Scan(0x34), Slash = Scan(0x35), ShiftRight = Scan(0x36),
    NumpadMultiply = Scan(0x37), AltLeft = Scan(0x38), Space = Scan(0x39), CapsLock = Scan(0x3A),
    F1 = Scan(0x3B), F2 = Scan(0x3C), F3 = Scan(0x3D), F4 = Scan(0x3E), F5 = Scan(0x3F),
    F6 = Scan(0x40), F7 = Scan(0x41), F8 = Scan(0x42), F9 = Scan(0x43), F10 = Scan(0x44),
    ScrollLock = Scan(0x46),
    Numpad7 = Scan(0x47), Numpad8 = Scan(0x48), Numpad9 = Scan(0x49), NumpadSubtract = Scan(0x4A),
    Numpad4 = Scan(0x4B), Numpad5 = Scan(0x4C), Numpad6 = Scan(0x4D), NumpadAdd = Scan(0x4E),
    Numpad1 = Scan(0x4F), Numpad2 = Scan(0x50), Numpad3 = Scan(0x51), Numpad0 = Scan(0x52),
    NumpadDecimal = Scan(0x53),
    // Tecla extra entre Shift esquerdo e Z (ABNT2/ISO).
    IntlBackslash = Scan(0x56),
    F11 = Scan(0x57), F12 = Scan(0x58),
    F13 = Scan(0x64), F14 = Scan(0x65), F15 = Scan(0x66), F16 = Scan(0x67), F17 = Scan(0x68),
    F18 = Scan(0x69), F19 = Scan(0x6A), F20 = Scan(0x6B), F21 = Scan(0x6C), F22 = Scan(0x6D),
    F23 = Scan(0x6E), F24 = Scan(0x76),
    // Tecla "/?" ao lado do Shift direito (ABNT2).
    IntlRo = Scan(0x73),
    // Ponto extra do teclado numérico (ABNT2).
    NumpadComma = Scan(0x7E),

    NumpadEnter = Ext(0x1C), ControlRight = Ext(0x1D), NumpadDivide = Ext(0x35), AltRight = Ext(0x38),
    Home = Ext(0x47), ArrowUp = Ext(0x48), PageUp = Ext(0x49), ArrowLeft = Ext(0x4B),
    ArrowRight = Ext(0x4D), End = Ext(0x4F), ArrowDown = Ext(0x50), PageDown = Ext(0x51),
    Insert = Ext(0x52), Delete = Ext(0x53), MetaLeft = Ext(0x5B), MetaRight = Ext(0x5C),
    ContextMenu = Ext(0x5D),

    PrintScreen = Vk(0x2C, true), Pause = Vk(0x13, false), NumLock = Vk(0x90, true),
    AudioVolumeMute = Vk(0xAD, true), AudioVolumeDown = Vk(0xAE, true), AudioVolumeUp = Vk(0xAF, true),
    MediaTrackNext = Vk(0xB0, true), MediaTrackPrevious = Vk(0xB1, true), MediaStop = Vk(0xB2, true),
    MediaPlayPause = Vk(0xB3, true),
}

impl KeyCode {
    /// Posição da tecla em `ALL`, útil para tabelas indexadas.
    pub fn index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey(pub String);

impl fmt::Display for UnknownKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tecla desconhecida: \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownKey {}

impl FromStr for KeyCode {
    type Err = UnknownKey;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        KeyCode::ALL
            .iter()
            .copied()
            .find(|k| k.name() == s)
            .ok_or_else(|| UnknownKey(s.to_owned()))
    }
}

impl Serialize for KeyCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for KeyCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_round_trip() {
        for &key in KeyCode::ALL {
            assert_eq!(key.name().parse::<KeyCode>(), Ok(key));
        }
    }

    #[test]
    fn outputs_are_unique() {
        let mut seen = HashSet::new();
        for &key in KeyCode::ALL {
            let id = match key.output() {
                KeyOutput::Scan(s) => (0, s),
                KeyOutput::Ext(s) => (1, s),
                KeyOutput::Vk(v, _) => (2, v),
            };
            assert!(seen.insert(id), "saída duplicada em {key}");
        }
    }

    #[test]
    fn index_matches_position() {
        for (i, &key) in KeyCode::ALL.iter().enumerate() {
            assert_eq!(key.index(), i);
        }
    }
}
