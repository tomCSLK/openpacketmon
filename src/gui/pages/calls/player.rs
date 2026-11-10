//! Listening to a call (`player()` of the draft): the waveform of both directions, play and pause,
//! a click on the waveform jumps there, which side to hear; before the first listening or saving
//! the note on § 201 of the criminal code; saving as WAV.

use std::num::NonZero;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, CornerRadius, Rect, RichText, Sense, Ui};
use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink};

use crate::engine::Engine;
use crate::gui::load::{Loader, Task};
use crate::gui::theme::{self, mono_font, palette};
use crate::gui::widgets::{self, Kind};
use crate::gui::{App, frame, icons};
use crate::view::calls::CallKey;
use crate::view::calls::audio::{self, BARS, Bar, CallAudio, Channel};
use crate::view::format;
use crate::view::i18n::Lang;

/// The column of the names left of the waveform (`.player .lane`)
const LABEL_W: f32 = 124.0;
const WAVE_H: f32 = 44.0;

/// What waits for the note on § 201
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ask {
    Play,
    Wav,
}

/// The sound output while a call plays
struct Playing {
    _sink: MixerDeviceSink,
    _output: rodio::Player,
    from: f64,
    started: Instant,
}

impl Playing {
    fn position(&self) -> f64 {
        self.from + self.started.elapsed().as_secs_f64()
    }
}

#[derive(Default)]
pub struct Player {
    audio: Loader<(CallKey, usize), Option<CallAudio>>,
    /// The call (and capture) the player stands in
    key: Option<(CallKey, usize)>,
    playing: Option<Playing>,
    channel: Channel,
    position: f64,
    /// No sound output could be opened
    no_output: bool,
    pub(crate) ask: Option<Ask>,
    wav: Task<Saved>,
}

/// How saving as WAV ended
pub enum Saved {
    Cancelled,
    /// The path of the file
    Done(String),
    /// The error
    Failed(String),
}

/// What the card asked for
#[derive(Default, Clone, Copy)]
pub struct Actions {
    play: bool,
    seek: Option<f64>,
    channel: Option<Channel>,
}

impl Player {
    /// Stops playing: another page, another call
    pub fn stop(&mut self) {
        if let Some(playing) = self.playing.take() {
            self.position = playing.position();
        }
    }

    fn now(&self) -> f64 {
        self.playing
            .as_ref()
            .map_or(self.position, Playing::position)
    }

    /// Loads the voice of this call; `every` again while it runs
    pub fn load(
        &mut self,
        ctx: &egui::Context,
        t: Lang,
        engine: &Engine,
        key: (CallKey, usize),
        every: Option<Duration>,
    ) {
        if self.key != Some(key) {
            self.stop();
            *self = Self {
                audio: std::mem::take(&mut self.audio),
                wav: std::mem::take(&mut self.wav),
                key: Some(key),
                ..Self::default()
            };
        }
        self.audio.poll();
        let engine = engine.clone();
        let call = key.0;
        self.audio.load(ctx, &key, every, move || {
            audio::call_audio(t, &engine, call)
        });
    }

    pub fn audio(&self, key: &(CallKey, usize)) -> Option<&CallAudio> {
        self.audio.value_of(key).and_then(Option::as_ref)
    }

    /// The voice of `call` is loaded (for the pictures of the tests)
    #[cfg(test)]
    pub fn has_audio(&self, call: CallKey) -> bool {
        self.key.is_some_and(|key| {
            key.0 == call && self.audio.value_of(&key).is_some_and(Option::is_some)
        })
    }

    /// Stands at `position` seconds and hears `channel`, without sound (for the pictures)
    #[cfg(test)]
    pub fn show_at(&mut self, position: f64, channel: Channel) {
        self.position = position;
        self.channel = channel;
    }

    fn start(&mut self, audio: &CallAudio, from: f64) {
        self.stop();
        let from = if from >= audio.seconds { 0.0 } else { from };
        self.position = from;
        let (Ok(mut sink), Some(channels), Some(rate)) = (
            DeviceSinkBuilder::open_default_sink(),
            NonZero::new(1_u16),
            NonZero::new(audio::RATE),
        ) else {
            self.no_output = true;
            return;
        };
        sink.log_on_drop(false);
        let output = rodio::Player::connect_new(sink.mixer());
        output.append(SamplesBuffer::new(
            channels,
            rate,
            audio.samples(self.channel, from),
        ));
        output.play();
        self.no_output = false;
        self.playing = Some(Playing {
            _sink: sink,
            _output: output,
            from,
            started: Instant::now(),
        });
    }

    /// Carries out what the card asked for, the note on § 201 first
    pub fn act(&mut self, audio: &CallAudio, actions: Actions, confirmed: bool) {
        if let Some(channel) = actions.channel {
            self.channel = channel;
            if self.playing.is_some() {
                let at = self.now();
                self.start(audio, at);
            }
        }
        if let Some(at) = actions.seek {
            if self.playing.is_some() {
                self.start(audio, at);
            } else {
                self.position = at;
            }
        }
        if actions.play {
            if self.playing.is_some() {
                self.stop();
            } else if confirmed {
                let at = self.position;
                self.start(audio, at);
            } else {
                self.ask = Some(Ask::Play);
            }
        }
        if let Some(playing) = &self.playing
            && playing.position() >= audio.seconds
        {
            self.playing = None;
            self.position = audio.seconds;
        }
    }

    /// Plays after the note on § 201 was confirmed
    pub fn play_now(&mut self, audio: &CallAudio) {
        let at = self.position;
        self.start(audio, at);
    }

    /// Saves the call as WAV, after a dialog for the file
    pub fn save(&self, ctx: &egui::Context, audio: &CallAudio) {
        let audio = audio.clone();
        self.wav.start(ctx, move || {
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(&audio.file_name)
                .add_filter("WAV", &["wav"])
                .save_file()
            else {
                return Saved::Cancelled;
            };
            match std::fs::write(&path, audio.wav()) {
                Ok(()) => Saved::Done(path.display().to_string()),
                Err(e) => Saved::Failed(e.to_string()),
            }
        });
    }

    /// The result of saving, once
    pub fn saved(&self) -> Option<Saved> {
        self.wav.take()
    }

    /// The body of the card "Anhören"
    pub fn card(&self, ui: &mut Ui, t: Lang, audio: &CallAudio) -> Actions {
        let p = palette(ui.ctx());
        let mut actions = Actions::default();
        let now = self.now().min(audio.seconds);
        let share = if audio.seconds > 0.0 {
            (now / audio.seconds) as f32
        } else {
            0.0
        };
        ui.spacing_mut().item_spacing.y = theme::SP3;
        let lanes = [
            (
                icons::ARROW_UP,
                t.get("call-play-sent"),
                &audio.sent_name,
                &audio.sent_bars,
                p.s2,
                self.channel == Channel::Received,
            ),
            (
                icons::ARROW_DOWN,
                t.get("call-play-received"),
                &audio.received_name,
                &audio.received_bars,
                p.s1,
                self.channel == Channel::Sent,
            ),
        ];
        let narrow = ui.ctx().content_rect().width() <= 900.0;
        for (icon, label, name, bars, color, mute) in lanes {
            if let Some(at) = lane(ui, icon, &label, name, bars, color, mute, share, narrow) {
                actions.seek = Some(f64::from(at) * audio.seconds);
            }
        }
        // the axis below the waveforms (`.axis`, 18 px high), 6 px closer (`margin: -6px 0 0 …`)
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), theme::FS_XS * 1.5 - 6.0),
            Sense::hover(),
        );
        let indent = if narrow { 0.0 } else { LABEL_W + theme::SP3 };
        let y = rect.top() - 6.0 + theme::FS_XS * 0.75;
        for (text, x, align) in [
            ("0:00".to_owned(), rect.left() + indent, Align2::LEFT_CENTER),
            (
                clock(audio.seconds / 2.0),
                f32::midpoint(rect.left() + indent, rect.right()),
                Align2::CENTER_CENTER,
            ),
            (clock(audio.seconds), rect.right(), Align2::RIGHT_CENTER),
        ] {
            ui.painter().text(
                egui::pos2(x, y),
                align,
                text,
                mono_font(theme::FS_XS),
                p.text_3,
            );
        }
        // `.ctl` wraps when the card is narrow
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(theme::SP3, theme::SP3);
            let (icon, label) = if self.playing.is_some() {
                (icons::PAUSE, t.get("call-pause"))
            } else {
                (icons::PLAY, t.get("call-play"))
            };
            if widgets::icon_text_button(ui, Some(icon), &label, Kind::Primary, false).clicked() {
                actions.play = true;
            }
            // `.time`: at least 96 px, so the controls do not move while it counts
            let time = ui.painter().layout_no_wrap(
                format!("{} / {}", clock(now), clock(audio.seconds)),
                mono_font(theme::FS_SM),
                p.text_2,
            );
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(time.size().x.max(96.0), time.size().y),
                Sense::hover(),
            );
            ui.painter().galley(rect.min, time, p.text_2);
            let labels = [
                t.get("call-play-both"),
                t.get("call-play-sent"),
                t.get("call-play-received"),
            ];
            let chosen = match self.channel {
                Channel::Both => 0,
                Channel::Sent => 1,
                Channel::Received => 2,
            };
            if let Some(i) = widgets::segmented_sized(ui, &labels, chosen, true) {
                actions.channel = Some(match i {
                    1 => Channel::Sent,
                    2 => Channel::Received,
                    _ => Channel::Both,
                });
            }
        });
        let mut help = String::new();
        if audio.lost {
            help.push_str(&t.get("call-play-lost"));
            help.push(' ');
        }
        if audio.running {
            help.push_str(&t.get("call-play-running"));
            help.push(' ');
        }
        help.push_str(&t.get("call-play-needs-packets"));
        ui.label(widgets::small(help).color(p.text_3));
        if self.no_output {
            ui.label(widgets::small(t.get("call-play-no-output")).color(p.err));
        }
        if self.playing.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(30));
        }
        actions
    }
}

/// `1:31`
fn clock(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    format::clock(seconds.max(0.0).round() as u64)
}

/// A direction: its name, its waveform and the playhead; returns where a click on it points (0–1).
/// `narrow` puts the name above the waveform (`@media (max-width: 900px)` of the draft).
#[allow(clippy::too_many_arguments)]
fn lane(
    ui: &mut Ui,
    icon: egui::ImageSource<'static>,
    label: &str,
    name: &str,
    bars: &[Bar],
    color: Color32,
    mute: bool,
    share: f32,
    narrow: bool,
) -> Option<f32> {
    let p = palette(ui.ctx());
    // the name (`.lbl`, line height 1.3): an arrow and the direction, the side in small letters below
    let first = theme::FS_SM * 1.3;
    let lines = first + theme::FS_XS * 1.3;
    let (top, left, label_w, wave) = if narrow {
        let (block, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), lines), Sense::hover());
        ui.add_space(theme::SP1 - ui.spacing().item_spacing.y);
        let (wave, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), WAVE_H), Sense::hover());
        (block.top(), block.left(), block.width(), wave)
    } else {
        let (row, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), WAVE_H), Sense::hover());
        let wave = Rect::from_min_max(
            egui::pos2(row.left() + LABEL_W + theme::SP3, row.top()),
            row.right_bottom(),
        );
        (row.center().y - lines / 2.0, row.left(), LABEL_W, wave)
    };
    let icon_rect = Rect::from_min_size(
        egui::pos2(left, top + (first - widgets::ICON_SM) / 2.0),
        egui::vec2(widgets::ICON_SM, widgets::ICON_SM),
    );
    widgets::paint_icon(ui, icon, icon_rect, p.text_2);
    ui.painter().text(
        egui::pos2(icon_rect.right() + 4.0, top + first / 2.0),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(theme::FS_SM),
        p.text_2,
    );
    let mut job = egui::text::LayoutJob::simple_singleline(
        name.to_owned(),
        mono_font(theme::FS_XS),
        p.text_3,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(label_w);
    let small = ui.painter().layout_job(job);
    let y = top + first + (theme::FS_XS * 1.3 - small.size().y) / 2.0;
    ui.painter().galley(egui::pos2(left, y), small, p.text_3);
    // the waveform (`.wave`): bars with 1 px between them, 2 px inside the edge
    let response = ui.interact(wave, ui.scope_id().with(("wave", label)), Sense::click());
    ui.painter()
        .rect_filled(wave, CornerRadius::same(theme::RADIUS_SM), p.surface_2);
    let inner = wave.shrink2(egui::vec2(2.0, 0.0));
    #[allow(clippy::cast_precision_loss)]
    let n = bars.len().max(1) as f32;
    let width = ((inner.width() - (n - 1.0)) / n).max(1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let done = (share * BARS as f32).floor() as usize;
    for (i, bar) in bars.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let x = inner.left() + i as f32 * (width + 1.0);
        let height = (0.08 + 0.92 * bar.level) * WAVE_H;
        let rect = Rect::from_center_size(
            egui::pos2(x + width / 2.0, wave.center().y),
            egui::vec2(width, height),
        );
        // `.wave.mute i` comes last in the draft and dims the red bars too
        let alpha = if mute {
            0.12
        } else if bar.gap || i < done {
            1.0
        } else {
            0.45
        };
        let fill = if bar.gap { p.err } else { color }.gamma_multiply(alpha);
        ui.painter().rect_filled(rect, CornerRadius::same(1), fill);
    }
    // the playhead (`.head`): 2 px, 3 px over the edges
    let x = wave.left() + share * wave.width();
    ui.painter().rect_filled(
        Rect::from_min_max(
            egui::pos2(x - 1.0, wave.top() - 3.0),
            egui::pos2(x + 1.0, wave.bottom() + 3.0),
        ),
        CornerRadius::same(1),
        p.text,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
        .clicked()
        .then(|| response.interact_pointer_pos())
        .flatten()
        .map(|at| ((at.x - wave.left()) / wave.width()).clamp(0.0, 1.0))
}

/// The note on § 201 of the criminal code (`lawModal`); returns what it was confirmed for
pub fn law_dialog(app: &mut App, ctx: &egui::Context) -> Option<Ask> {
    let ask = app.pages.calls.player.ask?;
    let t = app.t;
    let mut yes = false;
    let mut cancel = false;
    let close = frame::dialog(
        ctx,
        "law-dialog",
        t,
        &t.get("call-law-title"),
        Some(icons::ALERT),
        |ui| {
            // `.mbody p` with the line height of the page (1.45)
            for id in ["call-law-text", "call-law-text-2"] {
                ui.label(
                    RichText::new(t.get(id))
                        .size(theme::FS_BASE)
                        .line_height(Some(theme::FS_BASE * 1.45)),
                );
            }
            ui.label(
                RichText::new(t.get("call-law-once"))
                    .size(theme::FS_SM)
                    .line_height(Some(theme::FS_SM * 1.45))
                    .color(palette(ui.ctx()).text_3),
            );
            // with the spacing before it: the margin below the last paragraph (`.law p`, 8 px)
            // and the half leading of its line
            ui.add_space(2.0);
        },
        |ui| {
            let label = t.get(match ask {
                Ask::Play => "call-law-play",
                Ask::Wav => "call-law-save",
            });
            yes = widgets::button(ui, label, Kind::Primary).clicked();
            cancel = widgets::button(ui, t.get("call-law-cancel"), Kind::Secondary).clicked();
        },
    );
    if yes || cancel || close {
        app.pages.calls.player.ask = None;
    }
    if !yes {
        return None;
    }
    let today = jiff::Zoned::now().date().to_string();
    app.engine.lock().conf.settings.calls.law_confirmed = today;
    app.engine.store_conf();
    let view = crate::view::settings::settings(t, &app.engine.lock());
    app.pages.settings.law_changed(view.calls_law);
    Some(ask)
}
