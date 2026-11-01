//! The connections: the list in pages, and the details of one connection.

use std::time::Duration;

use eframe::egui::{self, Align, ComboBox, Layout, RichText, ScrollArea, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use crate::config::favorite::FavoriteKey;
use crate::engine::Engine;
use crate::gui::filter_field::{FilterField, button_widths, filter_help};
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, Page, icons, local_time};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::report::types::sort_type::SortType;
use crate::view::Jump;
use crate::view::connections::{
    ConnectionDetail, ConnectionList, FILTER_EXAMPLES, Fact, FactValue, ListRequest,
    connection_detail, connection_list, filter_field_names, latency_allowed,
};
use crate::view::i18n::Lang;

const SORTS: [SortType; 3] = [SortType::Neutral, SortType::Descending, SortType::Ascending];
/// The width of the choice of the order: its longest text fits, or it would grow with the text
const SORT_WIDTH: f32 = 190.0;

#[derive(Clone, PartialEq)]
struct Key {
    filter: String,
    sort: SortType,
    page: Option<usize>,
    size: Option<usize>,
    capture: usize,
    unit: crate::networking::types::data_representation::DataUnit,
}

#[derive(Default)]
pub struct State {
    shown: Option<String>,
    field: FilterField,
    sort: usize,
    page: Option<usize>,
    size: Option<usize>,
    list: Loader<Key, ConnectionList>,
    detail: Loader<(AddressPortPair, usize), Option<ConnectionDetail>>,
    page_text: String,
}

#[cfg(test)]
impl State {
    pub fn set_sort(&mut self, sort: usize) {
        self.sort = sort;
    }
}

#[derive(Default)]
struct Actions {
    apply: bool,
    reset: bool,
    statistics: bool,
    capture: bool,
    example: Option<String>,
    open: Option<AddressPortPair>,
}

pub fn show(app: &mut App, ui: &mut Ui, filter: &str) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let state = &mut app.pages.connections;
    if state.shown.as_deref() != Some(filter) {
        state.shown = Some(filter.to_owned());
        state.field.set(filter);
        state.page = None;
    }
    let key = Key {
        filter: filter.to_owned(),
        sort: SORTS[state.sort],
        page: state.page,
        size: state.size,
        capture: app.live.capture_id,
        unit: app.live.unit,
    };
    state.list.poll();
    let engine = app.engine.clone();
    let job_key = key.clone();
    let every = app.live.running.then_some(Duration::from_secs(1));
    state.list.load(&ctx, &key, every, move || {
        connection_list(
            t,
            &engine.lock(),
            &ListRequest {
                filter: &job_key.filter,
                sort: job_key.sort,
                page: job_key.page,
                size: job_key.size,
            },
        )
    });

    let mut actions = Actions::default();
    head(
        app,
        ui,
        &t.get("nav-connections"),
        Some(&t.get("connections-subtitle")),
        |ui| {
            actions.statistics = widgets::button(ui, t.get("nav-statistics"), Kind::Secondary)
                .on_hover_text(t.get("connections-statistics-hint"))
                .clicked();
        },
    );
    if !app.live.running {
        widgets::notice(ui, |ui| {
            ui.label(t.get("no-capture"));
            actions.capture = widgets::link(ui, t.get("no-capture-start")).clicked();
        });
    }
    let engine = app.engine.clone();
    let state = &mut app.pages.connections;
    let problem = state.field.problem(t, true).map(str::to_owned);
    let filtered = !filter.is_empty();
    ui.horizontal(|ui| {
        let reset = filtered || state.sort != 0;
        let mut beside = button_widths(ui, t, reset);
        beside.push(SORT_WIDTH);
        let room = widgets::field_width(ui, 1, &beside);
        let valid = (!state.field.text.trim().is_empty()).then_some(problem.is_none());
        let out = state.field.show(
            ui,
            t,
            &engine,
            true,
            &t.get("connections-filter-placeholder"),
            valid,
            room,
        );
        actions.apply |= out.apply;
        let labels = [t.get("sort-recent"), t.get("sort-desc"), t.get("sort-asc")];
        ComboBox::from_id_salt("connections-sort")
            .width(SORT_WIDTH)
            .selected_text(labels[state.sort].clone())
            .show_index(ui, &mut state.sort, labels.len(), |i| labels[i].clone());
        actions.apply |= widgets::button(ui, t.get("apply"), Kind::Primary).clicked();
        if reset {
            actions.reset = widgets::button(ui, t.get("filter-reset"), Kind::Secondary).clicked();
        }
    });
    if let Some(problem) = &problem {
        let p = palette(ui.ctx());
        ui.label(RichText::new(problem).small().color(p.down));
    }
    actions.example = filter_help(
        ui,
        &t.get("connections-filter-help"),
        &t.get("connections-filter-help-text"),
        &FILTER_EXAMPLES,
        &[
            (
                t.get("packets-filter-protocols"),
                crate::filter::CONNECTION_PROTOCOLS.join(" "),
            ),
            (
                t.get("packets-filter-fields"),
                filter_field_names().join(" "),
            ),
        ],
        |_| {},
    );
    if let Some(list) = state.list.value() {
        if let Some(error) = &list.error {
            widgets::error(ui, error);
        }
        let title = if list.total == 0 {
            t.get("connections-none")
        } else {
            format!(
                "{} {}",
                t.get2(
                    "connections-range",
                    "from",
                    list.first_row,
                    "to",
                    list.last_row
                ),
                t.get2(
                    "connections-of",
                    "total",
                    list.total,
                    "data",
                    list.matching_data.as_str()
                )
            )
        };
        let page_text = &mut state.page_text;
        let (mut new_size, mut new_page) = (None, None);
        widgets::flush_card(
            ui,
            &title,
            |ui| {
                let chosen = list
                    .sizes
                    .iter()
                    .find(|(_, on)| *on)
                    .map_or(50, |(n, _)| *n);
                ComboBox::from_id_salt("rows-per-page")
                    .selected_text(chosen.to_string())
                    .show_ui(ui, |ui| {
                        for (n, on) in &list.sizes {
                            if ui.selectable_label(*on, n.to_string()).clicked() {
                                new_size = Some(*n);
                            }
                        }
                    });
                ui.label(t.get("rows-per-page"));
            },
            |ui| {
                if list.rows.is_empty() {
                    let text = if filtered {
                        "connections-none-filtered"
                    } else {
                        "connections-empty"
                    };
                    widgets::empty(ui, &t.get(text));
                    return;
                }
                let room = ui.available_height() - if list.pages > 1 { 46.0 } else { 0.0 };
                actions.open = widgets::table_area(ui, Some(room), |ui| table(ui, t, list));
                if list.pages > 1 {
                    new_page = widgets::pager(ui, t, list.page, list.pages, page_text);
                }
            },
        );
        if new_size.is_some() {
            state.size = new_size;
            state.page = None;
        } else if new_page.is_some() {
            state.page = new_page;
        }
    }

    if let Some(example) = actions.example {
        app.go(Page::Connections { filter: example });
    } else if actions.statistics {
        app.go(Page::Statistics {
            filter: String::new(),
            connections: filter.to_owned(),
        });
    } else if actions.capture {
        app.go(Page::Capture);
    } else if actions.reset {
        app.pages.connections.sort = 0;
        app.go(Page::Connections {
            filter: String::new(),
        });
    } else if actions.apply {
        let filter = app.pages.connections.field.text.trim().to_owned();
        app.go(Page::Connections { filter });
    } else if let Some(key) = actions.open {
        app.go(Page::Connection(key));
    }
}

fn table(ui: &mut Ui, t: Lang, list: &ConnectionList) -> Option<AddressPortPair> {
    let p = palette(ui.ctx());
    let mut builder = TableBuilder::new(ui)
        .id_salt("connections")
        .sense(Sense::click())
        .resizable(true)
        .auto_shrink(false)
        .animate_scrolling(false)
        .max_scroll_height(f32::INFINITY)
        .min_scrolled_height(0.0)
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::initial(170.0).at_least(90.0).clip(true))
        .column(Column::exact(18.0))
        .column(Column::initial(170.0).at_least(90.0).clip(true))
        .column(Column::exact(56.0))
        .column(Column::initial(80.0).at_least(40.0).clip(true));
    if list.programs {
        builder = builder.column(Column::initial(100.0).at_least(40.0).clip(true));
    }
    builder = builder
        .column(Column::remainder().at_least(90.0).clip(true))
        .column(Column::exact(68.0))
        .column(Column::exact(84.0))
        .column(Column::exact(140.0));
    let mut heads = vec![
        (t.get("conn-source"), false),
        (String::new(), false),
        (t.get("conn-destination"), false),
        (t.get("conn-protocol"), false),
        (t.get("conn-service"), false),
    ];
    if list.programs {
        heads.push((t.get("conn-program"), false));
    }
    heads.extend([
        (t.get("conn-host"), false),
        (t.get("conn-packets"), true),
        (t.get("conn-data"), true),
        (t.get("conn-last"), true),
    ]);
    let mut open = None;
    builder
        .header(26.0, |mut header| {
            for (text, right) in &heads {
                header.col(|ui| {
                    let text = RichText::new(text).small().color(p.muted);
                    cell(ui, text, *right);
                });
            }
        })
        .body(|body| {
            body.rows(28.0, list.rows.len(), |mut row| {
                let r = &list.rows[row.index()];
                let mono = |s: &str| RichText::new(s).monospace();
                let quiet = |s: &str, none: bool| {
                    if none {
                        RichText::new("–").color(p.faint)
                    } else {
                        RichText::new(s)
                    }
                };
                row.col(|ui| cell(ui, mono(&r.src), false));
                row.col(|ui| {
                    let (color, hint) = if r.outgoing {
                        (p.accent, t.get("direction-out"))
                    } else {
                        (p.muted, t.get("direction-in"))
                    };
                    ui.label(RichText::new("→").color(color))
                        .on_hover_text(hint);
                });
                row.col(|ui| cell(ui, mono(&r.dst), false));
                row.col(|ui| cell(ui, RichText::new(&r.proto), false));
                row.col(|ui| cell(ui, quiet(&r.service, r.service_na), false));
                if list.programs {
                    row.col(|ui| cell(ui, quiet(&r.program, r.program_na), false));
                }
                row.col(|ui| {
                    let host = if r.host_sub.is_empty() {
                        r.host.clone()
                    } else {
                        format!("{} · {}", r.host, r.host_sub)
                    };
                    cell(ui, RichText::new(host), false);
                });
                row.col(|ui| cell(ui, mono(&r.packets), true));
                row.col(|ui| cell(ui, mono(&r.data), true));
                row.col(|ui| cell(ui, RichText::new(local_time(t, r.last)).small(), true));
                if row.response().clicked()
                    && let Jump::Connection(key) = &r.jump
                {
                    open = Some(*key);
                }
            });
        });
    open
}

fn cell(ui: &mut Ui, text: RichText, right: bool) {
    let label = egui::Label::new(text).truncate().selectable(false);
    if right {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.add(label));
    } else {
        ui.add(label);
    }
}

fn read_detail(t: Lang, engine: &Engine, key: &AddressPortPair) -> Option<ConnectionDetail> {
    let state = engine.lock();
    let info = state.info_traffic.map.get(key)?;
    Some(connection_detail(t, &state, key, info))
}

fn fact_lines(
    ui: &mut Ui,
    t: Lang,
    id: &str,
    facts: &[Fact],
    jump: &mut Option<Jump>,
    before: impl FnOnce(&mut Ui),
) {
    widgets::facts(ui, id, |ui| {
        before(ui);
        for f in facts {
            widgets::fact_label(ui, &f.label);
            ui.horizontal_wrapped(|ui| {
                match &f.value {
                    FactValue::Time(ts) => ui.label(local_time(t, *ts)),
                    FactValue::Text(text) => ui.label(text),
                };
                for l in &f.links {
                    let link = widgets::link(ui, l.label.as_str());
                    if link.on_hover_text(&l.title).clicked() {
                        *jump = Some(l.jump.clone());
                    }
                }
            });
            ui.end_row();
        }
    });
}

pub fn show_connection(app: &mut App, ui: &mut Ui, key: &AddressPortPair) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let p = palette(&ctx);
    let load_key = (*key, app.live.capture_id);
    let state = &mut app.pages.connections;
    state.detail.poll();
    let engine = app.engine.clone();
    let job_key = *key;
    let every = app.live.running.then_some(Duration::from_secs(1));
    state.detail.load(&ctx, &load_key, every, move || {
        read_detail(t, &engine, &job_key)
    });
    let title = state
        .detail
        .value_of(&load_key)
        .and_then(Option::as_ref)
        .map_or_else(|| t.get("connection-title"), |d| d.title.clone());
    head(app, ui, &title, Some(&t.get("connection-title")), |_| {});
    let Some(found) = app.pages.connections.detail.value_of(&load_key) else {
        return;
    };
    let Some(d) = found else {
        widgets::notice_text(ui, &t.get("gui-connection-gone"));
        return;
    };
    let mut jump = None;
    let mut favorite = None;
    let mut measure = false;
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.columns(2, |columns| {
            widgets::card(
                &mut columns[0],
                &t.get("connection-title"),
                |_| {},
                |ui| {
                    fact_lines(ui, t, "connection-facts", &d.facts, &mut jump, |_| {});
                    if !d.message_types.is_empty() {
                        ui.add_space(8.0);
                        ui.strong(t.get("conn-message-types"));
                        widgets::facts(ui, "message-types", |ui| {
                            for (name, n) in &d.message_types {
                                widgets::fact_label(ui, name);
                                ui.label(n);
                                ui.end_row();
                            }
                        });
                    }
                },
            );
            widgets::card(
                &mut columns[1],
                &t.get("connection-remote"),
                |_| {},
                |ui| {
                    let mut address_jump = None;
                    fact_lines(ui, t, "remote-facts", &d.host_facts, &mut jump, |ui| {
                        widgets::fact_label(ui, &t.get("host-address"));
                        ui.horizontal(|ui| {
                            widgets::code(ui, &d.remote.to_string());
                            let (icon, color, text) = if d.remote_favorite {
                                (icons::STAR_ON, p.warn, t.get("favorite-address-remove"))
                            } else {
                                (icons::STAR, p.faint, t.get("favorite-address-add"))
                            };
                            if widgets::icon_button(ui, icon, color, &text).clicked() {
                                favorite = Some(!d.remote_favorite);
                            }
                            let link = widgets::link(ui, t.get("connection-list"));
                            if link
                                .on_hover_text(t.get("connection-address-connections"))
                                .clicked()
                            {
                                address_jump = Some(d.address_jump.clone());
                            }
                        });
                        ui.end_row();
                    });
                    if address_jump.is_some() {
                        jump = address_jump;
                    }
                    if d.latency_supported || d.latency.is_some() {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            widgets::fact_label(ui, &t.get("latency"));
                            ui.label(d.latency.as_deref().unwrap_or("–"));
                            if d.latency_supported {
                                if d.measuring {
                                    widgets::disabled_button(ui, t.get("latency-measuring"));
                                } else {
                                    measure = widgets::small_button(
                                        ui,
                                        t.get("latency-measure"),
                                        Kind::Secondary,
                                    )
                                    .clicked();
                                }
                            }
                        });
                    }
                },
            );
        });
    });
    let remote = d.remote;
    let favorite_now = favorite;
    if measure && latency_allowed(&app.engine.lock(), remote) {
        app.engine.measure_latency(remote);
    }
    if let Some(add) = favorite_now {
        app.engine
            .lock()
            .add_or_remove_favorite(&FavoriteKey::Address(remote), add);
        app.engine.store_conf();
        app.pages.connections.detail.reset();
    }
    if let Some(jump) = jump {
        app.jump(jump);
    }
}
