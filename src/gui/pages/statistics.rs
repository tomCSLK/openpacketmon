//! The statistics of the stored packets.

use std::time::Duration;

use eframe::egui::{self, Align, Layout, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};

use crate::gui::chart::{Series, chart};
use crate::gui::filter_field::{FilterField, button_widths};
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, Page};
use crate::networking::types::data_representation::DataUnit;
use crate::utils::types::timestamp::Timestamp;
use crate::view::Jump;
use crate::view::i18n::Lang;
use crate::view::statistics::{Chart, HierarchyRow, Statistics, statistics};

/// The rows of the small tables of findings and times
const ROW_H: f32 = 24.0;
/// The width of their numbers
const COLUMN: f32 = 72.0;

type Key = (String, String, usize);

#[derive(Default)]
pub struct State {
    shown: Option<(String, String)>,
    display: FilterField,
    connections: FilterField,
    data: Loader<Key, Statistics>,
    /// The chart shows packets instead of bits
    packets: bool,
}

#[derive(Default)]
struct Actions {
    apply: bool,
    reset: bool,
    capture: bool,
    to_connections: bool,
    connection: Option<crate::networking::types::address_port_pair::AddressPortPair>,
    jump: Option<Jump>,
}

fn line(c: &Chart, values: &[f64], factor: f64) -> Vec<[f64; 2]> {
    #[allow(clippy::cast_precision_loss)] // the points of a chart
    values
        .iter()
        .enumerate()
        .map(|(i, v)| [(i as i64 * c.step) as f64, v * factor])
        .collect()
}

pub fn show(app: &mut App, ui: &mut Ui, display: &str, connections: &str) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let p = palette(&ctx);
    let filters = (display.to_owned(), connections.to_owned());
    let state = &mut app.pages.statistics;
    if state.shown.as_ref() != Some(&filters) {
        state.display.set(display);
        state.connections.set(connections);
        state.shown = Some(filters.clone());
    }
    let key = (filters.0, filters.1, app.live.capture_id);
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
        app,
        ui,
        &t.get("nav-statistics"),
        Some(&t.get("stats-subtitle")),
        |ui| {
            show_packets =
                widgets::button(ui, t.get("stats-show-packets"), Kind::Secondary).clicked();
        },
    );
    let engine = app.engine.clone();
    let state = &mut app.pages.statistics;
    if let Some(s) = state.data.value() {
        if !s.running {
            widgets::notice(ui, |ui| {
                ui.label(t.get("no-capture"));
                actions.capture = widgets::link(ui, t.get("no-capture-start")).clicked();
            });
        } else if !s.unavailable.is_empty() {
            widgets::notice_text(ui, &s.unavailable);
        }
        if let Some((ends, key)) = &s.connection {
            widgets::notice(ui, |ui| {
                ui.label(t.get("filter-connection"));
                widgets::code(ui, ends);
                if widgets::link(ui, t.get("filter-connection-open")).clicked() {
                    actions.connection = Some(*key);
                }
            });
        }
    }
    if !connections.is_empty() {
        widgets::notice(ui, |ui| {
            ui.label(t.get("filter-connections"));
            actions.to_connections = widgets::link(ui, t.get("filter-connections-open")).clicked();
        });
    }

    let display_problem = state.display.problem(t, false).map(str::to_owned);
    let connections_problem = state.connections.problem(t, true).map(str::to_owned);
    let with_connections = !connections.is_empty();
    ui.horizontal(|ui| {
        let reset = !display.is_empty() || with_connections;
        let fields = if with_connections { 2 } else { 1 };
        let width = widgets::field_width(ui, fields, &button_widths(ui, t, reset));
        if with_connections {
            let out = state.connections.show(
                ui,
                t,
                &engine,
                true,
                &t.get("connections-filter-placeholder"),
                Some(connections_problem.is_none()),
                width,
            );
            actions.apply |= out.apply;
        }
        let valid = (!state.display.text.trim().is_empty()).then_some(display_problem.is_none());
        let out = state.display.show(
            ui,
            t,
            &engine,
            false,
            &t.get("stats-filter-placeholder"),
            valid,
            width,
        );
        actions.apply |= out.apply;
        actions.apply |= widgets::button(ui, t.get("apply"), Kind::Primary).clicked();
        if reset {
            actions.reset = widgets::button(ui, t.get("filter-reset"), Kind::Secondary).clicked();
        }
    });
    if let Some(problem) = display_problem.or(connections_problem) {
        ui.label(RichText::new(problem).small().color(p.down));
    }

    let packets_unit = &mut state.packets;
    if let Some(s) = state.data.value_of(&key) {
        if let Some(error) = &s.error {
            widgets::error(ui, &error.message);
        } else {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                page(ui, t, s, packets_unit, &mut actions);
            });
        }
    }

    if show_packets {
        app.go(Page::Packets {
            filter: display.to_owned(),
            connections: connections.to_owned(),
            goto: None,
        });
    } else if actions.capture {
        app.go(Page::Capture);
    } else if actions.to_connections {
        app.go(Page::Connections {
            filter: connections.to_owned(),
        });
    } else if let Some(key) = actions.connection {
        app.go(Page::Connection(key));
    } else if actions.reset {
        app.go(Page::Statistics {
            filter: String::new(),
            connections: String::new(),
        });
    } else if actions.apply {
        let state = &app.pages.statistics;
        let page = Page::Statistics {
            filter: state.display.text.trim().to_owned(),
            connections: state.connections.text.trim().to_owned(),
        };
        app.go(page);
    } else if let Some(jump) = actions.jump {
        app.jump(jump);
    }
}

fn page(ui: &mut Ui, t: Lang, s: &Statistics, show_packets: &mut bool, actions: &mut Actions) {
    let p = palette(ui.ctx());
    widgets::tiles(ui, &s.summary);
    let mut unit = None;
    widgets::card(
        ui,
        &t.get("stats-chart"),
        |ui| {
            let labels = [t.get("unit-bits"), t.get("unit-packets")];
            unit = widgets::segmented(ui, &labels, usize::from(*show_packets));
        },
        |ui| {
            if s.chart.packets.is_empty() {
                widgets::empty(ui, &t.get("chart-empty"));
                return;
            }
            let start = Timestamp::new(s.chart.start, 0);
            if *show_packets {
                let series = [Series {
                    name: t.get("unit-packets"),
                    color: p.accent,
                    points: line(&s.chart, &s.chart.packets, 1.0),
                }];
                chart(ui, "stats-chart", 220.0, start, &series, move |v| {
                    super::overview::rate(t, DataUnit::Packets, v)
                });
            } else {
                let series = [Series {
                    name: t.get("unit-bits"),
                    color: p.accent,
                    points: line(&s.chart, &s.chart.bytes, 8.0),
                }];
                chart(ui, "stats-chart", 220.0, start, &series, move |v| {
                    super::overview::rate(t, DataUnit::Bits, v)
                });
            }
        },
    );
    if let Some(i) = unit {
        *show_packets = i == 1;
    }
    widgets::flush_card(
        ui,
        &t.get("stats-hierarchy"),
        |_| {},
        |ui| {
            if s.empty {
                widgets::empty(ui, &t.get("stats-empty"));
                return;
            }
            let height = 26.0 + s.hierarchy.len() as f32 * 26.0 + 6.0;
            widgets::table_area(ui, Some(height), |ui| {
                hierarchy(ui, t, &s.hierarchy, actions);
            });
        },
    );
    widgets::hint(ui, &t.get("stats-hierarchy-hint"));
    ui.columns(2, |c| {
        widgets::card(
            &mut c[0],
            &t.get("stats-findings"),
            |_| {},
            |ui| {
                egui::Grid::new("findings")
                    .num_columns(2)
                    .striped(false)
                    .min_row_height(ROW_H)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for f in &s.findings {
                            let color = if f.problem && f.jump.is_some() {
                                p.down
                            } else {
                                p.text
                            };
                            match &f.jump {
                                Some(jump) => {
                                    if widgets::link(ui, f.label.as_str()).clicked() {
                                        actions.jump = Some(jump.clone());
                                    }
                                }
                                None => {
                                    ui.label(&f.label);
                                }
                            }
                            widgets::right_cell(
                                ui,
                                COLUMN,
                                RichText::new(&f.count).monospace().color(color),
                            );
                            ui.end_row();
                        }
                    });
                widgets::hint(ui, &t.get("stats-findings-hint"));
            },
        );
        widgets::card(
            &mut c[1],
            &t.get("stats-times"),
            |_| {},
            |ui| {
                let grid = egui::Grid::new("times")
                    .num_columns(4)
                    .min_row_height(ROW_H)
                    .spacing([24.0, 4.0]);
                grid.show(ui, |ui| {
                    ui.label("");
                    for head in ["stats-count", "stats-average", "stats-max"] {
                        widgets::right_cell(
                            ui,
                            COLUMN,
                            RichText::new(t.get(head)).small().color(p.muted),
                        );
                    }
                    ui.end_row();
                    for r in &s.times {
                        match &r.jump {
                            Some(jump) => {
                                if widgets::link(ui, r.label.as_str()).clicked() {
                                    actions.jump = Some(jump.clone());
                                }
                            }
                            None => {
                                ui.label(&r.label);
                            }
                        }
                        widgets::right_cell(ui, COLUMN, RichText::new(&r.count).monospace());
                        widgets::right_cell(ui, COLUMN, RichText::new(&r.average).monospace());
                        widgets::right_cell(ui, COLUMN, RichText::new(&r.max).monospace());
                        ui.end_row();
                    }
                });
                widgets::hint(ui, &t.get("stats-times-hint"));
            },
        );
    });
}

fn hierarchy(ui: &mut Ui, t: Lang, rows: &[HierarchyRow], actions: &mut Actions) {
    let p = palette(ui.ctx());
    let heads = [
        (t.get("conn-protocol"), false),
        (t.get("stats-packets"), true),
        ("%".to_owned(), true),
        (String::new(), false),
        (t.get("stats-bytes"), true),
        ("%".to_owned(), true),
    ];
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
    TableBuilder::new(ui)
        .id_salt("hierarchy")
        .vscroll(false)
        .auto_shrink(false)
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::remainder().at_least(160.0).clip(true))
        .column(Column::exact(96.0))
        .column(Column::exact(68.0))
        .column(Column::exact(160.0))
        .column(Column::exact(96.0))
        .column(Column::exact(68.0))
        .header(26.0, |mut header| {
            for (text, right) in &heads {
                header.col(|ui| {
                    let text = RichText::new(text).small().color(p.muted);
                    right_or_left(ui, text, *right);
                });
            }
        })
        .body(|body| {
            body.rows(26.0, rows.len(), |mut row| {
                let r = &rows[row.index()];
                row.col(|ui| {
                    ui.add_space(r.depth as f32 * 16.0);
                    if widgets::link(ui, RichText::new(r.name).monospace()).clicked() {
                        actions.jump = Some(r.jump.clone());
                    }
                });
                let mono = |s: &str| RichText::new(s).monospace();
                row.col(|ui| right_or_left(ui, mono(&r.packets), true));
                row.col(|ui| right_or_left(ui, mono(&r.packets_share).color(p.muted), true));
                row.col(|ui| {
                    let width = ui.available_width();
                    #[allow(clippy::cast_possible_truncation)] // percent
                    widgets::share_bar(ui, width, r.bar as f32, 0.0, p.accent, p.accent);
                });
                row.col(|ui| right_or_left(ui, mono(&r.bytes), true));
                row.col(|ui| right_or_left(ui, mono(&r.bytes_share).color(p.muted), true));
            });
        });
}

fn right_or_left(ui: &mut Ui, text: RichText, right: bool) {
    if right {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(text));
    } else {
        ui.label(text);
    }
}
