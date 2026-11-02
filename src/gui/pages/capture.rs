//! The capture page: choose what to analyze, start, pause and stop.

use std::time::Duration;

use eframe::egui::{self, ComboBox, Grid, RichText, ScrollArea, TextEdit, Ui};

use crate::engine::Engine;
use crate::gui::load::{Loader, Task};
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, Page};
use crate::networking::types::capture_context::CaptureSourcePicklist;
use crate::view::capture::{
    AdapterRow, CaptureSetup, PacketChoice, StartRequest, apply_start, capture_setup, start_error,
};
use crate::view::i18n::Lang;

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
    export: bool,
    export_dir: String,
    export_name: String,
    packets: PacketChoice,
    packets_max: usize,
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
            export: setup.export_enabled,
            export_dir: setup.export_dir.clone(),
            export_name: setup.export_name.clone(),
            packets: setup.packets,
            packets_max: setup
                .packets_max
                .iter()
                .find(|(_, _, selected)| *selected)
                .map_or(0, |(_, n, _)| *n),
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
            export: self.export,
            export_dir: self.export_dir.clone(),
            export_name: self.export_name.clone(),
            packets: Some(self.packets),
            packets_max: self.packets_max,
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

    ui.horizontal(|ui| {
        app.back_arrow(ui);
        ui.vertical(|ui| {
            widgets::page_head(
                ui,
                &t.get("capture-title"),
                Some(&t.get("capture-subtitle")),
                |_| {},
            );
        });
    });
    if let Some(error) = app.pages.capture.error.clone() {
        widgets::error(ui, &error);
        ui.add_space(4.0);
    }
    let running = app
        .pages
        .capture
        .snapshot
        .value_of(&app.live.capture_id)
        .is_some_and(|s| s.setup.running.is_some());
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        if running {
            running_card(app, ui);
        } else {
            setup_cards(app, ui);
        }
    });
}

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
    let p = palette(ui.ctx());
    let (fill, fg) = if run.frozen {
        (p.warn_soft, p.warn)
    } else {
        (p.ok_soft, p.ok)
    };
    let state = run.state.clone();
    let frozen = run.frozen;
    let mut lines: Vec<(String, String, bool)> = vec![(run.kind.clone(), run.name.clone(), true)];
    if !run.filter.is_empty() {
        lines.push((t.get("capture-filter"), run.filter.clone(), true));
    }
    if let Some(path) = &run.export {
        lines.push((t.get("capture-export"), path.clone(), true));
    }
    if let Some(link) = &run.link_type {
        lines.push((t.get("capture-link-type"), link.clone(), false));
    }
    if !run.listening.is_empty() {
        lines.push((t.get("capture-listening"), run.listening.join(", "), true));
    }
    if let Some(store) = &run.packet_store {
        lines.push((t.get("settings-packet-store"), store.clone(), false));
    }
    if let Some((adapter, by_app)) = &run.dropped {
        lines.push((
            t.get("capture-dropped"),
            t.get2(
                "capture-dropped-text",
                "adapter",
                adapter.as_str(),
                "app",
                by_app.as_str(),
            ),
            false,
        ));
    }
    let mut action = None;
    widgets::card(
        ui,
        &t.get("capture-running"),
        |ui| widgets::tag(ui, &state, fill, fg),
        |ui| {
            widgets::facts(ui, "capture-facts", |ui| {
                for (label, value, code) in &lines {
                    widgets::fact_label(ui, label);
                    if *code {
                        widgets::code(ui, value);
                    } else {
                        ui.label(value);
                    }
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if widgets::button(ui, t.get("capture-to-overview"), Kind::Secondary).clicked() {
                    action = Some(Action::Overview);
                }
                let pause = if frozen {
                    "capture-resume"
                } else {
                    "capture-pause"
                };
                if widgets::button(ui, t.get(pause), Kind::Secondary).clicked() {
                    action = Some(Action::Pause);
                }
                if widgets::button(ui, t.get("capture-stop"), Kind::Danger).clicked() {
                    action = Some(Action::Stop);
                }
            });
            widgets::hint(ui, &t.get("capture-stop-hint"));
        },
    );
    match action {
        Some(Action::Overview) => app.go(Page::Overview),
        Some(Action::Pause) => pause(app),
        Some(Action::Stop) => stop(app),
        None => {}
    }
}

enum Action {
    Overview,
    Pause,
    Stop,
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

fn setup_cards(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let capture_id = app.live.capture_id;
    let state = &mut app.pages.capture;
    if state.form.is_none()
        && let Some(snapshot) = state.snapshot.value_of(&capture_id)
    {
        state.form = Some(Form::new(&snapshot.setup));
    }
    let Some(snapshot) = state.snapshot.value() else {
        return;
    };
    let Some(form) = state.form.as_mut() else {
        return;
    };
    let mut pick_file = false;

    widgets::card(
        ui,
        &t.get("capture-source"),
        |_| {},
        |ui| {
            let sources = [
                CaptureSourcePicklist::Device,
                CaptureSourcePicklist::File,
                CaptureSourcePicklist::Ipfix,
            ];
            let labels = [
                t.get("source-device"),
                t.get("source-file"),
                t.get("source-ipfix"),
            ];
            let chosen = sources.iter().position(|s| *s == form.source).unwrap_or(0);
            if let Some(i) = widgets::segmented(ui, &labels, chosen) {
                form.source = sources[i];
            }
            ui.add_space(6.0);
            match form.source {
                CaptureSourcePicklist::Device => device_fields(ui, t, form, snapshot),
                CaptureSourcePicklist::File => {
                    ui.label(t.get("capture-file-path"));
                    ui.horizontal(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut form.file)
                                .hint_text("/home/…/capture.pcapng")
                                .desired_width((ui.available_width() - 110.0).max(120.0)),
                        );
                        pick_file = widgets::button(ui, t.get("gui-choose-file"), Kind::Secondary)
                            .clicked();
                    });
                    widgets::hint(ui, &t.get("capture-file-hint"));
                }
                CaptureSourcePicklist::Ipfix => {
                    Grid::new("ipfix").num_columns(2).show(ui, |ui| {
                        ui.label(t.get("capture-ipfix-addr"));
                        ui.add(TextEdit::singleline(&mut form.ipfix_addr).hint_text("0.0.0.0"));
                        ui.end_row();
                        ui.label(t.get("capture-ipfix-port"));
                        ui.add(TextEdit::singleline(&mut form.ipfix_port).hint_text("4739"));
                        ui.end_row();
                    });
                    widgets::hint(ui, &t.get("capture-ipfix-hint"));
                }
            }
        },
    );

    if form.source != CaptureSourcePicklist::Ipfix {
        ui.add_space(8.0);
        widgets::card(
            ui,
            &t.get("capture-filter"),
            |_| {},
            |ui| {
                ui.label(t.get("capture-bpf"));
                ui.add(
                    TextEdit::singleline(&mut form.bpf)
                        .hint_text("tcp port 443 or udp port 53")
                        .desired_width(f32::INFINITY),
                );
                widgets::hint(ui, &t.get("capture-bpf-hint"));
            },
        );
        ui.add_space(8.0);
        widgets::card(
            ui,
            &t.get("settings-packet-store"),
            |_| {},
            |ui| {
                let choices = [PacketChoice::Full, PacketChoice::Headers, PacketChoice::Off];
                let labels = [
                    t.get("capture-packets-full"),
                    t.get("capture-packets-headers"),
                    t.get("capture-packets-off"),
                ];
                let chosen = choices.iter().position(|c| *c == form.packets).unwrap_or(0);
                if let Some(i) = widgets::segmented(ui, &labels, chosen) {
                    form.packets = choices[i];
                }
                if form.packets != PacketChoice::Off {
                    let current = snapshot
                        .setup
                        .packets_max
                        .iter()
                        .find(|(_, n, _)| *n == form.packets_max)
                        .map(|(label, _, _)| label.clone())
                        .unwrap_or_default();
                    ComboBox::from_label(t.get("settings-packets-max"))
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (label, n, _) in &snapshot.setup.packets_max {
                                ui.selectable_value(&mut form.packets_max, *n, label);
                            }
                        });
                }
                widgets::hint(ui, &t.get("capture-packets-hint"));
            },
        );
    }

    ui.add_space(8.0);
    let request = form.request();
    let busy = state.start.busy();
    let mut clicked = false;
    ui.add_enabled_ui(!busy, |ui| {
        clicked = widgets::button(ui, t.get("capture-start"), Kind::Primary).clicked();
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

fn device_fields(ui: &mut Ui, t: Lang, form: &mut Form, snapshot: &Snapshot) {
    let p = palette(ui.ctx());
    ui.label(RichText::new(t.get("capture-adapters")).strong());
    widgets::hint(ui, &t.get("capture-adapters-hint"));
    if snapshot.setup.adapters.is_empty() {
        widgets::notice(ui, |ui| {
            ui.label(RichText::new(t.get("capture-no-adapters")).strong());
            ui.label(t.get("capture-no-adapters-hint"));
        });
    } else {
        Grid::new("adapters")
            .num_columns(3)
            .spacing([14.0, 6.0])
            .striped(false)
            .show(ui, |ui| {
                ui.scope(|ui| {
                    ui.set_min_width(280.0);
                    widgets::fact_label(ui, &t.get("capture-adapter"));
                });
                ui.scope(|ui| {
                    ui.set_min_width(300.0);
                    widgets::fact_label(ui, &t.get("capture-addresses"));
                });
                widgets::fact_label(ui, &t.get("capture-activity"));
                ui.end_row();
                for adapter in &snapshot.setup.adapters {
                    if adapter.no_ip {
                        // the preview only learns this after opening the adapter
                        if form.device == adapter.name {
                            form.device.clear();
                        }
                        no_ip_row(ui, t, adapter);
                        ui.end_row();
                        continue;
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let name = RichText::new(&adapter.label).strong();
                        ui.radio_value(&mut form.device, adapter.name.clone(), name);
                        if !adapter.desc.is_empty() {
                            ui.label(RichText::new(&adapter.desc).small().color(p.muted));
                        }
                    });
                    if adapter.addresses.is_empty() {
                        ui.label(RichText::new("–").color(p.faint));
                    } else {
                        ui.add(
                            egui::Label::new(RichText::new(&adapter.addresses).small())
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        )
                        .on_hover_text(&adapter.addresses);
                    }
                    ui.horizontal(|ui| {
                        let packets = snapshot
                            .previews
                            .iter()
                            .find(|(name, _)| *name == adapter.name)
                            .map_or(&[][..], |(_, packets)| packets.as_slice());
                        sparkline(ui, packets);
                        ui.label(&adapter.packets);
                    });
                    ui.end_row();
                }
            });
    }
    ui.add_space(4.0);
    ui.checkbox(&mut form.export, t.get("capture-export-enable"));
    if form.export {
        Grid::new("export").num_columns(2).show(ui, |ui| {
            ui.label(t.get("capture-export-dir"));
            ui.add(TextEdit::singleline(&mut form.export_dir).desired_width(320.0));
            ui.end_row();
            ui.label(t.get("capture-export-name"));
            ui.add(
                TextEdit::singleline(&mut form.export_name)
                    .hint_text("openpacketmon.pcap")
                    .desired_width(320.0),
            );
            ui.end_row();
        });
    }
}

/// An adapter without IP traffic (Bluetooth, NFLOG …): greyed out, cannot be chosen, a note
/// instead of the activity, since the preview does not watch it
fn no_ip_row(ui: &mut Ui, t: Lang, adapter: &AdapterRow) {
    let p = palette(ui.ctx());
    let note = t.get("capture-no-ip");
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let name = RichText::new(&adapter.label).strong();
        ui.add_enabled(false, egui::RadioButton::new(false, name))
            .on_disabled_hover_text(&note);
        if !adapter.desc.is_empty() {
            ui.label(RichText::new(&adapter.desc).small().color(p.faint));
        }
    });
    if adapter.addresses.is_empty() {
        ui.label(RichText::new("–").color(p.faint));
    } else {
        ui.add(
            egui::Label::new(RichText::new(&adapter.addresses).small().color(p.faint))
                .wrap_mode(egui::TextWrapMode::Truncate),
        )
        .on_hover_text(&adapter.addresses);
    }
    ui.label(RichText::new(note).small().color(p.muted));
}

/// The activity of an adapter in the last seconds
fn sparkline(ui: &mut Ui, packets: &[u128]) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 24.0), egui::Sense::hover());
    let max = packets.iter().copied().max().unwrap_or(0).max(1) as f32;
    let n = packets.len().max(2) - 1;
    let points: Vec<egui::Pos2> = packets
        .iter()
        .enumerate()
        .map(|(i, v)| {
            egui::pos2(
                rect.left() + rect.width() * i as f32 / n as f32,
                rect.bottom() - rect.height() * (*v as f32 / max),
            )
        })
        .collect();
    if points.len() > 1 {
        ui.painter()
            .add(egui::Shape::line(points, egui::Stroke::new(1.5, p.accent)));
    }
}
