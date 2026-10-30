//! Tema visual, seguindo o redesign em `examples/` (tons grafite com destaque âmbar).
//!
//! Inter e JetBrains Mono vão embutidas no executável (licença OFL, em `assets/fonts`);
//! os ícones dos botões vêm da Segoe Fluent Icons, que já existe no Windows.

use eframe::egui::{
    self, vec2, Color32, FontData, FontDefinitions, FontFamily, FontId, Response,
    RichText, Rounding, Sense, Shadow, Stroke, TextStyle, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(0x0e, 0x10, 0x14);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x12, 0x15, 0x1b);
pub const CARD: Color32 = Color32::from_rgb(0x17, 0x1b, 0x22);
pub const BORDER: Color32 = Color32::from_rgb(0x1f, 0x24, 0x2d);
pub const DIVIDER: Color32 = Color32::from_rgb(0x22, 0x28, 0x33);
pub const CONTROL: Color32 = Color32::from_rgb(0x1e, 0x23, 0x2c);
pub const CONTROL_HOVER: Color32 = Color32::from_rgb(0x26, 0x2c, 0x37);
pub const CONTROL_BORDER: Color32 = Color32::from_rgb(0x2e, 0x35, 0x42);
pub const FIELD: Color32 = Color32::from_rgb(0x12, 0x15, 0x1b);
pub const CHIP: Color32 = Color32::from_rgb(0x1b, 0x20, 0x29);
pub const CHIP_BORDER: Color32 = Color32::from_rgb(0x2a, 0x30, 0x3b);

pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xea, 0xee);
pub const TEXT_2: Color32 = Color32::from_rgb(0xae, 0xb6, 0xc4);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9a, 0xa3, 0xb2);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x7a, 0x83, 0x94);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x5c, 0x64, 0x72);

pub const ACCENT: Color32 = Color32::from_rgb(0xf2, 0xb5, 0x44);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xff, 0xc6, 0x5c);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x14, 0x11, 0x0a);
pub const ACCENT_BG: Color32 = Color32::from_rgb(0x2a, 0x24, 0x18);
pub const OK: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
pub const DANGER: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
pub const DANGER_TEXT: Color32 = Color32::from_rgb(0xff, 0x8a, 0x8d);

/// Glifos da Segoe Fluent Icons usados nos botões.
pub mod icon {
    pub const ADD: &str = "\u{E710}";
    pub const COPY: &str = "\u{E8C8}";
    pub const DELETE: &str = "\u{E74D}";
    pub const TARGET: &str = "\u{E7C8}";
    pub const SEARCH: &str = "\u{E721}";
    pub const POWER: &str = "\u{E7E8}";
    pub const RESET: &str = "\u{E777}";
    pub const ARROW: &str = "\u{E72A}";
    pub const KEYBOARD: &str = "\u{E765}";
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());

    let mut v = Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = FIELD;
    v.faint_bg_color = CARD;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(14.0);
    v.menu_rounding = Rounding::same(8.0);
    v.window_shadow = Shadow { offset: vec2(0.0, 24.0), blur: 60.0, spread: 0.0, color: Color32::from_black_alpha(110) };
    v.popup_shadow = Shadow { offset: vec2(0.0, 10.0), blur: 24.0, spread: 0.0, color: Color32::from_black_alpha(110) };
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.text_cursor.stroke = Stroke::new(2.0_f32, ACCENT);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = CARD;
    w.noninteractive.weak_bg_fill = CARD;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    w.noninteractive.rounding = Rounding::same(8.0);

    w.inactive.bg_fill = FIELD;
    w.inactive.weak_bg_fill = CONTROL;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, CONTROL_BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.inactive.rounding = Rounding::same(8.0);

    w.hovered.bg_fill = FIELD;
    w.hovered.weak_bg_fill = CONTROL_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x3a, 0x43, 0x52));
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.hovered.rounding = Rounding::same(8.0);
    w.hovered.expansion = 0.0;

    w.active.bg_fill = FIELD;
    w.active.weak_bg_fill = Color32::from_rgb(0x2a, 0x30, 0x3b);
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.active.rounding = Rounding::same(8.0);
    w.active.expansion = 0.0;

    w.open = w.hovered;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(14.0, 8.0);
        s.spacing.interact_size.y = 36.0;
        s.spacing.combo_height = 320.0;
        s.spacing.text_edit_width = 180.0;
        s.text_styles = [
            (TextStyle::Heading, semibold(16.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, medium(13.0)),
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Monospace, mono_medium(13.0)),
        ]
        .into();
    });
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

/// JetBrains Mono SemiBold, usada nas legendas das teclas.
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono".into()))
}

pub fn mono_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono-medium".into()))
}

pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("icons".into()))
}

/// Título pequeno em maiúsculas, para separar seções.
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).font(semibold(12.0)).color(TEXT_FAINT));
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// Botão comum, com fundo.
    Normal,
    /// Ação principal, em âmbar.
    Primary,
    /// Sem fundo, só borda.
    Ghost,
    /// Sem fundo; fica vermelho ao passar o mouse.
    Danger,
}

/// Botão no estilo do redesign, com ícone opcional à esquerda e texto extra à direita.
pub fn button(ui: &mut egui::Ui, kind: Kind, icon: Option<&str>, text: &str, enabled: bool) -> Response {
    button_with(ui, kind, icon, text, None, enabled, 0.0)
}

pub fn button_with(
    ui: &mut egui::Ui,
    kind: Kind,
    icon: Option<&str>,
    text: &str,
    hint: Option<&str>,
    enabled: bool,
    min_width: f32,
) -> Response {
    let height = 36.0;
    let font = if kind == Kind::Primary { semibold(13.0) } else { medium(13.0) };
    let painter = ui.painter();
    let text_galley = painter.layout_no_wrap(text.to_owned(), font, Color32::WHITE);
    let icon_galley = icon.map(|i| painter.layout_no_wrap(i.to_owned(), icon_font(13.0), Color32::WHITE));
    let hint_galley = hint.map(|h| painter.layout_no_wrap(h.to_owned(), mono_medium(11.0), Color32::WHITE));

    let gap = 8.0;
    let mut content = text_galley.size().x;
    if let Some(g) = &icon_galley {
        content += g.size().x + gap;
    }
    if let Some(g) = &hint_galley {
        content += g.size().x + gap;
    }
    let width = (content + 28.0).max(min_width);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
    let hovered = enabled && response.hovered();

    let (fill, stroke, fg) = match kind {
        Kind::Primary if !enabled => (ACCENT.gamma_multiply(0.35), ACCENT.gamma_multiply(0.0), ON_ACCENT.gamma_multiply(0.7)),
        Kind::Primary => (if hovered { ACCENT_HOVER } else { ACCENT }, Color32::TRANSPARENT, ON_ACCENT),
        Kind::Normal => (if hovered { CONTROL_HOVER } else { CONTROL }, CONTROL_BORDER, TEXT),
        Kind::Ghost => (if hovered { CONTROL_HOVER } else { Color32::TRANSPARENT }, CONTROL_BORDER, TEXT),
        Kind::Danger if hovered => (Color32::TRANSPARENT, DANGER, DANGER_TEXT),
        Kind::Danger => (Color32::TRANSPARENT, CONTROL_BORDER, TEXT),
    };
    let fg = if enabled || kind == Kind::Primary { fg } else { TEXT_FAINT };

    let painter = ui.painter();
    painter.rect(rect, 8.0, fill, Stroke::new(1.0_f32, stroke));
    let mut x = rect.center().x - content / 2.0;
    let cy = rect.center().y;
    if let Some(g) = icon_galley {
        painter.galley(egui::pos2(x, cy - g.size().y / 2.0), g.clone(), fg);
        x += g.size().x + gap;
    }
    painter.galley(egui::pos2(x, cy - text_galley.size().y / 2.0), text_galley.clone(), fg);
    x += text_galley.size().x + gap;
    if let Some(g) = hint_galley {
        painter.galley(egui::pos2(x, cy - g.size().y / 2.0), g, fg.gamma_multiply(0.7));
    }
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Botão quadrado só com ícone (ex.: subir/descer/remover passo de macro).
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str, enabled: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 32.0), sense);
    let hovered = enabled && response.hovered();
    let fill = if hovered { CONTROL_HOVER } else { Color32::TRANSPARENT };
    let color = if !enabled { TEXT_MUTED } else if hovered { TEXT } else { TEXT_DIM };
    ui.painter().rect(rect, 8.0, fill, Stroke::new(1.0_f32, CONTROL_BORDER));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, glyph, icon_font(12.0), color);
    response.on_hover_text(tooltip)
}

/// Pílula clicável (categorias e modificadores da combinação).
pub fn chip_toggle(ui: &mut egui::Ui, text: &str, extra: Option<&str>, on: bool) -> Response {
    let painter = ui.painter();
    let fg = if on { ON_ACCENT } else { TEXT_2 };
    let main = painter.layout_no_wrap(text.to_owned(), medium(13.0), fg);
    let extra = extra.map(|e| painter.layout_no_wrap(e.to_owned(), FontId::proportional(11.0), fg.gamma_multiply(0.7)));
    let content = main.size().x + extra.as_ref().map_or(0.0, |e| e.size().x + 8.0);
    let (rect, response) = ui.allocate_exact_size(vec2(content + 24.0, 32.0), Sense::click());
    let hovered = response.hovered();
    let (fill, stroke) = if on {
        (ACCENT, ACCENT)
    } else if hovered {
        (Color32::TRANSPARENT, Color32::from_rgb(0x3a, 0x43, 0x52))
    } else {
        (Color32::TRANSPARENT, CHIP_BORDER)
    };
    let painter = ui.painter();
    painter.rect(rect, 99.0, fill, Stroke::new(1.0_f32, stroke));
    let mut x = rect.left() + 12.0;
    let main_color = if on { ON_ACCENT } else if hovered { TEXT } else { TEXT_2 };
    painter.galley(egui::pos2(x, rect.center().y - main.size().y / 2.0), main.clone(), main_color);
    x += main.size().x + 8.0;
    if let Some(e) = extra {
        painter.galley(egui::pos2(x, rect.center().y - e.size().y / 2.0), e, fg.gamma_multiply(0.7));
    }
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Grupo encostado à direita (os itens são adicionados da direita para a esquerda).
/// Dentro de um `horizontal_wrapped`, se não couber na linha atual ele desce para
/// a próxima em vez de ficar por cima do que veio antes.
pub fn trailing(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    wrap_if(ui, width);
    // Desconta o espaçamento que o layout põe antes do item, senão ele não cabe e quebra a linha.
    let free = remaining_width(ui) - ui.spacing().item_spacing.x;
    ui.allocate_ui_with_layout(
        vec2(free.max(0.0), ui.spacing().interact_size.y),
        egui::Layout::right_to_left(egui::Align::Center),
        add,
    );
}

/// Espaço que ainda sobra na linha atual. Em layouts com quebra de linha,
/// `available_width()` devolve a largura inteira da linha, não o que sobra nela.
fn remaining_width(ui: &egui::Ui) -> f32 {
    ui.available_rect_before_wrap().width()
}

/// Dentro de um `horizontal_wrapped`, passa para a próxima linha se não couberem
/// `width` pontos. Necessário para grupos (`ui.horizontal`), cuja largura o egui
/// só conhece depois de desenhar.
pub fn wrap_if(ui: &mut egui::Ui, width: f32) {
    if remaining_width(ui) < width {
        ui.end_row();
    }
}

/// Rótulo à esquerda de um campo, como "Nome [___]".
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(13.0).color(TEXT_DIM));
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let mut add_static = |name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.to_owned(), FontData::from_static(bytes));
    };
    add_static("inter", include_bytes!("../../../assets/fonts/Inter-Regular.ttf"));
    add_static("inter-medium", include_bytes!("../../../assets/fonts/Inter-Medium.ttf"));
    add_static("inter-semibold", include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"));
    add_static("inter-bold", include_bytes!("../../../assets/fonts/Inter-Bold.ttf"));
    add_static("jbmono", include_bytes!("../../../assets/fonts/JetBrainsMono-SemiBold.ttf"));
    add_static("jbmono-medium", include_bytes!("../../../assets/fonts/JetBrainsMono-Medium.ttf"));

    // Do Windows: símbolos que faltam na Inter (⌨, ✔...) e os ícones dos botões.
    let dir = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or("C:\\Windows".into())).join("Fonts");
    let mut system = |name: &str, files: &[&str]| -> Option<String> {
        let bytes = files.iter().find_map(|f| std::fs::read(dir.join(f)).ok())?;
        fonts.font_data.insert(name.to_owned(), FontData::from_owned(bytes));
        Some(name.to_owned())
    };
    let symbols = system("segoe-symbol", &["seguisym.ttf"]);
    let segoe = system("segoe", &["segoeui.ttf"]);
    let icons = system("icons", &["SegoeIcons.ttf", "segmdl2.ttf"]);

    let fallback: Vec<String> = [segoe, symbols].into_iter().flatten().collect();
    let family = |main: &str| -> Vec<String> {
        std::iter::once(main.to_owned()).chain(fallback.iter().cloned()).collect()
    };
    fonts.families.insert(FontFamily::Proportional, family("inter"));
    fonts.families.insert(FontFamily::Monospace, family("jbmono-medium"));
    for (name, font) in [
        ("medium", "inter-medium"),
        ("semibold", "inter-semibold"),
        ("bold", "inter-bold"),
        ("mono", "jbmono"),
        ("mono-medium", "jbmono-medium"),
    ] {
        fonts.families.insert(FontFamily::Name(name.into()), family(font));
    }
    let icon_family: Vec<String> = icons.into_iter().chain(fallback.iter().cloned()).collect();
    fonts.families.insert(FontFamily::Name("icons".into()), icon_family);
    fonts
}
