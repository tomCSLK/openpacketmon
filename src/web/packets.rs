//! The packets kept of the capture: one long list with a display filter (the browser shows only the
//! rows in view and loads them in blocks, like a packet analyzer), the details of one packet with
//! its layers and a hex dump, and the download as a PCAP file.

use std::collections::HashMap;
use std::fmt::Write;
use std::net::IpAddr;

use askama::Template;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sniffnet_packet_parser::{LinkType, Protocol};

use crate::dissect::{self, Dissection, Field, Layer};
use crate::engine::analysis::{self, PacketAnalysis};
use crate::engine::packets::{PacketStore, SharedPacketStore, StoredPacket};
use crate::engine::state::EngineState;
use crate::filter::{self, ErrorKind, Filter};
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::utils::types::timestamp::Timestamp;
use crate::web::auth::Session;
use crate::web::connections::connection_query;
use crate::web::entries::encode;
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError, format};

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

/// The two ends and the page of the connection a display filter selects, for the note above
/// its packets and statistics
pub fn filtered_connection(engine: &EngineState, filter: &str) -> Option<(String, String)> {
    let key = filtered_conversation(engine.info_traffic.map.keys(), filter)?;
    Some((
        format!(
            "{} ⇄ {}",
            format::socket(&key.src_ip, key.src_port),
            format::socket(&key.dst_ip, key.dst_port)
        ),
        format!("/connection?{}", connection_query(key)),
    ))
}

/// The filter of the list and the packet to show, as given in the address
#[derive(Deserialize, Default, Clone)]
pub struct PacketQuery {
    /// Display filter
    #[serde(default)]
    pub(crate) f: String,
    /// Show this packet (or the next matching one) and mark it; without it the list follows the
    /// newest packets
    goto: Option<u64>,
}

impl PacketQuery {
    /// The filter as query string
    fn filter_query(&self) -> String {
        if self.f.trim().is_empty() {
            String::new()
        } else {
            format!("f={}", encode(self.f.trim()))
        }
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

/// A packet in the list (sent as JSON, the browser builds the row)
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
    state: &AppState,
    store: &SharedPacketStore,
    text: &str,
    filter: Option<&Filter>,
    f: impl FnOnce(&Matching) -> R,
) -> R {
    match filter {
        // the numbers in the store have no gaps
        None => {
            let (first, total) = {
                let store = store.lock();
                (store.iter().next().map_or(0, |p| p.number), store.len())
            };
            f(&Matching {
                total,
                nth: &|i| first + i as u64,
                upto: &|n| {
                    usize::try_from((n + 1).saturating_sub(first)).map_or(total, |c| c.min(total))
                },
            })
        }
        Some(filter) => state
            .packet_index
            .with_matches(store, text, filter, |matches| {
                f(&Matching {
                    total: matches.len(),
                    nth: &|i| matches[i],
                    upto: &|n| matches.partition_point(|m| *m <= n),
                })
            }),
    }
}

/// Which rows the list asks for
#[derive(Clone, Copy, Default)]
struct Window {
    /// Position of the first row among the matching packets, from 0; without it the rows around
    /// `goto`, else the newest
    from: Option<usize>,
    count: usize,
    /// A packet sought: the first match from this number on, else the last one
    goto: Option<u64>,
    /// The packet of the first row the browser shows: where it is now (older packets dropped
    /// from the store move the rows up)
    anchor: Option<u64>,
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

#[derive(Template)]
#[template(path = "packets.html")]
struct PacketsPage {
    t: Lang,
    layout: Layout,
    running: bool,
    /// Why no packets are kept (store off, IPFIX); empty if they are
    unavailable: String,
    /// The display filter as typed
    filter: String,
    /// The connection the filter selects: its two ends and its page
    connection: Option<(String, String)>,
    error: Option<String>,
    /// The list can be shown (packets kept, filter valid)
    list: bool,
    goto: Option<u64>,
    download: String,
    statistics: String,
    foot: Option<String>,
    examples: [&'static str; 8],
    protocols: String,
    fields: String,
}

const FILTER_EXAMPLES: [&str; 8] = [
    "dns",
    "tls.sni contains amazon",
    "ip.addr == 192.168.1.20 and not arp",
    "tcp.port in {80 443} and tcp.flags.syn",
    "domain contains google",
    "frame.len > 1000",
    "tcp.analysis.flags",
    "dns.time > 100ms",
];

/// `GET /packets`: the page; the browser loads the rows with [`rows_api`]
pub async fn list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PacketQuery>,
) -> Response {
    let t = i18n::current();
    let (layout, running, unavailable, store, connection) = {
        let engine = state.engine.lock();
        (
            Layout::new(&engine, &session),
            engine.is_running(),
            unavailable(t, &engine),
            engine.packets.clone(),
            filtered_connection(&engine, &query.f),
        )
    };
    let error = Filter::parse(&query.f).err().map(|e| filter_error(t, &e));
    let filter_query = query.filter_query();
    render(&PacketsPage {
        t,
        layout,
        running,
        unavailable,
        filter: query.f.clone(),
        connection,
        list: store.is_some() && error.is_none(),
        error,
        goto: query.goto,
        download: if filter_query.is_empty() {
            "/packets.pcap".to_owned()
        } else {
            format!("/packets.pcap?{filter_query}")
        },
        statistics: if filter_query.is_empty() {
            "/statistics".to_owned()
        } else {
            format!("/statistics?{filter_query}")
        },
        foot: store.as_ref().map(|s| foot_text(t, s)),
        examples: FILTER_EXAMPLES,
        protocols: filter::PROTOCOLS.join(" "),
        fields: filter::FIELDS
            .iter()
            .map(|(name, _, _)| *name)
            .chain(["frame.number", "frame.len", "frame.cap_len"])
            .collect::<Vec<_>>()
            .join(" "),
    })
}

#[derive(Deserialize)]
pub struct RowsQuery {
    #[serde(default)]
    f: String,
    from: Option<usize>,
    #[serde(default)]
    count: usize,
    goto: Option<u64>,
    anchor: Option<u64>,
}

/// A block of rows of the list
#[derive(Serialize)]
pub struct RowBlock {
    /// Matching packets
    total: usize,
    /// Position of the first row
    from: usize,
    rows: Vec<PacketRow>,
    /// Position and number of the packet found for `goto`
    goto: Option<(usize, u64)>,
    /// Where the anchor packet is now
    anchor: Option<usize>,
    foot: String,
    running: bool,
}

/// `GET /api/packets?f=…&from=…&count=…`: rows of the list (`goto` finds a packet, without
/// `from` and `goto` the newest rows)
pub async fn rows_api(
    State(state): State<AppState>,
    _session: Session,
    Query(query): Query<RowsQuery>,
) -> Result<Json<RowBlock>, WebError> {
    let t = i18n::current();
    let (store, running) = {
        let engine = state.engine.lock();
        (engine.packets.clone(), engine.is_running())
    };
    let store = store.ok_or(WebError::NotFound)?;
    let text = query.f.trim();
    let filter = Filter::parse(text).map_err(|_| WebError::NotFound)?;
    let window = Window {
        from: query.from,
        count: query.count,
        goto: query.goto,
        anchor: query.anchor,
    };
    let picked = with_matching(&state, &store, text, filter.as_ref(), |m| pick(m, window));
    Ok(Json(RowBlock {
        total: picked.total,
        from: picked.from,
        rows: rows(&store, &picked.numbers),
        goto: picked.goto,
        anchor: picked.anchor,
        foot: foot_text(t, &store),
        running,
    }))
}

#[derive(Deserialize)]
pub struct FilterCheck {
    #[serde(default)]
    f: String,
    /// `connections` for the connection filter, else the packet filter
    #[serde(default)]
    kind: String,
}

/// `GET /api/filter?f=…`: whether a display filter is valid, while it is typed
pub async fn check(_session: Session, Query(query): Query<FilterCheck>) -> Json<Option<String>> {
    let t = i18n::current();
    let parsed = if query.kind == "connections" {
        Filter::parse_connections(&query.f)
    } else {
        Filter::parse(&query.f)
    };
    Json(parsed.err().map(|e| filter_error(t, &e)))
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

#[derive(Template)]
#[template(path = "packet.html")]
struct PacketPage {
    t: Lang,
    layout: Layout,
    number: u64,
    protocol: &'static str,
    info: String,
    truncated: bool,
    headers_only: bool,
    layers: Vec<Layer>,
    hex: Vec<HexLine>,
    /// Note that the hex dump ends before the packet
    hex_cut: Option<String>,
    /// Note that a message was completed from the following packets
    assembled: Option<String>,
    time: String,
    connection: Option<String>,
    /// Follow the stream (TCP and UDP)
    stream: Option<String>,
    /// The packets and the statistics of the connection
    conversation: Option<String>,
    statistics: Option<String>,
    back: String,
    prev: Option<String>,
    next: Option<String>,
    /// Packets this one answers or acknowledges, or that answer it: label and link
    related: Vec<(String, String)>,
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

#[derive(Deserialize)]
pub struct PacketDetailQuery {
    n: u64,
    #[serde(default)]
    back: String,
}

/// `GET /packet?n=…`: layers and bytes of one packet
pub async fn detail(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PacketDetailQuery>,
) -> Result<Response, WebError> {
    let t = i18n::current();
    let (layout, store) = {
        let engine = state.engine.lock();
        (Layout::new(&engine, &session), engine.packets.clone())
    };
    let shared = store.ok_or(WebError::NotFound)?;
    let (record, related) = related_packets(t, &shared, query.n);
    let store = shared.lock();
    let packet = store.get(query.n).ok_or(WebError::NotFound)?;
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
    let back = crate::web::capture::back_path(&query.back, "/packets");
    let back_param = encode(&back);
    let neighbor = |n: u64| {
        store
            .get(n)
            .map(|_| format!("/packet?n={n}&back={back_param}"))
    };
    let page = PacketPage {
        t,
        layout,
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
        time: format::time_html(packet.timestamp),
        connection: packet
            .conn
            .as_ref()
            .map(|(key, _)| format!("/connection?{}", connection_query(key))),
        stream: packet
            .conn
            .filter(|(key, _)| matches!(key.protocol, Protocol::Tcp | Protocol::Udp))
            .map(|_| format!("/stream?n={}", packet.number)),
        conversation: packet
            .conn
            .as_ref()
            .map(|(key, _)| format!("/packets?f={}", encode(&conversation_filter(key)))),
        statistics: packet
            .conn
            .as_ref()
            .map(|(key, _)| format!("/statistics?f={}", encode(&conversation_filter(key)))),
        prev: query.n.checked_sub(1).and_then(neighbor),
        next: neighbor(query.n + 1),
        related: related
            .into_iter()
            .map(|(label, n)| (label, format!("/packet?n={n}&back={back_param}")))
            .collect(),
        back,
    };
    Ok(render(&page))
}

/// `GET /packets.pcap`: the kept packets (of the filter) as a PCAP file
pub async fn download(
    State(state): State<AppState>,
    _session: Session,
    Query(query): Query<PacketQuery>,
) -> Result<Response, WebError> {
    let store = state.engine.lock().packets.clone();
    let store = store.ok_or(WebError::NotFound)?;
    let filter = Filter::parse(&query.f).map_err(|_| WebError::NotFound)?;
    let file = if let Some(filter) = &filter {
        let numbers: Vec<u64> =
            state
                .packet_index
                .with_matches(&store, query.f.trim(), filter, |m| {
                    m.iter().copied().collect()
                });
        let store = store.lock();
        store.to_pcap(numbers.iter().filter_map(|n| store.get(*n)))
    } else {
        let store = store.lock();
        store.to_pcap(store.iter())
    };
    #[allow(clippy::cast_possible_wrap)]
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (y, mo, d, h, mi, s) = format::civil_utc(now);
    let name = format!("openpacketmon-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.pcap");
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.tcpdump.pcap".to_owned(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        file,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use axum::http::StatusCode;

    use super::*;
    use crate::networking::types::address_port_pair::AddressPortPair;
    use crate::web::testutil::*;

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
        use crate::engine::packets::SharedPacketStore;
        use crate::web::packet_index::FilterIndex;

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

    #[tokio::test]
    async fn without_a_capture_there_are_no_packets() {
        let h = harness().await;
        let reply = send(
            &h.app,
            get("/packets?f=tcp.port%20%3D%3D%20443", Some(&h.cookie)),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK);
        let reply = send(&h.app, get("/api/packets?count=50", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = send(&h.app, get("/packet?n=1", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = send(&h.app, get("/packets.pcap", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = send(&h.app, get("/packets.pcap", None)).await;
        assert_ne!(reply.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_wrong_filter_is_explained() {
        let h = harness().await;
        let reply = send(&h.app, get("/packets?f=tcp.port%20%3D%3D", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(
            reply.body.contains("Error at position 12"),
            "{}",
            reply.body
        );
        let reply = send(&h.app, get("/api/filter?f=foo", Some(&h.cookie))).await;
        assert!(
            reply.body.contains("unknown field or protocol"),
            "{}",
            reply.body
        );
        let reply = send(&h.app, get("/api/filter?f=dns", Some(&h.cookie))).await;
        assert_eq!(reply.body, "null");
        let reply = send(&h.app, get("/api/filter?f=dns", None)).await;
        assert_ne!(reply.status, StatusCode::OK);
    }
}
