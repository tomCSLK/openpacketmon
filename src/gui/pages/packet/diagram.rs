//! The structure of a packet (`.dg` of the draft), below the hex dump: for each layer its name,
//! a ruler of the bits 0 to 31 and its fields in rows of 32 bits. A field marks its bytes in the
//! hex dump and the smallest field around them in the layers; a field or byte chosen there marks
//! the fields in its bytes here. The rows come from [`crate::view::diagram`].

use std::ops::Range;

use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{
    self, Align2, CornerRadius, CursorIcon, FontId, Rect, Sense, Stroke, TextFormat, Ui, WidgetInfo,
};

use super::{Asked, Chosen, View};
use crate::dissect::{Field, Layer};
use crate::gui::theme::{self, medium_font, mono_font, palette};
use crate::gui::widgets;
use crate::view::diagram::{DiagramField, DiagramLayer, ROW_BITS};

/// Height of a row of fields (`.dgc`: 26 px)
const ROW_H: f32 = 26.0;
/// Height of the ruler: the numbers and the ticks under them
const RULER_H: f32 = 22.0;
/// Height of the name of a layer
const HEAD_H: f32 = 22.0;

/// The head of the card: the note and Names | Values
pub(super) fn tools(ui: &mut Ui, view: &View, asked: &mut Asked) {
    let t = view.t;
    let labels = [
        t.get("packet-diagram-names"),
        t.get("packet-diagram-values"),
    ];
    if let Some(i) = widgets::segmented_sized(ui, &labels, usize::from(view.values), true) {
        asked.values = Some(i == 1);
    }
    widgets::card_note(ui, &t.get("packet-diagram-hint"));
}

/// The layers of the packet one below the other
pub(super) fn body(ui: &mut Ui, view: &View, asked: &mut Asked) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            if view.d.diagram.is_empty() {
                widgets::hint(ui, &view.t.get("packet-diagram-none"));
                return;
            }
            let marks = marks(view);
            for (i, layer) in view.d.diagram.iter().enumerate() {
                if i > 0 {
                    ui.add_space(theme::SP4);
                }
                heading(ui, view, layer, asked);
                ruler(ui);
                grid(ui, view, i, layer, &marks[i], asked);
            }
        });
}

/// Which fields are marked: the one chosen here, else those in the bytes chosen (or, if none
/// lies wholly in them, those that share bytes with them)
fn marks(view: &View) -> Vec<Vec<bool>> {
    let diagram = &view.d.diagram;
    if let Some((layer, field)) = view.picked {
        return diagram
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (0..l.fields.len())
                    .map(|k| (i, k) == (layer, field))
                    .collect()
            })
            .collect();
    }
    let Some(range) = view.range.as_ref().filter(|r| !r.is_empty()) else {
        return diagram
            .iter()
            .map(|l| vec![false; l.fields.len()])
            .collect();
    };
    let inside = |r: &Range<usize>| range.start <= r.start && r.end <= range.end;
    let shared = |r: &Range<usize>| r.start < range.end && range.start < r.end;
    let any_inside = diagram
        .iter()
        .flat_map(|l| &l.fields)
        .any(|f| inside(&f.range));
    diagram
        .iter()
        .map(|l| {
            l.fields
                .iter()
                .map(|f| {
                    if any_inside {
                        inside(&f.range)
                    } else {
                        shared(&f.range)
                    }
                })
                .collect()
        })
        .collect()
}

/// The name of a layer and its size; a click marks the layer
fn heading(ui: &mut Ui, view: &View, layer: &DiagramLayer, asked: &mut Asked) {
    let p = palette(ui.ctx());
    let name = ui
        .painter()
        .layout_no_wrap(layer.name.clone(), medium_font(theme::FS_SM), p.text);
    let size = ui.painter().layout_no_wrap(
        layer.size.clone(),
        FontId::proportional(theme::FS_XS),
        p.text_3,
    );
    let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), HEAD_H), Sense::hover());
    let width = (name.size().x + theme::SP2 + size.size().x).min(row.width());
    let rect = Rect::from_min_size(
        row.min + egui::vec2(-theme::SP1, 1.0),
        egui::vec2(width + 2.0 * theme::SP1, HEAD_H - 4.0),
    );
    let response = ui
        .interact(
            rect,
            ui.scope_id().with(("dg-layer", layer.layer)),
            Sense::click(),
        )
        .on_hover_cursor(CursorIcon::PointingHand);
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &layer.name));
    let on = view
        .chosen
        .is_some_and(|c| c.layer == layer.layer && c.field.is_empty());
    if on {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), p.hl);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), p.hover);
    }
    widgets::focus_ring(ui, &response, theme::RADIUS_SM);
    let y = rect.center().y;
    let name_w = name.size().x;
    ui.painter().galley(
        egui::pos2(row.left(), y - name.size().y / 2.0).round(),
        name,
        p.text,
    );
    ui.painter().galley(
        egui::pos2(row.left() + name_w + theme::SP2, y - size.size().y / 2.0).round(),
        size,
        p.text_3,
    );
    if response.clicked() {
        asked.choose = Some(Chosen {
            layer: layer.layer,
            field: Vec::new(),
        });
    }
}

/// The bits 0, 8, 16, 24 and 31 over their columns, a tick between every two bits
fn ruler(ui: &mut Ui) {
    let p = palette(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), RULER_H), Sense::hover());
    let column = rect.width() / ROW_BITS as f32;
    let painter = ui.painter();
    for bit in [0, 8, 16, 24, 31] {
        painter.text(
            egui::pos2(rect.left() + (bit as f32 + 0.5) * column, rect.top()),
            Align2::CENTER_TOP,
            bit.to_string(),
            FontId::monospace(theme::FS_XS - 1.0),
            p.text_3,
        );
    }
    let stroke = Stroke::new(1.0, p.text_3);
    for edge in 0..=ROW_BITS {
        let x = painter.round_to_pixel_center(rect.left() + edge as f32 * column);
        let top = if edge % 8 == 0 {
            rect.bottom() - 8.0
        } else {
            rect.bottom() - 4.0
        };
        painter.vline(x, top..=rect.bottom(), stroke);
    }
}

/// The fields of a layer in their rows, with the lines between them
fn grid(
    ui: &mut Ui,
    view: &View,
    index: usize,
    layer: &DiagramLayer,
    marked: &[bool],
    asked: &mut Asked,
) {
    let p = palette(ui.ctx());
    let (area, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), layer.rows as f32 * ROW_H),
        Sense::hover(),
    );
    let column = area.width() / ROW_BITS as f32;
    let x = |bit: usize| area.left() + bit as f32 * column;
    let y = |row: usize| area.top() + row as f32 * ROW_H;
    let segment_rect = |row: usize, from: u32, to: u32| {
        Rect::from_min_max(
            egui::pos2(x(from as usize), y(row)),
            egui::pos2(x(to as usize), y(row + 1)),
        )
    };

    // every segment answers the pointer; only the one with the label takes the focus
    let mut responses = Vec::new();
    let mut hovered = None;
    for (k, field) in layer.fields.iter().enumerate() {
        for (s, segment) in field.segments.iter().enumerate() {
            let sense = if s == field.main {
                Sense::click()
            } else {
                Sense::CLICK
            };
            let response = ui
                .interact(
                    segment_rect(segment.row, segment.from, segment.to),
                    ui.scope_id().with(("dg", layer.layer, k, s)),
                    sense,
                )
                .on_hover_cursor(CursorIcon::PointingHand);
            if response.hovered() {
                hovered = Some(k);
            }
            responses.push((k, s, response));
        }
    }

    let painter = ui.painter();
    for (k, _, response) in &responses {
        let color = if marked.get(*k).copied().unwrap_or(false) {
            p.hl
        } else if hovered == Some(*k) {
            p.hover
        } else {
            continue;
        };
        painter.rect_filled(response.rect, 0.0, color);
    }

    let stroke = Stroke::new(1.0, p.text_3);
    let (across, down) = layer.lines();
    for line in across {
        painter.hline(
            x(line.from)..=x(line.to),
            painter.round_to_pixel_center(y(line.at)),
            stroke,
        );
    }
    for line in down {
        painter.vline(
            painter.round_to_pixel_center(x(line.at)),
            y(line.from)..=y(line.to),
            stroke,
        );
    }

    for (k, s, response) in responses {
        let field = &layer.fields[k];
        if s == field.main {
            // a field of whole rows has its text in the middle of them
            let whole = field
                .segments
                .iter()
                .all(|s| s.from == 0 && s.to == ROW_BITS && !s.broken);
            let rect = if whole {
                field.segments.iter().fold(response.rect, |rect, s| {
                    rect.union(segment_rect(s.row, s.from, s.to))
                })
            } else {
                response.rect
            };
            label(ui, field, rect, view.values);
            response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, field.line()));
            widgets::focus_ring(ui, &response, 0);
        }
        let response = response.on_hover_text(&field.tip);
        if response.clicked() {
            asked.choose = Some(containing(&view.d.layers, layer.layer, &field.range));
            asked.pick = Some((index, k));
        }
        response.context_menu(|ui| {
            view.item(ui, "copy-line", asked, || field.line());
            view.item(ui, "copy-value", asked, || field.value.clone());
            view.bytes_item(ui, &field.range, asked);
            view.all_item(ui, asked);
        });
    }

    // the broken edges of the long fields (`//`)
    let font = FontId::monospace(theme::FS_XS);
    for field in &layer.fields {
        for segment in field.segments.iter().filter(|s| s.broken) {
            let mid = y(segment.row) + ROW_H / 2.0;
            for edge in [x(0), x(ROW_BITS as usize)] {
                let galley = ui
                    .painter()
                    .layout_no_wrap("//".to_owned(), font.clone(), p.text_3);
                let size = galley.size();
                let at = egui::pos2(edge - size.x / 2.0, mid - size.y / 2.0);
                ui.painter()
                    .rect_filled(Rect::from_min_size(at, size), 0.0, p.surface);
                ui.painter().galley(at.round(), galley, p.text_3);
            }
        }
    }
}

/// The text of a field in the middle of its segment: the name (a short one if the name does
/// not fit) or the value, cut with "…" when too long
fn label(ui: &Ui, field: &DiagramField, rect: Rect, values: bool) {
    let p = palette(ui.ctx());
    let room = (rect.width() - 6.0).max(4.0);
    let font = if values && !field.block {
        mono_font(theme::FS_XS)
    } else {
        FontId::proportional(theme::FS_XS)
    };
    let mut text = field.label(values);
    if !values
        && !field.block
        && let Some(short) = field.short
        && ui
            .painter()
            .layout_no_wrap(text.clone(), font.clone(), p.text)
            .size()
            .x
            > room
    {
        short.clone_into(&mut text);
    }
    let mut job = LayoutJob::single_section(
        text,
        TextFormat {
            font_id: font,
            color: p.text,
            ..TextFormat::default()
        },
    );
    job.wrap = TextWrapping::truncate_at_width(room);
    let galley = ui.painter().layout_job(job);
    let at = rect.center() - galley.size() / 2.0;
    ui.painter().galley(at.round(), galley, p.text);
}

/// The smallest field of layer `index` around the bytes `range`, else the layer
fn containing(layers: &[Layer], index: usize, range: &Range<usize>) -> Chosen {
    fn search(
        fields: &[Field],
        range: &Range<usize>,
        path: &mut Vec<usize>,
        best: &mut Option<(usize, Vec<usize>)>,
    ) {
        for (i, field) in fields.iter().enumerate() {
            let r = &field.range;
            if r.is_empty() || r.start > range.start || range.end > r.end {
                continue;
            }
            path.push(i);
            if best.as_ref().is_none_or(|(len, _)| r.len() <= *len) {
                *best = Some((r.len(), path.clone()));
            }
            search(&field.children, range, path, best);
            path.pop();
        }
    }
    let mut best = None;
    if let Some(layer) = layers.get(index) {
        search(&layer.fields, range, &mut Vec::new(), &mut best);
    }
    Chosen {
        layer: index,
        field: best.map(|(_, path)| path).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_of_the_drawing_chooses_the_smallest_field_around_it() {
        let layers = vec![
            Layer {
                title: "Frame".to_owned(),
                range: 0..40,
                fields: Vec::new(),
            },
            Layer {
                title: "Internet Protocol Version 4".to_owned(),
                range: 0..20,
                fields: vec![
                    Field::new("Version", "4", 0..1),
                    Field::new("Header length", "20 bytes (5)", 0..1),
                    Field::new("Flags", "0x2", 6..8).with_children(vec![Field::new(
                        "Don't fragment",
                        "Set",
                        6..7,
                    )]),
                    Field::new("Time to live", "64", 8..9),
                ],
            },
        ];
        let chosen = |range| containing(&layers, 1, &range).field;
        assert_eq!(chosen(8..9), vec![3]);
        // two fields of the same byte: the last
        assert_eq!(chosen(0..1), vec![1]);
        // the fragment offset in the bytes of the flags: the flags, not their bit
        assert_eq!(chosen(6..8), vec![2]);
        // nothing around the options: the layer
        assert_eq!(chosen(20..24), Vec::<usize>::new());
    }
}
