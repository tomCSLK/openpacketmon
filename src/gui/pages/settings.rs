//! The settings. They take effect with "Save"; those of the packet store with the next capture.

use std::net::IpAddr;

use eframe::egui::{self, ComboBox, RichText, ScrollArea, Slider, TextEdit, Ui};

use crate::config::packet_store::PacketStoreConf;
use crate::config::settings::ThemeChoice;
use crate::engine::state::EngineState;
use crate::gui::load::Task;
use crate::gui::pages::head;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, language_of, sound, theme_preference};
use crate::mcp::Status;
use crate::mmdb::geo::DownloadError;
use crate::view::i18n::Lang;
use crate::view::settings::{
    Settings, SettingsChange, apply_settings, parse_addresses, parse_sound, settings, sound_code,
};

/// How to connect Claude Code, which starts this command itself
const MCP_COMMAND: &str = "claude mcp add openpacketmon -- openpacketmon mcp";

/// What the form holds until it is saved
struct Form {
    language: String,
    page_size: usize,
    theme: ThemeChoice,
    favorite_active: bool,
    favorite_sound: &'static str,
    addresses: String,
    volume: u8,
    mmdb_country: String,
    mmdb_asn: String,
    update_checks: bool,
    packets_enabled: bool,
    packets_max: usize,
    packets_memory: usize,
    packets_headers: bool,
    mcp_enabled: bool,
    /// Never stored: it ends with the window
    mcp_content: bool,
}

impl Form {
    fn new(s: &Settings, state: &EngineState) -> Self {
        Self {
            language: s
                .languages
                .iter()
                .find(|c| c.selected)
                .map_or_else(String::new, |c| c.value.to_owned()),
            page_size: s
                .page_sizes
                .iter()
                .find(|(_, on)| *on)
                .map_or(50, |(n, _)| *n),
            theme: state.conf.settings.theme,
            favorite_active: s.favorite_active,
            favorite_sound: sound_code(
                state
                    .conf
                    .settings
                    .notifications
                    .favorite_notification
                    .sound,
            ),
            addresses: s.favorite_addresses.clone(),
            volume: s.volume,
            mmdb_country: s.mmdb_country.clone(),
            mmdb_asn: s.mmdb_asn.clone(),
            update_checks: s.update_checks,
            packets_enabled: s.packets_enabled,
            packets_max: s
                .packets_max
                .iter()
                .find(|(_, _, on)| *on)
                .map_or(100_000, |(_, n, _)| *n),
            packets_memory: state.conf.packet_store.max_megabytes(),
            packets_headers: s.packets_headers,
            mcp_enabled: s.mcp_enabled,
            mcp_content: false,
        }
    }
}

#[derive(Default)]
pub struct State {
    view: Option<(Settings, Form)>,
    flash: Option<String>,
    invalid: Option<String>,
    geo: Task<Result<(), DownloadError>>,
    geo_error: Option<String>,
}

impl State {
    /// The form is filled again when the page opens the next time
    pub fn close(&mut self) {
        self.view = None;
        self.flash = None;
        self.invalid = None;
        self.geo_error = None;
    }
}

fn memory_label(megabytes: usize) -> String {
    if megabytes >= 1024 {
        format!("{} GiB", megabytes / 1024)
    } else {
        format!("{megabytes} MiB")
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    if app.pages.settings.view.is_none() {
        let state = app.engine.lock();
        let view = settings(t, &state);
        let mut form = Form::new(&view, &state);
        drop(state);
        form.mcp_content = app.mcp.content_allowed();
        app.pages.settings.view = Some((view, form));
    }
    if let Some(result) = app.pages.settings.geo.take() {
        match result {
            Ok(()) => {
                app.engine.lock().reload_geo();
                let view = settings(t, &app.engine.lock());
                if let Some((old, _)) = &mut app.pages.settings.view {
                    *old = view;
                }
                app.pages.settings.flash = Some(t.get("flash-geo-downloaded"));
            }
            Err(DownloadError::Busy) => {
                app.pages.settings.geo_error = Some(t.get("settings-geo-busy"));
            }
            Err(_) => app.pages.settings.geo_error = Some(t.get("settings-geo-failed")),
        }
    }
    head(
        app,
        ui,
        &t.get("nav-settings"),
        Some(&t.get("settings-subtitle")),
        |_| {},
    );
    let downloading = app.pages.settings.geo.busy();
    let mcp = app.mcp.status();
    let (mut save, mut download, mut preview) = (false, false, false);
    let state = &mut app.pages.settings;
    if let Some(flash) = &state.flash {
        widgets::flash(ui, flash);
    }
    let invalid = state.invalid.clone();
    let geo_error = state.geo_error.clone();
    let Some((view, form)) = &mut state.view else {
        return;
    };
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.set_max_width(760.0);
        display_card(ui, t, view, form);
        notifications_card(ui, t, view, form, invalid.as_deref(), &mut preview);
        geo_card(
            ui,
            t,
            view,
            form,
            geo_error.as_deref(),
            downloading,
            &mut download,
        );
        store_card(ui, t, view, form);
        mcp_card(ui, t, form, mcp);
        widgets::card(
            ui,
            &t.get("settings-updates"),
            |_| {},
            |ui| {
                ui.checkbox(&mut form.update_checks, t.get("settings-update-checks"));
                widgets::hint(ui, &t.get("settings-update-hint"));
            },
        );
        ui.add_space(4.0);
        save = widgets::button(ui, t.get("save"), Kind::Primary).clicked();
        ui.add_space(12.0);
    });
    if preview {
        sound::play(parse_sound(form.favorite_sound), form.volume);
    }
    if save {
        store(app, &ctx);
    }
    if download {
        let engine = app.engine.clone();
        let state = &mut app.pages.settings;
        state.geo_error = None;
        state.geo.start(&ctx, move || engine.download_geo());
    }
}

fn display_card(ui: &mut Ui, t: Lang, view: &Settings, form: &mut Form) {
    widgets::card(
        ui,
        &t.get("settings-display"),
        |_| {},
        |ui| {
            egui::Grid::new("display")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    widgets::fact_label(ui, &t.get("settings-language"));
                    let label = |c: &crate::view::settings::Choice| {
                        if c.value.is_empty() {
                            t.get("gui-language-system")
                        } else {
                            c.label.clone()
                        }
                    };
                    let chosen = view
                        .languages
                        .iter()
                        .find(|c| c.value == form.language)
                        .map(label)
                        .unwrap_or_default();
                    ComboBox::from_id_salt("language")
                        .width(220.0)
                        .selected_text(chosen)
                        .show_ui(ui, |ui| {
                            for c in &view.languages {
                                ui.selectable_value(
                                    &mut form.language,
                                    c.value.to_owned(),
                                    label(c),
                                );
                            }
                        });
                    ui.end_row();

                    widgets::fact_label(ui, &t.get("gui-theme"));
                    let themes = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];
                    let labels = [
                        t.get("gui-theme-system"),
                        t.get("gui-theme-light"),
                        t.get("gui-theme-dark"),
                    ];
                    let at = themes.iter().position(|c| *c == form.theme).unwrap_or(0);
                    if let Some(i) = widgets::segmented(ui, &labels, at) {
                        form.theme = themes[i];
                    }
                    ui.end_row();

                    widgets::fact_label(ui, &t.get("settings-page-size"));
                    ComboBox::from_id_salt("page-size")
                        .width(90.0)
                        .selected_text(form.page_size.to_string())
                        .show_ui(ui, |ui| {
                            for (n, _) in &view.page_sizes {
                                ui.selectable_value(&mut form.page_size, *n, n.to_string());
                            }
                        });
                    ui.end_row();
                });
        },
    );
}

fn notifications_card(
    ui: &mut Ui,
    t: Lang,
    view: &Settings,
    form: &mut Form,
    invalid: Option<&str>,
    preview: &mut bool,
) {
    widgets::card(
        ui,
        &t.get("nav-notifications"),
        |_| {},
        |ui| {
            if let Some(invalid) = invalid {
                widgets::error(ui, invalid);
            }
            ui.checkbox(
                &mut form.favorite_active,
                t.get("settings-favorite-notification"),
            );
            widgets::hint(ui, &t.get("settings-favorite-hint"));
            ui.label(t.get("settings-favorite-addresses"));
            ui.add(
                TextEdit::multiline(&mut form.addresses)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("192.168.1.20\n2001:db8::1")
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            widgets::hint(ui, &t.get("settings-favorite-addresses-hint"));
            ui.horizontal(|ui| {
                ui.label(t.get("settings-sound"));
                let chosen = view
                    .favorite_sounds
                    .iter()
                    .find(|c| c.value == form.favorite_sound)
                    .map(|c| c.label.clone())
                    .unwrap_or_default();
                ComboBox::from_id_salt("sound")
                    .width(160.0)
                    .selected_text(chosen)
                    .show_ui(ui, |ui| {
                        for c in &view.favorite_sounds {
                            let picked = ui
                                .selectable_value(&mut form.favorite_sound, c.value, &c.label)
                                .clicked();
                            *preview |= picked;
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(t.get("settings-volume"));
                // the handle takes its size from the height of the slider
                ui.spacing_mut().interact_size.y = ui.text_style_height(&egui::TextStyle::Body);
                let slider = ui.add(
                    Slider::new(&mut form.volume, 0..=100)
                        .step_by(5.0)
                        .show_value(false),
                );
                *preview |= slider.drag_stopped();
                ui.label(format!("{} %", form.volume));
            });
            widgets::hint(ui, &t.get("gui-sound-hint"));
        },
    );
}

fn geo_card(
    ui: &mut Ui,
    t: Lang,
    view: &Settings,
    form: &mut Form,
    error: Option<&str>,
    downloading: bool,
    download: &mut bool,
) {
    let p = crate::gui::theme::palette(ui.ctx());
    widgets::card(
        ui,
        &t.get("settings-geo"),
        |_| {},
        |ui| {
            if let Some(error) = error {
                widgets::error(ui, error);
            }
            widgets::facts(ui, "geo", |ui| {
                for row in &view.geo_rows {
                    widgets::fact_label(ui, &row.label);
                    ui.vertical(|ui| match &row.file {
                        Some((path, kind)) => {
                            ui.horizontal(|ui| {
                                ui.label(format!("{kind}, {}", row.built));
                                if row.old {
                                    ui.colored_label(p.warn, t.get("settings-geo-old"));
                                }
                            });
                            widgets::hint(ui, path);
                        }
                        None => {
                            ui.label(t.get("settings-geo-none"));
                        }
                    });
                    ui.end_row();
                }
            });
            if downloading {
                widgets::disabled_button(ui, t.get("settings-geo-downloading"));
            } else {
                *download =
                    widgets::button(ui, t.get("settings-geo-download"), Kind::Secondary).clicked();
            }
            widgets::hint(
                ui,
                &t.get1("settings-geo-download-hint", "dir", view.geo_dir.as_str()),
            );
            ui.hyperlink_to("IP Geolocation by DB-IP", "https://db-ip.com");
            ui.columns(2, |c| {
                c[0].label(t.get("settings-mmdb-country"));
                c[0].add(
                    TextEdit::singleline(&mut form.mmdb_country)
                        .hint_text("GeoLite2-Country.mmdb")
                        .desired_width(f32::INFINITY),
                );
                c[1].label(t.get("settings-mmdb-asn"));
                c[1].add(
                    TextEdit::singleline(&mut form.mmdb_asn)
                        .hint_text("GeoLite2-ASN.mmdb")
                        .desired_width(f32::INFINITY),
                );
            });
            widgets::hint(ui, &t.get("settings-mmdb-hint"));
        },
    );
}

fn store_card(ui: &mut Ui, t: Lang, view: &Settings, form: &mut Form) {
    widgets::card(
        ui,
        &t.get("settings-packet-store"),
        |_| {},
        |ui| {
            ui.checkbox(&mut form.packets_enabled, t.get("settings-packets-enabled"));
            egui::Grid::new("store")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    widgets::fact_label(ui, &t.get("settings-packets-max"));
                    let chosen = view
                        .packets_max
                        .iter()
                        .find(|(_, n, _)| *n == form.packets_max)
                        .map(|(label, _, _)| label.clone())
                        .unwrap_or_default();
                    ComboBox::from_id_salt("packets-max")
                        .width(140.0)
                        .selected_text(chosen)
                        .show_ui(ui, |ui| {
                            for (label, n, _) in &view.packets_max {
                                ui.selectable_value(&mut form.packets_max, *n, label);
                            }
                        });
                    ui.end_row();

                    widgets::fact_label(ui, &t.get("gui-packets-memory"));
                    ComboBox::from_id_salt("packets-memory")
                        .width(140.0)
                        .selected_text(memory_label(form.packets_memory))
                        .show_ui(ui, |ui| {
                            for n in PacketStoreConf::MEGABYTES_CHOICES {
                                ui.selectable_value(&mut form.packets_memory, n, memory_label(n));
                            }
                        });
                    ui.end_row();
                });
            ui.checkbox(&mut form.packets_headers, t.get("settings-packets-headers"));
            widgets::hint(ui, &t.get("settings-packets-hint"));
        },
    );
}

fn mcp_card(ui: &mut Ui, t: Lang, form: &mut Form, status: Status) {
    widgets::card(
        ui,
        &t.get("settings-mcp"),
        |_| {},
        |ui| {
            if status == Status::Failed {
                widgets::error(ui, &t.get("settings-mcp-failed"));
            }
            ui.checkbox(&mut form.mcp_enabled, t.get("settings-mcp-enabled"));
            widgets::hint(ui, &t.get("settings-mcp-hint"));
            if !form.mcp_enabled {
                form.mcp_content = false;
            }
            ui.add_enabled(
                form.mcp_enabled,
                egui::Checkbox::new(&mut form.mcp_content, t.get("settings-mcp-content")),
            );
            widgets::hint(ui, &t.get("settings-mcp-content-hint"));
            if let Status::Active { calls } = status {
                ui.label(t.get1("settings-mcp-active", "calls", calls.to_string()));
            }
            ui.label(t.get("settings-mcp-command"));
            ui.label(RichText::new(MCP_COMMAND).monospace());
        },
    );
}

fn store(app: &mut App, ctx: &egui::Context) {
    let t = app.t;
    let Some((_, form)) = &app.pages.settings.view else {
        return;
    };
    let (addresses, invalid): (Vec<IpAddr>, Vec<String>) = parse_addresses(&form.addresses);
    let change = SettingsChange {
        language: form.language.clone(),
        page_size: form.page_size,
        favorite_active: form.favorite_active,
        favorite_sound: parse_sound(form.favorite_sound),
        addresses,
        volume: form.volume,
        mmdb_country: form.mmdb_country.clone(),
        mmdb_asn: form.mmdb_asn.clone(),
        update_checks: form.update_checks,
        packets_enabled: form.packets_enabled,
        packets_max: form.packets_max,
        packets_memory: Some(form.packets_memory),
        packets_headers: form.packets_headers,
        mcp_enabled: form.mcp_enabled,
    };
    let theme = form.theme;
    let content = form.mcp_enabled && form.mcp_content;
    {
        let mut state = app.engine.lock();
        apply_settings(&mut state, &change);
        state.conf.settings.theme = theme;
    }
    app.engine.store_conf();
    app.mcp.apply(change.mcp_enabled);
    app.mcp.allow_content(content);
    ctx.set_theme(theme_preference(theme));
    let language = app.engine.lock().conf.language.clone();
    app.t = language_of(&language);
    let t = if app.t == t { t } else { app.t };
    let state = &mut app.pages.settings;
    state.close();
    state.flash = Some(t.get("flash-settings-saved"));
    if !invalid.is_empty() {
        state.invalid = Some(t.get1("settings-addresses-invalid", "words", invalid.join(" ")));
    }
}
