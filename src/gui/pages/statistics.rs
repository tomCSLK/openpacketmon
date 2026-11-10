//! The statistics of the stored packets (`pageStatistics` of the draft): the display filter, five
//! tiles, the traffic of the selection, the protocol hierarchy, the findings of the TCP analysis
//! and response times.

use std::time::Duration;

use eframe::egui::{self, Color32, CornerRadius, FontId, Response, Sense, Stroke, Ui};
use egui::emath::GuiRounding as _;

use crate::gui::chart::{Chart as ChartSpec, chart};
use crate::gui::filter_field::FilterField;
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::pages::packets::{Here, NoteClick, color_keys, selection_note};
use crate::gui::theme::{self, medium_font, mono_font, mono_medium_font, palette};
use crate::gui::widgets::{self, Col, Kind, TableSpec, Tile, Width};
use crate::gui::{App, Page, icons};
use crate::view::Jump;
use crate::view::i18n::Lang;
use crate::view::packets::FILTER_EXAMPLES;
use crate::view::statistics::{FindingRow, HierarchyRow, Statistics, TimeRow, statistics};

/// A row of the tables (`--row-h`)
const ROW_H: f32 = 34.0;
/// Indent of a level of the hierarchy (`.ind`: depth × 18 px)
const INDENT: f32 = 18.0;
/// The room of the percentage after the bar (`.pbar span`: at least 54 px, 8 px after the bar)
const SHARE_W: f32 = 54.0;
const SHARE_GAP: f32 = 8.0;

type Key = (String, String, usize);

#[derive(Default)]
pub struct State {
    shown: Option<String>,
    display: FilterField,
    data: Loader<Key, Statistics>,
}

/// What was clicked on the page
#[derive(Default)]
struct Actions {
    apply: bool,
    example: Option<String>,
    note: Option<NoteClick>,
    jump: Option<Jump>,
}

pub fn show(app: &mut App, ui: &mut Ui, display: &str, connections: &str) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    if !app.live.running {
        head(ui, &t.get("nav-statistics"), None, |_| {});
        if widgets::no_capture(ui, t) {
            app.go(Page::Capture);
        }
        return;
    }
    let state = &mut app.pages.statistics;
    if state.shown.as_deref() != Some(display) {
        state.display.set(display);
        state.shown = Some(display.to_owned());
    }
    let key = (
        display.to_owned(),
        connections.to_owned(),
        app.live.capture_id,
    );
    state.data.poll();
    let (engine, index) = (app.engine.clone(), app.index.clone());
    let job_key = key.clone();
    let live = state.data.value_of(&key).is_some_and(|s| s.live);
    let every = live.then_some(Duration::from_secs(2));
    state.data.load(&ctx, &key, every, move || {
        statistics(t, &engine, &index, &job_key.0, &job_key.1)
    });

    let mut actions = Actions::default();
    let mut show_packets = false;
    head(
        ui,
        &t.get("nav-statistics"),
        Some(&t.get("stats-subtitle")),
        |ui| {
            show_packets = widgets::icon_text_button(
                ui,
                Some(icons::PACKETS),
                &t.get("stats-show-packets"),
                Kind::Secondary,
                false,
            )
            .clicked();
        },
    );
    let engine = app.engine.clone();
    let state = &mut app.pages.statistics;
    let calls = state.data.value().map_or(0, |s| s.calls);
    let tab = crate::gui::pages::calls::tabs(ui, t, 0, calls);
    if let Some(s) = state.data.value_of(&key) {
        if !s.unavailable.is_empty() {
            widgets::notice_text(ui, &s.unavailable);
        }
        actions.note = selection_note(
            ui,
            t,
            s.connection.as_ref(),
            s.connections,
            connections,
            Here::Statistics,
        );
    }
    widgets::bare_card(ui, |ui| {
        actions.apply = state
            .display
            .bar(
                ui,
                t,
                &engine,
                false,
                &t.get("stats-filter-placeholder"),
                &t.get("packets-filter-help"),
            )
            .apply;
        actions.example = state.display.extras(
            ui,
            t,
            false,
            &t.get("packets-filter-help-text"),
            &FILTER_EXAMPLES,
            |ui| color_keys(ui, t),
        );
    });
    match state.data.value_of(&key) {
        Some(s) => match &s.error {
            Some(error) => widgets::error(ui, &error.message),
            None => page(ui, t, s, &mut actions),
        },
        None => widgets::loading(ui, state.data.running_for(), &t.get("gui-loading")),
    }

    let to = |filter: &str, connections: &str| Page::Statistics {
        filter: filter.to_owned(),
        connections: connections.to_owned(),
    };
    if let Some(page) = tab {
        app.go(page);
    } else if show_packets {
        app.go(Page::Packets {
            filter: display.to_owned(),
            connections: connections.to_owned(),
            goto: None,
        });
    } else if let Some(click) = actions.note {
        let page = match click {
            NoteClick::Connection(key) => Page::Connection(key),
            NoteClick::Packets | NoteClick::Statistics => Page::Packets {
                filter: display.to_owned(),
                connections: connections.to_owned(),
                goto: None,
            },
            NoteClick::Stream(n) => Page::Stream(n),
            NoteClick::Connections => Page::Connections {
                filter: connections.to_owned(),
            },
            NoteClick::All => to("", ""),
        };
        app.go(page);
    } else if actions.apply {
        let typed = app.pages.statistics.display.text.trim().to_owned();
        app.go(to(&typed, connections));
    } else if let Some(example) = actions.example {
        app.go(to(&example, connections));
    } else if let Some(jump) = actions.jump {
        app.jump(jump);
    }
}

fn page(ui: &mut Ui, t: Lang, s: &Statistics, actions: &mut Actions) {
    let tiles: Vec<Tile> = s
        .tiles
        .iter()
        .map(|tile| Tile {
            label: tile.label.clone(),
            value: tile.value.clone(),
            unit: tile.unit.clone(),
            sub: tile.sub.clone(),
            ..Tile::default()
        })
        .collect();
    widgets::tiles(ui, &tiles, true);
    widgets::flush_card(
        ui,
        &t.get("stats-chart"),
        |ui| widgets::legend(ui, &t.get("chart-in"), &t.get("chart-out")),
        |ui| {
            if s.chart.points.is_empty() {
                widgets::empty(ui, &t.get("stats-empty"));
                return;
            }
            let names = [t.get("chart-in"), t.get("chart-out")];
            chart(
                ui,
                &ChartSpec {
                    height: 200.0,
                    points: &s.chart.points,
                    ticks: 6,
                    names: [&names[0], &names[1]],
                },
                |i| s.chart.labels.get(i).cloned().unwrap_or_default(),
                |i| s.chart.tips.get(i).cloned().unwrap_or_default(),
                |v| rate(t, v),
            );
        },
    );
    widgets::flush_card(
        ui,
        &t.get("stats-hierarchy"),
        |ui| widgets::card_note(ui, &t.get("stats-hierarchy-hint")),
        |ui| hierarchy(ui, t, &s.hierarchy, actions),
    );
    widgets::grid2(
        ui,
        "statistics",
        |ui, fill| {
            widgets::flush_card(
                ui,
                &t.get("stats-findings"),
                |_| {},
                |ui| {
                    help(ui, &t.get("stats-findings-hint"));
                    findings(ui, t, &s.findings, actions);
                    fill.apply(ui);
                },
            );
        },
        |ui, fill| {
            widgets::flush_card(
                ui,
                &t.get("stats-times"),
                |_| {},
                |ui| {
                    help(ui, &t.get("stats-times-hint"));
                    times(ui, t, &s.times);
                    fill.apply(ui);
                },
            );
        },
    );
}

/// Bytes per second on the axis and under the pointer (`fbytes(v) + '/s'`)
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn rate(t: Lang, v: f64) -> String {
    format!(
        "{}/s",
        crate::view::format::bytes(t, v.max(0.0).round() as u128)
    )
}

/// The text above a table in a card (`p.help`: 13 px in `--text-3`, 12 px above, 16 px at the
/// sides)
fn help(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: theme::SP4 as i8,
            right: theme::SP4 as i8,
            top: theme::SP3 as i8,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                widgets::small(text)
                    .color(p.text_3)
                    .line_height(Some(theme::FS_SM * 1.45)),
            );
        });
}

/// The columns below 900 px leave out what is marked `.hide-sm`
fn narrow(ui: &Ui) -> bool {
    ui.ctx().content_rect().width() < 900.0
}

/// A link that is text until the pointer is over it: then underlined, and in `hover` (`.hier
/// td.name a`, `.findings td.name a`)
fn text_link(ui: &mut Ui, text: &str, font: FontId, color: Color32, hover: Color32) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    let color = if response.hovered() { hover } else { color };
    let at = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter()
        .galley_with_override_text_color(at, galley, color);
    if response.hovered() {
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(1.0, color));
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The protocol hierarchy (`table.hier`): the protocol as a link to its packets, the long name
/// from the fourth level on, packets, their share as a bar, bytes and the average rate
fn hierarchy(ui: &mut Ui, t: Lang, rows: &[HierarchyRow], actions: &mut Actions) {
    let p = palette(ui.ctx());
    let narrow = narrow(ui);
    let mut cols = vec![
        Col::new(t.get("stats-protocol"), Width::Rest(160.0)),
        Col::num(t.get("stats-packets"), Width::Px(120.0)),
        Col::new(t.get("stats-share"), Width::Share(0.28)),
        Col::num(t.get("stats-bytes"), Width::Px(120.0)),
    ];
    if !narrow {
        cols.push(Col::num(t.get("stats-rate"), Width::Px(146.0)));
    }
    widgets::table(
        ui,
        &TableSpec::new("hierarchy", cols, ROW_H, rows.len()),
        |ui, r, c| {
            let row = &rows[r];
            match c {
                0 => {
                    ui.add_space(row.depth as f32 * INDENT);
                    let link = text_link(
                        ui,
                        row.name,
                        FontId::proportional(theme::FS_BASE),
                        p.text,
                        p.accent,
                    )
                    .on_hover_text(row.long);
                    if link.clicked() {
                        actions.jump = Some(row.jump.clone());
                    }
                    if row.depth >= 3 && !narrow {
                        // a space apart (`a` and `span` in one cell)
                        ui.add_space(4.0 - ui.spacing().item_spacing.x);
                        ui.add(
                            egui::Label::new(widgets::small(row.long).color(p.text_3))
                                .selectable(false)
                                .truncate(),
                        );
                    }
                }
                1 => {
                    ui.label(widgets::mono(&row.packets));
                }
                2 => share_bar(ui, row),
                3 => {
                    ui.label(widgets::mono(&row.bytes));
                }
                _ => {
                    ui.label(widgets::mono(&row.rate));
                }
            }
        },
    );
    if rows.is_empty() {
        widgets::empty(ui, &t.get("stats-empty"));
    }
}

/// The share as a bar with the percentage after it (`.pbar`): the bar takes its share of the
/// cell, but leaves room for the percentage (a flex item that shrinks)
#[allow(clippy::cast_possible_truncation)]
fn share_bar(ui: &mut Ui, row: &HierarchyRow) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let galley = ui
        .painter()
        .layout_no_wrap(row.share.clone(), mono_font(theme::FS_XS), p.text_3);
    let text_w = galley.size().x.max(SHARE_W);
    let bar = (width * row.bar as f32 / 100.0)
        .min(width - SHARE_GAP - text_w)
        .max(0.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 8.0), Sense::hover());
    let bar_rect = egui::Rect::from_min_size(rect.min, egui::vec2(bar, 8.0));
    ui.painter()
        .rect_filled(bar_rect, CornerRadius::same(4), p.s1.gamma_multiply(0.75));
    let right = bar_rect.right() + SHARE_GAP + text_w;
    let at = egui::pos2(
        right - galley.size().x,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter()
        .galley(at.round_to_pixels(ui.pixels_per_point()), galley, p.text_3);
}

/// The findings of the TCP analysis (`table.findings`): a finding that packets have is a link to
/// them, its number in the warning color but for keep-alives
fn findings(ui: &mut Ui, t: Lang, rows: &[FindingRow], actions: &mut Actions) {
    let p = palette(ui.ctx());
    let cols = vec![
        Col::new(t.get("stats-finding"), Width::Rest(120.0)),
        Col::num(t.get("stats-count"), Width::Px(100.0)),
    ];
    widgets::table(
        ui,
        &TableSpec::new("findings", cols, ROW_H, rows.len()),
        |ui, r, c| {
            let row = &rows[r];
            if c == 0 {
                if let Some(jump) = &row.jump {
                    if text_link(ui, &row.label, medium_font(theme::FS_BASE), p.text, p.text)
                        .clicked()
                    {
                        actions.jump = Some(jump.clone());
                    }
                } else {
                    let color = if row.has { p.text } else { p.text_3 };
                    ui.label(egui::RichText::new(&row.label).color(color));
                }
            } else {
                let (font, color) = if !row.has {
                    (mono_font(theme::FS_SM), p.text_3)
                } else if row.warn {
                    (mono_medium_font(theme::FS_SM), p.warn)
                } else {
                    (mono_medium_font(theme::FS_SM), p.text)
                };
                ui.label(egui::RichText::new(&row.count).font(font).color(color));
            }
        },
    );
}

/// The response times: how often, on average and at most; rows without a measurement in
/// `--text-3` (`tr.off`)
fn times(ui: &mut Ui, t: Lang, rows: &[TimeRow]) {
    let p = palette(ui.ctx());
    let cols = vec![
        Col::new(t.get("stats-measurement"), Width::Rest(120.0)),
        Col::num(t.get("stats-count"), Width::Px(100.0)),
        Col::num(t.get("stats-average"), Width::Px(117.0)),
        Col::num(t.get("stats-max"), Width::Px(130.0)),
    ];
    widgets::table(
        ui,
        &TableSpec::new("times", cols, ROW_H, rows.len()),
        |ui, r, c| {
            let row = &rows[r];
            let color = if row.off { p.text_3 } else { p.text };
            let text = match c {
                0 => {
                    ui.label(egui::RichText::new(&row.label).color(color));
                    return;
                }
                1 => &row.count,
                2 => &row.average,
                _ => &row.max,
            };
            ui.label(widgets::mono(text).color(color));
        },
    );
}
