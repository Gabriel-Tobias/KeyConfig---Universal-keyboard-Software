//! Cartão de baixo: escolhe o que a tecla selecionada faz.
//!
//! Teclado e Multimídia aplicam no clique. As outras abas editam um rascunho
//! que só vai para o perfil ao clicar em "Aplicar".

use eframe::egui::{self, vec2, Align2, Color32, Frame, Margin, Rect, RichText, Sense, Stroke};
use keyconfig_core::geometry;
use keyconfig_core::{tr, Action, KeyCode, Language, MacroStep, Modifier, Profile, Trigger};

use crate::keyboard::summary;
use crate::theme::{
    self, icon, Kind, ACCENT, ACCENT_BG, BORDER, CARD, CHIP, CHIP_BORDER, CONTROL_HOVER, DIVIDER, TEXT,
    TEXT_DIM, TEXT_FAINT,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Keyboard,
    Media,
    Combination,
    Macro,
    Text,
    Run,
}

impl Page {
    const ALL: [Page; 6] = [Page::Keyboard, Page::Media, Page::Combination, Page::Macro, Page::Text, Page::Run];

    fn label(self) -> &'static str {
        match self {
            Page::Keyboard => tr("Teclado", "Keyboard", "Teclado"),
            Page::Media => tr("Multimídia", "Media", "Multimedia"),
            Page::Combination => tr("Combinação", "Shortcut", "Combinación"),
            Page::Macro => "Macro",
            Page::Text => tr("Texto", "Text", "Texto"),
            Page::Run => tr("Abrir programa", "Open program", "Abrir programa"),
        }
    }

    fn for_action(action: Option<&Action>) -> Page {
        match action {
            Some(Action::Key { to }) if MEDIA.contains(to) => Page::Media,
            Some(Action::Shortcut { .. }) => Page::Combination,
            Some(Action::Macro { .. }) => Page::Macro,
            Some(Action::Text { text }) if CHARACTERS.contains(&text.as_str()) => Page::Keyboard,
            Some(Action::DeadKey { .. }) => Page::Keyboard,
            Some(Action::Text { .. }) => Page::Text,
            Some(Action::Run { .. }) => Page::Run,
            _ => Page::Keyboard,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Category {
    Common,
    Advanced,
    Numpad,
    Modifiers,
    Accents,
    Characters,
}

impl Category {
    const ALL: [Category; 6] = [
        Category::Common,
        Category::Advanced,
        Category::Numpad,
        Category::Modifiers,
        Category::Accents,
        Category::Characters,
    ];
    /// Só as que geram teclas, para a aba Combinação.
    const KEYS: [Category; 3] = [Category::Common, Category::Advanced, Category::Numpad];

    fn label(self) -> &'static str {
        match self {
            Category::Common => tr("Comuns", "Common", "Comunes"),
            Category::Advanced => tr("Avançadas", "Advanced", "Avanzadas"),
            Category::Numpad => tr("Numérico", "Numpad", "Numérico"),
            Category::Modifiers => tr("Modificadores", "Modifiers", "Modificadores"),
            Category::Accents => tr("Acentos", "Accents", "Acentos"),
            Category::Characters => tr("Caracteres", "Characters", "Caracteres"),
        }
    }

    fn chips(self, lang: Language) -> Vec<Chip> {
        let keys = |list: &[KeyCode]| list.iter().map(|&k| Chip::key(k, lang)).collect();
        match self {
            Category::Common => keys(COMMON),
            Category::Advanced => keys(ADVANCED),
            Category::Numpad => keys(NUMPAD),
            Category::Modifiers => keys(MODIFIERS),
            Category::Accents => keyconfig_core::accent::ACCENTS
                .iter()
                .map(|&accent| Chip {
                    label: accent.to_string(),
                    hint: format!("{} {accent}: {}", tr("Acento", "Accent", "Acento"), combines_hint()),
                    action: Action::DeadKey { accent },
                })
                .collect(),
            Category::Characters => CHARACTERS
                .iter()
                .map(|&c| Chip {
                    label: c.to_owned(),
                    hint: format!("{} {c}", tr("Digita", "Types", "Escribe")),
                    action: Action::Text { text: c.to_owned() },
                })
                .collect(),
        }
    }
}

/// Uma opção da grade: o texto do botão e a ação que ele aplica.
struct Chip {
    label: String,
    hint: String,
    action: Action,
}

impl Chip {
    fn key(k: KeyCode, lang: Language) -> Chip {
        Chip { label: button_text(k, lang), hint: geometry::describe(k, lang), action: Action::Key { to: k } }
    }
}

use KeyCode::*;

const COMMON: &[KeyCode] = &[
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
    KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Digit0, Digit1, Digit2,
    Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Backquote, Minus, Equal, BracketLeft,
    BracketRight, Backslash, Semicolon, Quote, Comma, Period, Slash, IntlBackslash, IntlRo, Space,
];
const ADVANCED: &[KeyCode] = &[
    Escape, Tab, CapsLock, Enter, Backspace, Insert, Delete, Home, End, PageUp, PageDown, ArrowUp,
    ArrowDown, ArrowLeft, ArrowRight, PrintScreen, ScrollLock, Pause, ContextMenu, F1, F2, F3, F4,
    F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
];
const NUMPAD: &[KeyCode] = &[
    NumLock, NumpadDivide, NumpadMultiply, NumpadSubtract, NumpadAdd, NumpadDecimal, NumpadComma,
    NumpadEnter, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8,
    Numpad9,
];
const MODIFIERS: &[KeyCode] = &[
    ControlLeft, ShiftLeft, AltLeft, MetaLeft, ControlRight, ShiftRight, AltRight, MetaRight,
];
const MEDIA: &[KeyCode] = &[
    MediaPlayPause, MediaStop, MediaTrackPrevious, MediaTrackNext, AudioVolumeMute,
    AudioVolumeDown, AudioVolumeUp,
];

/// Caracteres digitados direto (via Unicode), que saem iguais em qualquer idioma do Windows.
const CHARACTERS: &[&str] = &[
    "?", "!", "/", "\\", "|", ":", ";", "@", "#", "$", "%", "&", "*", "(", ")", "[", "]", "{", "}",
    "<", ">", "=", "+", "-", "_", "'", "\"", "~", "^", "`", "´",
    "ç", "Ç", "ã", "õ", "ñ", "°", "ª", "º", "§", "€", "£", "¬", "¹", "²", "³",
];

/// Modificadores da aba Combinação, na ordem em que são apertados.
const COMBO_MODS: [(KeyCode, &str); 4] = [
    (ControlLeft, "Ctrl"),
    (ShiftLeft, "Shift"),
    (AltLeft, "Alt"),
    (MetaLeft, "Win"),
];

#[derive(Default)]
struct Drafts {
    mods: [bool; 4],
    combo_key: Option<KeyCode>,
    steps: Vec<MacroStep>,
    text: String,
    command: String,
    args: String,
}

impl Drafts {
    fn from_action(action: Option<&Action>) -> Self {
        let mut d = Drafts::default();
        match action {
            Some(Action::Shortcut { keys }) => {
                for (i, (m, _)) in COMBO_MODS.iter().enumerate() {
                    d.mods[i] = keys.iter().any(|k| same_modifier(*k, *m));
                }
                d.combo_key = keys.iter().copied().find(|k| !MODIFIERS.contains(k));
            }
            Some(Action::Macro { steps }) => d.steps = steps.clone(),
            Some(Action::Text { text }) => d.text = text.clone(),
            Some(Action::Run { command, args }) => {
                d.command = command.clone();
                d.args = args.clone();
            }
            _ => {}
        }
        d
    }

    fn shortcut_keys(&self) -> Vec<KeyCode> {
        COMBO_MODS
            .iter()
            .zip(self.mods)
            .filter(|(_, on)| *on)
            .map(|((k, _), _)| *k)
            .chain(self.combo_key)
            .collect()
    }
}

fn same_modifier(a: KeyCode, b: KeyCode) -> bool {
    let base = |k: KeyCode| match k {
        ControlRight => ControlLeft,
        ShiftRight => ShiftLeft,
        AltRight => AltLeft,
        MetaRight => MetaLeft,
        k => k,
    };
    base(a) == base(b)
}

pub struct Editor {
    tab: Page,
    drafts: Drafts,
    /// Gatilho sendo editado: toque comum ou com modificadores/toque duplo.
    trigger: Trigger,
    loaded_for: Option<(String, KeyCode, Trigger)>,
    query: String,
    category: Category,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            tab: Page::Keyboard,
            drafts: Drafts::default(),
            trigger: Trigger::PLAIN,
            loaded_for: None,
            query: String::new(),
            category: Category::Common,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, profile_id: &str, profile: &mut Profile, key: Option<KeyCode>) {
        Frame::none()
            .fill(CARD)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .rounding(16.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                match key {
                    Some(key) => self.contents(ui, profile_id, profile, key),
                    None => empty_state(ui),
                }
            });
    }

    fn contents(&mut self, ui: &mut egui::Ui, profile_id: &str, profile: &mut Profile, key: KeyCode) {
        let lang = profile.language;

        // Outra tecla começa no toque comum.
        if self.loaded_for.as_ref().is_some_and(|(id, k, _)| id != profile_id || *k != key) {
            self.trigger = Trigger::PLAIN;
        }
        let trigger = self.trigger;

        // Trocou de tecla, perfil ou gatilho: recarrega os rascunhos e escolhe a aba certa.
        let target = (profile_id.to_owned(), key, trigger);
        if self.loaded_for.as_ref() != Some(&target) {
            let action = profile.action(key, trigger);
            self.drafts = Drafts::from_action(action);
            self.tab = Page::for_action(action);
            if let Some(Action::DeadKey { .. }) = action {
                self.category = Category::Accents;
            } else if let Some(Action::Text { .. }) = action {
                self.category = Category::Characters;
            }
            self.loaded_for = Some(target);
        }

        // Cabeçalho: tecla → o que ela faz, e as ações rápidas.
        section(ui, Margin::symmetric(20.0, 18.0), true, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 12.0);
                ui.horizontal(|ui| {
                    if !trigger.mods.is_empty() {
                        let mods = Trigger { double_tap: false, ..trigger }.label();
                        ui.label(RichText::new(format!("{mods} +")).font(theme::mono(14.0)).color(TEXT_DIM));
                    }
                    key_badge(ui, key, lang);
                    if trigger.double_tap {
                        ui.label(RichText::new("2×").font(theme::mono(14.0)).color(TEXT_DIM));
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(icon::ARROW).font(theme::icon_font(14.0)).color(TEXT_FAINT));
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let (text, color) = match profile.action(key, trigger) {
                            Some(action) => (describe_action(action, lang), ACCENT),
                            None if trigger.is_plain() => (tr("Comportamento padrão", "Default behavior", "Comportamiento predeterminado").to_owned(), TEXT),
                            None => (tr("Nada configurado", "Nothing set", "Nada configurado").to_owned(), TEXT),
                        };
                        ui.label(RichText::new(text).font(theme::semibold(16.0)).color(color));
                        ui.label(RichText::new(tr("Escolha abaixo o que esta tecla deve fazer", "Choose below what this key should do", "Elige abajo qué debe hacer esta tecla")).size(12.0).color(TEXT_FAINT));
                    });
                });
                theme::trailing(ui, 320.0, |ui| {
                    let current = profile.action(key, trigger);
                    let (mapped, off) = (current.is_some(), current == Some(&Action::Disabled));
                    let reset = if trigger.is_plain() {
                        tr("Restaurar padrão", "Restore default", "Restaurar")
                    } else {
                        tr("Remover", "Remove", "Quitar")
                    };
                    if theme::button(ui, Kind::Ghost, Some(icon::RESET), reset, mapped).clicked() {
                        profile.set_action(key, trigger, None);
                    }
                    if theme::button(ui, Kind::Danger, Some(icon::POWER), tr("Desligar tecla", "Turn key off", "Desactivar tecla"), !off).clicked() {
                        profile.set_action(key, trigger, Some(Action::Disabled));
                    }
                });
            });
        });

        // Gatilho: com quais modificadores e se pede toque duplo.
        section(ui, Margin::symmetric(20.0, 14.0), true, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
                theme::field_label(ui, tr("Quando", "When", "Cuando"));
                for m in Modifier::ALL {
                    if theme::chip_toggle(ui, m.label(), None, self.trigger.mods.has(m)).clicked() {
                        self.trigger.mods.set(m, !self.trigger.mods.has(m));
                    }
                }
                ui.add_space(4.0);
                if theme::chip_toggle(ui, tr("Toque duplo", "Double tap", "Doble toque"), None, self.trigger.double_tap).clicked() {
                    self.trigger.double_tap = !self.trigger.double_tap;
                }
                let others = profile.combo_triggers(key);
                if !others.is_empty() {
                    theme::wrap_if(ui, 320.0);
                    ui.add_space(12.0);
                    theme::field_label(ui, tr("Configurados", "Set up", "Configurados"));
                    let plain_set = profile.mappings.contains_key(&key);
                    let list = plain_set.then_some(Trigger::PLAIN).into_iter().chain(others);
                    for t in list {
                        let name = if t.is_plain() { tr("Normal", "Normal", "Normal").to_owned() } else { t.label() };
                        if theme::chip_toggle(ui, &name, None, t == self.trigger).clicked() {
                            self.trigger = t;
                        }
                    }
                }
            });
            let hint = match (self.trigger.mods.is_empty(), self.trigger.double_tap) {
                (true, false) => None,
                (false, false) => Some(tr(
                    "Vale só segurando esses modificadores. Sem eles, a tecla continua como antes.",
                    "Only applies while holding these modifiers. Without them, the key works as before.",
                    "Solo funciona manteniendo estos modificadores. Sin ellos, la tecla sigue como antes.",
                )),
                (_, true) => Some(tr(
                    "Dois toques rápidos fazem esta ação. Um toque só espera 0,3 s pelo segundo e então faz o normal.",
                    "Two quick taps run this action. A single tap waits 0.3 s for a second one, then does the usual.",
                    "Dos toques rápidos hacen esta acción. Un solo toque espera 0,3 s al segundo y luego hace lo normal.",
                )),
            };
            if let Some(hint) = hint {
                ui.label(RichText::new(hint).size(12.0).color(TEXT_DIM));
            }
        });

        // Abas.
        section(ui, Margin::symmetric(16.0, 12.0), true, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for page in Page::ALL {
                    if tab_button(ui, page.label(), self.tab == page).clicked() {
                        self.tab = page;
                    }
                }
            });
        });

        section(ui, Margin { left: 20.0, right: 20.0, top: 18.0, bottom: 22.0 }, false, |ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            let current = profile.action(key, trigger).cloned();
            match self.tab {
                Page::Keyboard => {
                    if let Some(action) = self.chooser(ui, lang, &Category::ALL, current.as_ref()) {
                        profile.set_action(key, trigger, Some(action));
                    }
                }
                Page::Media => {
                    let chips: Vec<Chip> = MEDIA
                        .iter()
                        .map(|&k| Chip {
                            label: geometry::label(k, lang).to_owned(),
                            hint: geometry::describe(k, lang),
                            action: Action::Key { to: k },
                        })
                        .collect();
                    ui.label(RichText::new(tr("Controle de música e volume, em qualquer programa.", "Music and volume control, in any program.", "Control de música y volumen, en cualquier programa.")).color(TEXT_DIM));
                    if let Some(action) = chip_grid(ui, &chips, current.as_ref(), 120.0) {
                        profile.set_action(key, trigger, Some(action));
                    }
                }
                Page::Combination => {
                    if let Some(action) = self.combination(ui, lang) {
                        profile.set_action(key, trigger, Some(action));
                    }
                }
                Page::Macro => {
                    if let Some(action) = self.macro_editor(ui, lang) {
                        profile.set_action(key, trigger, Some(action));
                    }
                }
                Page::Text => {
                    ui.label(
                        RichText::new(tr("O texto é digitado do jeito que está escrito, com acentos, em qualquer idioma de teclado.", "The text is typed exactly as written, with accents, in any keyboard language.", "El texto se escribe tal cual, con tildes, en cualquier idioma de teclado."))
                            .color(TEXT_DIM),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut self.drafts.text)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY)
                            .margin(Margin::same(12.0))
                            .hint_text(tr("Ex.: Atenciosamente,\nGabriel", "E.g.: Best regards,\nGabriel", "Ej.: Saludos cordiales,\nGabriel")),
                    );
                    if theme::button(ui, Kind::Primary, None, tr("Aplicar", "Apply", "Aplicar"), !self.drafts.text.is_empty()).clicked() {
                        profile.set_action(key, trigger, Some(Action::Text { text: self.drafts.text.clone() }));
                    }
                }
                Page::Run => {
                    ui.label(RichText::new(tr("Abre um programa, arquivo, pasta ou site.", "Opens a program, file, folder or website.", "Abre un programa, archivo, carpeta o sitio web.")).color(TEXT_DIM));
                    egui::Grid::new("run").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
                        theme::field_label(ui, tr("Abrir", "Open", "Abrir"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.drafts.command)
                                .desired_width(480.0)
                                .margin(Margin::symmetric(12.0, 9.0))
                                .hint_text(tr(r"notepad.exe, C:\Jogos\jogo.exe ou https://...", r"notepad.exe, C:\Games\game.exe or https://...", r"notepad.exe, C:\Juegos\juego.exe o https://...")),
                        );
                        ui.end_row();
                        theme::field_label(ui, tr("Argumentos", "Arguments", "Argumentos"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.drafts.args)
                                .desired_width(480.0)
                                .margin(Margin::symmetric(12.0, 9.0))
                                .hint_text(tr("opcional", "optional", "opcional")),
                        );
                        ui.end_row();
                    });
                    let ok = !self.drafts.command.trim().is_empty();
                    if theme::button(ui, Kind::Primary, None, tr("Aplicar", "Apply", "Aplicar"), ok).clicked() {
                        profile.set_action(
                            key,
                            trigger,
                            Some(Action::Run {
                                command: self.drafts.command.trim().to_owned(),
                                args: self.drafts.args.trim().to_owned(),
                            }),
                        );
                    }
                }
            }
        });
    }

    /// Busca + categorias + grade de opções. Retorna a ação clicada.
    fn chooser(
        &mut self,
        ui: &mut egui::Ui,
        lang: Language,
        categories: &[Category],
        current: Option<&Action>,
    ) -> Option<Action> {
        if !categories.contains(&self.category) {
            self.category = categories[0];
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            search_field(ui, &mut self.query);
            for &cat in categories {
                let count = cat.chips(lang).len().to_string();
                let on = cat == self.category && self.query.is_empty();
                if theme::chip_toggle(ui, cat.label(), Some(&count), on).clicked() {
                    self.category = cat;
                    self.query.clear();
                }
            }
        });

        let query = self.query.trim().to_lowercase();
        let chips: Vec<Chip> = if query.is_empty() {
            if self.category == Category::Accents {
                ui.label(
                    RichText::new(tr("Combinam com a próxima letra: ~ + a = ã. Com espaço, sai só o acento.", "They combine with the next letter: ~ + a = ã. With space, you get just the accent.", "Se combinan con la siguiente letra: ~ + a = ã. Con espacio, sale solo el acento."))
                        .size(12.0)
                        .color(TEXT_DIM),
                );
            }
            self.category.chips(lang)
        } else {
            let mut found: Vec<Chip> = Vec::new();
            for cat in categories {
                for chip in cat.chips(lang) {
                    let matches = chip.label.to_lowercase().contains(&query) || chip.hint.to_lowercase().contains(&query);
                    if matches && !found.iter().any(|c| c.action == chip.action) {
                        found.push(chip);
                    }
                }
            }
            found
        };
        if chips.is_empty() {
            ui.label(RichText::new(tr("Nenhuma tecla encontrada.", "No key found.", "No se encontró ninguna tecla.")).color(TEXT_FAINT));
            return None;
        }
        chip_grid(ui, &chips, current, 64.0)
    }

    fn combination(&mut self, ui: &mut egui::Ui, lang: Language) -> Option<Action> {
        let mut result = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            theme::field_label(ui, tr("Segurar", "Hold", "Mantener"));
            for (i, (_, name)) in COMBO_MODS.iter().enumerate() {
                if theme::chip_toggle(ui, name, None, self.drafts.mods[i]).clicked() {
                    self.drafts.mods[i] = !self.drafts.mods[i];
                }
            }
            ui.add_space(12.0);
            let keys = self.drafts.shortcut_keys();
            let preview = if self.drafts.combo_key.is_none() {
                tr("escolha a tecla abaixo", "choose the key below", "elige la tecla abajo").to_owned()
            } else {
                keys.iter().map(|&k| geometry::label(k, lang)).collect::<Vec<_>>().join(" + ")
            };
            let color = if self.drafts.combo_key.is_some() { ACCENT } else { TEXT_FAINT };
            ui.label(RichText::new(preview).font(theme::mono(14.0)).color(color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::button(ui, Kind::Primary, None, tr("Aplicar", "Apply", "Aplicar"), self.drafts.combo_key.is_some()).clicked() {
                    result = Some(Action::Shortcut { keys });
                }
            });
        });
        let current = self.drafts.combo_key.map(|k| Action::Key { to: k });
        if let Some(Action::Key { to }) = self.chooser(ui, lang, &Category::KEYS, current.as_ref()) {
            self.drafts.combo_key = Some(to);
        }
        result
    }

    fn macro_editor(&mut self, ui: &mut egui::Ui, lang: Language) -> Option<Action> {
        let steps = &mut self.drafts.steps;
        ui.horizontal_wrapped(|ui| {
            theme::field_label(ui, tr("Adicionar", "Add", "Añadir"));
            let last_key = steps.iter().rev().find_map(step_key).unwrap_or(KeyA);
            let add = |ui: &mut egui::Ui, text: &str| theme::button(ui, Kind::Normal, Some(icon::ADD), text, true).clicked();
            if add(ui, tr("Tocar tecla", "Tap key", "Pulsar tecla")) {
                steps.push(MacroStep::Tap(last_key));
            }
            if add(ui, tr("Segurar tecla", "Hold key", "Mantener tecla")) {
                steps.push(MacroStep::Down(last_key));
            }
            if add(ui, tr("Soltar tecla", "Release key", "Soltar tecla")) {
                steps.push(MacroStep::Up(last_key));
            }
            if add(ui, tr("Pausa", "Pause", "Pausa")) {
                steps.push(MacroStep::DelayMs(100));
            }
            if add(ui, tr("Texto", "Text", "Texto")) {
                steps.push(MacroStep::Text(String::new()));
            }
        });

        if steps.is_empty() {
            ui.label(RichText::new(tr("A macro ainda não tem passos.", "The macro has no steps yet.", "La macro aún no tiene pasos.")).color(TEXT_FAINT));
        }

        let mut remove = None;
        let mut swap = None;
        let count = steps.len();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (i, step) in steps.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    Frame::none()
                        .fill(CHIP)
                        .stroke(Stroke::new(1.0_f32, CHIP_BORDER))
                        .rounding(10.0)
                        .inner_margin(Margin::symmetric(12.0, 6.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{:02}", i + 1)).font(theme::mono(13.0)).color(TEXT_FAINT));
                                step_editor(ui, step, lang);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if theme::icon_button(ui, icon::DELETE, tr("Remover", "Remove", "Quitar"), true).clicked() {
                                        remove = Some(i);
                                    }
                                    if theme::icon_button(ui, "\u{E70D}", tr("Descer", "Move down", "Bajar"), i + 1 < count).clicked() {
                                        swap = Some(i);
                                    }
                                    if theme::icon_button(ui, "\u{E70E}", tr("Subir", "Move up", "Subir"), i > 0).clicked() {
                                        swap = Some(i - 1);
                                    }
                                });
                            });
                        });
                });
            }
        });
        if let Some(i) = swap {
            steps.swap(i, i + 1);
        }
        if let Some(i) = remove {
            steps.remove(i);
        }

        let mut action = None;
        if theme::button(ui, Kind::Primary, None, tr("Aplicar", "Apply", "Aplicar"), !steps.is_empty()).clicked() {
            action = Some(Action::Macro { steps: steps.clone() });
        }
        action
    }
}

/// Bloco do cartão com margem própria e, opcionalmente, uma linha divisória embaixo.
fn section(ui: &mut egui::Ui, margin: Margin, divider: bool, add: impl FnOnce(&mut egui::Ui)) {
    let response = Frame::none().inner_margin(margin).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;
        add(ui);
    });
    if divider {
        let r = response.response.rect;
        ui.painter().hline(r.x_range(), r.bottom(), Stroke::new(1.0_f32, DIVIDER));
    }
}

fn empty_state(ui: &mut egui::Ui) {
    Frame::none().inner_margin(Margin::symmetric(20.0, 48.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.label(RichText::new(icon::KEYBOARD).font(theme::icon_font(28.0)).color(TEXT_FAINT));
            ui.label(RichText::new(tr("Selecione uma tecla", "Select a key", "Selecciona una tecla")).font(theme::semibold(15.0)).color(TEXT));
            ui.label(
                RichText::new(tr("Clique numa tecla do desenho ou use \"Capturar tecla\" para escolher o que ela faz.", "Click a key in the drawing or use \"Capture key\" to choose what it does.", "Haz clic en una tecla del dibujo o usa \"Capturar tecla\" para elegir qué hace."))
                    .size(13.0)
                    .color(TEXT_FAINT),
            );
        });
    });
}

/// A tecla selecionada, como uma tecla âmbar no cabeçalho.
fn key_badge(ui: &mut egui::Ui, key: KeyCode, lang: Language) {
    let label = match geometry::label(key, lang) {
        "" => geometry::space_name(),
        l => l,
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), theme::mono(15.0), TEXT);
    let width = (galley.size().x + 20.0).max(44.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 44.0), Sense::hover());
    ui.painter().rect(rect, 10.0, Color32::from_rgb(0x2e, 0x27, 0x16), Stroke::new(2.0_f32, ACCENT));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, TEXT);
}

fn tab_button(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::medium(14.0), TEXT);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 32.0, 36.0), Sense::click());
    let hovered = response.hovered();
    let (fill, color) = if on {
        (Color32::from_rgb(0x2a, 0x30, 0x3b), Color32::WHITE)
    } else if hovered {
        (CONTROL_HOVER.gamma_multiply(0.8), TEXT)
    } else {
        (Color32::TRANSPARENT, TEXT_DIM)
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Campo de busca com a lupa dentro, à esquerda.
fn search_field(ui: &mut egui::Ui, query: &mut String) {
    Frame::none()
        .fill(theme::FIELD)
        .stroke(Stroke::new(1.0_f32, theme::CONTROL_BORDER))
        .rounding(8.0)
        .inner_margin(Margin::symmetric(12.0, 0.0))
        .show(ui, |ui| {
            ui.set_height(34.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.label(RichText::new(icon::SEARCH).font(theme::icon_font(13.0)).color(TEXT_FAINT));
                ui.add(
                    egui::TextEdit::singleline(query)
                        .frame(false)
                        .desired_width(180.0)
                        .margin(Margin::ZERO)
                        .hint_text(tr("Buscar tecla…", "Search key…", "Buscar tecla…")),
                );
            });
        });
}

/// Grade de opções que se ajusta à largura (como `repeat(auto-fill, minmax(64px, 1fr))`).
fn chip_grid(ui: &mut egui::Ui, chips: &[Chip], current: Option<&Action>, min_width: f32) -> Option<Action> {
    const GAP: f32 = 8.0;
    let width = ui.available_width();
    let columns = (((width + GAP) / (min_width + GAP)).floor() as usize).max(1);
    let chip_width = (width - GAP * (columns - 1) as f32) / columns as f32;
    let rows = chips.len().div_ceil(columns);
    let (area, _) = ui.allocate_exact_size(vec2(width, rows as f32 * (40.0 + GAP) - GAP), Sense::hover());

    let mut picked = None;
    for (i, chip) in chips.iter().enumerate() {
        let (row, col) = (i / columns, i % columns);
        let rect = Rect::from_min_size(
            area.min + vec2(col as f32 * (chip_width + GAP), row as f32 * (40.0 + GAP)),
            vec2(chip_width, 40.0),
        );
        let response = ui.interact(rect, ui.id().with(("chip", i)), Sense::click());
        let active = current == Some(&chip.action);
        let hovered = response.hovered();
        let (fill, stroke) = if active || hovered { (ACCENT_BG, ACCENT) } else { (CHIP, CHIP_BORDER) };
        let painter = ui.painter();
        painter.rect(rect, 8.0, fill, Stroke::new(1.0_f32, stroke));
        let color = if active { ACCENT } else { TEXT };
        let text = fit(&chip.label, chip_width - 16.0, 13.0 * 0.62);
        painter.text(rect.center(), Align2::CENTER_CENTER, text, theme::mono_medium(13.0), color);
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.on_hover_text(&chip.hint).clicked() {
            picked = Some(chip.action.clone());
        }
    }
    picked
}

fn step_key(step: &MacroStep) -> Option<KeyCode> {
    match step {
        MacroStep::Tap(k) | MacroStep::Down(k) | MacroStep::Up(k) => Some(*k),
        _ => None,
    }
}

fn step_editor(ui: &mut egui::Ui, step: &mut MacroStep, lang: Language) {
    let key = step_key(step).unwrap_or(KeyA);
    let kinds = [
        (tr("Tocar", "Tap", "Pulsar"), MacroStep::Tap(key)),
        (tr("Segurar", "Hold", "Mantener"), MacroStep::Down(key)),
        (tr("Soltar", "Release", "Soltar"), MacroStep::Up(key)),
        (tr("Pausa", "Pause", "Pausa"), MacroStep::DelayMs(100)),
        (tr("Texto", "Text", "Texto"), MacroStep::Text(String::new())),
    ];
    let current = kinds
        .iter()
        .position(|(_, k)| std::mem::discriminant(k) == std::mem::discriminant(step))
        .unwrap_or(0);
    egui::ComboBox::from_id_salt("kind")
        .width(100.0)
        .selected_text(kinds[current].0)
        .show_ui(ui, |ui| {
            for (i, (name, new)) in kinds.iter().enumerate() {
                if ui.selectable_label(i == current, *name).clicked() && i != current {
                    *step = new.clone();
                }
            }
        });

    match step {
        MacroStep::Tap(k) | MacroStep::Down(k) | MacroStep::Up(k) => key_combo(ui, k, lang),
        MacroStep::DelayMs(ms) => {
            ui.add(egui::DragValue::new(ms).range(0..=60_000).suffix(" ms").speed(10));
        }
        MacroStep::Text(text) => {
            ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(300.0)
                    .margin(Margin::symmetric(10.0, 6.0))
                    .hint_text(tr("texto", "text", "texto")),
            );
        }
    }
}

/// Lista com todas as teclas, para os passos de macro.
fn key_combo(ui: &mut egui::Ui, key: &mut KeyCode, lang: Language) {
    egui::ComboBox::from_id_salt("key")
        .width(170.0)
        .selected_text(geometry::describe(*key, lang))
        .height(320.0)
        .show_ui(ui, |ui| {
            for &k in KeyCode::ALL {
                ui.selectable_value(key, k, geometry::describe(k, lang));
            }
        });
}

fn button_text(key: KeyCode, lang: Language) -> String {
    match key {
        // Lado abreviado: E/D (esquerdo/direito), L/R (left/right), I/D (izquierdo/derecho).
        ControlLeft | ShiftLeft | AltLeft | MetaLeft => {
            format!("{} {}", geometry::label(key, lang), tr("E", "L", "I"))
        }
        ControlRight | ShiftRight | MetaRight => {
            format!("{} {}", geometry::label(key, lang), tr("D", "R", "D"))
        }
        Space => geometry::space_name().into(),
        _ if MEDIA.contains(&key) => geometry::label(key, lang).into(),
        _ if NUMPAD.contains(&key) => geometry::describe(key, lang),
        _ => geometry::label(key, lang).into(),
    }
}

fn describe_action(action: &Action, lang: Language) -> String {
    match action {
        Action::Key { to } => geometry::describe(*to, lang),
        Action::Disabled => tr("Desligada", "Off", "Desactivada").to_owned(),
        Action::Shortcut { .. } => format!("{} {}", tr("Combinação", "Shortcut", "Combinación"), summary(action, lang)),
        Action::Macro { steps } => format!(
            "{} {} {}",
            tr("Macro com", "Macro with", "Macro con"),
            steps.len(),
            tr("passo(s)", "step(s)", "paso(s)")
        ),
        Action::Text { text } => format!("{} \"{}\"", tr("Digita", "Types", "Escribe"), text.lines().next().unwrap_or("")),
        Action::DeadKey { accent } => format!("{} {accent} ({})", tr("Acento", "Accent", "Acento"), combines_hint()),
        Action::Run { command, .. } => format!("{} {command}", tr("Abre", "Opens", "Abre")),
    }
}

fn combines_hint() -> &'static str {
    tr("combina com a próxima letra", "combines with the next letter", "se combina con la siguiente letra")
}

/// Corta textos que não cabem (`char_width` é a largura média de um caractere).
fn fit(text: &str, width: f32, char_width: f32) -> String {
    let max = (width / char_width).max(2.0) as usize;
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let cut: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}
