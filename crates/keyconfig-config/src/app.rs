use std::time::{Duration, Instant};

use eframe::egui::{self, vec2, Align2, Color32, Margin, RichText, Sense, Stroke};
use keyconfig_core::geometry;
use keyconfig_core::store::{Settings, Store};
use keyconfig_core::{KeyCode, Language, Layout, Profile};

use crate::editor::Editor;
use crate::theme::{self, ACCENT, BG, BORDER, CYAN, ERROR, OK, PANEL, SURFACE, SURFACE_HI, TEXT_DIM, TEXT_FAINT, WARN};
use crate::{capture, engine, keyboard};

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
        theme::apply(&cc.egui_ctx);

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
        brand(ui);
        ui.add_space(22.0);

        ui.horizontal(|ui| {
            theme::section(ui, "Perfis");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+ Novo").on_hover_text("Criar um perfil vazio").clicked() {
                    let current = &self.items[self.current].profile;
                    let mut profile = Profile::new("Novo perfil");
                    profile.layout = current.layout;
                    profile.language = current.language;
                    self.add_profile(profile);
                }
            });
        });
        ui.add_space(6.0);

        let list_height = (ui.available_height() - 210.0).max(80.0);
        egui::ScrollArea::vertical().max_height(list_height).show(ui, |ui| {
            for i in 0..self.items.len() {
                if self.profile_row(ui, i).clicked() && i != self.current {
                    self.current = i;
                    self.selected = None;
                }
                ui.add_space(4.0);
            }
            for (id, error) in &self.broken {
                ui.label(RichText::new(format!("⚠ {id}.json com erro")).color(ERROR)).on_hover_text(error);
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Duplicar").clicked() {
                let mut profile = self.items[self.current].profile.clone();
                profile.name = format!("{} (cópia)", profile.name);
                self.add_profile(profile);
            }
            let can_delete = self.items.len() > 1;
            if ui
                .add_enabled(can_delete, egui::Button::new(RichText::new("Excluir").color(ERROR.gamma_multiply(0.9))))
                .on_disabled_hover_text("É preciso ter pelo menos um perfil.")
                .clicked()
            {
                self.confirm_delete = true;
            }
        });
        if self.items[self.current].id != self.settings.active_profile {
            ui.add_space(6.0);
            let button = theme::primary("Usar este perfil").min_size(vec2(ui.available_width(), 34.0));
            if ui.add(button).clicked() {
                self.activate_current();
            }
        }

        // Empurra o cartão do motor para o fim da barra (altura aproximada do cartão).
        let card_height = if self.engine_running { 96.0 } else { 140.0 };
        ui.add_space((ui.available_height() - card_height).max(8.0));
        self.engine_card(ui);
    }

    fn profile_row(&self, ui: &mut egui::Ui, i: usize) -> egui::Response {
        let item = &self.items[i];
        let selected = i == self.current;
        let active = item.id == self.settings.active_profile;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
        let painter = ui.painter();

        let fill = if selected {
            SURFACE_HI
        } else if response.hovered() {
            SURFACE
        } else {
            Color32::TRANSPARENT
        };
        painter.rect(rect, 9.0, fill, if selected { Stroke::new(1.0_f32, BORDER) } else { Stroke::NONE });
        if selected {
            let bar = egui::Rect::from_min_size(rect.min + vec2(0.0, 12.0), vec2(3.0, rect.height() - 24.0));
            painter.rect_filled(bar, 2.0, ACCENT);
        }

        let name_color = if selected { Color32::WHITE } else { theme::TEXT };
        painter.text(rect.min + vec2(14.0, 9.0), Align2::LEFT_TOP, &item.profile.name, theme::bold(14.5), name_color);
        let detail = format!(
            "{} · {} · {} tecla(s)",
            item.profile.layout.label(),
            short_language(item.profile.language),
            item.profile.mappings.len()
        );
        painter.text(rect.min + vec2(14.0, 30.0), Align2::LEFT_TOP, detail, egui::FontId::proportional(11.5), TEXT_DIM);

        let mut right = rect.right() - 10.0;
        if active {
            let text = "EM USO";
            let galley = painter.layout_no_wrap(text.into(), theme::bold(9.5), OK);
            let pill = egui::Rect::from_min_size(
                egui::pos2(right - galley.size().x - 14.0, rect.top() + 10.0),
                galley.size() + vec2(14.0, 4.0),
            );
            painter.rect_filled(pill, 99.0, OK.gamma_multiply(0.16));
            painter.galley(pill.min + vec2(7.0, 2.0), galley, OK);
            right = pill.left() - 6.0;
        }
        if item.dirty() {
            painter.circle_filled(egui::pos2(right - 4.0, rect.top() + 17.0), 3.5, WARN);
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn engine_card(&mut self, ui: &mut egui::Ui) {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                let (color, title, detail) = if self.engine_running {
                    (OK, "Motor ativo", "Seus mapeamentos estão funcionando.")
                } else {
                    (ERROR, "Motor parado", "As teclas não estão sendo remapeadas.")
                };
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(10.0, 18.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.5, color);
                    ui.painter().circle_stroke(dot.center(), 7.0, Stroke::new(2.0_f32, color.gamma_multiply(0.25)));
                    ui.label(RichText::new(title).font(theme::bold(14.0)).color(color));
                });
                ui.label(RichText::new(detail).size(12.0).color(TEXT_DIM));
                if !self.engine_running {
                    ui.add_space(4.0);
                    let button = theme::primary("Iniciar motor").min_size(vec2(ui.available_width(), 32.0));
                    if ui.add(button).clicked() {
                        match engine::launch() {
                            Ok(()) => self.set_status("Motor iniciado.", false),
                            Err(e) => self.set_status(format!("Não foi possível iniciar o motor: {e}"), true),
                        }
                    }
                }
            });
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let dirty = self.any_dirty();
        let mut save = false;
        ui.horizontal(|ui| {
            let profile = &mut self.items[self.current].profile;
            ui.vertical(|ui| {
                theme::section(ui, "Perfil");
                ui.add(
                    egui::TextEdit::singleline(&mut profile.name)
                        .font(theme::bold(22.0))
                        .frame(false)
                        .desired_width(260.0)
                        .hint_text("Nome do perfil"),
                )
                .on_hover_text("Clique para renomear");
            });
            ui.add_space(16.0);

            ui.vertical(|ui| {
                theme::section(ui, "Formato");
                egui::ComboBox::from_id_salt("layout")
                    .width(150.0)
                    .selected_text(profile.layout.label())
                    .show_ui(ui, |ui| {
                        for layout in Layout::ALL {
                            ui.selectable_value(&mut profile.layout, layout, layout.label());
                        }
                    });
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                theme::section(ui, "Idioma");
                egui::ComboBox::from_id_salt("language")
                    .width(210.0)
                    .selected_text(profile.language.label())
                    .show_ui(ui, |ui| {
                        for language in Language::ALL {
                            ui.selectable_value(&mut profile.language, language, language.label());
                        }
                    });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button = theme::primary("Salvar").min_size(vec2(110.0, 36.0));
                save = ui.add_enabled(dirty, button).on_hover_text("Ctrl+S").clicked();
                if dirty {
                    theme::pill(ui, "NÃO SALVO", WARN);
                }
            });
        });
        if save {
            self.save_all();
        }
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

        let height = ui.available_height() - 56.0;
        ui.allocate_ui(vec2(ui.available_width(), height), |ui| {
            ui.add_space(14.0);
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
                    RichText::new("Aperte no seu teclado a tecla que você quer mapear…")
                        .font(theme::bold(14.0))
                        .color(CYAN),
                );
            } else {
                if ui
                    .add(theme::primary("⌨  Capturar tecla"))
                    .on_hover_text("Clique e aperte uma tecla no seu teclado para selecioná-la aqui.")
                    .clicked()
                {
                    self.capturing_since = Some(keys.presses);
                }

                egui::ComboBox::from_id_salt("hidden_keys")
                    .width(180.0)
                    .selected_text(match self.selected {
                        Some(k) if hidden.contains(&k) => geometry::describe(k, lang),
                        _ => "Outras teclas…".to_owned(),
                    })
                    .show_ui(ui, |ui| {
                        for &k in &hidden {
                            let mark = if profile.mappings.contains_key(&k) { "● " } else { "" };
                            let label = format!("{mark}{}", geometry::describe(k, lang));
                            if ui.selectable_label(self.selected == Some(k), label).clicked() {
                                self.selected = Some(k);
                            }
                        }
                    })
                    .response
                    .on_hover_text("Teclas que não estão no desenho, como F13–F24 e mídia.");

                if let Some(k) = keys.last {
                    let place = if hidden.contains(&k) { " · em Outras teclas" } else { "" };
                    ui.label(
                        RichText::new(format!("Última tecla: {}{place}", geometry::describe(k, lang)))
                            .color(TEXT_DIM),
                    );
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                legend(ui, "Apertada", keyboard::PRESSED);
                legend(ui, "Selecionada", CYAN);
                legend(ui, "Mapeada", keyboard::MAPPED);
            });
        });
    }

    fn status_toast(&self, ctx: &egui::Context) {
        let Some(status) = &self.status else { return };
        let age = status.at.elapsed().as_secs_f32();
        if age > 5.0 {
            return;
        }
        let fade = ((5.0 - age) / 0.5).clamp(0.0, 1.0);
        let color = if status.error { ERROR } else { OK };
        egui::Area::new(egui::Id::new("toast"))
            .anchor(Align2::CENTER_TOP, vec2(0.0, 84.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(theme::SURFACE.gamma_multiply(fade))
                    .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.6 * fade)))
                    .rounding(10.0)
                    .shadow(egui::Shadow { offset: vec2(0.0, 6.0), blur: 18.0, spread: 0.0, color: Color32::from_black_alpha((120.0 * fade) as u8) })
                    .inner_margin(Margin::symmetric(16.0, 10.0))
                    .show(ui, |ui| {
                        let icon = if status.error { "✖" } else { "✔" };
                        ui.label(RichText::new(format!("{icon}  {}", status.text)).color(color.gamma_multiply(fade)));
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.confirm_delete {
            let name = self.items[self.current].profile.name.clone();
            modal(ctx, "Excluir perfil", |ui| {
                ui.label(format!("Excluir o perfil \"{name}\"? Isso não pode ser desfeito."));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let delete = theme::primary("Excluir").fill(ERROR.gamma_multiply(0.85));
                    if ui.add(delete).clicked() {
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
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.add(theme::primary("Salvar e sair")).clicked() {
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

        let frame = |fill, x: f32, y: f32| egui::Frame::none().fill(fill).inner_margin(Margin::symmetric(x, y));

        egui::SidePanel::left("profiles")
            .resizable(false)
            .exact_width(260.0)
            .frame(frame(PANEL, 16.0, 18.0).stroke(Stroke::new(1.0_f32, BORDER)))
            .show(ctx, |ui| self.profiles_panel(ui));

        egui::TopBottomPanel::top("top")
            .frame(frame(BG, 24.0, 16.0))
            .show_separator_line(false)
            .show(ctx, |ui| self.top_bar(ui));

        egui::TopBottomPanel::bottom("editor")
            .resizable(true)
            .default_height(400.0)
            .min_height(220.0)
            .frame(frame(PANEL, 24.0, 16.0).stroke(Stroke::new(1.0_f32, BORDER)))
            .show(ctx, |ui| {
                let item = &mut self.items[self.current];
                self.editor.show(ui, &item.id, &mut item.profile, self.selected);
            });

        egui::CentralPanel::default()
            .frame(frame(BG, 24.0, 4.0))
            .show(ctx, |ui| self.keyboard_area(ui));

        self.dialogs(ctx);
        self.status_toast(ctx);
    }
}

/// Cabeçalho da barra lateral com o nome do app nas cores do logo.
fn brand(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(RichText::new("Key").font(theme::bold(26.0)).color(Color32::WHITE));
        ui.label(RichText::new("config").font(theme::bold(26.0)).color(ACCENT));
    });
    ui.label(RichText::new("Universal Keyboard Software").size(11.5).color(TEXT_FAINT));
}

fn legend(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(12.0).color(TEXT_DIM));
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
    ui.painter().rect(rect, 3.0, color.gamma_multiply(0.6), Stroke::new(1.5_f32, color));
    ui.add_space(8.0);
}

fn short_language(language: Language) -> &'static str {
    match language {
        Language::PtBrAbnt2 => "ABNT2",
        Language::EnUs => "US",
        Language::EnUsIntl => "US-Intl",
    }
}

fn modal(ctx: &egui::Context, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    // Escurece o fundo para destacar a pergunta.
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("modal_dim"))
        .fixed_pos(screen.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(150));
        });
    egui::Window::new(RichText::new(title).font(theme::bold(16.0)))
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(egui::Frame::window(&ctx.style()).inner_margin(Margin::same(20.0)))
        .show(ctx, contents);
}
