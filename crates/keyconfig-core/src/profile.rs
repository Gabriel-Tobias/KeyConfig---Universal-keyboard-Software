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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Win,
}

impl Modifier {
    pub const ALL: [Modifier; 4] = [Modifier::Ctrl, Modifier::Shift, Modifier::Alt, Modifier::Win];

    pub fn label(self) -> &'static str {
        match self {
            Modifier::Ctrl => "Ctrl",
            Modifier::Shift => "Shift",
            Modifier::Alt => "Alt",
            Modifier::Win => "Win",
        }
    }
}

/// Modificadores segurados, sem distinguir o lado. No JSON: `["ctrl", "shift"]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(from = "Vec<Modifier>", into = "Vec<Modifier>")]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Mods = Mods(0);

    pub fn has(self, m: Modifier) -> bool {
        self.0 & (1 << m as u8) != 0
    }

    pub fn set(&mut self, m: Modifier, on: bool) {
        if on {
            self.0 |= 1 << m as u8;
        } else {
            self.0 &= !(1 << m as u8);
        }
    }

    pub fn with(mut self, m: Modifier) -> Mods {
        self.set(m, true);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        Modifier::ALL.into_iter().filter(move |&m| self.has(m))
    }
}

impl From<Vec<Modifier>> for Mods {
    fn from(list: Vec<Modifier>) -> Self {
        list.into_iter().fold(Mods::NONE, Mods::with)
    }
}

impl From<Mods> for Vec<Modifier> {
    fn from(mods: Mods) -> Self {
        mods.iter().collect()
    }
}

/// Quando uma ação dispara: com quais modificadores e se pede toque duplo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Trigger {
    pub mods: Mods,
    pub double_tap: bool,
}

impl Trigger {
    /// Toque simples, com ou sem modificadores: o mapeamento comum da tecla.
    pub const PLAIN: Trigger = Trigger { mods: Mods::NONE, double_tap: false };

    pub fn is_plain(self) -> bool {
        self == Trigger::PLAIN
    }

    /// "Shift + 2×", para mostrar ao usuário. Vazio no gatilho comum.
    pub fn label(self) -> String {
        let mut parts: Vec<&str> = self.mods.iter().map(Modifier::label).collect();
        if self.double_tap {
            parts.push("2×");
        }
        parts.join(" + ")
    }
}

/// Ação que só vale com modificadores e/ou toque duplo, como `Shift` + `;` `;` = `?`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combo {
    pub key: KeyCode,
    #[serde(default, skip_serializing_if = "Mods::is_empty")]
    pub mods: Mods,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub double_tap: bool,
    pub action: Action,
}

impl Combo {
    pub fn trigger(&self) -> Trigger {
        Trigger { mods: self.mods, double_tap: self.double_tap }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub language: Language,
    /// Toque simples. Vale também com modificadores, se não houver um combo para eles.
    #[serde(default)]
    pub mappings: BTreeMap<KeyCode, Action>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combos: Vec<Combo>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Profile {
            name: name.into(),
            layout: Layout::default(),
            language: Language::default(),
            mappings: BTreeMap::new(),
            combos: Vec::new(),
        }
    }

    pub fn action(&self, key: KeyCode, trigger: Trigger) -> Option<&Action> {
        if trigger.is_plain() {
            return self.mappings.get(&key);
        }
        self.combos
            .iter()
            .find(|c| c.key == key && c.trigger() == trigger)
            .map(|c| &c.action)
    }

    /// Define (ou, com `None`, remove) a ação da tecla para este gatilho.
    pub fn set_action(&mut self, key: KeyCode, trigger: Trigger, action: Option<Action>) {
        if trigger.is_plain() {
            match action {
                Some(a) => self.mappings.insert(key, a),
                None => self.mappings.remove(&key),
            };
            return;
        }
        let pos = self.combos.iter().position(|c| c.key == key && c.trigger() == trigger);
        match (pos, action) {
            (Some(i), Some(a)) => self.combos[i].action = a,
            (Some(i), None) => {
                self.combos.remove(i);
            }
            (None, Some(a)) => {
                self.combos.push(Combo { key, mods: trigger.mods, double_tap: trigger.double_tap, action: a });
                self.combos.sort_by_key(|c| (c.key, c.trigger()));
            }
            (None, None) => {}
        }
    }

    /// Gatilhos especiais (não comuns) que a tecla tem.
    pub fn combo_triggers(&self, key: KeyCode) -> Vec<Trigger> {
        self.combos.iter().filter(|c| c.key == key).map(Combo::trigger).collect()
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
        let shift_double = Trigger { mods: Mods::NONE.with(Modifier::Shift), double_tap: true };
        profile.set_action(KeyCode::Slash, shift_double, Some(Action::Text { text: "?".into() }));
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(serde_json::from_str::<Profile>(&json).unwrap(), profile);
    }

    #[test]
    fn parses_combos() {
        let json = r#"{
            "name": "Combos",
            "combos": [
                { "key": "Slash", "mods": ["shift"], "double_tap": true, "action": { "type": "text", "text": "?" } },
                { "key": "KeyJ", "mods": ["ctrl", "alt"], "action": { "type": "disabled" } }
            ]
        }"#;
        let profile: Profile = serde_json::from_str(json).unwrap();
        let shift_double = Trigger { mods: Mods::NONE.with(Modifier::Shift), double_tap: true };
        assert_eq!(profile.action(KeyCode::Slash, shift_double), Some(&Action::Text { text: "?".into() }));
        assert_eq!(profile.action(KeyCode::Slash, Trigger::PLAIN), None);
        let ctrl_alt = Trigger { mods: Mods::NONE.with(Modifier::Alt).with(Modifier::Ctrl), double_tap: false };
        assert_eq!(profile.action(KeyCode::KeyJ, ctrl_alt), Some(&Action::Disabled));
        assert_eq!(ctrl_alt.label(), "Ctrl + Alt");
    }

    #[test]
    fn set_action_adds_replaces_and_removes() {
        let mut profile = Profile::new("x");
        let double = Trigger { mods: Mods::NONE, double_tap: true };
        profile.set_action(KeyCode::KeyA, double, Some(Action::Disabled));
        profile.set_action(KeyCode::KeyA, double, Some(Action::Text { text: "a".into() }));
        assert_eq!(profile.combos.len(), 1);
        assert_eq!(profile.combo_triggers(KeyCode::KeyA), vec![double]);
        profile.set_action(KeyCode::KeyA, double, None);
        assert!(profile.combos.is_empty());
        profile.set_action(KeyCode::KeyA, Trigger::PLAIN, Some(Action::Disabled));
        assert_eq!(profile.mappings[&KeyCode::KeyA], Action::Disabled);
    }
}
