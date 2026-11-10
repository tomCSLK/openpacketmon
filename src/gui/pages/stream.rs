//! "Follow stream" (`pageStream` of the draft): the payload of a connection as the two sides of a
//! conversation, as text or hex.

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Align, CornerRadius, Rect, RichText, Sense, TextFormat, Ui};

use crate::gui::load::Loader;
use crate::gui::pages::detail_head;
use crate::gui::theme::{self, mono_font, palette};
use crate::gui::widgets::{self, Kind, Tone};
use crate::gui::{App, Page, icons};
use crate::view::packets::conversation_filter;
use crate::view::stream::{Stream, StreamSide, StreamView, stream};

type Key = (u64, StreamView, StreamSide, usize);

/// Line height of the stream (`.stream`: 1.55)
const LH: f32 = theme::FS_SM * 1.55;

pub struct State {
    view: StreamView,
    side: StreamSide,
    content: Loader<Key, Option<Stream>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            view: StreamView::Text,
            side: StreamSide::Both,
            content: Loader::default(),
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui, number: u64) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let state = &mut app.pages.stream;
    let key = (number, state.view, state.side, app.live.capture_id);
    state.content.poll();
    let (engine, view, side) = (app.engine.clone(), state.view, state.side);
    state.content.load(&ctx, &key, None, move || {
        stream(t, &engine, number, view, side)
    });
    let content = app.pages.stream.content.value_of(&key);
    let known = content.is_some();
    let about = content
        .and_then(Option::as_ref)
        .map(|s| (s.label.clone(), s.service.clone(), s.connection));
    // the way back leads to the packets of the connection (`pakete.html?c=`)
    let list = Page::Packets {
        filter: about
            .as_ref()
            .map(|(.., key)| conversation_filter(key))
            .unwrap_or_default(),
        connections: String::new(),
        goto: None,
    };
    let mut packets = false;
    let mut connection = false;
    let mut views = None;
    let mut sides = None;
    let view_at = usize::from(view == StreamView::Hex);
    let side_at = match side {
        StreamSide::Both => 0,
        StreamSide::Client => 1,
        StreamSide::Server => 2,
    };
    detail_head(
        app,
        ui,
        &[(t.get("nav-packets"), list)],
        &t.get("packet-stream"),
        |ui| widgets::title(ui, &t.get("stream-title")),
        |ui| {
            let Some((label, service, _)) = &about else {
                return;
            };
            let p = palette(ui.ctx());
            // the first row of a wrapped line is as high as `interact_size`
            ui.spacing_mut().interact_size.y = widgets::LH_BASE;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                connection = widgets::mono_link_sized(ui, label, theme::FS_BASE * 0.92).clicked();
                if let Some(service) = service {
                    ui.label(RichText::new(format!(" · {service}")).color(p.text_3));
                }
            });
        },
        |ui| {
            if about.is_none() {
                return;
            }
            packets = widgets::icon_text_button(
                ui,
                Some(icons::PACKETS),
                &t.get("stream-packets"),
                Kind::Secondary,
                false,
            )
            .clicked();
            let labels = [t.get("stream-text"), "Hex".to_owned()];
            views = widgets::segmented(ui, &labels, view_at);
            let labels = [
                t.get("stream-both"),
                t.get("stream-client-side"),
                t.get("stream-server-side"),
            ];
            sides = widgets::segmented(ui, &labels, side_at);
        },
    );
    if !known {
        let waited = app.pages.stream.content.running_for();
        widgets::loading(ui, waited, &t.get("gui-loading"));
        return;
    }
    let Some(Some(s)) = app.pages.stream.content.value_of(&key) else {
        widgets::notice_text(ui, &t.get("gui-packet-gone"));
        return;
    };
    if s.tls {
        widgets::notice_icon(ui, Tone::Info, None, |ui| {
            ui.label(t.get("stream-tls"));
        });
    }
    if s.headers_only {
        widgets::notice_text(ui, &t.get("stream-headers-only"));
    }
    if s.cut {
        widgets::notice_text(ui, &t.get("stream-cut"));
    }
    widgets::custom_card(
        ui,
        |ui| {
            ui.spacing_mut().item_spacing.x = theme::SP4;
            legend(ui, true, &t.get("stream-client"), &s.client);
            legend(ui, false, &t.get("stream-server"), &s.server);
        },
        |ui| widgets::card_note(ui, &s.counts),
        |ui| body(ui, &t.get("stream-empty"), s),
    );
    let conversation = s.connection;
    if packets {
        app.go(Page::Packets {
            filter: conversation_filter(&conversation),
            connections: String::new(),
            goto: None,
        });
    } else if connection {
        app.go(Page::Connection(conversation));
    }
    let state = &mut app.pages.stream;
    if let Some(i) = views {
        state.view = if i == 0 {
            StreamView::Text
        } else {
            StreamView::Hex
        };
    }
    if let Some(i) = sides {
        state.side = [StreamSide::Both, StreamSide::Client, StreamSide::Server][i];
    }
}

/// A side in the head of the card (`.legend`): its mark, "Client" and the address
fn legend(ui: &mut Ui, client: bool, name: &str, address: &str) {
    let p = palette(ui.ctx());
    let (line, fill) = if client {
        (p.cli, p.cli_bg)
    } else {
        (p.srv, p.srv_bg)
    };
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, fill);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
            0.0,
            line,
        );
        let mut job = LayoutJob::default();
        let plain = TextFormat {
            font_id: egui::FontId::proportional(theme::FS_SM),
            color: p.text_2,
            ..TextFormat::default()
        };
        job.append(&format!("{name} "), 0.0, plain.clone());
        job.append(
            address,
            0.0,
            TextFormat {
                font_id: mono_font(theme::FS_SM * 0.92),
                ..plain
            },
        );
        ui.add(egui::Label::new(job).selectable(false));
    });
}

/// The payload (`pre.stream`): a block for what each side sends in a row (`.c`, `.s`), with the
/// bytes that are missing before it (`.gap`)
fn body(ui: &mut Ui, empty: &str, s: &Stream) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            if s.chunks.is_empty() {
                gap_line(ui, empty);
                return;
            }
            let width = ui.available_width();
            for chunk in &s.chunks {
                if let Some(gap) = &chunk.gap {
                    gap_line(ui, gap);
                }
                let (line, fill, color) = if chunk.client {
                    (p.cli, p.cli_bg, p.cli)
                } else {
                    (p.srv, p.srv_bg, p.srv)
                };
                let mut job = LayoutJob::single_section(
                    chunk.text.clone(),
                    TextFormat {
                        font_id: mono_font(theme::FS_SM),
                        color,
                        line_height: Some(LH),
                        valign: Align::Center,
                        ..TextFormat::default()
                    },
                );
                job.wrap.max_width = (width - 3.0 - 2.0 * theme::SP2).max(40.0);
                job.wrap.break_anywhere = false;
                let galley = ui.painter().layout_job(job);
                let height = galley.size().y.max(LH) + 4.0;
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
                if ui.is_rect_visible(rect) {
                    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
                        0.0,
                        line,
                    );
                    let at = egui::pos2(rect.left() + 3.0 + theme::SP2, rect.top() + 2.0);
                    ui.painter().galley(at, galley, color);
                }
                response.on_hover_text(&chunk.packets);
                ui.add_space(2.0);
            }
        });
}

/// A line in the stream that is no payload (`.gap`): italic, in `--text-3`
fn gap_line(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(theme::SP2 as i8, 4))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(theme::FS_SM)
                    .italics()
                    .color(p.text_3)
                    .line_height(Some(LH)),
            );
        });
}
