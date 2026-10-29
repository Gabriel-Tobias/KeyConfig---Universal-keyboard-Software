//! Modelo de dados do KeyConfig: teclas físicas, perfis e armazenamento em disco.
//! Não depende de Windows, então pode ser reutilizado pelo configurador.

pub mod keycode;
pub mod profile;
pub mod store;

pub use keycode::{KeyCode, KeyOutput};
pub use profile::{Action, Language, Layout, MacroStep, Profile};
