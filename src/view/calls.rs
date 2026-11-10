//! The calls of the capture (`pageCalls` and `pageCall` of the draft): calls from the SIP dialogs
//! with their messages, the voice streams of each call by SSRC with loss and jitter (RFC 3550,
//! appendix A.1 and A.8), and RTP streams found without SIP. All of it is read from the stored
//! packets and from what the analysis found about them (`engine::analysis::voip`).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use sniffnet_packet_parser::LinkType;

use crate::dissect::{self, Dissection, SdpMedia, Value, rtp};
use crate::engine::Engine;
use crate::engine::analysis::{self, PacketAnalysis, voip::Mark};
use crate::engine::state::EngineState;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::program::Program;
use crate::view::entries::program_label;
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::packets::unavailable;

/// SIP messages kept per call for its flow
const MAX_MESSAGES: usize = 500;
/// Loss in percent and mean jitter in milliseconds from which a stream is fair or bad
const FAIR_LOSS: f64 = 1.0;
const BAD_LOSS: f64 = 3.0;
const FAIR_JITTER: f64 = 30.0;
const BAD_JITTER: f64 = 50.0;
/// Where a router of AVM writes its captures, for the help on the page
pub const FRITZ_CAPTURE: &str = "fritz.box/html/capture.html";

/// A call: a SIP dialog started by INVITE, or RTP without SIP (the stream with the lower number
/// of its two directions)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CallKey {
    Sip(u32),
    Rtp(u32),
}

/// The icons the pages of calls use
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Phone,
    Check,
    Cross,
    Alert,
    Lock,
    Activity,
    Help,
}

/// How a call went (`CST` of the draft)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Running,
    Ended,
    Rejected,
    Missed,
    Cancelled,
    Ringing,
    Setup,
    Rtp,
}

impl State {
    fn id(self) -> &'static str {
        match self {
            Self::Running => "call-state-running",
            Self::Ended => "call-state-ended",
            Self::Rejected => "call-state-rejected",
            Self::Missed => "call-state-missed",
            Self::Cancelled => "call-state-cancelled",
            Self::Ringing => "call-state-ringing",
            Self::Setup => "call-state-setup",
            Self::Rtp => "call-state-rtp",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::Running => Icon::Activity,
            Self::Ended => Icon::Check,
            Self::Rejected | Self::Cancelled => Icon::Cross,
            Self::Missed => Icon::Alert,
            Self::Ringing | Self::Setup => Icon::Phone,
            Self::Rtp => Icon::Help,
        }
    }

    /// The color of the state: `run` for a call going on, `warn` for one that failed
    pub fn tone(self) -> Tone {
        match self {
            Self::Running | Self::Ringing => Tone::Run,
            Self::Rejected | Self::Missed => Tone::Warn,
            _ => Tone::Plain,
        }
    }
}

/// The color of a state (`.cstate`, `.cstate.run`, `.cstate.warn`)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Plain,
    Run,
    Warn,
}

/// The state of a call as shown: its name and what it means for this call
#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub state: State,
    pub label: String,
    pub sub: String,
}

/// The quality of a voice stream (`RATE` of the draft)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rate {
    Good,
    Fair,
    Bad,
}

impl Rate {
    pub fn label(self, t: Lang) -> String {
        t.get(match self {
            Self::Good => "rate-good",
            Self::Fair => "rate-fair",
            Self::Bad => "rate-bad",
        })
    }
}

/// A side of a call in the list (`party()` of the draft)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Party {
    pub name: String,
    /// The name is a name (bold), not an address
    pub strong: bool,
    /// An address in the font of addresses
    pub mono: bool,
    pub sub: String,
    /// The whole address, under the pointer
    pub title: String,
}

/// A tile above the list of calls
#[derive(Clone, Debug, PartialEq)]
pub struct CallTile {
    pub icon: Icon,
    pub label: String,
    pub value: String,
    pub sub: String,
    pub warn: bool,
}

/// A row of the list of calls
#[derive(Clone, Debug, PartialEq)]
pub struct CallRow {
    pub key: CallKey,
    pub status: Status,
    pub start: String,
    pub duration: String,
    pub duration_sub: String,
    pub outgoing: bool,
    pub direction: String,
    pub from: Party,
    pub to: Party,
    /// Empty: none (a dash)
    pub codec: String,
    pub codec_title: String,
    pub rate: Option<Rate>,
    pub rate_title: String,
    pub packets: String,
    pub playable: bool,
}

/// The latest registration at a SIP server (the foot of the list)
#[derive(Clone, Debug, PartialEq)]
pub struct Registration {
    pub account: String,
    /// `bei sip.example.net um 14:36:09 Uhr, gültig 1 Stunde`
    pub text: String,
    pub packet: u64,
    /// The password is not in the packets, only a digest (empty without one)
    pub digest: String,
}

/// The list of calls
#[derive(Default)]
pub struct CallList {
    pub live: bool,
    pub unavailable: String,
    pub tiles: Vec<CallTile>,
    pub rows: Vec<CallRow>,
    pub registration: Option<Registration>,
}

/// A piece of a value on the page of a call
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    Strong(String),
    /// Gray (`.muted`, `.links`)
    Muted(String),
    Mono(String),
    /// Small and gray in the font of addresses (a path, an address next to a name)
    SmallMono(String),
    /// The number of a packet as a link
    Packet(u64),
    /// A link to the packets of this display filter
    Packets(String, String),
    State(Status),
    /// An arrow up (sent) or down (received)
    Dir(bool),
    /// The call goes on (`.cstate.run`)
    Running(String),
}

/// A row of the facts of a call
#[derive(Clone, Debug, PartialEq)]
pub struct Fact {
    pub label: String,
    pub parts: Vec<Part>,
}

/// A row of the flow of a call (the ladder)
#[derive(Clone, Debug, PartialEq)]
pub enum FlowRow {
    /// A SIP message: from this computer to the right, else to the left
    Message {
        number: u64,
        at: String,
        clock: String,
        label: String,
        right: bool,
        bad: bool,
        ok: bool,
    },
    /// The voice streams as a band from this computer to the far end of the media
    Media {
        number: u64,
        at: String,
        lead: String,
        text: String,
        locked: bool,
        filter: String,
    },
}

/// The flow of a call
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flow {
    pub hint: String,
    /// The lanes: name and what is below it
    pub lanes: Vec<(String, String)>,
    pub rows: Vec<FlowRow>,
}

/// A voice stream of a call
#[derive(Clone, Debug, PartialEq)]
pub struct StreamRow {
    pub sent: bool,
    pub direction: String,
    pub path: String,
    pub ssrc: String,
    pub filter: String,
    pub packets: String,
    /// Lost packets and their share, `None` without loss
    pub lost: Option<(String, String)>,
    /// Mean and largest jitter, `None` without a clock rate
    pub jitter: Option<String>,
    pub gap: String,
    pub late: String,
    pub rate: Option<Rate>,
    pub rate_title: String,
}

/// What the card "Anhören" shows
#[derive(Clone, Debug, PartialEq)]
pub enum Listen {
    /// Nothing to play: a lock for content that cannot be read, else a phone
    Off {
        lock: bool,
        heading: String,
        text: String,
    },
    /// The voice can be played
    Ready { hint: String },
}

/// The page of a call
#[derive(Clone, Debug, PartialEq)]
pub struct CallPage {
    pub live: bool,
    pub sip: bool,
    pub from: String,
    pub to: String,
    pub status: Status,
    /// The display filter of its packets, and the label of the button
    pub filter: String,
    pub packets_button: String,
    pub notice: Option<(String, String)>,
    pub card: String,
    pub facts: Vec<Fact>,
    pub listen: Listen,
    pub flow: Flow,
    pub streams: Vec<StreamRow>,
    /// Without a stream: heading and text
    pub no_streams: Option<(String, String)>,
    pub rtcp: String,
}

/// A call with a bad connection, for "Auffällig" on the overview
#[derive(Clone, Debug, PartialEq)]
pub struct BadCall {
    pub key: CallKey,
    pub text: String,
}

// ---- reading the packets ------------------------------------------------------------------------

/// A stored packet of a call
struct Seen {
    number: u64,
    record: PacketAnalysis,
    mark: Mark,
    data: Arc<[u8]>,
    conn: Option<AddressPortPair>,
}

/// What the analysis knows about a media stream
#[derive(Clone)]
struct MediaFacts {
    call: Option<u32>,
    setup: Option<u64>,
    srtp: bool,
    formats: Arc<[(u8, String)]>,
    partner: Option<u32>,
}

/// The stored packets of calls, and what goes with them
struct Scan {
    live: bool,
    unavailable: String,
    headers_only: bool,
    link_type: LinkType,
    seen: Vec<Seen>,
    media: HashMap<u32, MediaFacts>,
    /// Time of the newest packet
    now: i64,
}

fn scan(t: Lang, engine: &Engine) -> Scan {
    let (running, finished, unavailable, store) = {
        let state = engine.lock();
        (
            state.is_running(),
            state.traffic_history.no_more_packets(),
            unavailable(t, &state),
            state.packets.clone(),
        )
    };
    let Some(store) = store else {
        return Scan {
            live: false,
            unavailable,
            headers_only: false,
            link_type: LinkType::Ethernet(pcap::Linktype::ETHERNET),
            seen: Vec::new(),
            media: HashMap::new(),
            now: 0,
        };
    };
    let last = store.lock().last_number();
    analysis::advance(&store, last);
    let analysis = store.analysis();
    let marked: Vec<(u64, PacketAnalysis)> = analysis
        .after(0)
        .filter(|(_, r)| matches!(r.voip, Some(Mark::Sip { .. } | Mark::Media { .. })))
        .map(|(n, r)| (n, *r))
        .collect();
    let voip = analysis.voip();
    let mut media = HashMap::new();
    for (_, r) in &marked {
        if let Some(Mark::Media { media: m, .. }) = r.voip {
            media.entry(m).or_insert_with(|| {
                let facts = voip.media(m);
                MediaFacts {
                    call: facts.and_then(|f| f.call),
                    setup: facts.and_then(|f| f.setup),
                    srtp: facts.is_some_and(|f| f.srtp),
                    formats: facts.map_or_else(|| Arc::from([]), |f| f.formats.clone()),
                    partner: voip.partner(m),
                }
            });
        }
    }
    let packets = store.lock();
    let seen = marked
        .into_iter()
        .filter_map(|(number, record)| {
            let p = packets.get(number)?;
            Some(Seen {
                number,
                record,
                mark: record.voip?,
                data: p.data.clone(),
                conn: p.conn.as_ref().map(|(key, _)| *key),
            })
        })
        .collect();
    let now = packets
        .get(packets.last_number())
        .and_then(|p| p.timestamp.to_usecs())
        .unwrap_or(0);
    Scan {
        live: running && !finished,
        unavailable,
        headers_only: packets.headers_only(),
        link_type: packets.link_type(),
        seen,
        media,
        now,
    }
}

/// A SIP message of a call
#[derive(Clone)]
struct Msg {
    number: u64,
    time: i64,
    outgoing: bool,
    src: Option<IpAddr>,
    dst: Option<IpAddr>,
    method: Option<String>,
    status: Option<u16>,
    reason: String,
    cseq: u64,
    cseq_method: String,
    call_id: String,
    from: String,
    to: String,
    uri: String,
    sdp: Vec<SdpMedia>,
    /// The header with the answer to a challenge (`Authorization`, `Proxy-Authorization`)
    auth: Option<String>,
    expires: Option<u64>,
    conn: Option<AddressPortPair>,
}

impl Msg {
    fn read(seen: &Seen, d: &Dissection) -> Self {
        let text = |key: &str| {
            d.values
                .iter()
                .find_map(|(k, v)| match v {
                    Value::Text(t) if *k == key => Some(t.clone()),
                    _ => None,
                })
                .unwrap_or_default()
        };
        let num = |key: &str| {
            d.values.iter().find_map(|(k, v)| match v {
                Value::Num(n) if *k == key => Some(*n),
                _ => None,
            })
        };
        let ip = |keys: [&str; 2]| {
            d.values.iter().rev().find_map(|(k, v)| match v {
                Value::Ip(a) if keys.contains(k) => Some(*a),
                _ => None,
            })
        };
        let fields: Vec<(&str, &str)> = d
            .layers
            .iter()
            .filter(|l| l.title.starts_with("Session Initiation Protocol"))
            .flat_map(|l| l.fields.iter())
            .map(|f| (f.name.as_ref(), f.value.as_str()))
            .collect();
        let header = |names: &[&str]| {
            fields
                .iter()
                .find(|(n, _)| names.iter().any(|w| n.eq_ignore_ascii_case(w)))
                .map(|(_, v)| *v)
        };
        let first = fields.first().map_or("", |(_, v)| *v);
        let status = num("sip.status_code").and_then(|n| u16::try_from(n).ok());
        let mut words = first.splitn(3, ' ');
        let (_, second, third) = (words.next(), words.next(), words.next());
        let (uri, reason) = if status.is_some() {
            (String::new(), third.unwrap_or_default().to_owned())
        } else {
            (second.unwrap_or_default().to_owned(), String::new())
        };
        let auth = ["Proxy-Authorization", "Authorization"]
            .into_iter()
            .find(|name| header(&[name]).is_some_and(|v| v.trim_start().starts_with("Digest")))
            .map(str::to_owned);
        let expires = header(&["Expires"])
            .and_then(|v| v.trim().parse().ok())
            .or_else(|| {
                let contact = header(&["Contact", "m"])?;
                let at = contact.to_ascii_lowercase().find("expires=")?;
                let digits: String = contact[at + 8..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                digits.parse().ok()
            });
        Self {
            number: seen.number,
            time: seen.record.time,
            outgoing: seen.record.outgoing,
            src: ip(["ip.src", "ipv6.src"]),
            dst: ip(["ip.dst", "ipv6.dst"]),
            method: Some(text("sip.method")).filter(|m| !m.is_empty()),
            status,
            reason,
            cseq: num("sip.cseq.seq").unwrap_or_default(),
            cseq_method: text("sip.cseq.method"),
            call_id: text("sip.call_id"),
            from: text("sip.from"),
            to: text("sip.to"),
            uri,
            sdp: d.sdp.clone(),
            auth,
            expires,
            conn: seen.conn,
        }
    }

    fn is(&self, method: &str) -> bool {
        self.method.as_deref() == Some(method)
    }

    /// A response to a request of this method
    fn answers(&self, method: &str) -> bool {
        self.status.is_some() && self.cseq_method == method
    }

    /// The text on the arrow of the flow: `INVITE (SDP)`, `200 OK (SDP)`
    fn label(&self) -> String {
        let mut label = match (&self.method, self.status) {
            (Some(m), _) => m.clone(),
            (None, Some(code)) => format!("{code} {}", self.reason).trim_end().to_owned(),
            (None, None) => String::new(),
        };
        if !self.sdp.is_empty() {
            label.push_str(" (SDP)");
        }
        label
    }
}

/// A voice stream: the RTP packets of one SSRC from one end to another
struct Stream {
    ssrc: u32,
    src: SocketAddr,
    dst: SocketAddr,
    media: u32,
    payload_type: u8,
    packets: u64,
    bytes: u64,
    first: u64,
    first_time: i64,
    last_time: i64,
    outgoing: bool,
    /// Sequence numbers (RFC 3550 A.1): the first, the highest and its wraps
    base: u16,
    max: u16,
    cycles: u32,
    /// Packets that came after a later one (or twice)
    late: u64,
    /// Interarrival jitter (RFC 3550 A.8) in units of the clock
    clock: Option<u32>,
    transit: Option<u32>,
    jitter: f64,
    jitter_sum: f64,
    jitter_max: f64,
    jitter_count: u64,
    /// Largest time between two packets, in microseconds
    gap: i64,
}

impl Stream {
    fn new(seen: &Seen, src: SocketAddr, dst: SocketAddr, media: u32, h: &rtp::RtpHeader) -> Self {
        Self {
            ssrc: h.ssrc,
            src,
            dst,
            media,
            payload_type: h.payload_type,
            packets: 0,
            bytes: 0,
            first: seen.number,
            first_time: seen.record.time,
            last_time: seen.record.time,
            outgoing: seen.record.outgoing,
            base: h.seq,
            max: h.seq.wrapping_sub(1),
            cycles: 0,
            late: 0,
            clock: None,
            transit: None,
            jitter: 0.0,
            jitter_sum: 0.0,
            jitter_max: 0.0,
            jitter_count: 0,
            gap: 0,
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn add(&mut self, time: i64, len: u32, h: &rtp::RtpHeader, clock: Option<u32>) {
        if self.packets > 0 {
            self.gap = self.gap.max(time - self.last_time);
        }
        self.packets += 1;
        self.bytes += u64::from(len);
        self.last_time = time;
        self.payload_type = h.payload_type;
        let step = h.seq.wrapping_sub(self.max).cast_signed();
        if step > 0 {
            if h.seq < self.max {
                self.cycles += 1;
            }
            self.max = h.seq;
        } else {
            self.late += 1;
        }
        let Some(clock) = clock.filter(|c| *c > 0) else {
            return;
        };
        if self.clock != Some(clock) {
            self.clock = Some(clock);
            self.transit = None;
        }
        // the arrival in units of the clock; only differences count, so it may wrap
        let arrival = (i128::from(time) * i128::from(clock) / 1_000_000) as u32;
        let transit = arrival.wrapping_sub(h.timestamp);
        if let Some(before) = self.transit {
            let d = f64::from(transit.wrapping_sub(before).cast_signed().unsigned_abs());
            self.jitter += (d - self.jitter) / 16.0;
            self.jitter_sum += self.jitter;
            self.jitter_max = self.jitter_max.max(self.jitter);
            self.jitter_count += 1;
        }
        self.transit = Some(transit);
    }

    /// Packets expected from the sequence numbers
    fn expected(&self) -> u64 {
        (u64::from(self.cycles) << 16) + u64::from(self.max) + 1 - u64::from(self.base)
    }

    fn lost(&self) -> u64 {
        self.expected().saturating_sub(self.packets)
    }

    #[allow(clippy::cast_precision_loss)]
    fn loss(&self) -> f64 {
        let expected = self.expected();
        if expected == 0 {
            0.0
        } else {
            self.lost() as f64 * 100.0 / expected as f64
        }
    }

    /// Mean and largest jitter in milliseconds
    #[allow(clippy::cast_precision_loss)]
    fn jitter_ms(&self) -> Option<(f64, f64)> {
        let clock = f64::from(self.clock?);
        if self.jitter_count == 0 {
            return Some((0.0, 0.0));
        }
        let mean = self.jitter_sum / self.jitter_count as f64;
        Some((mean * 1000.0 / clock, self.jitter_max * 1000.0 / clock))
    }

    fn rate(&self) -> Rate {
        let loss = self.loss();
        let jitter = self.jitter_ms().map_or(0.0, |(mean, _)| mean);
        if loss >= BAD_LOSS || jitter >= BAD_JITTER {
            Rate::Bad
        } else if loss >= FAIR_LOSS || jitter >= FAIR_JITTER {
            Rate::Fair
        } else {
            Rate::Good
        }
    }
}

/// A call as read from its packets
#[derive(Default)]
struct Built {
    msgs: Vec<Msg>,
    sip: u64,
    rtp: u64,
    rtcp: u64,
    bytes: u64,
    first: Option<(u64, i64)>,
    last_time: i64,
    streams: Vec<Stream>,
    /// Reports of RTCP with the address of their sender
    reports: Vec<(IpAddr, rtp::Report)>,
    /// The first RTP or RTCP packet sent by this computer, for its program
    conn: Option<AddressPortPair>,
    /// The first packet of media, for its program when nothing was sent
    any_conn: Option<AddressPortPair>,
}

impl Built {
    fn count(&mut self, seen: &Seen) {
        self.bytes += u64::from(seen.record.len);
        self.last_time = self.last_time.max(seen.record.time);
        if self.first.is_none() {
            self.first = Some((seen.number, seen.record.time));
        }
    }
}

/// All calls of the capture, and the latest registration
struct Calls {
    built: HashMap<CallKey, Built>,
    registration: Option<(Msg, Msg)>,
    /// Display names by address, from all SIP messages
    names: HashMap<String, String>,
    media: HashMap<u32, MediaFacts>,
    /// Addresses that registered at a SIP server: the phones on this side
    registrants: HashSet<IpAddr>,
}

fn build(scan: &Scan, only: Option<CallKey>) -> Calls {
    let mut built: HashMap<CallKey, Built> = HashMap::new();
    let mut registers: HashMap<(String, u64), Msg> = HashMap::new();
    let mut registration = None;
    let mut names = HashMap::new();
    let mut registrants = HashSet::new();
    for seen in &scan.seen {
        match seen.mark {
            Mark::Sip { call, .. } => {
                let key = call.map(CallKey::Sip);
                if only.is_some() && key != only && call.is_some() {
                    continue;
                }
                let d = dissect::dissect(&seen.data, scan.link_type);
                let msg = Msg::read(seen, &d);
                for value in [&msg.from, &msg.to] {
                    let (name, uri) = split_party(value);
                    if !name.is_empty() {
                        names.insert(uri, name);
                    }
                }
                match key {
                    Some(key) => {
                        let b = built.entry(key).or_default();
                        b.count(seen);
                        b.sip += 1;
                        if b.msgs.len() < MAX_MESSAGES {
                            b.msgs.push(msg);
                        }
                    }
                    None if msg.is("REGISTER") => {
                        registrants.extend(msg.src);
                        registers.insert((msg.call_id.clone(), msg.cseq), msg);
                    }
                    None if msg.answers("REGISTER") && msg.status == Some(200) => {
                        if let Some(request) = registers.get(&(msg.call_id.clone(), msg.cseq)) {
                            let valid = msg.expires.or(request.expires);
                            registration =
                                (valid != Some(0)).then(|| (request.clone(), msg.clone()));
                        }
                    }
                    None => {}
                }
            }
            Mark::Media { media, rtcp } => {
                let Some(facts) = scan.media.get(&media) else {
                    continue;
                };
                let key = match facts.call {
                    Some(call) => CallKey::Sip(call),
                    None => CallKey::Rtp(facts.partner.map_or(media, |p| p.min(media))),
                };
                if only.is_some_and(|o| o != key) {
                    continue;
                }
                let Some((src, dst, payload)) = dissect::udp_ends(&seen.data, scan.link_type)
                else {
                    continue;
                };
                let m = &seen.data[payload];
                let b = built.entry(key).or_default();
                b.count(seen);
                if b.any_conn.is_none() {
                    b.any_conn = seen.conn;
                }
                if seen.record.outgoing && b.conn.is_none() {
                    b.conn = seen.conn;
                }
                if rtcp {
                    b.rtcp += 1;
                    b.reports
                        .extend(rtp::reports(m).into_iter().map(|r| (src.ip(), r)));
                    continue;
                }
                let Some(h) = rtp::rtp_header(m) else {
                    continue;
                };
                b.rtp += 1;
                let clock = rtp::clock_rate(h.payload_type, &facts.formats);
                let known = b
                    .streams
                    .iter()
                    .position(|s| s.ssrc == h.ssrc && s.src == src && s.dst == dst);
                let i = known.unwrap_or_else(|| {
                    b.streams.push(Stream::new(seen, src, dst, media, &h));
                    b.streams.len() - 1
                });
                b.streams[i].add(seen.record.time, seen.record.len, &h, clock);
            }
            Mark::Candidate => {}
        }
    }
    Calls {
        built,
        registration,
        names,
        media: scan.media.clone(),
        registrants,
    }
}

/// Display name and address of `"Anna" <sip:anna@example.net>`
fn split_party(value: &str) -> (String, String) {
    match (value.find('<'), value.rfind('>')) {
        (Some(open), Some(close)) if open < close => (
            value[..open].trim().trim_matches('"').trim().to_owned(),
            value[open + 1..close].trim().to_owned(),
        ),
        _ => (String::new(), value.trim().to_owned()),
    }
}

/// An address without its scheme, as the draft writes it (`anna@sip.example.net`)
fn uri_text(uri: &str) -> String {
    let lower = uri.to_ascii_lowercase();
    for scheme in ["sip:", "sips:", "tel:"] {
        if lower.starts_with(scheme) {
            return uri[scheme.len()..].to_owned();
        }
    }
    uri.to_owned()
}

fn is_anonymous(uri: &str) -> bool {
    uri.to_ascii_lowercase().contains("anonymous")
}

/// The host of a SIP URI (`sip:sip.example.net:5060` → `sip.example.net`)
fn uri_host(uri: &str) -> String {
    let rest = uri_text(uri);
    let rest = rest.rsplit_once('@').map_or(rest.as_str(), |(_, h)| h);
    let rest = rest.split([';', '?', '>']).next().unwrap_or_default();
    match rest.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) && !host.contains(':') => {
            host.to_owned()
        }
        _ => rest.to_owned(),
    }
}

// ---- what a call means ---------------------------------------------------------------------

/// What a SIP call came to
struct Summary {
    state: State,
    outgoing: bool,
    /// The address of this computer in the call
    near: Option<IpAddr>,
    /// Addresses of this computer's media (from its session descriptions)
    near_media: HashSet<IpAddr>,
    invite: Option<usize>,
    ok: Option<usize>,
    ring: Option<usize>,
    end: Option<usize>,
    /// The final response that rejected the call
    failed: Option<usize>,
    start: i64,
    stop: i64,
    answer: Option<i64>,
}

impl Summary {
    /// `registrants` tell this side where the direction of the packets cannot: in a file, every
    /// private address counts as local, the phone as well as the router it calls through
    fn read(b: &Built, now: i64, registrants: &HashSet<IpAddr>) -> Self {
        let find = |f: &dyn Fn(&Msg) -> bool| b.msgs.iter().position(f);
        let invite = find(&|m| m.is("INVITE"));
        let ok = find(&|m| m.answers("INVITE") && m.status == Some(200));
        let ring = find(&|m| m.answers("INVITE") && matches!(m.status, Some(180 | 183)));
        let cancel = find(&|m| m.is("CANCEL"));
        let bye = find(&|m| m.is("BYE"));
        let failed = find(&|m| {
            m.answers("INVITE") && m.status.is_some_and(|s| s >= 300 && s != 401 && s != 407)
        })
        .filter(|_| ok.is_none());
        let first = invite.or((!b.msgs.is_empty()).then_some(0));
        let registered = |ip: Option<IpAddr>| ip.is_some_and(|ip| registrants.contains(&ip));
        let outgoing = first.is_some_and(|i| {
            let m = &b.msgs[i];
            if registered(m.src) {
                true
            } else if registered(m.dst) {
                false
            } else {
                m.outgoing
            }
        });
        let near = first.and_then(|i| {
            let m = &b.msgs[i];
            if outgoing { m.src } else { m.dst }
        });
        let near_media = b
            .msgs
            .iter()
            .filter(|m| near.is_some() && m.src == near)
            .flat_map(|m| m.sdp.iter().filter_map(|s| s.addr))
            .collect();
        let start = b.first.map_or(0, |(_, t)| t);
        let time = |i: usize| b.msgs[i].time;
        let (state, end, stop) = if let Some(i) = cancel {
            let state = if outgoing {
                State::Cancelled
            } else {
                State::Missed
            };
            (state, Some(i), time(i))
        } else if let Some(i) = failed {
            (State::Rejected, Some(i), time(i))
        } else if let (Some(_), Some(i)) = (ok, bye) {
            (State::Ended, Some(i), time(i))
        } else if ok.is_some() {
            (State::Running, None, now.max(b.last_time))
        } else if ring.is_some() {
            (State::Ringing, None, now.max(b.last_time))
        } else {
            (State::Setup, None, now.max(b.last_time))
        };
        Self {
            state,
            outgoing,
            near,
            near_media,
            invite,
            ok,
            ring,
            end,
            failed,
            start,
            stop,
            answer: ok.map(time),
        }
    }

    /// Seconds of talking
    #[allow(clippy::cast_precision_loss)]
    fn talk(&self) -> f64 {
        self.answer
            .map_or(0.0, |a| (self.stop - a).max(0) as f64 / 1e6)
    }

    /// Seconds until the call was answered, or until it ended
    #[allow(clippy::cast_precision_loss)]
    fn wait(&self) -> f64 {
        (self.answer.unwrap_or(self.stop) - self.start).max(0) as f64 / 1e6
    }

    /// The stream was sent by this computer
    fn sent(&self, s: &Stream) -> bool {
        let ip = s.src.ip();
        if self.near == Some(ip) || self.near_media.contains(&ip) {
            true
        } else if self.near == Some(s.dst.ip()) || self.near_media.contains(&s.dst.ip()) {
            false
        } else {
            s.outgoing
        }
    }
}

/// The meaning of a status code of SIP that rejects a call (`SIPDE` of the draft)
fn meaning(t: Lang, code: u16) -> Option<String> {
    let id = match code {
        404 => "sip-meaning-404",
        480 => "sip-meaning-480",
        486 => "sip-meaning-486",
        487 => "sip-meaning-487",
        603 => "sip-meaning-603",
        _ => return None,
    };
    Some(t.get(id))
}

/// `486 Busy Here (besetzt)`
fn rejection(t: Lang, msg: &Msg) -> String {
    let code = msg.status.unwrap_or_default();
    let mut text = format!("{code} {}", msg.reason).trim_end().to_owned();
    if let Some(meaning) = meaning(t, code) {
        let _ = write!(text, " ({meaning})");
    }
    text
}

fn status(t: Lang, b: &Built, s: &Summary) -> Status {
    let sub = match s.state {
        State::Rejected => s
            .failed
            .map(|i| rejection(t, &b.msgs[i]))
            .unwrap_or_default(),
        State::Missed => t.get("call-sub-missed"),
        State::Cancelled => t.get("call-sub-cancelled"),
        State::Ended => {
            let by_us = s
                .end
                .is_some_and(|i| s.near.is_some() && b.msgs[i].src == s.near);
            t.get(if by_us {
                "call-sub-ended-us"
            } else {
                "call-sub-ended-them"
            })
        }
        State::Rtp => t.get("call-sub-rtp"),
        State::Running => s
            .answer
            .map(|a| t.get1("call-sub-running", "time", clock(a)[..5].to_owned()))
            .unwrap_or_default(),
        State::Ringing | State::Setup => String::new(),
    };
    Status {
        state: s.state,
        label: t.get(s.state.id()),
        sub,
    }
}

/// A codec by its payload type: short (`G.711 A-law`), long (`G.711 A-law (PCMA), 8 kHz`), and
/// whether openpacketmon can play it
struct Codec {
    short: String,
    long: String,
    known: bool,
    g711: bool,
}

fn codec(t: Lang, pt: u8, formats: &[(u8, String)]) -> Codec {
    let named = formats
        .iter()
        .find(|(f, name)| *f == pt && !name.is_empty())
        .map(|(_, name)| name.as_str());
    let (encoding, rate) = match named {
        Some(name) => {
            let mut parts = name.split('/');
            let encoding = parts.next().unwrap_or_default().to_owned();
            (encoding, parts.next().and_then(|r| r.parse::<u32>().ok()))
        }
        None => match pt {
            0 => ("PCMU".to_owned(), Some(8000)),
            8 => ("PCMA".to_owned(), Some(8000)),
            9 => ("G722".to_owned(), Some(8000)),
            _ => match rtp::static_type(pt) {
                Some((name, rate)) => (name.to_owned(), Some(rate)),
                None => {
                    return Codec {
                        short: t.get("call-codec-unknown"),
                        long: t.get1("call-codec-dynamic", "type", u16::from(pt)),
                        known: false,
                        g711: false,
                    };
                }
            },
        },
    };
    let upper = encoding.to_ascii_uppercase();
    let short = match upper.as_str() {
        "PCMA" => "G.711 A-law".to_owned(),
        "PCMU" => "G.711 µ-law".to_owned(),
        "G722" => "G.722".to_owned(),
        "OPUS" => "Opus".to_owned(),
        _ => encoding.clone(),
    };
    let khz = rate.map(|r| format!("{} kHz", format::decimal(t, f64::from(r) / 1000.0, 1)));
    let long = match (short != encoding, khz) {
        (true, Some(k)) => format!("{short} ({encoding}), {k}"),
        (true, None) => format!("{short} ({encoding})"),
        (false, Some(k)) => format!("{short}, {k}"),
        (false, None) => short.clone(),
    };
    Codec {
        g711: upper == "PCMA" || upper == "PCMU",
        short,
        long,
        known: true,
    }
}

/// The codecs a session description offers for voice, without events and comfort noise
fn offered(t: Lang, sdp: &[SdpMedia]) -> Vec<String> {
    let Some(audio) = sdp
        .iter()
        .find(|m| m.kind == "audio")
        .or_else(|| sdp.first())
    else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    for (pt, name) in &audio.formats {
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("telephone-event") || lower.starts_with("cn/") || *pt == 13 {
            continue;
        }
        let short = codec(t, *pt, &audio.formats).short;
        if !names.contains(&short) {
            names.push(short);
        }
    }
    names
}

/// Codecs as alternatives, the variants of G.711 together: `G.711 (A-law oder µ-law)`
fn either(t: Lang, names: &[String]) -> String {
    let or = format!(" {} ", t.get("call-or"));
    let g711: Vec<&str> = names
        .iter()
        .filter_map(|n| n.strip_prefix("G.711 "))
        .collect();
    let mut parts: Vec<String> = Vec::new();
    for name in names {
        if name.starts_with("G.711 ") && g711.len() > 1 {
            if parts.iter().all(|p| !p.starts_with("G.711")) {
                parts.push(format!("G.711 ({})", g711.join(&or)));
            }
        } else {
            parts.push(name.clone());
        }
    }
    parts.join(&or)
}

// ---- formatting ----------------------------------------------------------------------------------

/// A local time of day from microseconds since 1970 (`14:40:57`)
fn clock(us: i64) -> String {
    jiff::Timestamp::from_microsecond(us).map_or_else(
        |_| String::new(),
        |time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%H:%M:%S")
                .to_string()
        },
    )
}

/// A local time with milliseconds (`14:40:57,679`)
fn clock_ms(t: Lang, us: i64) -> String {
    format!(
        "{}{}{:03}",
        clock(us),
        t.decimal_separator(),
        us.rem_euclid(1_000_000) / 1000
    )
}

/// Seconds as `1:31`
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn mmss(seconds: f64) -> String {
    format::clock(seconds.max(0.0).round() as u64)
}

/// Seconds after the start of a call (`+8,902 s`)
#[allow(clippy::cast_precision_loss)]
fn after(t: Lang, us: i64) -> String {
    format!("+{} s", format::fixed(t, us.max(0) as f64 / 1e6, 3))
}

fn ssrc_text(ssrc: u32) -> String {
    format!("0x{ssrc:08x}")
}

/// The name of a host, if one is known
fn host_name(state: &EngineState, ip: IpAddr) -> Option<String> {
    let (rdns, host) = state.addresses_resolved.get(&ip)?;
    let name = if host.domain.is_empty() {
        rdns.clone()
    } else {
        host.domain.clone()
    };
    (!name.is_empty() && name != ip.to_string()).then_some(name)
}

/// The program of a connection: name and path
fn program_of(
    t: Lang,
    state: &EngineState,
    conn: Option<&AddressPortPair>,
) -> Option<(String, String)> {
    let info = state.info_traffic.map.get(conn?)?;
    match &info.program {
        Program::NamePath((_, path)) => Some((program_label(t, &info.program), path.clone())),
        _ => None,
    }
}

// ---- the list ----------------------------------------------------------------------------------

/// What both pages need to know of a call
struct Described {
    key: CallKey,
    status: Status,
    start: i64,
    talk: f64,
    answered: bool,
    outgoing: bool,
    from: Party,
    to: Party,
    label: String,
    codec: String,
    codec_title: String,
    rate: Option<Rate>,
    rate_title: String,
    packets: u64,
    playable: bool,
    wait: f64,
    /// For "Auffällig": the bad stream, its loss and jitter
    bad: Option<(bool, f64, Option<f64>)>,
}

/// The stream that carries most of the call, and its codec
fn main_stream(b: &Built) -> Option<&Stream> {
    b.streams.iter().max_by_key(|s| s.packets)
}

fn worst(b: &Built) -> Option<&Stream> {
    b.streams.iter().max_by_key(|s| (s.rate(), s.lost()))
}

/// Whether the streams of a call can be played (G.711, not encrypted, whole packets)
fn playable(calls: &Calls, b: &Built, headers_only: bool, t: Lang) -> bool {
    !headers_only
        && main_stream(b).is_some_and(|s| {
            let facts = calls.media.get(&s.media);
            let formats = facts.map_or(&[][..], |f| &f.formats[..]);
            !facts.is_some_and(|f| f.srtp) && codec(t, s.payload_type, formats).g711
        })
}

#[allow(clippy::too_many_lines)]
fn describe(
    t: Lang,
    state: &EngineState,
    calls: &Calls,
    key: CallKey,
    b: &Built,
    now: i64,
    headers_only: bool,
) -> Described {
    let main = main_stream(b);
    let formats = |s: &Stream| {
        calls
            .media
            .get(&s.media)
            .map_or_else(|| Arc::from([]), |f| f.formats.clone())
    };
    let (codec_text, codec_title, known) = match main {
        Some(s) => {
            let c = codec(t, s.payload_type, &formats(s));
            let title = if c.known {
                String::new()
            } else {
                t.get1(
                    "call-codec-unknown-title",
                    "type",
                    u16::from(s.payload_type),
                )
            };
            (c.short, title, c.known)
        }
        None => (String::new(), String::new(), true),
    };
    let worst = worst(b);
    let rate = worst.map(Stream::rate);
    let jitter_known = b.streams.iter().all(|s| s.clock.is_some());
    let rate_title = if rate.is_some() && !jitter_known {
        t.get("rate-loss-only")
    } else {
        String::new()
    };
    let bad = worst.filter(|s| s.rate() == Rate::Bad).map(|s| {
        let sent = match key {
            CallKey::Sip(_) => Summary::read(b, now, &calls.registrants).sent(s),
            CallKey::Rtp(_) => Some(s.src) == rtp_near(b, &calls.registrants),
        };
        (sent, s.loss(), s.jitter_ms().map(|(mean, _)| mean))
    });
    let packets = b.sip + b.rtp + b.rtcp;
    let this = t.get("call-this-computer");
    match key {
        CallKey::Sip(_) => {
            let s = Summary::read(b, now, &calls.registrants);
            let first = s.invite.map(|i| &b.msgs[i]).or(b.msgs.first());
            let (from_name, from_uri) = first.map(|m| split_party(&m.from)).unwrap_or_default();
            let (to_name, to_uri) = first.map(|m| split_party(&m.to)).unwrap_or_default();
            let name = |own: String, uri: &str| {
                if own.is_empty() {
                    calls.names.get(uri).cloned().unwrap_or_default()
                } else {
                    own
                }
            };
            let party = |name: String, uri: &str, here: bool| {
                let anonymous = is_anonymous(uri);
                let sub = if here {
                    this.clone()
                } else if anonymous {
                    t.get("call-hidden-number")
                } else if name.is_empty() {
                    String::new()
                } else {
                    uri_text(uri)
                };
                Party {
                    strong: !name.is_empty(),
                    name: if name.is_empty() { uri_text(uri) } else { name },
                    mono: false,
                    sub,
                    title: uri_text(uri),
                }
            };
            let from = party(name(from_name, &from_uri), &from_uri, s.outgoing);
            let to = party(name(to_name, &to_uri), &to_uri, !s.outgoing);
            Described {
                key,
                status: status(t, b, &s),
                start: s.start,
                talk: s.talk(),
                answered: s.ok.is_some(),
                outgoing: s.outgoing,
                label: format!("{} → {}", from.name, to.name),
                from,
                to,
                codec: if main.is_some() {
                    codec_text
                } else {
                    String::new()
                },
                codec_title,
                rate,
                rate_title,
                packets,
                playable: playable(calls, b, headers_only, t),
                wait: s.wait(),
                bad,
            }
        }
        CallKey::Rtp(_) => {
            let near = rtp_near(b, &calls.registrants);
            let far = rtp_far(b, near);
            let program = program_of(t, state, b.conn.as_ref().or(b.any_conn.as_ref()));
            let far_name = far.and_then(|f| host_name(state, f.ip()));
            let start = b.first.map_or(0, |(_, time)| time);
            #[allow(clippy::cast_precision_loss)]
            let talk = (b.last_time - start).max(0) as f64 / 1e6;
            let from = Party {
                name: near.map(|n| n.ip().to_string()).unwrap_or_default(),
                strong: false,
                mono: true,
                sub: match &program {
                    Some((name, _)) => format!("{this} · {name}"),
                    None => this.clone(),
                },
                title: String::new(),
            };
            let to = match (&far_name, far) {
                (Some(name), Some(f)) => Party {
                    name: name.clone(),
                    strong: false,
                    mono: false,
                    sub: f.to_string(),
                    title: String::new(),
                },
                (_, far) => Party {
                    name: far.map(|f| f.to_string()).unwrap_or_default(),
                    strong: false,
                    mono: true,
                    sub: String::new(),
                    title: String::new(),
                },
            };
            let host =
                far_name.unwrap_or_else(|| far.map(|f| f.ip().to_string()).unwrap_or_default());
            Described {
                key,
                status: Status {
                    state: State::Rtp,
                    label: t.get(State::Rtp.id()),
                    sub: t.get("call-sub-rtp"),
                },
                start,
                talk,
                answered: false,
                outgoing: near.is_some() && b.streams.first().map(|s| s.src) == near,
                label: t.get1("call-label-rtp", "host", host),
                from,
                to,
                codec: if known {
                    codec_text
                } else {
                    t.get("call-codec-unknown")
                },
                codec_title,
                rate,
                rate_title,
                packets,
                playable: known && playable(calls, b, headers_only, t),
                wait: 0.0,
                bad,
            }
        }
    }
}

/// The end of this computer in RTP without SIP: the source of what it sent
/// This side of a call without SIP: a phone that registered, else the sender of local packets
fn rtp_near(b: &Built, registrants: &HashSet<IpAddr>) -> Option<SocketAddr> {
    let registered = |a: &SocketAddr| registrants.contains(&a.ip());
    b.streams
        .iter()
        .find_map(|s| {
            if registered(&s.src) {
                Some(s.src)
            } else if registered(&s.dst) {
                Some(s.dst)
            } else {
                None
            }
        })
        .or_else(|| b.streams.iter().find(|s| s.outgoing).map(|s| s.src))
        .or_else(|| b.streams.first().map(|s| s.dst))
}

fn rtp_far(b: &Built, near: Option<SocketAddr>) -> Option<SocketAddr> {
    b.streams.iter().find_map(|s| {
        if Some(s.src) == near {
            Some(s.dst)
        } else if Some(s.dst) == near {
            Some(s.src)
        } else {
            None
        }
    })
}

fn row(t: Lang, d: &Described) -> CallRow {
    let rtp = d.key_is_rtp();
    let (duration, duration_sub) = if d.answered || rtp {
        let sub = if d.status.state == State::Running {
            t.get("call-dur-running")
        } else {
            String::new()
        };
        (mmss(d.talk), sub)
    } else {
        let seconds = format::fixed(t, d.wait, 1);
        let sub = if d.status.state == State::Missed {
            t.get1("call-dur-rang", "seconds", seconds)
        } else {
            t.get1("call-dur-after", "seconds", seconds)
        };
        ("–".to_owned(), sub)
    };
    CallRow {
        key: d.key,
        status: d.status.clone(),
        start: clock(d.start),
        duration,
        duration_sub,
        outgoing: d.outgoing,
        direction: t.get(if d.outgoing { "call-out" } else { "call-in" }),
        from: d.from.clone(),
        to: d.to.clone(),
        codec: d.codec.clone(),
        codec_title: d.codec_title.clone(),
        rate: d.rate,
        rate_title: d.rate_title.clone(),
        packets: format::count(t, u128::from(d.packets)),
        playable: d.playable,
    }
}

impl Described {
    fn key_is_rtp(&self) -> bool {
        matches!(self.key, CallKey::Rtp(_))
    }
}

/// All calls, newest first
fn described(t: Lang, engine: &Engine) -> (Scan, Calls, Vec<Described>) {
    let scan = scan(t, engine);
    let calls = build(&scan, None);
    let state = engine.lock();
    let mut list: Vec<Described> = calls
        .built
        .iter()
        .map(|(key, b)| describe(t, &state, &calls, *key, b, scan.now, scan.headers_only))
        .collect();
    drop(state);
    list.sort_by_key(|d| std::cmp::Reverse(d.start));
    (scan, calls, list)
}

/// The list of calls (`pageCalls`)
#[allow(clippy::too_many_lines)]
pub fn call_list(t: Lang, engine: &Engine) -> CallList {
    let (scan, calls, list) = described(t, engine);
    let sip: Vec<&Described> = list.iter().filter(|d| !d.key_is_rtp()).collect();
    let running = sip
        .iter()
        .filter(|d| d.status.state == State::Running)
        .count();
    let answered: Vec<&&Described> = sip.iter().filter(|d| d.answered).collect();
    let failed: Vec<&&Described> = sip
        .iter()
        .filter(|d| {
            !d.answered
                && !matches!(
                    d.status.state,
                    State::Ringing | State::Setup | State::Running
                )
        })
        .collect();
    let mut failed_names: Vec<String> = Vec::new();
    for d in &failed {
        let name = d.status.label.to_lowercase();
        if !failed_names.contains(&name) {
            failed_names.push(name);
        }
    }
    let bad = list.iter().filter(|d| d.rate == Some(Rate::Bad)).count();
    let rtp_only = list.iter().filter(|d| d.key_is_rtp()).count();
    let talk: f64 = answered.iter().map(|d| d.talk).sum();
    let count = |n: usize| format::count(t, n as u128);
    let tiles = vec![
        CallTile {
            icon: Icon::Phone,
            label: t.get("calls-tile-calls"),
            value: count(sip.len()),
            sub: if running > 0 {
                t.get1("calls-tile-running", "count", running)
            } else {
                t.get("calls-tile-none-running")
            },
            warn: false,
        },
        CallTile {
            icon: Icon::Check,
            label: t.get("calls-tile-answered"),
            value: count(answered.len()),
            sub: t.get1("calls-tile-talk", "time", mmss(talk)),
            warn: false,
        },
        CallTile {
            icon: Icon::Cross,
            label: t.get("calls-tile-failed"),
            value: count(failed.len()),
            sub: if failed_names.is_empty() {
                "–".to_owned()
            } else {
                failed_names.join(", ")
            },
            warn: false,
        },
        CallTile {
            icon: Icon::Alert,
            label: t.get("calls-tile-bad"),
            value: count(bad),
            sub: t.get("calls-tile-bad-sub"),
            warn: bad > 0,
        },
        CallTile {
            icon: Icon::Lock,
            label: t.get("calls-tile-rtp"),
            value: count(rtp_only),
            sub: t.get("calls-tile-rtp-sub"),
            warn: false,
        },
    ];
    let registration = calls.registration.as_ref().map(|(request, ok)| {
        let (_, account) = split_party(&ok.to);
        let valid = ok.expires.or(request.expires).unwrap_or(3600);
        Registration {
            account: uri_text(&account),
            text: t.fmt(
                "calls-registered-at",
                &[
                    ("registrar", uri_host(&request.uri).into()),
                    ("time", clock(ok.time).into()),
                    ("valid", validity(t, valid).into()),
                ],
            ),
            packet: ok.number,
            digest: if request.auth.is_some() {
                t.get("calls-registered-digest")
            } else {
                String::new()
            },
        }
    });
    CallList {
        live: scan.live,
        unavailable: scan.unavailable,
        tiles,
        rows: list.iter().map(|d| row(t, d)).collect(),
        registration,
    }
}

/// How long a registration holds (`1 Stunde`)
fn validity(t: Lang, seconds: u64) -> String {
    if seconds >= 3600 && seconds.is_multiple_of(3600) {
        t.get1("calls-valid-hours", "count", seconds / 3600)
    } else if seconds >= 60 && seconds.is_multiple_of(60) {
        t.get1("calls-valid-minutes", "count", seconds / 60)
    } else {
        t.get1("calls-valid-seconds", "count", seconds)
    }
}

/// The number of SIP calls with stored packets, for the tab of the statistics
pub fn count(engine: &Engine) -> usize {
    let Some(store) = engine.lock().packets.clone() else {
        return 0;
    };
    let first = store.lock().iter().next().map_or(u64::MAX, |p| p.number);
    let analysis = store.analysis();
    let voip = analysis.voip();
    let mut calls = HashSet::new();
    for (_, r) in analysis.after(first.saturating_sub(1)) {
        match r.voip {
            Some(Mark::Sip {
                call: Some(call), ..
            }) => {
                calls.insert(call);
            }
            Some(Mark::Media { media, .. }) => {
                if let Some(call) = voip.media(media).and_then(|m| m.call) {
                    calls.insert(call);
                }
            }
            _ => {}
        }
    }
    calls.len()
}

/// The call a packet belongs to (not a registration), from the analysis
pub fn key_of(analysis: &analysis::Analysis, number: u64) -> Option<CallKey> {
    let voip = analysis.voip();
    match analysis.get(number)?.voip? {
        Mark::Sip {
            call: Some(call), ..
        } => Some(CallKey::Sip(call)),
        Mark::Media { media, .. } => Some(match voip.media(media)?.call {
            Some(call) => CallKey::Sip(call),
            None => CallKey::Rtp(voip.partner(media).map_or(media, |p| p.min(media))),
        }),
        _ => None,
    }
}

/// The call a packet belongs to, and its name (`Anna Beispiel → support@example.com`); not for a
/// registration
pub fn call_of(t: Lang, engine: &Engine, number: u64) -> Option<(CallKey, String)> {
    let store = engine.lock().packets.clone()?;
    let (key, start) = {
        let analysis = store.analysis();
        let key = key_of(&analysis, number)?;
        let start = match key {
            CallKey::Sip(call) => analysis.voip().start(call),
            CallKey::Rtp(_) => None,
        };
        (key, start)
    };
    // the names from the first message of the call, while it is kept
    let first = start.and_then(|start| {
        let packets = store.lock();
        Some((packets.get(start)?.data.clone(), packets.link_type()))
    });
    if let Some((data, link_type)) = first {
        let d = dissect::dissect(&data, link_type);
        let text = |key: &str| {
            d.values.iter().find_map(|(k, v)| match v {
                Value::Text(t) if *k == key => Some(t.clone()),
                _ => None,
            })
        };
        let side = |value: Option<String>| {
            let (name, uri) = split_party(&value.unwrap_or_default());
            if name.is_empty() {
                uri_text(&uri)
            } else {
                name
            }
        };
        if let (Some(from), Some(to)) = (text("sip.from"), text("sip.to")) {
            return Some((key, format!("{} → {}", side(Some(from)), side(Some(to)))));
        }
    }
    let page = call_page(t, engine, key)?;
    let label = if page.sip {
        format!("{} → {}", page.from, page.to)
    } else {
        t.get1("call-label-rtp", "host", page.to)
    };
    Some((key, label))
}

/// Calls with a bad connection, for "Auffällig" on the overview
pub fn bad_calls(t: Lang, engine: &Engine) -> Vec<BadCall> {
    let (_, _, list) = described(t, engine);
    list.iter()
        .filter_map(|d| {
            let (sent, loss, jitter) = d.bad?;
            let side = t.get(if sent {
                "attn-call-sent"
            } else {
                "attn-call-received"
            });
            let mut text = t.fmt(
                "attn-call-text",
                &[
                    ("call", d.label.clone().into()),
                    ("time", clock(d.start)[..5].to_owned().into()),
                    ("loss", format::fixed(t, loss, 1).into()),
                    ("side", side.into()),
                ],
            );
            if let Some(jitter) = jitter {
                text.push_str(&t.get1("attn-call-jitter", "jitter", format::fixed(t, jitter, 1)));
            }
            Some(BadCall { key: d.key, text })
        })
        .collect()
}

// ---- the page of a call ------------------------------------------------------------------------

/// The page of a call (`pageCall`); `None` if none of its packets are stored
#[allow(clippy::too_many_lines)]
pub fn call_page(t: Lang, engine: &Engine, key: CallKey) -> Option<CallPage> {
    let scan = scan(t, engine);
    let calls = build(&scan, Some(key));
    let b = calls.built.get(&key)?;
    let state = engine.lock();
    let d = describe(t, &state, &calls, key, b, scan.now, scan.headers_only);
    let sip = matches!(key, CallKey::Sip(_));
    let summary = sip.then(|| Summary::read(b, scan.now, &calls.registrants));
    let sent_of = |s: &Stream| match &summary {
        Some(summary) => summary.sent(s),
        None => Some(s.src) == rtp_near(b, &calls.registrants),
    };
    let mut streams: Vec<&Stream> = b.streams.iter().collect();
    streams.sort_by_key(|s| (!sent_of(s), s.first));
    let filter = match key {
        CallKey::Sip(_) => {
            let id = b
                .msgs
                .first()
                .map(|m| m.call_id.clone())
                .unwrap_or_default();
            format!("voip.call_id == \"{}\"", id.replace('"', "\\\""))
        }
        CallKey::Rtp(_) => ssrc_filter(&streams),
    };
    let formats_of = |s: &Stream| {
        calls
            .media
            .get(&s.media)
            .map_or_else(|| Arc::from([]), |f| f.formats.clone())
    };
    let main = main_stream(b);
    let main_codec = main.map(|s| codec(t, s.payload_type, &formats_of(s)));

    // the far ends: the SIP server, and where the voice went
    let near_ip = match &summary {
        Some(s) => s.near,
        None => rtp_near(b, &calls.registrants).map(|n| n.ip()),
    };
    let sip_peer = summary.as_ref().and_then(|s| {
        let m = &b.msgs[s.invite.unwrap_or(0)];
        if s.outgoing { m.dst } else { m.src }
    });
    let media_peer = streams
        .iter()
        .map(|s| if sent_of(s) { s.dst.ip() } else { s.src.ip() })
        .next()
        .or_else(|| {
            b.msgs
                .iter()
                .filter(|m| m.src != near_ip)
                .flat_map(|m| m.sdp.iter().filter_map(|s| s.addr))
                .next()
        });
    let name_or_ip = |ip: IpAddr| host_name(&state, ip).unwrap_or_else(|| ip.to_string());
    let program = program_of(
        t,
        &state,
        match &summary {
            Some(s) => s
                .invite
                .and_then(|i| b.msgs[i].conn.as_ref())
                .or(b.conn.as_ref()),
            None => b.conn.as_ref().or(b.any_conn.as_ref()),
        },
    );

    let mut facts = Vec::new();
    let fact = |label: &str, parts: Vec<Part>| Fact {
        label: t.get(label),
        parts,
    };
    let packets_parts = |kinds: String| {
        vec![
            Part::Mono(format::count(t, u128::from(d.packets))),
            Part::Muted(format!(
                "{kinds} · {}",
                format::bytes(t, u128::from(b.bytes))
            )),
            Part::Packets(t.get("call-show"), filter.clone()),
        ]
    };
    if let Some(s) = &summary {
        let first = &b.msgs[s.invite.unwrap_or(0)];
        facts.push(fact("call-f-state", vec![Part::State(d.status.clone())]));
        let mut from = vec![Part::Text(first.from.clone())];
        if !s.outgoing && is_anonymous(&split_party(&first.from).1) {
            from.push(Part::Muted(t.get("call-hidden-number")));
        }
        facts.push(fact("call-f-from", from));
        facts.push(fact(
            "call-f-to",
            vec![Part::Text(format!("<{}>", split_party(&first.to).1))],
        ));
        facts.push(fact(
            "call-f-direction",
            vec![
                Part::Dir(s.outgoing),
                Part::Text(t.get(if s.outgoing { "call-out" } else { "call-in" })),
                Part::Muted(t.get(if s.outgoing {
                    "call-from-here"
                } else {
                    "call-to-here"
                })),
            ],
        ));
        let mut start = vec![Part::Mono(clock_ms(t, s.start))];
        if let Some(i) = s.invite {
            start.push(Part::Muted(t.get("call-invite-in")));
            start.push(Part::Packet(b.msgs[i].number));
        }
        facts.push(fact("call-f-start", start));
        let mut wait = vec![Part::Mono(format!("{} s", format::fixed(t, s.wait(), 1)))];
        if let Some(i) = s.ring {
            wait.push(Part::Muted(t.get("call-rings-from")));
            wait.push(Part::Packet(b.msgs[i].number));
        }
        let wait_label = if s.ok.is_some() {
            "call-f-wait-answer"
        } else if s.state == State::Missed {
            "call-f-rang"
        } else {
            "call-f-wait"
        };
        facts.push(fact(wait_label, wait));
        if s.ok.is_some() {
            let mut talk = vec![Part::Mono(mmss(s.talk()))];
            if s.state == State::Running {
                talk.push(Part::Running(t.get("call-dur-running")));
            }
            facts.push(fact("call-f-talk", talk));
        }
        if let Some(i) = s.end {
            facts.push(fact(
                "call-f-end",
                vec![
                    Part::Mono(clock_ms(t, b.msgs[i].time)),
                    Part::Muted(t.get1("call-end-in", "what", d.status.sub.clone())),
                    Part::Packet(b.msgs[i].number),
                ],
            ));
        }
        match (&main_codec, main) {
            (Some(c), Some(stream)) => {
                // negotiated: the last session description before the voice
                let setup = b
                    .msgs
                    .iter()
                    .rev()
                    .find(|m| !m.sdp.is_empty() && m.number < stream.first)
                    .map(|m| m.number)
                    .or_else(|| calls.media.get(&stream.media).and_then(|f| f.setup));
                let mut parts = vec![Part::Text(c.long.clone())];
                if let Some(n) = setup {
                    parts.push(Part::Muted(t.get("call-codec-setup")));
                    parts.push(Part::Packet(n));
                }
                facts.push(fact("call-f-codec", parts));
            }
            _ => {
                if let Some(offer) = b.msgs.iter().find(|m| !m.sdp.is_empty()) {
                    let names = offered(t, &offer.sdp);
                    if !names.is_empty() {
                        facts.push(fact(
                            "call-f-offered",
                            vec![
                                Part::Text(names.join(", ")),
                                Part::Muted(t.get("call-offer-sdp")),
                                Part::Packet(offer.number),
                            ],
                        ));
                    }
                }
            }
        }
        if let Some((name, path)) = &program {
            facts.push(fact(
                "call-f-program",
                vec![Part::Strong(name.clone()), Part::SmallMono(path.clone())],
            ));
        }
        if let Some(peer) = sip_peer {
            let mut parts = match host_name(&state, peer) {
                Some(name) => vec![Part::Text(name), Part::SmallMono(peer.to_string())],
                None => vec![Part::Mono(peer.to_string())],
            };
            if let Some(media) = media_peer.filter(|m| *m != peer) {
                parts.push(Part::Muted(t.get1(
                    "call-media-via",
                    "host",
                    name_or_ip(media),
                )));
            }
            facts.push(fact("call-f-provider", parts));
        }
        facts.push(fact(
            "call-f-callid",
            vec![Part::Mono(first.call_id.clone())],
        ));
        if s.outgoing
            && let Some(auth) = b
                .msgs
                .iter()
                .find(|m| m.is("INVITE") && m.src == s.near && m.auth.is_some())
        {
            let challenge = b
                .msgs
                .iter()
                .find(|m| m.answers("INVITE") && matches!(m.status, Some(401 | 407)));
            let mut parts = vec![
                Part::Text(t.get("call-auth")),
                Part::SmallMono(auth.auth.clone().unwrap_or_default()),
            ];
            match challenge {
                Some(c) => {
                    parts.push(Part::Muted(t.get1(
                        "call-auth-after",
                        "code",
                        c.status.unwrap_or_default(),
                    )));
                    parts.push(Part::Packet(c.number));
                    parts.push(Part::Muted(t.get("call-auth-mcp")));
                }
                None => parts.push(Part::Muted(t.get("call-auth-again"))),
            }
            facts.push(fact("call-f-auth", parts));
        }
        let kinds = if b.rtp + b.rtcp > 0 {
            t.fmt(
                "call-packets-kinds",
                &[
                    ("sip", format::count(t, u128::from(b.sip)).into()),
                    ("rtp", format::count(t, u128::from(b.rtp)).into()),
                    ("rtcp", format::count(t, u128::from(b.rtcp)).into()),
                ],
            )
        } else {
            "SIP".to_owned()
        };
        facts.push(fact("call-f-packets", packets_parts(kinds)));
    } else {
        let near = rtp_near(b, &calls.registrants);
        let far = rtp_far(b, near);
        facts.push(fact("call-f-state", vec![Part::State(d.status.clone())]));
        let mut ends = vec![Part::Mono(format!(
            "{} ↔ {}",
            near.map(|n| n.to_string()).unwrap_or_default(),
            far.map(|f| f.to_string()).unwrap_or_default()
        ))];
        if let Some(name) = far.and_then(|f| host_name(&state, f.ip())) {
            ends.push(Part::Muted(name));
        }
        facts.push(fact("call-f-ends", ends));
        if let Some((name, path)) = &program {
            facts.push(fact(
                "call-f-program",
                vec![Part::Strong(name.clone()), Part::SmallMono(path.clone())],
            ));
        }
        facts.push(fact(
            "call-f-found",
            vec![Part::Text(t.get("call-found-form"))],
        ));
        if let Some(c) = &main_codec {
            facts.push(fact("call-f-codec", vec![Part::Text(c.long.clone())]));
            if !c.known {
                facts.push(fact(
                    "call-f-content",
                    vec![Part::Text(t.get("call-content-unknown"))],
                ));
            }
        }
        facts.push(fact("call-f-start", vec![Part::Mono(clock_ms(t, d.start))]));
        facts.push(fact("call-f-duration", vec![Part::Mono(mmss(d.talk))]));
        facts.push(fact("call-f-packets", packets_parts("RTP".to_owned())));
    }

    // the flow
    let start = d.start;
    let band = |first: u64| {
        let sent: u64 = streams
            .iter()
            .filter(|s| sent_of(s))
            .map(|s| s.packets)
            .sum();
        let received: u64 = streams
            .iter()
            .filter(|s| !sent_of(s))
            .map(|s| s.packets)
            .sum();
        let locked = main_codec.as_ref().is_some_and(|c| !c.known)
            || streams
                .iter()
                .any(|s| calls.media.get(&s.media).is_some_and(|f| f.srtp));
        let lead = match (&main_codec, main) {
            (Some(c), _) if c.known => c.short.clone(),
            (_, Some(s)) => t.get1("call-band-type", "type", u16::from(s.payload_type)),
            _ => String::new(),
        };
        let mut text = t.fmt(
            "call-band",
            &[
                ("sent", format::count(t, u128::from(sent)).into()),
                ("received", format::count(t, u128::from(received)).into()),
                ("time", mmss(d.talk).into()),
            ],
        );
        if b.rtcp > 0 {
            text.push_str(" · ");
            text.push_str(&t.get1("call-band-rtcp", "count", b.rtcp));
        }
        if d.status.state == State::Running {
            text.push_str(" · ");
            text.push_str(&t.get("call-dur-running"));
        }
        let first_time = streams.iter().map(|s| s.first_time).min().unwrap_or(start);
        FlowRow::Media {
            number: first,
            at: after(t, first_time - start),
            lead,
            text,
            locked,
            filter: ssrc_filter(&streams),
        }
    };
    let here_small = |ip: Option<IpAddr>| {
        let ip = ip.map(|i| i.to_string()).unwrap_or_default();
        match &program {
            Some((name, _)) => format!("{ip} · {name}"),
            None => ip,
        }
    };
    let mut flow = Flow::default();
    let first_rtp = streams.iter().map(|s| s.first).min();
    if let Some(s) = &summary {
        flow.hint = t.get("call-flow-hint");
        flow.lanes
            .push((t.get("call-lane-here"), here_small(s.near)));
        // without a name the address heads the lane, and only the role stands below
        let lane = |ip: IpAddr, role: &str| match host_name(&state, ip) {
            Some(name) => (name, format!("{ip} · {role}")),
            None => (ip.to_string(), role.to_owned()),
        };
        if let Some(peer) = sip_peer {
            flow.lanes.push(lane(peer, "SIP"));
        }
        if let Some(media) = media_peer.filter(|m| Some(*m) != sip_peer) {
            flow.lanes.push(lane(media, "RTP"));
        }
        let mut banded = false;
        for m in &b.msgs {
            if let Some(first) = first_rtp
                && !banded
                && m.number > first
            {
                flow.rows.push(band(first));
                banded = true;
            }
            let failed = m.status.is_some_and(|c| c >= 400 && c != 401 && c != 407);
            flow.rows.push(FlowRow::Message {
                number: m.number,
                at: after(t, m.time - start),
                clock: clock_ms(t, m.time),
                label: m.label(),
                right: s.near.is_some() && m.src == s.near,
                bad: failed,
                ok: m.answers("INVITE") && m.status == Some(200),
            });
        }
        if let (Some(first), false) = (first_rtp, banded) {
            flow.rows.push(band(first));
        }
    } else {
        let far = rtp_far(b, rtp_near(b, &calls.registrants));
        flow.hint = t.get("call-flow-hint-rtp");
        flow.lanes
            .push((t.get("call-lane-here"), here_small(near_ip)));
        flow.lanes.push((t.get("call-lane-nosip"), "–".to_owned()));
        if let Some(far) = far.map(|f| f.ip()) {
            flow.lanes.push(match host_name(&state, far) {
                Some(name) => (name, far.to_string()),
                None => (far.to_string(), "RTP".to_owned()),
            });
        }
        if let Some(first) = first_rtp {
            flow.rows.push(band(first));
        }
    }

    // the streams
    let rows: Vec<StreamRow> = streams
        .iter()
        .map(|s| {
            let sent = sent_of(s);
            let loss = s.loss();
            let missing = s.lost();
            let jitter_known = s.clock.is_some();
            StreamRow {
                sent,
                direction: t.get(if sent { "call-sent" } else { "call-received" }),
                path: format!("{} → {}", s.src, s.dst),
                ssrc: ssrc_text(s.ssrc),
                filter: format!("rtp.ssrc == {}", ssrc_text(s.ssrc)),
                packets: format::count(t, u128::from(s.packets)),
                lost: (missing > 0).then(|| {
                    (
                        format::count(t, u128::from(missing)),
                        format!("({} %)", format::fixed(t, loss, 1)),
                    )
                }),
                jitter: s.jitter_ms().map(|(mean, max)| {
                    format!(
                        "{} / {} ms",
                        format::fixed(t, mean, 1),
                        format::fixed(t, max, 1)
                    )
                }),
                #[allow(clippy::cast_precision_loss)]
                gap: format!("{} ms", format::fixed(t, s.gap as f64 / 1000.0, 1)),
                late: format::count(t, u128::from(s.late)),
                rate: Some(s.rate()),
                rate_title: if jitter_known {
                    String::new()
                } else {
                    t.get("rate-from-loss")
                },
            }
        })
        .collect();
    let no_streams = rows.is_empty().then(|| {
        let text = match &summary {
            Some(s) if s.ok.is_none() => {
                let names = b
                    .msgs
                    .iter()
                    .find(|m| !m.sdp.is_empty())
                    .map(|m| offered(t, &m.sdp))
                    .unwrap_or_default();
                if names.is_empty() {
                    t.get("call-no-stream-text-plain")
                } else {
                    t.get1("call-no-stream-text", "codecs", either(t, &names))
                }
            }
            _ => t.get("call-no-stream-missing"),
        };
        (t.get("call-no-stream"), text)
    });
    let rtcp = if b.rtcp == 0 {
        t.get("call-rtcp-none")
    } else {
        let mut text = t.get1("call-rtcp", "count", b.rtcp);
        // what the far end says about the stream sent from here
        let sent = streams.iter().find(|s| sent_of(s));
        let report = sent.and_then(|s| {
            b.reports
                .iter()
                .rev()
                .find(|(from, r)| r.about == s.ssrc && Some(*from) != near_ip)
                .map(|(_, r)| (s, *r))
        });
        if let Some((s, r)) = report {
            let loss = f64::from(r.fraction) * 100.0 / 256.0;
            let jitter = s.clock.map(|c| f64::from(r.jitter) * 1000.0 / f64::from(c));
            text.push(' ');
            text.push_str(&match jitter {
                Some(j) => t.fmt(
                    "call-rtcp-peer",
                    &[
                        ("loss", format::fixed(t, loss, 1).into()),
                        ("jitter", format::fixed(t, j, 1).into()),
                    ],
                ),
                None => t.get1("call-rtcp-peer-loss", "loss", format::fixed(t, loss, 1)),
            });
        }
        text
    };

    // the notice on a bad connection
    let notice = d.bad.map(|(sent, loss, jitter)| {
        let lead = t.get(if sent { "call-bad-out" } else { "call-bad-in" });
        let mut text = t.get1("call-bad-loss", "loss", format::fixed(t, loss, 1));
        if let Some(j) = jitter {
            text.push_str(&t.get1("call-bad-jitter", "jitter", format::fixed(t, j, 1)));
        }
        text.push_str(&t.get("call-bad-cause"));
        (lead, text)
    });

    let listen = listen(
        t,
        &d,
        summary.as_ref(),
        b,
        main_codec.as_ref(),
        scan.headers_only,
        &calls,
    );
    let (from, to) = (d.from.name.clone(), d.to.name.clone());
    Some(CallPage {
        live: scan.live && (!sip || d.status.state == State::Running),
        sip,
        from,
        to,
        status: d.status.clone(),
        filter,
        packets_button: t.get(if sip {
            "call-packets"
        } else {
            "call-packets-stream"
        }),
        notice,
        card: t.get(if sip { "call-card" } else { "call-card-stream" }),
        facts,
        listen,
        flow,
        streams: rows,
        no_streams,
        rtcp,
    })
}

/// The display filter of the packets of these streams
fn ssrc_filter(streams: &[&Stream]) -> String {
    let mut ssrcs: Vec<String> = Vec::new();
    for s in streams {
        let text = ssrc_text(s.ssrc);
        if !ssrcs.contains(&text) {
            ssrcs.push(text);
        }
    }
    match ssrcs.as_slice() {
        [one] => format!("rtp.ssrc == {one}"),
        _ => format!("rtp.ssrc in {{{}}}", ssrcs.join(" ")),
    }
}

/// What the card "Anhören" shows
fn listen(
    t: Lang,
    d: &Described,
    summary: Option<&Summary>,
    b: &Built,
    main_codec: Option<&Codec>,
    headers_only: bool,
    calls: &Calls,
) -> Listen {
    let off = |lock: bool, heading: &str, text: String| Listen::Off {
        lock,
        heading: t.get(heading),
        text,
    };
    if d.playable {
        let codec = main_codec.map(|c| c.short.clone()).unwrap_or_default();
        return Listen::Ready {
            hint: t.get1("call-listen-hint", "codec", codec),
        };
    }
    if b.streams.is_empty() {
        return match summary {
            Some(s) if matches!(s.state, State::Missed | State::Cancelled) => {
                off(false, "call-play-none", t.get("call-play-missed"))
            }
            Some(s) if s.state == State::Rejected => off(
                false,
                "call-play-none",
                t.get1(
                    "call-play-rejected",
                    "reason",
                    s.failed
                        .map(|i| rejection(t, &b.msgs[i]))
                        .unwrap_or_default(),
                ),
            ),
            Some(s) if matches!(s.state, State::Ringing | State::Setup) => {
                off(false, "call-play-not-yet", t.get("call-play-waiting"))
            }
            _ => off(false, "call-no-stream", t.get("call-play-missing")),
        };
    }
    let srtp = b
        .streams
        .iter()
        .any(|s| calls.media.get(&s.media).is_some_and(|f| f.srtp));
    if summary.is_none() && main_codec.is_some_and(|c| !c.known) {
        return off(true, "call-play-locked", t.get("call-play-locked-text"));
    }
    if srtp {
        return off(true, "call-play-locked", t.get("call-play-srtp"));
    }
    if headers_only {
        return off(false, "call-play-off", t.get("call-play-headers"));
    }
    let codec = main_codec.map(|c| c.short.clone()).unwrap_or_default();
    off(
        false,
        "call-play-off",
        t.get1("call-play-codec", "codec", codec),
    )
}

pub mod audio;

#[cfg(test)]
pub(crate) mod tests;
