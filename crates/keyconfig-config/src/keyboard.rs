//! Desenho do teclado clicável.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke};
use keyconfig_core::geometry::{self, KeyRect};
use keyconfig_core::{Action, KeyCode, Language, Profile};

pub const ACCENT: Color32 = Color32::from_rgb(0x4c, 0x9a, 0xff);
const KEY_FILL: Color32 = Color32::from_gray(46);
const KEY_HOVER: Color32 = Color32::from_gray(60);
const KEY_MAPPED: Color32 = Color32::from_rgb(0x1f, 0x45, 0x7a);
const KEY_STROKE: Color32 = Color32::from_gray(78);
const SELECTED: Color32 = Color32::from_rgb(0xff, 0xc4, 0x3d);
const PRESSED: Color32 = Color32::from_rgb(0x2e, 0xd8, 0xa3);
const FN_FILL: Color32 = Color32::from_gray(30);

/// Resumo curto do que a tecla faz, para caber nela ("Esc", "Ctrl+C", "Macro"...).
pub fn summary(action: &Action, language: Language) -> String {
    match action {
        Action::Key { to } => match geometry::label(*to, language) {
            "" => "Espaço".to_owned(),
            label => label.to_owned(),
        },
        Action::Disabled => "Desligada".to_owned(),
        Action::Shortcut { keys } => keys
            .iter()
            .map(|&k| geometry::label(k, language))
            .collect::<Vec<_>>()
            .join("+"),
        Action::Macro { .. } => "Macro".to_owned(),
        // Um caractere só (ex.: "~") aparece na tecla em vez de "Texto".
        Action::Text { text } if text.chars().count() == 1 => text.clone(),
        Action::Text { .. } => "Texto".to_owned(),
        Action::DeadKey { accent } => accent.to_string(),
        Action::Run { .. } => "Abrir".to_owned(),
    }
}

/// Desenha o teclado do perfil, com as teclas físicas em `pressed` acesas,
/// e retorna a tecla clicada, se houver.
pub fn show(
    ui: &mut egui::Ui,
    profile: &Profile,
    selected: Option<KeyCode>,
    pressed: &[KeyCode],
) -> Option<KeyCode> {
    let keys = geometry::keys(profile.layout, profile.language);
    let fn_slots = geometry::fn_slots(profile.layout, profile.language);
    let width_u = keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
    let height_u = keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);

    let avail = ui.available_size();
    let unit = (avail.x / width_u).min(avail.y / height_u).clamp(24.0, 58.0);
    let size = vec2(width_u * unit, height_u * unit);
    let left_pad = ((avail.x - size.x) / 2.0).max(0.0);

    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.add_space(left_pad);
        let (response, painter) = ui.allocate_painter(size, Sense::click());
        let origin = response.rect.min;
        let hover = response.hover_pos();

        let to_rect = |x: f32, y: f32, w: f32, h: f32| {
            Rect::from_min_size(origin + vec2(x * unit, y * unit), vec2(w * unit, h * unit)).shrink(2.0)
        };
        let rects = |k: &KeyRect| {
            let mut r = vec![to_rect(k.x, k.y, k.w, k.h)];
            if let Some([x, y, w, h]) = k.extra {
                r.push(to_rect(x, y, w, h));
            }
            r
        };

        for key in &keys {
            let parts = rects(key);
            let hovered = hover.is_some_and(|p| parts.iter().any(|r| r.contains(p)));
            let action = profile.mappings.get(&key.code);
            let is_pressed = pressed.contains(&key.code);
            let fill = match (action.is_some(), hovered) {
                _ if is_pressed => PRESSED.gamma_multiply(0.55),
                (true, false) => KEY_MAPPED,
                (true, true) => KEY_MAPPED.gamma_multiply(1.3),
                (false, true) => KEY_HOVER,
                (false, false) => KEY_FILL,
            };
            let stroke = if selected == Some(key.code) {
                Stroke::new(2.5_f32, SELECTED)
            } else if is_pressed {
                Stroke::new(2.0_f32, PRESSED)
            } else {
                Stroke::new(1.0_f32, KEY_STROKE)
            };
            // Brilho em volta da tecla apertada.
            if is_pressed {
                for (grow, alpha) in [(7.0, 0.12), (4.0, 0.25)] {
                    for r in &parts {
                        painter.rect_filled(r.expand(grow), 8.0, PRESSED.gamma_multiply(alpha));
                    }
                }
            }
            if let [top, bottom] = parts[..] {
                // Enter em "L": preenche as duas partes juntas e contorna o formato inteiro.
                let bottom = Rect::from_min_max(pos2(bottom.left(), top.top() + 1.0), bottom.max);
                painter.rect_filled(top, 5.0, fill);
                painter.rect_filled(bottom, 5.0, fill);
                let outline = vec![
                    top.left_top(),
                    top.right_top(),
                    bottom.right_bottom(),
                    bottom.left_bottom(),
                    pos2(bottom.left(), top.bottom()),
                    top.left_bottom(),
                ];
                painter.add(egui::Shape::closed_line(outline, stroke));
            } else {
                painter.rect(parts[0], 5.0, fill, stroke);
            }

            let main = parts[0];
            let label = geometry::label(key.code, profile.language);
            let font = if label.chars().count() > 3 { unit * 0.24 } else { unit * 0.32 };
            match action {
                Some(action) => {
                    painter.text(
                        pos2(main.center().x, main.top() + main.height() * 0.32),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(font * 0.85),
                        Color32::from_gray(170),
                    );
                    painter.text(
                        pos2(main.center().x, main.top() + main.height() * 0.7),
                        Align2::CENTER_CENTER,
                        fit(&summary(action, profile.language), main.width(), unit * 0.22),
                        FontId::proportional(unit * 0.22),
                        Color32::WHITE,
                    );
                }
                None => {
                    painter.text(
                        main.center(),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(font),
                        Color32::from_gray(225),
                    );
                }
            }

            if hovered && response.clicked() {
                clicked = Some(key.code);
            }
        }

        // Fn: só referência, porque o teclado trata essa tecla sozinho e ela não chega ao Windows.
        let mut over_fn = false;
        for &[x, y, w, h] in &fn_slots {
            let r = to_rect(x, y, w, h);
            over_fn |= hover.is_some_and(|p| r.contains(p));
            painter.rect(r, 5.0, FN_FILL, Stroke::new(1.0_f32, Color32::from_gray(55)));
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                "Fn",
                FontId::proportional(unit * 0.3),
                Color32::from_gray(95),
            );
        }

        if over_fn {
            response.on_hover_text_at_pointer(
                "A tecla Fn é tratada pelo próprio teclado e nunca chega ao Windows, \
                 então nenhum programa consegue remapeá-la.\n\
                 As combinações com Fn (ex.: Fn + F1) chegam como teclas normais e podem ser mapeadas.",
            );
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    });
    clicked
}

/// Corta textos que não cabem na tecla.
fn fit(text: &str, width: f32, font: f32) -> String {
    let max = ((width / (font * 0.55)) as usize).max(3);
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let cut: String = text.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}
