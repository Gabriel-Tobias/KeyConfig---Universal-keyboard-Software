//! Posição e rótulo de cada tecla, para desenhar o teclado no configurador.
//!
//! Medidas em "unidades" (1u = largura de uma tecla de letra).

use crate::{KeyCode, Language, Layout};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyRect {
    pub code: KeyCode,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Segundo retângulo para teclas em "L", como o Enter ISO/ABNT2.
    pub extra: Option<[f32; 4]>,
}

/// Teclas desenhadas para um formato e idioma.
pub fn keys(layout: Layout, language: Language) -> Vec<KeyRect> {
    let iso = language == Language::PtBrAbnt2;
    let mut b = Builder::default();
    match layout {
        Layout::Compact40 => compact40(&mut b, iso),
        Layout::Compact60 => {
            alpha_block(&mut b, iso, 0.0, Escape60::Yes, 2.75);
            bottom_row_full(&mut b, 4.0);
        }
        Layout::Compact65 => {
            alpha_block(&mut b, iso, 0.0, Escape60::Yes, 1.75);
            b.at(15.0, 0.0).key(KeyCode::Delete, 1.0);
            b.at(15.0, 1.0).key(KeyCode::PageUp, 1.0);
            b.at(15.0, 2.0).key(KeyCode::PageDown, 1.0);
            b.at(14.0, 3.0).key(KeyCode::ArrowUp, 1.0).key(KeyCode::End, 1.0);
            bottom_row_compact(&mut b, 4.0);
        }
        Layout::Compact75 => {
            compact_function_row(&mut b);
            b.gap(0.25).key(KeyCode::PrintScreen, 1.0).key(KeyCode::Delete, 1.0);
            alpha_block(&mut b, iso, 1.25, Escape60::No, 1.75);
            b.at(15.0, 1.25).key(KeyCode::Home, 1.0);
            b.at(15.0, 2.25).key(KeyCode::PageUp, 1.0);
            b.at(15.0, 3.25).key(KeyCode::PageDown, 1.0);
            b.at(14.0, 4.25).key(KeyCode::ArrowUp, 1.0).key(KeyCode::End, 1.0);
            bottom_row_compact(&mut b, 5.25);
        }
        Layout::Tenkeyless => tenkeyless(&mut b, iso),
        Layout::Compact96 => compact96(&mut b, iso),
        Layout::Full => {
            tenkeyless(&mut b, iso);
            numpad(&mut b, iso, 18.5, 1.25, true);
        }
    }
    b.keys
}

/// Rótulo curto impresso na tecla.
pub fn label(code: KeyCode, language: Language) -> &'static str {
    use KeyCode::*;
    let abnt2 = language == Language::PtBrAbnt2;
    match code {
        Backquote if abnt2 => "'",
        BracketLeft if abnt2 => "´",
        BracketRight if abnt2 => "[",
        Backslash if abnt2 => "]",
        Semicolon if abnt2 => "Ç",
        Quote if abnt2 => "~",
        Slash if abnt2 => ";",
        IntlRo if abnt2 => "/",
        NumpadDecimal if abnt2 => ",",
        NumpadComma if abnt2 => ".",
        AltRight if language == Language::EnUs => "Alt",
        Backquote => "`",
        BracketLeft => "[",
        BracketRight => "]",
        Backslash => "\\",
        Semicolon => ";",
        Quote => "'",
        Slash => "/",
        IntlRo => "Ro",
        NumpadDecimal => ".",
        NumpadComma => ",",
        AltRight => "AltGr",

        Escape => "Esc",
        Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4", Digit5 => "5",
        Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9", Digit0 => "0",
        Minus => "-", Equal => "=", Backspace => "Backspace", Tab => "Tab",
        KeyQ => "Q", KeyW => "W", KeyE => "E", KeyR => "R", KeyT => "T", KeyY => "Y",
        KeyU => "U", KeyI => "I", KeyO => "O", KeyP => "P", KeyA => "A", KeyS => "S",
        KeyD => "D", KeyF => "F", KeyG => "G", KeyH => "H", KeyJ => "J", KeyK => "K",
        KeyL => "L", KeyZ => "Z", KeyX => "X", KeyC => "C", KeyV => "V", KeyB => "B",
        KeyN => "N", KeyM => "M",
        Enter | NumpadEnter => "Enter",
        ControlLeft | ControlRight => "Ctrl",
        ShiftLeft | ShiftRight => "Shift",
        AltLeft => "Alt",
        MetaLeft | MetaRight => "Win",
        ContextMenu => "Menu",
        Comma => ",", Period => ".", Space => "",
        CapsLock => "Caps",
        F1 => "F1", F2 => "F2", F3 => "F3", F4 => "F4", F5 => "F5", F6 => "F6",
        F7 => "F7", F8 => "F8", F9 => "F9", F10 => "F10", F11 => "F11", F12 => "F12",
        F13 => "F13", F14 => "F14", F15 => "F15", F16 => "F16", F17 => "F17", F18 => "F18",
        F19 => "F19", F20 => "F20", F21 => "F21", F22 => "F22", F23 => "F23", F24 => "F24",
        PrintScreen => "PrtSc", ScrollLock => "ScrLk", Pause => "Pause",
        Insert => "Ins", Home => "Home", PageUp => "PgUp",
        Delete => "Del", End => "End", PageDown => "PgDn",
        ArrowUp => "↑", ArrowDown => "↓", ArrowLeft => "←", ArrowRight => "→",
        NumLock => "Num",
        NumpadDivide => "/", NumpadMultiply => "*", NumpadSubtract => "-", NumpadAdd => "+",
        Numpad0 => "0", Numpad1 => "1", Numpad2 => "2", Numpad3 => "3", Numpad4 => "4",
        Numpad5 => "5", Numpad6 => "6", Numpad7 => "7", Numpad8 => "8", Numpad9 => "9",
        IntlBackslash => "\\",
        AudioVolumeMute => "Mudo", AudioVolumeDown => "Vol -", AudioVolumeUp => "Vol +",
        MediaTrackNext => "Próxima", MediaTrackPrevious => "Anterior", MediaStop => "Parar",
        MediaPlayPause => "Play/Pause",
    }
}

/// Nome sem ambiguidade, para listas de escolha ("Shift esquerdo", "Num 7"...).
pub fn describe(code: KeyCode, language: Language) -> String {
    use KeyCode::*;
    let side = match code {
        ShiftLeft | ControlLeft | AltLeft | MetaLeft => " esquerdo",
        ShiftRight | ControlRight | MetaRight => " direito",
        AltRight if label(code, language) == "Alt" => " direito",
        _ => "",
    };
    let label = label(code, language);
    let numpad = code.name().starts_with("Numpad") || code == NumLock;
    match code {
        Space => "Espaço".to_owned(),
        AudioVolumeMute | AudioVolumeDown | AudioVolumeUp => format!("Mídia: {label}"),
        MediaTrackNext | MediaTrackPrevious | MediaStop | MediaPlayPause => {
            format!("Mídia: {label}")
        }
        NumLock => "Num Lock".to_owned(),
        _ if numpad => format!("Num {label}"),
        _ => format!("{label}{side}"),
    }
}

#[derive(PartialEq)]
enum Escape60 {
    Yes,
    No,
}

#[derive(Default)]
struct Builder {
    keys: Vec<KeyRect>,
    x: f32,
    y: f32,
}

impl Builder {
    fn at(&mut self, x: f32, y: f32) -> &mut Self {
        self.x = x;
        self.y = y;
        self
    }

    fn gap(&mut self, w: f32) -> &mut Self {
        self.x += w;
        self
    }

    fn key(&mut self, code: KeyCode, w: f32) -> &mut Self {
        self.sized(code, w, 1.0)
    }

    fn sized(&mut self, code: KeyCode, w: f32, h: f32) -> &mut Self {
        self.keys.push(KeyRect { code, x: self.x, y: self.y, w, h, extra: None });
        self.x += w;
        self
    }

    fn keys(&mut self, codes: &[KeyCode], w: f32) -> &mut Self {
        for &code in codes {
            self.key(code, w);
        }
        self
    }
}

const DIGITS: [KeyCode; 10] = {
    use KeyCode::*;
    [Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Digit0]
};
const TOP_LETTERS: [KeyCode; 10] = {
    use KeyCode::*;
    [KeyQ, KeyW, KeyE, KeyR, KeyT, KeyY, KeyU, KeyI, KeyO, KeyP]
};
const HOME_LETTERS: [KeyCode; 9] = {
    use KeyCode::*;
    [KeyA, KeyS, KeyD, KeyF, KeyG, KeyH, KeyJ, KeyK, KeyL]
};
const BOTTOM_LETTERS: [KeyCode; 7] = {
    use KeyCode::*;
    [KeyZ, KeyX, KeyC, KeyV, KeyB, KeyN, KeyM]
};

/// Bloco principal (número, letras e Shifts), 15u de largura, começando em `y`.
fn alpha_block(b: &mut Builder, iso: bool, y: f32, escape: Escape60, right_shift: f32) {
    use KeyCode::*;
    // Em 60% e 65% o Esc ocupa o lugar do ` (o ` fica numa camada Fn do teclado).
    let first = if escape == Escape60::Yes { Escape } else { Backquote };
    b.at(0.0, y).key(first, 1.0).keys(&DIGITS, 1.0).key(Minus, 1.0).key(Equal, 1.0).key(Backspace, 2.0);

    b.at(0.0, y + 1.0).key(Tab, 1.5).keys(&TOP_LETTERS, 1.0).key(BracketLeft, 1.0).key(BracketRight, 1.0);
    if iso {
        b.keys.push(KeyRect {
            code: Enter,
            x: 13.5,
            y: y + 1.0,
            w: 1.5,
            h: 1.0,
            extra: Some([13.75, y + 2.0, 1.25, 1.0]),
        });
    } else {
        b.key(Backslash, 1.5);
    }

    b.at(0.0, y + 2.0).key(CapsLock, 1.75).keys(&HOME_LETTERS, 1.0).key(Semicolon, 1.0).key(Quote, 1.0);
    if iso {
        b.key(Backslash, 1.0);
    } else {
        b.key(Enter, 2.25);
    }

    b.at(0.0, y + 3.0);
    if iso {
        b.key(ShiftLeft, 1.25).key(IntlBackslash, 1.0);
    } else {
        b.key(ShiftLeft, 2.25);
    }
    b.keys(&BOTTOM_LETTERS, 1.0).key(Comma, 1.0).key(Period, 1.0).key(Slash, 1.0);
    if iso {
        b.key(IntlRo, 1.0).key(ShiftRight, right_shift - 1.0);
    } else {
        b.key(ShiftRight, right_shift);
    }
}

fn bottom_row_full(b: &mut Builder, y: f32) {
    use KeyCode::*;
    b.at(0.0, y)
        .key(ControlLeft, 1.25).key(MetaLeft, 1.25).key(AltLeft, 1.25).key(Space, 6.25)
        .key(AltRight, 1.25).key(MetaRight, 1.25).key(ContextMenu, 1.25).key(ControlRight, 1.25);
}

/// Linha de baixo dos 65%/75%, com as setas encaixadas à direita.
fn bottom_row_compact(b: &mut Builder, y: f32) {
    use KeyCode::*;
    b.at(0.0, y)
        .key(ControlLeft, 1.25).key(MetaLeft, 1.25).key(AltLeft, 1.25).key(Space, 6.25)
        .key(AltRight, 1.0).key(ContextMenu, 1.0).key(ControlRight, 1.0)
        .key(ArrowLeft, 1.0).key(ArrowDown, 1.0).key(ArrowRight, 1.0);
}

fn tenkeyless(b: &mut Builder, iso: bool) {
    use KeyCode::*;
    b.at(0.0, 0.0).key(Escape, 1.0);
    b.gap(1.0).keys(&[F1, F2, F3, F4], 1.0);
    b.gap(0.5).keys(&[F5, F6, F7, F8], 1.0);
    b.gap(0.5).keys(&[F9, F10, F11, F12], 1.0);
    b.gap(0.25).keys(&[PrintScreen, ScrollLock, Pause], 1.0);

    alpha_block(b, iso, 1.25, Escape60::No, 2.75);
    bottom_row_full(b, 5.25);

    b.at(15.25, 1.25).keys(&[Insert, Home, PageUp], 1.0);
    b.at(15.25, 2.25).keys(&[Delete, End, PageDown], 1.0);
    b.at(16.25, 4.25).key(ArrowUp, 1.0);
    b.at(15.25, 5.25).keys(&[ArrowLeft, ArrowDown, ArrowRight], 1.0);
}

/// Teclado numérico com o canto superior esquerdo em (`x`, `y`). Nos 96% o 0 é
/// estreito, porque a seta para a direita ocupa o lugar dele.
fn numpad(b: &mut Builder, iso: bool, x: f32, y: f32, wide_zero: bool) {
    use KeyCode::*;
    b.at(x, y).keys(&[NumLock, NumpadDivide, NumpadMultiply, NumpadSubtract], 1.0);
    b.at(x, y + 1.0).keys(&[Numpad7, Numpad8, Numpad9], 1.0);
    // No ABNT2 o + é menor para caber a tecla de ponto extra.
    if iso {
        b.key(NumpadAdd, 1.0);
        b.at(x, y + 2.0).keys(&[Numpad4, Numpad5, Numpad6, NumpadComma], 1.0);
    } else {
        b.sized(NumpadAdd, 1.0, 2.0);
        b.at(x, y + 2.0).keys(&[Numpad4, Numpad5, Numpad6], 1.0);
    }
    b.at(x, y + 3.0).keys(&[Numpad1, Numpad2, Numpad3], 1.0).sized(NumpadEnter, 1.0, 2.0);
    if wide_zero {
        b.at(x, y + 4.0).key(Numpad0, 2.0).key(NumpadDecimal, 1.0);
    } else {
        b.at(x + 1.0, y + 4.0).key(Numpad0, 1.0).key(NumpadDecimal, 1.0);
    }
}

/// Esc e F1–F12 em blocos de 4, sem o espaço largo do TKL (75% e 96%).
fn compact_function_row(b: &mut Builder) {
    use KeyCode::*;
    b.at(0.0, 0.0).key(Escape, 1.0);
    b.gap(0.25).keys(&[F1, F2, F3, F4], 1.0);
    b.gap(0.25).keys(&[F5, F6, F7, F8], 1.0);
    b.gap(0.25).keys(&[F9, F10, F11, F12], 1.0);
}

/// 96% / "1800": numérico colado ao bloco principal e setas abaixo do Shift direito.
fn compact96(b: &mut Builder, iso: bool) {
    use KeyCode::*;
    compact_function_row(b);
    b.gap(0.25).key(Delete, 1.0);
    b.at(15.5, 0.0).keys(&[Home, End, PageUp, PageDown], 1.0);

    alpha_block(b, iso, 1.25, Escape60::No, 1.75);
    b.at(14.25, 4.25).key(ArrowUp, 1.0);
    b.at(0.0, 5.25)
        .key(ControlLeft, 1.25).key(MetaLeft, 1.25).key(AltLeft, 1.25).key(Space, 6.25)
        .key(AltRight, 1.0).key(ContextMenu, 1.0).key(ControlRight, 1.0)
        .gap(0.25).key(ArrowLeft, 1.0).key(ArrowDown, 1.0).key(ArrowRight, 1.0);

    numpad(b, iso, 15.5, 1.25, false);
}

/// 40%: só letras e a linha de baixo; números e F1–F12 ficam na camada Fn do teclado.
fn compact40(b: &mut Builder, iso: bool) {
    use KeyCode::*;
    b.at(0.0, 0.0).key(Escape, 1.25).keys(&TOP_LETTERS, 1.0).key(Backspace, 1.5);
    b.at(0.0, 1.0).key(CapsLock, 1.5).keys(&HOME_LETTERS, 1.0).key(Semicolon, 1.0).key(Enter, 1.25);
    b.at(0.0, 2.0);
    if iso {
        b.key(ShiftLeft, 1.25).key(IntlBackslash, 1.0);
    } else {
        b.key(ShiftLeft, 2.0);
    }
    b.keys(&BOTTOM_LETTERS, 1.0).key(Comma, 1.0).key(Period, 1.0);
    b.key(ShiftRight, if iso { 1.5 } else { 1.75 });
    b.at(0.0, 3.0)
        .key(ControlLeft, 1.25).key(MetaLeft, 1.25).key(AltLeft, 1.25).key(Space, 6.25)
        .key(AltRight, 1.25).key(ControlRight, 1.5);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const LAYOUTS: [Layout; 7] = Layout::ALL;
    const LANGUAGES: [Language; 3] = [Language::PtBrAbnt2, Language::EnUs, Language::EnUsIntl];

    #[test]
    fn no_duplicate_keys() {
        for layout in LAYOUTS {
            for language in LANGUAGES {
                let mut seen = HashSet::new();
                for key in keys(layout, language) {
                    assert!(seen.insert(key.code), "{key:?} repetida em {layout:?}/{language:?}");
                }
            }
        }
    }

    #[test]
    fn no_overlapping_keys() {
        fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
            const E: f32 = 0.001;
            a[0] + a[2] > b[0] + E && b[0] + b[2] > a[0] + E && a[1] + a[3] > b[1] + E && b[1] + b[3] > a[1] + E
        }
        fn rects(k: &KeyRect) -> Vec<[f32; 4]> {
            let mut r = vec![[k.x, k.y, k.w, k.h]];
            r.extend(k.extra);
            r
        }
        for layout in LAYOUTS {
            for language in LANGUAGES {
                let all = keys(layout, language);
                for (i, a) in all.iter().enumerate() {
                    for b in &all[i + 1..] {
                        for ra in rects(a) {
                            for rb in rects(b) {
                                assert!(!overlaps(ra, rb), "{:?} e {:?} se sobrepõem em {layout:?}", a.code, b.code);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn compact40_rows_have_same_width() {
        for language in LANGUAGES {
            let all = keys(Layout::Compact40, language);
            for row in 0..4 {
                let width: f32 = all.iter().filter(|k| k.y == row as f32).map(|k| k.w).sum();
                assert!((width - 12.75).abs() < 0.001, "linha {row} com {width}u em {language:?}");
            }
        }
    }

    #[test]
    fn main_rows_are_15_units() {
        for language in LANGUAGES {
            let all = keys(Layout::Compact60, language);
            let rects: Vec<[f32; 4]> = all
                .iter()
                .flat_map(|k| std::iter::once([k.x, k.y, k.w, k.h]).chain(k.extra))
                .collect();
            for row in 0..5 {
                let width: f32 = rects.iter().filter(|r| r[1] == row as f32).map(|r| r[2]).sum();
                assert!((width - 15.0).abs() < 0.001, "linha {row} com {width}u em {language:?}");
            }
        }
    }
}
