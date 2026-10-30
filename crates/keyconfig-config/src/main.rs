//! Configurador do KeyConfig: edita perfis e avisa o motor para recarregar.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod capture;
mod editor;
mod engine;
mod keyboard;
mod single_instance;
mod theme;

use eframe::egui;

fn main() -> eframe::Result {
    if !single_instance::acquire() {
        single_instance::activate_existing();
        return Ok(());
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("KeyConfig")
        .with_inner_size([1360.0, 900.0])
        .with_min_inner_size([980.0, 640.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon/icon-256.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "KeyConfig",
        options,
        Box::new(|cc| Ok(Box::new(app::ConfigApp::new(cc)))),
    )
}
