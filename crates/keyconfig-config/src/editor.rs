//! Painel de baixo: escolhe o que a tecla selecionada faz.
//!
//! Teclado e Multimídia aplicam no clique. As outras abas editam um rascunho
//! que só vai para o perfil ao clicar em "Aplicar".

use eframe::egui::{self, vec2, RichText};
use keyconfig_core::geometry;
use keyconfig_core::{Action, KeyCode, Language, MacroStep, Profile};

use crate::keyboard::{summary, ACCENT};

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
            Page::Keyboard => "Teclado",
            Page::Media => "Multimídia",
            Page::Combination => "Combinação",
            Page::Macro => "Macro",
            Page::Text => "Texto",
            Page::Run => "Abrir programa",
        }
    }

    fn for_action(action: Option<&Action>) -> Page {
        match action {
            Some(Action::Key { to }) if MEDIA.contains(to) => Page::Media,
            Some(Action::Shortcut { .. }) => Page::Combination,
            Some(Action::Macro { .. }) => Page::Macro,
            Some(Action::Text { .. }) => Page::Text,
            Some(Action::Run { .. }) => Page::Run,
            _ => Page::Keyboard,
        }
    }
}

use KeyCode::*;

const COMMON: &[KeyCode] = &[
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
    KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Digit1, Digit2, Digit3,
    Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Digit0, Backquote, Minus, Equal, BracketLeft,
    BracketRight, Backslash, Semicolon, Quote, Comma, Period, Slash, IntlBackslash, IntlRo, Space,
];
const ADVANCED: &[KeyCode] = &[
    Escape, Tab, CapsLock, Enter, Backspace, Insert, Delete, Home, End, PageUp, PageDown, ArrowUp,
    ArrowDown, ArrowLeft, ArrowRight, PrintScreen, ScrollLock, Pause, ContextMenu, F1, F2, F3, F4,
    F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
];
const NUMPAD: &[KeyCode] = &[
    NumLock, NumpadDivide, NumpadMultiply, NumpadSubtract, Numpad7, Numpad8, Numpad9, NumpadAdd,
    Numpad4, Numpad5, Numpad6, NumpadComma, Numpad1, Numpad2, Numpad3, NumpadEnter, Numpad0,
    NumpadDecimal,
];
const MODIFIERS: &[KeyCode] = &[
    ControlLeft, ShiftLeft, AltLeft, MetaLeft, ControlRight, ShiftRight, AltRight, MetaRight,
];
const MEDIA: &[KeyCode] = &[
    MediaPlayPause, MediaStop, MediaTrackPrevious, MediaTrackNext, AudioVolumeMute,
    AudioVolumeDown, AudioVolumeUp,
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
    loaded_for: Option<(String, KeyCode)>,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            tab: Page::Keyboard,
            drafts: Drafts::default(),
            loaded_for: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, profile_id: &str, profile: &mut Profile, key: Option<KeyCode>) {
        let lang = profile.language;
        let Some(key) = key else {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Clique numa tecla do teclado acima para escolher o que ela faz.").size(16.0));
            });
            return;
        };

        // Trocou de tecla ou de perfil: recarrega os rascunhos e escolhe a aba certa.
        let target = (profile_id.to_owned(), key);
        if self.loaded_for.as_ref() != Some(&target) {
            let action = profile.mappings.get(&key);
            self.drafts = Drafts::from_action(action);
            self.tab = Page::for_action(action);
            self.loaded_for = Some(target);
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new(geometry::describe(key, lang)).size(18.0).strong());
            ui.label(RichText::new("→").size(18.0));
            let current = match profile.mappings.get(&key) {
                Some(action) => describe_action(action, lang),
                None => "comportamento padrão".to_owned(),
            };
            ui.label(RichText::new(current).size(16.0).color(ACCENT));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(profile.mappings.contains_key(&key), egui::Button::new("Restaurar padrão")).clicked() {
                    profile.mappings.remove(&key);
                }
                let disabled = profile.mappings.get(&key) == Some(&Action::Disabled);
                if ui.add_enabled(!disabled, egui::Button::new("Desligar tecla")).clicked() {
                    profile.mappings.insert(key, Action::Disabled);
                }
            });
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            for tab in Page::ALL {
                let text = RichText::new(tab.label()).size(15.0);
                if ui.selectable_label(self.tab == tab, text).clicked() {
                    self.tab = tab;
                }
                ui.add_space(8.0);
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match self.tab {
            Page::Keyboard => {
                let current = match profile.mappings.get(&key) {
                    Some(Action::Key { to }) => Some(*to),
                    _ => None,
                };
                let groups = [
                    ("Comuns", COMMON, 10),
                    ("Avançadas", ADVANCED, 6),
                    ("Teclado numérico", NUMPAD, 4),
                    ("Modificadores", MODIFIERS, 4),
                ];
                if let Some(to) = palette(ui, lang, &groups, current) {
                    profile.mappings.insert(key, Action::Key { to });
                }
            }
            Page::Media => {
                let current = match profile.mappings.get(&key) {
                    Some(Action::Key { to }) => Some(*to),
                    _ => None,
                };
                if let Some(to) = palette(ui, lang, &[("Mídia", MEDIA, 7)], current) {
                    profile.mappings.insert(key, Action::Key { to });
                }
            }
            Page::Combination => {
                if let Some(action) = self.combination(ui, lang) {
                    profile.mappings.insert(key, action);
                }
            }
            Page::Macro => {
                if let Some(action) = self.macro_editor(ui, lang) {
                    profile.mappings.insert(key, action);
                }
            }
            Page::Text => {
                ui.label("O texto é digitado do jeito que está escrito, com acentos, em qualquer idioma de teclado.");
                ui.add(
                    egui::TextEdit::multiline(&mut self.drafts.text)
                        .desired_rows(5)
                        .desired_width(600.0)
                        .hint_text("Ex.: Atenciosamente,\nGabriel"),
                );
                if apply_button(ui, !self.drafts.text.is_empty()) {
                    profile.mappings.insert(key, Action::Text { text: self.drafts.text.clone() });
                }
            }
            Page::Run => {
                ui.label("Abre um programa, arquivo, pasta ou site.");
                egui::Grid::new("run").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label("Abrir:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drafts.command)
                            .desired_width(460.0)
                            .hint_text(r"notepad.exe, C:\Jogos\jogo.exe ou https://..."),
                    );
                    ui.end_row();
                    ui.label("Argumentos:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drafts.args)
                            .desired_width(460.0)
                            .hint_text("opcional"),
                    );
                    ui.end_row();
                });
                if apply_button(ui, !self.drafts.command.trim().is_empty()) {
                    profile.mappings.insert(
                        key,
                        Action::Run {
                            command: self.drafts.command.trim().to_owned(),
                            args: self.drafts.args.trim().to_owned(),
                        },
                    );
                }
            }
        });
    }

    fn combination(&mut self, ui: &mut egui::Ui, lang: Language) -> Option<Action> {
        let mut result = None;
        ui.horizontal(|ui| {
            ui.label("Segurar:");
            for (i, (_, name)) in COMBO_MODS.iter().enumerate() {
                let on = self.drafts.mods[i];
                if ui.add(egui::Button::new(*name).selected(on).min_size(vec2(56.0, 28.0))).clicked() {
                    self.drafts.mods[i] = !on;
                }
            }
            ui.add_space(16.0);

            let keys = self.drafts.shortcut_keys();
            let preview = if keys.is_empty() {
                "escolha as teclas".to_owned()
            } else {
                keys.iter().map(|&k| button_text(k, lang)).collect::<Vec<_>>().join(" + ")
            };
            ui.label(RichText::new(preview).size(16.0).strong().color(ACCENT));
            ui.add_space(16.0);
            if apply_button(ui, self.drafts.combo_key.is_some()) {
                result = Some(Action::Shortcut { keys });
            }
        });
        ui.add_space(4.0);
        ui.label("E apertar:");
        let groups = [("Comuns", COMMON, 10), ("Avançadas", ADVANCED, 6), ("Teclado numérico", NUMPAD, 4)];
        if let Some(k) = palette(ui, lang, &groups, self.drafts.combo_key) {
            self.drafts.combo_key = Some(k);
        }
        result
    }

    fn macro_editor(&mut self, ui: &mut egui::Ui, lang: Language) -> Option<Action> {
        let steps = &mut self.drafts.steps;
        ui.horizontal(|ui| {
            ui.label("Adicionar:");
            let last_key = steps.iter().rev().find_map(step_key).unwrap_or(KeyA);
            if ui.button("Tocar tecla").clicked() {
                steps.push(MacroStep::Tap(last_key));
            }
            if ui.button("Segurar tecla").clicked() {
                steps.push(MacroStep::Down(last_key));
            }
            if ui.button("Soltar tecla").clicked() {
                steps.push(MacroStep::Up(last_key));
            }
            if ui.button("Pausa").clicked() {
                steps.push(MacroStep::DelayMs(100));
            }
            if ui.button("Texto").clicked() {
                steps.push(MacroStep::Text(String::new()));
            }
        });
        ui.add_space(4.0);

        if steps.is_empty() {
            ui.label(RichText::new("A macro ainda não tem passos.").italics());
        }

        let mut action = None;
        let mut remove = None;
        let mut swap = None;
        let count = steps.len();
        for (i, step) in steps.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{:>2}.", i + 1));
                    step_editor(ui, step, lang);
                    if ui.add_enabled(i > 0, egui::Button::new("⬆")).clicked() {
                        swap = Some(i - 1);
                    }
                    if ui.add_enabled(i + 1 < count, egui::Button::new("⬇")).clicked() {
                        swap = Some(i);
                    }
                    if ui.button("🗑").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = swap {
            steps.swap(i, i + 1);
        }
        if let Some(i) = remove {
            steps.remove(i);
        }

        ui.add_space(6.0);
        if apply_button(ui, !steps.is_empty()) {
            action = Some(Action::Macro { steps: steps.clone() });
        }
        action
    }
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
        ("Tocar", MacroStep::Tap(key)),
        ("Segurar", MacroStep::Down(key)),
        ("Soltar", MacroStep::Up(key)),
        ("Pausa", MacroStep::DelayMs(100)),
        ("Texto", MacroStep::Text(String::new())),
    ];
    let current = kinds
        .iter()
        .position(|(_, k)| std::mem::discriminant(k) == std::mem::discriminant(step))
        .unwrap_or(0);
    egui::ComboBox::from_id_salt("kind")
        .width(90.0)
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
            ui.add(egui::TextEdit::singleline(text).desired_width(300.0).hint_text("texto"));
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

/// Grupos de botões de teclas lado a lado; retorna a tecla clicada.
fn palette(
    ui: &mut egui::Ui,
    lang: Language,
    groups: &[(&str, &[KeyCode], usize)],
    highlight: Option<KeyCode>,
) -> Option<KeyCode> {
    const BUTTON: egui::Vec2 = vec2(46.0, 28.0);
    let spacing = ui.spacing().item_spacing.x;
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        for (title, keys, columns) in groups {
            ui.vertical(|ui| {
                let width = *columns as f32 * (BUTTON.x + spacing) + 24.0;
                ui.set_width(width);
                ui.label(RichText::new(*title).strong().size(14.0));
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width(width);
                    for &k in *keys {
                        let button = egui::Button::new(button_text(k, lang))
                            .min_size(BUTTON)
                            .selected(highlight == Some(k));
                        if ui.add(button).on_hover_text(geometry::describe(k, lang)).clicked() {
                            picked = Some(k);
                        }
                    }
                });
            });
            ui.add_space(20.0);
        }
    });
    picked
}

fn button_text(key: KeyCode, lang: Language) -> String {
    match key {
        ControlLeft => "Ctrl E".into(),
        ControlRight => "Ctrl D".into(),
        ShiftLeft => "Shift E".into(),
        ShiftRight => "Shift D".into(),
        AltLeft => "Alt E".into(),
        MetaLeft => "Win E".into(),
        MetaRight => "Win D".into(),
        Space => "Espaço".into(),
        _ if MEDIA.contains(&key) || NUMPAD.contains(&key) => geometry::describe(key, lang).replace("Mídia: ", ""),
        _ => geometry::label(key, lang).into(),
    }
}

fn describe_action(action: &Action, lang: Language) -> String {
    match action {
        Action::Key { to } => geometry::describe(*to, lang),
        Action::Disabled => "desligada".to_owned(),
        Action::Shortcut { .. } => format!("combinação {}", summary(action, lang)),
        Action::Macro { steps } => format!("macro com {} passo(s)", steps.len()),
        Action::Text { text } => format!("digita \"{}\"", text.lines().next().unwrap_or("")),
        Action::Run { command, .. } => format!("abre {command}"),
    }
}

fn apply_button(ui: &mut egui::Ui, enabled: bool) -> bool {
    let button = egui::Button::new(RichText::new("Aplicar").size(15.0)).min_size(vec2(90.0, 30.0));
    ui.add_enabled(enabled, button).clicked()
}
