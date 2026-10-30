//! Configurador do KeyConfig: edita perfis e avisa o motor para recarregar.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor;
mod engine;
mod keyboard;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KeyConfig")
            .with_inner_size([1180.0, 880.0])
            .with_min_inner_size([860.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "KeyConfig",
        options,
        Box::new(|cc| Ok(Box::new(app::ConfigApp::new(cc)))),
    )
}
