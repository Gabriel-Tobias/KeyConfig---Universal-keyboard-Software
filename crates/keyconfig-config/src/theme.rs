//! Tema visual: cores do logo (preto profundo, azul e ciano), fontes e widgets.

use eframe::egui::{
    self, vec2, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin, RichText,
    Rounding, Shadow, Stroke, TextStyle, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(0x08, 0x0a, 0x0f);
pub const PANEL: Color32 = Color32::from_rgb(0x0e, 0x11, 0x18);
pub const SURFACE: Color32 = Color32::from_rgb(0x16, 0x1a, 0x24);
pub const SURFACE_HI: Color32 = Color32::from_rgb(0x1f, 0x25, 0x33);
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x2d, 0x3d);
pub const TEXT: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf5);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x95, 0x9d, 0xb2);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x5a, 0x62, 0x77);
pub const ACCENT: Color32 = Color32::from_rgb(0x2b, 0x8c, 0xff);
pub const CYAN: Color32 = Color32::from_rgb(0x1f, 0xd0, 0xff);
pub const OK: Color32 = Color32::from_rgb(0x3d, 0xdc, 0x97);
pub const WARN: Color32 = Color32::from_rgb(0xff, 0xb8, 0x4d);
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x6b, 0x6b);

pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());

    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = SURFACE;
    v.extreme_bg_color = Color32::from_rgb(0x0b, 0x0e, 0x14);
    v.faint_bg_color = SURFACE;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(8.0);
    v.window_shadow = Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: Color32::from_black_alpha(140) };
    v.popup_shadow = Shadow { offset: vec2(0.0, 6.0), blur: 16.0, spread: 0.0, color: Color32::from_black_alpha(120) };
    v.hyperlink_color = CYAN;
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    w.noninteractive.rounding = Rounding::same(8.0);

    w.inactive.bg_fill = SURFACE_HI;
    w.inactive.weak_bg_fill = SURFACE_HI;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.inactive.rounding = Rounding::same(8.0);

    w.hovered.bg_fill = Color32::from_rgb(0x28, 0x30, 0x44);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x28, 0x30, 0x44);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.7));
    w.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    w.hovered.rounding = Rounding::same(8.0);
    w.hovered.expansion = 0.0;

    w.active.bg_fill = Color32::from_rgb(0x30, 0x3a, 0x55);
    w.active.weak_bg_fill = Color32::from_rgb(0x30, 0x3a, 0x55);
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    w.active.rounding = Rounding::same(8.0);
    w.active.expansion = 0.0;

    w.open = w.hovered;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(12.0, 6.0);
        s.spacing.interact_size.y = 30.0;
        s.spacing.combo_height = 320.0;
        s.text_styles = [
            (TextStyle::Heading, bold(20.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(11.5)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
    });
}

/// Fonte Segoe UI Semibold (ou a normal, se não existir).
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

/// Título pequeno em maiúsculas, para separar seções.
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).font(bold(11.0)).color(TEXT_FAINT));
}

pub fn card() -> Frame {
    Frame::none()
        .fill(SURFACE)
        .rounding(10.0)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .inner_margin(Margin::same(12.0))
}

/// Botão de ação principal, em azul.
pub fn primary(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).font(bold(14.0)).color(Color32::WHITE))
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .rounding(8.0)
        .min_size(vec2(0.0, 34.0))
}

/// Etiqueta arredondada (ex.: "EM USO").
pub fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    Frame::none()
        .fill(color.gamma_multiply(0.18))
        .rounding(99.0)
        .inner_margin(Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(bold(10.5)).color(color));
        });
}

/// Abas em formato de "pílulas"; retorna `true` se a seleção mudou.
pub fn tabs<T: PartialEq + Copy>(ui: &mut egui::Ui, current: &mut T, items: &[(T, &str)]) -> bool {
    let mut changed = false;
    Frame::none()
        .fill(BG)
        .rounding(10.0)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .inner_margin(Margin::same(4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for &(value, label) in items {
                    let selected = *current == value;
                    let text = RichText::new(label).font(bold(13.5)).color(if selected { Color32::WHITE } else { TEXT_DIM });
                    let button = egui::Button::new(text)
                        .fill(if selected { ACCENT } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .rounding(7.0)
                        .min_size(vec2(0.0, 30.0));
                    if ui.add(button).clicked() && !selected {
                        *current = value;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Segoe UI tem as setas e símbolos que faltam nas fontes padrão do egui.
fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let dir = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or("C:\\Windows".into())).join("Fonts");
    let mut load = |name: &str, file: &str| {
        let bytes = std::fs::read(dir.join(file)).ok()?;
        fonts.font_data.insert(name.to_owned(), FontData::from_owned(bytes));
        Some(name.to_owned())
    };
    let regular = load("segoe", "segoeui.ttf");
    let semibold = load("segoe-semibold", "seguisb.ttf");
    let symbols = load("segoe-symbol", "seguisym.ttf");

    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    for name in [regular.clone(), symbols.clone()].into_iter().flatten().rev() {
        proportional.insert(0, name);
    }
    let fallback = proportional.clone();
    let bold_family: Vec<String> = semibold.into_iter().chain(fallback).collect();
    fonts.families.insert(FontFamily::Name("bold".into()), bold_family);
    fonts
}
