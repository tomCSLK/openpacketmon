//! Statistics of the stored packets, for all of them or those of a display filter: protocol
//! hierarchy, findings of the TCP analysis, response times and a traffic chart.

use std::collections::HashMap;

use askama::Template;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Serialize;

use crate::engine::analysis::{self, PacketAnalysis, flag};
use crate::filter::Filter;
use crate::web::auth::Session;
use crate::web::entries::encode;
use crate::web::i18n::{self, Lang};
use crate::web::packets::{PacketQuery, filter_error};
use crate::web::pages::{Layout, render};
use crate::web::{AppState, format};

/// Most points of the chart; the seconds are put together in steps to stay below
const CHART_POINTS: i64 = 300;
/// Steps of the chart in seconds
const CHART_STEPS: [i64; 17] = [
    1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 21600, 43200, 86400,
];

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
}

/// A line of the protocol hierarchy
pub struct HierarchyRow {
    pub name: &'static str,
    pub depth: usize,
    pub packets: String,
    pub packets_share: String,
    /// Width of the bar, 0–100
    pub bar: String,
    pub bytes: String,
    pub bytes_share: String,
    pub href: String,
}

/// A finding of the TCP analysis with its number of packets
pub struct FindingRow {
    pub label: String,
    pub count: String,
    pub problem: bool,
    pub href: Option<String>,
}

/// A response time: how often, on average, at most
pub struct TimeRow {
    pub label: String,
    pub count: String,
    pub average: String,
    pub max: String,
    pub href: Option<String>,
}

/// The chart: packets and bytes per second, in steps of `step` seconds from `start`
#[derive(Serialize, Default)]
pub struct Chart {
    start: i64,
    step: i64,
    packets: Vec<f64>,
    bytes: Vec<f64>,
}

#[derive(Template)]
#[template(path = "statistics.html")]
struct StatisticsPage {
    t: Lang,
    layout: Layout,
    running: bool,
    unavailable: String,
    filter: String,
    error: Option<String>,
    /// The statistics change while packets come
    live: bool,
    summary: Vec<(String, String)>,
    hierarchy: Vec<HierarchyRow>,
    findings: Vec<FindingRow>,
    times: Vec<TimeRow>,
    /// The chart data as JSON
    chart: String,
    packets_href: String,
    empty: bool,
}

/// A filter combined with the one of the page
fn and_filter(page: &str, extra: &str) -> String {
    if page.trim().is_empty() {
        extra.to_owned()
    } else {
        format!("({}) and {extra}", page.trim())
    }
}

fn packets_link(page: &str, extra: &str) -> String {
    format!("/packets?f={}", encode(&and_filter(page, extra)))
}

/// Percent with one decimal (`12.5 %`)
fn share(t: Lang, part: u64, whole: u64) -> (String, f64) {
    #[allow(clippy::cast_precision_loss)]
    let value = if whole == 0 {
        0.0
    } else {
        part as f64 * 100.0 / whole as f64
    };
    (
        format!("{value:.1} %").replace('.', &t.decimal_separator().to_string()),
        value,
    )
}

/// A duration: seconds with milliseconds below a minute, otherwise hours, minutes and seconds
fn duration(t: Lang, us: i64) -> String {
    let secs = us / 1_000_000;
    if secs < 60 {
        return format!(
            "{secs}{}{:03} s",
            t.decimal_separator(),
            us % 1_000_000 / 1000
        );
    }
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    if h > 0 {
        format!("{h} h {m:02} min {s:02} s")
    } else {
        format!("{m} min {s:02} s")
    }
}

/// The protocol hierarchy: every path adds to all of its beginnings
fn hierarchy(
    t: Lang,
    totals: &Totals,
    paths: &[Vec<&'static str>],
    page_filter: &str,
) -> Vec<HierarchyRow> {
    let mut nodes: HashMap<Vec<&'static str>, (u64, u64)> = HashMap::new();
    for (path, (packets, bytes)) in &totals.paths {
        let Some(protocols) = paths.get(usize::from(*path)) else {
            continue;
        };
        for end in 1..=protocols.len() {
            let node = nodes.entry(protocols[..end].to_vec()).or_default();
            node.0 += packets;
            node.1 += bytes;
        }
    }
    let mut rows = Vec::new();
    add_children(t, &nodes, &[], totals, page_filter, &mut rows);
    rows
}

fn add_children(
    t: Lang,
    nodes: &HashMap<Vec<&'static str>, (u64, u64)>,
    parent: &[&'static str],
    totals: &Totals,
    page_filter: &str,
    rows: &mut Vec<HierarchyRow>,
) {
    let mut children: Vec<(&Vec<&'static str>, &(u64, u64))> = nodes
        .iter()
        .filter(|(path, _)| path.len() == parent.len() + 1 && path.starts_with(parent))
        .collect();
    children.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(a.0.cmp(b.0)));
    for (path, (packets, bytes)) in children {
        let name = path[path.len() - 1];
        let (packets_share, _) = share(t, *packets, totals.packets);
        let (bytes_share, bar) = share(t, *bytes, totals.bytes);
        rows.push(HierarchyRow {
            name,
            depth: parent.len(),
            packets: format::count(t, u128::from(*packets)),
            packets_share,
            bar: format!("{bar:.1}"),
            bytes: format::bytes(t, u128::from(*bytes)),
            bytes_share,
            href: packets_link(page_filter, name),
        });
        add_children(t, nodes, path, totals, page_filter, rows);
    }
}

/// Packets and bytes per second in steps
fn chart(records: &[(i64, u32)], first: i64, last: i64) -> Chart {
    let (start, end) = (first.div_euclid(1_000_000), last.div_euclid(1_000_000));
    let span = end - start + 1;
    let step = CHART_STEPS
        .iter()
        .copied()
        .find(|s| span / s < CHART_POINTS)
        .unwrap_or(86_400);
    let start = start - start.rem_euclid(step);
    let len = usize::try_from((end - start) / step + 1).unwrap_or(0);
    let mut packets = vec![0.0; len];
    let mut bytes = vec![0.0; len];
    for (time, size) in records {
        let i = usize::try_from((time.div_euclid(1_000_000) - start) / step).unwrap_or(0);
        if i < len {
            packets[i] += 1.0;
            bytes[i] += f64::from(*size);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let per_second = step as f64;
    Chart {
        start,
        step,
        packets: packets.into_iter().map(|p| p / per_second).collect(),
        bytes: bytes.into_iter().map(|b| b / per_second).collect(),
    }
}

/// The tiles: packets, bytes, time span and average rates
fn summary(t: Lang, totals: &Totals) -> Vec<(String, String)> {
    let mut summary = vec![
        (
            t.get("stats-packets"),
            format::count(t, u128::from(totals.packets)),
        ),
        (
            t.get("stats-bytes"),
            format::bytes(t, u128::from(totals.bytes)),
        ),
    ];
    if let (Some(first), Some(last)) = (totals.first, totals.last) {
        let us = (last - first).max(0);
        #[allow(clippy::cast_precision_loss)]
        let seconds = us as f64 / 1e6;
        summary.push((t.get("stats-duration"), duration(t, us)));
        if seconds > 0.0 {
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            {
                summary.push((
                    t.get("stats-packet-rate"),
                    t.get1(
                        "stats-per-second",
                        "value",
                        format::count(t, (totals.packets as f64 / seconds).round() as u128),
                    ),
                ));
                summary.push((
                    t.get("stats-bit-rate"),
                    format!(
                        "{}/s",
                        format::bits(t, (totals.bytes as f64 * 8.0 / seconds).round() as u128)
                    ),
                ));
            }
        }
    }
    summary
}

/// The findings of the TCP analysis and the response times, with links to their packets
fn analysis_rows(t: Lang, totals: &Totals, page_filter: &str) -> (Vec<FindingRow>, Vec<TimeRow>) {
    let findings = FINDINGS
        .iter()
        .zip(totals.flags)
        .map(|((bit, id, field), count)| FindingRow {
            label: t.get(id),
            count: format::count(t, u128::from(count)),
            problem: bit & flag::PROBLEMS != 0 || *bit == flag::RESET,
            href: (count > 0).then(|| packets_link(page_filter, field)),
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let time_row = |id: &str, times: Times, field: &str| TimeRow {
        label: t.get(id),
        count: format::count(t, u128::from(times.count)),
        average: if times.count == 0 {
            "–".to_owned()
        } else {
            format::millis(t, times.sum as f64 / times.count as f64)
        },
        max: if times.count == 0 {
            "–".to_owned()
        } else {
            format::millis(t, f64::from(times.max))
        },
        href: (times.count > 0).then(|| packets_link(page_filter, field)),
    };
    let times = vec![
        time_row(
            "stats-handshake",
            totals.handshake,
            "tcp.analysis.handshake",
        ),
        time_row("stats-ack-rtt", totals.ack_rtt, "tcp.analysis.ack_rtt"),
        time_row("stats-dns-time", totals.dns, "dns.time"),
    ];
    (findings, times)
}

/// `GET /statistics?f=…`
pub async fn page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PacketQuery>,
) -> Response {
    let t = i18n::current();
    let (layout, running, unavailable, store, finished) = {
        let engine = state.engine.lock();
        (
            Layout::new(&engine, &session),
            engine.is_running(),
            crate::web::packets::unavailable(t, &engine),
            engine.packets.clone(),
            engine.traffic_history.no_more_packets(),
        )
    };
    let parsed = Filter::parse(&query.f);
    let error = parsed.as_ref().err().map(|e| filter_error(t, e));
    let filter = parsed.ok().flatten();
    let page_filter = query.f.trim().to_owned();

    let mut totals = Totals::default();
    let mut points = Vec::new();
    let mut paths = Vec::new();
    if let (Some(store), None) = (&store, &error) {
        let numbers: Option<Vec<u64>> = filter.as_ref().map(|filter| {
            state
                .packet_index
                .with_matches(store, &page_filter, filter, |m| m.iter().copied().collect())
        });
        let last = store.lock().last_number();
        analysis::advance(store, last);
        let analysis = store.analysis();
        let mut add = |r: &PacketAnalysis| {
            totals.add(r);
            points.push((r.time, r.len));
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

    let chart = match (totals.first, totals.last) {
        (Some(first), Some(last)) => chart(&points, first, last),
        _ => Chart::default(),
    };
    let summary = summary(t, &totals);
    let (findings, times) = analysis_rows(t, &totals, &page_filter);
    render(&StatisticsPage {
        t,
        layout,
        running,
        live: running && !finished && store.is_some() && error.is_none(),
        unavailable,
        filter: query.f.clone(),
        error,
        summary,
        hierarchy: hierarchy(t, &totals, &paths, &page_filter),
        findings,
        times,
        chart: serde_json::to_string(&chart)
            .unwrap_or_else(|_| "{}".to_owned())
            .replace('<', "\\u003c"),
        packets_href: if page_filter.is_empty() {
            "/packets".to_owned()
        } else {
            format!("/packets?f={}", encode(&page_filter))
        },
        empty: totals.packets == 0,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::web::testutil::*;

    #[test]
    fn the_chart_puts_seconds_together_in_steps() {
        let s = 1_000_000;
        let c = chart(
            &[(10 * s, 100), (10 * s + 5, 100), (12 * s, 50)],
            10 * s,
            12 * s,
        );
        assert_eq!((c.start, c.step), (10, 1));
        assert_eq!(c.packets, [2.0, 0.0, 1.0]);
        assert_eq!(c.bytes, [200.0, 0.0, 50.0]);
        // an hour: steps of 15 seconds
        let c = chart(&[(0, 1), (3600 * s, 1)], 0, 3600 * s);
        assert_eq!(c.step, 15);
        assert!(c.packets.len() < CHART_POINTS as usize);
        assert!((c.packets[0] - 1.0 / 15.0).abs() < 1e-9);
    }

    #[test]
    fn durations_and_times() {
        assert_eq!(duration(Lang::De, 12_345_678), "12,345 s");
        assert_eq!(duration(Lang::En, 125_000_000), "2 min 05 s");
        assert_eq!(duration(Lang::En, 3_725_000_000), "1 h 02 min 05 s");
        assert_eq!(format::millis(Lang::De, 12_345.0), "12,3 ms");
        assert_eq!(format::millis(Lang::En, 250.0), "0.250 ms");
        assert_eq!(share(Lang::De, 1, 8).0, "12,5 %");
    }

    #[test]
    fn filters_are_combined_with_the_one_of_the_page() {
        assert_eq!(and_filter("", "tcp"), "tcp");
        assert_eq!(and_filter(" dns or tcp ", "tcp"), "(dns or tcp) and tcp");
    }

    #[tokio::test]
    async fn the_page_answers_without_a_capture() {
        let h = harness().await;
        let reply = send(&h.app, get("/statistics?f=tcp", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        let reply = send(&h.app, get("/statistics?f=tcp.port%20==", Some(&h.cookie))).await;
        assert!(reply.body.contains("filter-error"), "{}", reply.body);
    }
}
