//! The connections of the capture: a searchable, sortable list in pages and the details of one
//! connection (with a latency measurement for live captures).

use std::fmt::Write;
use std::net::IpAddr;

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;
use sniffnet_packet_parser::Protocol;

use crate::config::favorite::FavoriteItem;
use crate::engine::state::EngineState;
use crate::filter::Filter;
use crate::networking::manage_packets::get_address_to_lookup;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_info_host::DataInfoHost;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
use crate::networking::types::latency::LatencyStatus;
use crate::networking::types::program::Program;
use crate::networking::types::service::Service;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::report::types::sort_type::SortType;
use crate::web::auth::Session;
use crate::web::entries::{encode, host_sub, program_label, service_label};
use crate::web::i18n::{self, Lang};
use crate::web::packets::{filter_error, filters_href};
use crate::web::pager::Pager;
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError, format};

/// Page sizes offered in the list
pub(crate) const PAGE_SIZES: [usize; 4] = [25, 50, 100, 250];
pub(crate) const DEFAULT_PAGE_SIZE: usize = 50;

/// The filter and position of the list, as given in the address
#[derive(Deserialize, Default, Clone)]
pub struct ListQuery {
    /// Connection filter (`ip.addr == 192.168.1.20 and port == 443`)
    #[serde(default)]
    f: String,
    /// `asc`, `desc` (by volume) or empty (most recent first)
    #[serde(default)]
    sort: String,
    page: Option<usize>,
    size: Option<usize>,
}

impl ListQuery {
    fn sort_type(&self) -> SortType {
        match self.sort.as_str() {
            "asc" => SortType::Ascending,
            "desc" => SortType::Descending,
            _ => SortType::Neutral,
        }
    }

    /// The address of this list with another page
    fn href(&self, page: usize) -> String {
        let mut pairs = Vec::new();
        if !self.f.trim().is_empty() {
            pairs.push(format!("f={}", encode(self.f.trim())));
        }
        if !self.sort.is_empty() {
            pairs.push(format!("sort={}", encode(&self.sort)));
        }
        if let Some(size) = self.size {
            pairs.push(format!("size={size}"));
        }
        if page > 1 {
            pairs.push(format!("page={page}"));
        }
        if pairs.is_empty() {
            "/connections".to_owned()
        } else {
            format!("/connections?{}", pairs.join("&"))
        }
    }

    /// The filter and sorting as hidden fields of the page size and page forms
    fn hidden_fields(&self) -> Vec<(&'static str, String)> {
        [("f", self.f.trim()), ("sort", self.sort.as_str())]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(n, v)| (n, v.to_owned()))
            .collect()
    }

    fn is_filtered(&self) -> bool {
        !self.f.trim().is_empty()
    }
}

/// Examples of the help of the connection filter
const FILTER_EXAMPLES: [&str; 8] = [
    "ip.addr == 192.168.1.20",
    "port == 443 and proto == tcp",
    "domain contains google",
    "country == US and not as contains amazon",
    "program contains firefox",
    "remote == 10.0.0.0/8",
    "favorite and bytes > 100000",
    "bytes > 1000000 and direction == outgoing",
];

/// A connection in the list
#[allow(clippy::struct_excessive_bools)] // flags of a table row
pub struct ConnectionRow {
    pub href: String,
    pub src: String,
    pub dst: String,
    pub outgoing: bool,
    pub proto: String,
    pub service: String,
    pub program: String,
    /// Service or program do not apply (ARP, ICMP …): the table shows a dash with the text as tooltip
    pub service_na: bool,
    pub program_na: bool,
    /// Name and AS of the remote host, if resolved
    pub host: String,
    pub host_sub: String,
    pub packets: String,
    pub data: String,
    pub last: String,
}

/// Query string that identifies a connection on the details page
pub(crate) fn connection_query(key: &AddressPortPair) -> String {
    let mut query = format!(
        "src={}&dst={}&proto={}",
        encode(&key.src_ip.to_string()),
        encode(&key.dst_ip.to_string()),
        encode(&key.protocol.to_string())
    );
    if let Some(port) = key.src_port {
        let _ = write!(query, "&sport={port}");
    }
    if let Some(port) = key.dst_port {
        let _ = write!(query, "&dport={port}");
    }
    if let Some(exporter) = key.exporter {
        let _ = write!(
            query,
            "&exporter={}&odid={}",
            encode(&exporter.addr.to_string()),
            exporter.observation_domain_id
        );
    }
    query
}

fn connection_row(
    t: Lang,
    state: &EngineState,
    key: &AddressPortPair,
    info: &InfoAddressPortPair,
) -> ConnectionRow {
    let unit = state.conf.data_repr.data_unit;
    let remote = get_address_to_lookup(key, info.traffic_direction);
    let (host, host_sub) = match state.addresses_resolved.get(&remote) {
        Some((rdns, host)) => (
            if host.domain.is_empty() {
                rdns.clone()
            } else {
                host.domain.clone()
            },
            self::host_sub(host),
        ),
        None => (String::new(), String::new()),
    };
    ConnectionRow {
        href: format!("/connection?{}", connection_query(key)),
        src: format::socket(&key.src_ip, key.src_port),
        dst: format::socket(&key.dst_ip, key.dst_port),
        outgoing: info.traffic_direction == TrafficDirection::Outgoing,
        proto: key.protocol.to_string(),
        service: service_label(t, &info.service),
        program: program_label(t, &info.program),
        service_na: info.service == Service::NotApplicable,
        program_na: info.program == Program::NotApplicable,
        host,
        host_sub,
        packets: format::count(t, info.packets),
        data: format::amount(
            t,
            unit,
            info.transmitted_data(DataRepr {
                data_unit: unit,
                per_second: false,
            }),
        ),
        last: format::time_html(info.final_timestamp),
    }
}

#[derive(Template)]
#[template(path = "connections.html")]
#[allow(clippy::struct_excessive_bools)] // switches of a page or a command line
struct ConnectionsPage {
    t: Lang,
    layout: Layout,
    running: bool,
    query: ListQuery,
    hidden_filters: Vec<(&'static str, String)>,
    filtered: bool,
    error: Option<String>,
    examples: [&'static str; 8],
    protocols: String,
    fields: String,
    rows: Vec<ConnectionRow>,
    total: usize,
    matching_data: String,
    pages: usize,
    first_row: usize,
    last_row: usize,
    pager: Pager,
    sizes: Vec<(usize, bool)>,
    programs: bool,
    unit: &'static str,
    back: String,
    /// The statistics of the packets of the connections of the filter
    statistics: String,
}

/// `GET /connections`
pub async fn list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let size = query
        .size
        .filter(|s| PAGE_SIZES.contains(s))
        .or_else(|| Some(engine.conf.page_size).filter(|s| PAGE_SIZES.contains(s)))
        .unwrap_or(DEFAULT_PAGE_SIZE);
    let unit = engine.conf.data_repr.data_unit;
    let repr = DataRepr {
        data_unit: unit,
        per_second: false,
    };
    let parsed = Filter::parse_connections(query.f.trim());
    let error = parsed.as_ref().err().map(|e| filter_error(t, e));
    // a wrong filter shows no connections rather than all of them
    let filter = parsed.ok().flatten();
    let requested = query.page.unwrap_or(1).max(1);
    let (mut entries, total, matching) = if error.is_some() {
        (Vec::new(), 0, DataInfo::default())
    } else {
        engine.searched_entries(filter.as_ref(), query.sort_type(), repr, requested, size)
    };
    let pages = total.div_ceil(size).max(1);
    let page = requested.min(pages);
    if page != requested {
        // beyond the last page (e.g. after the list got shorter): show the last one
        entries = engine
            .searched_entries(filter.as_ref(), query.sort_type(), repr, page, size)
            .0;
    }
    let rows: Vec<ConnectionRow> = entries
        .iter()
        .map(|(key, info)| connection_row(t, &engine, key, info))
        .collect();
    let first_row = if total == 0 { 0 } else { (page - 1) * size + 1 };
    let last_row = first_row + rows.len().saturating_sub(1);
    render(&ConnectionsPage {
        t,
        layout: Layout::new(&engine, &session),
        running: engine.is_running(),
        filtered: query.is_filtered(),
        hidden_filters: query.hidden_fields(),
        error,
        examples: FILTER_EXAMPLES,
        protocols: crate::filter::CONNECTION_PROTOCOLS.join(" "),
        fields: crate::filter::CONNECTION_FIELDS
            .iter()
            .map(|(name, _, _)| *name)
            .collect::<Vec<_>>()
            .join(" "),
        rows,
        total,
        matching_data: format::amount(t, unit, matching.tot_data(repr)),
        pages,
        first_row,
        last_row,
        pager: Pager::new(page, pages, |p| query.href(p), "/connections", {
            let mut fields = query.hidden_fields();
            if let Some(size) = query.size {
                fields.push(("size", size.to_string()));
            }
            fields
        }),
        sizes: PAGE_SIZES.iter().map(|s| (*s, *s == size)).collect(),
        programs: engine.capture_source.supports_programs(),
        unit: crate::web::api::unit_code(unit),
        back: query.href(page),
        statistics: filters_href("/statistics", "", &query.f),
        query,
    })
}

#[derive(Deserialize)]
pub struct ConnectionQuery {
    #[serde(default)]
    src: String,
    #[serde(default)]
    dst: String,
    sport: Option<u16>,
    dport: Option<u16>,
    #[serde(default)]
    proto: String,
    exporter: Option<String>,
    odid: Option<u32>,
}

/// The connection named by the details address
fn find_connection<'a>(
    state: &'a EngineState,
    q: &ConnectionQuery,
) -> Option<(&'a AddressPortPair, &'a InfoAddressPortPair)> {
    let src: IpAddr = q.src.parse().ok()?;
    let dst: IpAddr = q.dst.parse().ok()?;
    let exporter: Option<IpAddr> = match &q.exporter {
        Some(e) => Some(e.parse().ok()?),
        None => None,
    };
    state.info_traffic.map.iter().find(|(key, _)| {
        key.src_ip == src
            && key.dst_ip == dst
            && key.src_port == q.sport
            && key.dst_port == q.dport
            && key.protocol.to_string() == q.proto
            && key.exporter.map(|e| e.addr) == exporter
            && key.exporter.map(|e| e.observation_domain_id) == q.odid
    })
}

/// A jump from a value of the details page to the list it belongs to
pub struct FactLink {
    pub label: String,
    /// Says where the link leads
    pub title: String,
    pub href: String,
}

/// A labelled value of the details page
pub struct Fact {
    pub label: String,
    pub value: String,
    /// `value` is markup made by this program (a time), not text
    pub is_html: bool,
    pub links: Vec<FactLink>,
}

impl Fact {
    fn text(label: String, value: String) -> Self {
        Self {
            label,
            value,
            is_html: false,
            links: Vec::new(),
        }
    }

    fn time(label: String, ts: crate::utils::types::timestamp::Timestamp) -> Self {
        Self {
            label,
            value: format::time_html(ts),
            is_html: true,
            links: Vec::new(),
        }
    }

    fn with_links(mut self, links: Vec<FactLink>) -> Self {
        self.links = links;
        self
    }
}

#[derive(Template)]
#[template(path = "connection.html")]
#[allow(clippy::struct_excessive_bools)] // switches of a page or a command line
struct ConnectionPage {
    t: Lang,
    layout: Layout,
    title: String,
    facts: Vec<Fact>,
    remote: String,
    host_facts: Vec<Fact>,
    message_types: Vec<(String, String)>,
    latency: Option<String>,
    latency_supported: bool,
    measuring: bool,
    self_query: String,
    /// Connections of the remote address
    address_search: String,
    remote_favorite: bool,
}

fn latency_text(t: Lang, status: Option<&LatencyStatus>) -> Option<String> {
    Some(match status? {
        LatencyStatus::Measuring => t.get("latency-measuring"),
        LatencyStatus::Measured(d) => {
            #[allow(clippy::cast_precision_loss)]
            let ms = d.as_micros() as f64 / 1000.0;
            t.get1("latency-ms", "ms", (ms * 100.0).round() / 100.0)
        }
        LatencyStatus::Failed(e) => t.get1("latency-failed", "error", e.as_str()),
    })
}

/// Name, AS and country of the remote host, with a link to its connections
fn remote_host(t: Lang, engine: &EngineState, remote: IpAddr) -> Vec<Fact> {
    let Some((rdns, host)) = engine.addresses_resolved.get(&remote) else {
        return Vec::new();
    };
    let filter = FavoriteItem::Host((host.clone(), DataInfoHost::default())).connection_filter();
    let mut facts = vec![
        Fact::text(t.get("host-rdns"), rdns.clone()).with_links(vec![FactLink {
            label: t.get("connection-list"),
            title: t.get("connection-host-connections"),
            href: format!(
                "/connections?{}",
                crate::web::entries::search_query(&filter)
            ),
        }]),
    ];
    if !host.asn.name.is_empty() {
        facts.push(Fact::text(
            t.get("host-asn"),
            format!("AS{} {}", host.asn.code, host.asn.name),
        ));
    }
    if host.country != crate::countries::types::country::Country::ZZ {
        facts.push(Fact::text(t.get("host-country"), host.country.to_string()));
    }
    facts
}

/// Links from the number of packets of a connection to the packets and their statistics, with
/// the display filter of the connection
fn packet_links(t: Lang, key: &AddressPortPair) -> Vec<FactLink> {
    let filter = encode(&crate::web::packets::conversation_filter(key));
    vec![
        FactLink {
            label: t.get("connection-packets"),
            title: t.get("connection-packets-hint"),
            href: format!("/packets?f={filter}"),
        },
        FactLink {
            label: t.get("connection-statistics"),
            title: t.get("connection-statistics-hint"),
            href: format!("/statistics?f={filter}"),
        },
    ]
}

/// `GET /connection?src=…`: details of one connection
pub async fn detail(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ConnectionQuery>,
) -> Result<Response, WebError> {
    let t = i18n::current();
    let engine = state.engine.lock();
    let (key, info) = find_connection(&engine, &query).ok_or(WebError::NotFound)?;
    let remote = get_address_to_lookup(key, info.traffic_direction);
    let data = info.data_info();
    let bytes = DataRepr::bytes(false);
    let packet_links = if engine.packets.is_some() {
        packet_links(t, key)
    } else {
        Vec::new()
    };
    let mut facts = vec![
        Fact::text(
            t.get("conn-source"),
            format::socket(&key.src_ip, key.src_port),
        ),
        Fact::text(t.get("conn-source-mac"), format::mac(info.src_mac)),
        Fact::text(
            t.get("conn-destination"),
            format::socket(&key.dst_ip, key.dst_port),
        ),
        Fact::text(t.get("conn-destination-mac"), format::mac(info.dst_mac)),
        Fact::text(t.get("conn-protocol"), key.protocol.to_string()),
        Fact::text(
            t.get("conn-direction"),
            t.get(match info.traffic_direction {
                TrafficDirection::Outgoing => "direction-out",
                TrafficDirection::Incoming => "direction-in",
            }),
        ),
        Fact::text(t.get("conn-service"), service_label(t, &info.service)),
        Fact::text(
            t.get("conn-program"),
            match &info.program {
                Program::NamePath((name, path)) => {
                    format!("{name} ({path})")
                }
                other => program_label(t, other),
            },
        ),
        Fact::text(t.get("conn-packets"), format::count(t, info.packets)).with_links(packet_links),
        Fact::text(t.get("conn-data"), format::bytes(t, data.tot_data(bytes))),
        Fact::time(t.get("conn-first"), info.initial_timestamp),
        Fact::time(t.get("conn-last"), info.final_timestamp),
    ];
    if let Some(vlan) = info.vlan_id {
        facts.push(Fact::text(t.get("conn-vlan"), vlan.to_string()));
    }
    if let Some(exporter) = key.exporter {
        facts.push(Fact::text(t.get("conn-exporter"), exporter.to_string()));
    }
    let host_facts = remote_host(t, &engine, remote);
    let mut message_types: Vec<(String, String)> = info
        .message_types
        .iter()
        .map(|(m, n)| (m.to_string(), format::count(t, *n as u128)))
        .collect();
    message_types.sort();
    let status = engine.latency_statuses.get(&remote);
    let page = ConnectionPage {
        t,
        layout: Layout::new(&engine, &session),
        title: format!(
            "{} → {}",
            format::socket(&key.src_ip, key.src_port),
            format::socket(&key.dst_ip, key.dst_port)
        ),
        facts,
        remote: remote.to_string(),
        host_facts,
        message_types,
        latency: latency_text(t, status),
        latency_supported: engine.capture_source.supports_latency()
            && !matches!(key.protocol, Protocol::Arp),
        measuring: matches!(status, Some(LatencyStatus::Measuring)),
        self_query: connection_query(key),
        address_search: crate::web::entries::search_query(&format!("ip.addr == {remote}")),
        remote_favorite: engine.conf.favorites.contains_address(&remote),
    };
    Ok(render(&page))
}

#[derive(Deserialize)]
pub struct LatencyForm {
    #[serde(default)]
    csrf_token: String,
    #[serde(default)]
    ip: String,
    #[serde(default)]
    back: String,
}

/// `POST /connection/latency`: measures the round trip time to the remote address of a connection
pub async fn latency(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LatencyForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    let ip: IpAddr = form.ip.parse().map_err(|_| WebError::NotFound)?;
    {
        let engine = state.engine.lock();
        // only addresses of the capture, and only where the source allows it
        let known = engine
            .info_traffic
            .map
            .iter()
            .any(|(k, v)| get_address_to_lookup(k, v.traffic_direction) == ip);
        if !known || !engine.capture_source.supports_latency() {
            return Err(WebError::NotFound);
        }
    }
    state.engine.measure_latency(ip);
    Ok(Redirect::to(&crate::web::capture::back_path(
        &form.back,
        "/connections",
    )))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::web::testutil::*;

    #[test]
    fn list_addresses_keep_the_filters() {
        let query = ListQuery {
            f: " domain == example.com ".into(),
            sort: "desc".into(),
            size: Some(100),
            ..ListQuery::default()
        };
        assert_eq!(
            query.href(3),
            "/connections?f=domain%20%3D%3D%20example.com&sort=desc&size=100&page=3"
        );
        assert_eq!(ListQuery::default().href(1), "/connections");
        assert!(query.is_filtered());
        assert!(!ListQuery::default().is_filtered());
    }

    #[tokio::test]
    async fn list_and_unknown_connections() {
        let h = harness().await;
        let reply = send(
            &h.app,
            get("/connections?f=tcp&page=9&size=7", Some(&h.cookie)),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK);
        let reply = send(
            &h.app,
            get(
                "/connection?src=192.0.2.1&dst=192.0.2.2&proto=TCP&sport=1&dport=2",
                Some(&h.cookie),
            ),
        )
        .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let form = format!("csrf_token={}&ip=192.0.2.2", h.csrf);
        let reply = send(&h.app, post("/connection/latency", &h.cookie, &form)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }
}
