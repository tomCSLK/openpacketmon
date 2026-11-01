//! The native window: the pages of `crate::view`, drawn with egui.
//!
//! The window never waits for the capture: everything that reads the engine runs in the
//! background (see [`load`]), and each page shows the last result until the next one is there.

// Sizes on the screen are floats, counts are not; a page reads best as one function from top to
// bottom; and what the user did in a frame is a set of switches.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

mod chart;
mod filter_field;
mod icons;
mod load;
mod pages;
mod sound;
#[cfg(test)]
mod tests;
mod theme;
mod widgets;

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{
    self, Color32, Context, CornerRadius, FontId, Frame, Layout, Margin, Mesh, Modal, Panel,
    Response, RichText, Sense, Shape, Stroke, ThemePreference, Ui, ViewportBuilder,
    ViewportCommand, WidgetInfo,
};

use crate::APP_TITLE;
use crate::config::favorite::Favorite;
use crate::config::settings::ThemeChoice;
use crate::engine::Engine;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::capture_context::CaptureSource;
use crate::networking::types::data_representation::DataUnit;
use crate::notifications::types::sound::Sound;
use crate::utils::types::timestamp::Timestamp;
use crate::view::capture::capture_problem;
use crate::view::filter_index::FilterIndex;
use crate::view::i18n::Lang;
use crate::view::{Jump, Status};
use load::Loader;
use theme::{palette, semibold_font};
use widgets::Kind;

/// How often the state of the capture is read while packets come
const LIVE_EVERY: Duration = Duration::from_millis(300);
/// How often it is read otherwise
const IDLE_EVERY: Duration = Duration::from_secs(1);
/// Width of the side bar
const SIDEBAR_WIDTH: f32 = 232.0;
const SOURCE_URL: &str = env!("CARGO_PKG_REPOSITORY");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A page of the window, with what it shows; the way back is a list of these
#[derive(Clone, PartialEq, Debug)]
pub enum Page {
    Capture,
    Overview,
    Connections {
        filter: String,
    },
    Connection(AddressPortPair),
    Packets {
        filter: String,
        connections: String,
        goto: Option<u64>,
    },
    Packet(u64),
    Stream(u64),
    Statistics {
        filter: String,
        connections: String,
    },
    Entries(Favorite),
    Notifications,
    Settings,
    Licenses,
}

impl From<Jump> for Page {
    fn from(jump: Jump) -> Self {
        match jump {
            Jump::Connection(key) => Self::Connection(key),
            Jump::Connections { filter } => Self::Connections { filter },
            Jump::Packets {
                filter,
                connections,
            } => Self::Packets {
                filter,
                connections,
                goto: None,
            },
            Jump::Statistics {
                filter,
                connections,
            } => Self::Statistics {
                filter,
                connections,
            },
        }
    }
}

/// The entries of the side bar
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Nav {
    Overview,
    Connections,
    Packets,
    Statistics,
    Hosts,
    Services,
    Programs,
    Notifications,
    Capture,
    Settings,
}

impl Page {
    /// The entry of the side bar this page belongs to
    fn nav(&self) -> Option<Nav> {
        match self {
            Self::Capture => Some(Nav::Capture),
            Self::Overview => Some(Nav::Overview),
            Self::Connections { .. } | Self::Connection(_) => Some(Nav::Connections),
            Self::Packets { .. } | Self::Packet(_) | Self::Stream(_) => Some(Nav::Packets),
            Self::Statistics { .. } => Some(Nav::Statistics),
            Self::Entries(Favorite::Host) => Some(Nav::Hosts),
            Self::Entries(Favorite::Service) => Some(Nav::Services),
            Self::Entries(Favorite::Program) => Some(Nav::Programs),
            Self::Notifications => Some(Nav::Notifications),
            Self::Settings => Some(Nav::Settings),
            Self::Licenses => None,
        }
    }

    /// A page without filters, as the side bar opens it
    fn of_nav(nav: Nav) -> Self {
        match nav {
            Nav::Overview => Self::Overview,
            Nav::Connections => Self::Connections {
                filter: String::new(),
            },
            Nav::Packets => Self::Packets {
                filter: String::new(),
                connections: String::new(),
                goto: None,
            },
            Nav::Statistics => Self::Statistics {
                filter: String::new(),
                connections: String::new(),
            },
            Nav::Hosts => Self::Entries(Favorite::Host),
            Nav::Services => Self::Entries(Favorite::Service),
            Nav::Programs => Self::Entries(Favorite::Program),
            Nav::Notifications => Self::Notifications,
            Nav::Capture => Self::Capture,
            Nav::Settings => Self::Settings,
        }
    }
}

/// The state of the capture, read in the background for all pages
pub struct Live {
    pub status: Status,
    pub running: bool,
    pub frozen: bool,
    /// An analyzed file has been read completely
    pub finished: bool,
    /// The capture stopped with an error, for example because the capture helper ended
    pub problem: Option<String>,
    /// Closing the window ends a capture that cannot be read again
    pub capturing: bool,
    pub capture_id: usize,
    pub unit: DataUnit,
    sound: (u64, Sound),
    volume: u8,
}

impl Live {
    fn read(t: Lang, engine: &Engine) -> Self {
        let state = engine.lock();
        Self {
            status: Status::read(&state),
            running: state.is_running(),
            frozen: state.frozen,
            finished: state.traffic_history.no_more_packets(),
            problem: capture_problem(t, &state),
            capturing: state.is_running()
                && !matches!(state.capture_source, CaptureSource::File(_)),
            capture_id: state.capture_id(),
            unit: state.conf.data_repr.data_unit,
            sound: (state.sound_event.0, state.sound_event.1),
            volume: state.conf.settings.notifications.volume,
        }
    }
}

/// The window
pub struct App {
    pub engine: Engine,
    pub t: Lang,
    /// The packets that pass the display filters in use
    pub index: Arc<FilterIndex>,
    pub live: Live,
    live_loader: Loader<(), Live>,
    pub page: Page,
    /// The pages to go back to
    history: Vec<Page>,
    /// Asking whether the running capture may end with the window
    closing: bool,
    /// The user has answered: the window may close
    close_now: bool,
    sound_seen: u64,
    pub pages: pages::Pages,
}

impl App {
    fn new(ctx: &Context, engine: Engine) -> Self {
        egui_extras::install_image_loaders(ctx);
        theme::install(ctx);
        let (language, choice) = {
            let state = engine.lock();
            (state.conf.language.clone(), state.conf.settings.theme)
        };
        ctx.set_theme(theme_preference(choice));
        let t = language_of(&language);
        let live = Live::read(t, &engine);
        let page = if live.running {
            Page::Overview
        } else {
            Page::Capture
        };
        Self {
            sound_seen: live.sound.0,
            engine,
            t,
            index: Arc::new(FilterIndex::default()),
            live,
            live_loader: Loader::default(),
            page,
            history: Vec::new(),
            closing: false,
            close_now: false,
            pages: pages::Pages::default(),
        }
    }

    /// Opens a page; the current one becomes a step back
    pub fn go(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        let from = std::mem::replace(&mut self.page, page);
        self.pages.settings.close();
        self.history.push(from);
        if self.history.len() > 200 {
            self.history.remove(0);
        }
    }

    /// Replaces the current page without a step back, for paging through packets
    pub fn replace(&mut self, page: Page) {
        self.page = page;
    }

    pub fn jump(&mut self, jump: Jump) {
        self.go(jump.into());
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn back(&mut self) {
        self.pages.settings.close();
        if let Some(page) = self.history.pop() {
            self.page = page;
        }
    }

    /// The arrow left of a title
    pub fn back_arrow(&mut self, ui: &mut Ui) {
        if !self.can_go_back() {
            return;
        }
        let p = palette(ui.ctx());
        let hint = self.t.get("back-hint");
        if widgets::icon_button(ui, icons::BACK, p.muted, &hint).clicked() {
            self.back();
        }
    }

    /// The design changed in the side bar
    fn toggle_theme(&mut self, ctx: &Context) {
        let choice = match ctx.theme() {
            egui::Theme::Dark => ThemeChoice::Light,
            egui::Theme::Light => ThemeChoice::Dark,
        };
        ctx.set_theme(theme_preference(choice));
        self.engine.lock().conf.settings.theme = choice;
        self.engine.store_conf();
    }

    /// Reads the state of the capture and acts on what changed
    fn tick(&mut self, ctx: &Context) {
        if self.live_loader.poll()
            && let Some(live) = self.live_loader.take()
        {
            self.live = live;
        }
        let every = if self.live.running && !self.live.frozen && !self.live.finished {
            LIVE_EVERY
        } else {
            IDLE_EVERY
        };
        let engine = self.engine.clone();
        let t = self.t;
        self.live_loader
            .load(ctx, &(), Some(every), move || Live::read(t, &engine));
        ctx.request_repaint_after(every);

        if self.live.sound.0 != self.sound_seen {
            self.sound_seen = self.live.sound.0;
            sound::play(self.live.sound.1, self.live.volume);
        }

        let (back, close) = ctx.input(|i| {
            (
                i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)
                    || i.pointer.button_pressed(egui::PointerButton::Extra1),
                i.viewport().close_requested(),
            )
        });
        if back {
            self.back();
        }
        if close && !self.close_now && self.live.capturing {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.closing = true;
        }
    }

    fn close_dialog(&mut self, ctx: &Context) {
        if !self.closing {
            return;
        }
        let t = self.t;
        let response = Modal::new(egui::Id::NULL.with("close-dialog")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(RichText::new(t.get("gui-close-title")).font(semibold_font(16.0)));
            ui.add_space(4.0);
            ui.label(t.get("gui-close-text"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if widgets::button(ui, t.get("gui-close-confirm"), Kind::Danger).clicked() {
                    self.close_now = true;
                    self.closing = false;
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                if widgets::button(ui, t.get("gui-close-cancel"), Kind::Secondary).clicked() {
                    self.closing = false;
                }
            });
        });
        if response.should_close() {
            self.closing = false;
        }
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let p = palette(ui.ctx());
        let rect = ui.max_rect();
        let mut mesh = Mesh::default();
        mesh.colored_vertex(rect.left_top(), p.side_top);
        mesh.colored_vertex(rect.right_top(), p.side_top);
        mesh.colored_vertex(rect.left_bottom(), p.side_bottom);
        mesh.colored_vertex(rect.right_bottom(), p.side_bottom);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        ui.painter().add(Shape::mesh(mesh));

        Panel::bottom("sidebar-foot")
            .frame(Frame::new().inner_margin(Margin::same(12)))
            .show_separator_line(false)
            .resizable(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                self.sidebar_foot(ui);
            });
        Frame::new().inner_margin(Margin::same(12)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(icons::LOGO)
                        .fit_to_exact_size(egui::vec2(32.0, 32.0))
                        .corner_radius(CornerRadius::same(6)),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(APP_TITLE)
                            .font(semibold_font(16.0))
                            .color(p.side_text),
                    )
                    .selectable(false),
                );
            });
            ui.add_space(10.0);
            egui::ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .auto_shrink(false)
                .show(ui, |ui| self.nav(ui));
        });
    }

    fn nav(&mut self, ui: &mut Ui) {
        let t = self.t;
        {
            nav_label(ui, &t.get("nav-traffic"));
            let current = self.page.nav();
            for (nav, icon, id) in [
                (Nav::Overview, icons::OVERVIEW, "nav-overview"),
                (Nav::Connections, icons::CONNECTIONS, "nav-connections"),
                (Nav::Packets, icons::PACKETS, "nav-packets"),
                (Nav::Statistics, icons::STATISTICS, "nav-statistics"),
                (Nav::Hosts, icons::HOSTS, "nav-hosts"),
                (Nav::Services, icons::SERVICES, "nav-services"),
                (Nav::Programs, icons::PROGRAMS, "nav-programs"),
                (
                    Nav::Notifications,
                    icons::NOTIFICATIONS,
                    "nav-notifications",
                ),
            ] {
                let badge = (nav == Nav::Notifications).then_some(self.live.status.unread);
                if nav_item(ui, icon, &t.get(id), current == Some(nav), badge).clicked() {
                    self.open_nav(nav);
                }
            }
            ui.add_space(10.0);
            nav_label(ui, &t.get("nav-setup"));
            for (nav, icon, id) in [
                (Nav::Capture, icons::CAPTURE, "nav-capture"),
                (Nav::Settings, icons::SETTINGS, "nav-settings"),
            ] {
                if nav_item(ui, icon, &t.get(id), current == Some(nav), None).clicked() {
                    self.open_nav(nav);
                }
            }
        }
    }

    fn open_nav(&mut self, nav: Nav) {
        self.go(Page::of_nav(nav));
    }

    fn sidebar_foot(&mut self, ui: &mut Ui) {
        let p = palette(ui.ctx());
        let t = self.t;
        ui.visuals_mut().hyperlink_color = p.side_muted;
        if nav_item(ui, icons::THEME, &t.get("nav-theme"), false, None).clicked() {
            self.toggle_theme(ui.ctx());
        }
        if let Some(version) = self.live.status.update.clone() {
            ui.hyperlink_to(
                RichText::new(t.get1("update-available", "version", version.as_str()))
                    .small()
                    .color(p.side_text),
                format!("{SOURCE_URL}/releases"),
            );
        }
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 0.0),
            Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 1.0);
                let meta = |text: String| RichText::new(text).small().color(p.side_muted);
                ui.label(meta(format!("{APP_TITLE} {VERSION} ·")));
                ui.hyperlink_to(meta(t.get("source-code")), SOURCE_URL);
                ui.label(meta(format!("· {} ·", t.get("based-on-sniffnet"))));
                if ui.link(meta(t.get("licenses"))).clicked() {
                    self.go(Page::Licenses);
                }
                if self.live.status.geo_dbip {
                    ui.label(meta("·".to_owned()));
                    ui.hyperlink_to(
                        meta("IP Geolocation by DB-IP".to_owned()),
                        "https://db-ip.com",
                    );
                }
            },
        );
    }

    fn content(&mut self, ui: &mut Ui) {
        if let Some(problem) = self.live.problem.clone() {
            widgets::error(ui, &problem);
            ui.add_space(4.0);
        }
        pages::show(self, ui);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        Panel::left("sidebar")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new())
            .show(ui, |ui| self.sidebar(ui));
        let p = palette(&ctx);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::same(18)))
            .show(ui, |ui| self.content(ui));
        self.close_dialog(&ctx);
    }
}

/// The heading of a group of the side bar
fn nav_label(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new(text).small().color(p.side_muted)).selectable(false));
    ui.add_space(2.0);
}

/// An entry of the side bar
fn nav_item(
    ui: &mut Ui,
    icon: egui::ImageSource<'static>,
    text: &str,
    active: bool,
    badge: Option<usize>,
) -> Response {
    let p = palette(ui.ctx());
    let size = egui::vec2(ui.available_width(), 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, active, text));
    let radius = CornerRadius::same(8);
    if active {
        ui.painter().rect_filled(rect, radius, p.side_active);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, radius, p.side_hover);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(2.0, Color32::from_rgb(0xd9, 0xf5, 0x5b)),
            egui::StrokeKind::Inside,
        );
    }
    let color = if active {
        p.side_active_text
    } else {
        p.side_text
    };
    let icon_rect = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(20.0, 0.0),
        egui::vec2(18.0, 18.0),
    );
    egui::Image::new(icon).tint(color).paint_at(ui, icon_rect);
    let count = badge.filter(|n| *n > 0);
    let text_left = rect.left() + 38.0;
    let mut text_right = rect.right() - 6.0;
    if let Some(count) = count {
        let label = if count > 99 {
            "99+".to_owned()
        } else {
            count.to_string()
        };
        let galley = ui
            .painter()
            .layout_no_wrap(label, semibold_font(11.5), Color32::WHITE);
        // a rectangle, not a circle, and never narrower than this
        let size = egui::vec2((galley.size().x + 11.0).max(22.0), 20.0);
        let right = rect.right_center() - egui::vec2(6.0, 0.0);
        let badge = egui::Rect::from_center_size(right - egui::vec2(size.x / 2.0, 0.0), size);
        ui.painter()
            .rect_filled(badge, CornerRadius::same(5), p.down);
        ui.painter()
            .galley(badge.center() - galley.size() / 2.0, galley, Color32::WHITE);
        text_right = badge.left() - 6.0;
    }
    // the text gives way to the number
    let font = FontId::proportional(14.0);
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(text_right - text_left);
    let galley = ui.painter().layout_job(job);
    let top = rect.center().y - galley.size().y / 2.0;
    ui.painter()
        .galley(egui::pos2(text_left, top), galley, color);
    match count {
        Some(count) if count > 99 => response.on_hover_text(count.to_string()),
        _ => response,
    }
}

pub fn theme_preference(choice: ThemeChoice) -> ThemePreference {
    match choice {
        ThemeChoice::System => ThemePreference::System,
        ThemeChoice::Light => ThemePreference::Light,
        ThemeChoice::Dark => ThemePreference::Dark,
    }
}

/// The language of the settings, otherwise the one of the desktop
pub fn language_of(code: &str) -> Lang {
    Lang::parse(code).unwrap_or_else(|| {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|v| !v.is_empty()))
            .map(|v| Lang::from_accept_language(&v.replace('_', "-")))
            .unwrap_or_default()
    })
}

/// A time in the time zone of the computer
pub fn local_time(t: Lang, ts: Timestamp) -> String {
    let Ok(time) = jiff::Timestamp::from_second(ts.secs()) else {
        return String::new();
    };
    let format = match t {
        Lang::De => "%d.%m.%Y %H:%M:%S",
        Lang::En => "%Y-%m-%d %H:%M:%S",
    };
    time.to_zoned(jiff::tz::TimeZone::system())
        .strftime(format)
        .to_string()
}

fn window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../../resources/packaging/linux/graphics/openpacketmon_64x64.png");
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

/// Opens the window and returns when it is closed
pub fn run(engine: Engine) -> Result<(), eframe::Error> {
    let mut viewport = ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_app_id(crate::APP_NAME)
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([760.0, 480.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        crate::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(&cc.egui_ctx, engine)))),
    )
}
