//! The details of a packet: its layers and fields, and its bytes. A field marks its bytes in the
//! hex dump, and a byte selects the smallest field that contains it.

use std::ops::Range;
use std::time::Duration;

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Color32, RichText, ScrollArea, Sense, TextFormat, Ui};

use crate::dissect::{Field, Layer};
use crate::gui::load::Loader;
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, Page, local_time};
use crate::view::Jump;
use crate::view::packets::{HexLine, PacketDetail, conversation_filter, packet_detail};

/// Room for the offset, 16 bytes and their characters
const HEX_WIDTH: f32 = 610.0;

#[derive(Default)]
pub struct State {
    detail: Loader<(u64, usize), Option<PacketDetail>>,
    /// The bytes of the field chosen
    selected: Option<Range<usize>>,
    number: u64,
}

pub fn show(app: &mut App, ui: &mut Ui, number: u64) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let key = (number, app.live.capture_id);
    let state = &mut app.pages.packet;
    if state.number != number {
        state.number = number;
        state.selected = None;
    }
    state.detail.poll();
    let engine = app.engine.clone();
    let every = app.live.running.then_some(Duration::from_secs(2));
    state
        .detail
        .load(&ctx, &key, every, move || packet_detail(t, &engine, number));
    let Some(found) = app.pages.packet.detail.value_of(&key) else {
        app.back_arrow(ui);
        return;
    };
    let Some(d) = found else {
        app.back_arrow(ui);
        widgets::notice_text(ui, &t.get("gui-packet-gone"));
        return;
    };

    let mut go: Option<Page> = None;
    let mut replace: Option<Page> = None;
    let mut back = false;
    let subtitle = format!("{} · {}", d.info, local_time(t, d.time));
    let title = t.get1("packet-title", "number", d.number);
    let p = palette(&ctx);
    let protocol = d.protocol;
    let can_go_back = app.can_go_back();
    ui.horizontal(|ui| {
        if can_go_back
            && widgets::icon_button(ui, crate::gui::icons::BACK, p.muted, &t.get("back-hint"))
                .clicked()
        {
            back = true;
        }
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(&title);
                widgets::tag(ui, protocol, p.neutral_soft, p.text);
            });
            ui.add(egui::Label::new(RichText::new(&subtitle).color(p.muted)).truncate());
        });
    });
    ui.horizontal_wrapped(|ui| {
        match d.prev {
            Some(n)
                if widgets::small_button(
                    ui,
                    format!("‹ {}", t.get("packet-prev")),
                    Kind::Secondary,
                )
                .clicked() =>
            {
                replace = Some(Page::Packet(n));
            }
            _ => {}
        }
        match d.next {
            Some(n)
                if widgets::small_button(
                    ui,
                    format!("{} ›", t.get("packet-next")),
                    Kind::Secondary,
                )
                .clicked() =>
            {
                replace = Some(Page::Packet(n));
            }
            _ => {}
        }
        if let Some(key) = &d.connection {
            ui.separator();
            ui.label(RichText::new(t.get("packet-connection-label")).color(p.muted));
            let conversation = conversation_filter(key);
            if widgets::small_button(ui, t.get("nav-packets"), Kind::Secondary)
                .on_hover_text(t.get("connection-packets-hint"))
                .clicked()
            {
                go = Some(
                    Jump::Packets {
                        filter: conversation.clone(),
                        connections: String::new(),
                    }
                    .into(),
                );
            }
            if widgets::small_button(ui, t.get("nav-statistics"), Kind::Secondary)
                .on_hover_text(t.get("connection-statistics-hint"))
                .clicked()
            {
                go = Some(
                    Jump::Statistics {
                        filter: conversation,
                        connections: String::new(),
                    }
                    .into(),
                );
            }
            if d.has_stream
                && widgets::small_button(ui, t.get("packet-stream"), Kind::Secondary)
                    .on_hover_text(t.get("stream-title"))
                    .clicked()
            {
                go = Some(Page::Stream(d.number));
            }
            if widgets::small_button(ui, t.get("packet-connection"), Kind::Secondary).clicked() {
                go = Some(Page::Connection(*key));
            }
        }
        for (label, n) in &d.related {
            if widgets::link(ui, label.as_str()).clicked() {
                go = Some(Page::Packet(*n));
            }
        }
    });
    if let Some(note) = &d.assembled {
        widgets::notice_text(ui, note);
    }
    if d.truncated {
        let note = if d.headers_only {
            "packet-truncated-headers"
        } else {
            "packet-truncated"
        };
        widgets::notice_text(ui, &t.get(note));
    }

    let selected = app.pages.packet.selected.clone();
    let mut pick = None;
    let Some(Some(d)) = app.pages.packet.detail.value_of(&key) else {
        return;
    };
    let height = ui.available_height();
    // the bytes need their width, the layers take what is left
    let room = ui.available_width();
    let gap = ui.spacing().item_spacing.x;
    let bytes_width = HEX_WIDTH.min(room * 0.6);
    let layers_width = room - bytes_width - gap;
    let column = egui::Layout::top_down(egui::Align::Min);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(layers_width, height), column, |ui| {
            ui.set_height(height);
            widgets::flush_card(
                ui,
                &t.get("packet-layers"),
                |_| {},
                |ui| {
                    ScrollArea::both()
                        .id_salt("layers")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.spacing_mut().interact_size.y = 22.0;
                            for (i, layer) in d.layers.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add_space(10.0);
                                    ui.vertical(|ui| {
                                        layer_tree(ui, i, layer, selected.as_ref(), &mut pick);
                                    });
                                });
                            }
                        });
                },
            );
        });
        ui.allocate_ui_with_layout(egui::vec2(bytes_width, height), column, |ui| {
            ui.set_height(height);
            widgets::flush_card(
                ui,
                &t.get("packet-bytes"),
                |_| {},
                |ui| {
                    ScrollArea::both()
                        .id_salt("hex")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.add_space(8.0);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.spacing_mut().interact_size.y = 20.0;
                            for line in &d.hex {
                                ui.horizontal(|ui| {
                                    ui.add_space(14.0);
                                    if let Some(byte) = hex_line(ui, line, selected.as_ref()) {
                                        pick = smallest_field(&d.layers, byte);
                                    }
                                });
                            }
                            if let Some(note) = &d.hex_cut {
                                ui.add_space(6.0);
                                widgets::hint(ui, note);
                            }
                        });
                },
            );
        });
    });

    if pick.is_some() {
        app.pages.packet.selected = pick;
    }
    if back {
        app.back();
    }
    if let Some(page) = replace {
        app.replace(page);
    } else if let Some(page) = go {
        app.go(page);
    }
}

fn layer_tree(
    ui: &mut Ui,
    index: usize,
    layer: &Layer,
    selected: Option<&Range<usize>>,
    pick: &mut Option<Range<usize>>,
) {
    let header = egui::CollapsingHeader::new(&layer.title)
        .id_salt(index)
        .default_open(true)
        .show(ui, |ui| {
            for field in &layer.fields {
                field_row(ui, field, selected, pick);
            }
        });
    if header.header_response.clicked() {
        *pick = Some(layer.range.clone());
    }
}

fn field_row(
    ui: &mut Ui,
    field: &Field,
    selected: Option<&Range<usize>>,
    pick: &mut Option<Range<usize>>,
) {
    let p = palette(ui.ctx());
    let mut job = LayoutJob::default();
    let body = egui::TextStyle::Body.resolve(ui.style());
    let format = |color: Color32| TextFormat {
        font_id: body.clone(),
        color,
        ..TextFormat::default()
    };
    job.append(field.name, 0.0, format(p.muted));
    if !field.value.is_empty() {
        job.append(": ", 0.0, format(p.muted));
        job.append(&field.value, 0.0, format(p.text));
    }
    let on = selected == Some(&field.range);
    let response = ui.add(egui::Button::selectable(on, job));
    if response.clicked() {
        *pick = Some(field.range.clone());
    }
    if !field.children.is_empty() {
        ui.indent((field.name, field.range.start), |ui| {
            for child in &field.children {
                field_row(ui, child, selected, pick);
            }
        });
    }
}

/// The smallest field around a byte
fn smallest_field(layers: &[Layer], byte: usize) -> Option<Range<usize>> {
    fn search(fields: &[Field], byte: usize, best: &mut Option<Range<usize>>) {
        for field in fields {
            if field.range.contains(&byte) {
                if best.as_ref().is_none_or(|b| field.range.len() <= b.len()) {
                    *best = Some(field.range.clone());
                }
                search(&field.children, byte, best);
            }
        }
    }
    let mut best = None;
    for layer in layers {
        if layer.range.contains(&byte) {
            search(&layer.fields, byte, &mut best);
        }
    }
    best
}

/// A line of the hex dump; returns the byte clicked
fn hex_line(ui: &mut Ui, line: &HexLine, selected: Option<&Range<usize>>) -> Option<usize> {
    let p = palette(ui.ctx());
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let plain = TextFormat {
        font_id: font.clone(),
        color: p.text,
        ..TextFormat::default()
    };
    let marked = TextFormat {
        background: p.accent_soft,
        ..plain.clone()
    };
    let offset = TextFormat {
        font_id: font,
        color: p.faint,
        ..TextFormat::default()
    };
    // the byte of every character of the line
    let mut bytes: Vec<Option<usize>> = Vec::new();
    let mut job = LayoutJob::default();
    let mut add = |text: &str, format: &TextFormat, byte: Option<usize>| {
        job.append(text, 0.0, format.clone());
        bytes.extend(std::iter::repeat_n(byte, text.chars().count()));
    };
    add(&line.offset, &offset, None);
    add("  ", &plain, None);
    for (i, (byte, text)) in line.bytes.iter().enumerate() {
        let on = selected.is_some_and(|r| r.contains(byte));
        add(text, if on { &marked } else { &plain }, Some(*byte));
        add(if i == 7 { "  " } else { " " }, &plain, None);
    }
    let missing = 16 - line.bytes.len();
    let padding = missing * 3 + usize::from(line.bytes.len() <= 7) + 1;
    add(&" ".repeat(padding), &plain, None);
    let mut buffer = [0; 4];
    for (byte, c) in &line.text {
        let on = selected.is_some_and(|r| r.contains(byte));
        let text = c.encode_utf8(&mut buffer);
        add(text, if on { &marked } else { &plain }, Some(*byte));
    }
    let galley = ui.painter().layout_job(job);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    ui.painter().galley(rect.min, galley.clone(), p.text);
    let at = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let index: usize = galley.cursor_from_pos(at - rect.min).index.into();
    bytes.get(index).copied().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_selects_the_smallest_field() {
        let layers = vec![Layer {
            title: "Layer".to_owned(),
            range: 0..20,
            fields: vec![
                Field::new("whole", "", 0..20).with_children(vec![
                    Field::new("flags", "", 4..6),
                    Field::new("other", "", 10..12),
                ]),
                Field::new("tail", "", 18..20),
            ],
        }];
        assert_eq!(smallest_field(&layers, 5), Some(4..6));
        assert_eq!(smallest_field(&layers, 8), Some(0..20));
        assert_eq!(smallest_field(&layers, 19), Some(18..20));
        assert_eq!(smallest_field(&layers, 25), None);
    }
}
