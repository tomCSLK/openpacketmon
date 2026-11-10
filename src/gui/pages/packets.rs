//! The packet list, as in the draft (`docs/entwurf/assets/pages.js`, `pagePackets`): one long
//! list like a packet analyzer, its newest packets at the bottom. Only the rows in view are read,
//! in blocks in the background; at the end it follows the new packets of a running capture.
//! A click selects a packet, a double click or Enter opens it.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Galley, Key, Layout, Margin, Rect, Sense,
    Stroke, Ui, WidgetInfo, pos2, vec2,
};
use egui::emath::GuiRounding as _;

use crate::engine::Engine;
use crate::gui::filter_field::FilterField;
use crate::gui::frame::W_MD;
use crate::gui::load::{Loader, Task};
use crate::gui::pages::head;
use crate::gui::theme::{self, Palette, medium_font, palette, semibold_font};
use crate::gui::widgets::{self, Kind, Tone};
use crate::gui::{App, Page, icons};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::packets::{
    ConnectionNote, FILTER_EXAMPLES, PacketList, PacketRow, RowBlock, Window, packet_list,
    pcap_file, row_block,
};
use sniffnet_packet_parser::Protocol;

/// A row of the list (`--pkt-h`), with its line of 1 px
const ROW_H: f32 = 26.0;
/// The head of the list (`.pkt th`: 30 px with its line)
const HEAD_H: f32 = 30.0;
/// The bar below the list (`.followbar`: 32 px with its line)
const FOOT_H: f32 = 32.0;
/// How long a new row is marked at its left (`@keyframes arrive`)
const ARRIVE: Duration = Duration::from_millis(1_200);
/// How often the rows are read while packets come
const LIVE: Duration = Duration::from_millis(300);
/// The most rows of one block
const MAX_BLOCK: usize = 500;
/// The field to go to a packet (`.goto input`)
const GOTO_W: f32 = 92.0;
/// Columns aligned to the right: number, time and length (`th.r`)
const RIGHT: [bool; 7] = [true, true, false, false, false, true, false];

/// The widths of the columns (`colgroup`): number, time, source, destination, protocol, length;
/// the info takes the rest
fn columns(width: f32) -> [f32; 7] {
    let part = width * 0.2;
    let fixed = 72.0 + 96.0 + 2.0 * part + 84.0 + 64.0;
    [72.0, 96.0, part, part, 84.0, 64.0, (width - fixed).max(0.0)]
}

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
    /// The packet the page was opened with (`n` of the draft)
    goto_seen: Option<u64>,
    field: FilterField,
    head: Loader<Filters, PacketList>,
    block: Loader<BlockKey, Reply>,
    key: Option<BlockKey>,
    /// A block has come for the filters in use, so an empty list means there are no packets
    received: bool,
    /// When the filters were put in use
    opened: Option<Instant>,
    rows: HashMap<usize, PacketRow>,
    total: usize,
    /// Packets kept and older ones dropped
    kept: usize,
    dropped: u64,
    /// The numbers of the oldest and of the newest packet kept
    numbers: Option<(u64, u64)>,
    /// Stays at the end of the list
    follow: bool,
    /// Goes to the end once the first rows are there (the newest packets are at the bottom)
    to_end: bool,
    was_at_end: bool,
    /// The selected packet (`selN`) and its row
    selected: Option<PacketRow>,
    /// The position of the list to select once its row has been read (keys up and down)
    select_at: Option<usize>,
    /// The position to scroll into view once the height of the list is known
    reveal_at: Option<usize>,
    /// The packet to find
    goto: Option<u64>,
    /// It was typed: a packet that is not in the list is told
    typed: bool,
    goto_text: String,
    /// The newest packet seen, and from which packet on the rows came when
    newest: u64,
    arrivals: Vec<(u64, Instant)>,
    /// The scroll position to take in the next frame
    scroll_to: Option<f32>,
    /// The scroll position of the last frame
    offset: f32,
    first: usize,
    visible: usize,
    height: f32,
    save: Task<Option<Result<String, String>>>,
    /// A short message for the bottom of the window
    message: Option<String>,
}

#[cfg(test)]
impl State {
    pub fn first_row(&self) -> usize {
        self.first
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn selected(&self) -> Option<u64> {
        self.selected.as_ref().map(|r| r.number)
    }

    pub fn visible(&self) -> usize {
        self.visible
    }

    pub fn typed(&self) -> &str {
        &self.field.text
    }
}

impl State {
    fn open(&mut self, filters: Filters, goto: Option<u64>, live: bool, finished: bool) {
        self.field.set(&filters.display);
        self.filters = Some(filters);
        self.rows.clear();
        self.total = 0;
        self.key = None;
        self.received = false;
        self.opened = Some(Instant::now());
        self.goto = goto;
        self.typed = false;
        if goto.is_none() {
            self.selected = None;
        }
        self.select_at = None;
        self.reveal_at = None;
        self.first = 0;
        self.scroll_to = Some(0.0);
        self.follow = goto.is_none() && live;
        // as in a packet analyzer the newest packets are at the bottom, also while paused; a file
        // that has been read starts at its first packet
        self.to_end = goto.is_none() && !finished;
        self.was_at_end = false;
        self.newest = 0;
        self.arrivals.clear();
    }

    /// How long the first block has been awaited
    fn waited(&self) -> Duration {
        self.opened.map_or(Duration::ZERO, |at| at.elapsed())
    }

    /// Scrolling up ends following; coming back to the end takes it up again
    fn after_scrolling(&mut self, at_end: bool, explicit: bool, live: bool) {
        if !explicit {
            if self.follow && !at_end {
                self.follow = false;
            } else if !self.follow && live && at_end && !self.was_at_end {
                self.follow = true;
            }
        }
        self.was_at_end = at_end;
    }

    /// A key moves the list: Home to its start, End to its end (and on with the new packets)
    fn step(&mut self, step: Step, live: bool) {
        let max = self.max_offset();
        let to = match step {
            Step::First => 0.0,
            Step::Last => max,
            Step::Rows(rows) => (self.offset + rows as f32 * ROW_H).clamp(0.0, max),
        };
        if step == Step::Last {
            self.selected = None;
            self.select_at = None;
        }
        self.follow = to >= max && live;
        self.scroll_to = Some(to);
    }

    fn max_offset(&self) -> f32 {
        (self.total as f32 * ROW_H - self.height).max(0.0)
    }

    /// Takes a block of rows that has arrived
    fn receive(&mut self, reply: Reply) {
        self.received = true;
        let goto = self.goto.take();
        let Some(block) = reply.block else {
            self.rows.clear();
            self.total = 0;
            return;
        };
        let newest = block.numbers.1;
        if self.newest > 0 && newest > self.newest {
            self.arrivals.push((self.newest + 1, Instant::now()));
        }
        self.newest = self.newest.max(newest);
        self.arrivals.retain(|(_, at)| at.elapsed() < ARRIVE);
        self.total = block.total;
        self.kept = block.kept;
        self.dropped = block.dropped;
        self.numbers = (block.kept > 0).then_some(block.numbers);
        self.rows = block
            .rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| (block.from + i, r))
            .collect();
        if let Some(asked) = goto {
            self.goto_found(asked, block.goto);
        } else if self.follow || self.to_end {
            // at once: the scroll area reaches the end only after drawing the frame, at a place
            // whose rows may have just gone
            self.to_end = false;
            self.scroll_to = Some(self.max_offset());
        } else if let Some(now) = block.anchor
            && now != reply.first
        {
            // older packets were dropped from the store: the same packets stay in view
            let moved = now as f32 - reply.first as f32;
            self.scroll_to = Some(((self.first as f32 + moved) * ROW_H).max(0.0));
        }
    }

    /// The packet asked for is selected and in view; one that is not in the list is told when it
    /// was typed
    fn goto_found(&mut self, asked: u64, found: Option<(usize, u64)>) {
        self.follow = false;
        self.to_end = false;
        let typed = std::mem::take(&mut self.typed);
        match found {
            Some((position, number)) if number == asked => {
                self.selected = self.rows.get(&position).cloned();
                self.reveal(position);
            }
            _ if typed => self.message = Some(asked.to_string()),
            _ => {}
        }
    }

    /// Scrolls a row into view, a third down the list, unless it is well in view (`scrollToN`)
    fn reveal(&mut self, position: usize) {
        self.reveal_at = Some(position);
    }

    /// Scrolls to the row to reveal, once the height of the list is known
    fn take_reveal(&mut self) {
        let Some(position) = self.reveal_at.take() else {
            return;
        };
        let y = position as f32 * ROW_H;
        let offset = self.scroll_to.unwrap_or(self.offset);
        if y < offset || y > offset + self.height - ROW_H * 3.0 {
            self.scroll_to = Some((y - self.height / 3.0).clamp(0.0, self.max_offset()));
        }
    }

    /// The keys up and down select the next row (the first or the last without a selection)
    fn move_selection(&mut self, down: bool) {
        if self.total == 0 {
            return;
        }
        let current = self.selected.as_ref().and_then(|s| {
            self.rows
                .iter()
                .find(|(_, r)| r.number == s.number)
                .map(|(position, _)| *position)
        });
        let target = match (current, &self.selected) {
            (Some(position), _) if down => (position + 1).min(self.total - 1),
            (Some(position), _) => position.saturating_sub(1),
            // the selected row is not read: first back to it
            (None, Some(selected)) => {
                self.goto = Some(selected.number);
                return;
            }
            (None, None) if down => 0,
            (None, None) => self.total - 1,
        };
        self.follow = false;
        self.select_at = Some(target);
        self.reveal(target);
    }

    /// Selects the row waited for, once it has been read
    fn take_select_at(&mut self) {
        if let Some(position) = self.select_at
            && let Some(row) = self.rows.get(&position)
        {
            self.selected = Some(row.clone());
            self.select_at = None;
        }
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
        if self.follow || self.to_end {
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

    /// The mark of a row that has just come: how strong it still is
    fn arrival(&self, number: u64) -> Option<f32> {
        let (_, at) = self
            .arrivals
            .iter()
            .rev()
            .find(|(from, _)| number >= *from)?;
        let done = at.elapsed().as_secs_f32() / ARRIVE.as_secs_f32();
        // `ease-out`: it fades fast at first
        (done < 1.0).then(|| (1.0 - done).powi(2))
    }
}

fn load_block(
    state: &mut State,
    ctx: &egui::Context,
    t: Lang,
    engine: &Engine,
    index: &Arc<crate::view::filter_index::FilterIndex>,
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

/// The page with these filters
fn packets_page(display: &str, connections: &str) -> Page {
    Page::Packets {
        filter: display.trim().to_owned(),
        connections: connections.trim().to_owned(),
        goto: None,
    }
}

/// Where a key moves the list
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    First,
    Last,
    Rows(i32),
}

/// What the user asked for in this frame
#[derive(Default)]
struct Actions {
    save: bool,
    statistics: bool,
    capture: bool,
    /// To the page of the connections of the connection filter
    connections: bool,
    connection: Option<AddressPortPair>,
    stream: Option<u64>,
    /// All packets, without the filters
    all: bool,
    apply: bool,
    open: Option<u64>,
    example: Option<String>,
    call: Option<crate::view::calls::CallKey>,
}

#[allow(clippy::similar_names)]
pub fn show(app: &mut App, ui: &mut Ui, display: &str, connections: &str, goto: Option<u64>) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let running = app.live.running;
    let mut actions = Actions::default();
    if !running {
        head(ui, &t.get("nav-packets"), None, |_| {});
        if widgets::no_capture(ui, t) {
            app.go(Page::Capture);
        }
        return;
    }
    let filters = Filters {
        display: display.to_owned(),
        connections: connections.to_owned(),
        capture: app.live.capture_id,
    };
    let live = running && !app.live.frozen && !app.live.finished;
    if app.pages.packets.filters.as_ref() != Some(&filters) {
        app.pages
            .packets
            .open(filters.clone(), goto, live, app.live.finished);
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
    state.take_select_at();
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
    if let Some(Some(result)) = state.save.take() {
        let text = match result {
            Ok(path) => t.get1("gui-saved", "path", path),
            Err(e) => t.get1("gui-save-failed", "error", e),
        };
        state.message = None;
        app.toast(text);
    }

    let state = &mut app.pages.packets;
    let head_known = state.head.value_of(&filters).is_some();
    let list = state.head.value_of(&filters);
    let has_store = list.is_some_and(|l| l.store);
    let filtered = !display.is_empty() || !connections.is_empty();
    head(
        ui,
        &t.get("nav-packets"),
        Some(&t.get("packets-subtitle")),
        |ui| {
            if has_store {
                let save = if filtered {
                    "packets-download-filtered"
                } else {
                    "packets-download"
                };
                actions.save = widgets::icon_text_button(
                    ui,
                    Some(icons::DOWNLOAD),
                    &t.get(save),
                    Kind::Secondary,
                    false,
                )
                .clicked();
                actions.statistics = widgets::icon_text_button(
                    ui,
                    Some(icons::STATISTICS),
                    &t.get("nav-statistics"),
                    Kind::Secondary,
                    false,
                )
                .clicked();
            }
        },
    );
    if let Some(list) = list {
        if !list.unavailable.is_empty() {
            widgets::notice_text(ui, &list.unavailable);
        }
        match selection_note(
            ui,
            t,
            list.connection.as_ref(),
            list.connections,
            connections,
            Here::Packets,
        ) {
            Some(NoteClick::Connection(key)) => actions.connection = Some(key),
            Some(NoteClick::Statistics) => actions.statistics = true,
            Some(NoteClick::Stream(n)) => actions.stream = Some(n),
            Some(NoteClick::Connections) => actions.connections = true,
            Some(NoteClick::All) => actions.all = true,
            Some(NoteClick::Packets) | None => {}
        }
    }

    // the list stands while it is not yet known whether there is one, with a note that it loads;
    // a filter that cannot be read shows in its place
    let error = list
        .and_then(|l| l.error.as_ref())
        .map(|e| e.message.clone());
    if has_store || !head_known {
        list_card(app, ui, filtered, live, error.as_deref(), &mut actions);
    }
    if let Some(asked) = app.pages.packets.message.take() {
        let state = &app.pages.packets;
        let text = match (state.numbers, asked.parse::<u64>()) {
            (Some((first, last)), Ok(n)) if n < first || n > last => t.fmt(
                "packets-goto-missing",
                &[
                    ("number", asked.clone().into()),
                    ("first", format::count(t, u128::from(first)).into()),
                    ("last", format::count(t, u128::from(last)).into()),
                ],
            ),
            _ => t.get1("packets-goto-outside", "number", asked),
        };
        app.toast(text);
    }
    run_actions(app, &ctx, actions, display, connections);
}

/// Where the note on the selection stands (`here` of `connNote`): it leaves out the way to itself
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Here {
    Packets,
    Statistics,
}

/// What was clicked in the note on the selection
pub(super) enum NoteClick {
    Connection(AddressPortPair),
    Packets,
    Statistics,
    Stream(u64),
    Connections,
    All,
}

/// The note on the packets a page shows (`connNote`): those of one connection, or of the
/// connections of a connection filter, with the ways to the pages about them
pub(super) fn selection_note(
    ui: &mut Ui,
    t: Lang,
    connection: Option<&ConnectionNote>,
    count: Option<usize>,
    connections: &str,
    here: Here,
) -> Option<NoteClick> {
    let mut click = None;
    if let Some(note) = connection {
        widgets::notice_icon(ui, Tone::Info, Some(icons::CONNECTIONS), |ui| {
            ui.label(t.get("note-connection"));
            if widgets::mono_link(ui, &note.label).clicked() {
                click = Some(NoteClick::Connection(note.key));
            }
            ui.label(t.get("note-both"));
            if here != Here::Packets {
                widgets::dot(ui);
                if widgets::link(ui, t.get("nav-packets")).clicked() {
                    click = Some(NoteClick::Packets);
                }
            }
            if here != Here::Statistics {
                widgets::dot(ui);
                if widgets::link(ui, t.get("nav-statistics")).clicked() {
                    click = Some(NoteClick::Statistics);
                }
            }
            if let Some(stream) = note.stream {
                widgets::dot(ui);
                if widgets::link(ui, t.get("packet-stream")).clicked() {
                    click = Some(NoteClick::Stream(stream));
                }
            }
            widgets::dot(ui);
            if widgets::link(ui, t.get("note-all")).clicked() {
                click = Some(NoteClick::All);
            }
        });
    } else if let Some(count) = count {
        widgets::notice_icon(ui, Tone::Info, Some(icons::CONNECTIONS), |ui| {
            ui.label(t.get1("note-connections", "count", format::count(t, count as u128)));
            widgets::inline_code(ui, connections);
            widgets::dot(ui);
            if widgets::link(ui, t.get("nav-connections")).clicked() {
                click = Some(NoteClick::Connections);
            }
            widgets::dot(ui);
            if widgets::link(ui, t.get("note-all")).clicked() {
                click = Some(NoteClick::All);
            }
        });
    }
    click
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
    } else if let Some(n) = actions.stream {
        app.go(Page::Stream(n));
    } else if let Some(key) = actions.call {
        app.go(Page::Call(key));
    } else if actions.all {
        app.go(packets_page("", ""));
    } else if actions.apply {
        let page = packets_page(&app.pages.packets.field.text, connections);
        app.go(page);
    } else if let Some(example) = actions.example {
        app.go(packets_page(&example, connections));
    } else if let Some(number) = actions.open {
        app.pages.packets.follow = false;
        // back from the packet, the list shows it selected
        if let Page::Packets { goto, .. } = &mut app.page {
            *goto = Some(number);
        }
        app.pages.packets.goto_seen = Some(number);
        app.go(Page::Packet(number));
    }
}

/// The card of the list (`.pktcard`): the filter and the field to go to a packet, the help, the
/// selected packet, the list and the bar below it
fn list_card(
    app: &mut App,
    ui: &mut Ui,
    filtered: bool,
    live: bool,
    error: Option<&str>,
    actions: &mut Actions,
) {
    let t = app.t;
    let engine = app.engine.clone();
    let frozen = app.live.running && app.live.frozen;
    let state = &mut app.pages.packets;
    let focus_goto = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, Key::G));
    let mut go = false;
    widgets::bare_card(ui, |ui| {
        let goto_label = t.get("packets-goto");
        let label_w = ui
            .painter()
            .layout_no_wrap(
                goto_label.clone(),
                FontId::proportional(theme::FS_SM),
                Color32::PLACEHOLDER,
            )
            .size()
            .x;
        let out = state.field.bar_with(
            ui,
            t,
            &engine,
            false,
            &t.get("packets-filter-placeholder"),
            &t.get("packets-filter-help"),
            label_w + 6.0 + GOTO_W,
            |ui| {
                let p = palette(ui.ctx());
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(widgets::small(&goto_label).color(p.text_3));
                let id = egui::Id::NULL.with("packets-goto");
                let mut text = std::mem::take(&mut state.goto_text);
                let field = widgets::input(
                    ui,
                    &mut text,
                    id,
                    vec2(GOTO_W, 28.0),
                    true,
                    &t.get("packets-goto-placeholder"),
                );
                state.goto_text = text;
                let response = field
                    .response
                    .response
                    .clone()
                    .on_hover_text(t.get("packets-goto-key"));
                if focus_goto {
                    response.request_focus();
                    let all = egui::text::CCursorRange::select_all(&field.galley);
                    let mut edit = field.state;
                    edit.cursor.set_char_range(Some(all));
                    edit.store(ui.ctx(), id);
                }
                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    go = true;
                }
            },
        );
        actions.apply = out.apply;
        actions.example = state.field.extras(
            ui,
            t,
            false,
            &t.get("packets-filter-help-text"),
            &FILTER_EXAMPLES,
            |ui| color_keys(ui, t),
        );
        if let Some(selected) = state.selected.clone() {
            selection_bar(ui, t, &selected, actions);
        }
        let body = (ui.available_height() - HEAD_H - FOOT_H).max(4.0 * ROW_H);
        if let Some(error) = error {
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), HEAD_H + body), Sense::hover());
            let mut child = ui.new_child(
                egui::UiBuilder::new().max_rect(rect.shrink2(vec2(theme::SP3, theme::SP3))),
            );
            widgets::error(&mut child, error);
        } else {
            table(ui, state, t, live, filtered, body, actions);
        }
        follow_bar(ui, t, state, filtered, live, frozen);
    });
    if go {
        let typed = state.goto_text.trim().to_owned();
        match (typed.parse::<u64>(), state.numbers) {
            (Ok(n), Some((first, last))) if n < first || n > last => {
                state.message = Some(typed);
            }
            (Ok(n), _) => {
                state.goto = Some(n);
                state.typed = true;
            }
            (Err(_), _) if !typed.is_empty() => state.message = Some(typed),
            _ => {}
        }
    }
}

/// The meaning of the colors of the rows, on the colors themselves (`.rowkeys`)
pub(super) fn color_keys(ui: &mut Ui, t: Lang) {
    let p = palette(ui.ctx());
    ui.add_space(theme::SP3 - theme::SP2);
    ui.spacing_mut().interact_size.y = 20.0;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        ui.label(widgets::small(t.get("packets-colors")).color(p.text_3));
        for (class, text) in [
            ("bad", t.get("packets-color-bad")),
            ("tcp-control", t.get("packets-color-tcp")),
            ("dns", "DNS".to_owned()),
            ("local", t.get("packets-color-local")),
            ("tls", "TLS".to_owned()),
            ("http", "HTTP".to_owned()),
            ("icmp", "ICMP".to_owned()),
            ("udp", t.get("packets-color-udp")),
            ("voip", t.get("packets-color-voip")),
        ] {
            let galley =
                ui.painter()
                    .layout_no_wrap(text, FontId::proportional(theme::FS_XS), p.text);
            let (rect, _) =
                ui.allocate_exact_size(vec2(galley.size().x + 12.0, 20.0), Sense::hover());
            ui.painter().rect(
                rect,
                CornerRadius::same(theme::RADIUS_SM),
                p.row(class).unwrap_or(p.surface),
                Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );
            let at = pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0);
            ui.painter()
                .galley(at.round_to_pixels(ui.pixels_per_point()), galley, p.text);
        }
    });
}

/// The bar of the selected packet (`.selbar`): its number and line, then the ways on
fn selection_bar(ui: &mut Ui, t: Lang, row: &PacketRow, actions: &mut Actions) {
    let p = palette(ui.ctx());
    Frame::new()
        .fill(p.accent_bg)
        .inner_margin(Margin::symmetric(theme::SP3 as i8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let size = vec2(ui.available_width(), widgets::CONTROL_SM_H);
            ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                ui.label(
                    egui::RichText::new(t.get1("packets-selected", "number", row.number))
                        .font(semibold_font(theme::FS_SM))
                        .color(p.text),
                );
                let info: String = if row.info.chars().count() > 70 {
                    row.info.chars().take(70).chain(['…']).collect()
                } else {
                    row.info.clone()
                };
                let line = format!("{} · {info}", row.protocol);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = theme::SP2;
                    // right to left: the last way first
                    if let Some(key) = row.call
                        && widgets::icon_text_button(
                            ui,
                            Some(icons::PHONE),
                            &t.get("packets-sel-call"),
                            Kind::Secondary,
                            true,
                        )
                        .clicked()
                    {
                        actions.call = Some(key);
                    }
                    if let Some(key) = row.connection {
                        if matches!(key.protocol, Protocol::Tcp | Protocol::Udp)
                            && widgets::small_button(ui, t.get("packet-stream"), Kind::Secondary)
                                .clicked()
                        {
                            actions.stream = Some(row.number);
                        }
                        if widgets::small_button(ui, t.get("connection-title"), Kind::Secondary)
                            .clicked()
                        {
                            actions.connection = Some(key);
                        }
                    }
                    if open_button(ui, &t.get("packets-open"), "Enter").clicked() {
                        actions.open = Some(row.number);
                    }
                    // the line of the packet right after its number, in the room the ways leave
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(line)
                                    .font(theme::mono_font(theme::FS_SM))
                                    .color(p.text),
                            )
                            .truncate(),
                        );
                    });
                });
            });
        });
    widgets::rule(ui);
}

/// The small primary button with its key (`.btn.sm.primary` with `kbd`)
fn open_button(ui: &mut Ui, text: &str, key: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let label =
        ui.painter()
            .layout_no_wrap(text.to_owned(), medium_font(theme::FS_SM), p.accent_text);
    let kbd = ui
        .painter()
        .layout_no_wrap(key.to_owned(), theme::mono_font(11.0), p.text_2);
    let kbd_size = vec2(kbd.size().x + 12.0, 11.0 + 4.0 + 3.0);
    let width = theme::SP2 + label.size().x + 6.0 + kbd_size.x + theme::SP2;
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, widgets::CONTROL_SM_H), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text));
    let fill = if response.hovered() {
        p.accent_hover
    } else {
        p.accent
    };
    let radius = CornerRadius::same(theme::RADIUS);
    ui.painter().rect_filled(rect, radius, fill);
    let ppp = ui.pixels_per_point();
    let mut x = rect.left() + theme::SP2;
    let at = pos2(x, rect.center().y - label.size().y / 2.0).round_to_pixels(ppp);
    let label_w = label.size().x;
    ui.painter().galley(at, label, p.accent_text);
    x += label_w + 6.0;
    let key_rect = Rect::from_min_size(pos2(x, rect.center().y - kbd_size.y / 2.0), kbd_size)
        .round_to_pixels(ppp);
    let small = CornerRadius::same(theme::RADIUS_SM);
    // the lower border is 2 px (`border-bottom-width: 2px`)
    ui.painter().rect_filled(key_rect, small, p.border_strong);
    ui.painter().rect_filled(
        Rect::from_min_max(key_rect.min + vec2(1.0, 1.0), key_rect.max - vec2(1.0, 2.0)),
        CornerRadius::same(theme::RADIUS_SM - 1),
        p.surface,
    );
    let at = pos2(
        key_rect.left() + 6.0,
        key_rect.top() + 1.0 + (key_rect.height() - 3.0 - kbd.size().y) / 2.0,
    )
    .round_to_pixels(ppp);
    ui.painter().galley(at, kbd, p.text_2);
    widgets::focus_ring(ui, &response, theme::RADIUS);
    response
}

/// The keys of the list (`keydown` of the draft), unless a text field has the focus
fn keys(ui: &Ui, state: &mut State, live: bool, actions: &mut Actions) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let page = state.visible.saturating_sub(1).max(1) as i32;
    let plain = egui::Modifiers::NONE;
    let (down, up, enter, step) = ui.input_mut(|i| {
        let step = [
            (Key::Home, Step::First),
            (Key::End, Step::Last),
            (Key::PageUp, Step::Rows(-page)),
            (Key::PageDown, Step::Rows(page)),
        ]
        .into_iter()
        .find_map(|(key, step)| i.consume_key(plain, key).then_some(step));
        (
            i.consume_key(plain, Key::ArrowDown),
            i.consume_key(plain, Key::ArrowUp),
            i.consume_key(plain, Key::Enter),
            step,
        )
    });
    if down || up {
        state.move_selection(down);
    }
    if enter && let Some(selected) = &state.selected {
        actions.open = Some(selected.number);
    }
    if let Some(step) = step {
        state.step(step, live);
    }
}

/// One line of text in a cell, cut with "…" where it does not fit (`text-overflow: ellipsis`)
fn cell_text(ui: &Ui, text: &str, width: f32, color: Color32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(theme::mono_font(theme::FS_SM), color),
    );
    job.wrap = TextWrapping::truncate_at_width(width.max(1.0));
    ui.painter().layout_job(job)
}

/// The color of a row under the pointer (`filter: brightness(.97)`, dark 1.18)
fn brighter(color: Color32, dark: bool) -> Color32 {
    let f = if dark { 1.18 } else { 0.97 };
    let c = |v: u8| (f32::from(v) * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(c(color.r()), c(color.g()), c(color.b()))
}

/// The head of the list (`.pkt th`)
fn table_head(ui: &mut Ui, t: Lang, widths: &[f32; 7]) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEAD_H), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - 1.0)),
        0.0,
        p.surface_2,
    );
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, p.border),
    );
    let heads = [
        t.get("packet-number"),
        t.get("packet-time"),
        t.get("conn-source"),
        t.get("conn-destination"),
        t.get("conn-protocol"),
        t.get("packet-length"),
        t.get("packet-info"),
    ];
    let mut x = rect.left();
    for ((text, width), right) in heads.iter().zip(widths).zip(RIGHT) {
        let galley = painter.layout_no_wrap(text.clone(), medium_font(theme::FS_SM), p.text_3);
        let left = if right {
            x + width - theme::SP2 - galley.size().x
        } else {
            x + theme::SP2
        };
        let at = pos2(left, rect.top() + (HEAD_H - 1.0 - galley.size().y) / 2.0);
        painter.galley(at.round_to_pixels(ui.pixels_per_point()), galley, p.text_3);
        x += width;
    }
}

fn table(
    ui: &mut Ui,
    state: &mut State,
    t: Lang,
    live: bool,
    filtered: bool,
    body: f32,
    actions: &mut Actions,
) {
    let p = *palette(ui.ctx());
    let width = ui.available_width();
    let widths = columns(width);
    table_head(ui, t, &widths);
    if !state.received || state.total == 0 {
        let (rect, _) = ui.allocate_exact_size(vec2(width, body), Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        if state.received {
            let text = if filtered {
                "packets-none-filtered"
            } else {
                "packets-empty"
            };
            widgets::empty(&mut child, &t.get(text));
        } else {
            widgets::loading(&mut child, state.waited(), &t.get("packets-loading"));
        }
        return;
    }
    keys(ui, state, live, actions);
    state.height = body;
    state.take_reveal();
    let explicit = state.scroll_to.is_some();
    let dark = ui.visuals().dark_mode;
    ui.spacing_mut().scroll.floating_allocated_width = 0.0;
    // the scroll area counts the rows with this spacing
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut area = egui::ScrollArea::vertical()
        .id_salt("packet-list")
        .auto_shrink(false)
        .max_height(body)
        .min_scrolled_height(body)
        .animated(false)
        .stick_to_bottom(state.follow);
    if let Some(offset) = state.scroll_to.take() {
        area = area.vertical_scroll_offset(offset);
    }
    let mut click = None;
    let mut open = None;
    let mut animating = false;
    let output = area.show_rows(ui, ROW_H, state.total, |ui, range| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for position in range {
            let (rect, response) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
            let Some(r) = state.rows.get(&position) else {
                // a row not read yet still takes its height, or the list jumps
                continue;
            };
            response
                .widget_info(|| WidgetInfo::labeled(egui::Role::Row, true, r.number.to_string()));
            let selected = state
                .selected
                .as_ref()
                .is_some_and(|s| s.number == r.number);
            let base = p.row(r.color).unwrap_or(p.surface);
            let fill = if selected {
                p.accent
            } else if response.hovered() {
                brighter(base, dark)
            } else {
                base
            };
            let fg = if selected { p.accent_text } else { p.text };
            let painter = ui.painter();
            // the line between the rows is the color of the card (`border-bottom: 1px surface`)
            let cells = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - 1.0));
            painter.rect_filled(cells, 0.0, fill);
            if let Some(strength) = state.arrival(r.number) {
                animating = true;
                let mark = Rect::from_min_size(cells.min, vec2(3.0, cells.height()));
                painter.rect_filled(mark, 0.0, p.accent.gamma_multiply(strength));
            }
            row_cells(ui, r, cells, &widths, fg);
            if response.double_clicked() {
                open = Some(r.number);
            } else if response.clicked() {
                click = Some(position);
            }
        }
    });
    if animating {
        ui.ctx().request_repaint();
    }
    if let Some(position) = click {
        state.selected = state.rows.get(&position).cloned();
        state.select_at = None;
        state.follow = false;
    }
    if let Some(number) = open {
        actions.open = Some(number);
    }
    state.height = output.inner_rect.height();
    state.visible = (state.height / ROW_H).ceil() as usize;
    state.offset = output.state.clamped_offset().y;
    state.first = (state.offset / ROW_H).floor() as usize;
    let at_end = state.offset >= state.max_offset() - 1.0;
    state.after_scrolling(at_end, explicit, live);
}

/// The cells of a row (`.pkt td`): 8 px inside, number, time and length to the right
fn row_cells(ui: &Ui, r: &PacketRow, cells: Rect, widths: &[f32; 7], color: Color32) {
    let texts = [
        r.number.to_string(),
        r.time.clone(),
        r.src.clone(),
        r.dst.clone(),
        r.protocol.to_owned(),
        r.length.to_string(),
        r.info.clone(),
    ];
    let ppp = ui.pixels_per_point();
    let mut x = cells.left();
    for ((text, width), right) in texts.iter().zip(widths).zip(RIGHT) {
        let room = width - 2.0 * theme::SP2;
        if room > 0.0 {
            let galley = cell_text(ui, text, room, color);
            let left = if right {
                x + width - theme::SP2 - galley.size().x
            } else {
                x + theme::SP2
            };
            let at = pos2(left, cells.center().y - galley.size().y / 2.0).round_to_pixels(ppp);
            ui.painter().galley(at, galley, color);
        }
        x += width;
    }
}

/// The bar below the list (`.followbar`): packets kept and dropped, those of the selection; at
/// the right whether the list follows the new packets
fn follow_bar(ui: &mut Ui, t: Lang, state: &State, filtered: bool, live: bool, frozen: bool) {
    let p = *palette(ui.ctx());
    widgets::rule(ui);
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), FOOT_H - 1.0), Sense::hover());
    let mut job = LayoutJob::default();
    let num = TextFormat::simple(theme::mono_font(theme::FS_XS), p.text_2);
    let text = TextFormat::simple(FontId::proportional(theme::FS_XS), p.text_3);
    let mut part = |n: String, words: String, first: bool| {
        if !first {
            job.append(" · ", 0.0, text.clone());
        }
        job.append(&n, 0.0, num.clone());
        job.append(&format!(" {words}"), 0.0, text.clone());
    };
    part(
        format::count(t, state.kept as u128),
        t.get1("packets-stored", "count", state.kept as u64),
        true,
    );
    part(
        format::count(t, u128::from(state.dropped)),
        t.get("packets-older"),
        false,
    );
    if filtered && state.received {
        part(
            format::count(t, state.total as u128),
            t.get1("packets-hits", "count", state.total as u64),
            false,
        );
    }
    let ppp = ui.pixels_per_point();
    let galley = ui.painter().layout_job(job);
    let at = pos2(
        rect.left() + theme::SP3,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter()
        .galley(at.round_to_pixels(ppp), galley, p.text_3);
    // `hide-md`
    if ui.ctx().content_rect().width() < W_MD {
        return;
    }
    let right = rect.right() - theme::SP3;
    if live && state.follow {
        status(
            ui,
            &p,
            right,
            rect.center().y,
            icons::ACTIVITY,
            &t.get("packets-live"),
            false,
        );
    } else if live {
        let galley = ui.painter().layout_no_wrap(
            t.get("packets-follow-hint"),
            FontId::proportional(theme::FS_XS),
            p.text_3,
        );
        let at = pos2(
            right - galley.size().x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter()
            .galley(at.round_to_pixels(ppp), galley, p.text_3);
    } else if frozen {
        status(
            ui,
            &p,
            right,
            rect.center().y,
            icons::PAUSE,
            &t.get("packets-paused"),
            true,
        );
    }
}

/// A state with its icon (`.st.ok`, `.st.warn`), its right edge at `right`
fn status(
    ui: &Ui,
    p: &Palette,
    right: f32,
    y: f32,
    icon: egui::ImageSource<'static>,
    text: &str,
    warn: bool,
) {
    let color = if warn { p.warn } else { p.ok };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), medium_font(theme::FS_XS), color);
    // `.st`: 22 px high, 6 px left (none for `.ok`) and 8 px right, 4 px after the icon
    let left_pad = if warn { 6.0 } else { 0.0 };
    let width = left_pad + 14.0 + 4.0 + galley.size().x + 8.0;
    let rect = Rect::from_min_size(pos2(right - width, y - 11.0), vec2(width, 22.0));
    if warn {
        ui.painter().rect(
            rect,
            CornerRadius::same(11),
            p.warn_bg,
            Stroke::new(1.0, p.warn_line),
            egui::StrokeKind::Inside,
        );
    }
    let icon_rect = Rect::from_min_size(pos2(rect.left() + left_pad, y - 7.0), vec2(14.0, 14.0));
    widgets::paint_icon(ui, icon, icon_rect, color);
    let at = pos2(icon_rect.right() + 4.0, y - galley.size().y / 2.0);
    ui.painter()
        .galley(at.round_to_pixels(ui.pixels_per_point()), galley, color);
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use super::*;

    fn state() -> State {
        State {
            received: true,
            total: 1_000,
            height: 240.0,
            follow: true,
            was_at_end: true,
            ..State::default()
        }
    }

    /// The rows `from..total` of a list of `total` rows, as a block that has arrived
    fn block(from: usize, total: usize) -> Reply {
        let row = |i: usize| PacketRow {
            number: i as u64 + 1,
            time: String::new(),
            src: String::new(),
            dst: String::new(),
            protocol: "TCP",
            length: 60,
            info: String::new(),
            connection: None,
            color: "",
            call: None,
        };
        Reply {
            block: Some(RowBlock {
                total,
                from,
                rows: (from..total).map(row).collect(),
                goto: None,
                anchor: None,
                kept: total,
                dropped: 0,
                numbers: (1, total as u64),
            }),
            first: 0,
        }
    }

    fn list_of(state: State) -> Harness<'static, State> {
        Harness::builder()
            .with_size(egui::vec2(1000.0, 400.0))
            .with_context_setup(crate::gui::theme::install)
            .build_ui_state(
                |ui, state: &mut State| {
                    let mut actions = Actions::default();
                    table(ui, state, Lang::En, true, false, 300.0, &mut actions);
                },
                state,
            )
    }

    /// Before the first block nobody knows that the list is empty, so it does not say so
    #[test]
    fn an_empty_list_is_only_told_once_it_is_known() {
        let mut list = list_of(State {
            opened: Instant::now().checked_sub(Duration::from_secs(1)),
            ..State::default()
        });
        list.run_steps(2);
        assert!(list.query_by_label("Loading packets …").is_some());
        assert!(list.query_by_label("No packets yet.").is_none());

        list.state_mut().receive(block(0, 0));
        list.run_steps(2);
        assert!(list.query_by_label("Loading packets …").is_none());
        assert!(list.query_by_label("No packets yet.").is_some());
    }

    /// More packets came than rows are in view: the frame that takes the block must show its
    /// rows. Waiting for the scroll area to reach the end draws the old place, whose rows have
    /// just gone, and the list flashes empty.
    #[test]
    fn following_shows_a_new_block_in_the_same_frame() {
        let mut list = list_of(State {
            follow: true,
            was_at_end: true,
            ..State::default()
        });
        list.state_mut().receive(block(960, 1_000));
        list.step();
        list.step();
        assert!(list.query_by_label("1000").is_some());

        list.state_mut().receive(block(1_400, 1_500));
        list.step();
        assert!(list.query_by_label("1500").is_some());
        assert!(list.state().follow);
        // and it stays there, with the frames between two blocks
        list.step();
        assert!(list.query_by_label("1500").is_some());
        assert!(list.state().follow);
    }

    /// Not following, the list stays where it is while it grows, and its rows stay
    #[test]
    fn a_new_block_leaves_a_list_that_does_not_follow_in_place() {
        let mut list = list_of(State {
            scroll_to: Some(100.0 * ROW_H),
            received: true,
            ..State::default()
        });
        list.state_mut().receive(block(0, 1_000));
        list.step();
        list.step();
        assert_eq!(list.state().first_row(), 100);
        assert!(list.query_by_label("105").is_some());

        let grown = Reply {
            block: block(0, 1_500).block.map(|mut block| {
                block.rows.truncate(1_000);
                block
            }),
            first: 100,
        };
        list.state_mut().receive(grown);
        list.step();
        assert_eq!(list.state().first_row(), 100);
        assert!(list.query_by_label("105").is_some());
    }

    /// New packets are marked at their left for a moment (`tr.new`)
    #[test]
    fn new_packets_are_marked_for_a_moment() {
        let mut s = State::default();
        s.receive(block(0, 100));
        assert_eq!(s.arrival(50), None);
        s.receive(block(0, 120));
        assert!(s.arrival(101).is_some_and(|a| a > 0.9));
        assert_eq!(s.arrival(100), None);
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
    fn a_jump_keeps_following_as_it_is() {
        let mut s = state();
        // a jump is not the user scrolling up
        s.after_scrolling(false, true, true);
        assert!(s.follow);
    }

    #[test]
    fn the_keys_move_the_list_and_stop_at_its_ends() {
        let mut s = state();
        s.offset = 240.0;
        s.step(Step::Rows(-1), false);
        assert_eq!(s.scroll_to, Some(240.0 - ROW_H));
        s.step(Step::Rows(-100), false);
        assert_eq!(s.scroll_to, Some(0.0));
        s.step(Step::Rows(100_000), false);
        assert_eq!(s.scroll_to, Some(s.max_offset()));
        s.step(Step::First, false);
        assert_eq!(s.scroll_to, Some(0.0));
        s.step(Step::Last, false);
        assert_eq!(s.scroll_to, Some(s.max_offset()));
    }

    /// Home leaves the end, End goes back there and follows again, without a selection
    #[test]
    fn home_ends_following_and_end_takes_it_up_again() {
        let mut s = state();
        s.offset = s.max_offset();
        s.step(Step::First, true);
        assert!(!s.follow);
        s.selected = Some(block(0, 1).block.unwrap().rows[0].clone());
        s.step(Step::Last, true);
        assert!(s.follow);
        assert!(s.selected.is_none());
    }

    /// The keys up and down select the next packet and keep it in view
    #[test]
    fn the_arrows_select_the_next_packet() {
        let mut s = state();
        s.follow = false;
        s.receive(block(0, 1_000));
        s.move_selection(true);
        s.take_select_at();
        assert_eq!(s.selected(), Some(1));
        s.move_selection(true);
        s.take_select_at();
        assert_eq!(s.selected(), Some(2));
        s.move_selection(false);
        s.move_selection(false);
        s.take_select_at();
        assert_eq!(s.selected(), Some(1));
        assert!(!s.follow);
    }

    /// A packet found by its number is selected and scrolled to a third of the list
    #[test]
    fn going_to_a_packet_selects_it() {
        let mut s = state();
        s.receive(block(0, 1_000));
        s.goto_found(500, Some((499, 500)));
        s.take_reveal();
        assert_eq!(s.selected(), Some(500));
        assert_eq!(s.scroll_to, Some(499.0 * ROW_H - 80.0));
        assert!(!s.follow);
        // one that is not in the list is told when it was typed
        s.typed = true;
        s.goto_found(5_000, Some((999, 1_000)));
        assert_eq!(s.message.as_deref(), Some("5000"));
        assert_eq!(s.selected(), Some(500));
    }
}
