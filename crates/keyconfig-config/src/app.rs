use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText};
use keyconfig_core::geometry;
use keyconfig_core::store::{Settings, Store};
use keyconfig_core::{KeyCode, Language, Layout, Profile};

use crate::editor::Editor;
use crate::{capture, engine, keyboard};

const OK_COLOR: Color32 = Color32::from_rgb(0x5c, 0xd6, 0x7a);
const ERROR_COLOR: Color32 = Color32::from_rgb(0xff, 0x6b, 0x6b);

struct Item {
    id: String,
    profile: Profile,
    /// Como está no disco, para saber se há alterações não salvas.
    saved: Profile,
}

impl Item {
    fn dirty(&self) -> bool {
        self.profile != self.saved
    }
}

struct Status {
    text: String,
    error: bool,
    at: Instant,
}

pub struct ConfigApp {
    store: Result<Store, String>,
    settings: Settings,
    items: Vec<Item>,
    /// Arquivos de perfil que não puderam ser lidos: (id, erro).
    broken: Vec<(String, String)>,
    current: usize,
    selected: Option<KeyCode>,
    editor: Editor,
    status: Option<Status>,
    engine_running: bool,
    engine_checked: Instant,
    confirm_delete: bool,
    confirm_close: bool,
    allow_close: bool,
    /// Esperando o usuário apertar uma tecla física para selecioná-la; guarda a
    /// contagem de toques de quando a captura começou.
    capturing_since: Option<u64>,
    brought_to_front: bool,
}

impl ConfigApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut(|s| s.spacing.button_padding = egui::vec2(8.0, 4.0));
        cc.egui_ctx.set_fonts(windows_fonts());

        let mut app = ConfigApp {
            store: Store::open().map_err(|e| e.to_string()),
            settings: Settings::default(),
            items: Vec::new(),
            broken: Vec::new(),
            current: 0,
            selected: None,
            editor: Editor::new(),
            status: None,
            engine_running: engine::is_running(),
            engine_checked: Instant::now(),
            confirm_delete: false,
            confirm_close: false,
            allow_close: false,
            capturing_since: None,
            brought_to_front: false,
        };
        capture::install(cc);
        if let Err(e) = app.load() {
            app.store = Err(e);
        }
        app
    }

    fn load(&mut self) -> Result<(), String> {
        let store = self.store.as_ref().map_err(Clone::clone)?;
        // Garante o perfil padrão na primeira execução; erros aparecem na lista abaixo.
        let _ = store.load_active();
        self.settings = store.load_settings().map_err(|e| e.to_string())?.0;

        self.items.clear();
        self.broken.clear();
        for entry in store.list_profiles().map_err(|e| e.to_string())? {
            match entry.profile {
                Ok(profile) => self.items.push(Item {
                    id: entry.id,
                    saved: profile.clone(),
                    profile,
                }),
                Err(e) => self.broken.push((entry.id, e.to_string())),
            }
        }
        if self.items.is_empty() {
            let profile = Profile::new("Padrão");
            store.save_profile("default", &profile).map_err(|e| e.to_string())?;
            self.items.push(Item { id: "default".into(), saved: profile.clone(), profile });
        }
        self.current = self
            .items
            .iter()
            .position(|i| i.id == self.settings.active_profile)
            .unwrap_or(0);
        Ok(())
    }

    fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = Some(Status { text: text.into(), error, at: Instant::now() });
    }

    fn any_dirty(&self) -> bool {
        self.items.iter().any(Item::dirty)
    }

    fn save_all(&mut self) {
        let Ok(store) = &self.store else { return };
        let mut errors = Vec::new();
        for item in self.items.iter_mut().filter(|i| i.dirty()) {
            match store.save_profile(&item.id, &item.profile) {
                Ok(()) => item.saved = item.profile.clone(),
                Err(e) => errors.push(e.to_string()),
            }
        }
        if errors.is_empty() {
            let msg = if engine::request_reload() {
                "Salvo e aplicado."
            } else {
                "Salvo. O motor não está rodando, então as mudanças valem quando ele for iniciado."
            };
            self.set_status(msg, false);
        } else {
            self.set_status(format!("Erro ao salvar: {}", errors.join("; ")), true);
        }
    }

    fn activate_current(&mut self) {
        let Ok(store) = &self.store else { return };
        let item = &mut self.items[self.current];
        if item.dirty() {
            if let Err(e) = store.save_profile(&item.id, &item.profile) {
                let msg = format!("Erro ao salvar: {e}");
                self.set_status(msg, true);
                return;
            }
            item.saved = item.profile.clone();
        }
        self.settings.active_profile = item.id.clone();
        let name = item.profile.name.clone();
        match store.save_settings(&self.settings) {
            Ok(()) => {
                engine::request_reload();
                self.set_status(format!("Agora o perfil em uso é \"{name}\"."), false);
            }
            Err(e) => self.set_status(format!("Erro ao salvar: {e}"), true),
        }
    }

    /// Cria um perfil já salvo em disco, para o id nunca colidir com outro.
    fn add_profile(&mut self, profile: Profile) {
        let Ok(store) = &self.store else { return };
        let id = store.new_profile_id(&profile.name);
        if let Err(e) = store.save_profile(&id, &profile) {
            self.set_status(format!("Erro ao criar perfil: {e}"), true);
            return;
        }
        self.items.push(Item { id, saved: profile.clone(), profile });
        self.current = self.items.len() - 1;
        self.selected = None;
    }

    fn delete_current(&mut self) {
        let Ok(store) = &self.store else { return };
        let id = self.items[self.current].id.clone();
        if let Err(e) = store.delete_profile(&id) {
            self.set_status(format!("Erro ao excluir: {e}"), true);
            return;
        }
        self.items.remove(self.current);
        self.current = 0;
        self.selected = None;
        if self.settings.active_profile == id {
            self.settings.active_profile = self.items[0].id.clone();
            let _ = store.save_settings(&self.settings);
            engine::request_reload();
        }
        self.set_status("Perfil excluído.", false);
    }

    fn profiles_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("Perfis");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("+ Novo").clicked() {
                let current = &self.items[self.current].profile;
                let mut profile = Profile::new("Novo perfil");
                profile.layout = current.layout;
                profile.language = current.language;
                self.add_profile(profile);
            }
            if ui.button("Duplicar").clicked() {
                let mut profile = self.items[self.current].profile.clone();
                profile.name = format!("{} (cópia)", profile.name);
                self.add_profile(profile);
            }
            let can_delete = self.items.len() > 1;
            if ui
                .add_enabled(can_delete, egui::Button::new("Excluir"))
                .on_disabled_hover_text("É preciso ter pelo menos um perfil.")
                .clicked()
            {
                self.confirm_delete = true;
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for i in 0..self.items.len() {
                let item = &self.items[i];
                let active = item.id == self.settings.active_profile;
                let mut text = String::new();
                text.push_str(if active { "● " } else { "   " });
                text.push_str(&item.profile.name);
                if item.dirty() {
                    text.push_str("  *");
                }
                let label = RichText::new(text).size(15.0);
                let response = ui.add_sized(
                    [ui.available_width(), 28.0],
                    egui::SelectableLabel::new(i == self.current, label),
                );
                if response.clicked() && i != self.current {
                    self.current = i;
                    self.selected = None;
                }
            }
            for (id, error) in &self.broken {
                ui.label(RichText::new(format!("⚠ {id}.json com erro")).color(ERROR_COLOR))
                    .on_hover_text(error);
            }
        });

        ui.add_space(8.0);
        if self.items[self.current].id == self.settings.active_profile {
            ui.label(RichText::new("● Este perfil está em uso").color(OK_COLOR));
        } else if ui.button("Usar este perfil").clicked() {
            self.activate_current();
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(10.0);
            if self.engine_running {
                ui.label(RichText::new("● Motor ativo").color(OK_COLOR));
            } else {
                if ui.button("Iniciar motor").clicked() {
                    match engine::launch() {
                        Ok(()) => self.set_status("Motor iniciado.", false),
                        Err(e) => self.set_status(format!("Não foi possível iniciar o motor: {e}"), true),
                    }
                }
                ui.label(RichText::new("● Motor parado: as teclas não estão sendo remapeadas").color(ERROR_COLOR));
            }
            ui.separator();
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let profile = &mut self.items[self.current].profile;
            ui.label("Nome:");
            ui.add(egui::TextEdit::singleline(&mut profile.name).desired_width(180.0));
            ui.add_space(12.0);

            ui.label("Formato:");
            egui::ComboBox::from_id_salt("layout")
                .selected_text(profile.layout.label())
                .show_ui(ui, |ui| {
                    for layout in Layout::ALL {
                        ui.selectable_value(&mut profile.layout, layout, layout.label());
                    }
                });
            ui.add_space(12.0);

            ui.label("Idioma:");
            egui::ComboBox::from_id_salt("language")
                .width(200.0)
                .selected_text(profile.language.label())
                .show_ui(ui, |ui| {
                    for language in Language::ALL {
                        ui.selectable_value(&mut profile.language, language, language.label());
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save = egui::Button::new(RichText::new("Salvar (Ctrl+S)").size(15.0));
                if ui.add_enabled(self.any_dirty(), save).clicked() {
                    self.save_all();
                }
            });
        });

        if let Some(status) = &self.status {
            if status.at.elapsed() < Duration::from_secs(6) {
                let color = if status.error { ERROR_COLOR } else { OK_COLOR };
                ui.label(RichText::new(&status.text).color(color));
            }
        }
        ui.add_space(4.0);
    }

    fn keyboard_area(&mut self, ui: &mut egui::Ui) {
        let profile = &self.items[self.current].profile;
        let lang = profile.language;
        let keys = capture::snapshot();

        if let Some(since) = self.capturing_since {
            if keys.presses > since {
                self.selected = keys.last;
                self.capturing_since = None;
            }
        }

        let height = ui.available_height() - 72.0;
        ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
            ui.add_space(12.0);
            if let Some(key) = keyboard::show(ui, profile, self.selected, &keys.held) {
                self.selected = Some(key);
                self.capturing_since = None;
            }
        });

        // Teclas que não aparecem no desenho deste formato (F13+, mídia, numpad no 75%...).
        let visible: Vec<KeyCode> = geometry::keys(profile.layout, lang).iter().map(|k| k.code).collect();
        let hidden: Vec<KeyCode> = KeyCode::ALL.iter().copied().filter(|k| !visible.contains(k)).collect();

        ui.horizontal(|ui| {
            if self.capturing_since.is_some() {
                if ui.button("Cancelar").clicked() {
                    self.capturing_since = None;
                }
                ui.label(
                    RichText::new("Aperte no seu teclado a tecla que você quer mapear...")
                        .color(keyboard::ACCENT)
                        .strong(),
                );
            } else {
                if ui
                    .button("Capturar tecla")
                    .on_hover_text("Clique e aperte uma tecla no seu teclado para selecioná-la aqui.")
                    .clicked()
                {
                    self.capturing_since = Some(keys.presses);
                }
                match keys.last {
                    Some(k) => {
                        let place = if hidden.contains(&k) { " (fora do desenho, está em Outras teclas)" } else { "" };
                        ui.label(format!("Última tecla apertada: {}{place}", geometry::describe(k, lang)));
                    }
                    None => {
                        ui.label(RichText::new("Aperte qualquer tecla para vê-la acender no desenho.").weak());
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Outras teclas:");
            let text = match self.selected {
                Some(k) if hidden.contains(&k) => geometry::describe(k, lang),
                _ => "escolher...".to_owned(),
            };
            egui::ComboBox::from_id_salt("hidden_keys")
                .width(200.0)
                .height(320.0)
                .selected_text(text)
                .show_ui(ui, |ui| {
                    for k in hidden {
                        let mark = if profile.mappings.contains_key(&k) { "● " } else { "" };
                        let label = format!("{mark}{}", geometry::describe(k, lang));
                        if ui.selectable_label(self.selected == Some(k), label).clicked() {
                            self.selected = Some(k);
                        }
                    }
                });
            ui.label(
                RichText::new("Teclas que não estão no desenho, como F13–F24 e mídia.")
                    .weak()
                    .small(),
            );
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.confirm_delete {
            let name = self.items[self.current].profile.name.clone();
            modal(ctx, "Excluir perfil", |ui| {
                ui.label(format!("Excluir o perfil \"{name}\"? Isso não pode ser desfeito."));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Excluir").clicked() {
                        self.confirm_delete = false;
                        self.delete_current();
                    }
                    if ui.button("Cancelar").clicked() {
                        self.confirm_delete = false;
                    }
                });
            });
        }

        if self.confirm_close {
            modal(ctx, "Alterações não salvas", |ui| {
                ui.label("Existem alterações que ainda não foram salvas.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Salvar e sair").clicked() {
                        self.save_all();
                        if !self.any_dirty() {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        self.confirm_close = false;
                    }
                    if ui.button("Sair sem salvar").clicked() {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Cancelar").clicked() {
                        self.confirm_close = false;
                    }
                });
            });
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Aberto pelo ícone da bandeja, o Windows pode deixar a janela minimizada ou atrás das outras.
        if !self.brought_to_front {
            self.brought_to_front = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        if let Err(e) = &self.store {
            let e = e.clone();
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Não foi possível abrir a configuração");
                ui.label(e);
            });
            return;
        }

        if self.engine_checked.elapsed() > Duration::from_secs(1) {
            self.engine_running = engine::is_running();
            self.engine_checked = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(1));

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) && self.any_dirty() {
            self.save_all();
        }

        if ctx.input(|i| i.viewport().close_requested()) && self.any_dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }

        egui::SidePanel::left("profiles")
            .resizable(false)
            .exact_width(230.0)
            .show(ctx, |ui| self.profiles_panel(ui));

        egui::TopBottomPanel::top("top").show(ctx, |ui| self.top_bar(ui));

        egui::TopBottomPanel::bottom("editor")
            .resizable(true)
            .default_height(420.0)
            .min_height(220.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                let item = &mut self.items[self.current];
                self.editor.show(ui, &item.id, &mut item.profile, self.selected);
            });

        egui::CentralPanel::default().show(ctx, |ui| self.keyboard_area(ui));

        self.dialogs(ctx);
    }
}

/// Segoe UI tem as setas e símbolos que faltam nas fontes padrão do egui.
fn windows_fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let dir = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or("C:\\Windows".into())).join("Fonts");
    let mut loaded = Vec::new();
    for (name, file) in [("segoe", "segoeui.ttf"), ("segoe-symbol", "seguisym.ttf")] {
        if let Ok(bytes) = std::fs::read(dir.join(file)) {
            fonts.font_data.insert(name.to_owned(), egui::FontData::from_owned(bytes));
            loaded.push(name.to_owned());
        }
    }
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        for (i, name) in loaded.into_iter().enumerate() {
            family.insert(i, name);
        }
    }
    fonts
}

fn modal(ctx: &egui::Context, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, contents);
}
