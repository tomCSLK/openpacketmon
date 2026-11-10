//! The details of a packet (`pagePacket` of the draft): the way back to the list, paging through
//! the packets of the list, the connection, the layers with their fields and the bytes. A field
//! marks its bytes in the hex dump, and a byte selects the smallest field that contains it.

use std::ops::Range;
use std::time::Duration;

use eframe::egui::text::LayoutJob;
use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, CornerRadius, CursorIcon, Galley, Key, Layout, Modifiers, Rect, RichText,
    ScrollArea, Sense, TextFormat, Ui, UiBuilder, WidgetInfo,
};

use crate::dissect::{Field, Layer};
use crate::gui::load::Loader;
use crate::gui::pages::detail_head;
use crate::gui::theme::{self, medium_font, mono_font, palette};
use crate::gui::widgets::{self, Kind, SegItem, Tone};
use crate::gui::{App, Page, icons};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::view::Jump;
use crate::view::i18n::Lang;
use crate::view::packets::{
    HexLine, PacketDetail, Related, connection_label, conversation_filter, dump_text, field_line,
    field_value, layer_text, layers_text, neighbors, packet_detail, selection_hex, selection_text,
};

/// Line height of the layers and fields (13 px, `--lh: 1.45`)
const LH: f32 = theme::FS_SM * 1.45;
/// Line height of the hex dump (`.hex`: 1.6)
const HEX_LH: f32 = theme::FS_SM * 1.6;
/// Room left of the fields (`.layers .f`: 34 px)
const FIELD_INDENT: f32 = 34.0;
/// Room left of each level of the details of a field
const CHILD_INDENT: f32 = 16.0;
/// The column of the names (`minmax(140px, 220px)`)
const KEY_MIN: f32 = 140.0;
const KEY_MAX: f32 = 220.0;
/// The small icons (`.ico.sm`)
const ICON_SM: f32 = 14.0;

/// A layer or a field in it, by the path of indices to it
#[derive(Clone, PartialEq, Debug)]
struct Chosen {
    layer: usize,
    /// Empty for the layer itself
    field: Vec<usize>,
}

/// What a detail was loaded for: the packet and the filters of the list it is paged in
#[derive(Clone, PartialEq)]
struct DetailKey {
    number: u64,
    capture: usize,
    display: String,
    connections: String,
}

#[derive(Default)]
pub struct State {
    detail: Loader<DetailKey, Option<PacketDetail>>,
    number: u64,
    /// The layer or field chosen
    chosen: Option<Chosen>,
    /// The layers opened (`details open`)
    open: Vec<bool>,
    /// Bring the chosen field into view (after a click on a byte)
    reveal: bool,
}

/// What the page asks for in a frame
#[derive(Default)]
struct Asked {
    choose: Option<Chosen>,
    /// Open or close a layer
    toggle: Option<usize>,
    copy: Option<String>,
    go: Option<Page>,
    step: Option<u64>,
}

impl Asked {
    /// Takes over what another part of the page asks for
    fn merge(&mut self, other: Asked) {
        self.choose = other.choose.or(self.choose.take());
        self.toggle = other.toggle.or(self.toggle);
        self.copy = other.copy.or(self.copy.take());
        self.go = other.go.or(self.go.take());
        self.step = other.step.or(self.step);
    }
}

pub fn show(app: &mut App, ui: &mut Ui, number: u64) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    // the list the packet was opened from: its filters for paging, and the packet it shows when
    // going back
    let (display, connections) = match app.history.last_mut() {
        Some(Page::Packets {
            filter,
            connections,
            goto,
        }) => {
            *goto = Some(number);
            (filter.clone(), connections.clone())
        }
        _ => (String::new(), String::new()),
    };
    let list = Page::Packets {
        filter: display.clone(),
        connections: connections.clone(),
        goto: Some(number),
    };
    let key = DetailKey {
        number,
        capture: app.live.capture_id,
        display: display.clone(),
        connections: connections.clone(),
    };
    let state = &mut app.pages.packet;
    if state.number != number {
        state.number = number;
        state.chosen = None;
        state.open.clear();
        state.reveal = false;
    }
    state.detail.poll();
    let (engine, index) = (app.engine.clone(), app.index.clone());
    let every = app.live.running.then_some(Duration::from_secs(2));
    state.detail.load(&ctx, &key, every, move || {
        packet_detail(t, &engine, number).map(|mut d| {
            (d.prev, d.next) = neighbors(t, &engine, &index, &display, &connections, number);
            d
        })
    });

    let found = app.pages.packet.detail.value_of(&key);
    let head = found.map(|d| {
        d.as_ref()
            .map(|d| (d.protocol, d.info.clone(), d.prev, d.next))
    });
    let title = t.get1("packet-title", "number", number);
    let mut asked = Asked::default();
    let (prev_text, next_text) = (t.get("packet-prev"), t.get("packet-next"));
    detail_head(
        app,
        ui,
        &[(t.get("nav-packets"), list)],
        &title,
        |ui| {
            widgets::title(ui, &title);
            if let Some(Some((protocol, ..))) = &head {
                widgets::tag(ui, protocol);
            }
        },
        |ui| {
            if let Some(Some((_, info, ..))) = &head {
                let p = palette(ui.ctx());
                ui.add(
                    egui::Label::new(
                        RichText::new(info)
                            .font(mono_font(theme::FS_BASE * 0.92))
                            .color(p.text_3)
                            .line_height(Some(widgets::LH_BASE)),
                    )
                    .wrap(),
                );
            }
        },
        |ui| {
            let Some(Some((_, _, prev, next))) = head else {
                return;
            };
            let items = [
                SegItem {
                    before: Some(icons::CHEV_LEFT),
                    text: &prev_text,
                    after: None,
                    enabled: prev.is_some(),
                },
                SegItem {
                    before: None,
                    text: &next_text,
                    after: Some(icons::CHEV_RIGHT),
                    enabled: next.is_some(),
                },
            ];
            match widgets::seg_nav(ui, &items) {
                Some(0) => asked.step = prev,
                Some(_) => asked.step = next,
                None => {}
            }
        },
    );
    if let Some(Some((_, _, prev, next))) = head
        && !ctx.egui_wants_keyboard_input()
    {
        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::ArrowLeft) && prev.is_some() {
                asked.step = prev;
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowRight) && next.is_some() {
                asked.step = next;
            }
        });
    }

    match app.pages.packet.detail.value_of(&key) {
        None => {}
        Some(None) => widgets::notice_text(ui, &t.get("gui-packet-gone")),
        Some(Some(d)) => {
            let state = &app.pages.packet;
            let open = if state.open.len() == d.layers.len() {
                state.open.clone()
            } else {
                // as in the draft: the last two layers of four or more, else the last one
                let n = d.layers.len();
                let from = if n >= 4 { n - 2 } else { n.saturating_sub(1) };
                (0..n).map(|i| i >= from).collect()
            };
            let view = View {
                t,
                d,
                chosen: state.chosen.as_ref(),
                range: state
                    .chosen
                    .as_ref()
                    .and_then(|c| chosen_range(&d.layers, c)),
                open: &open,
                reveal: state.reveal,
            };
            body(ui, &view, &mut asked);
            // the text of a chosen field or layer goes to the clipboard
            if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)))
                && asked.copy.is_none()
                && let Some(text) = view.chosen.and_then(|c| chosen_text(&d.layers, c))
            {
                asked.copy = Some(text);
            }
            let state = &mut app.pages.packet;
            state.open = open;
            state.reveal = false;
        }
    }

    let state = &mut app.pages.packet;
    if let Some(i) = asked.toggle
        && let Some(open) = state.open.get_mut(i)
    {
        *open = !*open;
    }
    if let Some(chosen) = asked.choose {
        if let Some(open) = state.open.get_mut(chosen.layer)
            && !chosen.field.is_empty()
        {
            // a byte chooses a field in a closed layer, too
            state.reveal = !*open;
            *open = true;
        }
        state.chosen = Some(chosen);
    }
    if let Some(text) = asked.copy {
        ctx.copy_text(text);
    }
    if let Some(n) = asked.step {
        app.replace(Page::Packet(n));
    } else if let Some(page) = asked.go {
        app.go(page);
    }
}

/// What the parts of the page draw from
struct View<'a> {
    t: Lang,
    d: &'a PacketDetail,
    chosen: Option<&'a Chosen>,
    /// The bytes of what is chosen
    range: Option<Range<usize>>,
    open: &'a [bool],
    reveal: bool,
}

impl View<'_> {
    /// An entry of a menu that copies a text
    fn item(&self, ui: &mut Ui, key: &str, asked: &mut Asked, text: impl FnOnce() -> String) {
        if ui.button(self.t.get(key)).clicked() {
            asked.copy = Some(text());
            ui.close();
        }
    }

    /// The entry for the bytes of a range, if there are any in the dump
    fn bytes_item(&self, ui: &mut Ui, range: &Range<usize>, asked: &mut Asked) {
        let hex = selection_hex(&self.d.hex, range);
        if !hex.is_empty() {
            self.item(ui, "copy-bytes-hex", asked, || hex);
        }
    }

    /// The entry the menus have in common
    fn all_item(&self, ui: &mut Ui, asked: &mut Asked) {
        ui.separator();
        self.item(ui, "copy-all", asked, || layers_text(&self.d.layers));
    }
}

/// The note on the connection, the notes on the packet and the two cards
fn body(ui: &mut Ui, view: &View, asked: &mut Asked) {
    let (t, d) = (view.t, view.d);
    if let Some(key) = &d.connection {
        connection_note(
            ui,
            t,
            key,
            d.has_stream.then_some(d.number),
            d.call.as_ref(),
            asked,
        );
    }
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
    let (mut left, mut right, mut copy) = (Asked::default(), Asked::default(), false);
    widgets::split2(
        ui,
        |ui| {
            widgets::flush_card(
                ui,
                &t.get("packet-layers"),
                |ui| widgets::card_note(ui, &t.get("packet-layers-hint")),
                |ui| layers(ui, view, &mut left),
            );
        },
        |ui| {
            let count = d.hex.iter().map(|line| line.bytes.len()).sum::<usize>();
            widgets::flush_card(
                ui,
                &t.get("packet-bytes"),
                |ui| {
                    if widgets::icon_text_button(
                        ui,
                        Some(icons::COPY),
                        &t.get("packet-copy"),
                        Kind::Ghost,
                        true,
                    )
                    .clicked()
                    {
                        copy = true;
                    }
                    // between the title and the button (`justify-content: space-between`)
                    let text = t.get1("packet-bytes-count", "count", count as f64);
                    let p = palette(ui.ctx());
                    let galley = ui.painter().layout_no_wrap(
                        text.clone(),
                        egui::FontId::proportional(theme::FS_SM),
                        p.text_3,
                    );
                    let room = ui.available_width() - ui.spacing().item_spacing.x;
                    ui.add_space(((room - galley.size().x) / 2.0).max(0.0));
                    widgets::card_note(ui, &text);
                },
                |ui| hex_dump(ui, view, &mut right),
            );
        },
    );
    if copy {
        right.copy = Some(selection_hex(&d.hex, &(0..usize::MAX)));
    }
    asked.merge(left);
    asked.merge(right);
}

/// "Connection: TCP a:p → b:q · Packets · Statistics · Stream · Details" (`.notice.info`)
fn connection_note(
    ui: &mut Ui,
    t: Lang,
    key: &AddressPortPair,
    stream: Option<u64>,
    call: Option<&(crate::view::calls::CallKey, String)>,
    asked: &mut Asked,
) {
    widgets::notice_icon(ui, Tone::Info, Some(icons::CONNECTIONS), |ui| {
        ui.label(t.get("packet-connection-label"));
        if widgets::mono_link(ui, &connection_label(key)).clicked() {
            asked.go = Some(Page::Connection(*key));
        }
        widgets::dot(ui);
        let filter = conversation_filter(key);
        if widgets::link(ui, t.get("nav-packets")).clicked() {
            asked.go = Some(
                Jump::Packets {
                    filter: filter.clone(),
                    connections: String::new(),
                }
                .into(),
            );
        }
        widgets::dot(ui);
        if widgets::link(ui, t.get("nav-statistics")).clicked() {
            asked.go = Some(
                Jump::Statistics {
                    filter,
                    connections: String::new(),
                }
                .into(),
            );
        }
        if let Some(n) = stream {
            widgets::dot(ui);
            if widgets::link(ui, t.get("packet-stream")).clicked() {
                asked.go = Some(Page::Stream(n));
            }
        }
        widgets::dot(ui);
        if widgets::link(ui, t.get("packet-connection")).clicked() {
            asked.go = Some(Page::Connection(*key));
        }
        // the call on a line of its own
        if let Some((call, label)) = call {
            ui.end_row();
            let p = palette(ui.ctx());
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(widgets::ICON_SM, widgets::ICON_SM),
                egui::Sense::hover(),
            );
            widgets::paint_icon(ui, icons::PHONE, rect, p.accent);
            ui.label(t.get("packet-call"));
            if widgets::link(ui, label.as_str()).clicked() {
                asked.go = Some(Page::Call(*call));
            }
        }
    });
}

/// The layers (`ul.layers`): a line for each, open ones with their fields and related packets
fn layers(ui: &mut Ui, view: &View, asked: &mut Asked) {
    let p = palette(ui.ctx());
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.add_space(theme::SP1);
    let last = view.d.layers.len().saturating_sub(1);
    for (i, layer) in view.d.layers.iter().enumerate() {
        let open = view.open.get(i).copied().unwrap_or(false);
        layer_line(ui, view, i, layer, open, asked);
        if open {
            for (k, field) in layer.fields.iter().enumerate() {
                field_rows(ui, view, i, &mut vec![k], field, asked);
            }
            for related in view.d.related.iter().filter(|r| r.layer == i) {
                related_line(ui, related, asked);
            }
        }
        if i < last {
            let (line, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, p.border);
        }
    }
    ui.add_space(theme::SP1);
}

/// The line of a layer (`summary`): it opens and closes the layer and marks its bytes
fn layer_line(
    ui: &mut Ui,
    view: &View,
    index: usize,
    layer: &Layer,
    open: bool,
    asked: &mut Asked,
) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let text_left = theme::SP3 + ICON_SM + 6.0;
    let galley = ui.painter().layout(
        layer.title.clone(),
        medium_font(theme::FS_SM),
        p.text,
        (width - text_left - theme::SP3).max(40.0),
    );
    let height = galley.size().y.max(LH) + 12.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &layer.title));
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let on = view
        .chosen
        .is_some_and(|c| c.layer == index && c.field.is_empty());
    if on {
        ui.painter().rect_filled(rect, 0.0, p.selected);
        let bar = Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height()));
        ui.painter().rect_filled(bar, 0.0, p.accent);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, p.hover);
    }
    let icon = if open {
        icons::CHEV_DOWN
    } else {
        icons::CHEV_RIGHT
    };
    let first = rect.top() + 6.0 + LH / 2.0;
    let side = ICON_SM;
    widgets::paint_icon(
        ui,
        icon,
        Rect::from_center_size(
            egui::pos2(rect.left() + theme::SP3 + side / 2.0, first),
            egui::vec2(side, side),
        ),
        p.text_3,
    );
    let at = egui::pos2(
        rect.left() + text_left,
        rect.top() + 6.0 + (LH - row_height(&galley)) / 2.0,
    );
    ui.painter().galley(at.round(), galley, p.text);
    if response.clicked() {
        asked.toggle = Some(index);
        asked.choose = Some(Chosen {
            layer: index,
            field: Vec::new(),
        });
    }
    response.context_menu(|ui| {
        view.item(ui, "copy-layer", asked, || layer_text(layer));
        view.bytes_item(ui, &layer.range, asked);
        view.all_item(ui, asked);
    });
}

/// The height of the first line of a galley
fn row_height(galley: &Galley) -> f32 {
    galley.rows.first().map_or(LH, |row| row.height())
}

/// A field (`.f`): its name and value in two columns, then its details one level further in
fn field_rows(
    ui: &mut Ui,
    view: &View,
    layer: usize,
    path: &mut Vec<usize>,
    field: &Field,
    asked: &mut Asked,
) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let depth = (path.len() - 1) as f32;
    let room = (width - FIELD_INDENT - theme::SP3).max(0.0);
    let key_w = (room - theme::SP3).clamp(KEY_MIN, KEY_MAX);
    let value_w = (room - key_w - theme::SP3).max(20.0);
    let indent = depth * CHILD_INDENT;
    let key = ui.painter().layout(
        field.name.to_string(),
        egui::FontId::proportional(theme::FS_SM),
        p.text_3,
        (key_w - indent).max(20.0),
    );
    let (font, color) = if field.warn {
        (medium_font(theme::FS_SM), p.warn)
    } else {
        (mono_font(theme::FS_SM), p.text)
    };
    let value = ui
        .painter()
        .layout(field.value.clone(), font, color, value_w);
    let height = key.size().y.max(value.size().y).max(LH) + 6.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let line = field_line(field);
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &line));
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let on = view
        .chosen
        .is_some_and(|c| c.layer == layer && c.field == *path);
    if on {
        ui.painter().rect_filled(rect, 0.0, p.hl);
        if view.reveal {
            response.scroll_to_me(None);
        }
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, p.hover);
    }
    let top = rect.top() + 3.0;
    let left = rect.left() + FIELD_INDENT;
    let center = |galley: &Galley| (LH - row_height(galley)).max(0.0) / 2.0;
    ui.painter().galley(
        egui::pos2(left + indent, top + center(&key)).round(),
        key,
        p.text_3,
    );
    let value_y = top + center(&value);
    ui.painter().galley(
        egui::pos2(left + key_w + theme::SP3, value_y).round(),
        value,
        color,
    );
    if response.clicked() && !field.range.is_empty() {
        asked.choose = Some(Chosen {
            layer,
            field: path.clone(),
        });
    }
    response.context_menu(|ui| {
        view.item(ui, "copy-line", asked, || line.clone());
        if !field.value.is_empty() {
            view.item(ui, "copy-value", asked, || field_value(field));
        }
        view.bytes_item(ui, &field.range, asked);
        view.all_item(ui, asked);
    });
    for (k, child) in field.children.iter().enumerate() {
        path.push(k);
        field_rows(ui, view, layer, path, child, asked);
        path.pop();
    }
}

/// A packet that answers or acknowledges this one, or that this one answers (`.rel`):
/// "→ acknowledged in packet 1206 (after 18.4 ms)"
fn related_line(ui: &mut Ui, related: &Related, asked: &mut Asked) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, LH + 6.0), Sense::hover());
    let inner = Rect::from_min_max(
        egui::pos2(rect.left() + FIELD_INDENT, rect.top() + 3.0),
        egui::pos2(rect.right() - theme::SP3, rect.bottom() - 3.0),
    );
    let mut line = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    line.spacing_mut().item_spacing.x = 6.0;
    line.label(
        RichText::new(&related.lead)
            .size(theme::FS_SM)
            .color(p.text_2),
    );
    if widgets::mono_link_sized(&mut line, &related.number.to_string(), theme::FS_SM).clicked() {
        asked.go = Some(Page::Packet(related.number));
    }
    line.label(
        RichText::new(&related.after)
            .size(theme::FS_SM)
            .color(p.text_3),
    );
}

/// The hex dump (`pre.hex`): offset, 16 bytes in two groups of eight and their characters. Only
/// the lines in view are laid out.
fn hex_dump(ui: &mut Ui, view: &View, asked: &mut Asked) {
    let p = palette(ui.ctx());
    let d = view.d;
    let font = mono_font(theme::FS_SM);
    let char_w = ui
        .painter()
        .layout_no_wrap("0".to_owned(), font.clone(), p.text)
        .size()
        .x;
    // offset, two spaces, 16 × 3 + 1, a space, 16 characters
    let line_w = char_w * 72.0;
    let pad = egui::vec2(theme::SP4, theme::SP3);
    ScrollArea::horizontal()
        .id_salt("hex")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let size = egui::vec2(
                (line_w + 2.0 * pad.x).max(ui.available_width()),
                d.hex.len() as f32 * HEX_LH + 2.0 * pad.y,
            );
            let (area, response) = ui.allocate_exact_size(size, Sense::click());
            let top = area.top() + pad.y;
            let left = area.left() + pad.x;
            let hovered_byte = response
                .hover_pos()
                .and_then(|at| byte_at(d, at, left, top, char_w));
            let clip = ui.clip_rect();
            let first = (((clip.top() - top) / HEX_LH).floor().max(0.0)) as usize;
            let last = (((clip.bottom() - top) / HEX_LH).ceil().max(0.0)) as usize;
            for (i, line) in d.hex.iter().enumerate().take(last + 1).skip(first) {
                let y = top + i as f32 * HEX_LH;
                hex_line(ui, line, egui::pos2(left, y), char_w, view, hovered_byte);
            }
            if hovered_byte.is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if response.clicked()
                && let Some(byte) = response
                    .interact_pointer_pos()
                    .and_then(|at| byte_at(d, at, left, top, char_w))
                && let Some(chosen) = smallest_field(&d.layers, byte)
            {
                asked.choose = Some(chosen);
            }
            let range = view.range.clone();
            response.context_menu(|ui| {
                if let Some(range) = range.as_ref().filter(|range| !range.is_empty()) {
                    view.item(ui, "copy-selection-hex", asked, || {
                        selection_hex(&d.hex, range)
                    });
                    view.item(ui, "copy-selection-text", asked, || {
                        selection_text(&d.hex, range)
                    });
                }
                view.item(ui, "copy-dump", asked, || dump_text(&d.hex));
                view.all_item(ui, asked);
            });
        });
    if let Some(note) = &d.hex_cut {
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: theme::SP4 as i8,
                right: theme::SP4 as i8,
                top: 0,
                bottom: theme::SP3 as i8,
            })
            .show(ui, |ui| widgets::hint(ui, note));
    }
}

/// The column of a byte's two hex digits and of its character in a line
fn columns(i: usize) -> (usize, usize) {
    (6 + i * 3 + usize::from(i > 7), 56 + i)
}

/// The byte under a point of the hex dump
fn byte_at(d: &PacketDetail, at: egui::Pos2, left: f32, top: f32, char_w: f32) -> Option<usize> {
    if at.y < top || at.x < left {
        return None;
    }
    let line = d.hex.get(((at.y - top) / HEX_LH) as usize)?;
    let column = ((at.x - left) / char_w) as usize;
    line.bytes.iter().enumerate().find_map(|(i, (byte, _))| {
        let (hex, text) = columns(i);
        (column == hex || column == hex + 1 || column == text).then_some(*byte)
    })
}

/// A line of the hex dump at `at` (its top left); chosen bytes on `--hl`, the one under the
/// pointer on `--hover`
fn hex_line(
    ui: &Ui,
    line: &HexLine,
    at: egui::Pos2,
    char_w: f32,
    view: &View,
    hovered: Option<usize>,
) {
    let p = palette(ui.ctx());
    let font = mono_font(theme::FS_SM);
    let format = |color: Color32| TextFormat {
        font_id: font.clone(),
        color,
        ..TextFormat::default()
    };
    let mut job = LayoutJob::default();
    job.append(&line.offset, 0.0, format(p.text_3));
    job.append("  ", 0.0, format(p.text));
    for (i, (_, hex)) in line.bytes.iter().enumerate() {
        job.append(hex, 0.0, format(p.text));
        job.append(if i == 7 { "  " } else { " " }, 0.0, format(p.text));
    }
    let missing = 16 - line.bytes.len();
    let padding = missing * 3 + usize::from(line.bytes.len() <= 7) + 1;
    job.append(&" ".repeat(padding), 0.0, format(p.text));
    let mut buffer = [0; 4];
    for (_, c) in &line.text {
        // what has no character is a dot in `--text-3` (`.dotc`)
        if *c == '·' {
            job.append(".", 0.0, format(p.text_3));
        } else {
            job.append(c.encode_utf8(&mut buffer), 0.0, format(p.text_2));
        }
    }
    let galley: Arc<Galley> = ui.painter().layout_job(job);
    let y = at.y + (HEX_LH - row_height(&galley)) / 2.0;
    // the marks behind the bytes (`b.on`, `b:hover`), as high as the text
    let mark_h = theme::FS_SM * 1.25;
    let mark = |column: usize, chars: usize, color: Color32| {
        let rect = Rect::from_min_size(
            egui::pos2(
                at.x + column as f32 * char_w,
                at.y + (HEX_LH - mark_h) / 2.0,
            ),
            egui::vec2(chars as f32 * char_w, mark_h),
        );
        ui.painter().rect_filled(rect, CornerRadius::same(2), color);
    };
    for (i, (byte, _)) in line.bytes.iter().enumerate() {
        let color = if view.range.as_ref().is_some_and(|r| r.contains(byte)) {
            p.hl
        } else if hovered == Some(*byte) {
            p.hover
        } else {
            continue;
        };
        let (hex, text) = columns(i);
        mark(hex, 2, color);
        mark(text, 1, color);
    }
    ui.painter()
        .galley(egui::pos2(at.x, y).round(), galley, p.text);
}

/// The bytes of a layer or field
fn chosen_range(layers: &[Layer], chosen: &Chosen) -> Option<Range<usize>> {
    if chosen.field.is_empty() {
        layers.get(chosen.layer).map(|layer| layer.range.clone())
    } else {
        field_at(layers, chosen).map(|field| field.range.clone())
    }
}

/// What copying takes of a chosen layer or field
fn chosen_text(layers: &[Layer], chosen: &Chosen) -> Option<String> {
    if chosen.field.is_empty() {
        layers.get(chosen.layer).map(layer_text)
    } else {
        field_at(layers, chosen).map(field_line)
    }
}

fn field_at<'a>(layers: &'a [Layer], chosen: &Chosen) -> Option<&'a Field> {
    let mut fields = &layers.get(chosen.layer)?.fields;
    let mut found = None;
    for &i in &chosen.field {
        let field = fields.get(i)?;
        found = Some(field);
        fields = &field.children;
    }
    found
}

/// The smallest field around a byte; of fields of the same size the last, the deepest
fn smallest_field(layers: &[Layer], byte: usize) -> Option<Chosen> {
    fn search(
        fields: &[Field],
        byte: usize,
        path: &mut Vec<usize>,
        best: &mut Option<(usize, Vec<usize>)>,
    ) {
        for (i, field) in fields.iter().enumerate() {
            if field.range.contains(&byte) {
                path.push(i);
                if best
                    .as_ref()
                    .is_none_or(|(len, _)| field.range.len() <= *len)
                {
                    *best = Some((field.range.len(), path.clone()));
                }
                search(&field.children, byte, path, best);
                path.pop();
            }
        }
    }
    let mut best: Option<(usize, usize, Vec<usize>)> = None;
    for (index, layer) in layers.iter().enumerate() {
        if !layer.range.contains(&byte) {
            continue;
        }
        let mut found = None;
        search(&layer.fields, byte, &mut Vec::new(), &mut found);
        if let Some((len, path)) = found
            && best.as_ref().is_none_or(|(smallest, ..)| len <= *smallest)
        {
            best = Some((len, index, path));
        }
    }
    best.map(|(_, layer, field)| Chosen { layer, field })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_selects_the_smallest_field() {
        let layers = vec![
            Layer {
                title: "Frame".to_owned(),
                range: 0..20,
                fields: vec![Field::new("Arrival Time", "", 0..0)],
            },
            Layer {
                title: "Layer".to_owned(),
                range: 0..20,
                fields: vec![
                    Field::new("whole", "", 0..20).with_children(vec![
                        Field::new("flags", "", 4..6),
                        Field::new("other", "", 10..12),
                    ]),
                    Field::new("tail", "", 18..20),
                ],
            },
        ];
        let found =
            |byte| smallest_field(&layers, byte).map(|c| chosen_range(&layers, &c).unwrap());
        assert_eq!(found(5), Some(4..6));
        assert_eq!(found(8), Some(0..20));
        assert_eq!(found(19), Some(18..20));
        assert_eq!(found(25), None);
        assert_eq!(
            smallest_field(&layers, 5),
            Some(Chosen {
                layer: 1,
                field: vec![0, 0]
            })
        );
    }

    #[test]
    fn the_columns_of_a_byte_follow_the_dump() {
        // "0000  00 01 02 03 04 05 06 07  08 …" and the characters after two spaces
        assert_eq!(columns(0), (6, 56));
        assert_eq!(columns(7), (27, 63));
        assert_eq!(columns(8), (31, 64));
        assert_eq!(columns(15), (52, 71));
    }
}
