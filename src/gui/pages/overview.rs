//! The overview (`docs/entwurf/uebersicht.html`): tiles, what stands out, the traffic of the last
//! five minutes and the top hosts, services and programs.

use std::time::Duration;

use eframe::egui::{self, Align, Layout, RichText, Sense, Ui, UiBuilder};

use crate::config::favorite::Favorite;
use crate::engine::Engine;
use crate::gui::chart::{Chart, chart};
use crate::gui::load::Loader;
use crate::gui::pages::{entries, head_rich};
use crate::gui::theme::{self, medium_font, palette};
use crate::gui::widgets::{self, Tile};
use crate::gui::{App, Page, SettingsPanel, icons};
use crate::networking::types::data_representation::DataUnit;
use crate::view::entries::EntryRow;
use crate::view::i18n::Lang;
use crate::view::overview::{AttentionKind, Overview, overview};
use crate::view::{Jump, format};

/// Seconds of the chart
const CHART_SECONDS: u32 = 300;
/// Height of a row of the top lists (`.top .trow`)
const TOP_ROW_H: f32 = 45.0;

struct Data {
    view: Overview,
    /// Unix second of the first sample
    start: i64,
    /// Second, incoming and outgoing bytes, incoming and outgoing packets
    samples: Vec<[f64; 5]>,
    /// The capture still receives packets
    live: bool,
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
    let view = overview(t, engine);
    let state = engine.lock();
    let history = &state.traffic_history;
    let last = history.ticks().saturating_sub(1);
    let from = last.saturating_sub(CHART_SECONDS - 1);
    Data {
        view,
        start: history.first_packet_timestamp().secs(),
        samples: history
            .samples()
            .iter()
            .filter(|s| s.second >= from)
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
        live: !history.no_more_packets() && !state.frozen,
    }
}

/// A rate in the unit of the page (`rateFmt`)
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // amounts of a chart
pub fn rate(t: Lang, unit: DataUnit, value: f64) -> String {
    let value = value.max(0.0);
    let amount = match unit {
        DataUnit::Packets if value < 10.0 => {
            format!("{value:.1}").replace('.', &t.decimal_separator().to_string())
        }
        _ => format::amount(t, unit, value.round() as u128),
    };
    format!("{amount}/s")
}

/// Incoming and outgoing of each sample in the unit of the page
fn points(samples: &[[f64; 5]], unit: DataUnit) -> Vec<[f64; 2]> {
    let (column, factor) = match unit {
        DataUnit::Packets => (3, 1.0),
        DataUnit::Bytes => (1, 1.0),
        DataUnit::Bits => (1, 8.0),
    };
    samples
        .iter()
        .map(|s| [s[column] * factor, s[column + 1] * factor])
        .collect()
}

/// The time of day of a second
fn clock(second: i64) -> String {
    jiff::Timestamp::from_second(second).map_or_else(
        |_| String::new(),
        |time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%H:%M:%S")
                .to_string()
        },
    )
}

/// The rate now: the mean of the three seconds before the last one, which is not complete yet
fn rate_now(points: &[[f64; 2]]) -> [f64; 2] {
    let n = points.len();
    let last = &points[n.saturating_sub(4)..n.saturating_sub(1)];
    let count = last.len().max(1) as f64;
    [
        last.iter().map(|p| p[0]).sum::<f64>() / count,
        last.iter().map(|p| p[1]).sum::<f64>() / count,
    ]
}

fn tiles(ui: &mut Ui, t: Lang, d: &Overview) -> Option<Page> {
    let p = palette(ui.ctx());
    let tl = &d.tiles;
    let items = [
        Tile::amount(
            t.get("tile-total"),
            &tl.total,
            t.get1("tile-packets", "count", tl.packets.clone()),
        )
        .icon(icons::ACTIVITY, None),
        Tile::amount(
            t.get("tile-in"),
            &tl.incoming,
            t.get1("tile-share", "percent", tl.in_share),
        )
        .icon(icons::ARROW_DOWN, Some(p.s1)),
        Tile::amount(
            t.get("tile-out"),
            &tl.outgoing,
            t.get1("tile-share", "percent", tl.out_share),
        )
        .icon(icons::ARROW_UP, Some(p.s2)),
        Tile {
            link: true,
            ..Tile::amount(
                t.get("tile-connections"),
                &tl.connections,
                t.get("tile-connections-sub"),
            )
            .icon(icons::CONNECTIONS, None)
        },
        Tile {
            link: true,
            ..Tile::amount(
                t.get("tile-hosts"),
                &tl.hosts,
                t.get1("tile-hosts-sub", "count", tl.favorite_hosts),
            )
            .icon(icons::HOSTS, None)
        },
        Tile {
            warn: tl.dropped.is_some_and(|d| d > 0),
            ..Tile::amount(
                t.get("tile-dropped"),
                &tl.dropped
                    .map_or_else(|| "–".to_owned(), |d| format::count(t, d)),
                t.get("tile-dropped-sub"),
            )
            .icon(icons::ALERT, None)
        },
    ];
    match widgets::tiles(ui, &items, false) {
        Some(3) => Some(Page::Connections {
            filter: String::new(),
        }),
        Some(4) => Some(Page::Entries(Favorite::Host)),
        _ => None,
    }
}

fn attention(ui: &mut Ui, t: Lang, d: &Overview) -> Option<Jump> {
    if d.attention.is_empty() {
        return None;
    }
    let mut jump = None;
    widgets::flush_card(
        ui,
        &t.get("attn-title"),
        |ui| widgets::card_note(ui, &t.get("attn-hint")),
        |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let last = d.attention.len() - 1;
            for (i, row) in d.attention.iter().enumerate() {
                let (warn, icon, button) = match row.kind {
                    AttentionKind::Tcp => (true, icons::ALERT, "nav-statistics"),
                    AttentionKind::Favorites => (false, icons::STAR, "nav-notifications"),
                    AttentionKind::PlainWeb => (false, icons::INFO, "nav-packets"),
                    AttentionKind::Call => (true, icons::PHONE, "attn-call-button"),
                };
                if widgets::attention_row(
                    ui,
                    warn,
                    icon,
                    &row.title,
                    &row.text,
                    &t.get(button),
                    i == last,
                ) {
                    jump = Some(row.jump.clone());
                }
            }
        },
    );
    jump
}

/// "now ↓ 38,8 KB/s ↑ 10,4 KB/s" in the head of the chart (`.rate`)
fn rate_line(ui: &mut Ui, t: Lang, unit: DataUnit, now: [f64; 2]) {
    let p = palette(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    let plain = |color| egui::TextFormat {
        font_id: egui::FontId::proportional(theme::FS_SM),
        color,
        ..Default::default()
    };
    let bold = egui::TextFormat {
        font_id: theme::mono_medium_font(theme::FS_SM),
        color: p.text_3,
        ..Default::default()
    };
    job.append(&t.get("rate-now"), 0.0, plain(p.text_3));
    job.append(" ↓", 0.0, plain(p.s1));
    job.append(" ", 0.0, plain(p.text_3));
    job.append(&rate(t, unit, now[0]), 0.0, bold.clone());
    job.append(" ↑", 0.0, plain(p.s2));
    job.append(" ", 0.0, plain(p.text_3));
    job.append(&rate(t, unit, now[1]), 0.0, bold);
    ui.add(egui::Label::new(job).selectable(false));
}

fn traffic(ui: &mut Ui, t: Lang, unit: DataUnit, d: &Data) {
    let points = points(&d.samples, unit);
    let now = rate_now(&points);
    widgets::flush_card(
        ui,
        &t.get("chart-title"),
        |ui| {
            if !points.is_empty() {
                rate_line(ui, t, unit, now);
            }
            ui.add_space(theme::SP4 - theme::SP3);
            widgets::legend(ui, &t.get("chart-in"), &t.get("chart-out"));
        },
        |ui| {
            if points.is_empty() {
                widgets::empty(ui, &t.get("chart-empty"));
                return;
            }
            let seconds: Vec<i64> = d.samples.iter().map(|s| d.start + s[0] as i64).collect();
            let last = seconds.len() - 1;
            let live = d.live;
            let names = [t.get("chart-in"), t.get("chart-out")];
            chart(
                ui,
                &Chart {
                    height: 220.0,
                    points: &points,
                    ticks: 5,
                    names: [&names[0], &names[1]],
                },
                |i| {
                    if live && i == last {
                        t.get("chart-now")
                    } else {
                        clock(seconds[i])
                    }
                },
                |i| t.get1("chart-tip-time", "time", clock(seconds[i])),
                |v| rate(t, unit, v),
            );
        },
    );
}

/// What was clicked in a top list
enum TopClick {
    All,
    Action(entries::Action),
}

/// A row of a top list (`.top .trow`): star, name with a short note, the amount, and below them
/// the traffic as a bar
fn top_row(ui: &mut Ui, t: Lang, row: &EntryRow, max: f32, last: bool) -> Option<entries::Action> {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, TOP_ROW_H), Sense::hover());
    if ui.rect_contains_pointer(rect) {
        ui.painter().rect_filled(rect, 0.0, p.hover);
    }
    if !last {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, p.border),
        );
    }
    let mut action = None;
    let inner = egui::Rect::from_min_max(
        rect.min + egui::vec2(theme::SP2, 6.0),
        rect.max - egui::vec2(theme::SP3, 6.0),
    );
    let mut line = ui.new_child(
        UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                inner.min,
                egui::vec2(inner.width(), 24.0),
            ))
            .layout(Layout::left_to_right(Align::Center)),
    );
    line.spacing_mut().item_spacing.x = theme::SP2;
    match &row.key {
        Some(key) => {
            let text = t.get(if row.favorite {
                "favorite-remove"
            } else {
                "favorite-add"
            });
            if widgets::star(&mut line, row.favorite, &text).clicked() {
                action = Some(entries::Action::Favorite(key.clone(), !row.favorite));
            }
        }
        None => {
            line.add_space(24.0);
        }
    }
    let value_w = 96.0;
    let name_w = line.available_width() - value_w - theme::SP2;
    line.allocate_ui_with_layout(
        egui::vec2(name_w, 24.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_width(name_w);
            ui.spacing_mut().item_spacing.x = 6.0;
            let name = RichText::new(&row.label)
                .font(medium_font(theme::FS_BASE))
                .color(p.text);
            if widgets::link_text(ui, name)
                .on_hover_text(&row.label)
                .clicked()
            {
                action = Some(entries::Action::Open(row.jump.clone()));
            }
            if !row.short.is_empty() {
                ui.add(
                    egui::Label::new(RichText::new(&row.short).size(theme::FS_XS).color(p.text_3))
                        .selectable(false)
                        .truncate(),
                );
            }
        },
    );
    line.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.label(widgets::mono(&row.total));
    });
    let mut bar = ui.new_child(UiBuilder::new().max_rect(egui::Rect::from_min_size(
        egui::pos2(inner.left() + 24.0 + theme::SP2, inner.top() + 24.0 + 3.0),
        egui::vec2(inner.width() - 24.0 - theme::SP2, 6.0),
    )));
    let total = (row.bar_in + row.bar_out) / 100.0;
    let width = bar.available_width();
    widgets::tbar(
        &mut bar,
        width,
        6.0,
        total / max.max(0.000_1),
        row.bar_in,
        row.bar_out,
    );
    action
}

fn top_card(
    ui: &mut Ui,
    t: Lang,
    icon: egui::ImageSource<'static>,
    title: &str,
    rows: &[EntryRow],
    empty: &str,
    height: f32,
) -> (Option<TopClick>, f32) {
    let p = palette(ui.ctx());
    let mut all = false;
    let mut click = None;
    let mut natural = 0.0;
    widgets::icon_card(
        ui,
        Some(icon),
        title,
        |ui| {
            all = widgets::link_sized(ui, &t.get("show-all"), theme::FS_SM, p.accent).clicked();
        },
        |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let top = ui.cursor().top();
            if rows.is_empty() {
                widgets::empty(ui, empty);
            }
            // the largest entry fills the bar
            let max = rows.first().map_or(1.0, |r| (r.bar_in + r.bar_out) / 100.0);
            let last = rows.len().saturating_sub(1);
            for (i, row) in rows.iter().enumerate() {
                if let Some(action) = top_row(ui, t, row, max, i == last) {
                    click = Some(TopClick::Action(action));
                }
            }
            // the cards of a row are equally high (`.grid3`)
            natural = ui.cursor().top() - top;
            if natural < height {
                ui.add_space(height - natural);
            }
        },
    );
    (if all { Some(TopClick::All) } else { click }, natural)
}

fn tops(ui: &mut Ui, t: Lang, d: &Overview) -> (Option<Page>, Option<entries::Action>) {
    let empty = t.get("entries-empty");
    // programs are only known on an adapter; the card stays, with the reason
    let (programs, no_programs) = match &d.programs {
        Some(rows) => (rows.as_slice(), empty.clone()),
        None => (&[][..], t.get("programs-unsupported")),
    };
    let lists: [(
        egui::ImageSource<'static>,
        String,
        &[EntryRow],
        String,
        Favorite,
    ); 3] = [
        (
            icons::HOSTS,
            t.get("top-hosts"),
            &d.hosts,
            empty.clone(),
            Favorite::Host,
        ),
        (
            icons::SERVICES,
            t.get("top-services"),
            &d.services,
            empty,
            Favorite::Service,
        ),
        (
            icons::PROGRAMS,
            t.get("top-programs"),
            programs,
            no_programs,
            Favorite::Program,
        ),
    ];
    // as high as the highest card, known from the frame before
    let memory = widgets::once("overview-top-height");
    let rows = lists.iter().map(|l| l.2.len()).max().unwrap_or(0);
    let height = (rows as f32 * TOP_ROW_H).max(ui.data(|d| d.get_temp(memory)).unwrap_or(0.0));
    let mut page = None;
    let mut action = None;
    let mut handle = |click: Option<TopClick>, kind: Favorite| match click {
        Some(TopClick::All) => page = Some(Page::Entries(kind)),
        Some(TopClick::Action(a)) => action = Some(a),
        None => {}
    };
    // three columns, one below 1180 px (`.grid3`)
    if ui.ctx().content_rect().width() < 1180.0 {
        for (icon, title, rows, empty, kind) in &lists {
            let (click, _) = top_card(ui, t, icon.clone(), title, rows, empty, 0.0);
            handle(click, *kind);
        }
    } else {
        let gap = theme::SP4;
        let width = ui.available_width();
        let w = (width - 2.0 * gap) / 3.0;
        let top = ui.cursor().top();
        let left = ui.cursor().left();
        let mut bottom = top;
        let mut highest: f32 = 0.0;
        for (i, (icon, title, rows, empty, kind)) in lists.iter().enumerate() {
            let mut column = ui.new_child(
                UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::pos2(left + i as f32 * (w + gap), top),
                        egui::vec2(w, f32::INFINITY),
                    ))
                    .layout(Layout::top_down(Align::Min)),
            );
            let (click, natural) =
                top_card(&mut column, t, icon.clone(), title, rows, empty, height);
            highest = highest.max(natural);
            handle(click, *kind);
            bottom = bottom.max(column.min_rect().bottom());
        }
        ui.advance_cursor_after_rect(egui::Rect::from_min_max(
            egui::pos2(left, top),
            egui::pos2(left + width, bottom),
        ));
        let before: f32 = ui.data(|d| d.get_temp(memory)).unwrap_or(0.0);
        if (highest - before).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(memory, highest));
            ui.ctx().request_repaint();
        }
    }
    (page, action)
}

/// The text below the title: where and since when the capture runs
fn subtitle(ui: &mut Ui, t: Lang, d: &Overview) {
    let Some(c) = &d.capture else {
        return;
    };
    let p = palette(ui.ctx());
    let id = match c.source {
        "file" => "ov-sub-file",
        "ipfix" => "ov-sub-ipfix",
        _ => "ov-sub-adapter",
    };
    let text = t.fmt(
        id,
        &[
            ("name", c.name.clone().into()),
            ("time", c.since.clone().into()),
            ("duration", c.duration.clone().into()),
        ],
    );
    widgets::code_line(ui, &text, &c.name, theme::FS_BASE, p.text_3);
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let unit = app.live.unit;
    let key = (app.live.capture_id, unit);
    let state = &mut app.pages.overview;
    state.data.poll();
    let engine = app.engine.clone();
    let every = app.live.running.then_some(Duration::from_secs(1));
    state.data.load(&ctx, &key, every, move || read(t, &engine));

    let running = app.live.running;
    let mut chosen_unit = None;
    let index = entries::unit_index(unit);
    let data = app.pages.overview.data.value();
    let title = t.get("nav-overview");
    head_rich(
        ui,
        &title,
        |ui| {
            if let Some(d) = data {
                subtitle(ui, t, &d.view);
            }
        },
        |ui| {
            if running {
                chosen_unit = widgets::segmented_sized(ui, &entries::unit_labels(t), index, true);
            }
        },
    );

    let mut go = None;
    let mut jump = None;
    let mut action = None;
    if app.live.status.geo_missing && running {
        widgets::notice(ui, |ui| {
            ui.label(t.get("geo-missing"));
            if widgets::link(ui, t.get("geo-missing-link")).clicked() {
                go = Some(Page::Settings(SettingsPanel::Geo));
            }
        });
    }
    if !running {
        if widgets::no_capture(ui, t) {
            go = Some(Page::Capture);
        }
    } else if let Some(d) = app.pages.overview.data.value() {
        go = go.or(tiles(ui, t, &d.view));
        jump = attention(ui, t, &d.view);
        traffic(ui, t, unit, d);
        let (page, clicked) = tops(ui, t, &d.view);
        go = go.or(page);
        action = clicked;
    }

    if let Some(index) = chosen_unit {
        entries::set_unit(app, index);
    }
    if let Some(action) = action {
        action.run(app);
    } else if let Some(jump) = jump {
        app.jump(jump);
    } else if let Some(page) = go {
        app.go(page);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chart_takes_the_columns_of_the_unit() {
        let samples = [[3.0, 10.0, 20.0, 1.0, 2.0]];
        assert_eq!(points(&samples, DataUnit::Bytes), [[10.0, 20.0]]);
        assert_eq!(points(&samples, DataUnit::Bits), [[80.0, 160.0]]);
        assert_eq!(points(&samples, DataUnit::Packets), [[1.0, 2.0]]);
    }

    #[test]
    fn the_rate_now_leaves_out_the_last_second() {
        let points = [[1.0, 0.0], [2.0, 0.0], [3.0, 0.0], [4.0, 0.0], [100.0, 0.0]];
        assert!((rate_now(&points)[0] - 3.0).abs() < f64::EPSILON);
        assert!((rate_now(&[])[0]).abs() < f64::EPSILON);
    }

    #[test]
    fn small_packet_rates_keep_a_decimal() {
        assert_eq!(rate(Lang::De, DataUnit::Packets, 2.5), "2,5/s");
        assert_eq!(rate(Lang::De, DataUnit::Packets, 12.4), "12/s");
        assert_eq!(rate(Lang::De, DataUnit::Bytes, 75_000.0), "75,0 KB/s");
    }
}
