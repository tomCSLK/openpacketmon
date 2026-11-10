//! The calls (`pageCalls` and `pageCall` of the draft): a tab of the statistics with the list of
//! calls, and the page of a call with its facts, its flow and its voice streams.

use std::time::Duration;

use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, ImageSource, Layout, Margin, Rect, Response,
    RichText, Sense, Stroke, TextFormat, Ui, UiBuilder,
};
use egui::emath::GuiRounding as _;

use crate::engine::Engine;
use crate::gui::load::Loader;
use crate::gui::pages::{detail_head, head};
use crate::gui::theme::{self, medium_font, mono_font, palette, semibold_font};
use crate::gui::widgets::{self, Col, Kind, PTab, TableSpec, Tile, Width};
use crate::gui::{App, Page, icons};
use crate::view::calls::audio::CallAudio;
use crate::view::calls::{
    self, CallKey, CallList, CallPage, CallRow, FRITZ_CAPTURE, Fact, Flow, FlowRow, Icon, Listen,
    Part, Party, Rate, Status, StreamRow, Tone,
};
use crate::view::i18n::Lang;

pub mod player;

use player::{Ask, Player, Saved};

/// The packets of calls, for the button of the list
pub const CALL_FILTER: &str = "sip or rtp or rtcp";
/// A row of the list of calls: two lines
const ROW_H2: f32 = 48.0;
/// The columns of the flow (`.ladder .lrow`): the time, the lanes, the packet
const TIME_W: f32 = 96.0;
const PACKET_W: f32 = 72.0;
const LANES_MIN: f32 = 420.0;

#[derive(Default)]
pub struct State {
    list: Loader<usize, CallList>,
    page: Loader<(CallKey, usize), Option<CallPage>>,
    /// Listening to the call in view
    pub player: Player,
}

/// The tabs of the statistics (`statTabs` of the draft); `calls` is the number of calls over SIP
pub fn tabs(ui: &mut Ui, t: Lang, current: usize, calls: usize) -> Option<Page> {
    let items = [
        PTab {
            icon: icons::STATISTICS,
            label: t.get("stats-tab-protocols"),
            count: None,
        },
        PTab {
            icon: icons::PHONE,
            label: t.get("stats-tab-calls"),
            count: (calls > 0).then(|| calls.to_string()),
        },
    ];
    match widgets::ptabs(ui, &items, current)? {
        0 => Some(Page::Statistics {
            filter: String::new(),
            connections: String::new(),
        }),
        _ => Some(Page::Calls),
    }
}

fn icon_of(icon: Icon) -> ImageSource<'static> {
    match icon {
        Icon::Phone => icons::PHONE,
        Icon::Check => icons::CHECK_CIRCLE,
        Icon::Cross => icons::X_CIRCLE,
        Icon::Alert => icons::ALERT,
        Icon::Lock => icons::LOCK,
        Icon::Activity => icons::ACTIVITY,
        Icon::Help => icons::HELP,
    }
}

// ---- the list ----------------------------------------------------------------------------------

pub fn show_list(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    if !app.live.running {
        head(ui, &t.get("nav-statistics"), None, |_| {});
        if widgets::no_capture(ui, t) {
            app.go(Page::Capture);
        }
        return;
    }
    let key = app.live.capture_id;
    let state = &mut app.pages.calls;
    state.list.poll();
    let engine = app.engine.clone();
    let live = state.list.value_of(&key).is_some_and(|l| l.live);
    let every = live.then_some(Duration::from_secs(1));
    state
        .list
        .load(&ctx, &key, every, move || calls::call_list(t, &engine));

    let mut go = None;
    head(
        ui,
        &t.get("nav-statistics"),
        Some(&t.get("calls-subtitle")),
        |ui| {
            if widgets::icon_text_button(
                ui,
                Some(icons::PACKETS),
                &t.get("stats-show-packets"),
                Kind::Secondary,
                false,
            )
            .clicked()
            {
                go = Some(packets_of(CALL_FILTER));
            }
        },
    );
    let state = &mut app.pages.calls;
    let count = state.list.value_of(&key).map_or(0, |l| {
        l.rows
            .iter()
            .filter(|r| matches!(r.key, CallKey::Sip(_)))
            .count()
    });
    if let Some(page) = tabs(ui, t, 1, count) {
        go = Some(page);
    }
    match state.list.value_of(&key) {
        Some(list) => {
            if !list.unavailable.is_empty() {
                widgets::notice_text(ui, &list.unavailable);
            }
            if list.rows.is_empty() {
                empty(ui, t);
            } else {
                let tiles: Vec<Tile> = list
                    .tiles
                    .iter()
                    .map(|tile| {
                        Tile {
                            label: tile.label.clone(),
                            value: tile.value.clone(),
                            sub: tile.sub.clone(),
                            warn: tile.warn,
                            ..Tile::default()
                        }
                        .icon(icon_of(tile.icon), None)
                    })
                    .collect();
                widgets::tiles(ui, &tiles, true);
                if let Some(page) = list_card(ui, t, list) {
                    go = Some(page);
                }
                widgets::flush_card(
                    ui,
                    &t.get("calls-help-title"),
                    |_| {},
                    |ui| {
                        cbody(ui, |ui| help(ui, t));
                    },
                );
            }
        }
        None => widgets::loading(ui, state.list.running_for(), &t.get("gui-loading")),
    }
    if let Some(page) = go {
        app.go(page);
    }
}

fn packets_of(filter: &str) -> Page {
    Page::Packets {
        filter: filter.to_owned(),
        connections: String::new(),
        goto: None,
    }
}

/// The body of a card (`.cbody`: 16 px around)
fn cbody<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .inner_margin(Margin::same(theme::SP4 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui)
        })
        .inner
}

/// The page without calls: why there are none and how calls come into a capture
fn empty(ui: &mut Ui, t: Lang) {
    widgets::bare_card(ui, |ui| {
        widgets::empty_state(
            ui,
            Some(icons::PHONE_XL),
            &t.get("calls-empty"),
            &t.get("calls-empty-text"),
            None,
        );
        widgets::rule(ui);
        cbody(ui, |ui| {
            ui.spacing_mut().item_spacing.y = theme::SP2;
            ui.label(RichText::new(t.get("calls-help-title")).font(semibold_font(theme::FS_MD)));
            help(ui, t);
        });
    });
}

/// How calls come into a capture (`voipHelp` of the draft): four points with a bold lead
fn help(ui: &mut Ui, t: Lang) {
    let p = palette(ui.ctx());
    let path = t.get1("calls-help-2", "path", FRITZ_CAPTURE);
    let items = [
        (t.get("calls-help-1-lead"), t.get("calls-help-1")),
        (t.get("calls-help-2-lead"), path),
        (t.get("calls-help-3-lead"), t.get("calls-help-3")),
        (t.get("calls-help-4-lead"), t.get("calls-help-4")),
    ];
    ui.spacing_mut().item_spacing.y = 6.0;
    let width = ui.available_width();
    for (lead, text) in items {
        let mut job = LayoutJob::default();
        let plain = TextFormat {
            font_id: egui::FontId::proportional(theme::FS_SM),
            color: p.text,
            line_height: Some(theme::FS_SM * 1.45),
            ..TextFormat::default()
        };
        job.append(
            &lead,
            0.0,
            TextFormat {
                font_id: semibold_font(theme::FS_SM),
                ..plain.clone()
            },
        );
        job.append(" ", 0.0, plain.clone());
        // the path in the font of addresses
        match text.split_once(FRITZ_CAPTURE) {
            Some((before, after)) => {
                job.append(before, 0.0, plain.clone());
                job.append(
                    FRITZ_CAPTURE,
                    0.0,
                    TextFormat {
                        font_id: mono_font(theme::FS_SM),
                        ..plain.clone()
                    },
                );
                job.append(after, 0.0, plain);
            }
            None => job.append(&text, 0.0, plain),
        }
        job.wrap.max_width = width - 20.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(20.0, theme::FS_SM * 1.45), Sense::hover());
            ui.painter()
                .circle_filled(egui::pos2(rect.left() + 9.0, rect.center().y), 2.2, p.text);
            ui.add(egui::Label::new(job).wrap());
        });
    }
}

/// The card with the list of calls and the registration below it
fn list_card(ui: &mut Ui, t: Lang, list: &CallList) -> Option<Page> {
    let mut go = None;
    widgets::flush_card(
        ui,
        &t.get("calls-list"),
        |ui| widgets::card_note(ui, &t.get("calls-list-hint")),
        |ui| {
            go = table(ui, t, &list.rows);
            if let Some(reg) = &list.registration {
                widgets::rule(ui);
                Frame::new()
                    .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let p = palette(ui.ctx());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(18.0, 14.0), Sense::hover());
                            widgets::paint_icon(
                                ui,
                                icons::CHECK_CIRCLE,
                                Rect::from_min_size(rect.min, egui::vec2(14.0, 14.0)),
                                p.text_3,
                            );
                            let muted = |text: String| widgets::small(text).color(p.text_3);
                            ui.label(muted(format!("{} ", t.get("calls-registered"))));
                            ui.label(
                                RichText::new(&reg.account)
                                    .font(mono_font(theme::FS_SM))
                                    .color(p.text_3),
                            );
                            ui.label(muted(format!(" {} (", reg.text)));
                            if widgets::link_text(
                                ui,
                                widgets::small(t.get1(
                                    "calls-registered-packet",
                                    "number",
                                    reg.packet,
                                ))
                                .color(p.accent),
                            )
                            .clicked()
                            {
                                go = Some(Page::Packet(reg.packet));
                            }
                            let mut end = ").".to_owned();
                            if !reg.digest.is_empty() {
                                end.push(' ');
                                end.push_str(&reg.digest);
                            }
                            ui.label(muted(end));
                        });
                    });
            }
        },
    );
    go
}

/// The list of calls (`table.calls`)
fn table(ui: &mut Ui, t: Lang, rows: &[CallRow]) -> Option<Page> {
    let width = ui.ctx().content_rect().width();
    let md = width >= 1200.0;
    let sm = width >= 900.0;
    // the columns and which of them are shown
    let all = [
        (Col::new(t.get("calls-col-state"), Width::Px(179.0)), true),
        (Col::new(t.get("calls-col-start"), Width::Px(111.0)), sm),
        (
            Col::num(t.get("calls-col-duration"), Width::Px(105.0)),
            true,
        ),
        (Col::new(t.get("calls-col-direction"), Width::Px(125.0)), md),
        (Col::new(t.get("calls-col-from"), Width::Rest(120.0)), true),
        (Col::new(t.get("calls-col-to"), Width::Rest(120.0)), true),
        (Col::new(t.get("calls-col-codec"), Width::Px(104.0)), md),
        (Col::new(t.get("calls-col-quality"), Width::Px(118.0)), true),
        (Col::num(t.get("calls-col-packets"), Width::Px(71.0)), sm),
        (Col::new("", Width::Px(56.0)), sm),
    ];
    let shown: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, (_, on))| *on)
        .map(|(i, _)| i)
        .collect();
    let cols = all
        .into_iter()
        .filter(|(_, on)| *on)
        .map(|(c, _)| c)
        .collect();
    let mut spec = TableSpec::new("calls", cols, ROW_H2, rows.len());
    spec.click = true;
    let mut play = None;
    let out = widgets::table(ui, &spec, |ui, r, c| {
        let row = &rows[r];
        let p = palette(ui.ctx());
        match shown[c] {
            0 => two_lines(
                ui,
                false,
                |ui| state_label(ui, &row.status),
                &row.status.sub,
            ),
            1 => {
                ui.label(widgets::mono(&row.start));
            }
            2 => two_lines(
                ui,
                true,
                |ui| {
                    ui.label(widgets::mono(&row.duration));
                },
                &row.duration_sub,
            ),
            3 => {
                direction(ui, row.outgoing, &row.direction);
            }
            4 => party(ui, &row.from),
            5 => party(ui, &row.to),
            6 => {
                let response = if row.codec.is_empty() {
                    ui.label(RichText::new("–").color(p.text_3))
                } else {
                    ui.add(egui::Label::new(&row.codec).truncate())
                };
                if !row.codec_title.is_empty() {
                    response.on_hover_text(&row.codec_title);
                }
            }
            7 => {
                rate(ui, t, row.rate, &row.rate_title);
            }
            8 => {
                ui.label(widgets::mono(&row.packets));
            }
            _ => {
                if row.playable
                    && widgets::iconbtn(ui, icons::PLAY, &t.get("call-listen"), false, true)
                        .clicked()
                {
                    play = Some(row.key);
                }
            }
        }
    });
    play.or(out.clicked.map(|r| rows[r].key)).map(Page::Call)
}

/// A cell of two lines: what `first` draws, and `second` small and gray below it
fn two_lines(ui: &mut Ui, right: bool, first: impl FnOnce(&mut Ui), second: &str) {
    let p = palette(ui.ctx());
    let rect = ui.max_rect();
    let sub_h = theme::FS_XS * 1.3;
    let block = widgets::LH_BASE + if second.is_empty() { 0.0 } else { sub_h };
    let top = rect.center().y - block / 2.0;
    let layout = if right {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    let mut one = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(
                egui::pos2(rect.left(), top),
                egui::vec2(rect.width(), widgets::LH_BASE),
            ))
            .layout(layout),
    );
    one.spacing_mut().item_spacing.x = 4.0;
    first(&mut one);
    if !second.is_empty() {
        let mut two = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(rect.left(), top + widgets::LH_BASE),
                    egui::vec2(rect.width(), sub_h),
                ))
                .layout(layout),
        );
        two.add(
            egui::Label::new(
                RichText::new(second)
                    .size(theme::FS_XS)
                    .color(p.text_3)
                    .line_height(Some(sub_h)),
            )
            .truncate(),
        );
    }
}

/// The state of a call (`.cstate`): icon and name, green while it goes on, yellow when it failed
fn state_label(ui: &mut Ui, status: &Status) {
    let p = palette(ui.ctx());
    let (color, icon_color) = match status.state.tone() {
        Tone::Run => (p.ok, p.ok),
        Tone::Warn => (p.warn, p.warn),
        Tone::Plain => (p.text_2, p.text_3),
    };
    cstate(
        ui,
        icon_of(status.state.icon()),
        &status.label,
        color,
        icon_color,
    );
}

fn cstate(
    ui: &mut Ui,
    icon: ImageSource<'static>,
    text: &str,
    color: Color32,
    icon_color: Color32,
) {
    let spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 4.0;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(widgets::ICON_SM, widgets::ICON_SM),
        Sense::hover(),
    );
    widgets::paint_icon(ui, icon, rect, icon_color);
    ui.label(
        RichText::new(text)
            .font(medium_font(theme::FS_BASE))
            .color(color),
    );
    ui.spacing_mut().item_spacing.x = spacing;
}

/// The direction (`.dirc`): an arrow up for outgoing, down for incoming
fn direction(ui: &mut Ui, outgoing: bool, text: &str) {
    let p = palette(ui.ctx());
    let icon = if outgoing {
        icons::ARROW_UP
    } else {
        icons::ARROW_DOWN
    };
    let spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 4.0;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(widgets::ICON_SM, widgets::ICON_SM),
        Sense::hover(),
    );
    widgets::paint_icon(ui, icon, rect, p.text_3);
    ui.label(RichText::new(text).color(p.text_2));
    ui.spacing_mut().item_spacing.x = spacing;
}

/// A side of a call (`party()` of the draft): the name, bold if it is one, what it is below
fn party(ui: &mut Ui, party: &Party) {
    let p = palette(ui.ctx());
    let rect = ui.max_rect();
    let width = rect.width().min(210.0);
    let sub_h = theme::FS_XS * 1.3;
    let block = widgets::LH_BASE + if party.sub.is_empty() { 0.0 } else { sub_h };
    let top = rect.center().y - block / 2.0;
    let mut one = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(rect.left(), top),
        egui::vec2(width, widgets::LH_BASE),
    )));
    let text = if party.strong {
        RichText::new(&party.name).font(semibold_font(theme::FS_BASE))
    } else if party.mono {
        RichText::new(&party.name).font(mono_font(theme::FS_SM))
    } else {
        RichText::new(&party.name)
    };
    let response = one.add(egui::Label::new(text.color(p.text)).truncate());
    if !party.title.is_empty() {
        response.on_hover_text(&party.title);
    }
    if !party.sub.is_empty() {
        let mut two = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(
            egui::pos2(rect.left(), top + widgets::LH_BASE),
            egui::vec2(width, sub_h),
        )));
        two.add(
            egui::Label::new(
                RichText::new(&party.sub)
                    .size(theme::FS_XS)
                    .color(p.text_3)
                    .line_height(Some(sub_h)),
            )
            .truncate(),
        );
    }
}

/// The quality of a call or stream (`rateTag`): green, yellow or red, a dash without one
fn rate(ui: &mut Ui, t: Lang, rate: Option<Rate>, title: &str) {
    let p = palette(ui.ctx());
    let Some(rate) = rate else {
        ui.label(RichText::new("–").color(p.text_3));
        return;
    };
    let (tone, icon) = match rate {
        Rate::Good => (widgets::Tone::Ok, icons::CHECK_CIRCLE),
        Rate::Fair => (widgets::Tone::Warn, icons::ALERT),
        Rate::Bad => (widgets::Tone::Err, icons::X_CIRCLE),
    };
    let response = widgets::state(ui, tone, Some(icon), &rate.label(t));
    if !title.is_empty() {
        response.on_hover_text(title);
    }
}

// ---- the page of a call ------------------------------------------------------------------------

/// What was clicked on the page of a call
#[derive(Default)]
struct Clicks {
    go: Option<Page>,
    /// "Als WAV speichern"
    wav: bool,
    player: player::Actions,
}

pub fn show_call(app: &mut App, ui: &mut Ui, key: CallKey) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let crumbs = [
        (
            t.get("nav-statistics"),
            Page::Statistics {
                filter: String::new(),
                connections: String::new(),
            },
        ),
        (t.get("stats-tab-calls"), Page::Calls),
    ];
    if !app.live.running {
        detail_head(
            app,
            ui,
            &crumbs,
            &t.get("call-title"),
            |ui| widgets::title(ui, &t.get("call-title")),
            |_| {},
            |_| {},
        );
        app.pages.calls.player.stop();
        if widgets::no_capture(ui, t) {
            app.go(Page::Capture);
        }
        return;
    }
    let load_key = (key, app.live.capture_id);
    let state = &mut app.pages.calls;
    state.page.poll();
    let engine: Engine = app.engine.clone();
    let live = state
        .page
        .value_of(&load_key)
        .is_some_and(|p| p.as_ref().is_some_and(|p| p.live));
    let every = live.then_some(Duration::from_secs(1));
    state.page.load(&ctx, &load_key, every, move || {
        calls::call_page(t, &engine, key)
    });
    let page = app
        .pages
        .calls
        .page
        .value_of(&load_key)
        .and_then(Option::as_ref)
        .cloned();
    let playable = page
        .as_ref()
        .is_some_and(|p| matches!(p.listen, Listen::Ready { .. }));
    if playable {
        let engine = app.engine.clone();
        app.pages
            .calls
            .player
            .load(&ctx, t, &engine, load_key, every);
    }
    let audio = app.pages.calls.player.audio(&load_key).cloned();
    let mut clicks = Clicks::default();
    let title = page.as_ref().map(|c| {
        (
            if c.sip { "SIP" } else { "RTP" },
            format!("{} → {}", c.from, c.to),
            c.status.clone(),
            c.filter.clone(),
            c.packets_button.clone(),
        )
    });
    detail_head(
        app,
        ui,
        &crumbs,
        &t.get("call-title"),
        |ui| match &title {
            Some((tag, text, _, _, _)) => {
                ui.spacing_mut().item_spacing.x = theme::SP2;
                widgets::tag(ui, tag);
                ui.add(
                    egui::Label::new(RichText::new(text).font(semibold_font(theme::FS_XL)))
                        .truncate(),
                );
            }
            None => widgets::title(ui, &t.get("call-title")),
        },
        |ui| {
            if let Some((_, _, status, _, _)) = &title {
                // a line of text (`.pagehead p`), not a row of controls; the state (`.cstate`,
                // inline-flex) sits on the baseline and makes the line 2 px higher
                ui.spacing_mut().interact_size.y = theme::FS_BASE * 1.5 + 2.0;
                ui.horizontal(|ui| {
                    state_label(ui, status);
                    if !status.sub.is_empty() {
                        ui.label(RichText::new(&status.sub).color(palette(ui.ctx()).text_3));
                    }
                });
            }
        },
        |ui| {
            // right to left: "Als WAV speichern" stands right of "Pakete des Anrufs"
            if playable {
                clicks.wav = widgets::icon_text_button(
                    ui,
                    Some(icons::DOWNLOAD),
                    &t.get("call-wav"),
                    Kind::Secondary,
                    false,
                )
                .clicked();
            }
            if let Some((_, _, _, filter, button)) = &title
                && widgets::icon_text_button(
                    ui,
                    Some(icons::PACKETS),
                    button,
                    Kind::Secondary,
                    false,
                )
                .clicked()
            {
                clicks.go = Some(packets_of(filter));
            }
        },
    );
    match app.pages.calls.page.value_of(&load_key) {
        None => {
            widgets::loading(
                ui,
                app.pages.calls.page.running_for(),
                &t.get("gui-loading"),
            );
        }
        Some(None) => widgets::notice_text(ui, &t.get("call-gone")),
        Some(Some(_)) => {
            if let Some(page) = &page {
                let listen = (&app.pages.calls.player, audio.as_ref());
                call(ui, t, page, listen, &mut clicks);
            }
        }
    }
    if let Some(audio) = &audio {
        listen(app, &ctx, audio, clicks.player, clicks.wav);
    }
    match app.pages.calls.player.saved() {
        Some(Saved::Done(path)) => app.toast(t.get1("gui-saved", "path", path)),
        Some(Saved::Failed(e)) => app.toast(t.get1("gui-save-failed", "error", e)),
        Some(Saved::Cancelled) | None => {}
    }
    if let Some(go) = clicks.go {
        app.go(go);
    }
}

/// Plays, stops, saves; before the first listening or saving the note on § 201 of the criminal
/// code
fn listen(app: &mut App, ctx: &egui::Context, audio: &CallAudio, acts: player::Actions, wav: bool) {
    let confirmed = !app
        .engine
        .lock()
        .conf
        .settings
        .calls
        .law_confirmed
        .is_empty();
    let state = &mut app.pages.calls.player;
    state.act(audio, acts, confirmed);
    if wav {
        if confirmed {
            state.save(ctx, audio);
        } else {
            state.ask = Some(Ask::Wav);
        }
    }
    match player::law_dialog(app, ctx) {
        Some(Ask::Play) => app.pages.calls.player.play_now(audio),
        Some(Ask::Wav) => app.pages.calls.player.save(ctx, audio),
        None => {}
    }
}

fn call(
    ui: &mut Ui,
    t: Lang,
    page: &CallPage,
    listen: (&Player, Option<&CallAudio>),
    clicks: &mut Clicks,
) {
    if let Some((lead, text)) = &page.notice {
        widgets::notice(ui, |ui| {
            ui.label(RichText::new(lead).font(semibold_font(theme::FS_SM)));
            ui.label(text);
        });
    }
    let mut acts = None;
    widgets::split2(
        ui,
        |ui| {
            widgets::card_tight(
                ui,
                &page.card,
                |_| {},
                |ui| facts(ui, t, &page.facts, clicks),
            );
        },
        |ui| acts = listen_card(ui, t, &page.listen, listen.0, listen.1),
    );
    if let Some(acts) = acts {
        clicks.player = acts;
    }
    widgets::flush_card(
        ui,
        &t.get("call-flow"),
        |ui| widgets::card_note(ui, &page.flow.hint),
        |ui| {
            if let Some(go) = flow(ui, t, &page.flow) {
                clicks.go = Some(go);
            }
        },
    );
    widgets::flush_card(
        ui,
        &t.get("call-streams"),
        |_| {},
        |ui| {
            if let Some((heading, text)) = &page.no_streams {
                widgets::empty_state(ui, None, heading, text, None);
                return;
            }
            let p = palette(ui.ctx());
            Frame::new()
                .inner_margin(Margin {
                    left: theme::SP4 as i8,
                    right: theme::SP4 as i8,
                    top: theme::SP3 as i8,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        widgets::small(t.get("call-streams-help"))
                            .color(p.text_2)
                            .line_height(Some(theme::FS_SM * 1.45)),
                    );
                });
            if let Some(go) = streams(ui, t, &page.streams) {
                clicks.go = Some(go);
            }
            widgets::rule(ui);
            Frame::new()
                .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(widgets::small(&page.rtcp).color(p.text_3));
                });
        },
    );
}

/// The facts of a call (`dl.facts`)
fn facts(ui: &mut Ui, t: Lang, facts: &[Fact], clicks: &mut Clicks) {
    let labels: Vec<String> = facts.iter().map(|f| f.label.clone()).collect();
    widgets::facts(ui, &labels, |ui, i| {
        let parts = &facts[i].parts;
        let mut k = 0;
        while k < parts.len() {
            let end = group_end(parts, k);
            if end > k + 1 {
                // the group moves to the next line as one piece and flows on there
                let gap = ui.spacing().item_spacing.x;
                let width: f32 = parts[k..end]
                    .iter()
                    .map(|part| part_width(ui, part) + gap)
                    .sum::<f32>()
                    - gap;
                let left = ui.available_size_before_wrap().x;
                if width > left && left < ui.max_rect().width() - 1.0 {
                    ui.end_row();
                }
                for part in &parts[k..end] {
                    let go = match part {
                        Part::Muted(text) => {
                            // `12001;` without a gap
                            let gap = ui.spacing().item_spacing.x;
                            if text.starts_with([';', ',']) {
                                ui.spacing_mut().item_spacing.x = 0.0;
                            }
                            ui.label(RichText::new(text).color(palette(ui.ctx()).text_3));
                            ui.spacing_mut().item_spacing.x = gap;
                            None
                        }
                        _ => draw_part(ui, t, part),
                    };
                    if let Some(go) = go {
                        clicks.go = Some(go);
                    }
                }
            } else if let Some(go) = draw_part(ui, t, &parts[k]) {
                clicks.go = Some(go);
            }
            k = end;
        }
    });
}

/// A gray text with the packet it names, and the gray text right after (`; für …`), are one
/// piece in the draft (`<span class="muted">… <a>12001</a>; …</span>`): where it ends
fn group_end(parts: &[Part], k: usize) -> usize {
    match (&parts[k], parts.get(k + 1), parts.get(k + 2)) {
        (Part::Muted(_), Some(Part::Packet(_)), Some(Part::Muted(after)))
            if after.starts_with([';', ',']) =>
        {
            k + 3
        }
        (Part::Muted(_), Some(Part::Packet(_)), _) => k + 2,
        _ => k + 1,
    }
}

/// The width of a gray text or a packet number on one line
fn part_width(ui: &Ui, part: &Part) -> f32 {
    let (text, font) = match part {
        Part::Muted(text) => (text.clone(), egui::FontId::proportional(theme::FS_BASE)),
        Part::Packet(number) => (number.to_string(), mono_font(theme::FS_SM)),
        _ => return 0.0,
    };
    ui.painter()
        .layout_no_wrap(text, font, egui::Color32::PLACEHOLDER)
        .size()
        .x
}

/// A piece of text that moves to the next line when it does not fit behind the one before
/// (`flex-wrap` of the draft); a piece longer than the line breaks within itself there
fn unit(ui: &mut Ui, text: RichText) {
    let whole = ui.max_rect().width();
    let width = egui::WidgetText::from(text.clone())
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        )
        .size()
        .x;
    let left = ui.available_size_before_wrap().x;
    if width > left && left < whole - 1.0 {
        ui.end_row();
    }
    let mode = if width <= whole {
        egui::TextWrapMode::Extend
    } else {
        egui::TextWrapMode::Wrap
    };
    ui.add(egui::Label::new(text).wrap_mode(mode));
}

fn draw_part(ui: &mut Ui, t: Lang, part: &Part) -> Option<Page> {
    let p = palette(ui.ctx());
    match part {
        Part::Text(text) => unit(ui, RichText::new(text)),
        Part::Strong(text) => unit(ui, RichText::new(text).font(semibold_font(theme::FS_BASE))),
        Part::Muted(text) => unit(ui, RichText::new(text).color(p.text_3)),
        Part::Mono(text) => {
            widgets::code(ui, text);
        }
        Part::SmallMono(text) => {
            ui.label(
                RichText::new(text)
                    .font(mono_font(theme::FS_SM))
                    .color(p.text_3),
            );
        }
        Part::Packet(number) => {
            if widgets::link_text(
                ui,
                RichText::new(number.to_string())
                    .font(mono_font(theme::FS_SM))
                    .color(p.accent),
            )
            .clicked()
            {
                return Some(Page::Packet(*number));
            }
        }
        Part::Packets(text, filter) => {
            ui.label(RichText::new("·").color(p.text_3));
            if widgets::link_text(ui, RichText::new(text).color(p.accent)).clicked() {
                return Some(packets_of(filter));
            }
        }
        Part::State(status) => {
            state_label(ui, status);
            if !status.sub.is_empty() {
                ui.label(RichText::new(&status.sub).color(p.text_3));
            }
        }
        Part::Dir(outgoing) => {
            let icon = if *outgoing {
                icons::ARROW_UP
            } else {
                icons::ARROW_DOWN
            };
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(widgets::ICON_SM, widgets::ICON_SM),
                Sense::hover(),
            );
            widgets::paint_icon(ui, icon, rect, p.text);
        }
        Part::Running(text) => {
            cstate(ui, icons::ACTIVITY, text, p.ok, p.ok);
        }
    }
    let _ = t;
    None
}

/// The card "Anhören" (`#cplay`): the player, or why there is nothing to hear
fn listen_card(
    ui: &mut Ui,
    t: Lang,
    listen: &Listen,
    player: &Player,
    audio: Option<&CallAudio>,
) -> Option<player::Actions> {
    let hint = match listen {
        Listen::Ready { hint } => hint.clone(),
        Listen::Off { .. } => String::new(),
    };
    let mut acts = None;
    widgets::flush_card(
        ui,
        &t.get("call-listen"),
        |ui| {
            if !hint.is_empty() {
                widgets::card_note(ui, &hint);
            }
        },
        |ui| {
            cbody(ui, |ui| match listen {
                Listen::Off {
                    lock,
                    heading,
                    text,
                } => {
                    let icon = if *lock {
                        icons::LOCK_XL
                    } else {
                        icons::PHONE_XL
                    };
                    Frame::new()
                        .inner_margin(Margin::symmetric(0, (theme::SP4 - theme::SP6) as i8))
                        .show(ui, |ui| {
                            widgets::empty_state(ui, Some(icon), heading, text, None);
                        });
                }
                Listen::Ready { .. } => {
                    if let Some(audio) = audio {
                        acts = Some(player.card(ui, t, audio));
                    } else {
                        let p = palette(ui.ctx());
                        ui.label(widgets::small(t.get("call-play-needs-packets")).color(p.text_3));
                    }
                }
            });
        },
    );
    acts
}

/// The flow (`.ladder`): a row per SIP message as an arrow between the lanes, the voice streams
/// as a band; a click on a row opens its packet
fn flow(ui: &mut Ui, t: Lang, flow: &Flow) -> Option<Page> {
    let p = palette(ui.ctx());
    let ppp = ui.ctx().pixels_per_point();
    let mut go = None;
    let lanes_n = flow.lanes.len().max(2) as f32;
    ui.add_space(theme::SP2);
    let width = ui.available_width().max(TIME_W + LANES_MIN + PACKET_W);
    let left = ui.cursor().left();
    let lanes_w = width - TIME_W - PACKET_W;
    let lane_x =
        |i: f32, lanes: Rect| lanes.left() + (2.0 * i + 1.0) / (2.0 * lanes_n) * lanes.width();
    let xs_font = || mono_font(theme::FS_XS);

    // the head: the names of the lanes
    {
        let top = ui.cursor().top();
        let mut bottom = top;
        let lanes = Rect::from_min_size(egui::pos2(left + TIME_W, top), egui::vec2(lanes_w, 0.0));
        let gap = theme::SP2;
        let col_w = (lanes_w - gap * (lanes_n - 1.0)) / lanes_n;
        for (i, (name, small)) in flow.lanes.iter().enumerate() {
            let x = lanes.left() + i as f32 * (col_w + gap);
            let name =
                ui.painter()
                    .layout(name.clone(), semibold_font(theme::FS_SM), p.text, col_w);
            let small = ui
                .painter()
                .layout(small.clone(), xs_font(), p.text_3, col_w);
            let h = name.size().y + small.size().y;
            let at = egui::pos2(x + (col_w - name.size().x) / 2.0, top).round_to_pixels(ppp);
            let at2 = egui::pos2(x + (col_w - small.size().x) / 2.0, top + name.size().y)
                .round_to_pixels(ppp);
            ui.painter().galley(at, name, p.text);
            ui.painter().galley(at2, small, p.text_3);
            bottom = bottom.max(top + h);
        }
        let label = |text: String| {
            ui.painter()
                .layout_no_wrap(text, egui::FontId::proportional(theme::FS_XS), p.text_3)
        };
        let time = label(t.get("call-flow-time"));
        let packet = label(t.get("call-flow-packet"));
        ui.painter().galley(
            egui::pos2(
                left + TIME_W - theme::SP3 - time.size().x,
                bottom - time.size().y,
            )
            .round_to_pixels(ppp),
            time,
            p.text_3,
        );
        ui.painter().galley(
            egui::pos2(
                left + TIME_W + lanes_w + theme::SP3,
                bottom - packet.size().y,
            )
            .round_to_pixels(ppp),
            packet,
            p.text_3,
        );
        let rect = Rect::from_min_max(
            egui::pos2(left, top),
            egui::pos2(left + width, bottom + theme::SP2),
        );
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, p.border),
        );
        ui.advance_cursor_after_rect(rect);
        ui.add_space(2.0);
    }

    let spacing = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    for (r, row) in flow.rows.iter().enumerate() {
        let band = matches!(row, FlowRow::Media { .. });
        let height = if band { 40.0 } else { 30.0 };
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
        let lanes = Rect::from_min_size(
            egui::pos2(rect.left() + TIME_W, rect.top()),
            egui::vec2(lanes_w, height),
        );
        if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, p.hover);
        }
        for i in 0..flow.lanes.len() {
            let x = lane_x(i as f32, lanes).round() + 0.5;
            ui.painter()
                .vline(x, lanes.y_range(), Stroke::new(1.0, p.border_strong));
        }
        let (at, number, tip) = match row {
            FlowRow::Message {
                number, at, clock, ..
            } => (at, *number, clock.clone()),
            FlowRow::Media { number, at, .. } => (at, *number, String::new()),
        };
        let time = ui.painter().layout_no_wrap(at.clone(), xs_font(), p.text_3);
        ui.painter().galley(
            egui::pos2(
                rect.left() + TIME_W - theme::SP3 - time.size().x,
                rect.center().y - time.size().y / 2.0,
            )
            .round_to_pixels(ppp),
            time,
            p.text_3,
        );
        match row {
            FlowRow::Message {
                label,
                right,
                bad,
                ok,
                ..
            } => {
                let (x0, x1) = (lane_x(0.0, lanes), lane_x(1.0, lanes));
                let color = if *bad { p.err } else { p.text_2 };
                let y = rect.top() + 20.0;
                ui.painter().hline(x0..=x1, y, Stroke::new(1.5, color));
                let (tip_x, dir) = if *right {
                    (x1 + 1.0, -1.0)
                } else {
                    (x0 - 1.0, 1.0)
                };
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(tip_x, y),
                        egui::pos2(tip_x + dir * 8.0, y - 4.5),
                        egui::pos2(tip_x + dir * 8.0, y + 4.5),
                    ],
                    color,
                    Stroke::NONE,
                ));
                let font = if *ok {
                    theme::mono_semibold_font(theme::FS_XS)
                } else {
                    xs_font()
                };
                let text_color = if *bad { p.err } else { p.text };
                let max = (x1 - x0 - 12.0).max(10.0);
                let mut job = LayoutJob::simple_singleline(label.clone(), font, text_color);
                job.wrap.max_width = max;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                let galley = ui.painter().layout_job(job);
                ui.painter().galley(
                    egui::pos2(
                        f32::midpoint(x0, x1) - galley.size().x / 2.0,
                        y - 2.0 - galley.size().y,
                    )
                    .round_to_pixels(ppp),
                    galley,
                    text_color,
                );
            }
            FlowRow::Media {
                lead, text, locked, ..
            } => {
                let last = (flow.lanes.len().max(2) - 1) as f32;
                let band = Rect::from_min_max(
                    egui::pos2(lane_x(0.0, lanes), rect.top() + 7.0),
                    egui::pos2(lane_x(last, lanes), rect.bottom() - 7.0),
                );
                ui.painter().rect(
                    band,
                    CornerRadius::same(theme::RADIUS_SM),
                    if *locked { p.surface_2 } else { p.rows.voip },
                    Stroke::new(1.0, p.border_strong),
                    egui::StrokeKind::Inside,
                );
                let mut job = LayoutJob::default();
                let plain = TextFormat {
                    font_id: egui::FontId::proportional(theme::FS_XS),
                    color: p.text,
                    ..TextFormat::default()
                };
                job.append(
                    "RTP",
                    0.0,
                    TextFormat {
                        font_id: semibold_font(theme::FS_XS),
                        ..plain.clone()
                    },
                );
                job.append(&format!(" {lead} · {text}"), 0.0, plain);
                job.wrap.max_width = band.width() - 2.0 * theme::SP2 - 20.0;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                let galley = ui.painter().layout_job(job);
                let icon_w = if *locked { widgets::ICON_SM + 6.0 } else { 0.0 };
                let x = band.center().x - f32::midpoint(icon_w, galley.size().x);
                if *locked {
                    widgets::paint_icon(
                        ui,
                        icons::LOCK,
                        Rect::from_min_size(
                            egui::pos2(x, band.center().y - widgets::ICON_SM / 2.0),
                            egui::vec2(widgets::ICON_SM, widgets::ICON_SM),
                        ),
                        p.text,
                    );
                }
                ui.painter().galley(
                    egui::pos2(x + icon_w, band.center().y - galley.size().y / 2.0)
                        .round_to_pixels(ppp),
                    galley,
                    p.text,
                );
            }
        }
        // the packet as a link; the band leads to the packets of its streams
        let link_text = match row {
            FlowRow::Media { .. } => format!("{number} …"),
            FlowRow::Message { .. } => number.to_string(),
        };
        let mut link = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    egui::pos2(rect.left() + TIME_W + lanes_w + theme::SP3, rect.top()),
                    egui::vec2(PACKET_W - theme::SP3, height),
                ))
                .layout(Layout::left_to_right(Align::Center))
                .id_salt(("flow-link", r)),
        );
        let clicked_link = {
            let response: Response = widgets::link_text(
                &mut link,
                RichText::new(link_text).font(xs_font()).color(p.accent),
            );
            let response = match row {
                FlowRow::Media { .. } => response.on_hover_text(t.get("call-band-packets")),
                FlowRow::Message { .. } => response,
            };
            response.clicked()
        };
        if clicked_link {
            go = Some(match row {
                FlowRow::Media { filter, .. } => packets_of(filter),
                FlowRow::Message { .. } => Page::Packet(number),
            });
        } else if response.clicked() {
            go = Some(Page::Packet(number));
        }
        if !tip.is_empty() {
            let time_rect = Rect::from_min_size(rect.min, egui::vec2(TIME_W, height));
            if ui.rect_contains_pointer(time_rect) {
                response
                    .clone()
                    .on_hover_text(t.get1("call-flow-clock", "time", tip));
            }
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    ui.spacing_mut().item_spacing.y = spacing;
    ui.add_space(theme::SP3);
    go
}

/// The voice streams (`#cstreams`)
fn streams(ui: &mut Ui, t: Lang, rows: &[StreamRow]) -> Option<Page> {
    let width = ui.ctx().content_rect().width();
    let md = width >= 1200.0;
    let sm = width >= 900.0;
    let all = [
        (
            Col::new(t.get("call-col-direction"), Width::Px(130.0)),
            true,
        ),
        (Col::new(t.get("call-col-path"), Width::Rest(200.0)), true),
        (Col::new(t.get("call-col-ssrc"), Width::Px(104.0)), md),
        (Col::num(t.get("call-col-packets"), Width::Px(88.0)), true),
        (Col::num(t.get("call-col-lost"), Width::Px(86.0)), true),
        (Col::num(t.get("call-col-jitter"), Width::Px(134.0)), true),
        (Col::num(t.get("call-col-gap"), Width::Px(138.0)), sm),
        (Col::num(t.get("call-col-late"), Width::Px(104.0)), md),
        (Col::new(t.get("call-col-rate"), Width::Px(104.0)), true),
    ];
    let shown: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, (_, on))| *on)
        .map(|(i, _)| i)
        .collect();
    let cols = all
        .into_iter()
        .filter(|(_, on)| *on)
        .map(|(c, _)| c)
        .collect();
    let spec = TableSpec::new("call-streams", cols, theme::ROW_H, rows.len());
    let mut go = None;
    widgets::table(ui, &spec, |ui, r, c| {
        let row = &rows[r];
        let p = palette(ui.ctx());
        match shown[c] {
            0 => direction(ui, row.sent, &row.direction),
            1 => {
                ui.add(egui::Label::new(widgets::mono(&row.path)).truncate());
            }
            2 => {
                if widgets::link_text(ui, widgets::mono(&row.ssrc).color(p.accent)).clicked() {
                    go = Some(packets_of(&row.filter));
                }
            }
            3 => {
                ui.label(widgets::mono(&row.packets));
            }
            4 => match &row.lost {
                // `.count.warn` and the share in brackets, both in the font of numbers
                Some((count, share)) => {
                    ui.label(widgets::mono(share).color(p.text_3));
                    ui.label(
                        RichText::new(count)
                            .font(theme::mono_medium_font(theme::FS_SM))
                            .color(p.warn),
                    );
                }
                None => {
                    ui.label(widgets::mono("0"));
                }
            },
            5 => match &row.jitter {
                Some(jitter) => {
                    ui.label(widgets::mono(jitter));
                }
                None => {
                    ui.label(RichText::new(t.get("call-jitter-unknown")).color(p.text_3))
                        .on_hover_text(t.get("call-jitter-unknown-title"));
                }
            },
            6 => {
                ui.label(widgets::mono(&row.gap));
            }
            7 => {
                ui.label(widgets::mono(&row.late));
            }
            _ => rate(ui, t, row.rate, &row.rate_title),
        }
    });
    go
}
