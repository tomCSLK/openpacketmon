//! The capture page (`pageCapture` of the draft): choose what to analyze and start; while a
//! capture runs, its card with pausing, stopping and the way to the overview.

use std::time::Duration;

use eframe::egui::emath::GuiRounding as _;
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Align, Frame, Id, Layout, Margin, Rect, RichText, Sense, Stroke, TextFormat, Ui, pos2,
    vec2,
};

use crate::engine::Engine;
use crate::gui::load::{Loader, Task};
use crate::gui::pages::head;
use crate::gui::theme::{self, medium_font, mono_font, mono_semibold_font, palette};
use crate::gui::widgets::{self, Kind, Tone, end_field, field_help, field_label, field_row};
use crate::gui::{App, Page, icons};
use crate::networking::types::capture_context::CaptureSourcePicklist;
use crate::view::capture::{
    CaptureSetup, PacketChoice, RunState, RunningCapture, StartRequest, apply_start, capture_setup,
    start_error,
};
use crate::view::i18n::Lang;

/// The column of the radio buttons (`.adapters td:first-child`)
const RADIO_W: f32 = 40.0;
/// Shares of the columns "Adapter" and "Adressen", as the table of the draft lays them out
const ADAPTER_SHARE: f32 = 0.305;
const ADDRESS_SHARE: f32 = 0.289;
/// The curve of an adapter (`.adapters .spark`)
const SPARK: egui::Vec2 = vec2(140.0, 26.0);
/// Points of a curve: one a second for 30 seconds
const SPARK_POINTS: usize = 30;
/// Line heights in the table: the name (`--lh`), the description (`.sub`), an address
const LINE_NAME: f32 = theme::FS_BASE * 1.45;
const LINE_SUB: f32 = theme::FS_XS * 1.3;
const LINE_ADDR: f32 = theme::FS_XS * 1.35;

/// What the page reads
struct Snapshot {
    setup: CaptureSetup,
    /// Packets per second of each adapter in the last 30 seconds
    previews: Vec<(String, Vec<u128>)>,
}

/// The choices on the page, kept while it is open
struct Form {
    source: CaptureSourcePicklist,
    device: String,
    file: String,
    ipfix_addr: String,
    ipfix_port: String,
    bpf: String,
    promiscuous: bool,
    export: bool,
    export_dir: String,
    export_name: String,
    packets: PacketChoice,
}

impl Form {
    fn new(setup: &CaptureSetup) -> Self {
        Self {
            source: setup.source,
            device: setup
                .adapters
                .iter()
                .find(|a| a.selected && !a.no_ip)
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            file: setup.file.clone(),
            ipfix_addr: setup.ipfix_addr.clone(),
            ipfix_port: setup.ipfix_port.clone(),
            bpf: setup.bpf.clone(),
            promiscuous: setup.promiscuous,
            export: setup.export_enabled,
            export_dir: setup.export_dir.clone(),
            export_name: setup.export_name.clone(),
            packets: setup.packets,
        }
    }

    fn request(&self) -> StartRequest {
        StartRequest {
            source: self.source,
            device: self.device.clone(),
            file: self.file.clone(),
            ipfix_addr: self.ipfix_addr.clone(),
            ipfix_port: self.ipfix_port.clone(),
            bpf: self.bpf.clone(),
            promiscuous: self.promiscuous,
            export: self.export,
            export_dir: self.export_dir.clone(),
            export_name: self.export_name.clone(),
            packets: Some(self.packets),
        }
    }
}

#[derive(Default)]
pub struct State {
    snapshot: Loader<usize, Snapshot>,
    form: Option<Form>,
    start: Task<Result<(), String>>,
    file_dialog: Task<Option<String>>,
    error: Option<String>,
}

fn read(t: Lang, engine: &Engine) -> Snapshot {
    let state = engine.lock();
    Snapshot {
        setup: capture_setup(t, &state),
        previews: state
            .previews
            .iter()
            .map(|p| {
                (
                    p.device.get_name().clone(),
                    p.packets.iter().copied().collect(),
                )
            })
            .collect(),
    }
}

fn start(app: &mut App, ctx: &egui::Context, request: StartRequest) {
    let engine = app.engine.clone();
    let t = app.t;
    app.pages.capture.error = None;
    app.pages.capture.start.start(ctx, move || {
        let problem = {
            let mut state = engine.lock();
            if state.is_running() {
                return Ok(());
            }
            apply_start(&mut state, &request)
        };
        if problem.is_none() && engine.start() {
            engine.store_conf();
            return Ok(());
        }
        Err(start_error(t, &engine.lock(), problem))
    });
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let state = &mut app.pages.capture;
    state.snapshot.poll();
    if let Some(result) = state.start.take() {
        match result {
            Ok(()) => {
                state.form = None;
                app.go(Page::Overview);
                return;
            }
            Err(e) => state.error = Some(e),
        }
    }
    if let Some(Some(path)) = state.file_dialog.take()
        && let Some(form) = &mut state.form
    {
        form.file = path;
    }
    let engine = app.engine.clone();
    state.snapshot.load(
        &ctx,
        &app.live.capture_id,
        Some(Duration::from_secs(1)),
        move || read(t, &engine),
    );

    let running = app
        .pages
        .capture
        .snapshot
        .value_of(&app.live.capture_id)
        .and_then(|s| s.setup.running.as_ref())
        .map(|r| r.subtitle.clone());
    let subtitle = running.clone().unwrap_or_else(|| t.get("capture-subtitle"));
    head(ui, &t.get("capture-title"), Some(&subtitle), |_| {});
    if running.is_some() {
        running_card(app, ui);
    } else {
        setup(app, ui);
    }
}

/// What the buttons of the running capture do
enum Action {
    Overview,
    Pause,
    Stop,
}

/// The card of the running capture: its state and duration, the facts, the buttons
fn running_card(app: &mut App, ui: &mut Ui) {
    let t = app.t;
    let Some(run) = app
        .pages
        .capture
        .snapshot
        .value()
        .and_then(|s| s.setup.running.as_ref())
    else {
        return;
    };
    let action = widgets::custom_card(
        ui,
        |ui| {
            let (tone, icon, id) = match run.state {
                RunState::Running => (Tone::Ok, icons::CHECK_CIRCLE, "capture-running"),
                RunState::Paused => (Tone::Warn, icons::PAUSE, "capture-state-paused"),
                RunState::Finished => (Tone::Ok, icons::CHECK_CIRCLE, "capture-state-finished"),
                RunState::Failed => (Tone::Err, icons::X_CIRCLE, "capture-state-error"),
            };
            widgets::state(ui, tone, Some(icon), &t.get(id));
        },
        |ui| widgets::card_note(ui, &run.duration),
        |ui| {
            Frame::new()
                .inner_margin(Margin::same(theme::SP4 as i8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    facts(ui, t, run);
                });
            let (left, overview) = foot(
                ui,
                |ui| {
                    ui.spacing_mut().item_spacing.x = theme::SP2;
                    let mut action = None;
                    if matches!(run.state, RunState::Running | RunState::Paused) {
                        let (icon, id) = if run.state == RunState::Paused {
                            (icons::PLAY, "capture-resume")
                        } else {
                            (icons::PAUSE, "capture-pause")
                        };
                        let text = t.get(id);
                        if widgets::icon_text_button(ui, Some(icon), &text, Kind::Secondary, false)
                            .clicked()
                        {
                            action = Some(Action::Pause);
                        }
                    }
                    let text = t.get("capture-stop");
                    if widgets::icon_text_button(ui, Some(icons::STOP), &text, Kind::Danger, false)
                        .clicked()
                    {
                        action = Some(Action::Stop);
                    }
                    let p = palette(ui.ctx());
                    let hint = widgets::small(t.get("capture-stop-hint")).color(p.text_3);
                    ui.add(egui::Label::new(hint).truncate());
                    action
                },
                |ui| {
                    let text = t.get("capture-to-overview");
                    widgets::text_icon_button(ui, &text, icons::ARROW_RIGHT, Kind::Primary)
                        .clicked()
                },
            );
            if overview {
                Some(Action::Overview)
            } else {
                left
            }
        },
    );
    match action {
        Some(Action::Overview) => app.go(Page::Overview),
        Some(Action::Pause) => pause(app),
        Some(Action::Stop) => stop(app),
        None => {}
    }
}

/// A row of the facts of the running capture
#[derive(Clone, Copy)]
enum Fact {
    Source,
    Link,
    Listening,
    Promiscuous,
    Filter,
    Store,
    Export,
    Dropped,
}

/// The facts of the running capture (`dl.facts`)
fn facts(ui: &mut Ui, t: Lang, run: &RunningCapture) {
    let p = palette(ui.ctx());
    let mut shown = vec![(Fact::Source, t.get("capture-source"))];
    if run.link_type.is_some() {
        shown.push((Fact::Link, t.get("capture-link-type")));
    }
    if !run.listening.is_empty() {
        shown.push((Fact::Listening, t.get("capture-listening")));
    }
    if run.promiscuous.is_some() {
        shown.push((Fact::Promiscuous, t.get("capture-promisc")));
    }
    if run.filter.is_some() {
        shown.push((Fact::Filter, t.get("capture-filter")));
    }
    if run.packet_store.is_some() {
        shown.push((Fact::Store, t.get("settings-packet-store")));
    }
    if run.export.is_some() {
        shown.push((Fact::Export, t.get("capture-export")));
    }
    if run.dropped.is_some() {
        shown.push((Fact::Dropped, t.get("capture-dropped")));
    }
    let labels: Vec<String> = shown.iter().map(|(_, label)| label.clone()).collect();
    let muted = |ui: &mut Ui, text: String| {
        ui.label(RichText::new(text).color(p.text_3));
    };
    widgets::facts(ui, &labels, |ui, i| match shown[i].0 {
        Fact::Source => {
            ui.label(
                RichText::new(&run.source)
                    .font(mono_semibold_font(theme::FS_SM))
                    .color(p.text),
            );
            if !run.source_desc.is_empty() {
                muted(ui, run.source_desc.clone());
            }
        }
        Fact::Link => {
            ui.label(run.link_type.clone().unwrap_or_default());
        }
        Fact::Listening => {
            widgets::code(ui, &run.listening.join(", "));
        }
        Fact::Promiscuous => {
            ui.label(run.promiscuous.clone().unwrap_or_default());
        }
        Fact::Filter => match run.filter.as_deref() {
            Some(filter) if !filter.is_empty() => {
                widgets::code(ui, filter);
            }
            _ => muted(ui, t.get("capture-no-filter")),
        },
        Fact::Store => {
            ui.label(run.packet_store.clone().unwrap_or_default());
        }
        Fact::Export => match run.export.as_deref() {
            Some(path) if !path.is_empty() => {
                widgets::code(ui, path);
            }
            _ => muted(ui, t.get("capture-no-export")),
        },
        Fact::Dropped => {
            if let Some((adapter, app)) = &run.dropped {
                dropped(ui, t, adapter, app);
            }
        }
    });
}

/// "0 vom Adapter, 0 von openpacketmon (Puffer voll)" with the numbers as code
fn dropped(ui: &mut Ui, t: Lang, adapter: &str, by_app: &str) {
    let p = palette(ui.ctx());
    let text = t.get2("capture-dropped-text", "adapter", "\u{1}", "app", "\u{2}");
    let plain = TextFormat {
        font_id: egui::FontId::proportional(theme::FS_BASE),
        color: p.text,
        ..Default::default()
    };
    let code = TextFormat {
        font_id: mono_font(theme::FS_SM),
        color: p.text,
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    let mut rest = text.as_str();
    while let Some(at) = rest.find(['\u{1}', '\u{2}']) {
        job.append(&rest[..at], 0.0, plain.clone());
        let number = if rest[at..].starts_with('\u{1}') {
            adapter
        } else {
            by_app
        };
        job.append(number, 0.0, code.clone());
        rest = &rest[at + 1..];
    }
    job.append(rest, 0.0, plain);
    ui.label(job);
}

/// The foot of a card (`.cfoot`): a line, then 12 px above and below a row of buttons of 32 px;
/// `left` from the left, `right` at the right end
fn foot<L, R>(
    ui: &mut Ui,
    left: impl FnOnce(&mut Ui) -> L,
    right: impl FnOnce(&mut Ui) -> R,
) -> (L, R) {
    widgets::rule(ui);
    Frame::new()
        .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let row = vec2(ui.available_width(), widgets::CONTROL_H);
            ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                let l = left(ui);
                let r = ui
                    .with_layout(Layout::right_to_left(Align::Center), right)
                    .inner;
                (l, r)
            })
            .inner
        })
        .inner
}

/// Pauses or resumes the capture
pub fn pause(app: &App) {
    app.engine.lock().toggle_freeze();
}

/// Ends the capture; its packets are gone then
pub fn stop(app: &mut App) {
    app.engine.stop();
    app.pages.capture.form = None;
    app.go(Page::Capture);
}

/// The choices before a capture: the source, the filter, the packet store, the PCAP file
fn setup(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let capture_id = app.live.capture_id;
    let state = &mut app.pages.capture;
    if state.form.is_none()
        && let Some(snapshot) = state.snapshot.value_of(&capture_id)
    {
        state.form = Some(Form::new(&snapshot.setup));
    }
    if let Some(error) = &state.error {
        widgets::notice_icon(ui, Tone::Err, Some(icons::X_CIRCLE), |ui| {
            ui.label(error.as_str());
        });
    }
    let Some(snapshot) = state.snapshot.value() else {
        return;
    };
    let Some(form) = state.form.as_mut() else {
        return;
    };
    let mut pick_file = false;

    let sources = [
        CaptureSourcePicklist::Device,
        CaptureSourcePicklist::File,
        CaptureSourcePicklist::Ipfix,
    ];
    let chosen = sources.iter().position(|s| *s == form.source).unwrap_or(0);
    let mut next_source = None;
    widgets::tall_card(
        ui,
        &t.get("capture-source"),
        widgets::CONTROL_H,
        |ui| {
            let labels = [
                t.get("source-device"),
                t.get("source-file"),
                t.get("source-ipfix"),
            ];
            let icons = [icons::ADAPTER, icons::FILE, icons::RADIO];
            next_source = widgets::segmented_icons(ui, &labels, &icons, chosen);
        },
        |ui| match form.source {
            CaptureSourcePicklist::Device => {
                adapters(ui, t, form, snapshot);
                promiscuous(ui, t, form);
            }
            CaptureSourcePicklist::File => body(ui, |ui| {
                field_label(ui, &t.get("capture-file-path"));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::SP2;
                    let browse = t.get("capture-browse");
                    let button = widgets::button_width(ui, &browse) + widgets::ICON_SM + 6.0;
                    let width = (ui.available_width() - button - theme::SP2).max(120.0);
                    widgets::input(
                        ui,
                        &mut form.file,
                        Id::NULL.with("capture-file"),
                        vec2(width, widgets::CONTROL_H),
                        false,
                        &t.get("capture-file-example"),
                    );
                    pick_file = widgets::icon_text_button(
                        ui,
                        Some(icons::FOLDER),
                        &browse,
                        Kind::Secondary,
                        false,
                    )
                    .clicked();
                });
                field_help(ui, &t.get("capture-file-hint"));
                end_field(ui);
            }),
            CaptureSourcePicklist::Ipfix => body(ui, |ui| {
                field_row(
                    ui,
                    |ui, width| {
                        field_label(ui, &t.get("capture-ipfix-addr"));
                        widgets::input(
                            ui,
                            &mut form.ipfix_addr,
                            Id::NULL.with("capture-ipfix-addr"),
                            vec2(width, widgets::CONTROL_H),
                            false,
                            &t.get("capture-ipfix-all"),
                        );
                        end_field(ui);
                    },
                    |ui, width| {
                        field_label(ui, &t.get("capture-ipfix-port"));
                        widgets::input(
                            ui,
                            &mut form.ipfix_port,
                            Id::NULL.with("capture-ipfix-port"),
                            vec2(width, widgets::CONTROL_H),
                            false,
                            "4739",
                        );
                        end_field(ui);
                    },
                );
                widgets::hint(ui, &t.get("capture-ipfix-hint"));
                ui.add_space(theme::SP2);
            }),
        },
    );

    if let Some(i) = next_source {
        form.source = sources[i];
    }

    // filters and the packet store only for sources with packets
    if form.source != CaptureSourcePicklist::Ipfix {
        widgets::grid2(
            ui,
            "capture",
            |ui, fill| {
                widgets::flush_card(
                    ui,
                    &t.get("capture-filter"),
                    |_| {},
                    |ui| {
                        body(ui, |ui| {
                            field_label(ui, &t.get("capture-bpf"));
                            let width = ui.available_width();
                            widgets::input(
                                ui,
                                &mut form.bpf,
                                Id::NULL.with("capture-bpf"),
                                vec2(width, widgets::CONTROL_H),
                                // the draft writes it in the font of the page
                                false,
                                &t.get("capture-bpf-example"),
                            );
                            field_help(ui, &t.get("capture-bpf-hint"));
                        });
                        fill.apply(ui);
                    },
                );
            },
            |ui, fill| {
                let choices = [PacketChoice::Full, PacketChoice::Headers, PacketChoice::Off];
                let chosen = choices.iter().position(|c| *c == form.packets).unwrap_or(0);
                let mut next = None;
                widgets::tall_card(
                    ui,
                    &t.get("settings-packet-store"),
                    widgets::CONTROL_SM_H,
                    |ui| {
                        let labels = [
                            t.get("capture-packets-full"),
                            t.get("capture-packets-headers"),
                            t.get("capture-packets-off"),
                        ];
                        next = widgets::segmented_sized(ui, &labels, chosen, true);
                    },
                    |ui| {
                        body(ui, |ui| {
                            let p = palette(ui.ctx());
                            let line = &snapshot.setup.packets_lines[chosen];
                            ui.add(
                                egui::Label::new(
                                    RichText::new(line)
                                        .font(medium_font(theme::FS_SM))
                                        .color(p.text)
                                        .line_height(Some(theme::FS_SM * 1.45)),
                                )
                                .wrap(),
                            );
                            ui.add_space(theme::SP2);
                            widgets::hint(ui, &t.get("capture-packets-hint"));
                        });
                        fill.apply(ui);
                    },
                );
                if let Some(i) = next {
                    form.packets = choices[i];
                }
            },
        );
    }

    // only adapters write PCAP files
    if form.source == CaptureSourcePicklist::Device {
        widgets::flush_card(
            ui,
            &t.get("capture-export"),
            |_| {},
            |ui| {
                body(ui, |ui| {
                    widgets::check(ui, &mut form.export, &t.get("capture-export-enable"));
                    ui.add_space(theme::SP2);
                    let on = form.export;
                    field_row(
                        ui,
                        |ui, width| {
                            field_label(ui, &t.get("capture-export-dir"));
                            widgets::input_on(
                                ui,
                                &mut form.export_dir,
                                Id::NULL.with("capture-export-dir"),
                                vec2(width, widgets::CONTROL_H),
                                false,
                                "",
                                on,
                            );
                            end_field(ui);
                        },
                        |ui, width| {
                            field_label(ui, &t.get("capture-export-name"));
                            widgets::input_on(
                                ui,
                                &mut form.export_name,
                                Id::NULL.with("capture-export-name"),
                                vec2(width, widgets::CONTROL_H),
                                false,
                                "openpacketmon.pcap",
                                on,
                            );
                            end_field(ui);
                        },
                    );
                });
            },
        );
    }

    let request = form.request();
    let busy = state.start.busy();
    let mut clicked = false;
    let row = vec2(ui.available_width(), widgets::CONTROL_H);
    ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
        ui.add_enabled_ui(!busy, |ui| {
            clicked = widgets::icon_text_button(
                ui,
                Some(icons::PLAY),
                &t.get("capture-start"),
                Kind::Primary,
                false,
            )
            .clicked();
        });
    });
    if pick_file {
        state.file_dialog.start(&ctx, || {
            rfd::FileDialog::new()
                .add_filter("PCAP", &["pcap", "pcapng", "cap"])
                .pick_file()
                .map(|p| p.display().to_string())
        });
    }
    if clicked {
        start(app, &ctx, request);
    }
}

/// The body of a card with 16 px around it (`.cbody`)
fn body(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .inner_margin(Margin::same(theme::SP4 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// Below the adapters: promiscuous mode, with a line above it
fn promiscuous(ui: &mut Ui, t: Lang, form: &mut Form) {
    let p = palette(ui.ctx());
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .hline(line.x_range(), line.center().y, Stroke::new(1.0, p.border));
    body(ui, |ui| {
        widgets::check(ui, &mut form.promiscuous, &t.get("capture-promisc-enable"));
        field_help(ui, &t.get("capture-promisc-hint"));
    });
}

/// The adapters (`table.adapters`): the hint, then a row for each with a radio button, name and
/// description, addresses and the curve of its packets
fn adapters(ui: &mut Ui, t: Lang, form: &mut Form, snapshot: &Snapshot) {
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
            widgets::hint(ui, &t.get("capture-adapters-hint"));
        });
    let rows = &snapshot.setup.adapters;
    if rows.is_empty() {
        body(ui, |ui| {
            widgets::notice(ui, |ui| {
                ui.label(RichText::new(t.get("capture-no-adapters")).strong());
                ui.label(t.get("capture-no-adapters-hint"));
            });
        });
        return;
    }
    let total = ui.available_width();
    let left = ui.cursor().left();
    let w_adapter = (total * ADAPTER_SHARE).round();
    let w_address = (total * ADDRESS_SHARE).round();
    let cols = [
        left,
        left + RADIO_W,
        left + RADIO_W + w_adapter,
        left + RADIO_W + w_adapter + w_address,
    ];
    let pad = theme::SP3;
    let pixels = ui.ctx().pixels_per_point();

    // the head (`.tbl th`)
    let (head, _) = ui.allocate_exact_size(vec2(total, 32.0), Sense::hover());
    ui.painter().rect_filled(head, 0.0, p.surface_2);
    ui.painter().hline(
        head.x_range(),
        head.bottom() - 0.5,
        Stroke::new(1.0, p.border),
    );
    for (i, id) in [
        (1, "capture-adapter"),
        (2, "capture-addresses"),
        (3, "capture-activity"),
    ] {
        let galley = ui
            .painter()
            .layout_no_wrap(t.get(id), medium_font(theme::FS_SM), p.text_3);
        let at = pos2(cols[i] + pad, head.center().y - 0.5 - galley.size().y / 2.0)
            .round_to_pixels(pixels);
        ui.painter().galley(at, galley, p.text_3);
    }

    // the highest number of packets in a second scales all curves
    let max = snapshot
        .previews
        .iter()
        .flat_map(|(_, packets)| packets.iter().copied())
        .max()
        .unwrap_or(0);
    let id = Id::NULL.with("capture-adapters");
    let last = rows.len() - 1;
    for (r, row) in rows.iter().enumerate() {
        let name_block = LINE_NAME + if row.desc.is_empty() { 0.0 } else { LINE_SUB };
        let addresses = row.addresses.len().max(1) as f32 * LINE_ADDR;
        let content = name_block.max(addresses).max(SPARK.y);
        // 4 px above and below, at least `--row-h`, and the line below
        let height = (content + 8.0).max(34.0).round() + if r < last { 1.0 } else { 0.0 };
        let (rect, _) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
        let sense = if row.no_ip {
            Sense::hover()
        } else {
            Sense::click()
        };
        let response = ui.interact(rect, id.with(r), sense);
        let chosen = !row.no_ip && form.device == row.name;
        let fill = if chosen {
            p.selected
        } else if response.hovered() {
            p.hover
        } else {
            egui::Color32::TRANSPARENT
        };
        let cell = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + height - 1.0));
        ui.painter().rect_filled(cell, 0.0, fill);
        if r < last {
            ui.painter().hline(
                rect.x_range(),
                rect.bottom() - 0.5,
                Stroke::new(1.0, p.border),
            );
        }
        let middle = rect.top() + (height - if r < last { 1.0 } else { 0.0 }) / 2.0;
        let (text, sub, addr) = if row.no_ip {
            (p.text_3, p.text_3, p.text_3)
        } else {
            (p.text, p.text_3, p.text_2)
        };
        widgets::radio_mark(
            ui,
            pos2(cols[0] + pad + 8.0, middle),
            chosen,
            !row.no_ip,
            response.hovered(),
        );
        // name and description
        let clip = Rect::from_x_y_ranges(cols[1]..=cols[2] - pad, rect.y_range());
        let painter = ui.painter().with_clip_rect(clip.intersect(ui.clip_rect()));
        let mut y = middle - name_block / 2.0;
        let x = cols[1] + pad;
        let name = painter.layout_no_wrap(
            row.label.clone(),
            mono_semibold_font(theme::FS_BASE * 0.92),
            text,
        );
        painter.galley(
            pos2(x, y + (LINE_NAME - name.size().y) / 2.0).round_to_pixels(pixels),
            name,
            text,
        );
        y += LINE_NAME;
        if !row.desc.is_empty() {
            let width = (cols[2] - pad - x).min(360.0);
            let desc = truncated(
                ui,
                &row.desc,
                egui::FontId::proportional(theme::FS_XS),
                sub,
                width,
            );
            painter.galley(
                pos2(x, y + (LINE_SUB - desc.size().y) / 2.0).round_to_pixels(pixels),
                desc,
                sub,
            );
        }
        // the addresses, one per line
        let clip = Rect::from_x_y_ranges(cols[2]..=cols[3] - pad, rect.y_range());
        let painter = ui.painter().with_clip_rect(clip.intersect(ui.clip_rect()));
        let x = cols[2] + pad;
        let mut y = middle - addresses / 2.0;
        let font = mono_font(theme::FS_XS);
        if row.addresses.is_empty() {
            let dash = painter.layout_no_wrap("–".to_owned(), font.clone(), p.text_3);
            painter.galley(
                pos2(x, y + (LINE_ADDR - dash.size().y) / 2.0).round_to_pixels(pixels),
                dash,
                p.text_3,
            );
        }
        for address in &row.addresses {
            let galley = painter.layout_no_wrap(address.clone(), font.clone(), addr);
            painter.galley(
                pos2(x, y + (LINE_ADDR - galley.size().y) / 2.0).round_to_pixels(pixels),
                galley,
                addr,
            );
            y += LINE_ADDR;
        }
        // the curve, or why there is none
        let x = cols[3] + pad;
        if row.no_ip {
            let note = ui.painter().layout_no_wrap(
                t.get("capture-no-ip"),
                egui::FontId::proportional(theme::FS_SM),
                p.text_3,
            );
            ui.painter().galley(
                pos2(x, middle - note.size().y / 2.0).round_to_pixels(pixels),
                note,
                p.text_3,
            );
        } else {
            let packets = snapshot
                .previews
                .iter()
                .find(|(name, _)| *name == row.name)
                .map_or(&[][..], |(_, packets)| packets.as_slice());
            let at = Rect::from_min_size(pos2(x, middle - SPARK.y / 2.0), SPARK);
            spark(ui, at, packets, max);
        }
        if response.clicked() && !row.no_ip {
            form.device.clone_from(&row.name);
        }
        if !row.no_ip {
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        // the preview only learns after opening an adapter that it has no IP traffic
        if row.no_ip && form.device == row.name {
            form.device.clear();
        }
    }
}

/// A text in one line, cut with "…" to `width`
fn truncated(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    ui.painter().layout_job(job)
}

/// The packets of an adapter in the last 30 seconds (`.spark`): a line from the bottom, 1.5 px in
/// the first color of the charts; a flat grey line without traffic
fn spark(ui: &Ui, rect: Rect, packets: &[u128], max: u128) {
    let p = palette(ui.ctx());
    // the newest second at the right end; seconds not seen yet count as none
    let skip = packets.len().saturating_sub(SPARK_POINTS);
    let seen = &packets[skip..];
    let mut values = vec![0_u128; SPARK_POINTS - seen.len()];
    values.extend_from_slice(seen);
    let flat = values.iter().all(|v| *v == 0);
    #[allow(clippy::cast_precision_loss)] // a share of the height
    let points: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let share = if flat {
                0.0
            } else {
                *v as f32 / max.max(1) as f32
            };
            pos2(
                rect.left() + k as f32 * rect.width() / (SPARK_POINTS - 1) as f32,
                rect.top() + 24.0 - share * 22.0,
            )
        })
        .collect();
    let color = if flat { p.border_strong } else { p.s1 };
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.5, color)));
}
