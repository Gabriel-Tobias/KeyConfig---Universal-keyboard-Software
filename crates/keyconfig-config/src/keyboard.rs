//! Desenho do teclado clicável, no estilo do redesign: teclas com borda inferior
//! mais grossa, legendas em JetBrains Mono e destaque âmbar.

use eframe::egui::{self, pos2, vec2, Align2, Color32, Painter, Rect, Sense, Shadow, Shape, Stroke};
use keyconfig_core::geometry::{self, KeyRect};
use keyconfig_core::{Action, KeyCode, Language, Profile};

use crate::theme::{self, ACCENT, BORDER, SIDEBAR, TEXT, TEXT_2, TEXT_MUTED};

/// Distância entre teclas de 1u no tamanho natural (50 px de tecla + 6 de espaço).
const PITCH: f32 = 56.0;
const GAP: f32 = 6.0;
const CASE_PAD: f32 = 20.0;

const KEY: Color32 = Color32::from_rgb(0x20, 0x25, 0x2f);
const KEY_HOVER: Color32 = Color32::from_rgb(0x29, 0x30, 0x3c);
const KEY_BORDER: Color32 = Color32::from_rgb(0x2e, 0x35, 0x42);
const KEY_BORDER_HOVER: Color32 = Color32::from_rgb(0x3a, 0x43, 0x52);
const MOD: Color32 = Color32::from_rgb(0x1b, 0x20, 0x29);
const REMAP: Color32 = Color32::from_rgb(0x2a, 0x24, 0x18);
const REMAP_BORDER: Color32 = Color32::from_rgb(0x5a, 0x47, 0x20);
const REMAP_BASE: Color32 = Color32::from_rgb(0x9a, 0x84, 0x58);
const SELECTED: Color32 = Color32::from_rgb(0x2e, 0x27, 0x16);
const OFF: Color32 = Color32::from_rgb(0x15, 0x18, 0x1e);
pub const PRESSED: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);

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

/// Teclas com nome por extenso, desenhadas mais discretas (como no redesign).
fn is_modifier(code: KeyCode) -> bool {
    use KeyCode::*;
    matches!(
        code,
        Tab | CapsLock | ShiftLeft | ShiftRight | ControlLeft | ControlRight | MetaLeft | MetaRight
            | AltLeft | AltRight | ContextMenu | Backspace | Enter | NumpadEnter | PrintScreen
            | ScrollLock | Pause | Insert | Home | End | PageUp | PageDown | NumLock
    )
}

/// Largura natural (escala 1) do teclado com o gabinete.
pub fn natural_width(profile: &Profile) -> f32 {
    let keys = geometry::keys(profile.layout, profile.language);
    let width_u = keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
    width_u * PITCH - GAP + 2.0 * CASE_PAD
}

/// Desenha o teclado do perfil, com as teclas físicas em `pressed` acesas,
/// e retorna a tecla clicada, se houver.
pub fn show(
    ui: &mut egui::Ui,
    profile: &Profile,
    selected: Option<KeyCode>,
    pressed: &[KeyCode],
    max_width: f32,
) -> Option<KeyCode> {
    let keys = geometry::keys(profile.layout, profile.language);
    let fn_slots = geometry::fn_slots(profile.layout, profile.language);
    let width_u = keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
    let height_u = keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);

    let scale = (max_width / natural_width(profile)).clamp(0.45, 1.1);
    let pitch = PITCH * scale;
    let gap = GAP * scale;
    let pad = CASE_PAD * scale;
    let size = vec2(width_u * pitch - gap + 2.0 * pad, height_u * pitch - gap + 2.0 * pad);

    let (response, painter) = ui.allocate_painter(size, Sense::click());
    let case = response.rect;
    painter.add(Shadow { offset: vec2(0.0, 24.0), blur: 60.0, spread: 0.0, color: Color32::from_black_alpha(90) }.as_shape(case, 18.0));
    painter.rect(case, 18.0, SIDEBAR, Stroke::new(1.0_f32, BORDER));

    let origin = case.min + vec2(pad, pad);
    let to_rect = |x: f32, y: f32, w: f32, h: f32| {
        Rect::from_min_size(origin + vec2(x * pitch, y * pitch), vec2(w * pitch - gap, h * pitch - gap))
    };
    let hover = response.hover_pos();
    let mut clicked = None;

    for key in &keys {
        let mut parts = vec![to_rect(key.x, key.y, key.w, key.h)];
        if let Some([x, y, w, h]) = key.extra {
            parts.push(to_rect(x, y, w, h));
        }
        let hovered = hover.is_some_and(|p| parts.iter().any(|r| r.contains(p)));
        let action = profile.mappings.get(&key.code);
        let state = KeyState {
            hovered,
            selected: selected == Some(key.code),
            pressed: pressed.contains(&key.code),
            modifier: is_modifier(key.code),
            action,
        };
        draw_key(&painter, key, &parts, &state, profile.language, scale);
        if hovered && response.clicked() {
            clicked = Some(key.code);
        }
    }

    // Fn: só referência, porque o teclado trata essa tecla sozinho e ela não chega ao Windows.
    let mut over_fn = false;
    for &[x, y, w, h] in &fn_slots {
        let r = to_rect(x, y, w, h);
        over_fn |= hover.is_some_and(|p| r.contains(p));
        let face = keycap(&painter, &[r], OFF, KEY_BORDER, scale);
        painter.text(face.center(), Align2::CENTER_CENTER, "Fn", theme::medium(12.0 * scale), TEXT_MUTED);
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
    clicked
}

struct KeyState<'a> {
    hovered: bool,
    selected: bool,
    pressed: bool,
    modifier: bool,
    action: Option<&'a Action>,
}

fn draw_key(painter: &Painter, key: &KeyRect, parts: &[Rect], s: &KeyState, language: Language, scale: f32) {
    let off = s.action == Some(&Action::Disabled);
    let remapped = s.action.is_some() && !off;

    let (mut fill, mut border) = match () {
        _ if s.selected => (SELECTED, ACCENT),
        _ if remapped => (REMAP, REMAP_BORDER),
        _ if off => (OFF, KEY_BORDER),
        _ if s.modifier => (MOD, KEY_BORDER),
        _ => (KEY, KEY_BORDER),
    };
    if s.hovered && !s.selected {
        fill = if remapped { lighten(REMAP, 0.06) } else { KEY_HOVER };
        border = if remapped { REMAP_BORDER } else { KEY_BORDER_HOVER };
    }
    if s.pressed {
        fill = PRESSED.gamma_multiply(0.22);
        border = PRESSED;
    }

    // Anel de seleção em volta da tecla (box-shadow 0 0 0 3px).
    if s.selected {
        for r in parts {
            painter.rect_filled(r.expand(3.0 * scale), 11.0 * scale, ACCENT.gamma_multiply(0.28));
        }
    }

    // Apertada: desce 1 px, como o :active do redesign.
    let parts: Vec<Rect> = if s.pressed {
        parts.iter().map(|r| r.translate(vec2(0.0, 1.0))).collect()
    } else {
        parts.to_vec()
    };
    let face = if off {
        let face = keycap(painter, &parts, fill, fill, scale);
        for r in &parts {
            dashed_rect(painter, *r, Stroke::new(1.0_f32, KEY_BORDER_HOVER));
        }
        face
    } else {
        keycap(painter, &parts, fill, border, scale)
    };

    let label = match geometry::label(key.code, language) {
        "" => "Espaço",
        l => l,
    };
    if key.code == KeyCode::Space && !remapped {
        return;
    }

    if remapped {
        let action = s.action.expect("remapeada tem ação");
        painter.text(
            face.center() - vec2(0.0, 7.0 * scale),
            Align2::CENTER_CENTER,
            label,
            theme::medium(9.0 * scale),
            REMAP_BASE,
        );
        let target = summary(action, language);
        painter.text(
            face.center() + vec2(0.0, 6.0 * scale),
            Align2::CENTER_CENTER,
            fit(&target, face.width(), 12.0 * scale * 0.62),
            theme::mono(12.0 * scale),
            ACCENT,
        );
        // Pontinho âmbar no canto, marcando que a tecla foi remapeada.
        painter.circle_filled(pos2(parts[0].right() - 7.5 * scale, parts[0].top() + 7.5 * scale), 2.5 * scale, ACCENT);
        return;
    }

    let color = if off { TEXT_MUTED } else if s.modifier { TEXT_2 } else { TEXT };
    let font = if s.modifier { theme::medium(12.0 * scale) } else { theme::mono(14.0 * scale) };
    painter.text(face.center(), Align2::CENTER_CENTER, label, font, color);
}

/// Tecla com borda de 1 px e 3 px embaixo; retorna a área interna (onde vão as legendas).
fn keycap(painter: &Painter, parts: &[Rect], fill: Color32, border: Color32, scale: f32) -> Rect {
    let radius = 9.0 * scale;
    let bottom = (3.0 * scale).max(2.0);
    // Enter em "L": a parte de baixo sobe até encostar na de cima, sem emenda.
    let parts: Vec<Rect> = match parts {
        [top, low] => vec![*top, Rect::from_min_max(pos2(low.left(), top.top() + radius * 2.0), low.max)],
        _ => parts.to_vec(),
    };
    for r in &parts {
        painter.rect_filled(*r, radius, border);
    }
    let inner: Vec<Rect> = parts
        .iter()
        .map(|r| Rect::from_min_max(r.min + vec2(1.0, 1.0), r.max - vec2(1.0, bottom)))
        .collect();
    for r in &inner {
        painter.rect_filled(*r, radius - 1.0, fill);
    }
    if let [top, low] = inner[..] {
        // Cobre a borda que sobra entre as duas partes do Enter.
        painter.rect_filled(Rect::from_min_max(pos2(low.left(), top.bottom() - radius), pos2(low.right(), top.bottom() + 2.0)), 0.0, fill);
    }
    inner[0]
}

fn dashed_rect(painter: &Painter, r: Rect, stroke: Stroke) {
    let points = vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(Shape::dashed_line(&points, stroke, 4.0, 3.0));
}

fn lighten(c: Color32, amount: f32) -> Color32 {
    let l = |v: u8| (v as f32 + (255.0 - v as f32) * amount) as u8;
    Color32::from_rgb(l(c.r()), l(c.g()), l(c.b()))
}

/// Corta textos que não cabem na tecla (`char_width` é a largura média de um caractere).
fn fit(text: &str, width: f32, char_width: f32) -> String {
    let max = ((width - 6.0) / char_width).max(3.0) as usize;
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let cut: String = text.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}
