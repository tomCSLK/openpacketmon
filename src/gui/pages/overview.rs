//! The overview: totals, the live chart and the top lists.

use std::time::Duration;

use eframe::egui::{RichText, ScrollArea, Ui};

use crate::config::favorite::Favorite;
use crate::engine::Engine;
use crate::gui::chart::{Series, chart};
use crate::gui::load::Loader;
use crate::gui::pages::packets::fill;
use crate::gui::pages::{capture, entries, head};
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, Page};
use crate::networking::types::data_representation::DataUnit;
use crate::utils::types::timestamp::Timestamp;
use crate::view::entries::EntryRow;
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::overview::{Overview, overview};

struct Data {
    view: Overview,
    start: Timestamp,
    /// Second, incoming and outgoing bytes, incoming and outgoing packets
    samples: Vec<[f64; 5]>,
}

#[derive(Default)]
pub struct State {
    data: Loader<(usize, DataUnit), Data>,
}

impl State {
    pub fn reset(&mut self) {
        self.data.reset();
    }
}

#[allow(clippy::cast_precision_loss)] // the points of a chart
fn read(t: Lang, engine: &Engine) -> Data {
    let state = engine.lock();
    let history = &state.traffic_history;
    Data {
        view: overview(t, &state),
        start: history.first_packet_timestamp(),
        samples: history
            .samples()
            .iter()
            .map(|s| {
                [
                    f64::from(s.second),
                    s.in_bytes as f64,
                    s.out_bytes as f64,
                    s.in_packets as f64,
                    s.out_packets as f64,
                ]
            })
            .collect(),
    }
}

/// A rate in the unit of the page
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // amounts of a chart
pub fn rate(t: Lang, unit: DataUnit, value: f64) -> String {
    format!("{}/s", format::amount(t, unit, value.max(0.0) as u128))
}

/// One column of the samples in the unit of the page
fn points(samples: &[[f64; 5]], unit: DataUnit, outgoing: bool) -> Vec<[f64; 2]> {
    let (column, factor) = match unit {
        DataUnit::Packets => (3, 1.0),
        DataUnit::Bytes => (1, 1.0),
        DataUnit::Bits => (1, 8.0),
    };
    let column = column + usize::from(outgoing);
    samples.iter().map(|s| [s[0], s[column] * factor]).collect()
}

fn top_card(
    ui: &mut Ui,
    id: &str,
    title: &str,
    t: Lang,
    rows: &[EntryRow],
    all: &mut bool,
    action: &mut Option<entries::Action>,
) {
    widgets::flush_card(
        ui,
        title,
        |ui| {
            *all = widgets::link(ui, t.get("show-all")).clicked();
        },
        |ui| {
            if rows.is_empty() {
                widgets::empty(ui, &t.get("entries-empty"));
            } else {
                let height = 26.0 + rows.len() as f32 * entries::ROW_H + 6.0;
                *action = widgets::table_area(ui, Some(height), |ui| {
                    entries::table(ui, id, t, rows, true, false)
                });
            }
        },
    );
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let p = palette(&ctx);
    let unit = app.live.unit;
    let key = (app.live.capture_id, unit);
    let state = &mut app.pages.overview;
    state.data.poll();
    let engine = app.engine.clone();
    let every = app.live.running.then_some(Duration::from_secs(1));
    state.data.load(&ctx, &key, every, move || read(t, &engine));

    let capture_head = state.data.value().and_then(|d| d.view.capture.as_ref());
    let subtitle = capture_head.map_or_else(
        || t.get("no-capture"),
        |c| format!("{}: {} · {}", c.kind, c.name, c.state),
    );
    let (running, frozen, finished) =
        capture_head.map_or((false, false, false), |c| (true, c.frozen, c.finished));
    let (mut pause, mut stop, mut chosen_unit) = (false, false, None);
    let index = entries::unit_index(unit);
    head(app, ui, &t.get("nav-overview"), Some(&subtitle), |ui| {
        if running {
            stop = widgets::button(ui, t.get("capture-stop"), Kind::Danger)
                .on_hover_text(t.get("capture-stop-hint"))
                .clicked();
            if !finished {
                let text = if frozen {
                    "capture-resume"
                } else {
                    "capture-pause"
                };
                pause = widgets::button(ui, t.get(text), Kind::Secondary).clicked();
            }
        }
        chosen_unit = widgets::segmented(ui, &entries::unit_labels(t), index);
    });

    let mut go = None;
    let mut action = None;
    if app.live.status.geo_missing {
        widgets::notice(ui, |ui| {
            ui.label(t.get("geo-missing"));
            if widgets::link(ui, t.get("geo-missing-link")).clicked() {
                go = Some(Page::Settings);
            }
        });
    }
    if let Some(d) = app.pages.overview.data.value() {
        if running {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let tiles: Vec<_> = d
                    .view
                    .tiles
                    .iter()
                    .map(|tile| (tile.label.clone(), tile.value.clone()))
                    .collect();
                widgets::tiles(ui, &tiles);
                let incoming = points(&d.samples, unit, false);
                let outgoing = points(&d.samples, unit, true);
                let now = match (incoming.last(), outgoing.last()) {
                    (Some(i), Some(o)) => fill(
                        &t.get("rate-now"),
                        &[rate(t, unit, i[1]), rate(t, unit, o[1])],
                    ),
                    _ => String::new(),
                };
                widgets::card(
                    ui,
                    &t.get("chart-title"),
                    |ui| {
                        ui.label(RichText::new(now).small().color(p.muted));
                    },
                    |ui| {
                        if d.samples.is_empty() {
                            widgets::empty(ui, &t.get("chart-empty"));
                            return;
                        }
                        let series = [
                            Series {
                                name: t.get("chart-in"),
                                color: p.accent,
                                points: incoming,
                            },
                            Series {
                                name: t.get("chart-out"),
                                color: p.ok,
                                points: outgoing,
                            },
                        ];
                        chart(ui, "live-chart", 240.0, d.start, &series, move |v| {
                            rate(t, unit, v)
                        });
                    },
                );
                let programs = d.view.programs.as_deref();
                let columns = 2 + usize::from(programs.is_some());
                let (mut hosts, mut services, mut all_programs) = (false, false, false);
                ui.columns(columns, |c| {
                    top_card(
                        &mut c[0],
                        "top-hosts",
                        &t.get("top-hosts"),
                        t,
                        &d.view.hosts,
                        &mut hosts,
                        &mut action,
                    );
                    top_card(
                        &mut c[1],
                        "top-services",
                        &t.get("top-services"),
                        t,
                        &d.view.services,
                        &mut services,
                        &mut action,
                    );
                    if let Some(rows) = programs {
                        top_card(
                            &mut c[2],
                            "top-programs",
                            &t.get("top-programs"),
                            t,
                            rows,
                            &mut all_programs,
                            &mut action,
                        );
                    }
                });
                if hosts {
                    go = Some(Page::Entries(Favorite::Host));
                } else if services {
                    go = Some(Page::Entries(Favorite::Service));
                } else if all_programs {
                    go = Some(Page::Entries(Favorite::Program));
                }
            });
        } else {
            widgets::notice(ui, |ui| {
                ui.label(t.get("no-capture"));
                if widgets::link(ui, t.get("no-capture-start")).clicked() {
                    go = Some(Page::Capture);
                }
            });
        }
    }

    if let Some(index) = chosen_unit {
        entries::set_unit(app, index);
    }
    if pause {
        capture::pause(app);
    }
    if stop {
        capture::stop(app);
    } else if let Some(action) = action {
        action.run(app);
    } else if let Some(page) = go {
        app.go(page);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chart_takes_the_column_of_the_unit() {
        let samples = [[3.0, 10.0, 20.0, 1.0, 2.0]];
        assert_eq!(points(&samples, DataUnit::Bytes, false), [[3.0, 10.0]]);
        assert_eq!(points(&samples, DataUnit::Bits, true), [[3.0, 160.0]]);
        assert_eq!(points(&samples, DataUnit::Packets, true), [[3.0, 2.0]]);
    }
}
