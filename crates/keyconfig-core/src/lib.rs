//! Modelo de dados do KeyConfig: teclas físicas, perfis e armazenamento em disco.
//! Não depende de Windows, então pode ser reutilizado pelo configurador.

pub mod accent;
pub mod geometry;
pub mod i18n;
pub mod keycode;
pub mod profile;
pub mod store;

pub use i18n::{tr, UiLang};
pub use keycode::{KeyCode, KeyLookup, KeyOutput};
pub use profile::{Action, Language, Layout, MacroStep, Profile};

/// Nomes compartilhados entre o motor e o configurador.
pub mod ipc {
    /// Classe da janela invisível do motor, usada para encontrá-lo.
    pub const ENGINE_WINDOW_CLASS: &str = "KeyConfigEngineWindow";
    /// Mensagem registrada que pede ao motor para recarregar o perfil ativo.
    pub const RELOAD_MESSAGE: &str = "KeyConfig.Reload";
    /// Marca (`dwExtraInfo`) das teclas que o motor envia ("KCFG").
    pub const INJECT_TAG: usize = 0x4B43_4647;
    pub const ENGINE_EXE: &str = "keyconfig.exe";
    pub const CONFIG_EXE: &str = "keyconfig-config.exe";
}
