//! Leitura e escrita da configuração em `%APPDATA%\KeyConfig`.
//!
//! ```text
//! KeyConfig\
//!   settings.json        -> { "active_profile": "default" }
//!   profiles\default.json
//! ```

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::Profile;

pub const DEFAULT_PROFILE_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub active_profile: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            active_profile: DEFAULT_PROFILE_ID.to_owned(),
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    NoAppData,
    InvalidProfileId(String),
    ProfileNotFound(PathBuf),
    Io(PathBuf, io::Error),
    Json(PathBuf, serde_json::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NoAppData => write!(f, "a variável APPDATA não está definida"),
            StoreError::InvalidProfileId(id) => write!(
                f,
                "nome de perfil inválido: \"{id}\" (use só letras, números, - e _)"
            ),
            StoreError::ProfileNotFound(path) => {
                write!(f, "perfil não encontrado: {}", path.display())
            }
            StoreError::Io(path, e) => write!(f, "{}: {e}", path.display()),
            StoreError::Json(path, e) => write!(f, "{}: {e}", path.display()),
        }
    }
}

impl std::error::Error for StoreError {}

/// Resultado de carregar o perfil ativo.
pub struct Loaded {
    pub settings: Settings,
    pub profile: Profile,
    /// `true` quando algum arquivo padrão foi criado agora (primeira execução).
    pub created_defaults: bool,
}

pub fn config_dir() -> Result<PathBuf, StoreError> {
    std::env::var_os("APPDATA")
        .map(|p| PathBuf::from(p).join("KeyConfig"))
        .ok_or(StoreError::NoAppData)
}

pub fn profiles_dir() -> Result<PathBuf, StoreError> {
    Ok(config_dir()?.join("profiles"))
}

/// Carrega o perfil ativo, criando a configuração padrão se ainda não existir.
pub fn load_active() -> Result<Loaded, StoreError> {
    let dir = config_dir()?;
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).map_err(|e| StoreError::Io(profiles.clone(), e))?;

    let mut created_defaults = false;

    let settings_path = dir.join("settings.json");
    let settings: Settings = if settings_path.exists() {
        read_json(&settings_path)?
    } else {
        let settings = Settings::default();
        write_json(&settings_path, &settings)?;
        created_defaults = true;
        settings
    };

    let id = &settings.active_profile;
    if !is_valid_profile_id(id) {
        return Err(StoreError::InvalidProfileId(id.clone()));
    }

    let profile_path = profiles.join(format!("{id}.json"));
    let profile: Profile = if profile_path.exists() {
        read_json(&profile_path)?
    } else if id == DEFAULT_PROFILE_ID {
        let profile = Profile::new("Padrão");
        write_json(&profile_path, &profile)?;
        created_defaults = true;
        profile
    } else {
        return Err(StoreError::ProfileNotFound(profile_path));
    };

    Ok(Loaded {
        settings,
        profile,
        created_defaults,
    })
}

/// O id vira nome de arquivo, então não pode conter separadores de caminho.
pub fn is_valid_profile_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, StoreError> {
    let text = fs::read_to_string(path).map_err(|e| StoreError::Io(path.to_owned(), e))?;
    // Editores do Windows às vezes salvam com BOM.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text).map_err(|e| StoreError::Json(path.to_owned(), e))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let text = serde_json::to_string_pretty(value).expect("tipos de configuração sempre serializam");
    fs::write(path, text).map_err(|e| StoreError::Io(path.to_owned(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_id_validation() {
        assert!(is_valid_profile_id("default"));
        assert!(is_valid_profile_id("jogos_fps-2"));
        assert!(!is_valid_profile_id(""));
        assert!(!is_valid_profile_id("..\\..\\windows"));
        assert!(!is_valid_profile_id("a/b"));
    }
}
