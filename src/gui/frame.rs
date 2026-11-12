//! The frame around every page, as in the draft (`docs/entwurf/assets/app.js`, `app.css`): the
//! header with the navigation, the capture and the icons on the right; the status bar; the note on a
//! new version; the help on the keys; the question before a capture ends.

use std::time::Duration;

use eframe::egui::{
    self, Align, Align2, Color32, Context, CornerRadius, Frame, Id, ImageSource, Key, Layout,
    Margin, Modal, Panel, Rect, Response, RichText, Sense, Stroke, Ui, WidgetInfo, pos2, vec2,
};

use crate::APP_TITLE;
use crate::gui::theme::{
    self, FS_BASE, FS_LG, FS_SM, FS_XS, medium_font, mono_font, mono_medium_font, palette,
    semibold_font,
};
use crate::gui::widgets::{self, Kind, focus_ring, paint_icon};
use crate::gui::{App, Nav, Page, SOURCE_URL, SettingsPanel, VERSION, icons, pages};
use crate::view::CaptureMode;
use crate::view::i18n::Lang;

/// Widths of the window below which parts of the frame give way (draft `app.css`, accepted
/// 2026-11-10): the name of the program; source code and Sniffnet in the status bar
pub const W_LG: f32 = 1440.0;
/// "Aufzeichnung läuft" in the header
const W_WHAT: f32 = 1280.0;
/// Dropped packets and the version in the status bar
pub const W_MD: f32 = 1200.0;
/// Hosts, services and programs move under "More"
pub const W_MORE: f32 = 1180.0;
/// The source of the capture in the header
const W_SRC: f32 = 1024.0;
/// Only icons in the navigation
const W_ICONS: f32 = 960.0;
/// A tighter header; packet store and the note on AI assistants leave the status bar
pub const W_SM: f32 = 900.0;

/// The id under which a page finds the request to focus its filter field (key `/`)
pub const FOCUS_FILTER: &str = "focus-filter";

/// An id from anything that can be hashed
fn id(salt: impl std::hash::Hash + std::fmt::Debug) -> Id {
    Id::NULL.with(salt)
}

/// The width of the window
pub fn width(ctx: &Context) -> f32 {
    ctx.content_rect().width()
}

/// The entries of the navigation; the last three move under "More" in narrow windows
const NAV: [(Nav, ImageSource<'static>, &str); 7] = [
    (Nav::Overview, icons::OVERVIEW, "nav-overview"),
    (Nav::Connections, icons::CONNECTIONS, "nav-connections"),
    (Nav::Packets, icons::PACKETS, "nav-packets"),
    (Nav::Statistics, icons::STATISTICS, "nav-statistics"),
    (Nav::Hosts, icons::HOSTS, "nav-hosts"),
    (Nav::Services, icons::SERVICES, "nav-services"),
    (Nav::Programs, icons::PROGRAMS, "nav-programs"),
];

/// What the frame shows on top of the pages
#[derive(Default)]
pub struct Overlays {
    /// The help on the keys
    pub help: bool,
    /// The question before the capture ends
    pub stop: bool,
    /// The note on a new version above the status bar
    pub update: bool,
}

/// A line of text painted at `x`, centered on `y`; returns its width
fn text_at(ui: &Ui, x: f32, y: f32, text: &str, font: egui::FontId, color: Color32) -> f32 {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let width = galley.size().x;
    ui.painter()
        .galley(pos2(x, y - galley.size().y / 2.0), galley, color);
    width
}

fn text_width(ui: &Ui, text: &str, font: egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER)
        .size()
        .x
}

/// The header: logo, navigation, the capture and the icons on the right (`.topbar`)
pub fn header(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let p = palette(&ctx);
    let w = width(&ctx);
    let (margin, gap) = if w < W_SM {
        (
            Margin {
                left: 12,
                right: 8,
                top: 0,
                bottom: 0,
            },
            theme::SP2,
        )
    } else {
        (
            Margin {
                left: 16,
                right: 12,
                top: 0,
                bottom: 0,
            },
            theme::SP3,
        )
    };
    Panel::top("header")
        .exact_size(theme::HEADER_H)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(p.surface).inner_margin(margin))
        .show(ui, |ui| {
            let full = ui.max_rect();
            let line = Rect::from_min_max(
                pos2(full.left() - f32::from(margin.left), full.bottom() - 1.0),
                pos2(full.right() + f32::from(margin.right), full.bottom()),
            );
            ui.painter().rect_filled(line, 0.0, p.border);
            ui.spacing_mut().item_spacing.x = gap;
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                brand(app, ui, w >= W_LG, gap);
                navigation(app, ui, w);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let current = app.page.nav();
                    let t = app.t;
                    if widgets::iconbtn(
                        ui,
                        icons::SETTINGS,
                        &t.get("nav-settings"),
                        current == Nav::Settings,
                        false,
                    )
                    .clicked()
                    {
                        app.go(Page::Settings(SettingsPanel::Display));
                    }
                    if widgets::iconbtn(ui, icons::HELP, &t.get("keys-hint"), false, false)
                        .clicked()
                    {
                        app.overlays.help = true;
                    }
                    let dark = ctx.theme() == egui::Theme::Dark;
                    let icon = if dark { icons::SUN } else { icons::MOON };
                    if widgets::iconbtn(ui, icon, &t.get("theme-toggle"), false, false)
                        .on_hover_text(t.get("theme-toggle-hint"))
                        .clicked()
                    {
                        app.toggle_theme(&ctx);
                    }
                    bell(app, ui, current == Nav::Notifications);
                    capture_chip(app, ui, w);
                });
            });
        });
}

/// The logo and, in wide windows, the name; leads to the overview (`.brand`)
fn brand(app: &mut App, ui: &mut Ui, name: bool, gap: f32) {
    let p = palette(ui.ctx());
    let font = semibold_font(theme::FS_MD);
    let name_w = if name {
        theme::SP2 + text_width(ui, APP_TITLE, font.clone())
    } else {
        0.0
    };
    let (rect, response) = ui.allocate_exact_size(vec2(24.0 + name_w, 24.0), Sense::click());
    let label = app.t.get("brand-hint");
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Link, true, &label));
    paint_icon(
        ui,
        icons::LOGO,
        Rect::from_min_size(rect.min, vec2(24.0, 24.0)),
        p.accent,
    );
    if name {
        text_at(
            ui,
            rect.left() + 24.0 + theme::SP2,
            rect.center().y,
            APP_TITLE,
            font,
            p.text,
        );
        // the name keeps a little room to the navigation
        ui.add_space(theme::SP2.min(gap));
    }
    focus_ring(ui, &response, theme::RADIUS_SM);
    if response.clicked() {
        app.go(Page::Overview);
    }
}

/// One entry of the navigation, full height, underlined when its page is open
fn nav_entry(
    ui: &mut Ui,
    rect: Rect,
    icon: ImageSource<'static>,
    text: &str,
    label: bool,
    active: bool,
    chevron: bool,
) -> Response {
    let p = palette(ui.ctx());
    let response = ui.interact(rect, id(("nav", text)), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::Role::Link, true, active, text));
    let hovered = response.hovered();
    if hovered {
        ui.painter().rect_filled(rect, 0.0, p.hover);
    }
    if active {
        let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.right_bottom());
        ui.painter().rect_filled(bar, 0.0, p.accent);
    }
    let color = if active || hovered { p.text } else { p.text_2 };
    let pad = if label { 10.0 } else { 9.0 };
    let y = rect.center().y;
    let mut x = rect.left() + pad;
    paint_icon(
        ui,
        icon,
        Rect::from_min_size(pos2(x, y - 8.0), vec2(16.0, 16.0)),
        color,
    );
    x += 16.0;
    if label {
        x += 6.0;
        x += text_at(ui, x, y, text, medium_font(FS_BASE), color);
    }
    if chevron {
        x += 6.0;
        paint_icon(
            ui,
            icons::CHEV_DOWN,
            Rect::from_min_size(pos2(x, y - 7.0), vec2(14.0, 14.0)),
            color,
        );
    }
    focus_ring(ui, &response, 0);
    if label {
        response
    } else {
        response.on_hover_text(text)
    }
}

/// The width of an entry of the navigation
fn nav_width(ui: &Ui, text: &str, label: bool, chevron: bool) -> f32 {
    let pad = if label { 10.0 } else { 9.0 };
    let mut w = 2.0 * pad + 16.0;
    if label {
        w += 6.0 + text_width(ui, text, medium_font(FS_BASE));
    }
    if chevron {
        w += 6.0 + 14.0;
    }
    w
}

/// The navigation: all pages in wide windows, the last three under "More" below 1180 px (`.nav`)
#[allow(clippy::similar_names)]
fn navigation(app: &mut App, ui: &mut Ui, w: f32) {
    let t = app.t;
    let label = w >= W_ICONS;
    let more = w < W_MORE;
    let current = app.page.nav();
    let shown = if more { &NAV[..4] } else { &NAV[..] };
    let texts: Vec<String> = shown.iter().map(|(_, _, id)| t.get(id)).collect();
    let more_text = t.get("nav-more");
    let mut widths: Vec<f32> = texts
        .iter()
        .map(|x| nav_width(ui, x, label, false))
        .collect();
    if more {
        widths.push(nav_width(ui, &more_text, label, true));
    }
    let total: f32 = widths.iter().sum();
    let (area, _) = ui.allocate_exact_size(vec2(total, theme::HEADER_H), Sense::hover());
    let mut x = area.left();
    for ((nav, icon, _), (text, width)) in shown.iter().zip(texts.iter().zip(&widths)) {
        let rect = Rect::from_min_size(pos2(x, area.top()), vec2(*width, theme::HEADER_H - 1.0));
        if nav_entry(ui, rect, icon.clone(), text, label, current == *nav, false).clicked() {
            app.go(Page::of_nav(*nav));
        }
        x += width;
    }
    if !more {
        return;
    }
    let rest = &NAV[4..];
    let active = rest.iter().any(|(nav, _, _)| current == *nav);
    let rect = Rect::from_min_size(
        pos2(x, area.top()),
        vec2(widths[widths.len() - 1], theme::HEADER_H - 1.0),
    );
    let response = nav_entry(ui, rect, icons::MORE, &more_text, label, active, true);
    let response = if label {
        response.on_hover_text(t.get("nav-more-hint"))
    } else {
        response
    };
    let p = palette(ui.ctx());
    egui::Popup::menu(&response)
        .gap(6.0)
        .align(egui::RectAlign::BOTTOM_START)
        .frame(menu_frame(p))
        .show(|ui| {
            ui.set_min_width(220.0 - 8.0);
            for (nav, icon, id) in rest {
                if menu_item(ui, icon.clone(), &t.get(id), current == *nav).clicked() {
                    app.go(Page::of_nav(*nav));
                    ui.close();
                }
            }
        });
}

/// The frame of a menu (`.menu-pop`)
pub fn menu_frame(p: &theme::Palette) -> Frame {
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .shadow(p.shadow_2())
        .inner_margin(Margin::same(theme::SP1 as i8))
}

/// An entry of a menu: icon and text, the current page marked
pub fn menu_item(ui: &mut Ui, icon: ImageSource<'static>, text: &str, current: bool) -> Response {
    let p = palette(ui.ctx());
    let height = 7.0 * 2.0 + ui.text_style_height(&egui::TextStyle::Body).max(16.0);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::Role::MenuItem, true, current, text));
    let radius = CornerRadius::same(theme::RADIUS_SM);
    if response.hovered() {
        ui.painter().rect_filled(rect, radius, p.hover);
    } else if current {
        ui.painter().rect_filled(rect, radius, p.selected);
    }
    let y = rect.center().y;
    paint_icon(
        ui,
        icon,
        Rect::from_min_size(pos2(rect.left() + 10.0, y - 8.0), vec2(16.0, 16.0)),
        p.text,
    );
    text_at(
        ui,
        rect.left() + 10.0 + 16.0 + theme::SP2,
        y,
        text,
        egui::FontId::proportional(FS_BASE),
        p.text,
    );
    response
}

/// The bell with the number of new notifications
fn bell(app: &mut App, ui: &mut Ui, current: bool) {
    let t = app.t;
    let unread = app.live.status.unread;
    let label = if unread > 0 {
        t.get1("bell-unread", "count", unread)
    } else {
        t.get("nav-notifications")
    };
    let response = widgets::iconbtn(ui, icons::BELL, &label, current, false);
    if unread > 0 {
        let p = palette(ui.ctx());
        let text = if unread > 99 {
            "99+".to_owned()
        } else {
            unread.to_string()
        };
        let galley = ui
            .painter()
            .layout_no_wrap(text, mono_medium_font(10.0), p.accent_text);
        let size = vec2((galley.size().x + 8.0).max(16.0), 16.0);
        let badge = Rect::from_min_size(
            pos2(response.rect.right() - size.x, response.rect.top() + 2.0),
            size,
        );
        // the border of 1.5 px in the color of the header sets the badge off from the bell
        ui.painter()
            .rect_filled(badge.expand(1.5), CornerRadius::same(10), p.surface);
        ui.painter()
            .rect_filled(badge, CornerRadius::same(8), p.accent);
        ui.painter()
            .galley(badge.center() - galley.size() / 2.0, galley, p.accent_text);
    }
    if response.clicked() {
        app.go(Page::Notifications);
    }
}

/// The capture at the top right (`.cap`): state, source and time, pause or resume and stop; without
/// a capture the button to start one: it opens the capture page, and on that page it starts the
/// capture with the choices there, like the button at the end of the page
#[allow(clippy::many_single_char_names)] // coordinates of the drawing
fn capture_chip(app: &mut App, ui: &mut Ui, w: f32) {
    let t = app.t;
    let p = palette(ui.ctx());
    let Some(chip) = app
        .live
        .status
        .capture
        .as_ref()
        .map(|c| crate::view::CaptureChip {
            mode: c.mode,
            source: c.source.clone(),
            time: c.time.clone(),
        })
    else {
        let on_page = app.page == Page::Capture;
        let starting = on_page && app.pages.capture.starting();
        let clicked = ui
            .add_enabled_ui(!starting, |ui| {
                widgets::icon_text_button(
                    ui,
                    Some(icons::PLAY),
                    &t.get("no-capture-start"),
                    Kind::Primary,
                    true,
                )
                .clicked()
            })
            .inner;
        if clicked && on_page {
            app.pages.capture.start_from_top();
        } else if clicked {
            app.go(Page::Capture);
        }
        return;
    };
    let what = match chip.mode {
        CaptureMode::Running => t.get("capture-running"),
        CaptureMode::Paused => t.get("cap-paused"),
        CaptureMode::Done => t.get("cap-done"),
    };
    let what_font = medium_font(FS_SM);
    let mono = mono_font(FS_XS);
    let what_w = 8.0 + text_width(ui, &what, what_font.clone());
    let src_w = 8.0 + text_width(ui, &chip.source, mono.clone());
    let buttons = if chip.mode == CaptureMode::Done { 1 } else { 2 };
    // the link: 10 px, dot, then the texts 8 px apart, 10 px; the buttons 31 px each
    let fixed = 10.0
        + 8.0
        + 10.0
        + 8.0
        + text_width(ui, &chip.time, mono.clone())
        + 31.0 * buttons as f32
        + 2.0;
    // what does not fit next to the navigation gives way, first the state, then the source
    let room = ui.available_width();
    let mut show_what = w >= W_WHAT;
    let mut show_src = w >= W_SRC;
    if show_what && fixed + what_w + if show_src { src_w } else { 0.0 } > room {
        show_what = false;
    }
    if show_src && fixed + src_w > room {
        show_src = false;
    }
    let link_w = fixed - 31.0 * buttons as f32 - 2.0
        + if show_what { what_w } else { 0.0 }
        + if show_src { src_w } else { 0.0 };
    let total = link_w + 31.0 * buttons as f32 + 2.0;
    let (outer, _) = ui.allocate_exact_size(vec2(total, theme::CTL_H), Sense::hover());
    let radius = CornerRadius::same(theme::RADIUS);
    ui.painter().rect_filled(outer, radius, p.surface_2);
    let inner = outer.shrink(1.0);
    let link = Rect::from_min_size(inner.min, vec2(link_w, inner.height()));
    let response = ui.interact(link, id("cap-link"), Sense::click());
    let label = format!("{what}, {}, {}", chip.source, chip.time);
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Link, true, &label));
    if response.hovered() {
        ui.painter().rect_filled(
            link,
            CornerRadius {
                nw: theme::RADIUS - 1,
                sw: theme::RADIUS - 1,
                ne: 0,
                se: 0,
            },
            p.hover,
        );
    }
    let y = link.center().y;
    let mut x = link.left() + 10.0;
    let (dot, ring) = match chip.mode {
        CaptureMode::Running => (p.ok, p.ok_bg),
        CaptureMode::Paused => (p.warn, p.warn_bg),
        CaptureMode::Done => (p.off, p.off_bg),
    };
    let center = pos2(x + 4.0, y);
    let spread = if chip.mode == CaptureMode::Running {
        // the dot beats once in 1.6 s; ten pictures a second are enough for it
        let time = ui.input(|i| i.time);
        let phase = (time % 1.6) / 1.6;
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        3.0 + 2.0 * (0.5 - 0.5 * (phase * std::f64::consts::TAU).cos()) as f32
    } else {
        3.0
    };
    ui.painter().circle_filled(center, 4.0 + spread, ring);
    ui.painter().circle_filled(center, 4.0, dot);
    x += 8.0;
    if show_what {
        x += 8.0;
        x += text_at(ui, x, y, &what, what_font, p.text);
    }
    if show_src {
        x += 8.0;
        x += text_at(ui, x, y, &chip.source, mono.clone(), p.text_2);
    }
    x += 8.0;
    text_at(ui, x, y, &chip.time, mono, p.text_3);
    let response = response.on_hover_text(t.get("cap-page-hint"));
    focus_ring(ui, &response, theme::RADIUS);
    if response.clicked() {
        app.go(Page::Capture);
    }

    let mut left = link.right();
    let mut cbtn =
        |ui: &mut Ui, icon: ImageSource<'static>, text: String, stop: bool, last: bool| {
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(left, inner.top()), vec2(1.0, inner.height())),
                0.0,
                p.border,
            );
            let rect =
                Rect::from_min_size(pos2(left + 1.0, inner.top()), vec2(30.0, inner.height()));
            left = rect.right();
            let response = ui.interact(rect, id(("cap-button", stop)), Sense::click());
            response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &text));
            let color = if response.hovered() {
                let corners = if last {
                    CornerRadius {
                        nw: 0,
                        sw: 0,
                        ne: theme::RADIUS - 1,
                        se: theme::RADIUS - 1,
                    }
                } else {
                    CornerRadius::ZERO
                };
                ui.painter().rect_filled(rect, corners, p.hover);
                if stop { p.err } else { p.text }
            } else {
                p.text_2
            };
            paint_icon(
                ui,
                icon,
                Rect::from_center_size(rect.center(), vec2(14.0, 14.0)),
                color,
            );
            focus_ring(ui, &response, theme::RADIUS_SM);
            response.on_hover_text(text).clicked()
        };
    match chip.mode {
        CaptureMode::Running => {
            if cbtn(ui, icons::PAUSE, t.get("capture-pause"), false, false) {
                pages::capture::pause(app);
            }
        }
        CaptureMode::Paused => {
            if cbtn(ui, icons::PLAY, t.get("capture-resume"), false, false) {
                pages::capture::pause(app);
            }
        }
        CaptureMode::Done => {}
    }
    if cbtn(ui, icons::STOP, t.get("capture-stop"), true, true) {
        app.overlays.stop = true;
    }
    ui.painter().rect_stroke(
        outer,
        radius,
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
}

/// A link of the status bar: pale, under the pointer in the color of the text
fn status_link(ui: &mut Ui, text: &str) -> Response {
    let p = palette(ui.ctx());
    let font = egui::FontId::proportional(FS_XS);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Link, true, text));
    let color = if response.hovered() { p.text } else { p.text_3 };
    if response.hovered() {
        let y = rect.bottom() - 2.0;
        ui.painter()
            .hline(rect.x_range(), y, Stroke::new(1.0, color));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().galley(rect.min, galley, color);
    focus_ring(ui, &response, 2);
    response
}

/// A plain text of the status bar
fn status_text(ui: &mut Ui, parts: &[(&str, bool)]) {
    let p = palette(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    for (text, number) in parts {
        let (font, color) = if *number {
            (mono_font(FS_XS), p.text_2)
        } else {
            (egui::FontId::proportional(FS_XS), p.text_3)
        };
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: font,
                color,
                valign: Align::Center,
                ..Default::default()
            },
        );
    }
    ui.add(egui::Label::new(job).selectable(false));
}

fn separator(ui: &mut Ui) {
    let p = palette(ui.ctx());
    ui.add(
        egui::Label::new(RichText::new("|").size(FS_XS).color(p.border_strong)).selectable(false),
    );
}

/// The status bar (`.statusbar`): numbers of the capture and the note on AI assistants on the left,
/// which give way in narrow windows; version, links and the note on DB-IP on the right, which stays
pub fn status_bar(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let p = palette(&ctx);
    let w = width(&ctx);
    let t = app.t;
    Panel::bottom("status")
        .exact_size(theme::STATUS_H)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p.surface)
                .inner_margin(Margin::symmetric(theme::SP3 as i8, 0)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect();
            let line = Rect::from_min_max(
                pos2(full.left() - theme::SP3, full.top()),
                pos2(full.right() + theme::SP3, full.top() + 1.0),
            );
            ui.painter().rect_filled(line, 0.0, p.border);
            ui.spacing_mut().item_spacing.x = theme::SP3;
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if app.live.status.geo_dbip && status_link(ui, "IP Geolocation by DB-IP").clicked()
                {
                    ui.ctx()
                        .open_url(egui::OpenUrl::new_tab("https://db-ip.com"));
                }
                if status_link(ui, &t.get("licenses")).clicked() {
                    app.go(Page::Settings(SettingsPanel::About));
                }
                if w >= W_LG {
                    if status_link(ui, &t.get("based-on-sniffnet")).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(
                            "https://github.com/GyulyVGC/sniffnet",
                        ));
                    }
                    if status_link(ui, &t.get("source-code")).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(SOURCE_URL));
                    }
                }
                if w >= W_MD {
                    status_text(ui, &[(&format!("{APP_TITLE} {VERSION}"), false)]);
                }
                if let Some(update) = &app.live.status.update {
                    separator(ui);
                    let text = t.get1("update-available", "version", update.version.as_str());
                    let response = update_button(ui, &text, app.overlays.update)
                        .on_hover_text(t.get("update-hint"));
                    if response.clicked() {
                        app.overlays.update = !app.overlays.update;
                    }
                    app.update_button = Some(response.rect);
                }
                // the left side takes what is left and is cut at its end
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                    status_left(app, ui, w);
                });
            });
        });
}

fn status_left(app: &mut App, ui: &mut Ui, w: f32) {
    let t = app.t;
    let p = palette(ui.ctx());
    match &app.live.status.counts {
        None => status_text(ui, &[(&t.get("no-capture"), false)]),
        Some(counts) => {
            let packets = format!("{} ", t.get("status-packets"));
            status_text(ui, &[(&packets, false), (&counts.packets, true)]);
            if w >= W_SM
                && let Some((kept, max, full)) = &counts.store
            {
                separator(ui);
                let store = format!("{} ", t.get("status-store"));
                let of = format!(" {} ", t.get("status-of"));
                let kind = if *full {
                    t.get("status-full")
                } else {
                    t.get("status-headers")
                };
                let kind = format!(", {kind}");
                let mut parts = vec![
                    (store.as_str(), false),
                    (kept.as_str(), true),
                    (of.as_str(), false),
                    (max.as_str(), true),
                ];
                if w >= W_LG {
                    parts.push((kind.as_str(), false));
                }
                status_text(ui, &parts);
            }
            if w >= W_MD {
                separator(ui);
                let dropped = format!("{} ", t.get("status-dropped"));
                status_text(ui, &[(&dropped, false), (&counts.dropped, true)]);
            }
        }
    }
    if w >= W_SM && matches!(app.mcp.status(), crate::mcp::Status::Active { .. }) {
        separator(ui);
        let content = app.mcp.content_allowed();
        let (text, color) = if content {
            (t.get("mcp-open-content"), p.warn)
        } else {
            (t.get("mcp-open"), p.text_2)
        };
        let dot = if content { p.warn } else { p.ok };
        let font = egui::FontId::proportional(FS_XS);
        let width = 7.0 + 6.0 + text_width(ui, &text, font.clone());
        let (rect, response) = ui.allocate_exact_size(vec2(width, 18.0), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Link, true, &text));
        ui.painter()
            .circle_filled(pos2(rect.left() + 3.5, rect.center().y), 3.5, dot);
        let color = if response.hovered() && !content {
            p.text
        } else {
            color
        };
        text_at(ui, rect.left() + 13.0, rect.center().y, &text, font, color);
        if response.on_hover_text(t.get("mcp-open-hint")).clicked() {
            app.go(Page::Settings(SettingsPanel::Assistants));
        }
    }
}

/// The note on a new version in the status bar (`.upd`)
fn update_button(ui: &mut Ui, text: &str, open: bool) -> Response {
    let p = palette(ui.ctx());
    let font = medium_font(FS_XS);
    let width = 6.0 + 14.0 + 4.0 + text_width(ui, text, font.clone()) + 6.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 20.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text));
    if response.hovered() || open {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), p.accent_bg);
    }
    let y = rect.center().y;
    paint_icon(
        ui,
        icons::DOWNLOAD,
        Rect::from_min_size(pos2(rect.left() + 6.0, y - 7.0), vec2(14.0, 14.0)),
        p.accent,
    );
    text_at(ui, rect.left() + 24.0, y, text, font, p.accent);
    focus_ring(ui, &response, theme::RADIUS_SM);
    response
}

/// How long a short message stays (`toast`: 4 s)
const TOAST: Duration = Duration::from_secs(4);

/// The short message at the bottom of the window (`.toast`): 40 px above its lower edge, in the
/// middle, dark with light text
pub fn toast(toast: &mut Option<(String, std::time::Instant)>, ctx: &Context) {
    let Some((text, since)) = toast.as_ref() else {
        return;
    };
    let left = TOAST.saturating_sub(since.elapsed());
    if left.is_zero() {
        *toast = None;
        return;
    }
    ctx.request_repaint_after(left);
    let p = palette(ctx);
    let screen = ctx.content_rect();
    let room = (screen.width() - 32.0).max(120.0);
    let galley = ctx.fonts_mut(|f| {
        f.layout(
            text.clone(),
            egui::FontId::proportional(FS_SM),
            p.surface,
            room - 32.0,
        )
    });
    let size = vec2(galley.size().x + 32.0, galley.size().y + 20.0);
    let rect = Rect::from_min_size(
        pos2(
            screen.center().x - size.x / 2.0,
            screen.bottom() - 40.0 - size.y,
        ),
        size,
    );
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        Id::NULL.with("toast"),
    ));
    painter.add(
        p.shadow_2()
            .as_shape(rect, CornerRadius::same(theme::RADIUS)),
    );
    painter.rect_filled(rect, CornerRadius::same(theme::RADIUS), p.text);
    painter.galley(rect.min + vec2(16.0, 10.0), galley, p.surface);
}

/// The small window on a new version above the status bar (`.updpop`)
pub fn update_note(app: &mut App, ctx: &Context) {
    if !app.overlays.update {
        return;
    }
    let Some(update) = &app.live.status.update else {
        app.overlays.update = false;
        return;
    };
    let t = app.t;
    let p = palette(ctx);
    let screen = ctx.content_rect();
    let width = 380.0_f32.min(screen.width() - 24.0);
    let version = update.version.clone();
    let mut close = ctx.input(|i| i.key_pressed(Key::Escape));
    let mut notes = false;
    let mut release = false;
    let area = egui::Area::new(id("update-note"))
        .order(egui::Order::Foreground)
        .anchor(
            Align2::RIGHT_BOTTOM,
            vec2(-theme::SP3, -(theme::STATUS_H + 6.0)),
        )
        .show(ctx, |ui| {
            Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(CornerRadius::same(theme::RADIUS))
                .shadow(p.shadow_2())
                .inner_margin(Margin {
                    left: 16,
                    right: 16,
                    top: 12,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    ui.set_width(width - 32.0);
                    ui.spacing_mut().item_spacing = vec2(theme::SP2, 0.0);
                    ui.horizontal_top(|ui| {
                        let (icon, _) = ui.allocate_exact_size(vec2(16.0, 18.0), Sense::hover());
                        paint_icon(
                            ui,
                            icons::ARROW_UP_CIRCLE,
                            Rect::from_min_size(icon.min + vec2(0.0, 2.0), vec2(16.0, 16.0)),
                            p.accent,
                        );
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width() - 26.0 - theme::SP2, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(
                                    RichText::new(t.get1("upd-title", "version", version.as_str()))
                                        .font(semibold_font(FS_BASE))
                                        .color(p.text),
                                );
                                let meta = if update.date.is_empty() {
                                    t.get1("upd-installed", "installed", VERSION)
                                } else {
                                    t.get2(
                                        "upd-meta",
                                        "installed",
                                        VERSION,
                                        "date",
                                        update.date.as_str(),
                                    )
                                };
                                ui.label(RichText::new(meta).size(FS_XS).color(p.text_3));
                            },
                        );
                        if widgets::iconbtn(ui, icons::X, &t.get("close"), false, true).clicked() {
                            close = true;
                        }
                    });
                    ui.add_space(theme::SP2);
                    widgets::code_line(ui, &t.get("upd-text-plain"), "SHA256SUMS", FS_SM, p.text);
                    ui.add_space(theme::SP3);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = theme::SP2;
                        release = widgets::icon_text_button(
                            ui,
                            Some(icons::EXTERNAL),
                            &t.get("upd-release-page"),
                            Kind::Primary,
                            true,
                        )
                        .clicked();
                        notes = widgets::icon_text_button(
                            ui,
                            Some(icons::FILE),
                            &t.get("upd-whats-new"),
                            Kind::Secondary,
                            true,
                        )
                        .clicked();
                    });
                });
        });
    // a click elsewhere closes it, except on the note in the status bar, which toggles it
    let outside = ctx.input(|i| i.pointer.any_click())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|at| {
            !area.response.rect.contains(at)
                && !app.update_button.is_some_and(|button| button.contains(at))
        });
    if release {
        ctx.open_url(egui::OpenUrl::new_tab(format!(
            "{SOURCE_URL}/releases/tag/v{version}"
        )));
    }
    if notes {
        app.go(Page::Settings(SettingsPanel::Updates));
        app.pages.settings.show_notes();
    }
    if close || outside || notes || release {
        app.overlays.update = false;
    }
}

/// A dialog in the window (`.modal`): title with an icon, a button to close it, the body and the
/// buttons at its foot, which `foot` adds from the right. Returns whether it should close.
pub fn dialog(
    ctx: &Context,
    name: &str,
    t: Lang,
    title: &str,
    icon: Option<ImageSource<'static>>,
    body: impl FnOnce(&mut Ui),
    foot: impl FnOnce(&mut Ui),
) -> bool {
    let p = palette(ctx);
    let screen = ctx.content_rect();
    let width = 460.0_f32.min(screen.width() - 32.0);
    let area =
        Modal::default_area(id(name)).anchor(Align2::CENTER_TOP, vec2(0.0, screen.height() * 0.14));
    let mut close = false;
    let response = Modal::new(id(name))
        .area(area)
        .backdrop_color(p.scrim)
        .frame(
            Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(CornerRadius::same(8))
                .shadow(p.shadow_2()),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(theme::SP2, 0.0);
            Frame::new()
                .inner_margin(Margin {
                    left: 24,
                    right: 24,
                    top: 16,
                    bottom: 8,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(icon) = icon {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                            paint_icon(ui, icon, rect, p.text);
                        }
                        ui.label(
                            RichText::new(title)
                                .font(semibold_font(FS_LG))
                                .color(p.text),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if widgets::iconbtn(ui, icons::X, &t.get("close"), false, false)
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });
                });
            Frame::new()
                .inner_margin(Margin {
                    left: 24,
                    right: 24,
                    top: 8,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.visuals_mut().override_text_color = Some(p.text_2);
                    ui.spacing_mut().item_spacing.y = theme::SP2;
                    body(ui);
                });
            let (line, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, p.border);
            Frame::new()
                .inner_margin(Margin {
                    left: 24,
                    right: 24,
                    top: 12,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = theme::SP2;
                        foot(ui);
                    });
                });
        });
    close || response.should_close()
}

/// A key as on the keyboard (`kbd`)
fn kbd(ui: &mut Ui, key: &str) {
    let p = palette(ui.ctx());
    let font = mono_font(11.0);
    let galley = ui.painter().layout_no_wrap(key.to_owned(), font, p.text_2);
    let size = vec2(galley.size().x + 10.0, galley.size().y.max(11.0) + 5.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let radius = CornerRadius::same(theme::RADIUS_SM);
    // the lower edge is 2 px, like a key
    ui.painter().rect_filled(rect, radius, p.border_strong);
    ui.painter().rect_filled(
        Rect::from_min_max(rect.min + vec2(1.0, 1.0), rect.max - vec2(1.0, 2.0)),
        CornerRadius::same(theme::RADIUS_SM - 1),
        p.surface,
    );
    ui.painter()
        .galley(pos2(rect.left() + 5.0, rect.top() + 2.0), galley, p.text_2);
}

/// The keys of a shortcut and what it does
type Shortcut = (Vec<String>, String);

/// The help on the keys (`?`)
pub fn help_dialog(app: &mut App, ctx: &Context) {
    if !app.overlays.help {
        return;
    }
    let t = app.t;
    let p = palette(ctx);
    let ctrl = t.get("key-ctrl");
    let end = t.get("key-end");
    let close_text = t.get("close");
    let mut close_button = false;
    let sections: [(String, Vec<Shortcut>); 3] = [
        (
            t.get("keys-everywhere"),
            vec![
                (vec!["/".into()], t.get("keys-slash")),
                (vec!["t".into()], t.get("keys-theme")),
                (vec!["?".into()], t.get("keys-help")),
                (vec!["Alt".into(), "←".into()], t.get("keys-back")),
                (vec!["Esc".into()], t.get("keys-esc")),
            ],
        ),
        (
            t.get("nav-packets"),
            vec![
                (vec![ctrl.clone(), "G".into()], t.get("packets-goto")),
                (vec!["↑".into(), "↓".into()], t.get("keys-select")),
                (vec!["Enter".into()], t.get("keys-open")),
                (vec![end], t.get("keys-end")),
            ],
        ),
        (
            t.get("keys-pages"),
            vec![
                (
                    vec![ctrl.clone(), "1".into(), "…".into(), "7".into()],
                    t.get("keys-pages-text"),
                ),
                (vec![ctrl, ",".into()], t.get("nav-settings")),
            ],
        ),
    ];
    let close = dialog(
        ctx,
        "help-dialog",
        t,
        &t.get("keys-title"),
        None,
        |ui| {
            egui::Grid::new("keys")
                .num_columns(2)
                .spacing(vec2(theme::SP4, theme::SP2))
                .show(ui, |ui| {
                    for (i, (section, rows)) in sections.iter().enumerate() {
                        if i > 0 {
                            ui.add_space(theme::SP2);
                            ui.end_row();
                        }
                        ui.label(
                            RichText::new(section.to_uppercase())
                                .size(FS_XS)
                                .extra_letter_spacing(0.48)
                                .color(p.text_3),
                        );
                        ui.end_row();
                        for (keys, what) in rows {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for key in keys {
                                    if key == "…" {
                                        ui.label(RichText::new("…").size(FS_SM).color(p.text));
                                    } else {
                                        kbd(ui, key);
                                    }
                                }
                            });
                            ui.label(RichText::new(what).size(FS_SM).color(p.text));
                            ui.end_row();
                        }
                    }
                });
        },
        |ui| {
            close_button = widgets::button(ui, close_text.clone(), Kind::Secondary).clicked();
        },
    );
    if close || close_button {
        app.overlays.help = false;
    }
}

/// The question before the capture ends, with what gets lost
pub fn stop_dialog(app: &mut App, ctx: &Context) {
    if !app.overlays.stop {
        return;
    }
    let t = app.t;
    let p = palette(ctx);
    let (packets, connections) = app
        .live
        .status
        .counts
        .as_ref()
        .map(|c| (c.packets.clone(), c.connections.clone()))
        .unwrap_or_default();
    let mut stop = false;
    let mut cancel = false;
    let close = dialog(
        ctx,
        "stop-dialog",
        t,
        &t.get("stop-title"),
        Some(icons::ALERT),
        |ui| {
            ui.label(
                RichText::new(t.get2(
                    "stop-text",
                    "packets",
                    packets.as_str(),
                    "connections",
                    connections.as_str(),
                ))
                .size(FS_BASE),
            );
            ui.label(
                RichText::new(t.get("stop-save-hint"))
                    .size(FS_SM)
                    .color(p.text_3),
            );
        },
        |ui| {
            stop = widgets::icon_text_button(
                ui,
                Some(icons::STOP),
                &t.get("capture-stop"),
                Kind::Danger,
                false,
            )
            .clicked();
            cancel = widgets::button(ui, t.get("gui-close-cancel"), Kind::Secondary).clicked();
        },
    );
    if stop {
        app.overlays.stop = false;
        pages::capture::stop(app);
    } else if close || cancel {
        app.overlays.stop = false;
    }
}

/// The keys that work on every page (`/`, `?`, `t`, Ctrl+1 … 7, Ctrl+,)
pub fn keys(app: &mut App, ctx: &Context) {
    if app.overlays.help || app.overlays.stop || app.closing {
        return;
    }
    let typing = ctx.egui_wants_keyboard_input();
    let (texts, page, settings) = ctx.input(|i| {
        let texts: Vec<String> = if i.modifiers.ctrl || i.modifiers.alt || i.modifiers.command {
            Vec::new()
        } else {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let page = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
        ]
        .iter()
        .position(|key| i.modifiers.command && i.key_pressed(*key));
        (
            texts,
            page,
            i.modifiers.command && i.key_pressed(Key::Comma),
        )
    });
    if let Some(index) = page {
        app.go(Page::of_nav(NAV[index].0));
        return;
    }
    if settings {
        app.go(Page::Settings(SettingsPanel::Display));
        return;
    }
    if typing {
        return;
    }
    for text in texts {
        match text.as_str() {
            "/" => {
                if !matches!(
                    app.page,
                    Page::Packets { .. }
                        | Page::Connections { .. }
                        | Page::Statistics { .. }
                        | Page::Entries(_)
                ) {
                    app.go(Page::Packets {
                        filter: String::new(),
                        connections: String::new(),
                        goto: None,
                    });
                }
                ctx.data_mut(|d| d.insert_temp(id(FOCUS_FILTER), true));
            }
            "?" => app.overlays.help = true,
            "t" => app.toggle_theme(ctx),
            _ => {}
        }
    }
}

/// Whether the page asked its filter field to take the keyboard (key `/`); asks only once
pub fn take_focus_request(ctx: &Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<bool>(id(FOCUS_FILTER)))
        .unwrap_or(false)
}

/// The room of a page inside the frame (`.page`: 24 px around, 16 px below 900 px, 12 px on top in
/// low windows; at most 1600 px wide, in the middle)
/// Space below the last part of a page that scrolls (`.page` padding-bottom)
pub fn page_bottom(ctx: &Context) -> f32 {
    if ctx.content_rect().width() < W_SM {
        theme::SP5
    } else {
        theme::SP6
    }
}

pub fn page_rect(ctx: &Context, panel: Rect) -> Rect {
    let screen = ctx.content_rect();
    let side = if screen.width() < W_SM {
        theme::SP4
    } else {
        theme::SP5.max((panel.width() - 1600.0) / 2.0 + theme::SP5)
    };
    let top = if screen.height() <= 600.0 {
        theme::SP3
    } else if screen.width() < W_SM {
        theme::SP4
    } else {
        theme::SP5
    };
    Rect::from_min_max(
        pos2(panel.left() + side, panel.top() + top),
        pos2(panel.right() - side, panel.bottom() - theme::SP4),
    )
}
