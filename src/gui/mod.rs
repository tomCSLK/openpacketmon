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
mod frame;
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
    self, Context, Frame, RichText, ThemePreference, Ui, UiBuilder, ViewportBuilder,
    ViewportCommand,
};

use crate::APP_TITLE;
use crate::config::favorite::Favorite;
use crate::config::settings::ThemeChoice;
use crate::engine::Engine;
use crate::mcp::Access;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::capture_context::CaptureSource;
use crate::networking::types::data_representation::DataUnit;
use crate::notifications::types::sound::Sound;
use crate::view::calls::CallKey;
use crate::view::capture::capture_problem;
use crate::view::filter_index::FilterIndex;
use crate::view::i18n::Lang;
use crate::view::{Jump, Status};
use load::Loader;
use theme::palette;
use widgets::Kind;

/// How often the state of the capture is read while packets come
const LIVE_EVERY: Duration = Duration::from_millis(300);
/// How often it is read otherwise
const IDLE_EVERY: Duration = Duration::from_secs(1);
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
    Settings(SettingsPanel),
    /// The calls (a tab of the statistics)
    Calls,
    Call(CallKey),
}

/// The parts of the settings (`.subnav`), in the order of the navigation
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsPanel {
    #[default]
    Display,
    Notifications,
    PacketStore,
    Calls,
    Geo,
    Assistants,
    Updates,
    About,
}

impl SettingsPanel {
    pub const ALL: [Self; 8] = [
        Self::Display,
        Self::Notifications,
        Self::PacketStore,
        Self::Calls,
        Self::Geo,
        Self::Assistants,
        Self::Updates,
        Self::About,
    ];
}

impl Page {
    /// Each page keeps where it was scrolled to; a detail page of another item starts at the top
    fn scroll_salt(&self) -> String {
        match self {
            Self::Connection(key) => format!("connection {key:?}"),
            Self::Settings(panel) => format!("settings {panel:?}"),
            Self::Packet(number) => format!("packet {number}"),
            Self::Stream(number) => format!("stream {number}"),
            Self::Call(key) => format!("call {key:?}"),
            other => format!("{:?}", std::mem::discriminant(other)),
        }
    }
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
            Jump::Notifications => Self::Notifications,
            Jump::Call(key) => Self::Call(key),
        }
    }
}

/// The entries of the header
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
    /// The entry of the header this page belongs to
    fn nav(&self) -> Nav {
        match self {
            Self::Capture => Nav::Capture,
            Self::Overview => Nav::Overview,
            Self::Connections { .. } | Self::Connection(_) => Nav::Connections,
            Self::Packets { .. } | Self::Packet(_) | Self::Stream(_) => Nav::Packets,
            Self::Statistics { .. } | Self::Calls | Self::Call(_) => Nav::Statistics,
            Self::Entries(Favorite::Host) => Nav::Hosts,
            Self::Entries(Favorite::Service) => Nav::Services,
            Self::Entries(Favorite::Program) => Nav::Programs,
            Self::Notifications => Nav::Notifications,
            Self::Settings(_) => Nav::Settings,
        }
    }

    /// A page without filters, as the header opens it
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
            Nav::Settings => Self::Settings(SettingsPanel::Display),
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
            status: Status::read(t, &state),
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
    /// The access for AI assistants
    pub mcp: Access,
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
    /// Dialogs and the note on a new version
    overlays: frame::Overlays,
    /// Where the note on a new version sits in the status bar
    update_button: Option<egui::Rect>,
    /// The user has answered: the window may close
    close_now: bool,
    sound_seen: u64,
    /// A short message at the bottom of the window (`.toast`) and since when it shows
    toast: Option<(String, std::time::Instant)>,
    pub pages: pages::Pages,
}

impl App {
    fn new(ctx: &Context, engine: Engine, mcp: Access) -> Self {
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
            mcp,
            t,
            index: Arc::new(FilterIndex::default()),
            live,
            live_loader: Loader::default(),
            page,
            history: Vec::new(),
            closing: false,
            overlays: frame::Overlays::default(),
            update_button: None,
            close_now: false,
            toast: None,
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

    /// Shows a short message at the bottom of the window for a few seconds (`App.toast`)
    pub fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), std::time::Instant::now()));
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

    /// Light or dark, from the header or the key `t`
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
        let mut confirm = false;
        let mut cancel = false;
        let close = frame::dialog(
            ctx,
            "close-dialog",
            t,
            &t.get("gui-close-title"),
            Some(icons::ALERT),
            |ui| {
                ui.label(RichText::new(t.get("gui-close-text")).size(theme::FS_BASE));
                ui.label(
                    RichText::new(t.get("stop-save-hint"))
                        .size(theme::FS_SM)
                        .color(palette(ui.ctx()).text_3),
                );
            },
            |ui| {
                confirm = widgets::icon_text_button(
                    ui,
                    Some(icons::STOP),
                    &t.get("gui-close-confirm"),
                    Kind::Danger,
                    false,
                )
                .clicked();
                cancel = widgets::button(ui, t.get("gui-close-cancel"), Kind::Secondary).clicked();
            },
        );
        if confirm {
            self.close_now = true;
            self.closing = false;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else if close || cancel {
            self.closing = false;
        }
    }

    fn content(&mut self, ui: &mut Ui) {
        if let Some(problem) = self.live.problem.clone() {
            widgets::error(ui, &problem);
        }
        pages::show(self, ui);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        frame::keys(self, &ctx);
        frame::header(self, ui);
        frame::status_bar(self, ui);
        let p = palette(&ctx);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg))
            .show(ui, |ui| {
                // the packet list fills the window; every other page scrolls as a whole, with
                // its head (`#main.fill` and `#main.page`)
                if matches!(self.page, Page::Packets { .. }) {
                    let room = frame::page_rect(&ctx, ui.max_rect());
                    ui.scope_builder(UiBuilder::new().max_rect(room), |ui| {
                        ui.spacing_mut().item_spacing.y = theme::SP4;
                        self.content(ui);
                    });
                } else {
                    // the bar of the page floats over its margin, as in the draft, and takes no room
                    let inner = ui.spacing().scroll.floating_allocated_width;
                    ui.spacing_mut().scroll.floating_allocated_width = 0.0;
                    egui::ScrollArea::vertical()
                        .id_salt(("page", self.page.scroll_salt()))
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            // the scrolled room is endless; its pages grow beyond this height
                            let mut panel = ui.max_rect();
                            panel.max.y = panel.min.y + ui.clip_rect().height();
                            let room = frame::page_rect(&ctx, panel);
                            ui.scope_builder(UiBuilder::new().max_rect(room), |ui| {
                                ui.spacing_mut().scroll.floating_allocated_width = inner;
                                ui.spacing_mut().item_spacing.y = theme::SP4;
                                self.content(ui);
                                ui.add_space(frame::page_bottom(&ctx) - theme::SP4);
                            });
                        });
                }
            });
        frame::update_note(self, &ctx);
        frame::toast(&mut self.toast, &ctx);
        frame::help_dialog(self, &ctx);
        frame::stop_dialog(self, &ctx);
        self.close_dialog(&ctx);
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
            .map(|v| v.replace('_', "-"))
            .or_else(windows_languages)
            .map(|v| Lang::from_accept_language(&v))
            .unwrap_or_default()
    })
}

/// The display languages of Windows in the order of the user, like `de-DE,en-US`; the variables
/// above are only set there by shells such as Git Bash
#[cfg(windows)]
fn windows_languages() -> Option<String> {
    let languages = sys_locale::get_locales().collect::<Vec<_>>().join(",");
    (!languages.is_empty()).then_some(languages)
}

#[cfg(not(windows))]
fn windows_languages() -> Option<String> {
    None
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

/// How the window draws: the defaults of egui, except for the graphics interfaces under Windows
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    #[cfg(windows)]
    {
        use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
        use eframe::wgpu::Backends;

        // Under Windows the window flickered with Vulkan whenever the mouse moved (Intel Arc,
        // 2026-11-02), not with DirectX 12 or OpenGL: take DirectX 12, else OpenGL.
        // `WGPU_BACKEND` still decides when it is set.
        let mut setup = WgpuSetupCreateNew::without_display_handle();
        setup.instance_descriptor.backends =
            Backends::from_env().unwrap_or(Backends::DX12 | Backends::GL);
        WgpuConfiguration {
            wgpu_setup: WgpuSetup::CreateNew(setup),
            ..WgpuConfiguration::default()
        }
    }
    #[cfg(not(windows))]
    eframe::egui_wgpu::WgpuConfiguration::default()
}

/// Opens the window and returns when it is closed
pub fn run(engine: Engine, mcp: Access) -> Result<(), eframe::Error> {
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
        wgpu_options: wgpu_options(),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        crate::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(&cc.egui_ctx, engine, mcp)))),
    )
}
