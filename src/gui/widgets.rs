//! The parts every page is made of (`docs/entwurf/assets/app.css`): page heads, cards, tiles,
//! tables, buttons, notes and fact lists.

use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Id, ImageSource, Layout, Margin, Rect, Response,
    RichText, Sense, Stroke, TextWrapMode, Ui, UiBuilder, WidgetInfo, WidgetText,
};

use crate::gui::icons;
use crate::gui::theme::{self, Palette, medium_font, palette, semibold_font};
use crate::view::i18n::Lang;
use egui::emath::GuiRounding as _;

/// Height of the buttons and fields
pub const CONTROL_H: f32 = theme::CTL_H;
/// Height of the small buttons in the heads of the cards
pub const CONTROL_SM_H: f32 = 26.0;
/// Size of the icons in buttons (`.ico.sm`)
pub const ICON_SM: f32 = 14.0;

/// Most data comes at once: only after this time a spinner tells that it is on its way, or it
/// would only flicker
const LOADING_DELAY: Duration = Duration::from_millis(150);

/// The look of a button (`.btn`, `.btn.primary`, `.btn.ghost`, `.btn.danger`)
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Fill, text and border of a button, at rest and under the pointer
fn colors(p: &Palette, kind: Kind, hovered: bool) -> (Color32, Color32, Color32) {
    match (kind, hovered) {
        (Kind::Primary, false) => (p.accent, p.accent_text, p.accent),
        (Kind::Primary, true) => (p.accent_hover, p.accent_text, p.accent_hover),
        (Kind::Secondary, false) => (p.surface, p.text, p.border_strong),
        (Kind::Secondary, true) => (p.hover, p.text, p.border_strong),
        (Kind::Ghost, false) => (Color32::TRANSPARENT, p.text_2, Color32::TRANSPARENT),
        (Kind::Ghost, true) => (p.hover, p.text, Color32::TRANSPARENT),
        (Kind::Danger, false) => (p.surface, p.err, p.border_strong),
        (Kind::Danger, true) => (p.err_bg, p.err, p.err_line),
    }
}

/// The outline of the keyboard focus (`:focus-visible`: 2 px, 2 px off the widget)
pub fn focus_ring(ui: &Ui, response: &Response, radius: u8) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            CornerRadius::same(radius.saturating_add(2)),
            Stroke::new(2.0, palette(ui.ctx()).accent),
            egui::StrokeKind::Outside,
        );
    }
}

/// Paints an icon (white SVG) in a color
pub fn paint_icon(ui: &Ui, icon: ImageSource<'static>, rect: egui::Rect, color: Color32) {
    egui::Image::new(icon).tint(color).paint_at(ui, rect);
}

/// A button: 32 px high (26 px when `small`), with an icon before the text if given
pub fn icon_text_button(
    ui: &mut Ui,
    icon: Option<ImageSource<'static>>,
    text: &str,
    kind: Kind,
    small: bool,
) -> Response {
    let (height, pad, size) = if small {
        (CONTROL_SM_H, theme::SP2, theme::FS_SM)
    } else {
        (CONTROL_H, theme::SP3, theme::FS_BASE)
    };
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), medium_font(size), Color32::PLACEHOLDER);
    let icon_w = if icon.is_some() { ICON_SM + 6.0 } else { 0.0 };
    let width = 2.0 * pad + icon_w + galley.size().x;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, enabled, text));
    if ui.is_rect_visible(rect) {
        let p = palette(ui.ctx());
        let (mut fill, mut fg, mut border) = colors(p, kind, enabled && response.hovered());
        if !enabled {
            fill = fill.gamma_multiply(0.5);
            fg = fg.gamma_multiply(0.5);
            border = border.gamma_multiply(0.5);
        }
        let radius = CornerRadius::same(theme::RADIUS);
        ui.painter().rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let mut x = rect.center().x - f32::midpoint(icon_w, galley.size().x);
        if let Some(icon) = icon {
            let at = egui::Rect::from_min_size(
                egui::pos2(x, rect.center().y - ICON_SM / 2.0),
                egui::vec2(ICON_SM, ICON_SM),
            );
            paint_icon(ui, icon, at, fg);
            x += icon_w;
        }
        let at = egui::pos2(x, rect.center().y - galley.size().y / 2.0)
            .round_to_pixels(ui.ctx().pixels_per_point());
        ui.painter().galley(at, galley, fg);
        focus_ring(ui, &response, theme::RADIUS);
    }
    response
}

/// A button with an icon after the text (`Zur Übersicht →`)
pub fn text_icon_button(
    ui: &mut Ui,
    text: &str,
    icon: ImageSource<'static>,
    kind: Kind,
) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        medium_font(theme::FS_BASE),
        Color32::PLACEHOLDER,
    );
    let icon_w = ICON_SM + 6.0;
    let width = 2.0 * theme::SP3 + icon_w + galley.size().x;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, CONTROL_H), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), text));
    if ui.is_rect_visible(rect) {
        let p = palette(ui.ctx());
        let (fill, fg, border) = colors(p, kind, response.hovered());
        ui.painter().rect(
            rect,
            CornerRadius::same(theme::RADIUS),
            fill,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let x = rect.center().x - f32::midpoint(icon_w, galley.size().x);
        let at = egui::pos2(x, rect.center().y - galley.size().y / 2.0)
            .round_to_pixels(ui.ctx().pixels_per_point());
        let after = x + galley.size().x + 6.0;
        ui.painter().galley(at, galley, fg);
        paint_icon(
            ui,
            icon,
            Rect::from_min_size(
                egui::pos2(after, rect.center().y - ICON_SM / 2.0),
                egui::vec2(ICON_SM, ICON_SM),
            ),
            fg,
        );
        focus_ring(ui, &response, theme::RADIUS);
    }
    response
}

/// A button of the given kind
pub fn button(ui: &mut Ui, text: impl Into<String>, kind: Kind) -> Response {
    icon_text_button(ui, None, &text.into(), kind, false)
}

/// The width of a button with this text
pub fn button_width(ui: &Ui, text: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        medium_font(theme::FS_BASE),
        Color32::PLACEHOLDER,
    );
    galley.size().x + 2.0 * theme::SP3
}

/// A small button
pub fn small_button(ui: &mut Ui, text: impl Into<String>, kind: Kind) -> Response {
    icon_text_button(ui, None, &text.into(), kind, true)
}

/// Buttons in a row, one of them chosen (`.seg`); returns the index of a clicked one
pub fn segmented(ui: &mut Ui, labels: &[String], chosen: usize) -> Option<usize> {
    segmented_sized(ui, labels, chosen, false)
}

/// [`segmented`], 32 px high, or 26 px when `small` (`.seg.sm`)
pub fn segmented_sized(
    ui: &mut Ui,
    labels: &[String],
    chosen: usize,
    small: bool,
) -> Option<usize> {
    segmented_in(ui, labels, &[], chosen, small, 0.0)
}

/// [`segmented`] with an icon before each text (`.seg button` with `.ico.sm`, 6 px apart)
pub fn segmented_icons(
    ui: &mut Ui,
    labels: &[String],
    icons: &[ImageSource<'static>],
    chosen: usize,
) -> Option<usize> {
    segmented_in(ui, labels, icons, chosen, false, 0.0)
}

/// [`segmented`] as wide as `width` (`.seg` in a `.field`): the buttons at the left
pub fn segmented_wide(ui: &mut Ui, labels: &[String], chosen: usize, width: f32) -> Option<usize> {
    segmented_in(ui, labels, &[], chosen, false, width)
}

#[allow(clippy::similar_names)]
fn segmented_in(
    ui: &mut Ui,
    labels: &[String],
    icons: &[ImageSource<'static>],
    chosen: usize,
    small: bool,
    least: f32,
) -> Option<usize> {
    let p = palette(ui.ctx());
    let (height, pad, size) = if small {
        (CONTROL_SM_H, theme::SP2, theme::FS_SM)
    } else {
        (CONTROL_H, theme::SP3, theme::FS_BASE)
    };
    let galleys: Vec<_> = labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap(l.clone(), medium_font(size), Color32::PLACEHOLDER)
        })
        .collect();
    let icon_w = if icons.is_empty() { 0.0 } else { ICON_SM + 6.0 };
    let width: f32 = galleys
        .iter()
        .map(|g| g.size().x + 2.0 * pad + icon_w)
        .sum::<f32>()
        + labels.len().saturating_sub(1) as f32;
    let wide = least > width + 2.0;
    let (outer, whole) =
        ui.allocate_exact_size(egui::vec2((width + 2.0).max(least), height), Sense::hover());
    let radius = CornerRadius::same(theme::RADIUS);
    let inner = theme::RADIUS - 1;
    ui.painter().rect_filled(outer, radius, p.surface);
    let mut clicked = None;
    let mut x = outer.left() + 1.0;
    let last = labels.len().saturating_sub(1);
    for (i, galley) in galleys.into_iter().enumerate() {
        let w = galley.size().x + 2.0 * pad + icon_w;
        let rect = egui::Rect::from_min_size(
            egui::pos2(x, outer.top() + 1.0),
            egui::vec2(w, height - 2.0),
        );
        let id = whole.id.with(("seg", i));
        let response = ui.interact(rect, id, Sense::click());
        let on = i == chosen;
        response
            .widget_info(|| WidgetInfo::selected(egui::Role::Button, true, on, labels[i].as_str()));
        let corners = CornerRadius {
            nw: if i == 0 { inner } else { 0 },
            sw: if i == 0 { inner } else { 0 },
            ne: if i == last && !wide { inner } else { 0 },
            se: if i == last && !wide { inner } else { 0 },
        };
        let (fill, fg) = if on {
            (p.selected, p.text)
        } else if response.hovered() {
            (p.hover, p.text)
        } else {
            (Color32::TRANSPARENT, p.text_2)
        };
        ui.painter().rect_filled(rect, corners, fill);
        let mut left = rect.center().x - f32::midpoint(icon_w, galley.size().x);
        if let Some(icon) = icons.get(i) {
            let at = Rect::from_min_size(
                egui::pos2(left, rect.center().y - ICON_SM / 2.0),
                egui::vec2(ICON_SM, ICON_SM),
            );
            paint_icon(ui, icon.clone(), at, fg);
            left += icon_w;
        }
        let at = egui::pos2(left, rect.center().y - galley.size().y / 2.0)
            .round_to_pixels(ui.ctx().pixels_per_point());
        ui.painter().galley(at, galley, fg);
        if i < last {
            let line = egui::Rect::from_min_max(
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right() + 1.0, rect.bottom()),
            );
            ui.painter().rect_filled(line, 0.0, p.border);
        }
        focus_ring(ui, &response, theme::RADIUS_SM);
        if response.clicked() {
            clicked = Some(i);
        }
        x += w + 1.0;
    }
    ui.painter().rect_stroke(
        outer,
        radius,
        Stroke::new(1.0, p.border_strong),
        egui::StrokeKind::Inside,
    );
    clicked
}

/// A part of [`seg_nav`]: an icon before or after the text, and whether it can be used
pub struct SegItem<'a> {
    pub before: Option<ImageSource<'static>>,
    pub text: &'a str,
    pub after: Option<ImageSource<'static>>,
    pub enabled: bool,
}

/// Buttons in a row without a chosen one (`.seg` of links, like "‹ Previous | Next ›"), 32 px
/// high; one that cannot be used is drawn faint. Returns the index of a clicked one.
pub fn seg_nav(ui: &mut Ui, items: &[SegItem]) -> Option<usize> {
    let p = palette(ui.ctx());
    let pad = theme::SP3;
    let gap = 6.0;
    let galleys: Vec<_> = items
        .iter()
        .map(|item| {
            ui.painter().layout_no_wrap(
                item.text.to_owned(),
                medium_font(theme::FS_BASE),
                Color32::PLACEHOLDER,
            )
        })
        .collect();
    let widths: Vec<f32> = items
        .iter()
        .zip(&galleys)
        .map(|(item, galley)| {
            let icons = [item.before.is_some(), item.after.is_some()]
                .iter()
                .filter(|on| **on)
                .count() as f32;
            2.0 * pad + galley.size().x + icons * (ICON_SM + gap)
        })
        .collect();
    let width: f32 = widths.iter().sum::<f32>() + items.len().saturating_sub(1) as f32;
    let (outer, whole) = ui.allocate_exact_size(egui::vec2(width + 2.0, CONTROL_H), Sense::hover());
    let radius = CornerRadius::same(theme::RADIUS);
    let inner = theme::RADIUS - 1;
    ui.painter().rect_filled(outer, radius, p.surface);
    let mut clicked = None;
    let mut x = outer.left() + 1.0;
    let last = items.len().saturating_sub(1);
    for (i, ((item, galley), w)) in items.iter().zip(galleys).zip(widths).enumerate() {
        let rect = egui::Rect::from_min_size(
            egui::pos2(x, outer.top() + 1.0),
            egui::vec2(w, CONTROL_H - 2.0),
        );
        let sense = if item.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, whole.id.with(("seg", i)), sense);
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, item.enabled, item.text));
        let corners = CornerRadius {
            nw: if i == 0 { inner } else { 0 },
            sw: if i == 0 { inner } else { 0 },
            ne: if i == last { inner } else { 0 },
            se: if i == last { inner } else { 0 },
        };
        let mut fg = p.text_2;
        if item.enabled && response.hovered() {
            ui.painter().rect_filled(rect, corners, p.hover);
            fg = p.text;
        }
        if !item.enabled {
            fg = fg.gamma_multiply(0.5);
        }
        let mut at = rect.left() + pad;
        let icon_at = |x: f32| {
            egui::Rect::from_min_size(
                egui::pos2(x, rect.center().y - ICON_SM / 2.0),
                egui::vec2(ICON_SM, ICON_SM),
            )
        };
        if let Some(icon) = item.before.clone() {
            paint_icon(ui, icon, icon_at(at), fg);
            at += ICON_SM + gap;
        }
        let text_w = galley.size().x;
        let pos = egui::pos2(at, rect.center().y - galley.size().y / 2.0)
            .round_to_pixels(ui.ctx().pixels_per_point());
        ui.painter().galley(pos, galley, fg);
        at += text_w + gap;
        if let Some(icon) = item.after.clone() {
            paint_icon(ui, icon, icon_at(at), fg);
        }
        if i < last {
            let line = egui::Rect::from_min_max(
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right() + 1.0, rect.bottom()),
            );
            ui.painter().rect_filled(line, 0.0, p.border);
        }
        focus_ring(ui, &response, theme::RADIUS_SM);
        if item.enabled && response.clicked() {
            clicked = Some(i);
        }
        x += w + 1.0;
    }
    ui.painter().rect_stroke(
        outer,
        radius,
        Stroke::new(1.0, p.border_strong),
        egui::StrokeKind::Inside,
    );
    clicked
}

/// A clickable icon (`.iconbtn`, 32 px, `small`: 26 px); `current` marks the page it opens
#[allow(clippy::similar_names)]
pub fn iconbtn(
    ui: &mut Ui,
    icon: ImageSource<'static>,
    text: &str,
    current: bool,
    small: bool,
) -> Response {
    let side = if small { CONTROL_SM_H } else { CONTROL_H };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, current, text));
    if ui.is_rect_visible(rect) {
        let p = palette(ui.ctx());
        let radius = CornerRadius::same(theme::RADIUS);
        let color = if current {
            ui.painter().rect_filled(rect, radius, p.accent_bg);
            p.accent
        } else if response.hovered() {
            ui.painter().rect_filled(rect, radius, p.hover);
            p.text
        } else {
            p.text_2
        };
        let size = if small { ICON_SM } else { 16.0 };
        paint_icon(
            ui,
            icon,
            egui::Rect::from_center_size(rect.center(), egui::vec2(size, size)),
            color,
        );
        focus_ring(ui, &response, theme::RADIUS);
    }
    response.on_hover_text(text)
}

/// A clickable icon without a frame (`.back`, `.star`); the text says what it does, for the
/// tooltip and screen readers
pub fn icon_button(
    ui: &mut Ui,
    source: ImageSource<'static>,
    color: Color32,
    text: &str,
) -> Response {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text));
    let p = palette(ui.ctx());
    let color = if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), p.hover);
        if color == p.text_3 { p.text } else { color }
    } else {
        color
    };
    paint_icon(
        ui,
        source,
        egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0)),
        color,
    );
    focus_ring(ui, &response, theme::RADIUS_SM);
    response.on_hover_text(text)
}

/// A link inside the window
pub fn link(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(egui::Link::new(text))
}

/// A link in a size and color of its own (`.crumb a`, `.small` links)
pub fn link_sized(ui: &mut Ui, text: &str, size: f32, color: Color32) -> Response {
    let response = ui.add(
        egui::Label::new(RichText::new(text).size(size).color(color))
            .sense(Sense::click())
            .selectable(false),
    );
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.hovered() {
        let rect = response.rect;
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(1.0, color));
    }
    response
}

/// Text in 13 px (`.small`)
pub fn small(text: impl Into<String>) -> RichText {
    RichText::new(text).size(theme::FS_SM)
}

/// The id of a widget that appears once in the window
pub fn once(salt: &str) -> Id {
    Id::NULL.with(("widgets", salt))
}

/// The window is at most 600 px high: page heads leave out their text
pub fn short_window(ctx: &egui::Context) -> bool {
    ctx.content_rect().height() <= 600.0
}

/// What was clicked in a page head
#[derive(Default)]
pub struct HeadClicks {
    pub back: bool,
    /// Index of a crumb
    pub crumb: Option<usize>,
}

/// The head of a page (`.pagehead`): the crumbs, the title (with the way back on detail pages) and
/// the text below it on the left, the actions at the right, level with the last line
pub fn page_head(
    ui: &mut Ui,
    crumbs: &[String],
    back: Option<&str>,
    title: impl FnOnce(&mut Ui),
    sub: impl FnOnce(&mut Ui),
    actions: impl FnOnce(&mut Ui),
) -> HeadClicks {
    let p = palette(ui.ctx());
    let mut clicks = HeadClicks::default();
    let full = ui.available_rect_before_wrap();
    let actions_id = once("head-actions");
    let actions_w: f32 = ui.data(|d| d.get_temp(actions_id)).unwrap_or(0.0);
    let gap = if actions_w > 0.0 { theme::SP4 } else { 0.0 };
    let left_w = (full.width() - actions_w - gap).max(160.0);
    let mut y = full.top();
    // rows get their height, so that centering stays in the row; the text below grows downwards
    let child = |ui: &mut Ui, y: f32, height: f32, layout: Layout| {
        ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(full.left(), y),
                    egui::vec2(left_w, height),
                ))
                .layout(layout),
        )
    };
    if !crumbs.is_empty() {
        let mut row = child(
            ui,
            y,
            theme::FS_SM * 1.45,
            Layout::left_to_right(Align::Center),
        );
        row.spacing_mut().item_spacing.x = theme::SP1;
        let last = crumbs.len() - 1;
        for (i, crumb) in crumbs.iter().enumerate() {
            if i < last {
                if link_sized(&mut row, crumb, theme::FS_SM, p.text_2).clicked() {
                    clicks.crumb = Some(i);
                }
                row.label(small("›").color(p.text_3));
            } else {
                row.label(small(crumb).color(p.text_3));
            }
        }
        y = row.min_rect().bottom() + theme::SP1;
    }
    let mut row = child(ui, y, 28.0, Layout::left_to_right(Align::Center));
    row.spacing_mut().item_spacing.x = theme::SP2;
    if let Some(hint) = back {
        row.add_space(-6.0);
        clicks.back = icon_button(&mut row, icons::BACK, p.text_3, hint).clicked();
    }
    title(&mut row);
    y = row.min_rect().bottom();
    if !short_window(ui.ctx()) {
        let mut below = child(
            ui,
            y + theme::SP1,
            f32::INFINITY,
            Layout::top_down(Align::Min),
        );
        below.style_mut().wrap_mode = Some(TextWrapMode::Wrap);
        sub(&mut below);
        if below.min_rect().height() > 0.0 {
            y = below.min_rect().bottom();
        }
    }
    let area = Rect::from_min_max(full.min, egui::pos2(full.right(), y));
    let mut right = ui.new_child(
        UiBuilder::new()
            .max_rect(area)
            .layout(Layout::right_to_left(Align::Max)),
    );
    right.spacing_mut().item_spacing.x = theme::SP2;
    actions(&mut right);
    let used = right.min_rect();
    let width = if used.width() > 0.0 {
        used.width()
    } else {
        0.0
    };
    if (width - actions_w).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(actions_id, width));
        ui.ctx().request_repaint();
    }
    let bottom = y.max(used.bottom());
    ui.advance_cursor_after_rect(Rect::from_min_max(
        full.min,
        egui::pos2(full.right(), bottom),
    ));
    clicks
}

/// The title of a page (`h1`)
pub fn title(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).font(semibold_font(theme::FS_XL))).truncate());
}

/// The text below the title (`.pagehead p`)
pub fn subtitle(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.label(
        RichText::new(text)
            .color(p.text_3)
            .line_height(Some(LH_BASE)),
    );
}

/// A text with one word in the font of code, wrapped (`<span class="mono">` in a line)
pub fn code_line(ui: &mut Ui, text: &str, code: &str, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::default();
    let plain = egui::TextFormat {
        font_id: egui::FontId::proportional(size),
        color,
        line_height: Some(size * 1.45),
        ..Default::default()
    };
    let mono = egui::TextFormat {
        font_id: theme::mono_font(size * 0.92),
        color,
        line_height: Some(size * 1.45),
        ..Default::default()
    };
    match text.split_once(code).filter(|_| !code.is_empty()) {
        Some((before, after)) => {
            job.append(before, 0.0, plain.clone());
            job.append(code, 0.0, mono);
            job.append(after, 0.0, plain);
        }
        None => job.append(text, 0.0, plain),
    }
    job.wrap.max_width = ui.available_width();
    ui.add(egui::Label::new(job).wrap());
}

/// Line height of the text in 14 px (`--lh: 1.45`)
pub const LH_BASE: f32 = theme::FS_BASE * 1.45;

fn card_frame(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(theme::RADIUS))
}

/// A horizontal line across the width of `ui`
pub fn rule(ui: &mut Ui) {
    let p = palette(ui.ctx());
    let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(line, 0.0, p.border);
}

/// The head of a card (`.card > header`): icon and title, then what `tools` adds at the right
fn card_head(
    ui: &mut Ui,
    icon: Option<ImageSource<'static>>,
    title: &str,
    row_h: f32,
    tools: impl FnOnce(&mut Ui),
) {
    let p = palette(ui.ctx());
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
        .show(ui, |ui| {
            let row = egui::vec2(ui.available_width(), row_h);
            ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                if let Some(icon) = icon {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                    paint_icon(ui, icon, rect, p.text);
                }
                ui.add(
                    egui::Label::new(RichText::new(title).font(semibold_font(theme::FS_MD)))
                        .selectable(false)
                        .truncate(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = theme::SP3;
                    tools(ui);
                });
            });
        });
    rule(ui);
}

/// A card whose head has other things than a title at the left (`.card > header` with the
/// legend of a stream): `head` from the left, what `tools` adds at the right; the body reaches
/// the edges
pub fn custom_card<R>(
    ui: &mut Ui,
    head: impl FnOnce(&mut Ui),
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    let p = palette(ui.ctx());
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            Frame::new()
                .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
                .show(ui, |ui| {
                    let row = egui::vec2(ui.available_width(), 24.0);
                    ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                        head(ui);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = theme::SP3;
                            tools(ui);
                        });
                    });
                });
            rule(ui);
            body(ui)
        })
        .inner
}

/// A card for facts (`.cbody.tight`: 8 px above and below, 16 px at the sides)
pub fn card_tight<R>(
    ui: &mut Ui,
    title: &str,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    icon_card(ui, None, title, tools, |ui| tight(ui, body))
}

/// The body of a card for facts (`.cbody.tight`)
pub fn tight<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP2 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui)
        })
        .inner
}

/// A card whose body reaches its edges, for tables and lists
pub fn flush_card<R>(
    ui: &mut Ui,
    title: &str,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    icon_card(ui, None, title, tools, body)
}

/// [`flush_card`] with an icon before the title
pub fn icon_card<R>(
    ui: &mut Ui,
    icon: Option<ImageSource<'static>>,
    title: &str,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    let p = palette(ui.ctx());
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            card_head(ui, icon, title, 24.0, tools);
            body(ui)
        })
        .inner
}

/// [`flush_card`] whose head holds a higher control (a `.seg` of 32 px, a `.seg.sm` of 26 px):
/// the row of the head is `row_h` high
pub fn tall_card<R>(
    ui: &mut Ui,
    title: &str,
    row_h: f32,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    let p = palette(ui.ctx());
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            card_head(ui, None, title, row_h, tools);
            body(ui)
        })
        .inner
}

/// A card without a head (`.card` around a toolbar, a table or an empty page)
pub fn bare_card<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    let p = palette(ui.ctx());
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            body(ui)
        })
        .inner
}

/// The foot of a card (`.cfoot`): a line, then the buttons
pub fn card_foot(ui: &mut Ui, right: bool, add: impl FnOnce(&mut Ui)) {
    rule(ui);
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let layout = if right {
                Layout::right_to_left(Align::Center)
            } else {
                Layout::left_to_right(Align::Center)
            };
            let row = egui::vec2(ui.available_width(), CONTROL_SM_H);
            ui.allocate_ui_with_layout(row, layout, |ui| {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                add(ui);
            });
        });
}

/// A row of controls in a card (`.toolbar`, 8 px above and below, 12 px at the sides, a line below)
pub fn toolbar(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP3 as i8, theme::SP2 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // a row is as high as its highest control (the small ones: 28 px)
            let interact = ui.spacing().interact_size.y;
            ui.spacing_mut().interact_size.y = 28.0;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::SP2, theme::SP2);
                ui.spacing_mut().interact_size.y = interact;
                add(ui);
            });
        });
    rule(ui);
}

/// The note in a toolbar at its right end (`.toolbar .sum`)
pub fn toolbar_sum(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.label(small(text).color(p.text_3));
    });
}

/// A tab below the head of a page (`.ptabs a`): icon, name and a count after it
pub struct PTab {
    pub icon: ImageSource<'static>,
    pub label: String,
    pub count: Option<String>,
}

/// The tabs below the head of a page (`.ptabs`: 7 px above and below the name, a line of the
/// accent color below the current one); returns the tab clicked
pub fn ptabs(ui: &mut Ui, tabs: &[PTab], current: usize) -> Option<usize> {
    let p = palette(ui.ctx());
    ui.add_space(-theme::SP1);
    let height = 7.0 + LH_BASE + 7.0 + 2.0;
    let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    ui.painter().hline(
        row.x_range(),
        row.bottom() - 0.5,
        Stroke::new(1.0, p.border),
    );
    let ppp = ui.ctx().pixels_per_point();
    let mut x = row.left();
    let mut clicked = None;
    for (i, tab) in tabs.iter().enumerate() {
        let on = i == current;
        let label = ui.painter().layout_no_wrap(
            tab.label.clone(),
            medium_font(theme::FS_BASE),
            Color32::PLACEHOLDER,
        );
        let count = tab.count.as_ref().map(|c| {
            ui.painter()
                .layout_no_wrap(c.clone(), theme::mono_font(theme::FS_XS), p.text_2)
        });
        let count_w = count.as_ref().map_or(0.0, |g| g.size().x + 14.0);
        let w = 12.0
            + ICON_SM
            + 6.0
            + label.size().x
            + if count.is_some() { 6.0 + count_w } else { 0.0 }
            + 12.0;
        let rect = Rect::from_min_size(egui::pos2(x, row.top()), egui::vec2(w, height - 1.0));
        let response = ui.interact(rect, Id::NULL.with(("ptab", &tab.label)), Sense::click());
        response.widget_info(|| WidgetInfo::selected(egui::Role::Tab, true, on, &tab.label));
        let hovered = response.hovered();
        if hovered {
            ui.painter().rect_filled(rect, 0.0, p.hover);
        }
        let color = if on || hovered { p.text } else { p.text_2 };
        let mid = rect.top() + 7.0 + LH_BASE / 2.0;
        let mut cx = rect.left() + 12.0;
        paint_icon(
            ui,
            tab.icon.clone(),
            Rect::from_center_size(
                egui::pos2(cx + ICON_SM / 2.0, mid),
                egui::vec2(ICON_SM, ICON_SM),
            ),
            p.text_3,
        );
        cx += ICON_SM + 6.0;
        let at = egui::pos2(cx, mid - label.size().y / 2.0).round_to_pixels(ppp);
        ui.painter().galley(at, label.clone(), color);
        cx += label.size().x + 6.0;
        if let Some(count) = count {
            let pill = Rect::from_min_size(
                egui::pos2(cx, mid - 9.0),
                egui::vec2(count.size().x + 14.0, 18.0),
            );
            ui.painter().rect(
                pill,
                CornerRadius::same(9),
                p.surface_2,
                Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );
            let at = (pill.center() - count.size() / 2.0).round_to_pixels(ppp);
            ui.painter().galley(at, count, p.text_2);
        }
        if on {
            ui.painter().rect_filled(
                Rect::from_min_max(
                    egui::pos2(rect.left(), row.bottom() - 2.0),
                    egui::pos2(rect.right(), row.bottom()),
                ),
                0.0,
                p.accent,
            );
        }
        if response.clicked() && !on {
            clicked = Some(i);
        }
        if !on {
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        x += w;
    }
    clicked
}

/// A value tile (`.tile`)
#[derive(Clone, Default)]
pub struct Tile {
    /// The icon before the label and its color (`None`: the color of the label)
    pub icon: Option<(ImageSource<'static>, Option<Color32>)>,
    pub label: String,
    /// The number, and its unit in smaller letters
    pub value: String,
    pub unit: String,
    pub sub: String,
    pub warn: bool,
    /// The tile opens a page
    pub link: bool,
}

impl Tile {
    /// A tile whose value is an amount like `42,2 MB`: the unit after the last space is set
    /// smaller
    pub fn amount(label: String, amount: &str, sub: String) -> Self {
        let (value, unit) = amount
            .rsplit_once(' ')
            .map_or((amount, ""), |(v, u)| (v, u));
        Self {
            label,
            value: value.to_owned(),
            unit: unit.to_owned(),
            sub,
            ..Self::default()
        }
    }

    pub fn icon(mut self, icon: ImageSource<'static>, color: Option<Color32>) -> Self {
        self.icon = Some((icon, color));
        self
    }
}

/// Columns of the tiles: six, three below 1500 px, two below 900 px (`.tiles`); five, three below
/// 1024 px, two below 900 px with `five` (`.tiles.t5`)
fn tile_columns(ctx: &egui::Context, five: bool) -> usize {
    let width = ctx.content_rect().width();
    if width < 900.0 {
        2
    } else if five {
        if width < 1024.0 { 3 } else { 5 }
    } else if width < 1500.0 {
        3
    } else {
        6
    }
}

/// Tiles in a grid; returns the index of a tile clicked
pub fn tiles(ui: &mut Ui, items: &[Tile], five: bool) -> Option<usize> {
    let ctx = ui.ctx().clone();
    let narrow = ctx.content_rect().width() < 900.0;
    let columns = tile_columns(&ctx, five);
    let gap = if narrow { theme::SP2 } else { theme::SP3 };
    let width = ui.available_width();
    let w = (width - gap * (columns as f32 - 1.0)) / columns as f32;
    let mut clicked = None;
    for (r, row) in items.chunks(columns).enumerate() {
        let top = ui.cursor().top();
        let mut bottom = top;
        for (c, tile) in row.iter().enumerate() {
            let rect = Rect::from_min_size(
                egui::pos2(ui.cursor().left() + c as f32 * (w + gap), top),
                egui::vec2(w, 0.0),
            );
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(Rect::from_min_size(rect.min, egui::vec2(w, f32::INFINITY)))
                    .layout(Layout::top_down(Align::Min)),
            );
            let used = paint_tile(&mut child, tile, narrow);
            if tile.link && used.clicked() {
                clicked = Some(r * columns + c);
            }
            bottom = bottom.max(used.rect.bottom());
        }
        ui.advance_cursor_after_rect(Rect::from_min_max(
            egui::pos2(ui.cursor().left(), top),
            egui::pos2(ui.cursor().left() + width, bottom),
        ));
        if (r + 1) * columns < items.len() {
            ui.add_space(gap - ui.spacing().item_spacing.y);
        }
    }
    clicked
}

fn paint_tile(ui: &mut Ui, tile: &Tile, narrow: bool) -> Response {
    let p = palette(ui.ctx());
    let id = ui.unique_id().with("tile");
    let hovered = tile.link && ui.ctx().read_response(id).is_some_and(|r| r.hovered());
    let (fg_k, fg_v) = if tile.warn {
        (p.warn, p.warn)
    } else {
        (p.text_2, p.text)
    };
    let inner = Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(
            1.0,
            if hovered { p.border_strong } else { p.border },
        ))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .shadow(if hovered {
            p.shadow_1()
        } else {
            egui::Shadow::NONE
        })
        .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            let row = egui::vec2(ui.available_width(), theme::FS_SM * 1.45);
            ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                if let Some((icon, color)) = &tile.icon {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(ICON_SM, ICON_SM), Sense::hover());
                    paint_icon(ui, icon.clone(), rect, color.unwrap_or(fg_k));
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(&tile.label)
                            .font(medium_font(theme::FS_SM))
                            .color(fg_k),
                    )
                    .selectable(false)
                    .truncate(),
                );
            });
            ui.add_space(theme::SP1);
            let size = if narrow { theme::FS_XL } else { theme::FS_2XL };
            let mut job = egui::text::LayoutJob::default();
            let format = |font: egui::FontId, color: Color32| egui::TextFormat {
                font_id: font,
                color,
                line_height: Some(size * 1.2),
                valign: Align::BOTTOM,
                ..Default::default()
            };
            job.append(
                &tile.value,
                0.0,
                format(theme::mono_semibold_font(size), fg_v),
            );
            if !tile.unit.is_empty() {
                job.append(" ", 0.0, format(theme::mono_semibold_font(size), fg_v));
                job.append(
                    &tile.unit,
                    0.0,
                    format(medium_font(theme::FS_BASE), p.text_2),
                );
            }
            job.wrap.max_rows = 1;
            ui.add(egui::Label::new(job).selectable(false).truncate());
            ui.add(
                egui::Label::new(
                    RichText::new(if tile.sub.is_empty() { " " } else { &tile.sub })
                        .size(theme::FS_XS)
                        .color(p.text_3)
                        .line_height(Some(theme::FS_XS * 1.45)),
                )
                .selectable(false)
                .truncate(),
            );
        });
    let sense = if tile.link {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(inner.response.rect, id, sense);
    if tile.link {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// The color of a note (`.notice.warn`, `.err`, `.info`; `ok` for a confirmation)
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Warn,
    Err,
    Info,
    Ok,
}

/// A note (`.notice`): icon, then what `add` adds, in 13 px
pub fn notice_of(ui: &mut Ui, tone: Tone, add: impl FnOnce(&mut Ui)) {
    notice_icon(ui, tone, None, add);
}

/// [`notice_of`] with another icon than that of its tone
pub fn notice_icon(
    ui: &mut Ui,
    tone: Tone,
    other: Option<ImageSource<'static>>,
    add: impl FnOnce(&mut Ui),
) {
    let p = palette(ui.ctx());
    let (fill, line, color, icon) = match tone {
        Tone::Warn => (p.warn_bg, p.warn_line, p.warn, icons::ALERT),
        Tone::Err => (p.err_bg, p.err_line, p.err, icons::ALERT),
        Tone::Info => (p.accent_bg, Color32::TRANSPARENT, p.accent, icons::INFO),
        Tone::Ok => (p.ok_bg, p.ok_line, p.ok, icons::CHECK_CIRCLE),
    };
    let icon = other.unwrap_or(icon);
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, line))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 19.0), Sense::hover());
                paint_icon(
                    ui,
                    icon,
                    Rect::from_min_size(rect.min + egui::vec2(0.0, 2.0), egui::vec2(16.0, 16.0)),
                    color,
                );
                // the rows of the text are as high as its lines (`--lh` of 13 px)
                ui.spacing_mut().interact_size.y = theme::FS_SM * 1.45;
                ui.horizontal_wrapped(|ui| {
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Body,
                        egui::FontId::proportional(theme::FS_SM),
                    );
                    ui.spacing_mut().item_spacing.x = theme::SP1;
                    add(ui);
                });
            });
        });
}

/// A note above the content of a page; `add` may add links
pub fn notice(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    notice_of(ui, Tone::Warn, add);
}

/// A note with a plain text
pub fn notice_text(ui: &mut Ui, text: &str) {
    notice(ui, |ui| {
        ui.label(text);
    });
}

/// What went wrong
pub fn error(ui: &mut Ui, text: &str) {
    notice_of(ui, Tone::Err, |ui| {
        ui.label(text);
    });
}

/// A small explanation (`p.muted.small`, `.help`)
pub fn hint(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    // columns are justified, which would spread the words of a wrapped text
    ui.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.label(
            small(text)
                .color(p.text_3)
                .line_height(Some(theme::FS_SM * 1.45)),
        );
    });
}

/// A link in the font of addresses in a text of 13 px (`a.mono`: `.92em`)
pub fn mono_link(ui: &mut Ui, text: &str) -> Response {
    mono_link_sized(ui, text, theme::FS_SM * 0.92)
}

/// A link in the font of addresses
pub fn mono_link_sized(ui: &mut Ui, text: &str, size: f32) -> Response {
    let p = palette(ui.ctx());
    let response = ui.add(
        egui::Label::new(
            RichText::new(text)
                .font(theme::mono_font(size))
                .color(p.accent),
        )
        .sense(Sense::click()),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The dot between the links of a line (` · `)
pub fn dot(ui: &mut Ui) {
    ui.label("·");
}

/// A short note at the right of the head of a card (`.hint`)
pub fn card_note(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.add(egui::Label::new(small(text).color(p.text_3)).truncate());
}

/// Stands for data on its way, which has been awaited for `waited`
pub fn loading(ui: &mut Ui, waited: Duration, text: &str) {
    if let Some(left) = LOADING_DELAY.checked_sub(waited)
        && !left.is_zero()
    {
        ui.ctx().request_repaint_after(left);
        return;
    }
    let p = palette(ui.ctx());
    ui.add_space(32.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Spinner::new().size(24.0).color(p.accent));
        ui.add_space(4.0);
        ui.label(RichText::new(text).color(p.text_3));
    });
    ui.add_space(32.0);
}

/// An empty page or list (`.empty`): a large icon, a heading, a text and a button, all centered;
/// returns whether the button was clicked
pub fn empty_state(
    ui: &mut Ui,
    icon: Option<ImageSource<'static>>,
    heading: &str,
    text: &str,
    button: Option<(ImageSource<'static>, &str)>,
) -> bool {
    let p = palette(ui.ctx());
    let mut clicked = false;
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP6 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = theme::SP2;
                if let Some(icon) = icon {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
                    paint_icon(ui, icon, rect, p.text_3);
                }
                ui.label(
                    RichText::new(heading)
                        .font(semibold_font(theme::FS_MD))
                        .color(p.text),
                );
                if !text.is_empty() {
                    ui.label(small(text).color(p.text_2));
                }
                if let Some((icon, label)) = button {
                    clicked =
                        icon_text_button(ui, Some(icon), label, Kind::Primary, false).clicked();
                }
            });
        });
    clicked
}

/// A centered heading for a list without content (`.empty h2`)
pub fn empty(ui: &mut Ui, text: &str) {
    empty_state(ui, None, text, "", None);
}

/// The card of a page while no capture runs; returns whether "Start capture" was clicked
pub fn no_capture(ui: &mut Ui, t: Lang) -> bool {
    bare_card(ui, |ui| {
        empty_state(
            ui,
            Some(icons::CAPTURE_XL),
            &t.get("no-capture"),
            &t.get("no-capture-text"),
            Some((icons::PLAY, &t.get("no-capture-start"))),
        )
    })
}

/// A label next to a name (`.tag`); `cc` for a country code (`.tag.cc`)
pub fn tag(ui: &mut Ui, text: &str) -> Response {
    tag_sized(ui, text, false)
}

/// A country code (`.tag.cc`)
pub fn tag_cc(ui: &mut Ui, text: &str) -> Response {
    tag_sized(ui, text, true)
}

fn tag_sized(ui: &mut Ui, text: &str, cc: bool) -> Response {
    let p = palette(ui.ctx());
    let font = if cc {
        theme::mono_font(11.0)
    } else {
        egui::FontId::proportional(theme::FS_XS)
    };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, p.text_2);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(galley.size().x + 12.0, 20.0), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS_SM),
        p.surface_2,
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    let at = (rect.center() - galley.size() / 2.0).round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, p.text_2);
    response
}

/// A small label in the accent color (`.pill`)
pub fn pill(ui: &mut Ui, text: &str) -> Response {
    let p = palette(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), medium_font(theme::FS_XS), p.accent);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(galley.size().x + 12.0, 20.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), p.accent_bg);
    let at = (rect.center() - galley.size() / 2.0).round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, p.accent);
    response
}

/// A state (`.st`): `ok` without a frame, the others on their pale color
pub fn state(ui: &mut Ui, tone: Tone, icon: Option<ImageSource<'static>>, text: &str) -> Response {
    let p = palette(ui.ctx());
    let (fg, fill, line) = match tone {
        Tone::Ok => (p.ok, Color32::TRANSPARENT, Color32::TRANSPARENT),
        Tone::Warn => (p.warn, p.warn_bg, p.warn_line),
        Tone::Err => (p.err, p.err_bg, p.err_line),
        Tone::Info => (p.off, p.off_bg, p.off_line),
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), medium_font(theme::FS_XS), fg);
    let left = if tone == Tone::Ok { 0.0 } else { 6.0 };
    let icon_w = if icon.is_some() { ICON_SM + 4.0 } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(left + icon_w + galley.size().x + 8.0, 22.0),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, text));
    ui.painter().rect(
        rect,
        CornerRadius::same(11),
        fill,
        Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
    let mut x = rect.left() + left;
    if let Some(icon) = icon {
        paint_icon(
            ui,
            icon,
            Rect::from_center_size(
                egui::pos2(x + ICON_SM / 2.0, rect.center().y),
                egui::vec2(ICON_SM, ICON_SM),
            ),
            fg,
        );
        x += icon_w;
    }
    let at = egui::pos2(x, rect.center().y - galley.size().y / 2.0)
        .round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, fg);
    response
}

/// A list of labels with their values (`dl.facts`): the labels in a column as wide as the
/// longest, a line below each row but the last
pub fn facts(ui: &mut Ui, labels: &[String], mut value: impl FnMut(&mut Ui, usize)) {
    let p = palette(ui.ctx());
    let label_w = labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap(
                    l.clone(),
                    egui::FontId::proportional(theme::FS_SM),
                    p.text_3,
                )
                .size()
                .x
        })
        .fold(0.0, f32::max);
    let width = ui.available_width();
    let last = labels.len().saturating_sub(1);
    let spacing = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    for (i, label) in labels.iter().enumerate() {
        let top = ui.cursor().top();
        let left = ui.cursor().left();
        let mut l = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
            egui::pos2(left, top + 8.0),
            egui::vec2(label_w + 1.0, 30.0),
        )));
        l.label(small(label).color(p.text_3));
        let value_x = left + label_w + theme::SP5;
        let mut v = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(value_x, top + 7.0),
                    egui::vec2(width - (value_x - left), LH_BASE),
                ))
                .layout(Layout::left_to_right(Align::Center).with_main_wrap(true)),
        );
        v.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
        value(&mut v, i);
        // 7 px, a line of text or the highest control, 7 px and the line below (`.facts dd`)
        let content = v.min_rect().height().max(LH_BASE);
        let bottom = top + 7.0 + content + 7.0 + if i < last { 1.0 } else { 0.0 };
        let row = Rect::from_min_max(egui::pos2(left, top), egui::pos2(left + width, bottom));
        if i < last {
            ui.painter().hline(
                row.x_range(),
                row.bottom().round() - 0.5,
                Stroke::new(1.0, p.border),
            );
        }
        ui.advance_cursor_after_rect(row);
    }
    ui.spacing_mut().item_spacing.y = spacing;
}

/// A text field (`input[type=text]`): `height` high, 10 px inside, the border of the accent
/// color and a ring of 3 px while it has the focus. `mono` writes in the font of addresses.
pub fn input(
    ui: &mut Ui,
    text: &mut String,
    id: Id,
    size: egui::Vec2,
    mono: bool,
    hint: &str,
) -> egui::text_edit::TextEditOutput {
    input_on(ui, text, id, size, mono, hint, true)
}

/// [`input`] that can be switched off (`input:disabled`): it looks the same, but takes no input
pub fn input_on(
    ui: &mut Ui,
    text: &mut String,
    id: Id,
    size: egui::Vec2,
    mono: bool,
    hint: &str,
    enabled: bool,
) -> egui::text_edit::TextEditOutput {
    let p = palette(ui.ctx());
    let focused = ui.memory(|m| m.has_focus(id));
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if focused {
        ui.painter().rect_filled(
            rect.expand(3.0),
            CornerRadius::same(theme::RADIUS + 3),
            p.accent_bg,
        );
    }
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        p.surface,
        Stroke::new(1.0, if focused { p.accent } else { p.border_strong }),
        egui::StrokeKind::Inside,
    );
    let font = if mono {
        theme::mono_font(theme::FS_SM)
    } else {
        egui::FontId::proportional(theme::FS_BASE)
    };
    let line_h = (theme::FS_BASE * 1.45).round();
    let inner = Rect::from_min_max(
        egui::pos2(rect.left() + 10.0, rect.center().y - line_h / 2.0),
        egui::pos2(rect.right() - 10.0, rect.center().y + line_h / 2.0),
    );
    // a child, so the cursor of the row stays where the field ends
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    // no taller than its line, or it starts at the top of the line and its text sits low
    child.spacing_mut().interact_size.y = 0.0;
    egui::TextEdit::singleline(text)
        .id(id)
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .font(font.clone())
        .hint_text(RichText::new(hint).font(font).color(p.text_3))
        .align(egui::Align2::LEFT_CENTER)
        .min_size(inner.size())
        .desired_width(inner.width())
        .interactive(enabled)
        .show(&mut child)
}

/// The mark of a radio button (`input[type=radio]`, 16 px in the accent color): a ring, a dot
/// in it when chosen, pale when it cannot be chosen
pub fn radio_mark(ui: &Ui, center: egui::Pos2, on: bool, enabled: bool, hovered: bool) {
    let p = palette(ui.ctx());
    let painter = ui.painter();
    if !enabled {
        painter.circle(center, 7.5, p.surface_2, Stroke::new(1.0, p.border));
    } else if on {
        painter.circle(center, 7.25, p.surface, Stroke::new(1.5, p.accent));
        painter.circle_filled(center, 4.0, p.accent);
    } else {
        let line = if hovered { p.text_2 } else { p.text_3 };
        painter.circle(center, 7.5, p.surface, Stroke::new(1.0, line));
    }
}

/// Code in a line of text (`code`: the font of addresses, a little smaller than the text)
pub fn inline_code(ui: &mut Ui, text: &str) -> Response {
    ui.label(RichText::new(text).font(theme::mono_font(theme::FS_SM * 0.92)))
}

/// A value in the font of addresses, which can be selected and copied
pub fn code(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Label::new(
        RichText::new(text).font(theme::mono_font(theme::FS_SM)),
    ))
}

/// The traffic of an entry as one bar (`.tbar`): `share` of the width of `width` (at least
/// 1.5 %), incoming in the first color, outgoing in the second
pub fn tbar(ui: &mut Ui, width: f32, height: f32, share: f32, incoming: f32, outgoing: f32) {
    let p = palette(ui.ctx());
    let (cell, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let full = (width * share.clamp(0.015, 1.0)).max(3.0);
    let rect = Rect::from_min_size(cell.min, egui::vec2(full, height));
    let radius = (height / 2.0) as u8;
    let total = incoming + outgoing;
    let painter = ui.painter();
    if total <= 0.0 {
        painter.rect_filled(rect, CornerRadius::same(radius), p.surface_3);
        return;
    }
    let one = full * incoming / total;
    // one shape, round outside and straight where the shares meet: the second color below, the
    // first one over it, so nothing shows through at the seam
    painter.rect_filled(
        rect,
        CornerRadius::same(radius),
        if outgoing > 0.0 { p.s2 } else { p.s1 },
    );
    if outgoing > 0.0 && one > 0.0 {
        let first = Rect::from_min_size(rect.min, egui::vec2(one, height));
        let corners = CornerRadius {
            nw: radius,
            sw: radius,
            ne: 0,
            se: 0,
        };
        painter.rect_filled(first.intersect(rect), corners, p.s1);
    }
}

/// The star of a favorite (`.star`, 24 px)
pub fn star(ui: &mut Ui, on: bool, text: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, on, text));
    let p = palette(ui.ctx());
    let color = if on {
        p.star
    } else if response.hovered() {
        p.text
    } else {
        p.text_3
    };
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), p.hover);
    }
    let icon = if on { icons::STAR_ON } else { icons::STAR };
    paint_icon(
        ui,
        icon,
        Rect::from_center_size(rect.center(), egui::vec2(ICON_SM, ICON_SM)),
        color,
    );
    focus_ring(ui, &response, theme::RADIUS_SM);
    response.on_hover_text(text)
}

/// A switch (`.switch`, 32 × 18) with its label; `big` for the label in 14 px (`.switch.big`)
pub fn switch(ui: &mut Ui, on: &mut bool, text: &str, big: bool) -> Response {
    let p = palette(ui.ctx());
    let (font, color) = if big {
        (medium_font(theme::FS_BASE), p.text)
    } else {
        (egui::FontId::proportional(theme::FS_SM), p.text_2)
    };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let gap = if text.is_empty() { 0.0 } else { theme::SP2 };
    let size = egui::vec2(32.0 + gap + galley.size().x, 18.0_f32.max(galley.size().y));
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(egui::Role::Switch, true, *on, text));
    let track = Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - 9.0),
        egui::vec2(32.0, 18.0),
    );
    let how = ui.ctx().animate_bool_with_time(response.id, *on, 0.15);
    let fill = if *on { p.accent } else { p.border_strong };
    ui.painter().rect_filled(track, CornerRadius::same(9), fill);
    let knob = egui::pos2(track.left() + 9.0 + 14.0 * how, track.center().y);
    ui.painter().add(egui::epaint::CircleShape::filled(
        knob + egui::vec2(0.0, 0.5),
        7.5,
        p.shadow.gamma_multiply(0.5),
    ));
    ui.painter().circle_filled(knob, 7.0, p.surface);
    let at = egui::pos2(track.right() + gap, rect.center().y - galley.size().y / 2.0)
        .round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, color);
    focus_ring(ui, &response, 9);
    response
}

/// The legend of a chart (`.legend`): a line in the first color, a dashed one in the second
pub fn legend(ui: &mut Ui, first: &str, second: &str) {
    let p = palette(ui.ctx());
    ui.spacing_mut().item_spacing.x = 6.0;
    for (text, color, dashed) in [(second, p.s2, true), (first, p.s1, false)] {
        ui.label(small(text).color(p.text_2));
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 14.0), Sense::hover());
        let y = rect.center().y;
        if dashed {
            ui.painter().extend(egui::Shape::dashed_line(
                &[egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                Stroke::new(2.0, color),
                4.0,
                2.0,
            ));
        } else {
            ui.painter()
                .hline(rect.x_range(), y, Stroke::new(2.0, color));
        }
        ui.add_space(theme::SP4 - 6.0);
    }
}

/// A choice from a list (`select`): 32 px high, 28 px in toolbars (`small`), the text and an
/// arrow; returns the index chosen
pub fn select(ui: &mut Ui, labels: &[String], current: usize, small: bool) -> Option<usize> {
    select_width(ui, labels, current, small, None)
}

/// [`select`] in a given width (`select` in a `.field`: as wide as the field)
pub fn select_width(
    ui: &mut Ui,
    labels: &[String],
    current: usize,
    small: bool,
    width: Option<f32>,
) -> Option<usize> {
    let p = palette(ui.ctx());
    let (height, size) = if small {
        (28.0, theme::FS_SM)
    } else {
        (CONTROL_H, theme::FS_BASE)
    };
    let font = egui::FontId::proportional(size);
    let widest = labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap(l.clone(), font.clone(), p.text)
                .size()
                .x
        })
        .fold(0.0, f32::max);
    let width = width.unwrap_or((widest + 10.0 + 30.0).ceil());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let text = labels.get(current).cloned().unwrap_or_default();
    response.widget_info(|| WidgetInfo::labeled(egui::Role::ComboBox, true, &text));
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let line = if open || response.has_focus() {
        p.accent
    } else {
        p.border_strong
    };
    if open || response.has_focus() {
        ui.painter().rect_filled(
            rect.expand(3.0),
            CornerRadius::same(theme::RADIUS + 3),
            p.accent_bg,
        );
    }
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        p.surface,
        Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
    let galley = ui.painter().layout_no_wrap(text, font.clone(), p.text);
    let at = egui::pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0)
        .round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, p.text);
    paint_icon(
        ui,
        icons::CHEV_DOWN,
        Rect::from_center_size(
            egui::pos2(rect.right() - 8.0 - ICON_SM / 2.0, rect.center().y),
            egui::vec2(ICON_SM, ICON_SM),
        ),
        p.text_2,
    );
    let mut chosen = None;
    egui::Popup::from_toggle_button_response(&response)
        .gap(2.0)
        .width(width)
        .frame(
            Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(CornerRadius::same(theme::RADIUS))
                .shadow(p.shadow_2())
                .inner_margin(Margin::same(theme::SP1 as i8)),
        )
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, label) in labels.iter().enumerate() {
                let (row, r) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), Sense::click());
                r.widget_info(|| {
                    WidgetInfo::selected(egui::Role::Button, true, i == current, label)
                });
                if r.hovered() || i == current {
                    ui.painter().rect_filled(
                        row,
                        CornerRadius::same(theme::RADIUS_SM),
                        if r.hovered() { p.hover } else { p.selected },
                    );
                }
                let g = ui
                    .painter()
                    .layout_no_wrap(label.clone(), font.clone(), p.text);
                let at = egui::pos2(row.left() + 6.0, row.center().y - g.size().y / 2.0)
                    .round_to_pixels(ui.ctx().pixels_per_point());
                ui.painter().galley(at, g, p.text);
                if r.clicked() {
                    chosen = Some(i);
                    ui.close();
                }
            }
        });
    chosen.filter(|&i| i != current)
}

/// A text of several lines (`textarea`, `.mono` for addresses): `rows` lines high, 8 px above
/// and below the text, 10 px at the sides; it grows with its text
pub fn textarea(ui: &mut Ui, text: &mut String, id: Id, rows: usize, mono: bool) -> Response {
    let p = palette(ui.ctx());
    let focused = ui.memory(|m| m.has_focus(id));
    let font = if mono {
        theme::mono_font(theme::FS_BASE)
    } else {
        egui::FontId::proportional(theme::FS_BASE)
    };
    let width = ui.available_width();
    // a line as Chrome sets it (`line-height: normal` of the font)
    let line = ui
        .fonts_mut(|f| f.row_height(&font))
        .max(theme::FS_BASE * 1.333);
    let layout = ui
        .painter()
        .layout(format!("{text} "), font.clone(), p.text, width - 20.0);
    let height = (layout.size().y.max(line * rows as f32) + 18.0).ceil();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    if focused {
        ui.painter().rect_filled(
            rect.expand(3.0),
            CornerRadius::same(theme::RADIUS + 3),
            p.accent_bg,
        );
    }
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        p.surface,
        Stroke::new(1.0, if focused { p.accent } else { p.border_strong }),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink2(egui::vec2(10.0, 9.0));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
    );
    child.add(
        egui::TextEdit::multiline(text)
            .id(id)
            .frame(Frame::NONE)
            .margin(Margin::ZERO)
            .font(font)
            .text_color(p.text)
            .desired_width(inner.width())
            .desired_rows(rows),
    )
}

/// A slider (`input[type=range]` in the accent color): an 8 px track, filled up to the round
/// handle; `step` is the step of the value
pub fn range(ui: &mut Ui, value: &mut u8, step: u8, width: f32, text: &str) -> Response {
    let p = palette(ui.ctx());
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::slider(true, f64::from(*value), text));
    let track = Rect::from_min_max(
        egui::pos2(rect.left() + 3.0, rect.center().y - 4.0),
        egui::pos2(rect.right() - 3.0, rect.center().y + 4.0),
    );
    if let Some(at) = response.interact_pointer_pos() {
        let share = ((at.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let step = f32::from(step.max(1));
        let new = ((share * 100.0 / step).round() * step).clamp(0.0, 100.0) as u8;
        if new != *value {
            *value = new;
            response.mark_changed();
        }
    }
    if response.has_focus() {
        let (left, right) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowUp),
            )
        });
        let new = if left {
            value.saturating_sub(step)
        } else if right {
            value.saturating_add(step).min(100)
        } else {
            *value
        };
        if new != *value {
            *value = new;
            response.mark_changed();
        }
    }
    let x = track.left() + track.width() * f32::from(*value) / 100.0;
    let painter = ui.painter();
    painter.rect(
        track,
        CornerRadius::same(4),
        p.surface_3,
        Stroke::new(1.0, p.border_strong),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(
        Rect::from_min_max(track.min, egui::pos2(x, track.bottom())),
        CornerRadius::same(4),
        p.accent,
    );
    let knob = egui::pos2(x, track.center().y);
    painter.circle_filled(knob, 8.0, p.accent);
    if response.hovered() || response.dragged() {
        painter.circle_stroke(knob, 8.0, Stroke::new(1.0, p.accent_hover));
    }
    focus_ring(ui, &response, 9);
    response
}

/// A box to tick with its label (`label.check`): 16 px in the accent color, 4 px above and below
pub fn check(ui: &mut Ui, on: &mut bool, text: &str) -> Response {
    let p = palette(ui.ctx());
    let font = egui::FontId::proportional(theme::FS_BASE);
    let galley = ui
        .painter()
        .layout(text.to_owned(), font, p.text, ui.available_width() - 24.0);
    // a line of the text is as high as in the draft (`--lh: 1.45`)
    let line = theme::FS_BASE * 1.45;
    let first_row = galley.rows.first().map_or(line, |r| r.height());
    let lift = ((line - first_row) / 2.0).max(0.0);
    let size = egui::vec2(
        16.0 + theme::SP2 + galley.size().x,
        (galley.size().y + 2.0 * lift).max(line) + 8.0,
    );
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, true, *on, text));
    let first = rect.top() + 4.0 + line / 2.0;
    let boxed =
        Rect::from_center_size(egui::pos2(rect.left() + 8.0, first), egui::vec2(16.0, 16.0));
    if *on {
        ui.painter()
            .rect_filled(boxed, CornerRadius::same(3), p.accent);
        ui.painter().add(egui::Shape::line(
            vec![
                boxed.left_top() + egui::vec2(3.5, 8.0),
                boxed.left_top() + egui::vec2(6.5, 11.0),
                boxed.left_top() + egui::vec2(12.5, 5.0),
            ],
            Stroke::new(2.0, p.surface),
        ));
    } else {
        ui.painter().rect(
            boxed,
            CornerRadius::same(3),
            p.surface,
            Stroke::new(
                1.0,
                if response.hovered() {
                    p.text_3
                } else {
                    p.border_strong
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    let at = egui::pos2(rect.left() + 16.0 + theme::SP2, rect.top() + 4.0 + lift)
        .round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, p.text);
    focus_ring(ui, &response, 3);
    response
}

/// A command to copy (`.codebox`): in the font of code on `--surface-2`, a button to copy it at
/// the right; returns whether that was clicked
pub fn codebox(ui: &mut Ui, code: &str, copy: &str) -> bool {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let font = theme::mono_font(theme::FS_SM);
    let galley = ui.painter().layout(
        code.to_owned(),
        font,
        p.text,
        width - theme::SP3 - 6.0 - CONTROL_SM_H - theme::SP2,
    );
    let height = galley.size().y.max(CONTROL_SM_H) + 12.0 + 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        p.surface_2,
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    let at = egui::pos2(
        rect.left() + 1.0 + theme::SP3,
        rect.center().y - galley.size().y / 2.0,
    )
    .round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, p.text);
    let button = Rect::from_min_size(
        egui::pos2(
            rect.right() - 7.0 - CONTROL_SM_H,
            rect.center().y - CONTROL_SM_H / 2.0,
        ),
        egui::vec2(CONTROL_SM_H, CONTROL_SM_H),
    );
    let mut child = ui.new_child(UiBuilder::new().max_rect(button));
    iconbtn(&mut child, icons::COPY, copy, false, true)
        .on_hover_text(copy)
        .clicked()
}

/// A group of fields with its title on the frame (`fieldset.fset` with `legend`): `--surface-2`
/// inside, 12 px above, 16 px at the sides, 16 px below it
pub fn fieldset(ui: &mut Ui, legend: &str, add: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    let galley =
        ui.painter()
            .layout_no_wrap(legend.to_uppercase(), medium_font(theme::FS_XS), p.text_3);
    // the title is a line of 12 px with `--lh: 1.45`; the frame starts in its middle
    let legend_h = theme::FS_XS * 1.45;
    let half = (legend_h / 2.0).round();
    let top = ui.cursor().top();
    let left = ui.cursor().left();
    let width = ui.available_width();
    let shape = ui.painter().add(egui::Shape::Noop);
    let mut inner = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(left + theme::SP4 + 1.0, top + legend_h + theme::SP3),
        egui::vec2(width - 2.0 * theme::SP4 - 2.0, f32::INFINITY),
    )));
    inner.set_width(width - 2.0 * theme::SP4 - 2.0);
    add(&mut inner);
    let bottom = inner.min_rect().bottom() + 1.0;
    let frame = Rect::from_min_max(
        egui::pos2(left, top + half),
        egui::pos2(left + width, bottom),
    );
    ui.painter().set(
        shape,
        egui::Shape::rect_filled(frame, CornerRadius::same(theme::RADIUS), p.surface_2),
    );
    ui.painter().rect_stroke(
        frame,
        CornerRadius::same(theme::RADIUS),
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    // the title interrupts the line (`legend`: 6 px at its sides)
    let title = Rect::from_min_size(
        egui::pos2(left + theme::SP4 - 6.0 + 1.0, top),
        egui::vec2(galley.size().x + 12.0, legend_h),
    );
    ui.painter().rect_filled(
        Rect::from_min_max(
            egui::pos2(title.left(), frame.top()),
            egui::pos2(title.right(), frame.top() + 1.0),
        ),
        0.0,
        p.surface_2,
    );
    ui.painter().galley(
        egui::pos2(title.left() + 6.0, title.center().y - galley.size().y / 2.0),
        galley,
        p.text_3,
    );
    ui.advance_cursor_after_rect(Rect::from_min_max(
        egui::pos2(left, top),
        egui::pos2(left + width, bottom),
    ));
    ui.add_space(theme::SP4);
}

/// Two cards side by side (`.split2`: 5 to 6 parts, 16 px apart), one below the other below
/// 1180 px; the cards keep their own heights
pub fn split2(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    if ui.ctx().content_rect().width() < 1180.0 {
        left(ui);
        right(ui);
        return;
    }
    let gap = theme::SP4;
    let width = ui.available_width();
    let w_left = (width - gap) * 5.0 / 11.0;
    let top = ui.cursor().top();
    let x = ui.cursor().left();
    let column = |ui: &mut Ui, from: f32, w: f32, salt: usize| {
        let mut column = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(from, top),
                    egui::vec2(w, f32::INFINITY),
                ))
                .layout(Layout::top_down(Align::Min))
                .id_salt(("split2", salt)),
        );
        column.spacing_mut().item_spacing.y = theme::SP4;
        column
    };
    let mut one = column(ui, x, w_left, 0);
    left(&mut one);
    let mut two = column(ui, x + w_left + gap, width - gap - w_left, 1);
    right(&mut two);
    let bottom = one.min_rect().bottom().max(two.min_rect().bottom());
    ui.advance_cursor_after_rect(Rect::from_min_max(
        egui::pos2(x, top),
        egui::pos2(x + width, bottom),
    ));
}

/// Room left in a card of [`grid2`] to be as high as the card beside it
#[derive(Default)]
pub struct Fill {
    /// Where the card should end, from the last frame
    bottom: Option<f32>,
    /// What was added
    added: f32,
}

impl Fill {
    /// Adds the room at the end of the body of the card
    pub fn apply(&mut self, ui: &mut Ui) {
        if let Some(bottom) = self.bottom {
            // the line of the card is below the body
            let gap = (bottom - 1.0 - ui.cursor().top()).floor();
            if gap > 0.0 {
                ui.add_space(gap);
                self.added = gap;
            }
        }
    }
}

/// Two cards side by side in equal columns (`.grid2`: 16 px apart, one below the other below
/// 900 px). As in a grid, the two cards are as high as the higher one: each fills up with its
/// [`Fill`] to the height the cards had in the last frame.
pub fn grid2(
    ui: &mut Ui,
    salt: &str,
    left: impl FnOnce(&mut Ui, &mut Fill),
    right: impl FnOnce(&mut Ui, &mut Fill),
) {
    if ui.ctx().content_rect().width() < 900.0 {
        left(ui, &mut Fill::default());
        right(ui, &mut Fill::default());
        return;
    }
    let id = Id::NULL.with(("grid2", salt));
    let height: Option<f32> = ui.ctx().data(|d| d.get_temp(id));
    let gap = theme::SP4;
    let width = ui.available_width();
    let w = (width - gap) / 2.0;
    let top = ui.cursor().top();
    let x = ui.cursor().left();
    let mut natural = 0.0_f32;
    let mut bottom = top;
    let (mut left, mut right) = (Some(left), Some(right));
    for (i, from) in [x, x + w + gap].into_iter().enumerate() {
        let mut column = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(from, top),
                    egui::vec2(w, f32::INFINITY),
                ))
                .layout(Layout::top_down(Align::Min))
                .id_salt(("grid2", salt, i)),
        );
        column.spacing_mut().item_spacing.y = theme::SP4;
        let mut fill = Fill {
            bottom: height.map(|h| top + h),
            added: 0.0,
        };
        if let Some(left) = left.take() {
            left(&mut column, &mut fill);
        } else if let Some(right) = right.take() {
            right(&mut column, &mut fill);
        }
        let rect = column.min_rect();
        natural = natural.max(rect.height() - fill.added);
        bottom = bottom.max(rect.bottom());
    }
    if height != Some(natural) {
        ui.ctx().data_mut(|d| d.insert_temp(id, natural));
        ui.ctx().request_repaint();
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(
        egui::pos2(x, top),
        egui::pos2(x + width, bottom),
    ));
}

/// The pages of a list (`.pager`): "Page", the numbers (the first two, the last two and those next
/// to the current one), then `note` at the right. Returns the page chosen.
pub fn pager(ui: &mut Ui, t: Lang, page: usize, pages: usize, note: &str) -> Option<usize> {
    let p = palette(ui.ctx());
    let mut chosen = None;
    rule(ui);
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP3 as i8, theme::SP2 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                ui.label(small(t.get("page-label")).color(p.text_3));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let shown: Vec<usize> = (1..=pages)
                        .filter(|&i| i <= 2 || i + 2 > pages || i.abs_diff(page) <= 1)
                        .collect();
                    for (k, &i) in shown.iter().enumerate() {
                        if k > 0 && i - shown[k - 1] > 1 {
                            page_number(ui, "…", false, false);
                        }
                        if page_number(ui, &i.to_string(), i == page, true).clicked() && i != page {
                            chosen = Some(i);
                        }
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(small(note).color(p.text_3));
                });
            });
        });
    chosen
}

fn page_number(ui: &mut Ui, text: &str, current: bool, link: bool) -> Response {
    let p = palette(ui.ctx());
    let font = if current {
        theme::mono_medium_font(theme::FS_XS)
    } else {
        theme::mono_font(theme::FS_XS)
    };
    let color = if current { p.text } else { p.text_2 };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let width = (galley.size().x + 12.0).max(28.0);
    let sense = if link && !current {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), sense);
    let radius = CornerRadius::same(theme::RADIUS_SM);
    if current {
        ui.painter().rect_filled(rect, radius, p.selected);
    } else if link && response.hovered() {
        ui.painter().rect_filled(rect, radius, p.hover);
    }
    let at = (rect.center() - galley.size() / 2.0).round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, color);
    response
}

/// The width of a column of a [`table`]
#[derive(Clone, Copy)]
pub enum Width {
    /// Fixed (`.w0` columns, numbers)
    Px(f32),
    /// A share of the width of the table (`style="width:22%"`)
    Share(f32),
    /// What is left, shared among such columns, at least this much
    Rest(f32),
}

/// A column of a [`table`]
pub struct Col {
    pub head: String,
    pub width: Width,
    /// Numbers at the right (`.num`)
    pub right: bool,
}

impl Col {
    pub fn new(head: impl Into<String>, width: Width) -> Self {
        Self {
            head: head.into(),
            width,
            right: false,
        }
    }

    pub fn num(head: impl Into<String>, width: Width) -> Self {
        Self {
            head: head.into(),
            width,
            right: true,
        }
    }
}

/// What a [`table`] looks like and holds
pub struct TableSpec<'a> {
    pub id: &'a str,
    pub cols: Vec<Col>,
    /// Height of a row (`--row-h` 34 px, more with a second line)
    pub row_h: f32,
    pub rows: usize,
    /// The body scrolls below a head that stays; otherwise all rows are shown
    pub scroll: bool,
    /// Rows can be clicked (`tr.click`)
    pub click: bool,
    /// The marked row (`tr.sel`)
    pub selected: Option<usize>,
    /// Space at the sides of the cells
    pub pad: f32,
    /// No head
    pub headless: bool,
}

impl<'a> TableSpec<'a> {
    pub fn new(id: &'a str, cols: Vec<Col>, row_h: f32, rows: usize) -> Self {
        Self {
            id,
            cols,
            row_h,
            rows,
            scroll: false,
            click: false,
            selected: None,
            pad: theme::SP3,
            headless: false,
        }
    }
}

/// What happened in a [`table`]
#[derive(Default)]
pub struct TableOut {
    pub clicked: Option<usize>,
    pub hovered: Option<usize>,
}

fn widths(cols: &[Col], total: f32) -> Vec<f32> {
    let fixed: f32 = cols
        .iter()
        .map(|c| match c.width {
            Width::Px(w) => w,
            Width::Share(s) => total * s,
            Width::Rest(_) => 0.0,
        })
        .sum();
    let rests = cols
        .iter()
        .filter(|c| matches!(c.width, Width::Rest(_)))
        .count()
        .max(1) as f32;
    let left = ((total - fixed) / rests).max(0.0);
    cols.iter()
        .map(|c| match c.width {
            Width::Px(w) => w,
            Width::Share(s) => total * s,
            Width::Rest(min) => left.max(min),
        })
        .collect()
}

/// A table (`.tbl`): a head in 13 px on the second surface, rows with a line below each but the
/// last, the row under the pointer marked. `cell` draws the cell of a row and column.
pub fn table(
    ui: &mut Ui,
    spec: &TableSpec,
    mut cell: impl FnMut(&mut Ui, usize, usize),
) -> TableOut {
    let p = palette(ui.ctx());
    let id = Id::NULL.with(("table", spec.id));
    let total = ui.available_width();
    let widths = widths(&spec.cols, total);
    if !spec.headless {
        let (head, _) = ui.allocate_exact_size(egui::vec2(total, 32.0), Sense::hover());
        ui.painter().rect_filled(head, 0.0, p.surface_2);
        ui.painter().hline(
            head.x_range(),
            head.bottom() - 0.5,
            Stroke::new(1.0, p.border),
        );
        let mut x = head.left();
        for (col, w) in spec.cols.iter().zip(&widths) {
            let rect = Rect::from_min_size(egui::pos2(x, head.top()), egui::vec2(*w, 31.0));
            let galley =
                ui.painter()
                    .layout_no_wrap(col.head.clone(), medium_font(theme::FS_SM), p.text_3);
            let gx = if col.right {
                rect.right() - spec.pad - galley.size().x
            } else {
                rect.left() + spec.pad
            };
            let at = egui::pos2(gx, rect.center().y - galley.size().y / 2.0)
                .round_to_pixels(ui.ctx().pixels_per_point());
            ui.painter()
                .with_clip_rect(rect.intersect(ui.clip_rect()))
                .galley(at, galley, p.text_3);
            x += w;
        }
    }
    let mut out = TableOut::default();
    let mut rows = |ui: &mut Ui, range: std::ops::Range<usize>| {
        for r in range {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(total, spec.row_h), Sense::hover());
            let sense = if spec.click {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = ui.interact(rect, id.with(r), sense);
            let hovered = ui.rect_contains_pointer(rect);
            let fill = if spec.selected == Some(r) {
                p.selected
            } else if hovered {
                p.hover
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 0.0, fill);
            if r + 1 < spec.rows {
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom() - 0.5,
                    Stroke::new(1.0, p.border),
                );
            }
            let mut x = rect.left();
            for (c, (col, w)) in spec.cols.iter().zip(&widths).enumerate() {
                let cell_rect = Rect::from_min_size(
                    egui::pos2(x, rect.top()),
                    egui::vec2(*w, spec.row_h - 1.0),
                );
                let inner = cell_rect.shrink2(egui::vec2(spec.pad, 0.0));
                let layout = if col.right {
                    Layout::right_to_left(Align::Center)
                } else {
                    Layout::left_to_right(Align::Center)
                };
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(inner)
                        .layout(layout)
                        .id_salt(("cell", r, c)),
                );
                child.set_clip_rect(cell_rect.intersect(ui.clip_rect()));
                child.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                child.spacing_mut().interact_size.y = 20.0;
                cell(&mut child, r, c);
                x += w;
            }
            if response.clicked() {
                out.clicked = Some(r);
            }
            if hovered {
                out.hovered = Some(r);
            }
            if spec.click {
                response.on_hover_cursor(egui::CursorIcon::PointingHand);
            }
        }
    };
    if spec.scroll {
        egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink(false)
            .show_rows(ui, spec.row_h, spec.rows, |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                rows(ui, range);
            });
    } else {
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        rows(ui, 0..spec.rows);
        ui.spacing_mut().item_spacing.y = spacing;
    }
    out
}

/// A label that works as a link (pointer, underline under the pointer)
pub fn link_text(ui: &mut Ui, text: RichText) -> Response {
    let response = ui
        .add(
            egui::Label::new(text)
                .sense(Sense::click())
                .selectable(false)
                .truncate(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.hovered() {
        let p = palette(ui.ctx());
        ui.painter().hline(
            response.rect.x_range(),
            response.rect.bottom() - 1.0,
            Stroke::new(1.0, p.accent),
        );
    }
    response
}

/// Numbers in a table cell (`td.num`, `td.mono`: Fira Code 13 px)
pub fn mono(text: impl Into<String>) -> RichText {
    RichText::new(text).font(theme::mono_font(theme::FS_SM))
}

/// A row of "Auffällig" (`.attn .arow`): icon, title and text, a button at the right; returns
/// whether the button was clicked
pub fn attention_row(
    ui: &mut Ui,
    warn: bool,
    icon: ImageSource<'static>,
    title: &str,
    text: &str,
    button: &str,
    last: bool,
) -> bool {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let top = ui.cursor().top();
    let left = ui.cursor().left();
    let mut clicked = false;
    // the text is measured first, so that icon and button stand in the middle of the row
    let button_w = button_width(ui, button) - 2.0 * theme::SP3 + 2.0 * theme::SP2;
    let text_w = (width - 2.0 * theme::SP4 - 22.0 - button_w - 2.0 * theme::SP3).max(40.0);
    let lines = |text: &str, font: egui::FontId| {
        ui.painter()
            .layout(text.to_owned(), font, Color32::PLACEHOLDER, text_w)
            .rows
            .len()
            .max(1) as f32
    };
    let text_h = lines(title, medium_font(theme::FS_BASE)) * LH_BASE
        + lines(text, egui::FontId::proportional(theme::FS_SM)) * theme::FS_SM * 1.45;
    let height = text_h.max(theme::ROW_H - 12.0);
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(
                egui::pos2(left + theme::SP4, top + 6.0),
                egui::vec2(width - 2.0 * theme::SP4, height),
            ))
            .layout(Layout::left_to_right(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = theme::SP3;
    let (icon_rect, _) = row.allocate_exact_size(egui::vec2(22.0, 16.0), Sense::hover());
    paint_icon(
        &row,
        icon,
        Rect::from_min_size(icon_rect.min, egui::vec2(16.0, 16.0)),
        if warn { p.warn } else { p.text_3 },
    );
    row.allocate_ui_with_layout(
        egui::vec2(text_w, text_h),
        Layout::top_down(Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_width(text_w);
            ui.add(
                egui::Label::new(
                    RichText::new(title)
                        .font(medium_font(theme::FS_BASE))
                        .line_height(Some(LH_BASE)),
                )
                .wrap(),
            );
            ui.add(
                egui::Label::new(
                    small(text)
                        .color(p.text_2)
                        .line_height(Some(theme::FS_SM * 1.45)),
                )
                .wrap(),
            );
        },
    );
    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
        clicked = small_button(ui, button, Kind::Secondary).clicked();
    });
    let bottom = (row.min_rect().bottom() + 6.0).max(top + theme::ROW_H);
    let rect = Rect::from_min_max(egui::pos2(left, top), egui::pos2(left + width, bottom));
    if !last {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, p.border),
        );
    }
    ui.advance_cursor_after_rect(rect);
    clicked
}

// ---- the old parts, until every page is redone (stage C of 5.6) ----

/// Narrowest field of a [`field_row`] (`minmax(180px, 1fr)`)
const FIELD_MIN: f32 = 180.0;

/// The name of a field (`.field > label`: 13 px, medium) and the 4 px below it
pub fn field_label(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .font(medium_font(theme::FS_SM))
                .color(p.text)
                .line_height(Some(theme::FS_SM * 1.45)),
        )
        .wrap(),
    );
    ui.add_space(theme::SP1);
}

/// The explanation below a control (`.help`), 4 px below it
pub fn field_help(ui: &mut Ui, text: &str) {
    ui.add_space(theme::SP1);
    hint(ui, text);
}

/// The space below a field (`.field`: 16 px)
pub fn end_field(ui: &mut Ui) {
    ui.add_space(theme::SP4);
}

/// Two fields side by side (`.field-row`), one below the other when there is no room
pub fn field_row(ui: &mut Ui, left: impl FnOnce(&mut Ui, f32), right: impl FnOnce(&mut Ui, f32)) {
    let width = ui.available_width();
    if width < 2.0 * FIELD_MIN + theme::SP4 {
        left(ui, width);
        right(ui, width);
        return;
    }
    let column = ((width - theme::SP4) / 2.0).floor();
    let top = ui.cursor().min;
    let mut bottom = top.y;
    for (i, add) in [
        Box::new(left) as Box<dyn FnOnce(&mut Ui, f32)>,
        Box::new(right),
    ]
    .into_iter()
    .enumerate()
    {
        let x = top.x + i as f32 * (column + theme::SP4);
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(x, top.y),
                    egui::vec2(column, f32::INFINITY),
                ))
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_width(column);
        add(&mut child, column);
        bottom = bottom.max(child.min_rect().bottom());
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(top, egui::pos2(top.x + width, bottom)));
}
