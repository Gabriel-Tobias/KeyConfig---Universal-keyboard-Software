use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{tr, KeyCode};

/// Formato físico do teclado. Por enquanto só informa o configurador quais teclas desenhar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Layout {
    #[serde(rename = "40%")]
    Compact40,
    #[serde(rename = "60%")]
    Compact60,
    #[serde(rename = "65%")]
    Compact65,
    #[serde(rename = "75%")]
    Compact75,
    #[serde(rename = "tkl")]
    Tenkeyless,
    #[serde(rename = "96%")]
    Compact96,
    #[default]
    #[serde(rename = "100%")]
    Full,
}

/// Idioma impresso nas teclas. O remapeamento trabalha com posições físicas,
/// então isso só muda os rótulos mostrados ao usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "pt-BR-abnt2")]
    PtBrAbnt2,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "en-US-intl")]
    EnUsIntl,
    #[serde(rename = "es-ES")]
    EsEs,
}

impl Layout {
    pub const ALL: [Layout; 7] = [
        Layout::Compact40,
        Layout::Compact60,
        Layout::Compact65,
        Layout::Compact75,
        Layout::Tenkeyless,
        Layout::Compact96,
        Layout::Full,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Layout::Compact40 => "40%",
            Layout::Compact60 => "60%",
            Layout::Compact65 => "65%",
            Layout::Compact75 => "75%",
            Layout::Tenkeyless => "TKL (80%)",
            Layout::Compact96 => "96% (1800)",
            Layout::Full => tr("Completo (100%)", "Full size (100%)", "Completo (100%)"),
        }
    }
}

impl Language {
    pub const ALL: [Language; 4] = [Language::PtBrAbnt2, Language::EnUs, Language::EnUsIntl, Language::EsEs];

    pub fn label(self) -> &'static str {
        match self {
            Language::PtBrAbnt2 => tr("Português (ABNT2)", "Portuguese (ABNT2)", "Portugués (ABNT2)"),
            Language::EnUs => tr("Inglês (US)", "English (US)", "Inglés (EE. UU.)"),
            Language::EnUsIntl => tr("Inglês (US Internacional)", "English (US International)", "Inglés (EE. UU. internacional)"),
            Language::EsEs => tr("Espanhol (Espanha)", "Spanish (Spain)", "Español (España)"),
        }
    }

    /// Layout ISO: Enter em "L" e a tecla extra ao lado do Shift esquerdo.
    pub fn is_iso(self) -> bool {
        matches!(self, Language::PtBrAbnt2 | Language::EsEs)
    }
}

/// O que acontece quando uma tecla mapeada é pressionada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Age como outra tecla.
    Key { to: KeyCode },
    /// Não faz nada.
    Disabled,
    /// Combinação de teclas (ex.: Ctrl+C), mantida enquanto a tecla estiver pressionada.
    Shortcut { keys: Vec<KeyCode> },
    /// Sequência de passos executada uma vez a cada toque.
    Macro { steps: Vec<MacroStep> },
    /// Digita um texto, independente do idioma do teclado.
    Text { text: String },
    /// Acento que se combina com a próxima letra (`~` + `a` = `ã`).
    DeadKey { accent: char },
    /// Abre um programa, arquivo, pasta ou site.
    Run {
        command: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        args: String,
    },
}

/// Um passo de macro, escrito como `{ "tap": "KeyA" }`, `{ "delay_ms": 100 }` etc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroStep {
    /// Aperta e solta.
    Tap(KeyCode),
    Down(KeyCode),
    Up(KeyCode),
    DelayMs(u32),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub mappings: BTreeMap<KeyCode, Action>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Profile {
            name: name.into(),
            layout: Layout::default(),
            language: Language::default(),
            mappings: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profile_json() {
        let json = r#"{
            "name": "Jogos",
            "layout": "75%",
            "language": "pt-BR-abnt2",
            "mappings": {
                "CapsLock": { "type": "key", "to": "Escape" },
                "MetaLeft": { "type": "disabled" }
            }
        }"#;
        let profile: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.layout, Layout::Compact75);
        assert_eq!(
            profile.mappings[&KeyCode::CapsLock],
            Action::Key { to: KeyCode::Escape }
        );
        assert_eq!(profile.mappings[&KeyCode::MetaLeft], Action::Disabled);
    }

    #[test]
    fn parses_macro_actions() {
        let json = r#"{
            "name": "Macros",
            "mappings": {
                "F13": { "type": "shortcut", "keys": ["ControlLeft", "KeyC"] },
                "F14": { "type": "macro", "steps": [
                    { "down": "ShiftLeft" }, { "tap": "KeyA" }, { "up": "ShiftLeft" },
                    { "delay_ms": 50 }, { "text": "olá" }
                ] },
                "F15": { "type": "text", "text": "Atenciosamente,\nGabriel" },
                "F16": { "type": "run", "command": "notepad.exe" },
                "F17": { "type": "dead_key", "accent": "~" }
            }
        }"#;
        let profile: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(
            profile.mappings[&KeyCode::F13],
            Action::Shortcut { keys: vec![KeyCode::ControlLeft, KeyCode::KeyC] }
        );
        assert_eq!(
            profile.mappings[&KeyCode::F14],
            Action::Macro {
                steps: vec![
                    MacroStep::Down(KeyCode::ShiftLeft),
                    MacroStep::Tap(KeyCode::KeyA),
                    MacroStep::Up(KeyCode::ShiftLeft),
                    MacroStep::DelayMs(50),
                    MacroStep::Text("olá".into()),
                ]
            }
        );
        assert_eq!(
            profile.mappings[&KeyCode::F16],
            Action::Run { command: "notepad.exe".into(), args: String::new() }
        );
        assert_eq!(profile.mappings[&KeyCode::F17], Action::DeadKey { accent: '~' });
    }

    #[test]
    fn missing_fields_use_defaults() {
        let profile: Profile = serde_json::from_str(r#"{ "name": "Vazio" }"#).unwrap();
        assert_eq!(profile, Profile::new("Vazio"));
    }

    #[test]
    fn rejects_unknown_key() {
        let json = r#"{ "name": "x", "mappings": { "Banana": { "type": "disabled" } } }"#;
        assert!(serde_json::from_str::<Profile>(json).is_err());
    }

    #[test]
    fn round_trips() {
        let mut profile = Profile::new("Teste");
        profile.mappings.insert(KeyCode::KeyA, Action::Key { to: KeyCode::KeyB });
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(serde_json::from_str::<Profile>(&json).unwrap(), profile);
    }
}
