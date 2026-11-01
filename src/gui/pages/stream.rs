//! "Follow stream": a connection as the two sides of a conversation.

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Ui};

use crate::gui::App;
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::view::Jump;
use crate::view::packets::conversation_filter;
use crate::view::stream::{Stream, StreamSide, StreamView, stream};

type Key = (u64, StreamView, StreamSide, usize);

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
    let p = palette(&ctx);
    let state = &mut app.pages.stream;
    let key = (number, state.view, state.side, app.live.capture_id);
    state.content.poll();
    let (engine, view, side) = (app.engine.clone(), state.view, state.side);
    state.content.load(&ctx, &key, None, move || {
        stream(t, &engine, number, view, side)
    });
    let mut packets = false;
    let mut views = None;
    let mut sides = None;
    let content = app.pages.stream.content.value_of(&key);
    let known = content.is_some();
    let title = content
        .and_then(Option::as_ref)
        .map_or_else(String::new, |s| s.title.clone());
    head(app, ui, &t.get("stream-title"), Some(&title), |ui| {
        packets = widgets::button(ui, t.get("stream-packets"), Kind::Secondary).clicked();
    });
    if !known {
        let waited = app.pages.stream.content.running_for();
        widgets::loading(ui, waited, &t.get("gui-loading"));
        return;
    }
    let Some(Some(s)) = app.pages.stream.content.value_of(&key) else {
        widgets::notice_text(ui, &t.get("gui-packet-gone"));
        return;
    };
    if s.headers_only {
        widgets::notice_text(ui, &t.get("stream-headers-only"));
    }
    if s.cut {
        widgets::notice_text(ui, &t.get("stream-cut"));
    }
    let client_line = format!("{} → {}: {}", s.client, s.server, s.client_counts);
    let server_line = format!("{} → {}: {}", s.server, s.client, s.server_counts);
    let view_at = usize::from(view == StreamView::Hex);
    let side_at = match side {
        StreamSide::Both => 0,
        StreamSide::Client => 1,
        StreamSide::Server => 2,
    };
    widgets::flush_card(
        ui,
        "",
        |ui| {
            let labels = [
                t.get("stream-both"),
                t.get("stream-client-side"),
                t.get("stream-server-side"),
            ];
            sides = widgets::segmented(ui, &labels, side_at);
            let labels = [t.get("stream-text"), "Hex".to_owned()];
            views = widgets::segmented(ui, &labels, view_at);
        },
        |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    legend(ui, p.accent_soft, &client_line);
                    legend(ui, p.down_soft, &server_line);
                });
            if s.chunks.is_empty() {
                widgets::empty(ui, &t.get("stream-empty"));
                return;
            }
            ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                Frame::new().inner_margin(Margin::same(10)).show(ui, |ui| {
                    for chunk in &s.chunks {
                        if let Some(gap) = &chunk.gap {
                            ui.label(RichText::new(gap).small().color(p.warn));
                        }
                        let (fill, fg) = if chunk.client {
                            (p.accent_soft, p.text)
                        } else {
                            (p.down_soft, p.text)
                        };
                        Frame::new()
                            .fill(fill)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&chunk.text).monospace().color(fg),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .on_hover_text(t.get1(
                                    "packet-title",
                                    "number",
                                    chunk.packet,
                                ));
                            });
                    }
                });
            });
        },
    );
    let conversation = s.connection;
    if packets {
        app.jump(Jump::Packets {
            filter: conversation_filter(&conversation),
            connections: String::new(),
        });
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

/// A direction and its color in the list below
fn legend(ui: &mut Ui, color: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::same(3), color);
        ui.add(egui::Label::new(RichText::new(text).small()).truncate());
    });
}
