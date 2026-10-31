//! The connections of the capture: a searchable, sortable list in pages and the details of one
//! connection (with a latency measurement for live captures).

use std::fmt;
use std::net::IpAddr;

use sniffnet_packet_parser::Protocol;

use crate::config::favorite::FavoriteItem;
use crate::engine::state::EngineState;
use crate::filter::Filter;
use crate::networking::manage_packets::get_address_to_lookup;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_info_host::DataInfoHost;
use crate::networking::types::data_representation::{DataRepr, DataUnit};
use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
use crate::networking::types::latency::LatencyStatus;
use crate::networking::types::program::Program;
use crate::networking::types::service::Service;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::report::types::sort_type::SortType;
use crate::utils::types::timestamp::Timestamp;
use crate::view::entries::{host_sub, program_label, service_label};
use crate::view::i18n::Lang;
use crate::view::packets::{conversation_filter, filter_error};
use crate::view::{Jump, format};

/// Page sizes offered in the list
pub(crate) const PAGE_SIZES: [usize; 4] = [25, 50, 100, 250];
pub(crate) const DEFAULT_PAGE_SIZE: usize = 50;

/// Examples of the help of the connection filter
pub const FILTER_EXAMPLES: [&str; 8] = [
    "ip.addr == 192.168.1.20",
    "port == 443 and proto == tcp",
    "domain contains google",
    "country == US and not as contains amazon",
    "program contains firefox",
    "remote == 10.0.0.0/8",
    "favorite and bytes > 100000",
    "bytes > 1000000 and direction == outgoing",
];

/// The names the connection filter knows besides the protocols
pub fn filter_field_names() -> Vec<&'static str> {
    crate::filter::CONNECTION_FIELDS
        .iter()
        .map(|(name, _, _)| *name)
        .collect()
}

/// A connection in the list
#[allow(clippy::struct_excessive_bools)] // flags of a table row
pub struct ConnectionRow {
    pub jump: Jump,
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
    pub last: Timestamp,
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
        jump: Jump::Connection(*key),
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
        last: info.final_timestamp,
    }
}

/// Which part of the connections a list shows
pub struct ListRequest<'a> {
    pub filter: &'a str,
    pub sort: SortType,
    pub page: Option<usize>,
    /// Rows per page, if not the size of the settings
    pub size: Option<usize>,
}

/// A page of the list of connections
pub struct ConnectionList {
    pub running: bool,
    /// What is wrong with the filter; a wrong filter shows no connections rather than all of them
    pub error: Option<String>,
    pub rows: Vec<ConnectionRow>,
    pub total: usize,
    /// Volume of all connections of the filter
    pub matching_data: String,
    /// The page shown (the last one if the one asked for is beyond it) and how many there are
    pub page: usize,
    pub pages: usize,
    /// Numbers of the first and the last row among all connections of the filter
    pub first_row: usize,
    pub last_row: usize,
    /// The page sizes offered and which one is in use
    pub sizes: Vec<(usize, bool)>,
    /// The capture source knows the programs
    pub programs: bool,
    pub unit: DataUnit,
}

pub fn connection_list(t: Lang, engine: &EngineState, request: &ListRequest) -> ConnectionList {
    let size = request
        .size
        .filter(|s| PAGE_SIZES.contains(s))
        .or_else(|| Some(engine.conf.page_size).filter(|s| PAGE_SIZES.contains(s)))
        .unwrap_or(DEFAULT_PAGE_SIZE);
    let unit = engine.conf.data_repr.data_unit;
    let repr = DataRepr {
        data_unit: unit,
        per_second: false,
    };
    let parsed = Filter::parse_connections(request.filter.trim());
    let error = parsed.as_ref().err().map(|e| filter_error(t, e));
    let filter = parsed.ok().flatten();
    let requested = request.page.unwrap_or(1).max(1);
    let (mut entries, total, matching) = if error.is_some() {
        (Vec::new(), 0, DataInfo::default())
    } else {
        engine.searched_entries(filter.as_ref(), request.sort, repr, requested, size)
    };
    let pages = total.div_ceil(size).max(1);
    let page = requested.min(pages);
    if page != requested {
        // beyond the last page (e.g. after the list got shorter): show the last one
        entries = engine
            .searched_entries(filter.as_ref(), request.sort, repr, page, size)
            .0;
    }
    let rows: Vec<ConnectionRow> = entries
        .iter()
        .map(|(key, info)| connection_row(t, engine, key, info))
        .collect();
    let first_row = if total == 0 { 0 } else { (page - 1) * size + 1 };
    let last_row = first_row + rows.len().saturating_sub(1);
    ConnectionList {
        running: engine.is_running(),
        error,
        rows,
        total,
        matching_data: format::amount(t, unit, matching.tot_data(repr)),
        page,
        pages,
        first_row,
        last_row,
        sizes: PAGE_SIZES.iter().map(|s| (*s, *s == size)).collect(),
        programs: engine.capture_source.supports_programs(),
        unit,
    }
}

/// A jump from a value of the details page to the list it belongs to
pub struct FactLink {
    pub label: String,
    /// Says where the link leads
    pub title: String,
    pub jump: Jump,
}

/// The value of a line of the details page
pub enum FactValue {
    Text(String),
    Time(Timestamp),
}

impl fmt::Display for FactValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Time(ts) => f.write_str(&format::time_utc(*ts)),
        }
    }
}

/// A labelled value of the details page
pub struct Fact {
    pub label: String,
    pub value: FactValue,
    pub links: Vec<FactLink>,
}

impl Fact {
    fn text(label: String, value: String) -> Self {
        Self {
            label,
            value: FactValue::Text(value),
            links: Vec::new(),
        }
    }

    fn time(label: String, ts: Timestamp) -> Self {
        Self {
            label,
            value: FactValue::Time(ts),
            links: Vec::new(),
        }
    }

    fn with_links(mut self, links: Vec<FactLink>) -> Self {
        self.links = links;
        self
    }
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
            jump: Jump::Connections { filter },
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
    let filter = conversation_filter(key);
    vec![
        FactLink {
            label: t.get("connection-packets"),
            title: t.get("connection-packets-hint"),
            jump: Jump::Packets {
                filter: filter.clone(),
                connections: String::new(),
            },
        },
        FactLink {
            label: t.get("connection-statistics"),
            title: t.get("connection-statistics-hint"),
            jump: Jump::Statistics {
                filter,
                connections: String::new(),
            },
        },
    ]
}

/// The details of one connection
#[allow(clippy::struct_excessive_bools)] // what the page may offer
pub struct ConnectionDetail {
    pub title: String,
    pub facts: Vec<Fact>,
    pub remote: IpAddr,
    pub host_facts: Vec<Fact>,
    pub message_types: Vec<(String, String)>,
    pub latency: Option<String>,
    pub latency_supported: bool,
    pub measuring: bool,
    /// The connections of the remote address
    pub address_jump: Jump,
    pub remote_favorite: bool,
}

pub fn connection_detail(
    t: Lang,
    engine: &EngineState,
    key: &AddressPortPair,
    info: &InfoAddressPortPair,
) -> ConnectionDetail {
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
    let host_facts = remote_host(t, engine, remote);
    let mut message_types: Vec<(String, String)> = info
        .message_types
        .iter()
        .map(|(m, n)| (m.to_string(), format::count(t, *n as u128)))
        .collect();
    message_types.sort();
    let status = engine.latency_statuses.get(&remote);
    ConnectionDetail {
        title: format!(
            "{} → {}",
            format::socket(&key.src_ip, key.src_port),
            format::socket(&key.dst_ip, key.dst_port)
        ),
        facts,
        remote,
        host_facts,
        message_types,
        latency: latency_text(t, status),
        latency_supported: engine.capture_source.supports_latency()
            && !matches!(key.protocol, Protocol::Arp),
        measuring: matches!(status, Some(LatencyStatus::Measuring)),
        address_jump: Jump::Connections {
            filter: format!("ip.addr == {remote}"),
        },
        remote_favorite: engine.conf.favorites.contains_address(&remote),
    }
}

/// Whether the latency to `ip` may be measured: only addresses of the capture, and only where the
/// source allows it
pub fn latency_allowed(engine: &EngineState, ip: IpAddr) -> bool {
    engine.capture_source.supports_latency()
        && engine
            .info_traffic
            .map
            .iter()
            .any(|(k, v)| get_address_to_lookup(k, v.traffic_direction) == ip)
}
