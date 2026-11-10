//! The connections of the capture: a searchable, sortable list in pages and the details of one
//! connection (with a latency measurement for live captures).

use std::net::IpAddr;

use sniffnet_packet_parser::Protocol;

use crate::config::favorite::{FavoriteItem, FavoriteKey};
use crate::countries::types::country::Country;
use crate::engine::state::EngineState;
use crate::filter::Filter;
use crate::networking::manage_packets::get_address_to_lookup;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_info_host::DataInfoHost;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::host::Host;
use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
use crate::networking::types::latency::LatencyStatus;
use crate::networking::types::program::Program;
use crate::networking::types::service::Service;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::networking::types::traffic_type::TrafficType;
use crate::report::types::sort_type::SortType;
use crate::utils::types::timestamp::Timestamp;
use crate::view::entries::{category_id, program_label, service_label};
use crate::view::i18n::Lang;
use crate::view::packets::{conversation_filter, filter_error};
use crate::view::{Jump, format};

/// Page sizes offered in the settings (`Verbindungen pro Seite`)
pub(crate) const PAGE_SIZES: [usize; 3] = [30, 50, 100];
pub(crate) const DEFAULT_PAGE_SIZE: usize = 30;

/// Examples of the help of the connection filter
pub const FILTER_EXAMPLES: [&str; 8] = [
    "ip.addr == 192.0.2.40",
    "port == 443",
    "domain contains example",
    "program == firefox",
    "direction == incoming",
    "country == US",
    "favorite",
    "tcp and bytes > 1000000",
];

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
    /// Service or program do not apply (ARP, ICMP …): the table shows a dash
    pub service_na: bool,
    pub program_na: bool,
    /// The remote host: its name (its address while it has none), AS or labels, country
    pub host: String,
    pub host_sub: String,
    pub country: String,
    /// The host, its address or its program is a favorite (the star of the row)
    pub favorite: bool,
    /// What the star changes: the favorites that apply when it is on, else the host (or the
    /// address while the host is unknown)
    pub favorite_keys: Vec<FavoriteKey>,
    pub packets: String,
    pub data: String,
    /// Incoming and outgoing in the unit of the list, for the bar
    pub bar_in: u128,
    pub bar_out: u128,
    /// Incoming and outgoing in bytes, under the bar
    pub bar_hint: String,
    pub last: Timestamp,
}

/// Labels of a host: local, loopback, bogon, multicast, broadcast
pub fn host_tags(t: Lang, info: &DataInfoHost) -> Vec<String> {
    let mut tags = Vec::new();
    if info.is_loopback {
        tags.push(t.get("tag-loopback"));
    } else if info.is_local {
        tags.push(t.get("tag-local"));
    }
    if let Some(bogon) = info.is_bogon {
        tags.push(t.get1("tag-bogon", "kind", bogon));
    }
    match info.traffic_type {
        TrafficType::Multicast => tags.push(t.get("tag-multicast")),
        TrafficType::Broadcast => tags.push(t.get("tag-broadcast")),
        TrafficType::Unicast => {}
    }
    tags
}

/// The AS of a host as `AS64496 Example Transit`, else its labels
fn as_or_tags(t: Lang, state: &EngineState, host: &Host) -> String {
    if host.asn.name.is_empty() {
        state
            .info_traffic
            .hosts
            .get(host)
            .map(|info| host_tags(t, info).join(", "))
            .unwrap_or_default()
    } else {
        format!("AS{} {}", host.asn.code, host.asn.name)
    }
}

/// The country code of a host, empty while it is unknown
fn country(host: &Host) -> String {
    if host.country == Country::ZZ {
        String::new()
    } else {
        host.country.to_string()
    }
}

/// Whether the connection belongs to a favorite, and what its star changes
fn favorite_of(
    state: &EngineState,
    remote: IpAddr,
    host: Option<&Host>,
    program: &Program,
) -> (bool, Vec<FavoriteKey>) {
    let favorites = &state.conf.favorites;
    let mut on = Vec::new();
    if let Some(host) = host
        && favorites.contains_host(host)
    {
        on.push(FavoriteKey::Host(host.clone()));
    }
    if favorites.contains_address(&remote) {
        on.push(FavoriteKey::Address(remote));
    }
    if favorites.contains_program(program) {
        on.push(FavoriteKey::Program(program.clone()));
    }
    if on.is_empty() {
        let key = host.map_or(FavoriteKey::Address(remote), |h| {
            FavoriteKey::Host(h.clone())
        });
        (false, vec![key])
    } else {
        (true, on)
    }
}

fn connection_row(
    t: Lang,
    state: &EngineState,
    key: &AddressPortPair,
    info: &InfoAddressPortPair,
) -> ConnectionRow {
    let unit = state.conf.data_repr.data_unit;
    let repr = DataRepr {
        data_unit: unit,
        per_second: false,
    };
    let remote = get_address_to_lookup(key, info.traffic_direction);
    let resolved = state.addresses_resolved.get(&remote);
    let (host, host_sub, cc) = match resolved {
        Some((rdns, host)) => (
            if host.domain.is_empty() {
                rdns.clone()
            } else {
                host.domain.clone()
            },
            as_or_tags(t, state, host),
            country(host),
        ),
        None => (remote.to_string(), String::new(), String::new()),
    };
    let (favorite, favorite_keys) =
        favorite_of(state, remote, resolved.map(|(_, h)| h), &info.program);
    let data = info.data_info();
    let bytes = DataRepr::bytes(false);
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
        country: cc,
        favorite,
        favorite_keys,
        packets: format::count(t, info.packets),
        data: format::amount(t, unit, info.transmitted_data(repr)),
        bar_in: data.incoming_data(repr),
        bar_out: data.outgoing_data(repr),
        bar_hint: t.fmt(
            "bar-hint",
            &[
                (
                    "incoming",
                    format::bytes(t, data.incoming_data(bytes)).into(),
                ),
                (
                    "outgoing",
                    format::bytes(t, data.outgoing_data(bytes)).into(),
                ),
            ],
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
    /// The page size in use
    pub size: usize,
    /// The largest connection of the filter in the unit of the list: it fills the bar
    pub largest: u128,
    /// The capture source knows the programs
    pub programs: bool,
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
    let (mut entries, total, matching, largest) = if error.is_some() {
        (Vec::new(), 0, DataInfo::default(), 0)
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
        error,
        rows,
        total,
        matching_data: format::amount(t, unit, matching.tot_data(repr)),
        page,
        pages,
        first_row,
        last_row,
        size,
        largest,
        programs: engine.capture_source.supports_programs(),
    }
}

fn latency_text(t: Lang, status: Option<&LatencyStatus>) -> Option<String> {
    Some(match status? {
        LatencyStatus::Measuring => t.get("latency-measuring"),
        LatencyStatus::Measured(d) => {
            #[allow(clippy::cast_precision_loss)]
            let ms = d.as_micros() as f64 / 1000.0;
            t.get1("latency-ms", "ms", format::decimal(t, ms, 1))
        }
        LatencyStatus::Failed(e) => t.get1("latency-failed", "error", e.as_str()),
    })
}

/// The details of one connection, as the two cards of the page show them
#[allow(clippy::struct_excessive_bools)] // what the page may offer
pub struct ConnectionDetail {
    /// `TCP`, before the title
    pub protocol: String,
    /// `192.0.2.20:39874 → 198.51.100.140:80`
    pub title: String,
    pub src: String,
    pub src_mac: String,
    pub dst: String,
    pub dst_mac: String,
    /// `TCP über IPv4`
    pub protocol_line: String,
    pub outgoing: bool,
    /// Name, and `80/tcp · Web` after it
    pub service: Option<(String, String)>,
    /// Name and path; the star changes the favorite
    pub program: Option<(String, String)>,
    pub program_favorite: bool,
    pub program_key: Option<FavoriteKey>,
    pub packets: String,
    /// The packets and the statistics of the connection, both directions; `None` without stored
    /// packets
    pub packets_jump: Option<Jump>,
    pub statistics_jump: Option<Jump>,
    /// A packet of the connection, to follow its stream (TCP and UDP)
    pub stream: Option<u64>,
    pub data: String,
    pub data_in: String,
    pub data_out: String,
    pub bar_in: u128,
    pub bar_out: u128,
    pub first: Timestamp,
    pub last: Timestamp,
    /// More lines when they apply: VLAN, exporter, message types
    pub more: Vec<(String, String)>,
    pub latency: Option<String>,
    pub latency_supported: bool,
    pub measuring: bool,
    /// The remote end
    pub remote: IpAddr,
    pub remote_favorite: bool,
    pub rdns: String,
    /// `AS64499` and its name, or the labels of the host
    pub asn: Option<(String, String)>,
    pub host_tags: String,
    pub country: String,
    /// The connections of the host and of the address
    pub host_jump: Option<Jump>,
    pub address_jump: Jump,
}

/// The first stored packet of a connection: the stream page follows its conversation
pub(crate) fn first_packet(engine: &EngineState, key: &AddressPortPair) -> Option<u64> {
    let store = engine.packets.as_ref()?;
    let store = store.lock();
    store
        .iter()
        .find(|p| p.conn.is_some_and(|(k, _)| k == *key))
        .map(|p| p.number)
}

#[allow(clippy::too_many_lines)] // one field after the other
pub fn connection_detail(
    t: Lang,
    engine: &EngineState,
    key: &AddressPortPair,
    info: &InfoAddressPortPair,
) -> ConnectionDetail {
    let remote = get_address_to_lookup(key, info.traffic_direction);
    let data = info.data_info();
    let bytes = DataRepr::bytes(false);
    let stored = engine.packets.is_some();
    let filter = conversation_filter(key);
    let outgoing = info.traffic_direction == TrafficDirection::Outgoing;
    let protocol_line = if key.protocol == Protocol::Arp {
        key.protocol.to_string()
    } else {
        t.fmt(
            "conn-over",
            &[
                ("protocol", key.protocol.to_string().into()),
                (
                    "ip",
                    if key.src_ip.is_ipv4() { "IPv4" } else { "IPv6" }.into(),
                ),
            ],
        )
    };
    let service = match &info.service {
        Service::Name(name) => {
            let port = if outgoing { key.dst_port } else { key.src_port };
            let category = t.get(category_id(info.service.category()));
            let sub = match port {
                Some(port) => format!(
                    "{port}/{} · {category}",
                    key.protocol.to_string().to_lowercase()
                ),
                None => category,
            };
            Some(((*name).to_owned(), sub))
        }
        _ => None,
    };
    let program = match &info.program {
        Program::NamePath((_, path)) => Some((program_label(t, &info.program), path.clone())),
        _ => None,
    };
    let favorites = &engine.conf.favorites;
    let resolved = engine.addresses_resolved.get(&remote);
    let mut more = Vec::new();
    if let Some(vlan) = info.vlan_id {
        more.push((t.get("conn-vlan"), vlan.to_string()));
    }
    if let Some(exporter) = key.exporter {
        more.push((t.get("conn-exporter"), exporter.to_string()));
    }
    let mut message_types: Vec<String> = info
        .message_types
        .iter()
        .map(|(m, n)| format!("{m} ({})", format::count(t, *n as u128)))
        .collect();
    message_types.sort();
    if !message_types.is_empty() {
        more.push((t.get("conn-message-types"), message_types.join(", ")));
    }
    let status = engine.latency_statuses.get(&remote);
    let streams = matches!(key.protocol, Protocol::Tcp | Protocol::Udp);
    ConnectionDetail {
        protocol: key.protocol.to_string(),
        title: format!(
            "{} → {}",
            format::socket(&key.src_ip, key.src_port),
            format::socket(&key.dst_ip, key.dst_port)
        ),
        src: format::socket(&key.src_ip, key.src_port),
        src_mac: format::mac(info.src_mac),
        dst: format::socket(&key.dst_ip, key.dst_port),
        dst_mac: format::mac(info.dst_mac),
        protocol_line,
        outgoing,
        service,
        program_favorite: favorites.contains_program(&info.program),
        program_key: program
            .is_some()
            .then(|| FavoriteKey::Program(info.program.clone())),
        program,
        packets: format::count(t, info.packets),
        packets_jump: stored.then(|| Jump::Packets {
            filter: filter.clone(),
            connections: String::new(),
        }),
        statistics_jump: stored.then(|| Jump::Statistics {
            filter,
            connections: String::new(),
        }),
        stream: if streams && stored {
            first_packet(engine, key)
        } else {
            None
        },
        data: format::bytes(t, data.tot_data(bytes)),
        data_in: format::bytes(t, data.incoming_data(bytes)),
        data_out: format::bytes(t, data.outgoing_data(bytes)),
        bar_in: data.incoming_data(bytes),
        bar_out: data.outgoing_data(bytes),
        first: info.initial_timestamp,
        last: info.final_timestamp,
        more,
        latency: latency_text(t, status),
        latency_supported: engine.capture_source.supports_latency()
            && !matches!(key.protocol, Protocol::Arp),
        measuring: matches!(status, Some(LatencyStatus::Measuring)),
        remote,
        remote_favorite: favorites.contains_address(&remote),
        rdns: resolved.map(|(rdns, _)| rdns.clone()).unwrap_or_default(),
        asn: resolved
            .map(|(_, h)| h)
            .filter(|h| !h.asn.name.is_empty())
            .map(|h| (format!("AS{}", h.asn.code), h.asn.name.clone())),
        host_tags: resolved
            .and_then(|(_, h)| engine.info_traffic.hosts.get(h))
            .map(|i| host_tags(t, i).join(", "))
            .unwrap_or_default(),
        country: resolved.map(|(_, h)| country(h)).unwrap_or_default(),
        host_jump: resolved.map(|(_, host)| Jump::Connections {
            filter: FavoriteItem::Host((host.clone(), DataInfoHost::default())).connection_filter(),
        }),
        address_jump: Jump::Connections {
            filter: format!("ip.addr == {remote}"),
        },
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
