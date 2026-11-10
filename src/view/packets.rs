//! The packets kept of the capture, as the interface shows them: one long list with a display
//! filter (only the rows in view are read, in blocks, like in a packet analyzer), the details of one
//! packet with its layers and a hex dump, and the export as a PCAP file.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::net::IpAddr;
use std::ops::Range;

use sniffnet_packet_parser::{LinkType, Protocol};

use crate::dissect::{self, Dissection, Field, Layer};
use crate::engine::Engine;
use crate::engine::analysis::voip::Mark;
use crate::engine::analysis::{self, Note};
use crate::engine::packets::{PacketStore, SharedPacketStore, StoredPacket};
use crate::engine::state::EngineState;
use crate::filter::{self, ErrorKind, Filter};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::utils::types::timestamp::Timestamp;
use crate::view::filter_index::FilterIndex;
use crate::view::format;
use crate::view::i18n::Lang;

/// Bytes shown in the hex dump; longer packets (loopback with offloading) are cut there
const HEX_LIMIT: usize = 16 * 1024;

/// Following segments searched for the rest of a message
pub(crate) const FOLLOWUP_PACKETS: usize = 50;
/// Most bytes added from following segments
const FOLLOWUP_BYTES: usize = 16 * 1024;
/// Most rows of the list sent at once
const MAX_ROWS: usize = 500;

/// A packet dissected with the rest of a message split over TCP segments (a large TLS Client
/// Hello), taken from the following packets of the same connection and direction
pub(crate) struct Dissected {
    pub d: Dissection,
    /// The payloads added after the packet's own bytes
    more: Vec<u8>,
    /// Numbers of the packets they come from
    used: Vec<u64>,
}

/// Dissects `packet`; a message that continues beyond it is completed from `following`
pub(crate) fn dissect_following<'a>(
    link_type: LinkType,
    packet: &StoredPacket,
    following: impl Iterator<Item = &'a StoredPacket>,
) -> Dissected {
    let d = dissect::dissect(&packet.data, link_type);
    let (Some(mut followup), Some(conn), false) = (d.followup, packet.conn, d.truncated) else {
        return Dissected {
            d,
            more: Vec::new(),
            used: Vec::new(),
        };
    };
    let mut more = Vec::new();
    let mut used = Vec::new();
    for next in following.take(FOLLOWUP_PACKETS) {
        if next.conn != Some(conn) {
            continue;
        }
        let Some((seq, range)) = dissect::tcp_payload(&next.data, link_type) else {
            continue;
        };
        // retransmissions and segments out of order are left out
        if range.is_empty() || seq != followup.next_seq {
            continue;
        }
        let len = range.len().min(FOLLOWUP_BYTES - more.len());
        more.extend_from_slice(&next.data[range.start..range.start + len]);
        used.push(next.number);
        if len >= followup.missing || more.len() >= FOLLOWUP_BYTES {
            break;
        }
        followup.missing -= len;
        followup.next_seq = seq.wrapping_add(u32::try_from(len).unwrap_or(0));
    }
    if used.is_empty() {
        return Dissected { d, more, used };
    }
    Dissected {
        d: dissect::dissect_with(&packet.data, link_type, &more),
        more,
        used,
    }
}

fn dissect_stored(store: &PacketStore, packet: &StoredPacket) -> Dissected {
    dissect_following(store.link_type(), packet, store.after(packet.number))
}

/// What the display filter sees of a packet
pub(crate) fn filter_packet<'a>(packet: &StoredPacket, d: &'a Dissection) -> filter::Packet<'a> {
    filter::Packet {
        number: packet.number,
        len: packet.orig_len,
        captured: packet.data.len(),
        d,
    }
}

/// The display filter for the packets of a connection in both directions
pub fn conversation_filter(key: &AddressPortPair) -> String {
    let (proto, ip) = (
        match key.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Icmpv4 => "icmp",
            Protocol::Icmpv6 => "icmpv6",
            Protocol::Igmp => "igmp",
            Protocol::Arp => "arp",
        },
        if key.protocol == Protocol::Arp {
            "arp"
        } else {
            "ip"
        },
    );
    let side = |src: IpAddr, sport: Option<u16>, dst: IpAddr, dport: Option<u16>| {
        let (src_key, dst_key) = if ip == "arp" {
            ("arp.src.ip", "arp.dst.ip")
        } else {
            ("ip.src", "ip.dst")
        };
        let mut s = format!("{src_key} == {src} and {dst_key} == {dst}");
        if let (Some(sp), Some(dp)) = (sport, dport)
            && matches!(key.protocol, Protocol::Tcp | Protocol::Udp)
        {
            let _ = write!(
                s,
                " and {proto}.srcport == {sp} and {proto}.dstport == {dp}"
            );
        }
        s
    };
    let there = side(key.src_ip, key.src_port, key.dst_ip, key.dst_port);
    let back = side(key.dst_ip, key.dst_port, key.src_ip, key.src_port);
    format!("{proto} and (({there}) or ({back}))")
}

/// The connection a display filter selects, if it is the filter of [`conversation_filter`]
fn filtered_conversation<'a>(
    mut keys: impl Iterator<Item = &'a AddressPortPair>,
    filter: &str,
) -> Option<&'a AddressPortPair> {
    let filter = filter.trim();
    // the source address comes first, so most connections are ruled out without any text
    let source: IpAddr = filter
        .split(" == ")
        .nth(1)?
        .split(' ')
        .next()?
        .parse()
        .ok()?;
    keys.find(|key| key.src_ip == source && conversation_filter(key) == filter)
}

/// The two ends of the connection a display filter selects and the connection, for the note above
/// its packets and statistics
pub fn filtered_connection(
    engine: &EngineState,
    filter: &str,
) -> Option<(String, AddressPortPair)> {
    let key = filtered_conversation(engine.info_traffic.map.keys(), filter)?;
    Some((connection_label(key), *key))
}

/// A connection in a line (`connLabel`): "TCP 192.0.2.20:39874 → 198.51.100.140:80", without
/// ports "ICMP 192.0.2.20 → 192.0.2.1"
pub fn connection_label(key: &AddressPortPair) -> String {
    if key.src_port.is_some() || key.dst_port.is_some() {
        format!(
            "{} {} → {}",
            key.protocol,
            format::socket(&key.src_ip, key.src_port),
            format::socket(&key.dst_ip, key.dst_port)
        )
    } else {
        format!("{} {} → {}", key.protocol, key.src_ip, key.dst_ip)
    }
}

/// The connection whose packets a display filter selects (`connNote`)
pub struct ConnectionNote {
    /// [`connection_label`]
    pub label: String,
    pub key: AddressPortPair,
    /// A packet of the connection whose stream can be followed (TCP and UDP)
    pub stream: Option<u64>,
}

/// The note for the connection a display filter selects, if it selects one
pub fn connection_note(engine: &EngineState, filter: &str) -> Option<ConnectionNote> {
    let (label, key) = filtered_connection(engine, filter)?;
    let stream = matches!(key.protocol, Protocol::Tcp | Protocol::Udp)
        .then(|| crate::view::connections::first_packet(engine, &key))
        .flatten();
    Some(ConnectionNote { label, key, stream })
}

/// A filter that cannot be read
pub(crate) struct FilterFault {
    pub message: String,
}

/// The packets a page is about: all, those of a display filter, those of the connections a
/// connection filter selects, or those of both
#[derive(Default)]
pub(crate) struct Selection {
    /// The display filter and its text
    display: Option<(String, Filter)>,
    /// The text of the connection filter and the connections it selects now
    connections: Option<(String, HashSet<AddressPortPair>)>,
}

impl Selection {
    pub(crate) fn read(
        t: Lang,
        engine: &EngineState,
        display: &str,
        connections: &str,
    ) -> Result<Self, FilterFault> {
        let (display, connections) = (display.trim(), connections.trim());
        let selected = Filter::parse_connections(connections).map_err(|e| FilterFault {
            message: filter_error(t, &e),
        })?;
        let filter = Filter::parse(display).map_err(|e| FilterFault {
            message: filter_error(t, &e),
        })?;
        Ok(Self {
            display: filter.map(|filter| (display.to_owned(), filter)),
            connections: selected
                .map(|filter| (connections.to_owned(), engine.selected_connections(&filter))),
        })
    }

    /// Calls `f` with the numbers of the selected packets, oldest first; nothing is selected
    /// without a filter
    fn with_numbers<R>(
        &self,
        index: &FilterIndex,
        store: &SharedPacketStore,
        f: impl FnOnce(&VecDeque<u64>) -> R,
    ) -> R {
        match (&self.display, &self.connections) {
            (None, None) => f(&VecDeque::new()),
            (Some((text, filter)), None) => index.with_matches(store, text, filter, f),
            (None, Some((text, keys))) => index.with_connections(store, text, keys, f),
            (Some((text, filter)), Some((connections, keys))) => {
                let mut both = index.with_matches(store, text, filter, VecDeque::clone);
                let of_connections =
                    index.with_connections(store, connections, keys, VecDeque::clone);
                both.retain(|n| of_connections.binary_search(n).is_ok());
                f(&both)
            }
        }
    }

    /// How many connections the connection filter selects, `None` without one
    pub(crate) fn connection_count(&self) -> Option<usize> {
        self.connections.as_ref().map(|(_, keys)| keys.len())
    }

    /// The numbers of the selected packets, `None` for all of them
    pub(crate) fn numbers(
        &self,
        index: &FilterIndex,
        store: &SharedPacketStore,
    ) -> Option<Vec<u64>> {
        (self.display.is_some() || self.connections.is_some())
            .then(|| self.with_numbers(index, store, |m| m.iter().copied().collect()))
    }
}

/// The time a packet arrived, in the local time zone as other packet analyzers write it
/// (`Nov  9, 2026 12:08:31.402117 CET`)
fn arrival_time(ts: Timestamp) -> String {
    let Ok(at) = jiff::Timestamp::new(ts.secs(), 0) else {
        return String::new();
    };
    let zoned = at.to_zoned(jiff::tz::TimeZone::system());
    format!(
        "{}.{:06} {}",
        zoned.strftime("%b %e, %Y %H:%M:%S"),
        ts.usecs().clamp(0, 999_999),
        zoned.strftime("%Z")
    )
}

/// Seconds since the first packet kept, as in other packet analyzers (`12.345678`)
fn relative_time(ts: Timestamp, first: Option<Timestamp>) -> String {
    let (Some(now), Some(first)) = (ts.to_usecs(), first.and_then(Timestamp::to_usecs)) else {
        return String::new();
    };
    let diff = now - first;
    let sign = if diff < 0 { "-" } else { "" };
    let diff = diff.abs();
    format!("{sign}{}.{:06}", diff / 1_000_000, diff % 1_000_000)
}

/// A packet in the list
#[derive(Clone)]
pub struct PacketRow {
    pub number: u64,
    pub time: String,
    pub src: String,
    pub dst: String,
    pub protocol: &'static str,
    pub length: u32,
    pub info: String,
    /// The connection of the packet (for "Connection" and "Stream" of the selected one)
    pub connection: Option<AddressPortPair>,
    /// Class of the coloring rule, empty for none
    pub color: &'static str,
    /// The call of the packet (for "Anruf" of the selected one)
    pub call: Option<crate::view::calls::CallKey>,
}

fn link_type_name(link_type: LinkType) -> &'static str {
    match link_type {
        LinkType::Ethernet(_) => "Ethernet",
        LinkType::LinuxSll(_) => "Linux cooked capture v1",
        LinkType::LinuxSll2(_) => "Linux cooked capture v2",
        LinkType::Null(_) | LinkType::Loop(_) => "BSD loopback",
        LinkType::RawIp(_) | LinkType::IPv4(_) | LinkType::IPv6(_) => "Raw IP",
        LinkType::Unsupported(_) => "unknown",
    }
}

/// Why there is no packet store
pub(crate) fn unavailable(t: Lang, state: &crate::engine::state::EngineState) -> String {
    if !state.is_running() || state.packets.is_some() {
        return String::new();
    }
    if state.conf.packet_store.enabled {
        t.get("packets-unavailable-source")
    } else {
        t.get("packets-disabled")
    }
}

/// The message of a filter error, with its position
pub(crate) fn filter_error(t: Lang, e: &filter::Error) -> String {
    let message = match &e.kind {
        ErrorKind::UnknownField(name) => t.get1("filter-error-unknown", "name", name),
        ErrorKind::MissingValue => t.get("filter-error-missing"),
        ErrorKind::BadValue { field, value } => {
            t.get2("filter-error-value", "value", value, "field", field)
        }
        ErrorKind::BadOperator { field, op } => {
            t.get2("filter-error-operator", "op", op, "field", field)
        }
        ErrorKind::Unexpected(text) => t.get1("filter-error-unexpected", "text", text),
        ErrorKind::MissingClose => t.get("filter-error-close"),
        ErrorKind::UnclosedString => t.get("filter-error-string"),
        ErrorKind::TooComplex => t.get("filter-error-complex"),
    };
    t.get2("filter-error-at", "pos", e.pos + 1, "message", message)
}

/// The packets the filter selects, in order: how many, the number of the i-th (from 0), how many
/// have a number up to the given one
struct Matching<'a> {
    total: usize,
    nth: &'a dyn Fn(usize) -> u64,
    upto: &'a dyn Fn(u64) -> usize,
}

fn with_matching<R>(
    index: &FilterIndex,
    store: &SharedPacketStore,
    selection: &Selection,
    f: impl FnOnce(&Matching) -> R,
) -> R {
    if selection.display.is_none() && selection.connections.is_none() {
        // the numbers in the store have no gaps
        let (first, total) = {
            let store = store.lock();
            (store.iter().next().map_or(0, |p| p.number), store.len())
        };
        return f(&Matching {
            total,
            nth: &|i| first + i as u64,
            upto: &|n| {
                usize::try_from((n + 1).saturating_sub(first)).map_or(total, |c| c.min(total))
            },
        });
    }
    selection.with_numbers(index, store, |matches| {
        f(&Matching {
            total: matches.len(),
            nth: &|i| matches[i],
            upto: &|n| matches.partition_point(|m| *m <= n),
        })
    })
}

/// Which rows the list asks for
#[derive(Clone, Copy, Default)]
pub struct Window {
    /// Position of the first row among the matching packets, from 0; without it the rows around
    /// `goto`, else the newest
    pub from: Option<usize>,
    pub count: usize,
    /// A packet sought: the first match from this number on, else the last one
    pub goto: Option<u64>,
    /// The packet of the first row shown: where it is now (older packets dropped from the store
    /// move the rows up)
    pub anchor: Option<u64>,
}

/// The rows picked from the matching packets
#[derive(Default, Debug, PartialEq)]
struct Picked {
    total: usize,
    /// Position of the first row
    from: usize,
    numbers: Vec<u64>,
    /// Position and number of the packet found for `goto`
    goto: Option<(usize, u64)>,
    /// Position of the anchor packet (or of the first one after it)
    anchor: Option<usize>,
}

fn pick(m: &Matching, w: Window) -> Picked {
    let total = m.total;
    let count = w.count.clamp(1, MAX_ROWS);
    let goto = w
        .goto
        .filter(|_| total > 0)
        .map(|number| (m.upto)(number.saturating_sub(1)).min(total - 1))
        .map(|index| (index, (m.nth)(index)));
    let from = match (w.from, goto) {
        (Some(from), _) => from,
        (None, Some((index, _))) => index.saturating_sub(count / 2),
        (None, None) => total,
    }
    .min(total.saturating_sub(count));
    Picked {
        total,
        from,
        numbers: (from..(from + count).min(total)).map(m.nth).collect(),
        goto,
        anchor: w
            .anchor
            .map(|number| (m.upto)(number.saturating_sub(1)).min(total)),
    }
}

/// The records of the analysis of these packets, analyzing them first if needed
fn analyzed(store: &SharedPacketStore, numbers: &[u64]) -> HashMap<u64, Note> {
    let Some(last) = numbers.iter().max() else {
        return HashMap::new();
    };
    analysis::advance(store, *last);
    let analysis = store.analysis();
    numbers
        .iter()
        .filter_map(|n| analysis.note(*n).map(|r| (*n, r)))
        .collect()
}

/// The rows of these packets (those dropped meanwhile are left out)
fn rows(store: &SharedPacketStore, numbers: &[u64]) -> Vec<PacketRow> {
    let records = analyzed(store, numbers);
    let calls: HashMap<u64, crate::view::calls::CallKey> = {
        let analysis = store.analysis();
        numbers
            .iter()
            .filter_map(|n| crate::view::calls::key_of(&analysis, *n).map(|k| (*n, k)))
            .collect()
    };
    let store = store.lock();
    let first = store.first_timestamp();
    numbers
        .iter()
        .filter_map(|n| store.get(*n))
        .map(|p| {
            let mut d = dissect_stored(&store, p).d;
            if let Some(note) = records.get(&p.number) {
                analysis::annotate(note, &p.data, &mut d);
            }
            let color = filter::color(&filter_packet(p, &d)).unwrap_or("");
            PacketRow {
                number: p.number,
                time: relative_time(p.timestamp, first),
                src: d.src,
                dst: d.dst,
                protocol: d.protocol,
                length: p.orig_len,
                info: d.info,
                connection: p.conn.as_ref().map(|(key, _)| *key),
                color,
                call: calls.get(&p.number).copied(),
            }
        })
        .collect()
}

pub const FILTER_EXAMPLES: [&str; 8] = [
    "dns",
    "tls.sni contains example",
    "ip.addr == 192.0.2.0/24",
    "tcp.analysis.flags",
    "http.request.method == GET",
    "tcp.port in {80 443}",
    "sip or rtp",
    "not arp and not mdns",
];

/// What surrounds the rows of the list
pub struct PacketList {
    /// Why no packets are kept (store off, IPFIX); empty if they are
    pub unavailable: String,
    /// The connection the display filter selects
    pub connection: Option<ConnectionNote>,
    /// How many connections the connection filter selects
    pub connections: Option<usize>,
    pub error: Option<FilterFault>,
    /// Packets are kept
    pub store: bool,
}

/// The state of the list for a display and a connection filter
pub fn packet_list(t: Lang, engine: &Engine, display: &str, connections: &str) -> PacketList {
    let (unavailable, store, connection, selection) = {
        let engine = engine.lock();
        (
            unavailable(t, &engine),
            engine.packets.is_some(),
            connection_note(&engine, display),
            Selection::read(t, &engine, display, connections),
        )
    };
    let (count, error) = match selection {
        Ok(selection) => (selection.connection_count(), None),
        Err(error) => (None, Some(error)),
    };
    PacketList {
        unavailable,
        connection,
        connections: count,
        error,
        store,
    }
}

/// A block of rows of the list
pub struct RowBlock {
    /// Matching packets
    pub total: usize,
    /// Position of the first row
    pub from: usize,
    pub rows: Vec<PacketRow>,
    /// Position and number of the packet found for `goto`
    pub goto: Option<(usize, u64)>,
    /// Where the anchor packet is now
    pub anchor: Option<usize>,
    /// Packets in the store, and older ones dropped because of its limits
    pub kept: usize,
    pub dropped: u64,
    /// The numbers of the oldest and of the newest packet kept
    pub numbers: (u64, u64),
}

/// The rows of a window of the list; `None` without packets or with a wrong filter
pub fn row_block(
    t: Lang,
    engine: &Engine,
    index: &FilterIndex,
    display: &str,
    connections: &str,
    window: Window,
) -> Option<RowBlock> {
    let (store, selection) = {
        let engine = engine.lock();
        (
            engine.packets.clone(),
            Selection::read(t, &engine, display, connections),
        )
    };
    let store = store?;
    let selection = selection.ok()?;
    let picked = with_matching(index, &store, &selection, |m| pick(m, window));
    let (kept, dropped, numbers) = {
        let s = store.lock();
        let last = s.last_number();
        let first = s.iter().next().map_or(last, |p| p.number);
        (s.len(), s.dropped(), (first, last))
    };
    Some(RowBlock {
        total: picked.total,
        from: picked.from,
        rows: rows(&store, &picked.numbers),
        goto: picked.goto,
        anchor: picked.anchor,
        kept,
        dropped,
        numbers,
    })
}

/// What is wrong with a filter while it is typed: a display filter, or a connection filter
pub fn filter_problem(t: Lang, text: &str, connections: bool) -> Option<String> {
    let parsed = if connections {
        Filter::parse_connections(text)
    } else {
        Filter::parse(text)
    };
    parsed.err().map(|e| filter_error(t, &e))
}

/// A line of the hex dump: offset, bytes as hex and as text, each with its position
pub struct HexLine {
    pub offset: String,
    pub bytes: Vec<(usize, String)>,
    pub text: Vec<(usize, char)>,
}

fn hex_dump(data: &[u8]) -> Vec<HexLine> {
    data.chunks(16)
        .enumerate()
        .map(|(line, chunk)| {
            let start = line * 16;
            HexLine {
                offset: format!("{start:04x}"),
                bytes: chunk
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (start + i, format!("{b:02x}")))
                    .collect(),
                text: chunk
                    .iter()
                    .enumerate()
                    .map(|(i, b)| {
                        let c = if b.is_ascii_graphic() || *b == b' ' {
                            char::from(*b)
                        } else {
                            '·'
                        };
                        (start + i, c)
                    })
                    .collect(),
            }
        })
        .collect()
}

/// How far after a packet its answer or ACK is looked for
const RELATED_SEARCH: usize = 20_000;

/// The record of the analysis of a packet and the packets related to it: the segment it
/// acknowledges, the query or request it answers, the SIP message that negotiated its RTP, and
/// the other way round
fn related_packets(t: Lang, store: &SharedPacketStore, n: u64) -> (Option<Note>, Vec<Related>) {
    let last = store.lock().last_number();
    analysis::advance(store, n.saturating_add(RELATED_SEARCH as u64).min(last));
    let analysis = store.analysis();
    let Some(note) = analysis.note(n) else {
        return (None, Vec::new());
    };
    let record = note.record;
    let related = |lead: String, number: u64, us: Option<u32>, of: Of| Related {
        lead,
        number,
        after: us.map_or_else(String::new, |us| {
            t.get1("packet-after", "time", format::millis(t, f64::from(us)))
        }),
        of,
        layer: 0,
    };
    let mut found = Vec::new();
    if let Some((acked, rtt)) = record.acks {
        found.push(related(t.get("packet-acks"), acked, Some(rtt), Of::Tcp));
    }
    if let Some((query, time)) = record.dns {
        found.push(related(t.get("packet-answers"), query, Some(time), Of::Dns));
    }
    let request = match record.voip {
        Some(Mark::Sip {
            answers: Some((request, time)),
            ..
        }) => {
            found.push(related(
                t.get("packet-answers"),
                request,
                Some(time),
                Of::Sip,
            ));
            false
        }
        Some(Mark::Sip { status: None, .. }) => true,
        _ => false,
    };
    if let Some(setup) = note.voip.as_ref().and_then(|v| v.media.as_ref()?.setup) {
        found.push(related(t.get("packet-sdp-setup"), setup, None, Of::Rtp));
    }
    let mut answers = 0;
    for (number, later) in analysis.after(n).take(RELATED_SEARCH) {
        if let Some((acked, rtt)) = later.acks
            && acked == n
        {
            found.push(related(
                t.get("packet-acked-by"),
                number,
                Some(rtt),
                Of::Tcp,
            ));
            // only one packet acknowledges a segment
            break;
        }
        if let Some((query, time)) = later.dns
            && query == n
        {
            found.push(related(
                t.get("packet-answered-by"),
                number,
                Some(time),
                Of::Dns,
            ));
            break;
        }
        if request
            && let Some(Mark::Sip {
                answers: Some((asked, time)),
                status: Some(status),
                ..
            }) = later.voip
            && asked == n
        {
            let lead = t.get1("packet-sip-answer", "status", status);
            found.push(related(lead, number, Some(time), Of::Sip));
            answers += 1;
            if answers >= MAX_ANSWERS {
                break;
            }
        }
    }
    (Some(note), found)
}

/// Answers to a SIP request shown at most (provisional ones, the final one, ones sent again)
const MAX_ANSWERS: usize = 20;

/// The layer a related packet is shown in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Of {
    Tcp,
    Dns,
    Sip,
    Rtp,
}

/// A packet this one answers or acknowledges, or that answers it (`.rel`): "→ acknowledged in
/// packet 1206 (after 18.4 ms)"
pub struct Related {
    /// "→ acknowledged in packet"
    pub lead: String,
    pub number: u64,
    /// "(after 18.4 ms)"
    pub after: String,
    /// The protocol it is about
    of: Of,
    /// The layer it is shown in
    pub layer: usize,
}

impl Related {
    /// The whole line
    pub fn text(&self) -> String {
        if self.after.is_empty() {
            format!("{} {}", self.lead, self.number)
        } else {
            format!("{} {} {}", self.lead, self.number, self.after)
        }
    }
}

/// Text for the clipboard. It comes from the network, and where it is pasted a control character
/// or a line break must not act: pasted into a terminal, a line break runs what stands before it
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '.' } else { c })
        .collect()
}

/// The value of a field, for the clipboard
pub fn field_value(field: &Field) -> String {
    clean(&field.value)
}

/// A field as one line, for the clipboard
pub fn field_line(field: &Field) -> String {
    if field.value.is_empty() {
        field.name.to_string()
    } else {
        format!("{}: {}", field.name, field_value(field))
    }
}

fn push_field(text: &mut String, field: &Field, depth: usize) {
    text.push('\n');
    text.push_str(&"  ".repeat(depth));
    text.push_str(&field_line(field));
    for child in &field.children {
        push_field(text, child, depth + 1);
    }
}

/// A layer with its fields, indented as in the tree, for the clipboard
pub fn layer_text(layer: &Layer) -> String {
    let mut text = clean(&layer.title);
    for field in &layer.fields {
        push_field(&mut text, field, 1);
    }
    text
}

/// All layers, for the clipboard
pub fn layers_text(layers: &[Layer]) -> String {
    layers.iter().map(layer_text).collect::<Vec<_>>().join("\n")
}

/// The bytes of a range as hex, for the clipboard
pub fn selection_hex(lines: &[HexLine], range: &Range<usize>) -> String {
    lines
        .iter()
        .flat_map(|line| &line.bytes)
        .filter(|(byte, _)| range.contains(byte))
        .map(|(_, hex)| hex.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The dot of the clipboard for what has no character; the screen shows a middle dot
fn plain_char(c: char) -> char {
    if c == '·' { '.' } else { c }
}

/// The bytes of a range as text, for the clipboard
pub fn selection_text(lines: &[HexLine], range: &Range<usize>) -> String {
    lines
        .iter()
        .flat_map(|line| &line.text)
        .filter(|(byte, _)| range.contains(byte))
        .map(|(_, c)| plain_char(*c))
        .collect()
}

/// The bytes as the hex dump shows them, for the clipboard
pub fn dump_text(lines: &[HexLine]) -> String {
    lines
        .iter()
        .map(|line| {
            let mut text = format!("{}  ", line.offset);
            for (i, (_, hex)) in line.bytes.iter().enumerate() {
                text.push_str(hex);
                text.push_str(if i == 7 { "  " } else { " " });
            }
            let missing = 16 - line.bytes.len();
            let padding = missing * 3 + usize::from(line.bytes.len() <= 7) + 1;
            text.push_str(&" ".repeat(padding));
            text.extend(line.text.iter().map(|(_, c)| plain_char(*c)));
            text
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One packet: its layers and bytes and what belongs to it
pub struct PacketDetail {
    pub number: u64,
    pub protocol: &'static str,
    pub info: String,
    pub truncated: bool,
    pub headers_only: bool,
    pub layers: Vec<Layer>,
    pub hex: Vec<HexLine>,
    /// Note that the hex dump ends before the packet
    pub hex_cut: Option<String>,
    /// Note that a message was completed from the following packets
    pub assembled: Option<String>,
    pub time: Timestamp,
    /// The connection of the packet
    pub connection: Option<AddressPortPair>,
    /// The stream of the packet can be followed (TCP and UDP)
    pub has_stream: bool,
    /// The neighbors in the store
    pub prev: Option<u64>,
    pub next: Option<u64>,
    /// Packets this one answers or acknowledges, or that answer it
    pub related: Vec<Related>,
    /// The call the packet belongs to, and its name
    pub call: Option<(crate::view::calls::CallKey, String)>,
}

/// The details of packet `n`, if it is kept
pub fn packet_detail(t: Lang, engine: &Engine, n: u64) -> Option<PacketDetail> {
    let shared = engine.lock().packets.clone()?;
    let (note, related) = related_packets(t, &shared, n);
    let call = crate::view::calls::call_of(t, engine, n);
    let store = shared.lock();
    let packet = store.get(n)?;
    let link_type = store.link_type();
    let Dissected { mut d, more, used } = dissect_stored(&store, packet);
    if let Some(note) = &note {
        analysis::annotate(note, &packet.data, &mut d);
        found_by_form(t, note, &mut d);
    }
    let captured = packet.data.len();
    let mut fields = vec![
        Field::new("Arrival Time", arrival_time(packet.timestamp), 0..0),
        Field::new(
            "Time since first packet",
            format!(
                "{} seconds",
                relative_time(packet.timestamp, store.first_timestamp())
            ),
            0..0,
        ),
        Field::new("Frame Length", format!("{} bytes", packet.orig_len), 0..0),
    ];
    fields.push(match store.interface() {
        Some(name) => Field::new("Interface", name.to_owned(), 0..0),
        None => Field::new("Link type", link_type_name(link_type).to_owned(), 0..0),
    });
    let frame = Layer {
        title: format!(
            "Frame {}: {} bytes on wire ({} bits), {captured} bytes captured",
            packet.number,
            packet.orig_len,
            u64::from(packet.orig_len) * 8
        ),
        range: 0..captured,
        fields,
    };
    let mut layers = vec![frame];
    layers.extend(d.layers);
    let mut related = related;
    for r in &mut related {
        let titles: &[&str] = match r.of {
            Of::Dns => &[
                "Domain Name System",
                "Multicast Domain",
                "Link-local Multicast",
            ],
            Of::Tcp => &["Transmission Control Protocol"],
            Of::Sip => &["Session Initiation Protocol"],
            Of::Rtp => &[
                "Real-Time Transport Protocol",
                "Real-time Transport Control",
            ],
        };
        r.layer = layers
            .iter()
            .rposition(|l| titles.iter().any(|t| l.title.starts_with(t)))
            .unwrap_or(layers.len() - 1);
    }
    let neighbor = |n: u64| store.get(n).map(|_| n);
    Some(PacketDetail {
        number: packet.number,
        protocol: d.protocol,
        info: d.info,
        truncated: d.truncated,
        headers_only: store.headers_only(),
        layers,
        hex: if more.is_empty() {
            hex_dump(&packet.data[..captured.min(HEX_LIMIT)])
        } else {
            let mut joined = packet.data.to_vec();
            joined.extend_from_slice(&more);
            hex_dump(&joined)
        },
        assembled: (!used.is_empty()).then(|| {
            let numbers: Vec<String> = used.iter().map(u64::to_string).collect();
            t.get2(
                "packet-assembled",
                "packets",
                numbers.join(", "),
                "offset",
                format!("0x{captured:04x}"),
            )
        }),
        hex_cut: (captured > HEX_LIMIT).then(|| t.get1("packet-hex-cut", "bytes", HEX_LIMIT)),
        time: packet.timestamp,
        connection: packet.conn.map(|(key, _)| key),
        has_stream: packet
            .conn
            .is_some_and(|(key, _)| matches!(key.protocol, Protocol::Tcp | Protocol::Udp)),
        prev: n.checked_sub(1).and_then(neighbor),
        next: neighbor(n + 1),
        related,
        call,
    })
}

/// Says how RTP found without SIP was found, and that a setting decides it
fn found_by_form(t: Lang, note: &Note, d: &mut Dissection) {
    let by_form = note
        .voip
        .as_ref()
        .and_then(|v| v.media.as_ref())
        .is_some_and(|m| !m.rtcp && m.setup.is_none());
    if by_form
        && let Some(layer) = d
            .layers
            .iter_mut()
            .find(|l| l.title == "Real-Time Transport Protocol")
    {
        let at = layer.range.end..layer.range.end;
        layer.fields.push(Field::new(
            t.get("packet-rtp-found"),
            t.get("packet-rtp-found-how"),
            at,
        ));
    }
}

/// The packets before and after packet `n` among those the filters of the list select (all kept
/// packets without a filter or with one that cannot be read), for paging through the details
pub fn neighbors(
    t: Lang,
    engine: &Engine,
    index: &FilterIndex,
    display: &str,
    connections: &str,
    n: u64,
) -> (Option<u64>, Option<u64>) {
    let (store, selection) = {
        let engine = engine.lock();
        (
            engine.packets.clone(),
            Selection::read(t, &engine, display, connections),
        )
    };
    let Some(store) = store else {
        return (None, None);
    };
    match selection {
        Ok(selection) if selection.display.is_some() || selection.connections.is_some() => {
            selection.with_numbers(index, &store, |numbers| {
                let at = numbers.partition_point(|&m| m < n);
                let prev = at.checked_sub(1).and_then(|i| numbers.get(i)).copied();
                let after = if numbers.get(at) == Some(&n) {
                    at + 1
                } else {
                    at
                };
                (prev, numbers.get(after).copied())
            })
        }
        _ => {
            let store = store.lock();
            let kept = |m: u64| store.get(m).map(|_| m);
            (
                n.checked_sub(1).and_then(kept),
                n.checked_add(1).and_then(kept),
            )
        }
    }
}

/// A PCAP file with the kept packets
pub struct PcapFile {
    /// Name with the time of the export
    pub name: String,
    pub data: Vec<u8>,
}

/// The kept packets (of the filters) as a PCAP file
pub fn pcap_file(
    t: Lang,
    engine: &Engine,
    index: &FilterIndex,
    display: &str,
    connections: &str,
) -> Option<PcapFile> {
    let (store, selection) = {
        let engine = engine.lock();
        (
            engine.packets.clone(),
            Selection::read(t, &engine, display, connections),
        )
    };
    let store = store?;
    let selection = selection.ok()?;
    let data = if let Some(numbers) = selection.numbers(index, &store) {
        let store = store.lock();
        store.to_pcap(numbers.iter().filter_map(|n| store.get(*n)))
    } else {
        let store = store.lock();
        store.to_pcap(store.iter())
    };
    Some(PcapFile {
        name: pcap_name(),
        data,
    })
}

/// The name of an exported file, with the time of the export
fn pcap_name() -> String {
    #[allow(clippy::cast_possible_wrap)]
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (y, mo, d, h, mi, s) = format::civil_utc(now);
    format!("openpacketmon-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.pcap")
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::networking::types::traffic_direction::TrafficDirection;

    use crate::config::conf::Conf;
    use crate::engine::Engine;
    use std::net::Ipv4Addr;

    use super::*;

    fn sample_layer() -> Layer {
        Layer {
            title: "Transmission Control Protocol".to_owned(),
            range: 0..20,
            fields: vec![
                Field::new("Source port", "443", 0..2),
                Field::new("Flags", "0x12", 13..14).with_children(vec![Field::new(
                    "Syn",
                    "set",
                    13..14,
                )]),
                Field::new("Payload", "", 20..20),
            ],
        }
    }

    #[test]
    fn a_layer_is_copied_as_its_tree() {
        assert_eq!(
            layer_text(&sample_layer()),
            "Transmission Control Protocol\n  Source port: 443\n  Flags: 0x12\n    Syn: set\n  Payload",
        );
        let two = [sample_layer(), sample_layer()];
        assert_eq!(layers_text(&two).lines().count(), 10);
    }

    /// Pasted into a terminal, a line break out of a packet would run what stands before it
    #[test]
    fn what_comes_from_the_network_cannot_act_where_it_is_pasted() {
        let field = Field::new("Host", "a\nrm -rf x\u{1b}[0m", 0..1);
        assert_eq!(field_value(&field), "a.rm -rf x.[0m");
        assert_eq!(field_line(&field), "Host: a.rm -rf x.[0m");
        let layer = Layer {
            title: "One\ntwo".to_owned(),
            range: 0..1,
            fields: Vec::new(),
        };
        assert_eq!(layer_text(&layer), "One.two");
    }

    #[test]
    fn bytes_are_copied_as_hex_as_text_and_as_a_dump() {
        let mut data = b"GET /\r\n".to_vec();
        data.extend_from_slice(&[0xff; 12]);
        let lines = hex_dump(&data);
        assert_eq!(selection_hex(&lines, &(0..3)), "47 45 54");
        // across the end of a line
        assert_eq!(selection_hex(&lines, &(15..17)), "ff ff");
        assert_eq!(selection_text(&lines, &(0..7)), "GET /..");
        assert_eq!(selection_hex(&lines, &(100..110)), "");
        let dump = dump_text(&lines);
        let mut rows = dump.lines();
        assert_eq!(
            rows.next(),
            Some("0000  47 45 54 20 2f 0d 0a ff  ff ff ff ff ff ff ff ff  GET /...........")
        );
        assert_eq!(
            rows.next(),
            Some("0010  ff ff ff                                          ...")
        );
    }

    fn packet(src: [u8; 4], sport: u16, dst: [u8; 4], dport: u16) -> StoredPacket {
        StoredPacket {
            number: 1,
            timestamp: Timestamp::new(1, 0),
            orig_len: 60,
            data: std::sync::Arc::new([]),
            conn: Some((
                AddressPortPair {
                    src_ip: IpAddr::V4(Ipv4Addr::from(src)),
                    src_port: Some(sport),
                    dst_ip: IpAddr::V4(Ipv4Addr::from(dst)),
                    dst_port: Some(dport),
                    protocol: Protocol::Tcp,
                    exporter: None,
                },
                TrafficDirection::Outgoing,
            )),
        }
    }

    #[test]
    fn a_client_hello_is_completed_from_the_next_segment() {
        use crate::config::packet_store::PacketStoreConf;
        use crate::dissect::tests::{ETH, client_hello, tcp_packet};

        let record = client_hello("split.example.org", 1600);
        let (first, second) = record.split_at(1400);
        let conn = packet([192, 168, 1, 10], 51000, [10, 0, 0, 2], 443).conn;
        let other = packet([192, 168, 1, 10], 51001, [10, 0, 0, 2], 443).conn;
        let mut store = PacketStore::new(PacketStoreConf::default(), ETH);
        let ts = Timestamp::new(1, 0);
        store.push(ts, 1454, tcp_packet(51000, 443, 5000, first).into(), conn);
        // another connection, then a segment that does not continue the message
        store.push(ts, 60, tcp_packet(51001, 443, 6400, second).into(), other);
        store.push(ts, 60, tcp_packet(51000, 443, 9999, second).into(), conn);
        let alone = dissect_stored(&store, store.get(1).unwrap());
        assert_eq!(alone.d.info, "Client Hello");
        assert!(alone.used.is_empty());

        store.push(ts, 60, tcp_packet(51000, 443, 6400, second).into(), conn);
        let joined = dissect_stored(&store, store.get(1).unwrap());
        assert_eq!(joined.d.info, "Client Hello (SNI=split.example.org)");
        assert_eq!(joined.used, [4]);
        assert_eq!(joined.more, second);
    }

    #[test]
    fn the_conversation_filter_takes_both_directions() {
        use crate::config::packet_store::PacketStoreConf;
        use crate::dissect::tests::{ETH, client_hello, tcp_packet, udp_packet};

        let key = packet([192, 168, 1, 10], 51000, [10, 0, 0, 2], 443)
            .conn
            .unwrap()
            .0;
        let text = conversation_filter(&key);
        assert_eq!(
            text,
            "tcp and ((ip.src == 192.168.1.10 and ip.dst == 10.0.0.2 and tcp.srcport == 51000 \
             and tcp.dstport == 443) or (ip.src == 10.0.0.2 and ip.dst == 192.168.1.10 \
             and tcp.srcport == 443 and tcp.dstport == 51000))"
        );
        assert!(filtered_conversation([&key].into_iter(), &format!(" {text} ")).is_some());
        assert!(filtered_conversation([&key].into_iter(), &format!("{text} and dns")).is_none());
        assert!(filtered_conversation([&key].into_iter(), "ip.src == 10.0.0.2").is_none());
        assert!(filtered_conversation([&key].into_iter(), "not == a filter").is_none());
        assert!(filtered_conversation([&key].into_iter(), "").is_none());
        let filter = Filter::parse(&text).unwrap().unwrap();
        let mut store = PacketStore::new(PacketStoreConf::default(), ETH);
        let ts = Timestamp::new(1, 0);
        let hello = client_hello("a.example", 0);
        store.push(ts, 100, tcp_packet(51000, 443, 1, &hello).into(), None);
        store.push(ts, 100, tcp_packet(51001, 443, 1, &hello).into(), None);
        store.push(ts, 60, udp_packet(51000, 443, b"x").into(), None);
        let passes: Vec<u64> = store
            .iter()
            .filter(|p| {
                let d = dissect_stored(&store, p).d;
                filter.matches(&filter_packet(p, &d))
            })
            .map(|p| p.number)
            .collect();
        assert_eq!(passes, [1]);
    }

    #[test]
    fn the_index_keeps_the_matches_as_packets_arrive() {
        use crate::config::packet_store::PacketStoreConf;
        use crate::dissect::tests::{ETH, client_hello, tcp_packet, udp_packet};

        let store = SharedPacketStore::new(PacketStoreConf::default(), ETH);
        let index = FilterIndex::default();
        let text = "tls.sni contains split";
        let filter = Filter::parse(text).unwrap().unwrap();
        let ts = Timestamp::new(1, 0);
        let conn = packet([192, 168, 1, 10], 51000, [10, 0, 0, 2], 443).conn;
        let record = client_hello("split.example.org", 1600);
        let (first, second) = record.split_at(1400);
        store
            .lock()
            .push(ts, 60, udp_packet(1, 2, b"x").into(), None);
        store
            .lock()
            .push(ts, 1454, tcp_packet(51000, 443, 5000, first).into(), conn);
        // the second segment is still missing: packet 2 waits, nothing matches yet
        assert!(index.with_matches(&store, text, &filter, |m| m.is_empty()));
        store
            .lock()
            .push(ts, 400, tcp_packet(51000, 443, 6400, second).into(), conn);
        let matches: Vec<u64> =
            index.with_matches(&store, text, &filter, |m| m.iter().copied().collect());
        assert_eq!(matches, [2]);
        // further packets are judged on top
        store.lock().push(
            ts,
            1454,
            tcp_packet(51000, 443, 9000, &client_hello("split.b", 0)).into(),
            conn,
        );
        let matches: Vec<u64> =
            index.with_matches(&store, text, &filter, |m| m.iter().copied().collect());
        assert_eq!(matches, [2, 4]);
    }

    #[test]
    fn the_index_waits_until_a_flow_is_known_as_rtp() {
        use crate::config::packet_store::PacketStoreConf;
        use crate::dissect::tests::{ETH, rtp_bytes, udp_between};

        let store = SharedPacketStore::new(PacketStoreConf::default(), ETH);
        let index = FilterIndex::default();
        let filter = Filter::parse("rtp").unwrap().unwrap();
        let ts = Timestamp::new(1, 0);
        let push = |i: u16| {
            let m = rtp_bytes(111, 100 + i, u32::from(i) * 960, 0x9b2e_7714, false, 80);
            let p = udp_between([192, 0, 2, 20], 52714, [198, 51, 100, 7], 10000, &m);
            store.lock().push(ts, 134, p.into(), None);
        };
        let matches = || -> Vec<u64> {
            index.with_matches(&store, "rtp", &filter, |m| m.iter().copied().collect())
        };
        for i in 0..5 {
            push(i);
        }
        // not decided yet: the packets wait
        assert!(matches().is_empty());
        for i in 5..12 {
            push(i);
        }
        assert_eq!(matches(), (1..=12).collect::<Vec<u64>>());
    }

    #[test]
    fn the_index_follows_the_connections_of_a_filter() {
        use crate::config::packet_store::PacketStoreConf;
        use crate::dissect::tests::{ETH, tcp_packet};

        let store = SharedPacketStore::new(PacketStoreConf::default(), ETH);
        let index = FilterIndex::default();
        let ts = Timestamp::new(1, 0);
        let a = packet([192, 168, 1, 10], 51000, [10, 0, 0, 2], 443).conn;
        let b = packet([192, 168, 1, 10], 51001, [10, 0, 0, 3], 80).conn;
        let push = |conn| {
            store
                .lock()
                .push(ts, 60, tcp_packet(51000, 443, 1, b"x").into(), conn);
        };
        push(a);
        push(b);
        push(None);
        let numbers = |keys: &HashSet<AddressPortPair>| -> Vec<u64> {
            index.with_connections(&store, "tcp", keys, |m| m.iter().copied().collect())
        };
        let only_a = HashSet::from([a.unwrap().0]);
        assert_eq!(numbers(&only_a), [1]);
        // new packets are judged on top, a changed selection takes the older ones too
        push(a);
        assert_eq!(numbers(&only_a), [1, 4]);
        let both = HashSet::from([a.unwrap().0, b.unwrap().0]);
        assert_eq!(numbers(&both), [1, 2, 4]);
        assert_eq!(numbers(&HashSet::new()), [0u64; 0]);
        assert_eq!(numbers(&both), [1, 2, 4]);

        // the same text as a display filter is another entry, and both filters can apply
        let filter = Filter::parse("frame.number > 1").unwrap().unwrap();
        let selection = Selection {
            display: Some(("frame.number > 1".to_owned(), filter)),
            connections: Some(("tcp".to_owned(), only_a)),
        };
        assert_eq!(selection.numbers(&index, &store), Some(vec![4]));
        let display = Filter::parse("tcp").unwrap().unwrap();
        let all: Vec<u64> =
            index.with_matches(&store, "tcp", &display, |m| m.iter().copied().collect());
        assert_eq!(all, [1, 2, 3, 4]);
        assert_eq!(Selection::default().numbers(&index, &store), None);
    }

    #[test]
    fn the_list_picks_blocks_of_rows() {
        let numbers = [3u64, 5, 8, 9, 12];
        let pick_from = |window: Window| {
            pick(
                &Matching {
                    total: numbers.len(),
                    nth: &|i| numbers[i],
                    upto: &|n| numbers.partition_point(|m| *m <= n),
                },
                window,
            )
        };
        let w = |from, count| Window {
            from,
            count,
            ..Window::default()
        };
        // a block, the newest rows, a block beyond the end
        assert_eq!(pick_from(w(Some(1), 2)).numbers, [5, 8]);
        let s = pick_from(w(None, 2));
        assert_eq!((s.from, s.numbers), (3, vec![9, 12]));
        let s = pick_from(w(Some(9), 2));
        assert_eq!((s.from, s.numbers), (3, vec![9, 12]));
        let s = pick_from(w(None, 10));
        assert_eq!((s.from, s.numbers.len(), s.total), (0, 5, 5));

        // go to packet: the rows around it; not in the list: the next one, or the last one
        let goto = |number, count| {
            pick_from(Window {
                goto: Some(number),
                count,
                ..Window::default()
            })
        };
        let s = goto(8, 3);
        assert_eq!(
            (s.from, s.numbers, s.goto),
            (1, vec![5, 8, 9], Some((2, 8)))
        );
        assert_eq!(goto(6, 1).goto, Some((2, 8)));
        assert_eq!(goto(1, 1).goto, Some((0, 3)));
        assert_eq!(goto(99, 1).goto, Some((4, 12)));
        let empty = pick(
            &Matching {
                total: 0,
                nth: &|_| 0,
                upto: &|_| 0,
            },
            Window {
                goto: Some(4),
                count: 2,
                ..Window::default()
            },
        );
        assert_eq!((empty.goto, empty.numbers.len()), (None, 0));

        // the anchor: where the first row shown is now (or the next one after it)
        let anchor = |number| {
            pick_from(Window {
                from: Some(0),
                count: 1,
                anchor: Some(number),
                ..Window::default()
            })
            .anchor
        };
        assert_eq!(anchor(8), Some(2));
        assert_eq!(anchor(4), Some(1));
        assert_eq!(anchor(1), Some(0));
    }

    #[test]
    fn times_are_relative_to_the_first_packet() {
        let first = Some(Timestamp::new(100, 900_000));
        assert_eq!(
            relative_time(Timestamp::new(102, 100_000), first),
            "1.200000"
        );
        assert_eq!(
            relative_time(Timestamp::new(100, 900_000), first),
            "0.000000"
        );
    }

    #[test]
    fn the_hex_dump_shows_text_and_positions() {
        let lines = hex_dump(b"GET / HTTP/1.1\r\nHost: x\r\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].offset, "0010");
        assert_eq!(lines[0].bytes[0], (0, "47".to_owned()));
        assert_eq!(lines[1].text[0], (16, 'H'));
        assert_eq!(lines[0].text[14], (14, '·'));
    }

    fn engine() -> (tokio::runtime::Runtime, Engine) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        (runtime, engine)
    }

    #[test]
    #[serial]
    fn a_wrong_filter_is_explained() {
        let (_runtime, engine) = engine();
        let list = packet_list(Lang::En, &engine, "tcp.port ==", "");
        let fault = list.error.unwrap();
        assert!(
            fault.message.contains("Error at position 12"),
            "{}",
            fault.message
        );

        // the connection filter has its own fields
        let list = packet_list(Lang::En, &engine, "", "country ==");
        assert!(list.error.is_some());
        assert!(
            packet_list(Lang::En, &engine, "", "country == US")
                .error
                .is_none()
        );

        let problem = filter_problem(Lang::En, "foo", false).unwrap();
        assert!(problem.contains("unknown field or protocol"), "{problem}");
        assert_eq!(filter_problem(Lang::En, "dns", false), None);
        assert_eq!(filter_problem(Lang::En, "country == US", true), None);
    }

    #[test]
    #[serial]
    fn without_a_capture_there_are_no_packets() {
        let (_runtime, engine) = engine();
        let index = FilterIndex::default();
        let window = Window {
            from: Some(0),
            count: 50,
            goto: None,
            anchor: None,
        };
        assert!(row_block(Lang::En, &engine, &index, "", "", window).is_none());
        assert!(pcap_file(Lang::En, &engine, &index, "", "").is_none());
    }
}
