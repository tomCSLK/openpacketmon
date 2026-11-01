//! The packet list: one long list like a packet analyzer. Only the rows in view are read, in
//! blocks in the background; at the end it follows the new packets of a running capture.

use std::collections::HashMap;
use std::time::Duration;

use eframe::egui::{self, Align, Key, Layout, RichText, Sense, TextEdit, TextWrapMode, Ui};
use egui_extras::{Column, TableBuilder};

use crate::engine::Engine;
use crate::gui::filter_field::{FilterField, filter_help};
use crate::gui::load::{Loader, Task};
use crate::gui::pages::head;
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, Page};
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::packets::{
    FILTER_EXAMPLES, PacketList, PacketRow, RowBlock, Window, filter_field_names, packet_list,
    pcap_file, row_block,
};

const ROW_H: f32 = 24.0;
/// How often the rows are read while packets come
const LIVE: Duration = Duration::from_millis(300);
/// The most rows of one block
const MAX_BLOCK: usize = 500;

/// Which rows are asked for
#[derive(Clone, PartialEq, Debug)]
struct BlockKey {
    filters: Filters,
    /// `None`: the end of the list
    from: Option<usize>,
    count: usize,
    goto: Option<u64>,
}

/// The filters in use, and the capture they apply to
#[derive(Clone, PartialEq, Debug)]
struct Filters {
    display: String,
    connections: String,
    capture: usize,
}

struct Reply {
    block: Option<RowBlock>,
    /// The first row in view when asked, to see if older packets were dropped meanwhile
    first: usize,
}

#[derive(Default)]
pub struct State {
    filters: Option<Filters>,
    goto_seen: Option<u64>,
    display: FilterField,
    connections: FilterField,
    head: Loader<Filters, PacketList>,
    block: Loader<BlockKey, Reply>,
    key: Option<BlockKey>,
    rows: HashMap<usize, PacketRow>,
    total: usize,
    foot: String,
    /// Stays at the end of the list
    follow: bool,
    /// Following was switched off by hand
    paused: bool,
    was_at_end: bool,
    /// The row marked by "go to packet"
    target: Option<u64>,
    goto: Option<u64>,
    goto_text: String,
    goto_note: Option<String>,
    /// The scroll position to take in the next frame
    scroll_to: Option<f32>,
    first: usize,
    visible: usize,
    height: f32,
    save: Task<Option<Result<String, String>>>,
    saved: Option<Result<String, String>>,
}

#[cfg(test)]
impl State {
    pub fn first_row(&self) -> usize {
        self.first
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn target(&self) -> Option<u64> {
        self.target
    }

    pub fn visible(&self) -> usize {
        self.visible
    }

    pub fn typed(&self) -> &str {
        &self.display.text
    }
}

impl State {
    fn open(&mut self, filters: Filters, goto: Option<u64>, running: bool) {
        self.display.set(&filters.display);
        self.connections.set(&filters.connections);
        self.filters = Some(filters);
        self.rows.clear();
        self.total = 0;
        self.key = None;
        self.goto = goto;
        self.target = None;
        self.goto_note = None;
        self.first = 0;
        self.scroll_to = Some(0.0);
        self.follow = goto.is_none() && running;
        self.paused = false;
        self.was_at_end = false;
    }

    /// The switch "follow": on, it jumps to the end at once, not only with the next block
    fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        self.paused = !self.follow;
        if self.follow {
            self.target = None;
            self.scroll_to = Some(self.max_offset());
        }
    }

    /// Scrolling up ends following; coming back to the end by hand takes it up again
    fn after_scrolling(&mut self, at_end: bool, explicit: bool, live: bool) {
        if !explicit {
            if self.follow && !at_end {
                self.follow = false;
            } else if !self.follow && !self.paused && live && at_end && !self.was_at_end {
                self.follow = true;
                self.target = None;
            }
        }
        self.was_at_end = at_end;
    }

    fn max_offset(&self) -> f32 {
        (self.total as f32 * ROW_H - self.height).max(0.0)
    }

    /// Takes a block of rows that has arrived
    fn receive(&mut self, reply: Reply) {
        let goto = self.goto.take();
        let Some(block) = reply.block else {
            self.rows.clear();
            self.total = 0;
            return;
        };
        self.total = block.total;
        self.rows = block
            .rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| (block.from + i, r))
            .collect();
        block.foot.clone_into(&mut self.foot);
        if let Some(asked) = goto {
            self.goto_found(asked, block.goto);
        } else if let Some(now) = block.anchor
            && !self.follow
            && now != reply.first
        {
            // older packets were dropped from the store: the same packets stay in view
            let moved = now as f32 - reply.first as f32;
            self.scroll_to = Some(((self.first as f32 + moved) * ROW_H).max(0.0));
        }
    }

    fn goto_found(&mut self, asked: u64, found: Option<(usize, u64)>) {
        self.follow = false;
        self.goto_note = Some(asked.to_string());
        let Some((position, number)) = found else {
            self.target = None;
            return;
        };
        self.target = Some(number);
        if number == asked {
            self.goto_note = None;
        }
        // never past the end of the list, that would show an empty area
        let centered = position as f32 * ROW_H - self.height / 2.0;
        self.scroll_to = Some(centered.clamp(0.0, self.max_offset()));
    }

    /// The rows to read next
    fn wanted(&self, filters: &Filters) -> BlockKey {
        let visible = self.visible.max(1);
        if let Some(goto) = self.goto {
            return BlockKey {
                filters: filters.clone(),
                from: None,
                count: (visible * 3).min(MAX_BLOCK),
                goto: Some(goto),
            };
        }
        if self.follow {
            return BlockKey {
                filters: filters.clone(),
                from: None,
                count: (visible * 2).min(MAX_BLOCK),
                goto: None,
            };
        }
        let last = (self.first + visible).min(self.total);
        let covered = (self.first..last).all(|row| self.rows.contains_key(&row));
        if covered
            && let Some(key) = &self.key
            && key.filters == *filters
            && key.from.is_some()
        {
            return key.clone();
        }
        BlockKey {
            filters: filters.clone(),
            from: Some(self.first.saturating_sub(visible)),
            count: (visible * 3).min(MAX_BLOCK),
            goto: None,
        }
    }
}

fn load_block(
    state: &mut State,
    ctx: &egui::Context,
    t: Lang,
    engine: &Engine,
    index: &std::sync::Arc<crate::view::filter_index::FilterIndex>,
    live: bool,
) {
    let Some(filters) = state.filters.clone() else {
        return;
    };
    let key = state.wanted(&filters);
    let window = Window {
        from: key.from,
        count: key.count,
        goto: key.goto,
        anchor: state.rows.get(&state.first).map(|r| r.number),
    };
    let first = state.first;
    let engine = engine.clone();
    let index = index.clone();
    let every = live.then_some(LIVE);
    state.block.load(ctx, &key, every, move || Reply {
        block: row_block(
            t,
            &engine,
            &index,
            &filters.display,
            &filters.connections,
            window,
        ),
        first,
    });
    state.key = Some(key);
}

/// Filters of a page with a link to itself
fn goto_page(display: &str, connections: &str) -> Page {
    Page::Packets {
        filter: display.trim().to_owned(),
        connections: connections.trim().to_owned(),
        goto: None,
    }
}

pub fn show(app: &mut App, ui: &mut Ui, display: &str, connections: &str, goto: Option<u64>) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let filters = Filters {
        display: display.to_owned(),
        connections: connections.to_owned(),
        capture: app.live.capture_id,
    };
    let running = app.live.running;
    let live = running && !app.live.frozen && !app.live.finished;
    if app.pages.packets.filters.as_ref() != Some(&filters) {
        app.pages.packets.open(filters.clone(), goto, live);
    } else if goto.is_some() && app.pages.packets.goto_seen != goto {
        app.pages.packets.goto = goto;
    }
    app.pages.packets.goto_seen = goto;

    let engine = app.engine.clone();
    let index = app.index.clone();
    let state = &mut app.pages.packets;
    if !live {
        state.follow = false;
    }
    state.head.poll();
    if state.block.poll()
        && let Some(reply) = state.block.take()
    {
        state.receive(reply);
    }
    {
        let engine = engine.clone();
        let f = filters.clone();
        state
            .head
            .load(&ctx, &filters, Some(Duration::from_secs(1)), move || {
                packet_list(t, &engine, &f.display, &f.connections)
            });
    }
    load_block(state, &ctx, t, &engine, &index, live);
    if let Some(result) = state.save.take()
        && result.is_some()
    {
        state.saved = result;
    }

    // what the page says about the capture and the filters
    let (has_list, has_foot, unavailable, error, connection) =
        state.head.value_of(&filters).map_or_else(
            || (false, false, String::new(), None, None),
            |l| {
                (
                    l.list,
                    l.foot.is_some(),
                    l.unavailable.clone(),
                    l.error.as_ref().map(|e| (e.message.clone(), e.connections)),
                    l.connection.clone(),
                )
            },
        );

    let mut actions = Actions::default();
    let filtered = !display.is_empty() || !connections.is_empty();
    head(
        app,
        ui,
        &t.get("nav-packets"),
        Some(&t.get("packets-subtitle")),
        |ui| {
            if has_foot {
                let save = if filtered {
                    "packets-download-filtered"
                } else {
                    "packets-download"
                };
                if widgets::button(ui, t.get(save), Kind::Secondary).clicked() {
                    actions.save = true;
                }
                if widgets::button(ui, t.get("nav-statistics"), Kind::Secondary).clicked() {
                    actions.statistics = true;
                }
            }
        },
    );
    if !running {
        widgets::notice(ui, |ui| {
            ui.label(t.get("no-capture"));
            if widgets::link(ui, t.get("no-capture-start")).clicked() {
                actions.capture = true;
            }
        });
    } else if !unavailable.is_empty() {
        widgets::notice_text(ui, &unavailable);
    }
    if let Some((ends, key)) = connection {
        widgets::notice(ui, |ui| {
            ui.label(t.get("filter-connection"));
            widgets::code(ui, &ends);
            if widgets::link(ui, t.get("filter-connection-open")).clicked() {
                actions.connection = Some(key);
            }
        });
    }
    if !connections.is_empty() {
        widgets::notice(ui, |ui| {
            ui.label(t.get("filter-connections"));
            if widgets::link(ui, t.get("filter-connections-open")).clicked() {
                actions.connections = true;
            }
        });
    }
    if let Some(saved) = &app.pages.packets.saved {
        match saved {
            Ok(path) => widgets::flash(ui, &t.get1("gui-saved", "path", path.as_str())),
            Err(e) => widgets::error(ui, &t.get1("gui-save-failed", "error", e.as_str())),
        }
    }
    filter_bar(app, ui, &mut actions, display, connections);
    if let Some((message, _)) = &error {
        widgets::error(ui, message);
    }
    actions.example = filter_help(
        ui,
        &t.get("packets-filter-help"),
        &t.get("packets-filter-help-text"),
        &FILTER_EXAMPLES,
        &[
            (
                t.get("packets-filter-protocols"),
                crate::filter::PROTOCOLS.join(" "),
            ),
            (
                t.get("packets-filter-fields"),
                filter_field_names().join(" "),
            ),
        ],
        |ui| color_legend(ui, t),
    );

    if has_list {
        list(app, ui, filtered, live, &mut actions);
    }

    run_actions(app, &ctx, actions, display, connections);
}

/// The meaning of the row colors, on the colors themselves
fn color_legend(ui: &mut Ui, t: Lang) {
    let p = palette(ui.ctx());
    ui.strong(t.get("packets-colors"));
    ui.horizontal_wrapped(|ui| {
        for (class, text) in [
            ("bad", t.get("packets-color-bad")),
            ("tcp-control", t.get("packets-color-tcp")),
            ("dns", "DNS".to_owned()),
            ("local", t.get("packets-color-local")),
            ("tls", "TLS/QUIC".to_owned()),
            ("http", "HTTP".to_owned()),
            ("icmp", "ICMP".to_owned()),
            ("udp", "UDP".to_owned()),
        ] {
            let chip = egui::Label::new(RichText::new(text).small().color(p.text))
                .wrap_mode(TextWrapMode::Extend);
            egui::Frame::new()
                .fill(p.row(class).unwrap_or(p.surface))
                .corner_radius(3.0)
                .inner_margin(egui::vec2(6.0, 2.0))
                .show(ui, |ui| ui.add(chip));
        }
    });
}

/// What the user asked for in this frame
#[derive(Default)]
struct Actions {
    save: bool,
    statistics: bool,
    capture: bool,
    connections: bool,
    connection: Option<crate::networking::types::address_port_pair::AddressPortPair>,
    apply: bool,
    reset: bool,
    open: Option<u64>,
    example: Option<String>,
}

fn run_actions(
    app: &mut App,
    ctx: &egui::Context,
    actions: Actions,
    display: &str,
    connections: &str,
) {
    let t = app.t;
    if actions.save {
        let (engine, index) = (app.engine.clone(), app.index.clone());
        let (display, connections) = (display.to_owned(), connections.to_owned());
        app.pages.packets.saved = None;
        app.pages.packets.save.start(ctx, move || {
            let file = pcap_file(t, &engine, &index, &display, &connections)?;
            let path = rfd::FileDialog::new()
                .set_file_name(&file.name)
                .save_file()?;
            Some(
                std::fs::write(&path, &file.data)
                    .map(|()| path.display().to_string())
                    .map_err(|e| e.to_string()),
            )
        });
    }
    if actions.statistics {
        app.go(Page::Statistics {
            filter: display.to_owned(),
            connections: connections.to_owned(),
        });
    } else if actions.capture {
        app.go(Page::Capture);
    } else if actions.connections {
        app.go(Page::Connections {
            filter: connections.to_owned(),
        });
    } else if let Some(key) = actions.connection {
        app.go(Page::Connection(key));
    } else if actions.apply {
        let state = &app.pages.packets;
        let page = goto_page(&state.display.text, &state.connections.text);
        app.go(page);
    } else if actions.reset {
        app.go(goto_page("", ""));
    } else if let Some(example) = actions.example {
        app.go(goto_page(&example, connections));
    } else if let Some(number) = actions.open {
        let state = &mut app.pages.packets;
        state.follow = false;
        state.target = Some(number);
        app.go(Page::Packet(number));
    }
}

fn filter_bar(app: &mut App, ui: &mut Ui, actions: &mut Actions, display: &str, connections: &str) {
    let t = app.t;
    let engine = app.engine.clone();
    let state = &mut app.pages.packets;
    let with_connections = !connections.is_empty();
    let display_problem = state.display.problem(t, false).map(str::to_owned);
    let connections_problem = state.connections.problem(t, true).map(str::to_owned);
    ui.horizontal(|ui| {
        let reset = !display.is_empty() || with_connections;
        let room = ui.available_width() - 100.0 - if reset { 110.0 } else { 0.0 };
        let width = if with_connections { room / 2.0 } else { room };
        if with_connections {
            let valid = Some(connections_problem.is_none());
            let out = state.connections.show(
                ui,
                t,
                &engine,
                true,
                &t.get("connections-filter-placeholder"),
                valid,
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
            &t.get("packets-filter-placeholder"),
            valid,
            width,
        );
        actions.apply |= out.apply;
        if widgets::button(ui, t.get("apply"), Kind::Primary).clicked() {
            actions.apply = true;
        }
        if reset && widgets::button(ui, t.get("filter-reset"), Kind::Secondary).clicked() {
            actions.reset = true;
        }
    });
    if let Some(problem) = display_problem.or(connections_problem) {
        let p = palette(ui.ctx());
        ui.label(RichText::new(problem).small().color(p.down));
    }
}

fn list(app: &mut App, ui: &mut Ui, filtered: bool, live: bool, actions: &mut Actions) {
    let t = app.t;
    let plain = app.engine.lock().conf.settings.plain_packets;
    let colored = !plain;
    let mut wanted_colors = colored;
    let state = &mut app.pages.packets;
    let shown_to = (state.first + state.visible).min(state.total);
    let title = if state.total == 0 {
        t.get("js-packets-none")
    } else {
        fill(
            &t.get("js-packets-range"),
            &[
                format::count(t, (state.first + 1) as u128),
                format::count(t, shown_to as u128),
                format::count(t, state.total as u128),
            ],
        )
    };
    let note = state.goto_note.clone().map(|asked| match state.target {
        Some(found) => fill(&t.get("js-packets-goto-other"), &[asked, found.to_string()]),
        None => fill(&t.get("js-packets-goto-none"), &[asked]),
    });
    if let Some(note) = note {
        widgets::notice_text(ui, &note);
    }
    let mut toggle_follow = false;
    let mut go = false;
    let focus_goto = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::G));
    let foot = state.foot.clone();
    let following = state.follow;
    let mut goto_text = std::mem::take(&mut state.goto_text);
    widgets::flush_card(
        ui,
        &title,
        |ui| {
            let id = ui.next_auto_id().with("goto");
            let goto = ui.add(
                TextEdit::singleline(&mut goto_text)
                    .id(id)
                    .hint_text(t.get("packets-goto-placeholder"))
                    .desired_width(72.0),
            );
            let goto = goto.on_hover_text(t.get("packets-goto-key"));
            if focus_goto {
                goto.request_focus();
            }
            if goto.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                go = true;
            }
            ui.label(t.get("packets-goto"));
            ui.checkbox(&mut wanted_colors, t.get("packets-colors"));
            if live {
                let kind = if following {
                    Kind::Primary
                } else {
                    Kind::Secondary
                };
                let response = widgets::small_button(ui, t.get("packets-live"), kind)
                    .on_hover_text(t.get("packets-follow-hint"));
                toggle_follow = response.clicked();
            }
        },
        |ui| {
            let room = ui.available_height() - 26.0;
            ui.allocate_ui(egui::vec2(ui.available_width(), room), |ui| {
                table(ui, state, t, colored, live, filtered, actions);
            });
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                widgets::hint(ui, &foot);
            });
        },
    );
    state.goto_text = goto_text;
    if toggle_follow {
        state.toggle_follow();
    }
    if go {
        state.goto = state.goto_text.trim().parse().ok();
    }
    if wanted_colors != colored {
        app.engine.lock().conf.settings.plain_packets = !wanted_colors;
        app.engine.store_conf();
    }
}

fn table(
    ui: &mut Ui,
    state: &mut State,
    t: Lang,
    colored: bool,
    live: bool,
    filtered: bool,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    if state.total == 0 {
        let text = if filtered {
            "packets-none-filtered"
        } else {
            "packets-empty"
        };
        widgets::empty(ui, &t.get(text));
        return;
    }
    let explicit = state.scroll_to.is_some();
    let pointer = ui.input(|i| i.pointer.hover_pos());
    // the gaps between the columns take the color of the row
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let mut builder = TableBuilder::new(ui)
        .id_salt("packet-list")
        .striped(false)
        .resizable(true)
        .sense(Sense::click())
        .animate_scrolling(false)
        .auto_shrink(false)
        .max_scroll_height(f32::INFINITY)
        .min_scrolled_height(0.0)
        .stick_to_bottom(state.follow)
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::exact(78.0))
        .column(Column::exact(92.0))
        .column(Column::initial(190.0).at_least(80.0).clip(true))
        .column(Column::exact(24.0))
        .column(Column::initial(190.0).at_least(80.0).clip(true))
        .column(Column::initial(84.0).at_least(50.0).clip(true))
        .column(Column::exact(66.0))
        .column(Column::remainder().at_least(120.0).clip(true));
    if let Some(offset) = state.scroll_to.take() {
        builder = builder.vertical_scroll_offset(offset);
    }
    let heads = [
        (t.get("packet-number"), true),
        (t.get("packet-time"), true),
        (t.get("conn-source"), false),
        (String::new(), false),
        (t.get("conn-destination"), false),
        (t.get("conn-protocol"), false),
        (t.get("packet-length"), true),
        (t.get("packet-info"), false),
    ];
    let (out_text, in_text) = (t.get("js-direction-out"), t.get("js-direction-in"));
    let target = state.target;
    let rows = &state.rows;
    let total = state.total;
    let mut clicked = None;
    let output = builder
        .header(26.0, |mut header| {
            for (text, right) in &heads {
                header.col(|ui| {
                    ui.painter().rect_filled(ui.max_rect(), 0.0, p.surface_2);
                    cell(ui, RichText::new(text).small().color(p.muted), *right);
                });
            }
        })
        .body(|body| {
            body.rows(ROW_H, total, |mut row| {
                let position = row.index();
                let Some(r) = rows.get(&position) else {
                    // a row not read yet still takes its height, or the list jumps
                    for _ in 0..heads.len() {
                        row.col(|_| {});
                    }
                    return;
                };
                let base = if colored { p.row(r.color) } else { None };
                let marked = target == Some(r.number);
                let draw = |row: &mut egui_extras::TableRow<'_, '_>,
                            add: &mut dyn FnMut(&mut Ui)| {
                    row.col(|ui| {
                        let rect = ui.max_rect();
                        let hot = pointer.is_some_and(|at| rect.y_range().contains(at.y));
                        let fill = if marked {
                            Some(p.accent_soft)
                        } else if hot {
                            Some(p.surface_hover)
                        } else {
                            base
                        };
                        if let Some(fill) = fill {
                            ui.painter().rect_filled(rect, 0.0, fill);
                        }
                        add(ui);
                    });
                };
                let text = p.text;
                let mono = |s: &str| RichText::new(s).monospace().color(text);
                draw(&mut row, &mut |ui| {
                    cell(ui, mono(&r.number.to_string()), true);
                });
                draw(&mut row, &mut |ui| cell(ui, mono(&r.time), true));
                draw(&mut row, &mut |ui| cell(ui, mono(&r.src), false));
                draw(&mut row, &mut |ui| {
                    let (color, hint) = match r.outgoing {
                        Some(true) => (p.accent, out_text.as_str()),
                        Some(false) => (p.muted, in_text.as_str()),
                        None => (p.faint, ""),
                    };
                    let arrow = cell_response(ui, RichText::new("→").color(color), false);
                    if !hint.is_empty() {
                        arrow.on_hover_text(hint);
                    }
                });
                draw(&mut row, &mut |ui| cell(ui, mono(&r.dst), false));
                draw(&mut row, &mut |ui| {
                    cell(ui, RichText::new(r.protocol).color(text), false);
                });
                draw(&mut row, &mut |ui| {
                    cell(ui, mono(&r.length.to_string()), true);
                });
                draw(&mut row, &mut |ui| {
                    cell(ui, RichText::new(&r.info).color(text), false);
                });
                if row.response().clicked() {
                    clicked = Some(r.number);
                }
            });
        });
    if let Some(number) = clicked {
        actions.open = Some(number);
    }
    state.height = output.inner_rect.height();
    state.visible = (state.height / ROW_H).ceil() as usize;
    state.first = (output.state.clamped_offset().y / ROW_H).floor() as usize;
    let at_end = output.state.clamped_offset().y >= state.max_offset() - 1.0;
    state.after_scrolling(at_end, explicit, live);
}

fn cell(ui: &mut Ui, text: RichText, right: bool) {
    let _ = cell_response(ui, text, right);
}

fn cell_response(ui: &mut Ui, text: RichText, right: bool) -> egui::Response {
    let label = egui::Label::new(text).truncate().selectable(false);
    let layout = if right {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.with_layout(layout, |ui| {
        ui.add_space(6.0);
        ui.add(label)
    })
    .inner
}

/// Puts values into a text with `%1`, `%2`, as the texts of the browser scripts have them
pub fn fill(text: &str, values: &[String]) -> String {
    values
        .iter()
        .enumerate()
        .fold(text.to_owned(), |text, (i, value)| {
            text.replace(&format!("%{}", i + 1), value)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State {
            total: 1_000,
            height: 240.0,
            follow: true,
            was_at_end: true,
            ..State::default()
        }
    }

    #[test]
    fn switching_on_follow_jumps_to_the_end_at_once() {
        let mut s = state();
        s.follow = false;
        s.paused = true;
        s.target = Some(5);
        s.toggle_follow();
        assert!(s.follow);
        assert_eq!(s.scroll_to, Some(1_000.0 * ROW_H - 240.0));
        assert_eq!(s.target, None);
        s.toggle_follow();
        assert!(!s.follow);
        assert!(s.paused);
    }

    #[test]
    fn scrolling_up_ends_following_and_the_end_takes_it_up_again() {
        let mut s = state();
        s.after_scrolling(false, false, true);
        assert!(!s.follow);
        s.after_scrolling(true, false, true);
        assert!(s.follow);
    }

    #[test]
    fn a_jump_or_the_switch_keeps_following_as_it_is() {
        let mut s = state();
        // a jump is not the user scrolling up
        s.after_scrolling(false, true, true);
        assert!(s.follow);
        // switched off by hand: the end does not switch it on again
        s.toggle_follow();
        s.after_scrolling(false, false, true);
        s.after_scrolling(true, false, true);
        assert!(!s.follow);
    }

    #[test]
    fn going_past_the_list_stays_at_its_end() {
        let mut s = state();
        s.goto_found(5_000, Some((999, 1_000)));
        assert_eq!(s.scroll_to, Some(s.max_offset()));
        assert_eq!(s.target, Some(1_000));
        assert_eq!(s.goto_note.as_deref(), Some("5000"));
        assert!(!s.follow);
    }
}
