use std::time::{Duration, Instant};

use eframe::egui::{self, pos2, vec2, Align2, Color32, Margin, Rect, RichText, Sense, Stroke};
use keyconfig_core::geometry;
use keyconfig_core::store::{Settings, Store};
use keyconfig_core::{i18n, tr, KeyCode, Language, Layout, Profile, UiLang};

use crate::editor::Editor;
use crate::theme::{
    self, icon, Kind, ACCENT, BG, BORDER, CARD, CONTROL, CONTROL_BORDER, DANGER, OK, SIDEBAR, TEXT,
    TEXT_DIM, TEXT_FAINT, TEXT_MUTED,
};
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
    started: Instant,
    /// Última tentativa de trazer a janela para a frente; `None` quando já deu certo.
    focus_attempt: Option<Instant>,
    logo: Option<egui::TextureHandle>,
}

impl ConfigApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let logo = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon/icon-128.png"))
            .ok()
            .map(|icon| {
                let image = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
                cc.egui_ctx.load_texture("logo", image, egui::TextureOptions::LINEAR)
            });

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
            started: Instant::now(),
            focus_attempt: Some(Instant::now() - Duration::from_secs(1)),
            logo,
        };
        capture::install(cc);
        if let Err(e) = app.load() {
            app.store = Err(e);
        }
        i18n::set(app.settings.ui_language.unwrap_or_else(system_language));
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
                tr("Salvo e aplicado.", "Saved and applied.", "Guardado y aplicado.")
            } else {
                tr("Salvo. O motor não está rodando, então as mudanças valem quando ele for iniciado.", "Saved. The engine is not running, so the changes take effect when it starts.", "Guardado. El motor no está en marcha, así que los cambios se aplicarán cuando se inicie.")
            };
            self.set_status(msg, false);
        } else {
            self.set_status(format!("{}: {}", tr("Erro ao salvar", "Error saving", "Error al guardar"), errors.join("; ")), true);
        }
    }

    fn activate_current(&mut self) {
        let Ok(store) = &self.store else { return };
        let item = &mut self.items[self.current];
        if item.dirty() {
            if let Err(e) = store.save_profile(&item.id, &item.profile) {
                let msg = format!("{}: {e}", tr("Erro ao salvar", "Error saving", "Error al guardar"));
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
                self.set_status(format!("{} \"{name}\".", tr("Agora o perfil em uso é", "The profile in use is now", "Ahora el perfil en uso es")), false);
            }
            Err(e) => self.set_status(format!("{}: {e}", tr("Erro ao salvar", "Error saving", "Error al guardar")), true),
        }
    }

    /// Cria um perfil já salvo em disco, para o id nunca colidir com outro.
    fn add_profile(&mut self, profile: Profile) {
        let Ok(store) = &self.store else { return };
        let id = store.new_profile_id(&profile.name);
        if let Err(e) = store.save_profile(&id, &profile) {
            self.set_status(format!("{}: {e}", tr("Erro ao criar perfil", "Error creating profile", "Error al crear el perfil")), true);
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
            self.set_status(format!("{}: {e}", tr("Erro ao excluir", "Error deleting", "Error al eliminar")), true);
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
        self.set_status(tr("Perfil excluído.", "Profile deleted.", "Perfil eliminado."), false);
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        self.brand(ui);
        ui.add_space(24.0);

        ui.horizontal(|ui| {
            theme::section(ui, tr("Perfis", "Profiles", "Perfiles"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::button(ui, Kind::Ghost, Some(icon::ADD), tr("Novo", "New", "Nuevo"), true)
                    .on_hover_text(tr("Criar um perfil vazio", "Create an empty profile", "Crear un perfil vacío"))
                    .clicked()
                {
                    let current = &self.items[self.current].profile;
                    let mut profile = Profile::new(tr("Novo perfil", "New profile", "Nuevo perfil"));
                    profile.layout = current.layout;
                    profile.language = current.language;
                    self.add_profile(profile);
                }
            });
        });
        ui.add_space(4.0);

        let list_height = (ui.available_height() - 140.0).max(80.0);
        egui::ScrollArea::vertical().max_height(list_height).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for i in 0..self.items.len() {
                if self.profile_row(ui, i).clicked() && i != self.current {
                    self.current = i;
                    self.selected = None;
                }
            }
            for (id, error) in &self.broken {
                ui.label(RichText::new(format!("⚠ {id}.json {}", tr("com erro", "has errors", "con errores"))).color(DANGER)).on_hover_text(error);
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let half = (ui.available_width() - 8.0) / 2.0;
            if theme::button_with(ui, Kind::Ghost, Some(icon::COPY), tr("Duplicar", "Duplicate", "Duplicar"), None, true, half).clicked() {
                let mut profile = self.items[self.current].profile.clone();
                profile.name = format!("{} ({})", profile.name, tr("cópia", "copy", "copia"));
                self.add_profile(profile);
            }
            let can_delete = self.items.len() > 1;
            if theme::button_with(ui, Kind::Danger, Some(icon::DELETE), tr("Excluir", "Delete", "Eliminar"), None, can_delete, half)
                .on_disabled_hover_text(tr("É preciso ter pelo menos um perfil.", "You need at least one profile.", "Se necesita al menos un perfil."))
                .clicked()
            {
                self.confirm_delete = true;
            }
        });
        if self.items[self.current].id != self.settings.active_profile {
            ui.add_space(4.0);
            let width = ui.available_width();
            if theme::button_with(ui, Kind::Primary, None, tr("Usar este perfil", "Use this profile", "Usar este perfil"), None, true, width).clicked() {
                self.activate_current();
            }
        }

        // Empurra o idioma e o cartão do motor para o fim da barra (alturas aproximadas).
        let footer_height = if self.engine_running { 64.0 } else { 108.0 } + 48.0;
        ui.add_space((ui.available_height() - footer_height).max(8.0));
        self.language_picker(ui);
        ui.add_space(10.0);
        self.engine_card(ui);
    }

    /// Idioma da interface. O motor também usa esta escolha no menu e nos avisos.
    fn language_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new("\u{E774}").font(theme::icon_font(14.0)).color(TEXT_FAINT))
                .on_hover_text(tr("Idioma do app", "App language", "Idioma de la app"));
            let current = i18n::current();
            let mut chosen = current;
            egui::ComboBox::from_id_salt("ui_language")
                .width(ui.available_width() - 8.0)
                .selected_text(current.native_name())
                .show_ui(ui, |ui| {
                    for lang in UiLang::ALL {
                        ui.selectable_value(&mut chosen, lang, lang.native_name());
                    }
                });
            if chosen != current {
                self.set_language(chosen);
            }
        });
    }

    fn set_language(&mut self, lang: UiLang) {
        i18n::set(lang);
        self.settings.ui_language = Some(lang);
        let Ok(store) = &self.store else { return };
        match store.save_settings(&self.settings) {
            // O motor relê as configurações e passa a usar o novo idioma no menu.
            Ok(()) => {
                engine::request_reload();
            }
            Err(e) => self.set_status(format!("{}: {e}", tr("Erro ao salvar", "Error saving", "Error al guardar")), true),
        }
    }

    fn brand(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if let Some(logo) = &self.logo {
                ui.add(egui::Image::new(logo).fit_to_exact_size(vec2(36.0, 36.0)));
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(RichText::new("Key").font(theme::bold(16.0)).color(Color32::WHITE));
                    ui.label(RichText::new("Config").font(theme::bold(16.0)).color(ACCENT));
                });
                ui.label(RichText::new("TobieusCorp").size(12.0).color(TEXT_FAINT));
            });
        });
    }

    fn profile_row(&self, ui: &mut egui::Ui, i: usize) -> egui::Response {
        let item = &self.items[i];
        let selected = i == self.current;
        let active = item.id == self.settings.active_profile;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
        let painter = ui.painter();

        let fill = if selected {
            CONTROL
        } else if response.hovered() {
            Color32::from_rgb(0x19, 0x1d, 0x25)
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, 8.0, fill);

        let mut x = rect.left() + 12.0;
        if active {
            painter.circle_filled(pos2(x + 4.0, rect.center().y), 4.0, OK);
            x += 18.0;
        }
        let color = if selected { Color32::WHITE } else { TEXT_DIM };
        let name = painter.layout_no_wrap(item.profile.name.clone(), theme::semibold(14.0), color);
        painter.galley(pos2(x, rect.center().y - name.size().y / 2.0), name.clone(), color);
        x += name.size().x + 8.0;
        if item.dirty() {
            painter.circle_filled(pos2(x, rect.center().y), 3.0, ACCENT);
        }

        if active {
            let galley = painter.layout_no_wrap(tr("Em uso", "In use", "En uso").to_owned(), theme::medium(11.0), OK);
            let pill = Rect::from_min_size(
                pos2(rect.right() - galley.size().x - 16.0 - 10.0, rect.center().y - galley.size().y / 2.0 - 3.0),
                galley.size() + vec2(16.0, 6.0),
            );
            painter.rect_filled(pill, 99.0, OK.gamma_multiply(0.12));
            painter.galley(pill.min + vec2(8.0, 3.0), galley, OK);
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn engine_card(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(CARD)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .rounding(10.0)
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let (color, title, detail) = if self.engine_running {
                    (OK, tr("Motor ativo", "Engine running", "Motor activo"), tr("Remapeamento ligado", "Remapping on", "Reasignación activada"))
                } else {
                    (DANGER, tr("Motor parado", "Engine stopped", "Motor detenido"), tr("Remapeamento desligado", "Remapping off", "Reasignación desactivada"))
                };
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let (dot, _) = ui.allocate_exact_size(vec2(18.0, 36.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 9.0, color.gamma_multiply(0.15));
                    ui.painter().circle_filled(dot.center(), 5.0, color);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(title).font(theme::semibold(13.0)).color(TEXT));
                        ui.label(RichText::new(detail).size(12.0).color(TEXT_FAINT));
                    });
                });
                if !self.engine_running {
                    ui.add_space(4.0);
                    let width = ui.available_width();
                    if theme::button_with(ui, Kind::Primary, None, tr("Iniciar motor", "Start engine", "Iniciar motor"), None, true, width).clicked() {
                        match engine::launch() {
                            Ok(()) => self.set_status(tr("Motor iniciado.", "Engine started.", "Motor iniciado."), false),
                            Err(e) => self.set_status(format!("{}: {e}", tr("Não foi possível iniciar o motor", "Could not start the engine", "No se pudo iniciar el motor")), true),
                        }
                    }
                }
            });
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let dirty = self.any_dirty();
        let mut save = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 10.0);
            let profile = &mut self.items[self.current].profile;

            ui.horizontal(|ui| {
                theme::field_label(ui, tr("Nome", "Name", "Nombre"));
                ui.add(
                    egui::TextEdit::singleline(&mut profile.name)
                        .desired_width(180.0)
                        .margin(Margin::symmetric(12.0, 9.0))
                        .font(theme::medium(13.0)),
                );
            });
            theme::wrap_if(ui, 230.0);
            ui.horizontal(|ui| {
                theme::field_label(ui, tr("Formato", "Layout", "Formato"));
                egui::ComboBox::from_id_salt("layout")
                    .width(130.0)
                    .selected_text(RichText::new(profile.layout.label()).font(theme::semibold(13.0)))
                    .show_ui(ui, |ui| {
                        for layout in Layout::ALL {
                            ui.selectable_value(&mut profile.layout, layout, layout.label());
                        }
                    });
            });
            theme::wrap_if(ui, 300.0);
            ui.horizontal(|ui| {
                theme::field_label(ui, tr("Idioma", "Language", "Idioma"));
                egui::ComboBox::from_id_salt("language")
                    .width(220.0)
                    .selected_text(RichText::new(profile.language.label()).font(theme::semibold(13.0)))
                    .show_ui(ui, |ui| {
                        for language in Language::ALL {
                            ui.selectable_value(&mut profile.language, language, language.label());
                        }
                    });
            });

            let needed = if dirty { 290.0 } else { 140.0 };
            theme::trailing(ui, needed, |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                save = theme::button_with(ui, Kind::Primary, None, tr("Salvar", "Save", "Guardar"), Some("Ctrl+S"), dirty, 0.0).clicked();
                if dirty {
                    ui.label(RichText::new(tr("Alterações não salvas", "Unsaved changes", "Cambios sin guardar")).size(12.0).color(ACCENT));
                }
            });
        });
        if save {
            self.save_all();
        }
    }

    fn keyboard_section(&mut self, ui: &mut egui::Ui) {
        let profile = &self.items[self.current].profile;
        let lang = profile.language;
        let keys = capture::snapshot();

        if let Some(since) = self.capturing_since {
            if keys.presses > since {
                self.selected = keys.last;
                self.capturing_since = None;
            }
        }

        // Largura comum do teclado e da barra embaixo dele, centralizados.
        let width = keyboard::natural_width(profile).min(ui.available_width()).max(640.0);
        let left = ((ui.available_width() - width) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(left);
            if let Some(key) = keyboard::show(ui, profile, self.selected, &keys.held, width) {
                self.selected = Some(key);
                self.capturing_since = None;
            }
        });
        ui.add_space(18.0);

        // Teclas que não aparecem no desenho deste formato (F13+, mídia, numpad no 75%...).
        let visible: Vec<KeyCode> = geometry::keys(profile.layout, lang).iter().map(|k| k.code).collect();
        let hidden: Vec<KeyCode> = KeyCode::ALL.iter().copied().filter(|k| !visible.contains(k)).collect();

        ui.horizontal(|ui| {
            ui.add_space(left);
            ui.allocate_ui(vec2(width, 36.0), |ui| ui.horizontal_wrapped(|ui| {
                ui.set_max_width(width);
                ui.spacing_mut().item_spacing = vec2(14.0, 10.0);
                if self.capturing_since.is_some() {
                    if theme::button(ui, Kind::Primary, Some(icon::TARGET), tr("Cancelar captura", "Cancel capture", "Cancelar captura"), true).clicked() {
                        self.capturing_since = None;
                    }
                    ui.label(RichText::new(tr("Aperte no seu teclado a tecla que você quer mapear…", "Press the key you want to map on your keyboard…", "Pulsa en tu teclado la tecla que quieres asignar…")).color(ACCENT));
                } else {
                    if theme::button(ui, Kind::Normal, Some(icon::TARGET), tr("Capturar tecla", "Capture key", "Capturar tecla"), true)
                        .on_hover_text(tr("Clique e aperte uma tecla no seu teclado para selecioná-la aqui.", "Click, then press a key on your keyboard to select it here.", "Haz clic y pulsa una tecla en tu teclado para seleccionarla aquí."))
                        .clicked()
                    {
                        self.capturing_since = Some(keys.presses);
                    }
                    if let Some(k) = keys.last {
                        theme::wrap_if(ui, 200.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.label(RichText::new(tr("Última tecla", "Last key", "Última tecla")).color(TEXT_DIM));
                            key_tag(ui, &geometry::describe(k, lang));
                        });
                    }
                    theme::wrap_if(ui, 270.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        theme::field_label(ui, tr("Outras teclas", "Other keys", "Otras teclas"));
                        egui::ComboBox::from_id_salt("hidden_keys")
                            .width(160.0)
                            .selected_text(match self.selected {
                                Some(k) if hidden.contains(&k) => geometry::describe(k, lang),
                                _ => tr("Escolher…", "Choose…", "Elegir…").to_owned(),
                            })
                            .show_ui(ui, |ui| {
                                for &k in &hidden {
                                    let mapped = profile.mappings.contains_key(&k) || profile.combos.iter().any(|c| c.key == k);
                                    let mark = if mapped { "● " } else { "" };
                                    let label = format!("{mark}{}", geometry::describe(k, lang));
                                    if ui.selectable_label(self.selected == Some(k), label).clicked() {
                                        self.selected = Some(k);
                                    }
                                }
                            });
                    });
                }

                theme::trailing(ui, 400.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    legend(ui, tr("Apertada", "Pressed", "Pulsada"), Legend::Pressed);
                    legend(ui, tr("Desligada", "Off", "Desactivada"), Legend::Off);
                    legend(ui, tr("Selecionada", "Selected", "Seleccionada"), Legend::Selected);
                    legend(ui, tr("Remapeada", "Remapped", "Reasignada"), Legend::Remapped);
                });
            }));
        });
        ui.horizontal(|ui| {
            ui.add_space(left);
            ui.label(
                RichText::new(tr("Use “Outras teclas” para o que não aparece no desenho, como F13–F24 e teclas de mídia.", "Use “Other keys” for anything not in the drawing, such as F13–F24 and media keys.", "Usa “Otras teclas” para lo que no aparece en el dibujo, como F13–F24 y teclas multimedia."))
                    .size(12.0)
                    .color(TEXT_FAINT),
            );
        });
    }

    /// Aberto pelo ícone da bandeja, o Windows pode deixar a janela minimizada ou
    /// atrás das outras. O eframe só mostra a janela depois dos primeiros quadros,
    /// e pedir foco antes disso não tem efeito, então insiste por alguns segundos
    /// até ela estar visível e ativa.
    fn bring_to_front(&mut self, ctx: &egui::Context) {
        let Some(last_try) = self.focus_attempt else { return };
        let (focused, minimized) = ctx.input(|i| {
            let v = i.viewport();
            (v.focused.unwrap_or(false), v.minimized.unwrap_or(false))
        });
        if focused && !minimized || self.started.elapsed() > Duration::from_secs(4) {
            self.focus_attempt = None;
            return;
        }
        if last_try.elapsed() > Duration::from_millis(250) {
            if minimized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.focus_attempt = Some(Instant::now());
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn status_toast(&self, ctx: &egui::Context) {
        let Some(status) = &self.status else { return };
        let age = status.at.elapsed().as_secs_f32();
        if age > 5.0 {
            return;
        }
        let fade = ((5.0 - age) / 0.5).clamp(0.0, 1.0);
        let color = if status.error { DANGER } else { OK };
        egui::Area::new(egui::Id::new("toast"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-28.0, -28.0))
            .interactable(false)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(CARD.gamma_multiply(fade))
                    .stroke(Stroke::new(1.0_f32, BORDER.gamma_multiply(fade)))
                    .rounding(10.0)
                    .shadow(egui::Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: Color32::from_black_alpha((120.0 * fade) as u8) })
                    .inner_margin(Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (dot, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
                            ui.painter().circle_filled(dot.center(), 4.0, color.gamma_multiply(fade));
                            ui.label(RichText::new(&status.text).color(TEXT.gamma_multiply(fade)));
                        });
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.confirm_delete {
            let name = self.items[self.current].profile.name.clone();
            modal(ctx, tr("Excluir perfil", "Delete profile", "Eliminar perfil"), |ui| {
                ui.label(RichText::new(format!("{} \"{name}\"? {}", tr("Excluir o perfil", "Delete the profile", "¿Eliminar el perfil"), tr("Isso não pode ser desfeito.", "This cannot be undone.", "No se puede deshacer."))).color(TEXT_DIM));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if theme::button(ui, Kind::Danger, Some(icon::DELETE), tr("Excluir", "Delete", "Eliminar"), true).clicked() {
                        self.confirm_delete = false;
                        self.delete_current();
                    }
                    if theme::button(ui, Kind::Ghost, None, tr("Cancelar", "Cancel", "Cancelar"), true).clicked() {
                        self.confirm_delete = false;
                    }
                });
            });
        }

        if self.confirm_close {
            modal(ctx, tr("Alterações não salvas", "Unsaved changes", "Cambios sin guardar"), |ui| {
                ui.label(RichText::new(tr("Existem alterações que ainda não foram salvas.", "There are changes that have not been saved yet.", "Hay cambios que aún no se han guardado.")).color(TEXT_DIM));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if theme::button(ui, Kind::Primary, None, tr("Salvar e sair", "Save and quit", "Guardar y salir"), true).clicked() {
                        self.save_all();
                        if !self.any_dirty() {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        self.confirm_close = false;
                    }
                    if theme::button(ui, Kind::Ghost, None, tr("Sair sem salvar", "Quit without saving", "Salir sin guardar"), true).clicked() {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if theme::button(ui, Kind::Ghost, None, tr("Cancelar", "Cancel", "Cancelar"), true).clicked() {
                        self.confirm_close = false;
                    }
                });
            });
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.bring_to_front(ctx);

        if let Err(e) = &self.store {
            let e = e.clone();
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading(tr("Não foi possível abrir a configuração", "Could not open the configuration", "No se pudo abrir la configuración"));
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

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(260.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(SIDEBAR)
                    .inner_margin(Margin::symmetric(16.0, 20.0))
                    .stroke(Stroke::new(1.0_f32, BORDER)),
            )
            .show(ctx, |ui| self.sidebar(ui));

        egui::TopBottomPanel::top("header")
            .show_separator_line(false)
            .frame(egui::Frame::none().fill(BG).inner_margin(Margin::symmetric(28.0, 16.0)))
            .show(ctx, |ui| {
                self.header(ui);
                // Linha divisória embaixo do cabeçalho, de ponta a ponta (fora da margem interna).
                let r = ui.min_rect().expand2(vec2(28.0, 16.0));
                let painter = ui.painter().clone().with_clip_rect(r.expand(1.0));
                painter.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0_f32, BORDER));
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    egui::Frame::none().inner_margin(Margin::same(28.0)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.keyboard_section(ui);
                        ui.add_space(28.0);
                        let item = &mut self.items[self.current];
                        self.editor.show(ui, &item.id, &mut item.profile, self.selected);
                    });
                });
            });

        self.dialogs(ctx);
        self.status_toast(ctx);
    }
}

enum Legend {
    Remapped,
    Selected,
    Off,
    Pressed,
}

/// Idioma do Windows, usado enquanto o usuário não escolhe um.
fn system_language() -> UiLang {
    let langid = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    UiLang::from_windows_langid(langid)
}

fn legend(ui: &mut egui::Ui, text: &str, kind: Legend) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
        let painter = ui.painter();
        match kind {
            Legend::Remapped => {
                painter.circle_filled(rect.center(), 4.0, ACCENT);
            }
            Legend::Selected => {
                painter.rect_stroke(rect.shrink(1.0), 3.0, Stroke::new(2.0_f32, ACCENT));
            }
            Legend::Off => {
                let points = vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
                painter.extend(egui::Shape::dashed_line(&points, Stroke::new(1.0_f32, TEXT_MUTED), 2.0, 2.0));
            }
            Legend::Pressed => {
                painter.rect(rect.shrink(1.0), 3.0, keyboard::PRESSED.gamma_multiply(0.3), Stroke::new(1.0_f32, keyboard::PRESSED));
            }
        }
        ui.label(RichText::new(text).size(12.0).color(TEXT_DIM));
    });
}

/// Nome de tecla em destaque, como `PgDn` na barra embaixo do teclado.
fn key_tag(ui: &mut egui::Ui, text: &str) {
    egui::Frame::none()
        .fill(CONTROL)
        .stroke(Stroke::new(1.0_f32, CONTROL_BORDER))
        .rounding(6.0)
        .inner_margin(Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(theme::mono_medium(13.0)).color(TEXT));
        });
}

fn modal(ctx: &egui::Context, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    // Escurece o fundo para destacar a pergunta.
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("modal_dim"))
        .fixed_pos(screen.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(160));
        });
    egui::Window::new(RichText::new(title).font(theme::semibold(16.0)).color(TEXT))
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(egui::Frame::window(&ctx.style()).inner_margin(Margin::same(24.0)))
        .show(ctx, contents);
}
