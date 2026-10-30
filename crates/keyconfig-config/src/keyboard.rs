//! Desenho do teclado clicável: gabinete com sombra e teclas em 3D
//! (face clara sobre uma "saia" mais escura), que afundam e brilham ao apertar.

use eframe::egui::{self, pos2, vec2, Align2, Color32, Painter, Rect, Sense, Shadow, Stroke};
use keyconfig_core::geometry::{self, KeyRect};
use keyconfig_core::{Action, KeyCode, Language, Profile};

use crate::theme::{self, ACCENT, BORDER, CYAN};

pub const MAPPED: Color32 = Color32::from_rgb(0x1d, 0x4e, 0xa8);
pub const PRESSED: Color32 = Color32::from_rgb(0x2e, 0xd8, 0xa3);
const FACE: Color32 = Color32::from_rgb(0x2a, 0x30, 0x40);
const SKIRT: Color32 = Color32::from_rgb(0x1a, 0x1e, 0x29);
const LEGEND: Color32 = Color32::from_rgb(0xdc, 0xe1, 0xec);

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

struct Look {
    face: Color32,
    skirt: Color32,
    pressed: bool,
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

    // Margem do gabinete em volta das teclas, em unidades.
    const PAD: f32 = 0.45;
    let avail = ui.available_size();
    let unit = (avail.x / (width_u + 2.0 * PAD))
        .min(avail.y / (height_u + 2.0 * PAD))
        .clamp(22.0, 62.0);
    let size = vec2((width_u + 2.0 * PAD) * unit, (height_u + 2.0 * PAD) * unit);
    let left_pad = ((avail.x - size.x) / 2.0).max(0.0);

    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.add_space(left_pad);
        let (response, painter) = ui.allocate_painter(size, Sense::click());
        draw_case(&painter, response.rect, unit);

        let origin = response.rect.min + vec2(PAD * unit, PAD * unit);
        let hover = response.hover_pos();
        let gap = (unit * 0.06).max(2.0);
        let to_rect = |x: f32, y: f32, w: f32, h: f32| {
            Rect::from_min_size(origin + vec2(x * unit, y * unit), vec2(w * unit, h * unit)).shrink(gap)
        };

        let mut glows: Vec<(Vec<Rect>, Color32)> = Vec::new();
        for key in &keys {
            let mut parts = vec![to_rect(key.x, key.y, key.w, key.h)];
            if let Some([x, y, w, h]) = key.extra {
                parts.push(to_rect(x, y, w, h));
            }
            let hovered = hover.is_some_and(|p| parts.iter().any(|r| r.contains(p)));
            let action = profile.mappings.get(&key.code);
            let is_pressed = pressed.contains(&key.code);

            let look = match () {
                _ if is_pressed => Look { face: PRESSED.gamma_multiply(0.75), skirt: PRESSED.gamma_multiply(0.35), pressed: true },
                _ if action.is_some() => Look { face: MAPPED, skirt: Color32::from_rgb(0x12, 0x2c, 0x63), pressed: false },
                _ => Look { face: FACE, skirt: SKIRT, pressed: false },
            };
            let look = if hovered && !is_pressed {
                Look { face: lighten(look.face, 0.12), ..look }
            } else {
                look
            };
            let face = draw_keycap(&painter, &parts, &look, unit);

            draw_legends(&painter, key, face, action, profile.language, unit);

            if selected == Some(key.code) {
                glows.push((parts.clone(), CYAN));
            } else if is_pressed {
                glows.push((parts.clone(), PRESSED));
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
            let look = Look { face: Color32::from_rgb(0x17, 0x1b, 0x25), skirt: Color32::from_rgb(0x10, 0x13, 0x1a), pressed: false };
            let face = draw_keycap(&painter, &[r], &look, unit);
            painter.text(face.center(), Align2::CENTER_CENTER, "Fn", theme::bold(unit * 0.26), theme::TEXT_FAINT);
        }

        // Brilho por cima de tudo, para "vazar" sobre as teclas vizinhas como luz.
        for (parts, color) in glows {
            for r in &parts {
                for (grow, alpha) in [(1.0, 1.0), (3.5, 0.35), (7.0, 0.12)] {
                    let width: f32 = if grow == 1.0 { 2.0 } else { 3.0 };
                    painter.rect_stroke(r.expand(grow), 7.0 + grow, Stroke::new(width, color.gamma_multiply(alpha)));
                }
            }
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

fn draw_case(painter: &Painter, rect: Rect, unit: f32) {
    let radius = unit * 0.35;
    let shadow = Shadow { offset: vec2(0.0, 14.0), blur: 40.0, spread: 0.0, color: Color32::from_black_alpha(170) };
    painter.add(shadow.as_shape(rect, radius));
    painter.rect(rect, radius, Color32::from_rgb(0x13, 0x17, 0x20), Stroke::new(1.0_f32, BORDER));
    // Fio de luz azul na borda de cima, lembrando o logo.
    let top = Rect::from_min_size(rect.min + vec2(radius, 0.0), vec2(rect.width() - 2.0 * radius, 1.5));
    painter.rect_filled(top, 0.0, ACCENT.gamma_multiply(0.45));
    // Placa interna um pouco mais escura, onde ficam as teclas.
    let plate = rect.shrink(unit * 0.22);
    painter.rect_filled(plate, radius * 0.7, Color32::from_rgb(0x0c, 0x0f, 0x15));
}

/// Desenha saia e face; retorna o retângulo da face (onde vão as legendas).
fn draw_keycap(painter: &Painter, parts: &[Rect], look: &Look, unit: f32) -> Rect {
    let r = unit * 0.11;
    let side = unit * 0.07;
    let bottom = unit * 0.14;

    // Enter em "L": a parte de baixo sobe até encostar na de cima, sem emenda.
    let bases: Vec<Rect> = match parts {
        [top, low] => vec![*top, Rect::from_min_max(pos2(low.left(), top.top() + unit * 0.3), low.max)],
        _ => parts.to_vec(),
    };
    for b in &bases {
        painter.rect_filled(*b, r, look.skirt);
    }

    let face_of = |b: &Rect| {
        if look.pressed {
            // Afundada: a face cobre quase toda a saia, um pouco mais para baixo.
            Rect::from_min_max(b.min + vec2(side * 0.6, bottom * 0.5), b.max - vec2(side * 0.6, side * 0.3))
        } else {
            Rect::from_min_max(b.min + vec2(side, 1.0), b.max - vec2(side, bottom))
        }
    };
    let faces: Vec<Rect> = bases.iter().map(face_of).collect();
    for f in &faces {
        painter.rect_filled(*f, r * 0.8, look.face);
        // Reflexo sutil no topo da face.
        let shine = Rect::from_min_size(f.min + vec2(r, 1.0), vec2(f.width() - 2.0 * r, 1.0));
        painter.rect_filled(shine, 0.0, Color32::from_white_alpha(18));
    }
    faces[0]
}

fn draw_legends(
    painter: &Painter,
    key: &KeyRect,
    face: Rect,
    action: Option<&Action>,
    language: Language,
    unit: f32,
) {
    let label = geometry::label(key.code, language);
    let long = label.chars().count() > 3;
    match action {
        Some(action) => {
            // Legenda original pequena no canto e a nova função grande no centro.
            painter.text(
                face.left_top() + vec2(unit * 0.09, unit * 0.05),
                Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(unit * 0.19),
                Color32::from_rgb(0x9f, 0xbd, 0xf5),
            );
            let text = summary(action, language);
            let size = if text.chars().count() > 3 { unit * 0.22 } else { unit * 0.3 };
            painter.text(
                face.center() + vec2(0.0, unit * 0.08),
                Align2::CENTER_CENTER,
                fit(&text, face.width(), size),
                theme::bold(size),
                Color32::WHITE,
            );
        }
        None => {
            let size = if long { unit * 0.21 } else { unit * 0.3 };
            painter.text(face.center(), Align2::CENTER_CENTER, label, theme::bold(size), LEGEND);
        }
    }
}

fn lighten(c: Color32, amount: f32) -> Color32 {
    let l = |v: u8| (v as f32 + (255.0 - v as f32) * amount) as u8;
    Color32::from_rgb(l(c.r()), l(c.g()), l(c.b()))
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
