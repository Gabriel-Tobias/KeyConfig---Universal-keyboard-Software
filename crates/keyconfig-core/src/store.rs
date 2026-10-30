//! Leitura e escrita da configuração em `%APPDATA%\KeyConfig`.
//!
//! ```text
//! KeyConfig\
//!   settings.json        -> { "active_profile": "default" }
//!   profiles\default.json
//!   profiles\jogos.json
//! ```
//!
//! O id de um perfil é o nome do arquivo; o nome mostrado ao usuário fica dentro dele.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{tr, Profile, UiLang};

pub const DEFAULT_PROFILE_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub active_profile: String,
    /// Idioma da interface; sem valor, segue o idioma do Windows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_language: Option<UiLang>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            active_profile: DEFAULT_PROFILE_ID.to_owned(),
            ui_language: None,
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
            StoreError::NoAppData => f.write_str(tr(
                "a variável APPDATA não está definida",
                "the APPDATA variable is not set",
                "la variable APPDATA no está definida",
            )),
            StoreError::InvalidProfileId(id) => write!(
                f,
                "{}: \"{id}\" ({})",
                tr("nome de perfil inválido", "invalid profile name", "nombre de perfil no válido"),
                tr("use só letras, números, - e _", "use only letters, numbers, - and _", "usa solo letras, números, - y _"),
            ),
            StoreError::ProfileNotFound(path) => write!(
                f,
                "{}: {}",
                tr("perfil não encontrado", "profile not found", "perfil no encontrado"),
                path.display()
            ),
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

/// Perfil encontrado na pasta, mesmo que o arquivo esteja com erro.
pub struct Entry {
    pub id: String,
    pub profile: Result<Profile, StoreError>,
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// A pasta padrão em `%APPDATA%\KeyConfig`.
    pub fn open() -> Result<Store, StoreError> {
        let appdata = std::env::var_os("APPDATA").ok_or(StoreError::NoAppData)?;
        Ok(Store::at(PathBuf::from(appdata).join("KeyConfig")))
    }

    pub fn at(dir: impl Into<PathBuf>) -> Store {
        Store { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.dir.join("profiles")
    }

    /// Lê as configurações, criando o arquivo padrão se não existir.
    /// O `bool` diz se o arquivo foi criado agora.
    pub fn load_settings(&self) -> Result<(Settings, bool), StoreError> {
        let path = self.dir.join("settings.json");
        if path.exists() {
            Ok((read_json(&path)?, false))
        } else {
            let settings = Settings::default();
            self.save_settings(&settings)?;
            Ok((settings, true))
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), StoreError> {
        write_json(&self.dir.join("settings.json"), settings)
    }

    pub fn load_profile(&self, id: &str) -> Result<Profile, StoreError> {
        let path = self.profile_path(id)?;
        if !path.exists() {
            return Err(StoreError::ProfileNotFound(path));
        }
        read_json(&path)
    }

    pub fn save_profile(&self, id: &str, profile: &Profile) -> Result<(), StoreError> {
        write_json(&self.profile_path(id)?, profile)
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), StoreError> {
        let path = self.profile_path(id)?;
        fs::remove_file(&path).map_err(|e| StoreError::Io(path, e))
    }

    /// Todos os perfis da pasta, ordenados pelo nome mostrado.
    pub fn list_profiles(&self) -> Result<Vec<Entry>, StoreError> {
        let dir = self.profiles_dir();
        fs::create_dir_all(&dir).map_err(|e| StoreError::Io(dir.clone(), e))?;
        let read = fs::read_dir(&dir).map_err(|e| StoreError::Io(dir.clone(), e))?;

        let mut entries: Vec<Entry> = read
            .filter_map(Result::ok)
            .filter_map(|e| {
                let path = e.path();
                if path.extension()? != "json" {
                    return None;
                }
                let id = path.file_stem()?.to_str()?.to_owned();
                is_valid_profile_id(&id).then(|| Entry {
                    profile: read_json(&path),
                    id,
                })
            })
            .collect();
        entries.sort_by_cached_key(|e| match &e.profile {
            Ok(p) => p.name.to_lowercase(),
            Err(_) => e.id.to_lowercase(),
        });
        Ok(entries)
    }

    /// Gera um id livre a partir do nome ("Jogos FPS" -> "jogos-fps", "jogos-fps-2"...).
    pub fn new_profile_id(&self, name: &str) -> String {
        let mut base: String = name
            .chars()
            .map(|c| fold_accent(c.to_ascii_lowercase()))
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        base = base.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
        if base.is_empty() {
            base = "perfil".to_owned();
        }
        let taken = |id: &str| self.profiles_dir().join(format!("{id}.json")).exists();
        if !taken(&base) {
            return base;
        }
        (2..)
            .map(|n| format!("{base}-{n}"))
            .find(|id| !taken(id))
            .expect("sempre existe um número livre")
    }

    /// Carrega o perfil ativo, criando a configuração padrão se ainda não existir.
    pub fn load_active(&self) -> Result<Loaded, StoreError> {
        let profiles = self.profiles_dir();
        fs::create_dir_all(&profiles).map_err(|e| StoreError::Io(profiles.clone(), e))?;

        let (settings, mut created_defaults) = self.load_settings()?;
        let id = &settings.active_profile;
        let profile = match self.load_profile(id) {
            Err(StoreError::ProfileNotFound(_)) if id == DEFAULT_PROFILE_ID => {
                let profile = Profile::new("Padrão");
                self.save_profile(id, &profile)?;
                created_defaults = true;
                profile
            }
            other => other?,
        };

        Ok(Loaded {
            settings,
            profile,
            created_defaults,
        })
    }

    fn profile_path(&self, id: &str) -> Result<PathBuf, StoreError> {
        if !is_valid_profile_id(id) {
            return Err(StoreError::InvalidProfileId(id.to_owned()));
        }
        Ok(self.profiles_dir().join(format!("{id}.json")))
    }
}

pub fn config_dir() -> Result<PathBuf, StoreError> {
    Ok(Store::open()?.dir)
}

pub fn load_active() -> Result<Loaded, StoreError> {
    Store::open()?.load_active()
}

/// O id vira nome de arquivo, então não pode conter separadores de caminho.
pub fn is_valid_profile_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn fold_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        _ => c,
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, StoreError> {
    let text = fs::read_to_string(path).map_err(|e| StoreError::Io(path.to_owned(), e))?;
    // Editores do Windows às vezes salvam com BOM.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text).map_err(|e| StoreError::Json(path.to_owned(), e))
}

/// Grava num arquivo temporário e renomeia, para o motor nunca ler um arquivo pela metade.
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let io_err = |e| StoreError::Io(path.to_owned(), e);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let text = serde_json::to_string_pretty(value).expect("tipos de configuração sempre serializam");
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(io_err)?;
    fs::rename(&tmp, path).map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempStore(Store);

    impl TempStore {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("keyconfig-test-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            TempStore(Store::at(dir))
        }
    }

    impl Drop for TempStore {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0.dir());
        }
    }

    #[test]
    fn profile_id_validation() {
        assert!(is_valid_profile_id("default"));
        assert!(is_valid_profile_id("jogos_fps-2"));
        assert!(!is_valid_profile_id(""));
        assert!(!is_valid_profile_id("..\\..\\windows"));
        assert!(!is_valid_profile_id("a/b"));
    }

    #[test]
    fn first_load_creates_defaults() {
        let t = TempStore::new("defaults");
        let loaded = t.0.load_active().unwrap();
        assert!(loaded.created_defaults);
        assert_eq!(loaded.profile.name, "Padrão");
        assert!(!t.0.load_active().unwrap().created_defaults);
    }

    #[test]
    fn save_list_and_delete() {
        let t = TempStore::new("crud");
        t.0.save_profile("b", &Profile::new("Zeta")).unwrap();
        t.0.save_profile("a", &Profile::new("Alfa")).unwrap();
        let names: Vec<_> = t.0.list_profiles().unwrap().into_iter()
            .map(|e| e.profile.unwrap().name)
            .collect();
        assert_eq!(names, ["Alfa", "Zeta"]);

        t.0.delete_profile("a").unwrap();
        assert_eq!(t.0.list_profiles().unwrap().len(), 1);
    }

    #[test]
    fn new_ids_are_slugs_and_unique() {
        let t = TempStore::new("ids");
        assert_eq!(t.0.new_profile_id("Jogos FPS!"), "jogos-fps");
        assert_eq!(t.0.new_profile_id("Edição de vídeo"), "edicao-de-video");
        assert_eq!(t.0.new_profile_id("???"), "perfil");
        t.0.save_profile("jogos-fps", &Profile::new("x")).unwrap();
        assert_eq!(t.0.new_profile_id("Jogos FPS"), "jogos-fps-2");
    }
}
