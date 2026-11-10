//! The connections: the list in pages, and the details of one connection, as in the draft
//! (`docs/entwurf/verbindungen.html`, `verbindung.html`).

use std::time::Duration;

use eframe::egui::{self, Align, Layout, Rect, RichText, Sense, Ui};

use crate::config::favorite::FavoriteKey;
use crate::engine::Engine;
use crate::gui::filter_field::FilterField;
use crate::gui::load::Loader;
use crate::gui::pages::{detail_head, entries, head};
use crate::gui::theme::{self, palette};
use crate::gui::widgets::{self, Col, Kind, TableSpec, Width};
use crate::gui::{App, Page, icons};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::data_representation::DataUnit;
use crate::report::types::sort_type::SortType;
use crate::utils::types::timestamp::Timestamp;
use crate::view::Jump;
use crate::view::connections::{
    ConnectionDetail, ConnectionList, FILTER_EXAMPLES, ListRequest, connection_detail,
    connection_list, latency_allowed,
};
use crate::view::i18n::Lang;

/// The orders of the list: largest, smallest, newest first
const SORTS: [SortType; 3] = [SortType::Descending, SortType::Ascending, SortType::Neutral];

#[derive(Clone, PartialEq)]
struct Key {
    filter: String,
    favorites: bool,
    sort: SortType,
    page: Option<usize>,
    capture: usize,
    unit: DataUnit,
}

#[derive(Default)]
pub struct State {
    shown: Option<String>,
    field: FilterField,
    sort: usize,
    page: Option<usize>,
    favorites: bool,
    list: Loader<Key, ConnectionList>,
    detail: Loader<(AddressPortPair, usize), Option<ConnectionDetail>>,
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
    packets: bool,
    statistics: bool,
    capture: bool,
    example: Option<String>,
    open: Option<AddressPortPair>,
    favorite: Option<(Vec<FavoriteKey>, bool)>,
    unit: Option<usize>,
}

/// The filter of the list: the one typed, and `favorite` while "Only favorites" is on
fn effective(filter: &str, favorites: bool) -> String {
    match (filter.trim(), favorites) {
        (f, false) => f.to_owned(),
        ("", true) => "favorite".to_owned(),
        (f, true) => format!("({f}) and favorite"),
    }
}

pub fn show(app: &mut App, ui: &mut Ui, filter: &str) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let mut actions = Actions::default();
    head(
        ui,
        &t.get("nav-connections"),
        Some(&t.get("connections-subtitle")),
        |ui| {
            if app.live.running {
                actions.statistics = widgets::icon_text_button(
                    ui,
                    Some(icons::STATISTICS),
                    &t.get("nav-statistics"),
                    Kind::Secondary,
                    false,
                )
                .on_hover_text(t.get("connections-statistics-hint"))
                .clicked();
                actions.packets = widgets::icon_text_button(
                    ui,
                    Some(icons::PACKETS),
                    &t.get("nav-packets"),
                    Kind::Secondary,
                    false,
                )
                .clicked();
            }
        },
    );
    if !app.live.running {
        if widgets::no_capture(ui, t) {
            app.go(Page::Capture);
        }
        return;
    }
    let state = &mut app.pages.connections;
    if state.shown.as_deref() != Some(filter) {
        state.shown = Some(filter.to_owned());
        state.field.set(filter);
        state.page = None;
    }
    let key = Key {
        filter: effective(filter, state.favorites),
        favorites: state.favorites,
        sort: SORTS[state.sort],
        page: state.page,
        capture: app.live.capture_id,
        unit: app.live.unit,
    };
    state.list.poll();
    let engine = app.engine.clone();
    let job_key = key.clone();
    let every = Some(Duration::from_secs(1));
    state.list.load(&ctx, &key, every, move || {
        connection_list(
            t,
            &engine.lock(),
            &ListRequest {
                filter: &job_key.filter,
                sort: job_key.sort,
                page: job_key.page,
                size: None,
            },
        )
    });

    let engine = app.engine.clone();
    let unit_index = entries::unit_index(app.live.unit);
    let state = &mut app.pages.connections;
    let mut new_page = None;
    let mut new_sort = None;
    let mut favorites = state.favorites;
    widgets::bare_card(ui, |ui| {
        let out = state.field.bar(
            ui,
            t,
            &engine,
            true,
            &t.get("connections-filter-placeholder"),
            &t.get("connections-filter-help"),
        );
        actions.apply = out.apply;
        actions.example = state.field.extras(
            ui,
            t,
            true,
            &t.get("connections-filter-help-text"),
            &FILTER_EXAMPLES,
            |_| {},
        );
        let list = state.list.value();
        widgets::toolbar(ui, |ui| {
            let labels = [t.get("sort-desc"), t.get("sort-asc"), t.get("sort-recent")];
            new_sort = widgets::select(ui, &labels, state.sort, true);
            actions.unit = widgets::segmented_sized(ui, &entries::unit_labels(t), unit_index, true);
            widgets::switch(ui, &mut favorites, &t.get("only-favorites"), false);
            if let Some(list) = list {
                widgets::toolbar_sum(ui, &sum(t, list));
            }
        });
        let Some(list) = list else {
            return;
        };
        if let Some(error) = &list.error {
            widgets::error(ui, error);
        }
        let (open, favorite) = table(ui, t, list);
        actions.open = open;
        actions.favorite = favorite;
        if list.rows.is_empty() {
            let text = if key.filter.is_empty() {
                "connections-empty"
            } else {
                "connections-none-filtered"
            };
            widgets::empty(ui, &t.get(text));
        }
        new_page = widgets::pager(
            ui,
            t,
            list.page,
            list.pages,
            &t.get1("per-page", "count", list.size),
        );
    });
    if let Some(sort) = new_sort {
        state.sort = sort;
    }
    if favorites != state.favorites {
        state.favorites = favorites;
        state.page = None;
    }
    if new_page.is_some() {
        state.page = new_page;
    }

    if let Some(index) = actions.unit {
        entries::set_unit(app, index);
    }
    if let Some((keys, add)) = actions.favorite {
        {
            let mut engine = app.engine.lock();
            for key in &keys {
                engine.add_or_remove_favorite(key, add);
            }
        }
        app.engine.store_conf();
        app.pages.connections.list.reset();
        app.pages.entries.list.reset();
        app.pages.overview.reset();
    }
    if let Some(example) = actions.example {
        app.go(Page::Connections { filter: example });
    } else if actions.statistics || actions.packets {
        let connections = effective(filter, app.pages.connections.favorites);
        app.go(if actions.packets {
            Page::Packets {
                filter: String::new(),
                connections,
                goto: None,
            }
        } else {
            Page::Statistics {
                filter: String::new(),
                connections,
            }
        });
    } else if actions.capture {
        app.go(Page::Capture);
    } else if actions.apply {
        let filter = app.pages.connections.field.text.trim().to_owned();
        app.go(Page::Connections { filter });
    } else if let Some(key) = actions.open {
        app.go(Page::Connection(key));
    }
}

/// "Connections 1–30 of 196 (42.1 MB)" at the right of the toolbar
fn sum(t: Lang, list: &ConnectionList) -> String {
    if list.total == 0 {
        return t.get("connections-none");
    }
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
}

/// The table of the list; returns the connection opened and a star clicked
#[allow(clippy::type_complexity)]
fn table(
    ui: &mut Ui,
    t: Lang,
    list: &ConnectionList,
) -> (Option<AddressPortPair>, Option<(Vec<FavoriteKey>, bool)>) {
    let p = palette(ui.ctx());
    // the program goes first when the window gets narrow (`.hide-md`)
    let programs = list.programs && ui.ctx().content_rect().width() >= 1200.0;
    let mut cols = vec![
        Col::new("", Width::Px(48.0)),
        Col::new(t.get("conn-source"), Width::Share(0.166)),
        Col::new(t.get("conn-destination"), Width::Share(0.166)),
        Col::new(t.get("conn-protocol"), Width::Share(0.0715)),
        Col::new(t.get("conn-service"), Width::Share(0.096)),
    ];
    if programs {
        cols.push(Col::new(t.get("conn-program"), Width::Share(0.093)));
    }
    cols.extend([
        Col::new(t.get("conn-host"), Width::Rest(140.0)),
        Col::num(t.get("conn-packets"), Width::Px(84.0)),
        Col::num(t.get("conn-data"), Width::Px(174.0)),
    ]);
    let two_lines = list.rows.iter().any(|r| !r.host_sub.is_empty());
    let mut spec = TableSpec::new(
        "connections",
        cols,
        if two_lines { 45.0 } else { 36.0 },
        list.rows.len(),
    );
    spec.click = true;
    let mut open = None;
    let mut favorite = None;
    let largest = list.largest.max(1) as f32;
    let out = widgets::table(ui, &spec, |ui, r, c| {
        let row = &list.rows[r];
        // without the program column, the later columns move one to the left
        let c = if !programs && c >= 5 { c + 1 } else { c };
        match c {
            0 => {
                let text = if row.favorite {
                    t.get("favorite-remove")
                } else {
                    t.get("favorite-add")
                };
                if widgets::star(ui, row.favorite, &text).clicked() {
                    favorite = Some((row.favorite_keys.clone(), !row.favorite));
                }
            }
            1 => {
                ui.label(widgets::mono(&row.src));
            }
            2 => {
                ui.label(widgets::mono(&row.dst));
            }
            3 => {
                ui.label(&row.proto);
            }
            4 => {
                ui.add(
                    egui::Label::new(if row.service_na { "–" } else { &row.service }).truncate(),
                );
            }
            5 => {
                let text = if row.program_na {
                    t.get("unknown")
                } else {
                    row.program.clone()
                };
                ui.add(egui::Label::new(text).truncate());
            }
            6 => {
                if host_cell(ui, row) {
                    open = Some(r);
                }
            }
            7 => {
                ui.label(widgets::mono(&row.packets));
            }
            _ => {
                #[allow(clippy::cast_precision_loss)]
                let (bar_in, bar_out) = (row.bar_in as f32, row.bar_out as f32);
                data_cell(
                    ui,
                    &row.data,
                    (bar_in + bar_out) / largest,
                    bar_in,
                    bar_out,
                    &row.bar_hint,
                );
            }
        }
    });
    let _ = p;
    let clicked = out.clicked.or(open);
    let jump = clicked.and_then(|r| match &list.rows[r].jump {
        Jump::Connection(key) => Some(*key),
        _ => None,
    });
    (if favorite.is_some() { None } else { jump }, favorite)
}

/// The remote end (`td.ell`): its name as a link with the country after it, its AS or labels
/// below; returns whether the name was clicked
fn host_cell(ui: &mut Ui, row: &crate::view::connections::ConnectionRow) -> bool {
    let p = palette(ui.ctx());
    let mut clicked = false;
    let rect = ui.max_rect();
    let lines = if row.host_sub.is_empty() { 1.0 } else { 2.0 };
    let block = widgets::LH_BASE + (lines - 1.0) * theme::FS_XS * 1.3;
    let top = rect.center().y - block / 2.0;
    let mut first = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                egui::pos2(rect.left(), top),
                egui::vec2(rect.width().min(280.0), widgets::LH_BASE),
            ))
            .layout(Layout::left_to_right(Align::Center)),
    );
    first.spacing_mut().item_spacing.x = 4.0;
    let name = RichText::new(&row.host)
        .size(theme::FS_BASE)
        .color(p.accent);
    if widgets::link_text(&mut first, name).clicked() {
        clicked = true;
    }
    if !row.country.is_empty() {
        widgets::tag_cc(&mut first, &row.country);
    }
    if !row.host_sub.is_empty() {
        let mut second = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
            egui::pos2(rect.left(), top + widgets::LH_BASE),
            egui::vec2(rect.width().min(360.0), theme::FS_XS * 1.3),
        )));
        second.add(
            egui::Label::new(
                RichText::new(&row.host_sub)
                    .size(theme::FS_XS)
                    .color(p.text_3)
                    .line_height(Some(theme::FS_XS * 1.3)),
            )
            .truncate(),
        );
    }
    clicked
}

/// The amount with its bar below (`td.num` with `.tbar`): the bar as long as the share of the
/// largest connection, incoming and outgoing in their colors
fn data_cell(ui: &mut Ui, text: &str, share: f32, incoming: f32, outgoing: f32, hint: &str) {
    let p = palette(ui.ctx());
    let rect = ui.max_rect();
    let block = widgets::LH_BASE + 8.0;
    let top = rect.center().y - block / 2.0;
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), theme::mono_font(theme::FS_SM), p.text);
    let at = egui::pos2(
        rect.right() - galley.size().x,
        top + (widgets::LH_BASE - galley.size().y) / 2.0,
    );
    ui.painter().galley(at, galley, p.text);
    let bar = Rect::from_min_size(
        egui::pos2(rect.left(), top + widgets::LH_BASE),
        egui::vec2(rect.width(), 8.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar));
    widgets::tbar(&mut child, bar.width(), 8.0, share, incoming, outgoing);
    let full = Rect::from_min_size(
        bar.min,
        egui::vec2((bar.width() * share.clamp(0.015, 1.0)).max(3.0), 8.0),
    );
    ui.interact(full, ui.unique_id().with("bar"), Sense::hover())
        .on_hover_text(hint);
}

fn read_detail(t: Lang, engine: &Engine, key: &AddressPortPair) -> Option<ConnectionDetail> {
    let state = engine.lock();
    let info = state.info_traffic.map.get(key)?;
    Some(connection_detail(t, &state, key, info))
}

/// What can be clicked on the page of a connection
#[derive(Default)]
struct DetailActions {
    jump: Option<Jump>,
    stream: Option<u64>,
    favorite: Option<(FavoriteKey, bool)>,
    measure: bool,
}

pub fn show_connection(app: &mut App, ui: &mut Ui, key: &AddressPortPair) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let load_key = (*key, app.live.capture_id);
    let state = &mut app.pages.connections;
    state.detail.poll();
    let engine = app.engine.clone();
    let job_key = *key;
    let every = app.live.running.then_some(Duration::from_secs(1));
    state.detail.load(&ctx, &load_key, every, move || {
        read_detail(t, &engine, &job_key)
    });
    let detail = app
        .pages
        .connections
        .detail
        .value_of(&load_key)
        .and_then(Option::as_ref);
    let crumbs = [(
        t.get("nav-connections"),
        Page::Connections {
            filter: String::new(),
        },
    )];
    let mut actions = DetailActions::default();
    let title_text = detail.map(|d| (d.protocol.clone(), d.title.clone()));
    let jumps = detail.map(|d| (d.packets_jump.clone(), d.statistics_jump.clone(), d.stream));
    detail_head(
        app,
        ui,
        &crumbs,
        &t.get("connection-title"),
        |ui| match &title_text {
            Some((protocol, title)) => {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                widgets::tag(ui, protocol);
                ui.add(
                    egui::Label::new(
                        RichText::new(title).font(theme::mono_medium_font(theme::FS_LG)),
                    )
                    .truncate(),
                );
            }
            None => widgets::title(ui, &t.get("connection-title")),
        },
        |_| {},
        |ui| {
            let Some((packets, statistics, stream)) = &jumps else {
                return;
            };
            if let Some(n) = stream
                && widgets::icon_text_button(
                    ui,
                    Some(icons::STREAM),
                    &t.get("stream-title"),
                    Kind::Secondary,
                    false,
                )
                .clicked()
            {
                actions.stream = Some(*n);
            }
            if let Some(jump) = statistics
                && widgets::icon_text_button(
                    ui,
                    Some(icons::STATISTICS),
                    &t.get("nav-statistics"),
                    Kind::Secondary,
                    false,
                )
                .clicked()
            {
                actions.jump = Some(jump.clone());
            }
            if let Some(jump) = packets
                && widgets::icon_text_button(
                    ui,
                    Some(icons::PACKETS),
                    &t.get("nav-packets"),
                    Kind::Secondary,
                    false,
                )
                .clicked()
            {
                actions.jump = Some(jump.clone());
            }
        },
    );
    let Some(found) = app.pages.connections.detail.value_of(&load_key) else {
        return;
    };
    let Some(d) = found else {
        widgets::notice_text(ui, &t.get("gui-connection-gone"));
        return;
    };
    let mut remote_actions = DetailActions::default();
    widgets::split2(
        ui,
        |ui| connection_card(ui, t, d, &mut actions),
        |ui| remote_card(ui, t, d, &mut remote_actions),
    );
    actions.jump = actions.jump.or(remote_actions.jump);
    actions.favorite = actions.favorite.or(remote_actions.favorite);
    let remote = d.remote;
    if actions.measure && latency_allowed(&app.engine.lock(), remote) {
        app.engine.measure_latency(remote);
    }
    if let Some((key, add)) = actions.favorite {
        app.engine.lock().add_or_remove_favorite(&key, add);
        app.engine.store_conf();
        app.pages.connections.detail.reset();
        app.pages.connections.list.reset();
    }
    if let Some(n) = actions.stream {
        app.go(Page::Stream(n));
    } else if let Some(jump) = actions.jump {
        app.jump(jump);
    }
}

/// `14:36:41,230` in the time zone of the computer
fn clock_ms(t: Lang, ts: Timestamp) -> String {
    let Ok(time) = jiff::Timestamp::from_microsecond(ts.secs() * 1_000_000 + ts.usecs()) else {
        return String::new();
    };
    let zoned = time.to_zoned(jiff::tz::TimeZone::system());
    format!(
        "{}{}{:03}",
        zoned.strftime("%H:%M:%S"),
        t.decimal_separator(),
        zoned.subsec_nanosecond() / 1_000_000
    )
}

/// A link among the values (`.facts .links`)
fn fact_link(ui: &mut Ui, text: &str) -> bool {
    let p = palette(ui.ctx());
    ui.label(RichText::new("·").color(p.text_3));
    widgets::link_text(ui, RichText::new(text).color(p.accent)).clicked()
}

/// The card "Connection" (`.facts`)
#[allow(clippy::many_single_char_names)] // coordinates of the drawing
fn connection_card(ui: &mut Ui, t: Lang, d: &ConnectionDetail, actions: &mut DetailActions) {
    let p = palette(ui.ctx());
    let mut labels = vec![
        t.get("conn-source"),
        t.get("conn-source-mac"),
        t.get("conn-destination"),
        t.get("conn-destination-mac"),
        t.get("conn-protocol"),
        t.get("conn-direction"),
        t.get("conn-service"),
        t.get("conn-program"),
        t.get("conn-packets"),
        t.get("conn-data"),
        t.get("conn-first"),
        t.get("conn-last"),
    ];
    labels.extend(d.more.iter().map(|(label, _)| label.clone()));
    labels.push(t.get("latency"));
    let fixed = 12;
    let latency = labels.len() - 1;
    widgets::card_tight(
        ui,
        &t.get("connection-title"),
        |_| {},
        |ui| {
            widgets::facts(ui, &labels, |ui, i| match i {
                0 => {
                    widgets::code(ui, &d.src);
                }
                1 => {
                    widgets::code(ui, &d.src_mac);
                }
                2 => {
                    widgets::code(ui, &d.dst);
                }
                3 => {
                    widgets::code(ui, &d.dst_mac);
                }
                4 => {
                    ui.label(&d.protocol_line);
                }
                5 => {
                    let (icon, text) = if d.outgoing {
                        (icons::ARROW_UP, t.get("direction-out"))
                    } else {
                        (icons::ARROW_DOWN, t.get("direction-in"))
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    widgets::paint_icon(ui, icon, rect, p.text);
                    ui.label(text);
                }
                6 => match &d.service {
                    Some((name, sub)) => {
                        ui.label(RichText::new(name).font(theme::semibold_font(theme::FS_BASE)));
                        ui.label(RichText::new(sub).color(p.text_3));
                    }
                    None => {
                        ui.label(RichText::new("–").color(p.text_3));
                    }
                },
                7 => match &d.program {
                    Some((name, path)) => {
                        ui.label(RichText::new(name).font(theme::semibold_font(theme::FS_BASE)));
                        ui.label(
                            RichText::new(path)
                                .font(theme::mono_font(theme::FS_SM))
                                .color(p.text_3),
                        );
                        let text = if d.program_favorite {
                            t.get("favorite-remove")
                        } else {
                            t.get("favorite-add")
                        };
                        if widgets::star(ui, d.program_favorite, &text).clicked()
                            && let Some(key) = &d.program_key
                        {
                            actions.favorite = Some((key.clone(), !d.program_favorite));
                        }
                    }
                    None => {
                        ui.label(RichText::new(t.get("unknown")).color(p.text_3));
                    }
                },
                8 => {
                    widgets::code(ui, &d.packets);
                    if let Some(jump) = &d.packets_jump
                        && fact_link(ui, &t.get("connection-packets"))
                    {
                        actions.jump = Some(jump.clone());
                    }
                    if let Some(jump) = &d.statistics_jump
                        && fact_link(ui, &t.get("connection-statistics"))
                    {
                        actions.jump = Some(jump.clone());
                    }
                    if let Some(n) = d.stream
                        && fact_link(ui, &t.get("packet-stream"))
                    {
                        actions.stream = Some(n);
                    }
                }
                9 => {
                    widgets::code(ui, &d.data);
                    ui.label(
                        widgets::small(format!("↓ {} · ↑ {}", d.data_in, d.data_out))
                            .color(p.text_3),
                    );
                    #[allow(clippy::cast_precision_loss)]
                    let (a, b) = (d.bar_in as f32, d.bar_out as f32);
                    let width = (ui.available_width() - 8.0).clamp(80.0, 200.0);
                    widgets::tbar(ui, width, 8.0, 1.0, a, b);
                }
                10 => {
                    widgets::code(ui, &clock_ms(t, d.first));
                }
                11 => {
                    widgets::code(ui, &clock_ms(t, d.last));
                }
                i if i == latency => {
                    if d.measuring {
                        ui.label(RichText::new(t.get("latency-measuring")).color(p.text_3));
                    } else if let Some(value) = &d.latency {
                        widgets::code(ui, value);
                        if d.latency_supported {
                            actions.measure |=
                                widgets::small_button(ui, t.get("latency-again"), Kind::Ghost)
                                    .clicked();
                        }
                    } else if d.latency_supported {
                        actions.measure |= widgets::icon_text_button(
                            ui,
                            Some(icons::CLOCK),
                            &t.get("latency-measure"),
                            Kind::Secondary,
                            true,
                        )
                        .clicked();
                    } else {
                        ui.label(RichText::new("–").color(p.text_3));
                    }
                }
                i => {
                    if let Some((_, value)) = d.more.get(i - fixed) {
                        ui.label(value);
                    }
                }
            });
        },
    );
}

/// The card "Remote end" (`.facts` and the buttons to its connections)
fn remote_card(ui: &mut Ui, t: Lang, d: &ConnectionDetail, actions: &mut DetailActions) {
    let p = palette(ui.ctx());
    let labels = [
        t.get("host-address"),
        t.get("host-rdns"),
        t.get("host-asn"),
        t.get("host-country"),
    ];
    let star_text = if d.remote_favorite {
        t.get("favorite-address-remove")
    } else {
        t.get("favorite-address-add")
    };
    let mut star = false;
    widgets::icon_card(
        ui,
        None,
        &t.get("connection-remote"),
        |ui| {
            star = widgets::star(ui, d.remote_favorite, &star_text).clicked();
        },
        |ui| {
            widgets::tight(ui, |ui| {
                widgets::facts(ui, &labels, |ui, i| match i {
                    0 => {
                        widgets::code(ui, &d.remote.to_string());
                    }
                    1 => {
                        ui.label(if d.rdns.is_empty() { "–" } else { &d.rdns });
                    }
                    2 => {
                        if let Some((code, name)) = &d.asn {
                            widgets::code(ui, code);
                            ui.label(name);
                        } else {
                            let text = if d.host_tags.is_empty() {
                                "–"
                            } else {
                                &d.host_tags
                            };
                            ui.label(RichText::new(text).color(p.text_3));
                        }
                    }
                    _ => {
                        if d.country.is_empty() {
                            ui.label(RichText::new("–").color(p.text_3));
                        } else {
                            widgets::tag_cc(ui, &d.country);
                        }
                    }
                });
            });
            // the buttons to the connections of the host and the address (`.cfoot`)
            widgets::card_foot(ui, false, |ui| {
                if let Some(jump) = &d.host_jump
                    && widgets::small_button(
                        ui,
                        t.get("connection-host-connections"),
                        Kind::Secondary,
                    )
                    .clicked()
                {
                    actions.jump = Some(jump.clone());
                }
                if widgets::small_button(
                    ui,
                    t.get("connection-address-connections"),
                    Kind::Secondary,
                )
                .clicked()
                {
                    actions.jump = Some(d.address_jump.clone());
                }
            });
        },
    );
    if star {
        actions.favorite = Some((FavoriteKey::Address(d.remote), !d.remote_favorite));
    }
}
