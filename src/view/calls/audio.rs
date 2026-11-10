//! Listening to a call: G.711 decoded after ITU-T Recommendation G.711 (own code, no other
//! player's), the voice of both directions on one timeline, a waveform for the page and a WAV file
//! (8 kHz, 16 bit, stereo: left sent, right received). Lost packets stay silent.

use std::sync::Arc;

use super::{
    CallKey, Engine, Lang, Mark, State, Stream, Summary, build, codec, describe, dissect, rtp,
    rtp_near, scan,
};

/// Samples per second of G.711
pub const RATE: u32 = 8000;
/// Bars of the waveform per direction (`BARS` of the draft)
pub const BARS: usize = 150;
/// The longest stretch of voice that is put together, in seconds
const MAX_SECONDS: i64 = 4 * 3600;
/// Below this level (RMS of 16-bit samples) the bars of the waveform do not grow
const QUIET: f64 = 1_000.0;

/// The two companding laws of G.711
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Law {
    /// A-law (PCMA, payload type 8)
    A,
    /// µ-law (PCMU, payload type 0)
    Mu,
}

impl Law {
    pub fn decode(self, byte: u8) -> i16 {
        match self {
            Law::A => alaw(byte),
            Law::Mu => ulaw(byte),
        }
    }
}

/// A-law to linear (G.711 table 1a and 2a): even bits inverted, sign, three bits of segment and four
/// of step; the value lies in the middle of its step
pub fn alaw(byte: u8) -> i16 {
    let a = byte ^ 0x55;
    let segment = (a >> 4) & 0x07;
    let step = i16::from(a & 0x0f) << 4;
    let magnitude = match segment {
        0 => step + 8,
        _ => (step + 0x108) << (segment - 1),
    };
    if a & 0x80 == 0 { -magnitude } else { magnitude }
}

/// µ-law to linear (G.711 table 1b and 2b): all bits inverted, sign, segment and step, with the
/// bias of 33 (here 132, scaled to 16 bit) taken off again
pub fn ulaw(byte: u8) -> i16 {
    let u = !byte;
    let segment = (u >> 4) & 0x07;
    let magnitude = ((i16::from(u & 0x0f) << 3) + 0x84) << segment;
    if u & 0x80 == 0 {
        magnitude - 0x84
    } else {
        0x84 - magnitude
    }
}

/// Which side to hear (`Beide | Gesendet | Empfangen`)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Channel {
    #[default]
    Both,
    Sent,
    Received,
}

/// A bar of the waveform: its height from 0 to 1, and whether packets of it were lost
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bar {
    pub level: f32,
    pub gap: bool,
}

/// The voice of a call, ready to play
#[derive(Clone, Debug)]
pub struct CallAudio {
    pub sent: Arc<[i16]>,
    pub received: Arc<[i16]>,
    pub seconds: f64,
    pub sent_bars: Vec<Bar>,
    pub received_bars: Vec<Bar>,
    /// Packets of a stream are missing: the red bars of the draft
    pub lost: bool,
    /// The names of both sides (`Anna Beispiel`, `support@example.com`)
    pub sent_name: String,
    pub received_name: String,
    /// The call goes on: what is stored so far
    pub running: bool,
    /// `openpacketmon-call-20261110-144057.wav`, after the start of the call
    pub file_name: String,
}

impl CallAudio {
    /// The samples from `from` seconds on, as the output plays them: one channel, both sides
    /// mixed or one of them
    pub fn samples(&self, channel: Channel, from: f64) -> Vec<f32> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let start = (from.max(0.0) * f64::from(RATE)) as usize;
        let len = self.sent.len().max(self.received.len());
        let at = |side: &[i16], i: usize| i32::from(side.get(i).copied().unwrap_or(0));
        (start..len)
            .map(|i| {
                let value = match channel {
                    Channel::Both => at(&self.sent, i) + at(&self.received, i),
                    Channel::Sent => at(&self.sent, i),
                    Channel::Received => at(&self.received, i),
                };
                #[allow(clippy::cast_precision_loss)]
                let value = value.clamp(-32_768, 32_767) as f32 / 32_768.0;
                value
            })
            .collect()
    }

    /// A WAV file: 8 kHz, 16 bit, stereo, left sent, right received
    pub fn wav(&self) -> Vec<u8> {
        let len = self.sent.len().max(self.received.len());
        let data = u32::try_from(len * 4).unwrap_or(u32::MAX - 36);
        let mut out = Vec::with_capacity(44 + len * 4);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        // PCM, two channels, the rate, bytes per second, bytes per frame, bits per sample
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&2_u16.to_le_bytes());
        out.extend_from_slice(&RATE.to_le_bytes());
        out.extend_from_slice(&(RATE * 4).to_le_bytes());
        out.extend_from_slice(&4_u16.to_le_bytes());
        out.extend_from_slice(&16_u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data.to_le_bytes());
        for i in 0..len {
            for side in [&self.sent, &self.received] {
                out.extend_from_slice(&side.get(i).copied().unwrap_or(0).to_le_bytes());
            }
        }
        out
    }
}

/// One direction put together: the samples and the stretches of lost packets
#[derive(Default)]
struct Side {
    samples: Vec<i16>,
    gaps: Vec<(usize, usize)>,
}

/// The voice of a call, if it can be played (G.711, not encrypted, whole packets)
pub fn call_audio(t: Lang, engine: &Engine, key: CallKey) -> Option<CallAudio> {
    let scan = scan(t, engine);
    let calls = build(&scan, Some(key));
    let b = calls.built.get(&key)?;
    let d = {
        let state = engine.lock();
        describe(t, &state, &calls, key, b, scan.now, scan.headers_only)
    };
    if !d.playable {
        return None;
    }
    let summary =
        matches!(key, CallKey::Sip(_)).then(|| Summary::read(b, scan.now, &calls.registrants));
    let near = rtp_near(b, &calls.registrants);
    let sent_of = |s: &Stream| match &summary {
        Some(summary) => summary.sent(s),
        None => Some(s.src) == near,
    };
    let main = |sent: bool| {
        b.streams
            .iter()
            .filter(|s| sent_of(s) == sent)
            .max_by_key(|s| s.packets)
    };
    let (out, back) = (main(true), main(false));
    let first = [out, back].iter().flatten().map(|s| s.first_time).min()?;
    // the timeline is the talk time (`Gesprächszeit`), and the voice before it (early media):
    // from the answer or the first voice to the end of the call or the last voice
    let talk = summary
        .as_ref()
        .and_then(|s| s.answer.map(|answer| (answer, s.stop)));
    let start = talk.map_or(first, |(answer, _)| answer.min(first));
    let side = |stream: Option<&Stream>| {
        stream.map_or_else(Side::default, |s| {
            let formats = calls
                .media
                .get(&s.media)
                .map_or_else(|| Arc::from([]), |f| f.formats.clone());
            let c = codec(t, s.payload_type, &formats);
            let law = if c.short.contains("A-law") {
                Law::A
            } else {
                Law::Mu
            };
            put_together(&scan, s, law, start)
        })
    };
    let (mut sent, mut received) = (side(out), side(back));
    let until = talk.map_or(0, |(_, stop)| {
        ((stop - start).clamp(0, MAX_SECONDS * 1_000_000) * i64::from(RATE) / 1_000_000)
            .try_into()
            .unwrap_or(0)
    });
    let len = sent.samples.len().max(received.samples.len()).max(until);
    // both sides as long as the timeline: the WAV file and the output end with it
    sent.samples.resize(len, 0);
    received.samples.resize(len, 0);
    let bars = |s: &Side, reference: f64| waveform(s, len, reference);
    let reference = [&sent, &received]
        .iter()
        .flat_map(|s| levels(&s.samples, len))
        .fold(QUIET, f64::max);
    let (sent_name, received_name) = if d.outgoing {
        (d.from.name.clone(), d.to.name.clone())
    } else {
        (d.to.name.clone(), d.from.name.clone())
    };
    let file_name = jiff::Timestamp::from_microsecond(d.start).map_or_else(
        |_| "openpacketmon-call.wav".to_owned(),
        |time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("openpacketmon-call-%Y%m%d-%H%M%S.wav")
                .to_string()
        },
    );
    #[allow(clippy::cast_precision_loss)]
    let seconds = len as f64 / f64::from(RATE);
    Some(CallAudio {
        sent_bars: bars(&sent, reference),
        received_bars: bars(&received, reference),
        lost: !sent.gaps.is_empty() || !received.gaps.is_empty(),
        sent: Arc::from(sent.samples),
        received: Arc::from(received.samples),
        seconds,
        sent_name,
        received_name,
        running: d.status.state == State::Running,
        file_name,
    })
}

/// The samples of one stream, each packet at its timestamp (RTP is not played in the order the
/// packets came, but in the order they were spoken); the first packet of the stream at the time
/// it came after `start`
fn put_together(scan: &super::Scan, s: &Stream, law: Law, start: i64) -> Side {
    let mut pieces: Vec<(i64, i64, Vec<i16>)> = Vec::new();
    let (mut seq, mut ts) = (None::<(u16, i64)>, None::<(u32, i64)>);
    for seen in &scan.seen {
        let Mark::Media { media, rtcp: false } = seen.mark else {
            continue;
        };
        if media != s.media {
            continue;
        }
        let Some((src, dst, payload)) = dissect::udp_ends(&seen.data, scan.link_type) else {
            continue;
        };
        if (src, dst) != (s.src, s.dst) {
            continue;
        }
        let m = &seen.data[payload];
        let Some(h) = rtp::rtp_header(m) else {
            continue;
        };
        if h.ssrc != s.ssrc || h.payload_type != s.payload_type {
            continue;
        }
        // numbers and timestamps go on past their wrap (RFC 3550 A.1)
        let n = match seq {
            None => 0,
            Some((last, at)) => at + i64::from(h.seq.wrapping_sub(last).cast_signed()),
        };
        seq = Some((h.seq, n));
        let when = match ts {
            None => 0,
            Some((last, at)) => at + i64::from(h.timestamp.wrapping_sub(last).cast_signed()),
        };
        ts = Some((h.timestamp, when));
        let voice = m[h.payload].iter().map(|b| law.decode(*b)).collect();
        pieces.push((n, when, voice));
    }
    let offset = (s.first_time - start).max(0) * i64::from(RATE) / 1_000_000;
    let limit = MAX_SECONDS * i64::from(RATE);
    let mut side = Side::default();
    for (_, when, voice) in &pieces {
        let at = offset + when;
        if at < 0 || at + i64::try_from(voice.len()).unwrap_or(limit) > limit {
            continue;
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let at = at as usize;
        if side.samples.len() < at + voice.len() {
            side.samples.resize(at + voice.len(), 0);
        }
        side.samples[at..at + voice.len()].copy_from_slice(voice);
    }
    // a jump in the numbers: the packets between were lost, their time stays silent
    pieces.sort_by_key(|(n, _, _)| *n);
    for pair in pieces.windows(2) {
        let ((n1, when1, voice1), (n2, when2, _)) = (&pair[0], &pair[1]);
        if n2 - n1 > 1 {
            let from = offset + when1 + i64::try_from(voice1.len()).unwrap_or(0);
            let to = offset + when2;
            if from >= 0 && to > from && to <= limit {
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                side.gaps.push((from as usize, to as usize));
            }
        }
    }
    side
}

/// The loudness (RMS) of each bar of `len` samples
fn levels(samples: &[i16], len: usize) -> Vec<f64> {
    (0..BARS)
        .map(|i| {
            let (from, to) = (i * len / BARS, (i + 1) * len / BARS);
            let part = samples.get(from..to.min(samples.len())).unwrap_or_default();
            if part.is_empty() {
                return 0.0;
            }
            let sum: f64 = part.iter().map(|v| f64::from(*v).powi(2)).sum();
            #[allow(clippy::cast_precision_loss)]
            let rms = (sum / part.len() as f64).sqrt();
            rms
        })
        .collect()
}

fn waveform(side: &Side, len: usize, reference: f64) -> Vec<Bar> {
    levels(&side.samples, len)
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let (from, to) = (i * len / BARS, (i + 1) * len / BARS);
            #[allow(clippy::cast_possible_truncation)]
            let level = (level / reference).min(1.0) as f32;
            Bar {
                level,
                gap: side.gaps.iter().any(|(a, b)| *a < to && *b > from),
            }
        })
        .collect()
}
