//! The settings (`pageSettings` of the draft): a navigation at the left (`.subnav`) and one part
//! at a time, each a card with "Save" at its foot. The design and the access of assistants take
//! effect at once, those of the packet store with the next capture.

use std::net::IpAddr;

use eframe::egui::emath::GuiRounding as _;
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Align, CornerRadius, Frame, Id, Layout, Margin, Rect, RichText, ScrollArea, Sense,
    Stroke, TextFormat, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use crate::config::packet_store::PacketStoreConf;
use crate::config::settings::ThemeChoice;
use crate::engine::state::EngineState;
use crate::gui::load::Task;
use crate::gui::pages::head;
use crate::gui::theme::{self, medium_font, mono_font, palette, semibold_font};
use crate::gui::widgets::{
    self, Kind, Tone, end_field, field_help as help, field_label as label, field_row,
};
use crate::gui::{
    App, SOURCE_URL, SettingsPanel, VERSION, icons, language_of, sound, theme_preference,
};
use crate::mcp::Status;
use crate::mmdb::geo::DownloadError;
use crate::utils::check_updates::UpdateStatus;
use crate::view::LICENSES;
use crate::view::i18n::Lang;
use crate::view::release_notes::{Block, Span, openable};
use crate::view::settings::{
    GeoRow, Latest, Settings, SettingsChange, apply_settings, parse_addresses, parse_sound,
    settings, sound_code, updates,
};

/// How to connect Claude Code, which starts this command itself
const MCP_COMMAND: &str = "claude mcp add openpacketmon -- openpacketmon mcp";
/// Most width of what a card of the settings holds (`.settings .cbody`: 760 px less 2 × 16 px)
const BODY_MAX: f32 = 728.0;
/// An entry of the navigation (`.subnav a`: 7 px above and below a line of 20 px)
const NAV_H: f32 = 34.0;

/// What the form holds until it is saved
struct Form {
    language: String,
    page_size: usize,
    favorite_active: bool,
    favorite_sound: &'static str,
    addresses: String,
    volume: u8,
    mmdb_country: String,
    mmdb_asn: String,
    update_checks: bool,
    packets_enabled: bool,
    /// As typed (`input[type=number]`)
    packets_max: String,
    packets_memory: usize,
    packets_headers: bool,
    calls_rtp: bool,
    calls_sip_any: bool,
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
            packets_max: s.packets_max.to_string(),
            packets_memory: state.conf.packet_store.max_megabytes(),
            packets_headers: s.packets_headers,
            calls_rtp: s.calls_rtp,
            calls_sip_any: s.calls_sip_any,
        }
    }
}

#[derive(Default)]
pub struct State {
    view: Option<(Settings, Form)>,
    /// The check for a new version the panel "Aktualisierungen" was made from
    update: Option<UpdateStatus>,
    invalid: Option<String>,
    geo: Task<Result<(), DownloadError>>,
    geo_error: Option<String>,
    /// Scroll to the release notes once ("What's new?" of the note on a new version)
    to_notes: bool,
}

impl State {
    /// The note on § 201 was confirmed or is to come again: the day the panel "Anrufe" shows
    pub fn law_changed(&mut self, calls_law: String) {
        if let Some((old, _)) = &mut self.view {
            old.calls_law = calls_law;
        }
    }

    /// The form is filled again when the page opens the next time
    pub fn close(&mut self) {
        self.view = None;
        self.update = None;
        self.invalid = None;
        self.geo_error = None;
    }

    /// Shows the release notes when the panel "Aktualisierungen" opens
    pub fn show_notes(&mut self) {
        self.to_notes = true;
    }
}

/// What the page asks for after drawing
#[derive(Default)]
struct Actions {
    save: bool,
    preview: bool,
    download: bool,
    theme: Option<ThemeChoice>,
    mcp: Option<bool>,
    content: Option<bool>,
    copy: Option<String>,
    open: Option<String>,
    panel: Option<SettingsPanel>,
    /// Show the note on § 201 of the criminal code again before listening
    law_reset: bool,
}

/// What the panels show besides the form
struct Context {
    t: Lang,
    theme: ThemeChoice,
    mcp: Status,
    mcp_on: bool,
    content: bool,
    downloading: bool,
    invalid: Option<String>,
    geo_error: Option<String>,
    to_notes: bool,
}

fn memory_label(megabytes: usize) -> String {
    if megabytes >= 1024 {
        format!("{} GiB", megabytes / 1024)
    } else {
        format!("{megabytes} MiB")
    }
}

fn panel_name(t: Lang, panel: SettingsPanel) -> String {
    t.get(match panel {
        SettingsPanel::Display => "settings-display",
        SettingsPanel::Notifications => "nav-notifications",
        SettingsPanel::PacketStore => "settings-packet-store",
        SettingsPanel::Calls => "settings-calls-nav",
        SettingsPanel::Geo => "settings-geo",
        SettingsPanel::Assistants => "settings-mcp",
        SettingsPanel::Updates => "settings-updates",
        SettingsPanel::About => "settings-about-nav",
    })
}

fn panel_icon(panel: SettingsPanel) -> egui::ImageSource<'static> {
    match panel {
        SettingsPanel::Display => icons::DISPLAY,
        SettingsPanel::Notifications => icons::BELL,
        SettingsPanel::PacketStore => icons::DATABASE,
        SettingsPanel::Calls => icons::PHONE,
        SettingsPanel::Geo => icons::GLOBE,
        SettingsPanel::Assistants => icons::BOT,
        SettingsPanel::Updates => icons::REFRESH,
        SettingsPanel::About => icons::FILE,
    }
}

pub fn show(app: &mut App, ui: &mut Ui, panel: SettingsPanel) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    if app.pages.settings.view.is_none() {
        let state = app.engine.lock();
        let view = settings(t, &state);
        let form = Form::new(&view, &state);
        let update = state.update_status.clone();
        drop(state);
        app.pages.settings.view = Some((view, form));
        app.pages.settings.update = Some(update);
    }
    if let Some(result) = app.pages.settings.geo.take() {
        match result {
            Ok(()) => {
                app.engine.lock().reload_geo();
                let view = settings(t, &app.engine.lock());
                if let Some((old, _)) = &mut app.pages.settings.view {
                    old.geo_rows = view.geo_rows;
                }
                app.toast(t.get("flash-geo-downloaded"));
            }
            Err(DownloadError::Busy) => {
                app.pages.settings.geo_error = Some(t.get("settings-geo-busy"));
            }
            Err(_) => app.pages.settings.geo_error = Some(t.get("settings-geo-failed")),
        }
    }
    // the check for a new version may end while the page is open
    let (update, mcp_on, theme) = {
        let state = app.engine.lock();
        (
            state.update_status.clone(),
            state.conf.mcp.enabled,
            state.conf.settings.theme,
        )
    };
    if app.pages.settings.update.as_ref() != Some(&update) {
        if let Some((view, _)) = &mut app.pages.settings.view {
            view.updates = updates(t, &update);
        }
        app.pages.settings.update = Some(update);
    }
    head(
        ui,
        &t.get("nav-settings"),
        Some(&t.get("settings-subtitle")),
        |_| {},
    );
    let info = Context {
        t,
        theme,
        mcp: app.mcp.status(),
        mcp_on,
        content: app.mcp.content_allowed(),
        downloading: app.pages.settings.geo.busy(),
        invalid: app.pages.settings.invalid.clone(),
        geo_error: app.pages.settings.geo_error.clone(),
        to_notes: std::mem::take(&mut app.pages.settings.to_notes)
            && panel == SettingsPanel::Updates,
    };
    let mut acts = Actions::default();
    let Some((view, form)) = &mut app.pages.settings.view else {
        return;
    };
    let width = ctx.content_rect().width();
    if width < 900.0 {
        // the navigation as a row above the panel (`.subnav` below 900 px)
        ui.spacing_mut().item_spacing.y = 0.0;
        acts.panel = nav_row(ui, t, panel);
        ui.add_space(theme::SP3);
        panel_body(ui, &info, panel, view, form, &mut acts);
    } else {
        let (nav_w, gap) = if width < 1024.0 {
            (180.0, theme::SP4)
        } else {
            (220.0, theme::SP5)
        };
        let top = ui.cursor().min;
        let full = ui.available_width();
        let mut right = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    pos2(top.x + nav_w + gap, top.y),
                    vec2(full - nav_w - gap, f32::INFINITY),
                ))
                .layout(Layout::top_down(Align::Min)),
        );
        panel_body(&mut right, &info, panel, view, form, &mut acts);
        let bottom = right.min_rect().bottom();
        // the navigation stays at the top while the panel scrolls (`position: sticky`)
        let count = SettingsPanel::ALL.len() as f32;
        let nav_h = count * NAV_H + (count - 1.0) * 2.0;
        let y = ui.clip_rect().top().min(bottom - nav_h).max(top.y);
        let mut left = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(pos2(top.x, y), vec2(nav_w, nav_h)))
                .layout(Layout::top_down(Align::Min)),
        );
        acts.panel = nav(&mut left, t, panel);
        ui.advance_cursor_after_rect(Rect::from_min_max(
            top,
            pos2(top.x + full, bottom.max(top.y + nav_h)),
        ));
    }
    act(app, &ctx, acts);
}

/// Does what the page asked for
fn act(app: &mut App, ctx: &egui::Context, acts: Actions) {
    let t = app.t;
    if let Some(next) = acts.panel {
        // the form keeps what was typed in other panels (`hashchange`)
        app.replace(crate::gui::Page::Settings(next));
    }
    if acts.preview
        && let Some((_, form)) = &app.pages.settings.view
    {
        sound::play(parse_sound(form.favorite_sound), form.volume);
    }
    if let Some(theme) = acts.theme {
        app.engine.lock().conf.settings.theme = theme;
        app.engine.store_conf();
        ctx.set_theme(theme_preference(theme));
    }
    if let Some(on) = acts.mcp {
        app.engine.lock().conf.mcp.enabled = on;
        app.engine.store_conf();
        app.mcp.apply(on);
        if !on {
            app.mcp.allow_content(false);
        }
    }
    if let Some(on) = acts.content {
        app.mcp.allow_content(on);
    }
    if let Some(text) = acts.copy {
        ctx.copy_text(text);
        app.toast(t.get("copied-command"));
    }
    if let Some(url) = acts.open {
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }
    if acts.save {
        store(app, ctx);
    }
    if acts.law_reset {
        app.engine.lock().conf.settings.calls.law_confirmed.clear();
        app.engine.store_conf();
        let view = settings(t, &app.engine.lock());
        app.pages.settings.law_changed(view.calls_law);
    }
    if acts.download {
        let engine = app.engine.clone();
        let state = &mut app.pages.settings;
        state.geo_error = None;
        state.geo.start(ctx, move || engine.download_geo());
    }
}

/// The navigation at the left (`.subnav`)
fn nav(ui: &mut Ui, t: Lang, current: SettingsPanel) -> Option<SettingsPanel> {
    ui.spacing_mut().item_spacing.y = 2.0;
    let mut chosen = None;
    for panel in SettingsPanel::ALL {
        let name = panel_name(t, panel);
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), NAV_H), Sense::click());
        response
            .widget_info(|| WidgetInfo::selected(egui::Role::Link, true, panel == current, &name));
        nav_entry(ui, rect, &response, panel, &name, panel == current, None);
        if response.clicked() && panel != current {
            chosen = Some(panel);
        }
    }
    chosen
}

/// The navigation as a row that scrolls sideways (`.subnav` below 900 px: entries with a frame,
/// 4 px apart)
fn nav_row(ui: &mut Ui, t: Lang, current: SettingsPanel) -> Option<SettingsPanel> {
    let mut chosen = None;
    ScrollArea::horizontal()
        .id_salt("settings-nav")
        .min_scrolled_height(0.0)
        .auto_shrink([false, true])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SP1;
                for panel in SettingsPanel::ALL {
                    let name = panel_name(t, panel);
                    let galley = ui.painter().layout_no_wrap(
                        name.clone(),
                        medium_font(theme::FS_BASE),
                        egui::Color32::PLACEHOLDER,
                    );
                    let width = 10.0 + 16.0 + theme::SP2 + galley.size().x + 10.0 + 2.0;
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(width, NAV_H + 2.0), Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::selected(egui::Role::Link, true, panel == current, &name)
                    });
                    let p = palette(ui.ctx());
                    nav_entry(
                        ui,
                        rect,
                        &response,
                        panel,
                        &name,
                        panel == current,
                        Some(p.border),
                    );
                    if response.clicked() && panel != current {
                        chosen = Some(panel);
                    }
                }
            });
            ui.add_space(theme::SP1);
        });
    chosen
}

/// An entry of the navigation: icon and name, the current one on `--selected`
fn nav_entry(
    ui: &Ui,
    rect: Rect,
    response: &egui::Response,
    panel: SettingsPanel,
    name: &str,
    current: bool,
    frame: Option<egui::Color32>,
) {
    let p = palette(ui.ctx());
    let radius = CornerRadius::same(theme::RADIUS_SM);
    let fill = if current {
        p.selected
    } else if response.hovered() {
        p.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        radius,
        fill,
        Stroke::new(1.0, frame.unwrap_or(egui::Color32::TRANSPARENT)),
        egui::StrokeKind::Inside,
    );
    let color = if current || response.hovered() {
        p.text
    } else {
        p.text_2
    };
    let x = rect.left() + if frame.is_some() { 11.0 } else { 10.0 };
    widgets::paint_icon(
        ui,
        panel_icon(panel),
        Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(16.0, 16.0)),
        color,
    );
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_owned(), medium_font(theme::FS_BASE), color);
    let at = pos2(
        x + 16.0 + theme::SP2,
        rect.center().y - galley.size().y / 2.0,
    )
    .round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().with_clip_rect(rect).galley(at, galley, color);
    widgets::focus_ring(ui, response, theme::RADIUS_SM);
}

fn panel_body(
    ui: &mut Ui,
    info: &Context,
    panel: SettingsPanel,
    view: &Settings,
    form: &mut Form,
    acts: &mut Actions,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    match panel {
        SettingsPanel::Display => display(ui, info, view, form, acts),
        SettingsPanel::Notifications => notifications(ui, info, view, form, acts),
        SettingsPanel::PacketStore => packet_store(ui, info.t, form, acts),
        SettingsPanel::Calls => calls(ui, info.t, view, form, acts),
        SettingsPanel::Geo => geo(ui, info, view, form, acts),
        SettingsPanel::Assistants => assistants(ui, info, acts),
        SettingsPanel::Updates => updates_panel(ui, info, view, form, acts),
        SettingsPanel::About => about(ui, info.t, acts),
    }
}

/// A card of the settings: the head, the body (`.cbody`, at most 760 px wide) and, with `save`,
/// the foot with "Save" at the right (`.cfoot`)
fn panel_card(
    ui: &mut Ui,
    t: Lang,
    title: &str,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui),
    save: Option<&mut bool>,
) -> Rect {
    let response = widgets::flush_card(ui, title, tools, |ui| {
        Frame::new()
            .inner_margin(Margin::same(theme::SP4 as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let width = ui.available_width().min(BODY_MAX);
                ui.scope(|ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    body(ui);
                });
            });
        if let Some(save) = save {
            widgets::card_foot(ui, true, |ui| {
                *save = widgets::button(ui, t.get("save"), Kind::Primary).clicked();
            });
        }
        ui.min_rect()
    });
    ui.add_space(theme::SP4);
    response
}

/// A line of text with parts in bold or in the font of code, wrapped
fn rich(ui: &mut Ui, parts: &[(&str, Style)], size: f32, line: f32) {
    let p = palette(ui.ctx());
    let mut job = LayoutJob::default();
    for (text, style) in parts {
        let (font, color) = match style {
            Style::Plain => (egui::FontId::proportional(size), p.text),
            Style::Bold => (semibold_font(size), p.text),
            Style::Mono => (mono_font(size * 0.92), p.text),
            Style::BoldMono => (theme::mono_semibold_font(size * 0.92), p.text),
        };
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font,
                color,
                line_height: Some(line),
                valign: Align::Center,
                ..TextFormat::default()
            },
        );
    }
    job.wrap.max_width = ui.available_width();
    ui.add(egui::Label::new(job).wrap());
}

#[derive(Clone, Copy)]
enum Style {
    Plain,
    Bold,
    Mono,
    BoldMono,
}

/// `text` with each of `codes` in the font of code
fn with_codes<'a>(text: &'a str, codes: &[&'a str]) -> Vec<(&'a str, Style)> {
    let mut parts = Vec::new();
    let mut rest = text;
    loop {
        let next = codes
            .iter()
            .filter(|c| !c.is_empty())
            .filter_map(|c| rest.find(c).map(|at| (at, *c)))
            .min_by_key(|(at, _)| *at);
        let Some((at, code)) = next else {
            break;
        };
        if at > 0 {
            parts.push((&rest[..at], Style::Plain));
        }
        parts.push((code, Style::Mono));
        rest = &rest[at + code.len()..];
    }
    if !rest.is_empty() {
        parts.push((rest, Style::Plain));
    }
    parts
}

/// The errors of a panel, above its fields
fn error(ui: &mut Ui, text: Option<&str>) {
    if let Some(text) = text {
        widgets::error(ui, text);
        ui.add_space(theme::SP4);
    }
}

fn display(ui: &mut Ui, info: &Context, view: &Settings, form: &mut Form, acts: &mut Actions) {
    let t = info.t;
    let mut save = false;
    panel_card(
        ui,
        t,
        &t.get("settings-display"),
        |_| {},
        |ui| {
            field_row(
                ui,
                |ui, width| {
                    label(ui, &t.get("settings-language"));
                    let labels: Vec<String> = view
                        .languages
                        .iter()
                        .map(|c| {
                            if c.value.is_empty() {
                                t.get("gui-language-system")
                            } else {
                                c.label.clone()
                            }
                        })
                        .collect();
                    let at = view
                        .languages
                        .iter()
                        .position(|c| c.value == form.language)
                        .unwrap_or(0);
                    if let Some(i) = widgets::select_width(ui, &labels, at, false, Some(width)) {
                        view.languages[i].value.clone_into(&mut form.language);
                    }
                    end_field(ui);
                },
                |ui, width| {
                    label(ui, &t.get("settings-page-size"));
                    let labels: Vec<String> =
                        view.page_sizes.iter().map(|(n, _)| n.to_string()).collect();
                    let at = view
                        .page_sizes
                        .iter()
                        .position(|(n, _)| *n == form.page_size)
                        .unwrap_or(0);
                    if let Some(i) = widgets::select_width(ui, &labels, at, false, Some(width)) {
                        form.page_size = view.page_sizes[i].0;
                    }
                    end_field(ui);
                },
            );
            label(ui, &t.get("gui-theme"));
            let themes = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];
            let labels = [
                t.get("gui-theme-system"),
                t.get("gui-theme-light"),
                t.get("gui-theme-dark"),
            ];
            let at = themes.iter().position(|c| *c == info.theme).unwrap_or(0);
            let width = ui.available_width();
            if let Some(i) = widgets::segmented_wide(ui, &labels, at, width) {
                acts.theme = Some(themes[i]);
            }
            help(ui, &t.get("settings-theme-hint"));
            end_field(ui);
        },
        Some(&mut save),
    );
    acts.save |= save;
}

fn notifications(
    ui: &mut Ui,
    info: &Context,
    view: &Settings,
    form: &mut Form,
    acts: &mut Actions,
) {
    let t = info.t;
    let mut save = false;
    panel_card(
        ui,
        t,
        &t.get("nav-notifications"),
        |_| {},
        |ui| {
            error(ui, info.invalid.as_deref());
            widgets::switch(
                ui,
                &mut form.favorite_active,
                &t.get("settings-favorite-notification"),
                true,
            );
            help(ui, &t.get("settings-favorite-hint"));
            end_field(ui);
            label(ui, &t.get("settings-favorite-addresses"));
            widgets::textarea(
                ui,
                &mut form.addresses,
                Id::NULL.with("favorite-addresses"),
                3,
                true,
            );
            help(ui, &t.get("settings-favorite-addresses-hint"));
            end_field(ui);
            field_row(
                ui,
                |ui, width| {
                    label(ui, &t.get("settings-sound"));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = theme::SP2;
                        let listen = t.get("settings-listen");
                        // icon and text of the button (`.btn` with an icon)
                        let button = widgets::button_width(ui, &listen) + 14.0 + 6.0;
                        let labels: Vec<String> = view
                            .favorite_sounds
                            .iter()
                            .map(|c| c.label.clone())
                            .collect();
                        let at = view
                            .favorite_sounds
                            .iter()
                            .position(|c| c.value == form.favorite_sound)
                            .unwrap_or(0);
                        let select = (width - theme::SP2 - button).max(80.0);
                        if let Some(i) = widgets::select_width(ui, &labels, at, false, Some(select))
                        {
                            form.favorite_sound = view.favorite_sounds[i].value;
                        }
                        acts.preview |= widgets::icon_text_button(
                            ui,
                            Some(icons::VOLUME),
                            &listen,
                            Kind::Secondary,
                            false,
                        )
                        .clicked();
                    });
                    end_field(ui);
                },
                |ui, width| {
                    label(ui, &t.get("settings-volume"));
                    widgets::range(ui, &mut form.volume, 5, width, &t.get("settings-volume"));
                    end_field(ui);
                },
            );
        },
        Some(&mut save),
    );
    acts.save |= save;
}

fn packet_store(ui: &mut Ui, t: Lang, form: &mut Form, acts: &mut Actions) {
    let mut save = false;
    panel_card(
        ui,
        t,
        &t.get("settings-packet-store"),
        |_| {},
        |ui| {
            widgets::switch(
                ui,
                &mut form.packets_enabled,
                &t.get("settings-packets-enabled"),
                true,
            );
            end_field(ui);
            field_row(
                ui,
                |ui, width| {
                    label(ui, &t.get("settings-packets-max"));
                    widgets::input(
                        ui,
                        &mut form.packets_max,
                        Id::NULL.with("packets-max"),
                        vec2(width, widgets::CONTROL_H),
                        false,
                        "",
                    );
                    end_field(ui);
                },
                |ui, width| {
                    label(ui, &t.get("gui-packets-memory"));
                    let labels: Vec<String> = PacketStoreConf::MEGABYTES_CHOICES
                        .iter()
                        .map(|n| memory_label(*n))
                        .collect();
                    let at = PacketStoreConf::MEGABYTES_CHOICES
                        .iter()
                        .position(|n| *n == form.packets_memory)
                        .unwrap_or(0);
                    if let Some(i) = widgets::select_width(ui, &labels, at, false, Some(width)) {
                        form.packets_memory = PacketStoreConf::MEGABYTES_CHOICES[i];
                    }
                    end_field(ui);
                },
            );
            widgets::check(
                ui,
                &mut form.packets_headers,
                &t.get("settings-packets-headers"),
            );
            ui.add_space(theme::SP2);
            widgets::hint(ui, &t.get("settings-packets-hint"));
            end_field(ui);
        },
        Some(&mut save),
    );
    acts.save |= save;
}

/// Settings → Calls: how calls are found, listening and the note on § 201 of the criminal code
fn calls(ui: &mut Ui, t: Lang, view: &Settings, form: &mut Form, acts: &mut Actions) {
    let mut save = false;
    panel_card(
        ui,
        t,
        &t.get("settings-calls"),
        |_| {},
        |ui| {
            widgets::switch(ui, &mut form.calls_rtp, &t.get("settings-calls-rtp"), true);
            help(ui, &t.get("settings-calls-rtp-help"));
            end_field(ui);
            widgets::switch(
                ui,
                &mut form.calls_sip_any,
                &t.get("settings-calls-sip"),
                false,
            );
            help(ui, &t.get("settings-calls-sip-help"));
            end_field(ui);
            let labels = [t.get("settings-calls-listen"), t.get("settings-calls-law")];
            widgets::facts(ui, &labels, |ui, i| {
                if i == 0 {
                    ui.label(t.get("settings-calls-listen-text"));
                } else if view.calls_law.is_empty() {
                    ui.label(t.get("settings-calls-law-open"));
                } else {
                    ui.label(t.get1("settings-calls-law-done", "date", view.calls_law.as_str()));
                    acts.law_reset |= widgets::small_button(
                        ui,
                        t.get("settings-calls-law-again"),
                        Kind::Secondary,
                    )
                    .clicked();
                }
            });
        },
        Some(&mut save),
    );
    acts.save |= save;
}

fn geo(ui: &mut Ui, info: &Context, view: &Settings, form: &mut Form, acts: &mut Actions) {
    let t = info.t;
    let mut save = false;
    panel_card(
        ui,
        t,
        &t.get("settings-geo"),
        |_| {},
        |ui| {
            error(ui, info.geo_error.as_deref());
            let labels: Vec<String> = view.geo_rows.iter().map(|r| r.label.clone()).collect();
            widgets::facts(ui, &labels, |ui, i| geo_value(ui, t, &view.geo_rows[i]));
            ui.add_space(theme::SP4);
            if info.downloading {
                ui.add_enabled_ui(false, |ui| {
                    widgets::icon_text_button(
                        ui,
                        Some(icons::DOWNLOAD),
                        &t.get("settings-geo-downloading"),
                        Kind::Secondary,
                        false,
                    );
                });
            } else {
                acts.download |= widgets::icon_text_button(
                    ui,
                    Some(icons::DOWNLOAD),
                    &t.get("settings-geo-download"),
                    Kind::Secondary,
                    false,
                )
                .clicked();
            }
            ui.add_space(theme::SP2);
            let text = t.get1("settings-geo-download-hint", "dir", view.geo_dir.as_str());
            ui.scope(|ui| {
                let p = palette(ui.ctx());
                let mut job = LayoutJob::default();
                for (part, style) in with_codes(&text, &[view.geo_dir.as_str()]) {
                    let font = match style {
                        Style::Mono => mono_font(theme::FS_SM * 0.92),
                        _ => egui::FontId::proportional(theme::FS_SM),
                    };
                    job.append(
                        part,
                        0.0,
                        TextFormat {
                            font_id: font,
                            color: p.text_3,
                            line_height: Some(theme::FS_SM * 1.45),
                            valign: Align::Center,
                            ..TextFormat::default()
                        },
                    );
                }
                job.wrap.max_width = ui.available_width();
                ui.add(egui::Label::new(job).wrap());
            });
            ui.add_space(theme::SP4);
            widgets::fieldset(ui, &t.get("settings-mmdb-own"), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let width = ui.available_width();
                label(ui, &t.get("settings-mmdb-country"));
                widgets::input(
                    ui,
                    &mut form.mmdb_country,
                    Id::NULL.with("mmdb-country"),
                    vec2(width, widgets::CONTROL_H),
                    true,
                    "",
                );
                end_field(ui);
                label(ui, &t.get("settings-mmdb-asn"));
                widgets::input(
                    ui,
                    &mut form.mmdb_asn,
                    Id::NULL.with("mmdb-asn"),
                    vec2(width, widgets::CONTROL_H),
                    true,
                    "",
                );
                help(ui, &t.get("settings-mmdb-hint"));
                end_field(ui);
            });
        },
        Some(&mut save),
    );
    acts.save |= save;
}

/// A database in use: its name as a state, then when it was built
fn geo_value(ui: &mut Ui, t: Lang, row: &GeoRow) {
    let p = palette(ui.ctx());
    let Some(name) = &row.name else {
        ui.label(RichText::new(t.get("settings-geo-none")).color(p.text_3));
        return;
    };
    let (tone, icon) = if row.old {
        (Tone::Warn, icons::ALERT)
    } else {
        (Tone::Ok, icons::CHECK_CIRCLE)
    };
    widgets::state(ui, tone, Some(icon), name).on_hover_text(&row.path);
    ui.label(RichText::new(&row.built).color(p.text_3));
    if row.old {
        ui.label(RichText::new(t.get("settings-geo-old")).color(p.warn));
    }
}

fn assistants(ui: &mut Ui, info: &Context, acts: &mut Actions) {
    let t = info.t;
    let p = palette(ui.ctx());
    let mut on = info.mcp_on;
    let mut content = info.content;
    let active = matches!(info.mcp, Status::Active { .. });
    let shown = info.content;
    panel_card(
        ui,
        t,
        &t.get("settings-mcp"),
        |ui| {
            if !active {
                return;
            }
            if shown {
                widgets::state(
                    ui,
                    Tone::Warn,
                    Some(icons::ALERT),
                    &t.get("settings-mcp-content-on"),
                );
            } else {
                widgets::state(
                    ui,
                    Tone::Ok,
                    Some(icons::CHECK_CIRCLE),
                    &t.get("settings-mcp-on"),
                );
            }
        },
        |ui| {
            if info.mcp == Status::Failed {
                error(ui, Some(&t.get("settings-mcp-failed")));
            }
            if widgets::switch(ui, &mut on, &t.get("settings-mcp-enabled"), true).changed() {
                acts.mcp = Some(on);
            }
            help(ui, &t.get("settings-mcp-hint"));
            end_field(ui);
            if !on {
                return;
            }
            if let Status::Active { calls } = info.mcp {
                let text = t.get1("settings-mcp-active", "calls", calls.to_string());
                let calls = calls.to_string();
                let parts: Vec<(&str, Style)> = match text.rfind(&calls) {
                    Some(at) => vec![
                        (&text[..at], Style::Plain),
                        (calls.as_str(), Style::BoldMono),
                        (&text[at + calls.len()..], Style::Plain),
                    ],
                    None => vec![(text.as_str(), Style::Plain)],
                };
                rich(ui, &parts, theme::FS_SM, theme::FS_SM * 1.45);
                ui.add_space(theme::SP2);
            }
            label(ui, &t.get("settings-mcp-command"));
            if widgets::codebox(ui, MCP_COMMAND, &t.get("copy")) {
                acts.copy = Some(MCP_COMMAND.to_owned());
            }
            end_field(ui);
            widgets::fieldset(ui, &t.get("settings-mcp-session"), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if widgets::switch(ui, &mut content, &t.get("settings-mcp-content"), false)
                    .changed()
                {
                    acts.content = Some(content);
                }
                help(ui, &t.get("settings-mcp-content-hint"));
                end_field(ui);
            });
        },
        None,
    );
    // below the card (`p.muted.small`)
    ui.add(
        egui::Label::new(
            RichText::new(t.get("settings-mcp-note"))
                .size(theme::FS_SM)
                .color(p.text_3)
                .line_height(Some(theme::FS_SM * 1.45)),
        )
        .wrap(),
    );
}

fn updates_panel(
    ui: &mut Ui,
    info: &Context,
    view: &Settings,
    form: &mut Form,
    acts: &mut Actions,
) {
    let t = info.t;
    let p = palette(ui.ctx());
    let updates = &view.updates;
    let mut save = false;
    panel_card(
        ui,
        t,
        &t.get("settings-updates"),
        |_| {},
        |ui| {
            widgets::switch(
                ui,
                &mut form.update_checks,
                &t.get("settings-update-checks"),
                true,
            );
            help(ui, &t.get("settings-update-hint"));
            end_field(ui);
            let labels = [
                t.get("settings-installed"),
                t.get("settings-latest"),
                t.get("settings-checked"),
            ];
            widgets::facts(ui, &labels, |ui, i| match i {
                0 => {
                    widgets::code(ui, updates.installed);
                }
                1 => match &updates.latest {
                    Latest::Current(version) => {
                        widgets::code(ui, version);
                        widgets::state(
                            ui,
                            Tone::Ok,
                            Some(icons::CHECK_CIRCLE),
                            &t.get("settings-current"),
                        );
                    }
                    Latest::Newer { version, date } => {
                        widgets::code(ui, version);
                        if !date.is_empty() {
                            ui.label(
                                RichText::new(t.get1("settings-published", "date", date.as_str()))
                                    .color(p.text_3),
                            );
                        }
                        widgets::pill(ui, &t.get("settings-new"));
                    }
                    Latest::Searching => {
                        ui.label(RichText::new(t.get("settings-searching")).color(p.text_3));
                    }
                    Latest::Unknown => {
                        ui.label(RichText::new(t.get("settings-latest-unknown")).color(p.text_3));
                    }
                },
                _ => match &updates.checked {
                    Some(at) => {
                        ui.label(at);
                    }
                    None => {
                        ui.label(RichText::new(t.get("settings-not-checked")).color(p.text_3));
                    }
                },
            });
            ui.add_space(theme::SP4);
            match &updates.latest {
                Latest::Newer { version, .. } => {
                    if new_version(ui, t, version, acts) {
                        acts.open = Some(format!("{SOURCE_URL}/releases/tag/v{version}"));
                    }
                }
                latest => {
                    let page = match latest {
                        Latest::Current(version) => {
                            format!("{SOURCE_URL}/releases/tag/v{version}")
                        }
                        _ => format!("{SOURCE_URL}/releases"),
                    };
                    if widgets::icon_text_button(
                        ui,
                        Some(icons::EXTERNAL),
                        &t.get("settings-release-page"),
                        Kind::Secondary,
                        false,
                    )
                    .clicked()
                    {
                        acts.open = Some(page);
                    }
                }
            }
            end_field(ui);
        },
        Some(&mut save),
    );
    acts.save |= save;
    if let Some((version, blocks)) = &updates.notes {
        let rect = panel_card_notes(ui, t, version, blocks, acts);
        if info.to_notes {
            ui.scroll_to_rect(rect, Some(Align::TOP));
        }
    }
}

/// The note on a new version without the launcher (`.notice.info` with `ol.steps`); returns
/// whether "Open release page" was clicked
fn new_version(ui: &mut Ui, t: Lang, version: &str, acts: &mut Actions) -> bool {
    let p = palette(ui.ctx());
    let mut open = false;
    let deb = format!("openpacketmon_{version}-1_amd64.deb");
    let setup = format!("openpacketmon-{version}-setup.exe");
    let command = format!("sha256sum -c --ignore-missing SHA256SUMS && sudo apt install ./{deb}");
    Frame::new()
        .fill(p.accent_bg)
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                let (rect, _) = ui.allocate_exact_size(vec2(16.0, 19.0), Sense::hover());
                widgets::paint_icon(
                    ui,
                    icons::DOWNLOAD,
                    Rect::from_min_size(rect.min + vec2(0.0, 2.0), vec2(16.0, 16.0)),
                    p.accent,
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let line = theme::FS_SM * 1.45;
                    let title = format!("{}. ", t.get1("upd-title", "version", version));
                    let text = t.get("upd-text-plain");
                    let mut parts = vec![(title.as_str(), Style::Bold)];
                    parts.extend(with_codes(&text, &["SHA256SUMS"]));
                    rich(ui, &parts, theme::FS_SM, line);
                    ui.add_space(theme::SP2);
                    let linux = t.get2("upd-step-linux", "deb", deb.as_str(), "sums", "SHA256SUMS");
                    let windows = t.get1("upd-step-windows", "exe", setup.as_str());
                    let restart = t.get("upd-step-restart");
                    for (n, step) in [linux, windows, restart].iter().enumerate() {
                        if n > 0 {
                            ui.add_space(theme::SP1);
                        }
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let (number, _) =
                                ui.allocate_exact_size(vec2(20.0, line), Sense::hover());
                            let galley = ui.painter().layout_no_wrap(
                                format!("{}.", n + 1),
                                egui::FontId::proportional(theme::FS_SM),
                                p.text,
                            );
                            ui.painter().galley(
                                pos2(
                                    number.right() - 6.0 - galley.size().x,
                                    number.center().y - galley.size().y / 2.0,
                                ),
                                galley,
                                p.text,
                            );
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let mut parts = Vec::new();
                                match n {
                                    0 => parts.push(("Linux: ", Style::Bold)),
                                    1 => parts.push(("Windows: ", Style::Bold)),
                                    _ => {}
                                }
                                parts.extend(with_codes(
                                    step,
                                    &[deb.as_str(), setup.as_str(), "SHA256SUMS"],
                                ));
                                rich(ui, &parts, theme::FS_SM, line);
                                if n == 0 {
                                    ui.add_space(theme::SP1);
                                    if widgets::codebox(ui, &command, &t.get("copy")) {
                                        acts.copy = Some(command.clone());
                                    }
                                }
                            });
                        });
                    }
                    ui.add_space(theme::SP3);
                    open = widgets::icon_text_button(
                        ui,
                        Some(icons::EXTERNAL),
                        &t.get("upd-release-page"),
                        Kind::Primary,
                        false,
                    )
                    .clicked();
                });
            });
        });
    open
}

/// "What's new in …" (`#relnotes`): the release notes as GitHub keeps them
fn panel_card_notes(
    ui: &mut Ui,
    t: Lang,
    version: &str,
    blocks: &[Block],
    acts: &mut Actions,
) -> Rect {
    let title = t.get1("settings-notes-title", "version", version);
    panel_card(
        ui,
        t,
        &title,
        |ui| widgets::card_note(ui, &t.get("settings-notes-hint")),
        |ui| release_notes(ui, blocks, acts),
        None,
    )
}

/// The release notes (`.relnotes`: 13 px, lines of 1.5)
fn release_notes(ui: &mut Ui, blocks: &[Block], acts: &mut Actions) {
    let p = palette(ui.ctx());
    let line = theme::FS_SM * 1.5;
    for (i, block) in blocks.iter().enumerate() {
        let (spans, heading, item) = match block {
            Block::Heading(spans) => (spans, true, false),
            Block::Paragraph(spans) => (spans, false, false),
            Block::Item(spans) => (spans, false, true),
        };
        if heading && i > 0 {
            ui.add_space(theme::SP3 - theme::SP2);
        }
        let next_item = matches!(blocks.get(i + 1), Some(Block::Item(_)));
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if item {
                let (rect, _) = ui.allocate_exact_size(vec2(20.0, line), Sense::hover());
                ui.painter()
                    .circle_filled(pos2(rect.left() + 9.0, rect.center().y), 2.5, p.text);
            }
            ui.vertical(|ui| {
                ui.spacing_mut().interact_size.y = line;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    for span in spans {
                        span_label(ui, span, heading, line, acts);
                    }
                });
            });
        });
        ui.add_space(if heading {
            theme::SP1
        } else if item && next_item {
            2.0
        } else {
            theme::SP2
        });
    }
}

fn span_label(ui: &mut Ui, span: &Span, heading: bool, line: f32, acts: &mut Actions) {
    let p = palette(ui.ctx());
    let size = theme::FS_SM;
    let plain = |text: &str| {
        let rich = RichText::new(text).line_height(Some(line)).color(p.text);
        if heading {
            rich.font(semibold_font(size))
        } else {
            rich.size(size)
        }
    };
    match span {
        Span::Text(text) => {
            ui.add(egui::Label::new(plain(text)).wrap());
        }
        Span::Bold(text) => {
            ui.add(egui::Label::new(plain(text).font(semibold_font(size))).wrap());
        }
        Span::Code(text) => {
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .font(mono_font(size * 0.95))
                        .color(p.text)
                        .line_height(Some(line)),
                )
                .wrap(),
            );
        }
        Span::Link(text, url) => {
            let response = ui.add(
                egui::Label::new(plain(text).color(p.accent))
                    .wrap()
                    .sense(Sense::click()),
            );
            if openable(url) {
                let response = response.on_hover_text(url.as_str());
                if response.clicked() {
                    acts.open = Some(url.clone());
                }
            }
        }
    }
}

fn about(ui: &mut Ui, t: Lang, acts: &mut Actions) {
    let p = palette(ui.ctx());
    let source = SOURCE_URL.trim_start_matches("https://");
    widgets::card_tight(
        ui,
        &t.get("settings-about"),
        |_| {},
        |ui| {
            let width = ui.available_width().min(BODY_MAX);
            ui.scope(|ui| {
                ui.set_width(width);
                let labels = [
                    t.get("about-version"),
                    t.get("about-license"),
                    t.get("about-source"),
                    t.get("about-based-on"),
                    t.get("settings-geo"),
                ];
                widgets::facts(ui, &labels, |ui, i| match i {
                    0 => {
                        widgets::code(ui, VERSION);
                    }
                    1 => {
                        ui.label("MIT OR Apache-2.0");
                    }
                    2 => {
                        if widgets::link_text(ui, RichText::new(source).color(p.accent)).clicked() {
                            acts.open = Some(SOURCE_URL.to_owned());
                        }
                    }
                    3 => {
                        ui.label(t.get("about-based-on-text"));
                    }
                    _ => {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label("IP Geolocation by");
                        if widgets::link_text(ui, RichText::new("DB-IP").color(p.accent)).clicked()
                        {
                            acts.open = Some("https://db-ip.com".to_owned());
                        }
                        ui.label("(CC BY 4.0)");
                    }
                });
            });
        },
    );
    ui.add_space(theme::SP4);
    panel_card(
        ui,
        t,
        &t.get("settings-third-party"),
        |ui| widgets::card_note(ui, "NOTICE · THIRD-PARTY-LICENSES.txt"),
        licenses,
        None,
    );
}

/// The texts of the licenses (`pre.lic`: at most 260 px high, scrolls, long lines wrapped)
fn licenses(ui: &mut Ui) {
    let p = palette(ui.ctx());
    let font = mono_font(theme::FS_XS);
    let height = (theme::FS_XS * 1.45).round();
    Frame::new()
        .fill(p.surface_2)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let advance = ui
                .painter()
                .layout_no_wrap("0".repeat(10), font.clone(), p.text_2)
                .size()
                .x
                / 10.0;
            // room for the scroll bar at the right
            let columns = ((ui.available_width() - 12.0) / advance).floor().max(20.0) as usize;
            let id = Id::NULL.with("license-lines");
            let lines: std::sync::Arc<Vec<(usize, usize)>> = ui.ctx().memory_mut(|m| {
                let cached = m
                    .data
                    .get_temp::<(usize, std::sync::Arc<Vec<(usize, usize)>>)>(id)
                    .filter(|(at, _)| *at == columns);
                if let Some((_, lines)) = cached {
                    lines
                } else {
                    let lines = std::sync::Arc::new(wrapped(LICENSES, columns));
                    m.data.insert_temp(id, (columns, lines.clone()));
                    lines
                }
            });
            ScrollArea::vertical()
                .id_salt("licenses")
                .auto_shrink([false, true])
                .max_height(260.0 - 2.0 * theme::SP3 - 2.0)
                .show_rows(ui, height, lines.len(), |ui, range| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for &(start, end) in &lines[range] {
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), height),
                            Sense::hover(),
                        );
                        let galley = ui.painter().layout_no_wrap(
                            LICENSES[start..end].trim_end().to_owned(),
                            font.clone(),
                            p.text_2,
                        );
                        ui.painter().galley(
                            pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
                            galley,
                            p.text_2,
                        );
                    }
                });
        });
    ui.add_space(theme::SP4);
}

/// The lines of `text` as ranges of bytes, longer ones wrapped after a space before `columns`
/// characters (`white-space: pre-wrap`)
fn wrapped(text: &str, columns: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in text.split('\n') {
        let (mut start, end) = (offset, offset + line.len());
        offset = end + 1;
        loop {
            let rest = &text[start..end];
            let Some((cut, _)) = rest.char_indices().nth(columns) else {
                out.push((start, end));
                break;
            };
            let at = rest[..cut]
                .rfind(' ')
                .filter(|&i| i > 0)
                .map_or(cut, |i| i + 1);
            out.push((start, start + at));
            start += at;
        }
    }
    out
}

/// Saves the form
fn store(app: &mut App, ctx: &egui::Context) {
    let t = app.t;
    let Some((_, form)) = &app.pages.settings.view else {
        return;
    };
    let (addresses, invalid): (Vec<IpAddr>, Vec<String>) = parse_addresses(&form.addresses);
    let digits: String = form
        .packets_max
        .chars()
        .filter(char::is_ascii_digit)
        .take(12)
        .collect();
    let packets_max = digits.parse().unwrap_or(PacketStoreConf::MAX_PACKETS);
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
        packets_max,
        packets_memory: Some(form.packets_memory),
        packets_headers: form.packets_headers,
        calls_rtp: form.calls_rtp,
        calls_sip_any: form.calls_sip_any,
        mcp_enabled: app.engine.lock().conf.mcp.enabled,
    };
    apply_settings(&mut app.engine.lock(), &change);
    app.engine.store_conf();
    let language = app.engine.lock().conf.language.clone();
    app.t = language_of(&language);
    let t = if app.t == t { t } else { app.t };
    let state = &mut app.pages.settings;
    state.close();
    if !invalid.is_empty() {
        state.invalid = Some(t.get1("settings-addresses-invalid", "words", invalid.join(" ")));
    }
    app.toast(t.get("flash-settings-saved"));
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_lines_of_the_licenses_wrap_after_a_space() {
        let text = "short\none two three four\nabcdefghij";
        let lines: Vec<&str> = wrapped(text, 9)
            .into_iter()
            .map(|(a, b)| &text[a..b])
            .collect();
        assert_eq!(
            lines,
            ["short", "one two ", "three ", "four", "abcdefghi", "j"]
        );
    }
}
