//! The packets kept of the capture, as the interface shows them: one long list with a display
//! filter (only the rows in view are read, in blocks, like in a packet analyzer), the details of one
//! packet with its layers and a hex dump, and the export as a PCAP file.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::net::IpAddr;

use serde::Serialize;
use sniffnet_packet_parser::{LinkType, Protocol};

use crate::dissect::{self, Dissection, Field, Layer};
use crate::engine::Engine;
use crate::engine::analysis::{self, PacketAnalysis};
use crate::engine::packets::{PacketStore, SharedPacketStore, StoredPacket};
use crate::engine::state::EngineState;
use crate::filter::{self, ErrorKind, Filter};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::traffic_direction::TrafficDirection;
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
    Some((
        format!(
            "{} ⇄ {}",
            format::socket(&key.src_ip, key.src_port),
            format::socket(&key.dst_ip, key.dst_port)
        ),
        *key,
    ))
}

/// A filter that cannot be read
pub(crate) struct FilterFault {
    pub message: String,
    /// It is the connection filter, not the display filter
    pub connections: bool,
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
            connections: true,
        })?;
        let filter = Filter::parse(display).map_err(|e| FilterFault {
            message: filter_error(t, &e),
            connections: false,
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

/// A packet in the list (the browser pages get it as JSON and build the row)
#[derive(Serialize)]
pub struct PacketRow {
    #[serde(rename = "n")]
    pub number: u64,
    pub time: String,
    pub src: String,
    pub dst: String,
    #[serde(rename = "proto")]
    pub protocol: &'static str,
    #[serde(rename = "len")]
    pub length: u32,
    pub info: String,
    #[serde(rename = "out")]
    pub outgoing: Option<bool>,
    /// Class of the coloring rule, empty for none
    pub color: &'static str,
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
fn analyzed(store: &SharedPacketStore, numbers: &[u64]) -> HashMap<u64, PacketAnalysis> {
    let Some(last) = numbers.iter().max() else {
        return HashMap::new();
    };
    analysis::advance(store, *last);
    let analysis = store.analysis();
    numbers
        .iter()
        .filter_map(|n| analysis.get(*n).map(|r| (*n, *r)))
        .collect()
}

/// The rows of these packets (those dropped meanwhile are left out)
fn rows(store: &SharedPacketStore, numbers: &[u64]) -> Vec<PacketRow> {
    let records = analyzed(store, numbers);
    let store = store.lock();
    let first = store.first_timestamp();
    numbers
        .iter()
        .filter_map(|n| store.get(*n))
        .map(|p| {
            let mut d = dissect_stored(&store, p).d;
            if let Some(record) = records.get(&p.number) {
                analysis::annotate(record, &mut d);
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
                outgoing: p
                    .conn
                    .as_ref()
                    .map(|(_, dir)| *dir == TrafficDirection::Outgoing),
                color,
            }
        })
        .collect()
}

/// What the store holds: packets, memory, link type …
fn foot_text(t: Lang, store: &SharedPacketStore) -> String {
    let store = store.lock();
    let mut parts = vec![
        t.get1(
            "packets-kept",
            "count",
            format::count(t, store.len() as u128),
        ),
        format::bytes(t, store.bytes() as u128),
        link_type_name(store.link_type()).to_owned(),
    ];
    if store.dropped() > 0 {
        parts.push(t.get1(
            "packets-dropped",
            "count",
            format::count(t, u128::from(store.dropped())),
        ));
    }
    if store.headers_only() {
        parts.push(t.get("packets-headers-only"));
    }
    parts.join(" · ")
}

pub const FILTER_EXAMPLES: [&str; 8] = [
    "dns",
    "tls.sni contains amazon",
    "ip.addr == 192.168.1.20 and not arp",
    "tcp.port in {80 443} and tcp.flags.syn",
    "domain contains google",
    "frame.len > 1000",
    "tcp.analysis.flags",
    "dns.time > 100ms",
];

/// The names the display filter knows besides the protocols
pub fn filter_field_names() -> Vec<&'static str> {
    filter::FIELDS
        .iter()
        .map(|(name, _, _)| *name)
        .chain(["frame.number", "frame.len", "frame.cap_len"])
        .collect()
}

/// What surrounds the rows of the list
pub struct PacketList {
    pub running: bool,
    /// Why no packets are kept (store off, IPFIX); empty if they are
    pub unavailable: String,
    /// The connection the display filter selects: its two ends and its key
    pub connection: Option<(String, AddressPortPair)>,
    pub error: Option<FilterFault>,
    /// The list can be shown (packets kept, filters valid)
    pub list: bool,
    pub foot: Option<String>,
}

/// The state of the list for a display and a connection filter
pub fn packet_list(t: Lang, engine: &Engine, display: &str, connections: &str) -> PacketList {
    let (running, unavailable, store, connection, error) = {
        let engine = engine.lock();
        (
            engine.is_running(),
            unavailable(t, &engine),
            engine.packets.clone(),
            filtered_connection(&engine, display),
            Selection::read(t, &engine, display, connections).err(),
        )
    };
    PacketList {
        running,
        unavailable,
        connection,
        list: store.is_some() && error.is_none(),
        error,
        foot: store.as_ref().map(|s| foot_text(t, s)),
    }
}

/// A block of rows of the list
#[derive(Serialize)]
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
    pub foot: String,
    pub running: bool,
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
    let (store, running, selection) = {
        let engine = engine.lock();
        (
            engine.packets.clone(),
            engine.is_running(),
            Selection::read(t, &engine, display, connections),
        )
    };
    let store = store?;
    let selection = selection.ok()?;
    let picked = with_matching(index, &store, &selection, |m| pick(m, window));
    Some(RowBlock {
        total: picked.total,
        from: picked.from,
        rows: rows(&store, &picked.numbers),
        goto: picked.goto,
        anchor: picked.anchor,
        foot: foot_text(t, &store),
        running,
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
/// acknowledges, the query it answers, and the other way round
fn related_packets(
    t: Lang,
    store: &SharedPacketStore,
    n: u64,
) -> (Option<PacketAnalysis>, Vec<(String, u64)>) {
    let last = store.lock().last_number();
    analysis::advance(store, n.saturating_add(RELATED_SEARCH as u64).min(last));
    let analysis = store.analysis();
    let Some(record) = analysis.get(n).copied() else {
        return (None, Vec::new());
    };
    let ms = |us: u32| format::millis(t, f64::from(us));
    let mut related = Vec::new();
    if let Some((acked, rtt)) = record.acks {
        related.push((
            t.get2("packet-acks", "packet", acked, "time", ms(rtt)),
            acked,
        ));
    }
    if let Some((query, time)) = record.dns {
        related.push((
            t.get2("packet-answers", "packet", query, "time", ms(time)),
            query,
        ));
    }
    for (number, later) in analysis.after(n).take(RELATED_SEARCH) {
        if let Some((acked, rtt)) = later.acks
            && acked == n
        {
            related.push((
                t.get2("packet-acked-by", "packet", number, "time", ms(rtt)),
                number,
            ));
            // only one packet acknowledges a segment
            break;
        }
        if let Some((query, time)) = later.dns
            && query == n
        {
            related.push((
                t.get2("packet-answered-by", "packet", number, "time", ms(time)),
                number,
            ));
            break;
        }
    }
    (Some(record), related)
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
    /// Packets this one answers or acknowledges, or that answer it: label and number
    pub related: Vec<(String, u64)>,
}

/// The details of packet `n`, if it is kept
pub fn packet_detail(t: Lang, engine: &Engine, n: u64) -> Option<PacketDetail> {
    let shared = engine.lock().packets.clone()?;
    let (record, related) = related_packets(t, &shared, n);
    let store = shared.lock();
    let packet = store.get(n)?;
    let link_type = store.link_type();
    let Dissected { mut d, more, used } = dissect_stored(&store, packet);
    if let Some(record) = &record {
        analysis::annotate(record, &mut d);
    }
    let captured = packet.data.len();
    let frame = Layer {
        title: format!(
            "Frame {}: {} bytes on wire, {captured} bytes captured",
            packet.number, packet.orig_len
        ),
        range: 0..captured,
        fields: vec![
            Field::new(
                "Time since the first packet",
                format!(
                    "{} s",
                    relative_time(packet.timestamp, store.first_timestamp())
                ),
                0..0,
            ),
            Field::new("Frame length", format!("{} bytes", packet.orig_len), 0..0),
            Field::new("Capture length", format!("{captured} bytes"), 0..0),
            Field::new("Link type", link_type_name(link_type).to_owned(), 0..0),
        ],
    };
    let mut layers = vec![frame];
    layers.extend(d.layers);
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
    })
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
    use std::net::Ipv4Addr;

    use super::*;

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
}
