use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::KeyCode;

/// Formato físico do teclado. Por enquanto só informa o configurador quais teclas desenhar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Layout {
    #[serde(rename = "60%")]
    Compact60,
    #[serde(rename = "65%")]
    Compact65,
    #[serde(rename = "75%")]
    Compact75,
    #[serde(rename = "tkl")]
    Tenkeyless,
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
}

/// O que acontece quando uma tecla mapeada é pressionada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Age como outra tecla.
    Key { to: KeyCode },
    /// Não faz nada.
    Disabled,
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
