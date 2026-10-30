//! Idioma da interface (português, inglês e espanhol).
//!
//! Os textos ficam no próprio código, nas três línguas lado a lado:
//! `tr("Salvar", "Save", "Guardar")`. É simples de manter neste tamanho de app
//! e o compilador garante que nenhuma tradução fica faltando.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiLang {
    #[serde(rename = "pt")]
    Pt,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "es")]
    Es,
}

impl UiLang {
    pub const ALL: [UiLang; 3] = [UiLang::Pt, UiLang::En, UiLang::Es];

    /// Nome do idioma escrito nele mesmo, para a lista de escolha.
    pub fn native_name(self) -> &'static str {
        match self {
            UiLang::Pt => "Português",
            UiLang::En => "English",
            UiLang::Es => "Español",
        }
    }

    /// A partir do idioma do Windows (`GetUserDefaultUILanguage`).
    pub fn from_windows_langid(langid: u16) -> UiLang {
        match langid & 0x3ff {
            0x16 => UiLang::Pt,
            0x0a => UiLang::Es,
            _ => UiLang::En,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: UiLang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> UiLang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => UiLang::En,
        2 => UiLang::Es,
        _ => UiLang::Pt,
    }
}

/// Escolhe o texto do idioma atual.
pub fn tr(pt: &'static str, en: &'static str, es: &'static str) -> &'static str {
    match current() {
        UiLang::Pt => pt,
        UiLang::En => en,
        UiLang::Es => es,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_windows_language() {
        assert_eq!(UiLang::from_windows_langid(0x0416), UiLang::Pt); // pt-BR
        assert_eq!(UiLang::from_windows_langid(0x0816), UiLang::Pt); // pt-PT
        assert_eq!(UiLang::from_windows_langid(0x0c0a), UiLang::Es); // es-ES
        assert_eq!(UiLang::from_windows_langid(0x080a), UiLang::Es); // es-MX
        assert_eq!(UiLang::from_windows_langid(0x0409), UiLang::En); // en-US
        assert_eq!(UiLang::from_windows_langid(0x0407), UiLang::En); // de-DE
    }

    #[test]
    fn switches_language() {
        set(UiLang::Es);
        assert_eq!(tr("Salvar", "Save", "Guardar"), "Guardar");
        set(UiLang::En);
        assert_eq!(tr("Salvar", "Save", "Guardar"), "Save");
        set(UiLang::Pt);
        assert_eq!(tr("Salvar", "Save", "Guardar"), "Salvar");
    }
}
