//! The parts every page is made of: page heads, cards, buttons, notes and fact lists.

use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, Frame, ImageSource, Layout, Margin, Response,
    RichText, Sense, Stroke, TextWrapMode, Ui, WidgetInfo, WidgetText,
};

use crate::gui::theme::{Palette, palette, semibold_font};
use crate::view::i18n::Lang;

/// Height of the buttons and fields
pub const CONTROL_H: f32 = 32.0;
/// Height of the small buttons in the heads of the cards
pub const CONTROL_SM_H: f32 = 26.0;

/// The look of a button
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Danger,
}

fn colors(p: &Palette, kind: Kind) -> (Color32, Color32, Stroke) {
    match kind {
        Kind::Primary => (p.accent, Color32::WHITE, Stroke::NONE),
        Kind::Secondary => (p.surface, p.text, Stroke::new(1.0, p.border_strong)),
        Kind::Danger => (p.down, Color32::WHITE, Stroke::NONE),
    }
}

/// A button of the given kind
pub fn button(ui: &mut Ui, text: impl Into<String>, kind: Kind) -> Response {
    sized_button(ui, text, kind, CONTROL_H)
}

/// A small button
pub fn small_button(ui: &mut Ui, text: impl Into<String>, kind: Kind) -> Response {
    sized_button(ui, text, kind, CONTROL_SM_H)
}

fn sized_button(ui: &mut Ui, text: impl Into<String>, kind: Kind, height: f32) -> Response {
    let (fill, fg, stroke) = colors(palette(ui.ctx()), kind);
    let text = RichText::new(text.into()).color(fg);
    ui.add(
        Button::new(text)
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(8))
            .min_size(egui::vec2(0.0, height)),
    )
}

/// A button that is shown but cannot be used
pub fn disabled_button(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.add_enabled_ui(false, |ui| button(ui, text, Kind::Secondary))
        .inner
}

/// Buttons in a row, one of them chosen; returns the index of a clicked one
pub fn segmented(ui: &mut Ui, labels: &[String], chosen: usize) -> Option<usize> {
    let p = palette(ui.ctx());
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let last = labels.len().saturating_sub(1);
        // from the right, the last one comes first
        let reversed = ui.layout().prefer_right_to_left();
        for at in 0..labels.len() {
            let i = if reversed { last - at } else { at };
            let label = &labels[i];
            let on = i == chosen;
            let (fill, fg) = if on {
                (p.accent, Color32::WHITE)
            } else {
                (p.surface, p.text)
            };
            let radius = CornerRadius {
                nw: if i == 0 { 8 } else { 0 },
                sw: if i == 0 { 8 } else { 0 },
                ne: if i == last { 8 } else { 0 },
                se: if i == last { 8 } else { 0 },
            };
            let response = ui.add(
                Button::new(RichText::new(label).color(fg))
                    .fill(fill)
                    .stroke(Stroke::new(1.0, p.border_strong))
                    .corner_radius(radius)
                    .min_size(egui::vec2(0.0, CONTROL_SM_H)),
            );
            if response.clicked() {
                clicked = Some(i);
            }
        }
    });
    clicked
}

/// The pages of a list: first, back, numbers around the current page, next, last, and a field for
/// a page number. Returns the page chosen.
pub fn pager(ui: &mut Ui, t: Lang, page: usize, pages: usize, text: &mut String) -> Option<usize> {
    let mut chosen = None;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(t.get("page-label"));
        let field = ui.add(
            egui::TextEdit::singleline(text)
                .hint_text(page.to_string())
                .desired_width(52.0),
        );
        if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            chosen = text.trim().parse::<usize>().ok().map(|n| n.clamp(1, pages));
            text.clear();
        }
        ui.label(t.get1("page-of-total", "pages", pages));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(14.0);
            let mut go = |ui: &mut Ui, label: String, to: usize, on: bool| {
                let kind = if to == page && on {
                    Kind::Primary
                } else {
                    Kind::Secondary
                };
                let enabled = to != page && (1..=pages).contains(&to);
                if ui
                    .add_enabled_ui(enabled || on, |ui| small_button(ui, label, kind))
                    .inner
                    .clicked()
                    && enabled
                {
                    chosen = Some(to);
                }
            };
            go(ui, t.get("page-last"), pages, false);
            go(ui, t.get("page-next"), page + 1, false);
            for n in (page.saturating_sub(2).max(1)..=(page + 2).min(pages)).rev() {
                go(ui, n.to_string(), n, true);
            }
            go(ui, t.get("page-prev"), page.saturating_sub(1), false);
            go(ui, t.get("page-first"), 1, false);
        });
    });
    ui.add_space(8.0);
    chosen
}

/// A clickable icon without a frame; the text says what it does, for the tooltip and screen readers
pub fn icon_button(
    ui: &mut Ui,
    source: ImageSource<'static>,
    color: Color32,
    text: &str,
) -> Response {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text));
    if response.hovered() {
        let p = palette(ui.ctx());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.surface_2);
    }
    if response.has_focus() {
        let p = palette(ui.ctx());
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.5, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    egui::Image::new(source)
        .tint(color)
        .paint_at(ui, rect.shrink(5.0));
    response.on_hover_text(text)
}

/// A link inside the window
pub fn link(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(egui::Link::new(text))
}

/// The head of a page: title on the left, then what `actions` adds on the right
pub fn page_head(ui: &mut Ui, title: &str, subtitle: Option<&str>, actions: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.set_max_width((ui.available_width() - 260.0).max(240.0));
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add(egui::Label::new(RichText::new(title).heading()).wrap_mode(TextWrapMode::Wrap));
            if let Some(subtitle) = subtitle {
                ui.label(RichText::new(subtitle).color(p.muted));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Min), actions);
    });
    ui.add_space(6.0);
}

fn card_frame(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
}

/// A card with a head and a body with space around it
pub fn card<R>(
    ui: &mut Ui,
    title: &str,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    flush_card(ui, title, tools, |ui| {
        Frame::new()
            .inner_margin(Margin::same(14))
            .show(ui, body)
            .inner
    })
}

/// A card whose body reaches its edges, for tables
pub fn flush_card<R>(
    ui: &mut Ui,
    title: &str,
    tools: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    let p = palette(ui.ctx());
    card_frame(p)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            Frame::new()
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.set_min_height(CONTROL_SM_H);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(title).font(semibold_font(14.0)));
                        ui.with_layout(Layout::right_to_left(Align::Center), tools);
                    });
                });
            let width = ui.available_width();
            let (line, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, p.border);
            ui.spacing_mut().item_spacing.y = 8.0;
            body(ui)
        })
        .inner
}

/// Room around a table inside of a card: a table of fixed height, or one to the end of the card
pub fn table_area<R>(ui: &mut Ui, height: Option<f32>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width = ui.available_width();
    let height = height.unwrap_or_else(|| ui.available_height());
    ui.allocate_ui(egui::vec2(width, height), |ui| {
        Frame::new()
            .inner_margin(Margin {
                left: 12,
                right: 12,
                top: 2,
                bottom: 2,
            })
            .show(ui, add)
            .inner
    })
    .inner
}

/// A tile with a large value
pub fn tile(ui: &mut Ui, label: &str, value: &str) {
    let p = palette(ui.ctx());
    card_frame(p)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(value).font(semibold_font(20.0)));
            ui.label(RichText::new(label).small().color(p.muted));
        });
}

/// Tiles of equal width, in as many rows as they need
pub fn tiles(ui: &mut Ui, items: &[(String, String)]) {
    let per_row = ((ui.available_width() / 170.0) as usize).clamp(1, items.len().max(1));
    for row in items.chunks(per_row) {
        ui.columns(per_row, |columns| {
            for (column, (label, value)) in columns.iter_mut().zip(row) {
                tile(column, label, value);
            }
        });
    }
}

fn message(ui: &mut Ui, fill: Color32, stroke: Color32, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(add);
        });
}

/// A note above the content of a page; `add` may add links
pub fn notice(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    message(ui, p.warn_soft, p.warn_soft, add);
}

/// A note with a plain text
pub fn notice_text(ui: &mut Ui, text: &str) {
    notice(ui, |ui| {
        ui.label(text);
    });
}

/// That something went well
pub fn flash(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    message(ui, p.ok_soft, p.ok_soft, |ui| {
        ui.label(text);
    });
}

/// What went wrong
pub fn error(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    message(ui, p.down_soft, p.down, |ui| {
        ui.label(text);
    });
}

/// A small explanation
pub fn hint(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    // columns are justified, which would spread the words of a wrapped text
    ui.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.label(RichText::new(text).small().color(p.muted));
    });
}

/// A cell of a grid with its text at the right edge
pub fn right_cell(ui: &mut Ui, width: f32, text: RichText) {
    let size = egui::vec2(width, ui.text_style_height(&egui::TextStyle::Body));
    ui.allocate_ui_with_layout(size, Layout::right_to_left(Align::Center), |ui| {
        ui.label(text);
    });
}

/// A centered text for a list without content
pub fn empty(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(text).color(p.muted));
    });
    ui.add_space(24.0);
}

/// A pale label next to a name
pub fn tag(ui: &mut Ui, text: &str, fill: Color32, fg: Color32) {
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(8, 1))
        .show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(text).small().color(fg)).selectable(false));
        });
}

/// A list of labels with their values
pub fn facts(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([18.0, 6.0])
        .show(ui, add);
}

/// A label of [`facts`]
pub fn fact_label(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.label(RichText::new(text).color(p.muted));
}

/// A value in the font of addresses, which can be selected and copied
pub fn code(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Label::new(RichText::new(text).monospace()))
}

/// A horizontal bar of two shares (0–100), for the traffic of a list entry
pub fn share_bar(ui: &mut Ui, width: f32, first: f32, second: f32, a: Color32, b: Color32) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 8.0), Sense::hover());
    let radius = CornerRadius::same(3);
    ui.painter().rect_filled(rect, radius, p.neutral_soft);
    let w = rect.width() / 100.0;
    let mut one = rect;
    one.set_width(first * w);
    ui.painter().rect_filled(one, radius, a);
    let mut two = rect;
    two.min.x = one.max.x;
    two.set_width(second * w);
    ui.painter().rect_filled(two, radius, b);
}
