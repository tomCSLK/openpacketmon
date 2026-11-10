//! Statistics of the stored packets, for all of them or those of a display filter (`pageStatistics`
//! of the draft): tiles, the traffic of the selection, the protocol hierarchy, the findings of the
//! TCP analysis and response times.

use std::collections::HashMap;

use crate::engine::Engine;
use crate::engine::analysis::{self, PacketAnalysis, flag};
use crate::view::filter_index::FilterIndex;
use crate::view::i18n::Lang;
use crate::view::packets::{ConnectionNote, FilterFault, Selection, connection_note, unavailable};
use crate::view::{Jump, format};

/// The findings shown: flag, message id, filter
const FINDINGS: [(u16, &str, &str); 8] = [
    (
        flag::RETRANSMISSION,
        "stats-retransmission",
        "tcp.analysis.retransmission",
    ),
    (
        flag::LOST_SEGMENT,
        "stats-lost-segment",
        "tcp.analysis.lost_segment",
    ),
    (
        flag::OUT_OF_ORDER,
        "stats-out-of-order",
        "tcp.analysis.out_of_order",
    ),
    (
        flag::DUPLICATE_ACK,
        "stats-duplicate-ack",
        "tcp.analysis.duplicate_ack",
    ),
    (
        flag::ZERO_WINDOW,
        "stats-zero-window",
        "tcp.analysis.zero_window",
    ),
    (flag::RESET, "stats-reset", "tcp.flags.rst"),
    (
        flag::KEEP_ALIVE,
        "stats-keep-alive",
        "tcp.analysis.keep_alive",
    ),
    (
        flag::WINDOW_UPDATE,
        "stats-window-update",
        "tcp.analysis.window_update",
    ),
];

/// The protocols of the hierarchy: their name in the filter, the short name in the table and the
/// long one beside it (`FN`, `LONG` of the draft)
const PROTOCOLS: [(&str, &str, &str); 26] = [
    ("eth", "Ethernet", "Ethernet"),
    ("vlan", "VLAN", "802.1Q Virtual LAN"),
    ("sll", "SLL", "Linux cooked capture"),
    ("loopback", "Loopback", "Null/Loopback"),
    ("ip", "IPv4", "Internet Protocol Version 4"),
    ("ipv6", "IPv6", "Internet Protocol Version 6"),
    ("arp", "ARP", "Address Resolution Protocol"),
    ("tcp", "TCP", "Transmission Control Protocol"),
    ("udp", "UDP", "User Datagram Protocol"),
    ("icmp", "ICMP", "Internet Control Message Protocol"),
    ("icmpv6", "ICMPv6", "Internet Control Message Protocol v6"),
    ("igmp", "IGMP", "Internet Group Management Protocol"),
    ("tls", "TLS", "Transport Layer Security"),
    ("http", "HTTP", "Hypertext Transfer Protocol"),
    ("dns", "DNS", "Domain Name System"),
    ("mdns", "MDNS", "Multicast DNS"),
    ("llmnr", "LLMNR", "Link-local Multicast Name Resolution"),
    ("quic", "QUIC", "QUIC IETF"),
    ("ssdp", "SSDP", "Simple Service Discovery Protocol"),
    ("ntp", "NTP", "Network Time Protocol"),
    ("dhcp", "DHCP", "Dynamic Host Configuration Protocol"),
    (
        "dhcpv6",
        "DHCPv6",
        "Dynamic Host Configuration Protocol for IPv6",
    ),
    ("sip", "SIP", "Session Initiation Protocol"),
    ("sdp", "SDP", "Session Description Protocol"),
    ("rtp", "RTP", "Real-Time Transport Protocol"),
    ("rtcp", "RTCP", "Real-time Transport Control Protocol"),
];

/// Short and long name of a protocol of the filter
fn protocol_names(id: &'static str) -> (&'static str, &'static str) {
    PROTOCOLS
        .iter()
        .find(|(name, ..)| *name == id)
        .map_or((id, id), |(_, short, long)| (*short, *long))
}

/// The protocols of a packet as the hierarchy shows them: multicast DNS and LLMNR right below
/// UDP, not below DNS
fn hierarchy_path(protocols: &[&'static str]) -> Vec<&'static str> {
    let mut path: Vec<&'static str> = Vec::with_capacity(protocols.len());
    for p in protocols {
        if matches!(*p, "mdns" | "llmnr") && path.last() == Some(&"dns") {
            path.pop();
        }
        path.push(p);
    }
    path
}

/// Sums of a response time
#[derive(Default, Clone, Copy)]
struct Times {
    count: u64,
    sum: u64,
    max: u32,
}

impl Times {
    fn add(&mut self, us: u32) {
        self.count += 1;
        self.sum += u64::from(us);
        self.max = self.max.max(us);
    }
}

/// What the statistics count
#[derive(Default)]
struct Totals {
    packets: u64,
    bytes: u64,
    first: Option<i64>,
    last: Option<i64>,
    paths: HashMap<u16, (u64, u64)>,
    flags: [u64; FINDINGS.len()],
    handshake: Times,
    ack_rtt: Times,
    dns: Times,
}

impl Totals {
    fn add(&mut self, r: &PacketAnalysis) {
        self.packets += 1;
        self.bytes += u64::from(r.len);
        self.first = Some(self.first.map_or(r.time, |t| t.min(r.time)));
        self.last = Some(self.last.map_or(r.time, |t| t.max(r.time)));
        let path = self.paths.entry(r.path).or_default();
        path.0 += 1;
        path.1 += u64::from(r.len);
        for (i, (bit, _, _)) in FINDINGS.iter().enumerate() {
            if r.flags & bit != 0 {
                self.flags[i] += 1;
            }
        }
        if let Some(us) = r.handshake {
            self.handshake.add(us);
        }
        if let Some((_, us)) = r.acks {
            self.ack_rtt.add(us);
        }
        if let Some((_, us)) = r.dns {
            self.dns.add(us);
        }
    }

    /// The time from the first to the last packet in seconds
    #[allow(clippy::cast_precision_loss)]
    fn span(&self) -> f64 {
        match (self.first, self.last) {
            (Some(first), Some(last)) => (last - first).max(0) as f64 / 1e6,
            _ => 0.0,
        }
    }
}

/// A tile (`.tile`): label, value, its unit and the line below
pub struct StatTile {
    pub label: String,
    pub value: String,
    pub unit: String,
    pub sub: String,
}

/// A line of the protocol hierarchy (`table.hier`)
pub struct HierarchyRow {
    pub name: &'static str,
    pub long: &'static str,
    pub depth: usize,
    pub packets: String,
    /// Share of the packets, `28,4 %`
    pub share: String,
    /// Width of the bar, 0.5–100
    pub bar: f64,
    pub bytes: String,
    pub rate: String,
    pub jump: Jump,
}

/// A finding of the TCP analysis with its number of packets (`table.findings`)
pub struct FindingRow {
    pub label: String,
    pub count: String,
    /// Packets have it (`tr.has`, else `tr.off`)
    pub has: bool,
    /// The number in the warning color (`.count.warn`)
    pub warn: bool,
    pub jump: Option<Jump>,
}

/// A response time: how often, on average, at most
pub struct TimeRow {
    pub label: String,
    pub count: String,
    pub average: String,
    pub max: String,
    /// Not measured (`tr.off`)
    pub off: bool,
}

/// The chart of the selection: incoming and outgoing bytes per second of each step, the labels
/// of the time axis and the times under the pointer
#[derive(Default)]
pub struct Chart {
    pub points: Vec<[f64; 2]>,
    pub labels: Vec<String>,
    pub tips: Vec<String>,
}

/// A filter combined with the one of the page
fn and_filter(page: &str, extra: &str) -> String {
    if page.trim().is_empty() {
        extra.to_owned()
    } else {
        format!("({}) and {extra}", page.trim())
    }
}

/// The packets behind a number of the statistics: their filters and `extra`
pub fn packets_jump(display: &str, connections: &str, extra: &str) -> Jump {
    Jump::Packets {
        filter: and_filter(display, extra),
        connections: connections.to_owned(),
    }
}

/// Seconds since the start of the capture as the packet list writes them (`ftime`: `12.345678`)
fn seconds(us: i64) -> String {
    let sign = if us < 0 { "-" } else { "" };
    let us = us.abs();
    format!("{sign}{}.{:06}", us / 1_000_000, us % 1_000_000)
}

/// Milliseconds of microseconds: one decimal, three below a millisecond
fn millis(t: Lang, us: f64) -> String {
    let ms = us / 1000.0;
    format!("{} ms", format::fixed(t, ms, if ms < 1.0 { 3 } else { 1 }))
}

/// Bits per second of bytes in `seconds` (`fbits(bytes / span)`)
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn bit_rate(t: Lang, bytes: u64, seconds: f64) -> String {
    let bits = (bytes as f64 * 8.0 / seconds.max(1.0)).round() as u128;
    format!("{}/s", format::bits(t, bits))
}

/// The tiles: packets, bytes, time span and average rates
#[allow(clippy::cast_precision_loss)]
fn tiles(t: Lang, totals: &Totals, start: i64) -> Vec<StatTile> {
    let span = totals.span();
    let tile = |label: &str, value: String, unit: &str, sub: String| StatTile {
        label: t.get(label),
        value,
        unit: unit.to_owned(),
        sub,
    };
    let split = |amount: String| -> (String, String) {
        amount
            .rsplit_once(' ')
            .map_or((amount.clone(), String::new()), |(v, u)| {
                (v.to_owned(), u.to_owned())
            })
    };
    let (bytes, bytes_unit) = split(format::bytes(t, u128::from(totals.bytes)));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (span_value, span_unit) = if span < 60.0 {
        (format::fixed(t, span, if span < 1.0 { 3 } else { 1 }), "s")
    } else {
        (format::clock(span.floor() as u64), "min")
    };
    let first = totals.first.map_or(0, |f| f - start);
    let last = totals.last.map_or(0, |l| l - start);
    let (rate, rate_unit) = split(bit_rate(t, totals.bytes, span));
    vec![
        tile(
            "stats-packets",
            format::count(t, u128::from(totals.packets)),
            "",
            String::new(),
        ),
        tile(
            "stats-bytes",
            bytes,
            &bytes_unit,
            t.get1(
                "stats-bytes-sub",
                "count",
                format::count(t, u128::from(totals.bytes)),
            ),
        ),
        tile(
            "stats-duration",
            span_value,
            span_unit,
            t.fmt(
                "stats-span-sub",
                &[
                    ("from", seconds(first).into()),
                    ("to", seconds(last).into()),
                ],
            ),
        ),
        tile(
            "stats-packet-rate",
            format::fixed(t, totals.packets as f64 / span.max(1.0), 1),
            "/s",
            String::new(),
        ),
        tile("stats-bit-rate", rate, &rate_unit, String::new()),
    ]
}

/// The local time of Unix seconds (`clock`)
fn local_clock(second: i64) -> String {
    jiff::Timestamp::from_second(second).map_or_else(
        |_| String::new(),
        |time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%H:%M:%S")
                .to_string()
        },
    )
}

/// The traffic of the selection (`sChart`): the whole capture as the time axis; if the selection
/// is shorter than two minutes, only its time span in 120 steps (else it would be a needle).
/// `packets` are time, length and direction of the selected packets; `start` and `end` the times
/// of the first and the last packet of the capture, all in microseconds.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn chart(t: Lang, packets: &[(i64, u32, bool)], totals: &Totals, start: i64, end: i64) -> Chart {
    let (Some(first), Some(_)) = (totals.first, totals.last) else {
        return Chart::default();
    };
    let span = totals.span();
    let long = span >= 120.0;
    let rel = |us: i64| (us - start) as f64 / 1e6;
    let from = if long { 0.0 } else { rel(first) };
    let dur = if long {
        rel(end).ceil().max(1.0)
    } else {
        span.max(0.001)
    };
    let bucket = if long {
        (dur / 240.0).ceil().max(1.0)
    } else {
        dur / 120.0
    };
    let steps = if long {
        (dur / bucket).ceil().max(1.0) as usize
    } else {
        120
    };
    // the step of a packet, counted in microseconds
    let (origin, width) = if long {
        (start, (bucket as i64 * 1_000_000).max(1))
    } else {
        (first, 0)
    };
    let span_us = (totals.last.unwrap_or(first) - first).max(1000);
    let mut bins = vec![[0.0_f64; 2]; steps];
    for (time, len, outgoing) in packets {
        let after = (time - origin).max(0);
        let i = if long {
            after / width
        } else {
            after.saturating_mul(120) / span_us
        };
        let i = usize::try_from(i).unwrap_or(usize::MAX).min(steps - 1);
        bins[i][usize::from(*outgoing)] += f64::from(*len);
    }
    let points = bins.iter().map(|[a, b]| [a / bucket, b / bucket]).collect();
    let decimals = if dur < 1.0 {
        3
    } else if dur < 10.0 {
        2
    } else {
        1
    };
    let start_second = start.div_euclid(1_000_000);
    let (mut labels, mut tips) = (Vec::with_capacity(steps), Vec::with_capacity(steps));
    for i in 0..steps {
        let s = from + i as f64 * bucket;
        if long {
            let clock = local_clock(start_second + s as i64);
            labels.push(clock.get(..5).unwrap_or(&clock).to_owned());
            tips.push(if bucket > 1.0 {
                t.fmt(
                    "stats-tip-step",
                    &[
                        ("time", clock.into()),
                        ("step", format::count(t, bucket as u128).into()),
                    ],
                )
            } else {
                t.get1("chart-tip-time", "time", clock)
            });
        } else {
            labels.push(format!("+{} s", format::fixed(t, s - from, decimals)));
            tips.push(t.fmt(
                "stats-tip-relative",
                &[
                    ("time", seconds((s * 1e6).round() as i64).into()),
                    ("after", format::fixed(t, s - from, decimals + 1).into()),
                ],
            ));
        }
    }
    Chart {
        points,
        labels,
        tips,
    }
}

/// The protocol hierarchy: every path adds to all of its beginnings; the most packets first
fn hierarchy(
    t: Lang,
    totals: &Totals,
    paths: &[Vec<&'static str>],
    filters: Filters,
) -> Vec<HierarchyRow> {
    let mut nodes: HashMap<Vec<&'static str>, (u64, u64)> = HashMap::new();
    for (path, (packets, bytes)) in &totals.paths {
        let Some(protocols) = paths.get(usize::from(*path)) else {
            continue;
        };
        let protocols = hierarchy_path(protocols);
        for end in 1..=protocols.len() {
            let node = nodes.entry(protocols[..end].to_vec()).or_default();
            node.0 += packets;
            node.1 += bytes;
        }
    }
    let mut rows = Vec::new();
    add_children(t, &nodes, &[], totals, filters, &mut rows);
    rows
}

/// The filters of the statistics
#[derive(Clone, Copy)]
struct Filters<'a> {
    display: &'a str,
    connections: &'a str,
}

fn add_children(
    t: Lang,
    nodes: &HashMap<Vec<&'static str>, (u64, u64)>,
    parent: &[&'static str],
    totals: &Totals,
    filters: Filters,
    rows: &mut Vec<HierarchyRow>,
) {
    let mut children: Vec<(&Vec<&'static str>, &(u64, u64))> = nodes
        .iter()
        .filter(|(path, _)| path.len() == parent.len() + 1 && path.starts_with(parent))
        .collect();
    children.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    let span = totals.span();
    for (path, (packets, bytes)) in children {
        let id = path[path.len() - 1];
        let (name, long) = protocol_names(id);
        #[allow(clippy::cast_precision_loss)]
        let share = if totals.packets == 0 {
            0.0
        } else {
            *packets as f64 * 100.0 / totals.packets as f64
        };
        rows.push(HierarchyRow {
            name,
            long,
            depth: parent.len(),
            packets: format::count(t, u128::from(*packets)),
            share: format!("{} %", format::fixed(t, share, 1)),
            bar: share.max(0.5),
            bytes: format::bytes(t, u128::from(*bytes)),
            rate: bit_rate(t, *bytes, span),
            jump: packets_jump(filters.display, filters.connections, id),
        });
        add_children(t, nodes, path, totals, filters, rows);
    }
}

/// The findings of the TCP analysis and the response times, with links to their packets
fn analysis_rows(t: Lang, totals: &Totals, filters: Filters) -> (Vec<FindingRow>, Vec<TimeRow>) {
    let findings = FINDINGS
        .iter()
        .zip(totals.flags)
        .map(|((bit, id, field), count)| FindingRow {
            label: t.get(id),
            count: format::count(t, u128::from(count)),
            has: count > 0,
            warn: count > 0 && *bit != flag::KEEP_ALIVE,
            jump: (count > 0).then(|| packets_jump(filters.display, filters.connections, field)),
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let time_row = |id: &str, times: Times| {
        let measured = times.count > 0;
        TimeRow {
            label: t.get(id),
            count: format::count(t, u128::from(times.count)),
            average: if measured {
                millis(t, times.sum as f64 / times.count as f64)
            } else {
                "–".to_owned()
            },
            max: if measured {
                millis(t, f64::from(times.max))
            } else {
                "–".to_owned()
            },
            off: !measured,
        }
    };
    let times = vec![
        time_row("stats-handshake", totals.handshake),
        time_row("stats-ack-rtt", totals.ack_rtt),
        time_row("stats-dns-time", totals.dns),
    ];
    (findings, times)
}

/// The statistics of a display and a connection filter
pub struct Statistics {
    pub unavailable: String,
    /// The connection the display filter selects
    pub connection: Option<ConnectionNote>,
    /// How many connections the connection filter selects
    pub connections: Option<usize>,
    pub error: Option<FilterFault>,
    /// The statistics change while packets come
    pub live: bool,
    pub tiles: Vec<StatTile>,
    pub chart: Chart,
    pub hierarchy: Vec<HierarchyRow>,
    pub findings: Vec<FindingRow>,
    pub times: Vec<TimeRow>,
    /// The calls over SIP, for the tab "Anrufe"
    pub calls: usize,
}

/// Counts the kept packets of the filters
pub fn statistics(
    t: Lang,
    engine: &Engine,
    index: &FilterIndex,
    display: &str,
    connections: &str,
) -> Statistics {
    let (running, unavailable, store, finished, connection, selection) = {
        let engine = engine.lock();
        (
            engine.is_running(),
            unavailable(t, &engine),
            engine.packets.clone(),
            engine.traffic_history.no_more_packets(),
            connection_note(&engine, display),
            Selection::read(t, &engine, display, connections),
        )
    };
    let (selection, error) = match selection {
        Ok(selection) => (selection, None),
        Err(fault) => (Selection::default(), Some(fault)),
    };
    let connection_count = selection.connection_count();

    let mut totals = Totals::default();
    let mut points = Vec::new();
    let mut paths = Vec::new();
    let (mut start, mut end) = (0, 0);
    if let (Some(store), None) = (&store, &error) {
        let numbers = selection.numbers(index, store);
        let last = {
            let store = store.lock();
            start = store
                .first_timestamp()
                .and_then(crate::utils::types::timestamp::Timestamp::to_usecs)
                .unwrap_or(0);
            let last = store.last_number();
            end = store
                .get(last)
                .and_then(|p| p.timestamp.to_usecs())
                .unwrap_or(start);
            last
        };
        analysis::advance(store, last);
        let analysis = store.analysis();
        let mut add = |r: &PacketAnalysis| {
            totals.add(r);
            points.push((r.time, r.len, r.outgoing));
        };
        match &numbers {
            Some(numbers) => numbers
                .iter()
                .filter_map(|n| analysis.get(*n))
                .for_each(&mut add),
            None => analysis.after(0).for_each(|(_, r)| add(r)),
        }
        paths = analysis.paths().to_vec();
    }
    if let Some(first) = totals.first {
        start = start.min(first);
    }

    let filters = Filters {
        display,
        connections,
    };
    let (findings, times) = analysis_rows(t, &totals, filters);
    Statistics {
        live: running && !finished && store.is_some() && error.is_none(),
        unavailable,
        connection,
        connections: connection_count,
        error,
        tiles: tiles(t, &totals, start),
        chart: chart(
            t,
            &points,
            &totals,
            start,
            end.max(totals.last.unwrap_or(0)),
        ),
        hierarchy: hierarchy(t, &totals, &paths, filters),
        findings,
        times,
        calls: crate::view::calls::count(engine),
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::config::conf::Conf;
    use crate::engine::Engine;

    fn totals_of(packets: &[(i64, u32, bool)]) -> Totals {
        let mut totals = Totals::default();
        for (time, len, _) in packets {
            totals.packets += 1;
            totals.bytes += u64::from(*len);
            totals.first = Some(totals.first.map_or(*time, |f: i64| f.min(*time)));
            totals.last = Some(totals.last.map_or(*time, |l: i64| l.max(*time)));
        }
        totals
    }

    #[test]
    fn a_short_selection_is_shown_in_120_steps() {
        let s = 1_000_000;
        // 12 s of a capture that started 100 s before
        let packets = [
            (110 * s, 100, false),
            (116 * s, 50, true),
            (122 * s, 30, false),
        ];
        let totals = totals_of(&packets);
        let c = chart(Lang::De, &packets, &totals, 10 * s, 122 * s);
        assert_eq!(c.points.len(), 120);
        // 0.1 s per step: 100 bytes in a tenth of a second are 1000 bytes per second
        assert!((c.points[0][0] - 1000.0).abs() < 1e-6);
        assert!((c.points[60][1] - 500.0).abs() < 1e-6);
        assert!((c.points[119][0] - 300.0).abs() < 1e-6);
        assert_eq!(c.labels[0], "+0,0 s");
        assert_eq!(c.labels[60], "+6,0 s");
        assert_eq!(c.tips[60], "106.000000 s (+6,00 s)");
    }

    #[test]
    fn a_long_selection_is_shown_on_the_time_of_the_capture() {
        let s = 1_000_000;
        // an hour: steps of 15 seconds from the start of the capture
        let packets = [(0, 1500, false), (3600 * s, 1500, true)];
        let totals = totals_of(&packets);
        let c = chart(Lang::En, &packets, &totals, 0, 3600 * s);
        assert_eq!(c.points.len(), 240);
        assert!((c.points[0][0] - 100.0).abs() < 1e-9);
        assert!((c.points[239][1] - 100.0).abs() < 1e-9);
        assert_eq!(c.labels[0].len(), 5);
        assert!(c.tips[1].ends_with("(15 s)"));
    }

    #[test]
    fn tiles_are_written_as_in_the_draft() {
        let s = 1_000_000;
        let packets = [(2 * s, 1500, false), (44 * s, 2000, true)];
        let shown = tiles(Lang::De, &totals_of(&packets), 0);
        let values: Vec<(&str, &str, &str)> = shown
            .iter()
            .map(|t| (t.value.as_str(), t.unit.as_str(), t.sub.as_str()))
            .collect();
        assert_eq!(values[0], ("2", "", ""));
        assert_eq!(values[1], ("3,50", "KB", "3.500 Bytes"));
        assert_eq!(values[2], ("42,0", "s", "2.000000 bis 44.000000 s"));
        assert_eq!(values[3], ("0,0", "/s", ""));
        assert_eq!(values[4], ("667", "b/s", ""));
        let packets = [(0, 100, false), (125 * s, 100, false)];
        let long = tiles(Lang::De, &totals_of(&packets), 0);
        assert_eq!(
            (long[2].value.as_str(), long[2].unit.as_str()),
            ("2:05", "min")
        );
    }

    #[test]
    fn multicast_dns_stands_right_below_udp() {
        assert_eq!(
            hierarchy_path(&["eth", "ip", "udp", "dns", "mdns"]),
            ["eth", "ip", "udp", "mdns"]
        );
        assert_eq!(
            hierarchy_path(&["eth", "ip", "udp", "dns"]),
            ["eth", "ip", "udp", "dns"]
        );
        assert_eq!(
            protocol_names("ip"),
            ("IPv4", "Internet Protocol Version 4")
        );
    }

    #[test]
    fn times_and_filters() {
        assert_eq!(millis(Lang::De, 12_345.0), "12,3 ms");
        assert_eq!(millis(Lang::En, 250.0), "0.250 ms");
        assert_eq!(millis(Lang::De, 212_600.0), "212,6 ms");
        assert_eq!(seconds(12_345_678), "12.345678");
        assert_eq!(and_filter("", "tcp"), "tcp");
        assert_eq!(and_filter(" dns or tcp ", "tcp"), "(dns or tcp) and tcp");
    }

    #[test]
    #[serial]
    fn the_connection_filter_is_no_display_filter() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        let index = FilterIndex::default();
        let page = statistics(Lang::En, &engine, &index, "", "country == US");
        assert!(page.error.is_none());
        let page = statistics(Lang::En, &engine, &index, "", "country ==");
        assert!(page.error.is_some());
    }
}
