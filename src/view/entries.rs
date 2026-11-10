//! Hosts, services and programs with their traffic (lists and the top entries of the overview).

use std::collections::{BTreeSet, HashMap};
use std::net::IpAddr;

use crate::config::favorite::{Favorite, FavoriteItem, FavoriteKey};
use crate::engine::state::EngineState;
use crate::networking::manage_packets::{get_address_to_lookup, service_of_port};
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::host::Host;
use crate::networking::types::program::Program;
use crate::networking::types::service::Service;
use crate::networking::types::service_category::ServiceCategory;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::networking::types::traffic_type::TrafficType;
use crate::report::types::sort_type::SortType;
use crate::view::i18n::Lang;
use crate::view::{Jump, format};

/// Maximum number of entries shown in a list
const MAX_ROWS: usize = 500;

/// A host, service or program in a list
pub struct EntryRow {
    pub label: String,
    /// Second line: address and AS or labels of a host, ports and category of a service, path of
    /// a program (`hostSub` of the draft)
    pub sub: String,
    /// Country code of a host (`App.flag`), empty while it is unknown
    pub cc: String,
    /// Number of connections, empty in the top lists
    pub connections: String,
    /// A short note after the name in the top lists: country or first label of a host, category
    /// of a service
    pub short: String,
    /// Small labels (local, multicast, …)
    pub tags: Vec<String>,
    pub total: String,
    pub incoming: String,
    pub outgoing: String,
    /// Widths of the incoming and outgoing parts of the bar (0–100, relative to the largest entry)
    pub bar_in: f32,
    pub bar_out: f32,
    /// The connections of this entry
    pub jump: Jump,
    pub favorite: bool,
    /// What marks the entry as a favorite; services cannot be favorites
    pub key: Option<FavoriteKey>,
}

impl EntryRow {}

pub fn kind_code(kind: Favorite) -> &'static str {
    match kind {
        Favorite::Host => "host",
        Favorite::Service => "service",
        Favorite::Program => "program",
    }
}

pub(crate) fn category_id(category: ServiceCategory) -> &'static str {
    match category {
        ServiceCategory::Web => "category-web",
        ServiceCategory::Email => "category-email",
        ServiceCategory::Chat => "category-chat",
        ServiceCategory::Media => "category-media",
        ServiceCategory::Files => "category-files",
        ServiceCategory::Storage => "category-storage",
        ServiceCategory::Database => "category-database",
        ServiceCategory::Remote => "category-remote",
        ServiceCategory::Management => "category-management",
        ServiceCategory::Discovery => "category-discovery",
        ServiceCategory::Network => "category-network",
        ServiceCategory::Vpn => "category-vpn",
        ServiceCategory::Identity => "category-identity",
        ServiceCategory::Printing => "category-printing",
        ServiceCategory::Gaming => "category-gaming",
        ServiceCategory::Industrial => "category-industrial",
        ServiceCategory::Development => "category-development",
        ServiceCategory::Middleware => "category-middleware",
        ServiceCategory::Security => "category-security",
        ServiceCategory::Licensing => "category-licensing",
        ServiceCategory::Other => "category-other",
    }
}

/// The text of a service: its name, or "unknown" / "not applicable"
pub fn service_label(t: Lang, service: &Service) -> String {
    match service {
        Service::Name(name) => (*name).to_owned(),
        Service::Unknown => t.get("unknown"),
        Service::NotApplicable => t.get("not-applicable"),
    }
}

/// Linux cuts process names to 15 bytes (`evolution-calen`)
const PROCESS_NAME_LEN: usize = 15;

/// The name of a program, completed from its path when Linux cut it
/// (`evolution-calen`, `/usr/libexec/evolution-calendar-factory` → `evolution-calendar-factory`)
fn program_name(name: &str, path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or_default();
    if name.len() == PROCESS_NAME_LEN && file.len() > name.len() && file.starts_with(name) {
        file.to_owned()
    } else {
        name.to_owned()
    }
}

/// The text of a program: its name, or "unknown" / "not applicable"
pub fn program_label(t: Lang, program: &Program) -> String {
    match program {
        Program::NamePath((name, path)) => program_name(name, path),
        Program::Unknown => t.get("unknown"),
        Program::NotApplicable => t.get("not-applicable"),
    }
}

/// AS name and country of a host, e.g. `GOOGLE · US`
pub fn host_sub(host: &Host) -> String {
    let mut parts = Vec::new();
    if !host.asn.name.is_empty() {
        parts.push(host.asn.name.clone());
    }
    if host.country != crate::countries::types::country::Country::ZZ {
        parts.push(host.country.to_string());
    }
    parts.join(" · ")
}

/// What the connections tell about the entries of the list: how many each has, the addresses
/// of a host and the ports of a service
#[derive(Default)]
pub struct Extras {
    hosts: HashMap<Host, (usize, Vec<IpAddr>)>,
    services: HashMap<Service, (usize, BTreeSet<(u16, String)>)>,
    programs: HashMap<Program, usize>,
}

impl Extras {
    pub fn read(state: &EngineState) -> Self {
        let mut extras = Self::default();
        for (key, info) in &state.info_traffic.map {
            let remote = get_address_to_lookup(key, info.traffic_direction);
            if let Some((_, host)) = state.addresses_resolved.get(&remote) {
                let entry = extras.hosts.entry(host.clone()).or_default();
                entry.0 += 1;
                if !entry.1.contains(&remote) {
                    entry.1.push(remote);
                }
            }
            if matches!(info.service, Service::Name(_)) {
                let entry = extras.services.entry(info.service).or_default();
                entry.0 += 1;
                // the port that names the service, the one of the remote end first
                let (remote_port, local_port) = match info.traffic_direction {
                    TrafficDirection::Outgoing => (key.dst_port, key.src_port),
                    TrafficDirection::Incoming => (key.src_port, key.dst_port),
                };
                if let Some(port) = [remote_port, local_port]
                    .into_iter()
                    .flatten()
                    .find(|port| service_of_port(*port, key.protocol) == Some(info.service))
                {
                    entry.1.insert((port, key.protocol.to_string()));
                }
            }
            *extras.programs.entry(info.program.clone()).or_default() += 1;
        }
        extras
    }
}

/// The second line of a host (`hostSub`): its labels or its AS, with its address unless the
/// address is its name
fn host_line(host: &Host, tags: &[String], addresses: &[IpAddr]) -> String {
    let address = match addresses {
        [] => String::new(),
        [one] => one.to_string(),
        [first, rest @ ..] => format!("{first} +{}", rest.len()),
    };
    let address = if address.is_empty()
        || addresses
            .first()
            .is_some_and(|a| a.to_string() == host.domain)
    {
        String::new()
    } else {
        address
    };
    let what = if !tags.is_empty() {
        tags.join(", ")
    } else if host.asn.name.is_empty() {
        String::new()
    } else {
        format!("AS{} {}", host.asn.code, host.asn.name)
    };
    match (tags.is_empty(), address.is_empty(), what.is_empty()) {
        (_, true, _) => what,
        (_, false, true) => address,
        (false, false, false) => format!("{what} · {address}"),
        (true, false, false) => format!("{address} · {what}"),
    }
}

/// The second line of a service: the ports it was seen on and its category (`443/TCP · Web`)
fn service_line(category: String, ports: &BTreeSet<(u16, String)>) -> String {
    let mut seen: Vec<String> = ports
        .iter()
        .take(3)
        .map(|(port, protocol)| format!("{port}/{protocol}"))
        .collect();
    if ports.len() > 3 {
        seen.push("…".to_owned());
    }
    if seen.is_empty() {
        category
    } else {
        format!("{} · {category}", seen.join(", "))
    }
}

/// Rows of entries in the unit chosen by the user; bars are relative to the largest entry
pub fn entry_rows(t: Lang, state: &EngineState, items: &[FavoriteItem]) -> Vec<EntryRow> {
    entry_rows_with(t, state, items, None)
}

/// [`entry_rows`] with what the connections tell about them, for the list of a page
#[allow(clippy::too_many_lines)]
fn entry_rows_with(
    t: Lang,
    state: &EngineState,
    items: &[FavoriteItem],
    extras: Option<&Extras>,
) -> Vec<EntryRow> {
    let repr = DataRepr {
        data_unit: state.conf.data_repr.data_unit,
        per_second: false,
    };
    let max = items
        .iter()
        .map(|i| i.data_info().tot_data(repr))
        .max()
        .unwrap_or(0)
        .max(1);
    let favorites = &state.conf.favorites;
    items
        .iter()
        .map(|item| {
            let data = item.data_info();
            let (incoming, outgoing) = (data.incoming_data(repr), data.outgoing_data(repr));
            let (label, sub, tags, favorite) = match item {
                FavoriteItem::Host((host, info)) => {
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
                    let label = if host.domain.is_empty() {
                        t.get("unknown")
                    } else {
                        host.domain.clone()
                    };
                    let sub = match extras {
                        Some(extras) => host_line(
                            host,
                            &tags,
                            extras
                                .hosts
                                .get(host)
                                .map_or(&[][..], |(_, a)| a.as_slice()),
                        ),
                        None => host_sub(host),
                    };
                    (label, sub, tags, favorites.contains_host(host))
                }
                FavoriteItem::Service((service, _)) => {
                    let category = service.category();
                    let sub = if matches!(service, Service::Name(_)) {
                        let category = t.get(category_id(category));
                        match extras.and_then(|e| e.services.get(service)) {
                            Some((_, ports)) => service_line(category, ports),
                            None => category,
                        }
                    } else {
                        String::new()
                    };
                    (service_label(t, service), sub, Vec::new(), false)
                }
                FavoriteItem::Program((program, _)) => {
                    let path = program.path();
                    let sub = if path.is_empty() && extras.is_some() {
                        t.get("program-unassigned")
                    } else {
                        path
                    };
                    (
                        program_label(t, program),
                        sub,
                        Vec::new(),
                        favorites.contains_program(program),
                    )
                }
                FavoriteItem::Address((address, _)) => (
                    address.to_string(),
                    String::new(),
                    Vec::new(),
                    favorites.contains_address(address),
                ),
            };
            let short = match item {
                FavoriteItem::Host((host, _))
                    if host.country != crate::countries::types::country::Country::ZZ =>
                {
                    host.country.to_string()
                }
                FavoriteItem::Host(_) => tags.first().cloned().unwrap_or_default(),
                FavoriteItem::Service(_) => sub.clone(),
                FavoriteItem::Program(_) | FavoriteItem::Address(_) => String::new(),
            };
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let width = |v: u128| (v as f64 * 100.0 / max as f64) as f32;
            let connections = extras.map_or_else(String::new, |e| {
                let count = match item {
                    FavoriteItem::Host((host, _)) => e.hosts.get(host).map_or(0, |(n, _)| *n),
                    FavoriteItem::Service((service, _)) => {
                        e.services.get(service).map_or(0, |(n, _)| *n)
                    }
                    FavoriteItem::Program((program, _)) => {
                        e.programs.get(program).copied().unwrap_or(0)
                    }
                    FavoriteItem::Address(_) => 0,
                };
                format::count(t, count as u128)
            });
            let cc = match item {
                FavoriteItem::Host((host, _))
                    if host.country != crate::countries::types::country::Country::ZZ =>
                {
                    host.country.to_string()
                }
                _ => String::new(),
            };
            EntryRow {
                label,
                sub,
                cc,
                connections,
                short,
                tags,
                total: format::amount(t, repr.data_unit, incoming + outgoing),
                incoming: format::amount(t, repr.data_unit, incoming),
                outgoing: format::amount(t, repr.data_unit, outgoing),
                bar_in: width(incoming),
                bar_out: width(outgoing),
                jump: Jump::Connections {
                    filter: item.connection_filter(),
                },
                favorite,
                key: match item {
                    FavoriteItem::Host((host, _)) => Some(FavoriteKey::Host(host.clone())),
                    FavoriteItem::Service(_) => None,
                    FavoriteItem::Program((program, _)) => {
                        Some(FavoriteKey::Program(program.clone()))
                    }
                    FavoriteItem::Address((address, _)) => Some(FavoriteKey::Address(*address)),
                },
            }
        })
        .collect()
}

/// The entries of a kind, sorted by volume, as a list
pub struct EntryList {
    pub rows: Vec<EntryRow>,
    /// All entries found; the rows stop at [`MAX_ROWS`]
    pub total: usize,
}

/// The entries of `kind` whose name or second line contains `text`
pub fn entry_list(
    t: Lang,
    state: &EngineState,
    kind: Favorite,
    sort: SortType,
    only_favorites: bool,
    text: &str,
) -> EntryList {
    let repr = DataRepr {
        data_unit: state.conf.data_repr.data_unit,
        per_second: false,
    };
    let needle = text.trim().to_lowercase();
    let items = state.favorite_entries(kind, repr, sort, only_favorites);
    let extras = Extras::read(state);
    let mut rows = entry_rows_with(t, state, &items, Some(&extras));
    if !needle.is_empty() {
        rows.retain(|r| {
            format!("{} {}", r.label, r.sub)
                .to_lowercase()
                .contains(&needle)
        });
        // the bars are relative to the largest entry shown
        let largest = rows
            .iter()
            .map(|r| r.bar_in + r.bar_out)
            .fold(0.0_f32, f32::max);
        if largest > 0.0 {
            for r in &mut rows {
                r.bar_in *= 100.0 / largest;
                r.bar_out *= 100.0 / largest;
            }
        }
    }
    let total = rows.len();
    rows.truncate(MAX_ROWS);
    EntryList { rows, total }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_second_line_of_a_host_follows_the_draft() {
        let host = Host {
            domain: "video.example.net".into(),
            asn: crate::networking::types::asn::Asn {
                code: "64496".into(),
                name: "Example Transit".into(),
            },
            ..Host::default()
        };
        let address: IpAddr = "203.0.113.80".parse().unwrap();
        assert_eq!(
            host_line(&host, &[], &[address]),
            "203.0.113.80 · AS64496 Example Transit"
        );
        assert_eq!(
            host_line(&host, &["lokal".to_owned()], &[address]),
            "lokal · 203.0.113.80"
        );
        let other: IpAddr = "203.0.113.81".parse().unwrap();
        assert_eq!(
            host_line(&host, &[], &[address, other]),
            "203.0.113.80 +1 · AS64496 Example Transit"
        );
        // a host named by its address
        let bare = Host {
            domain: "192.0.2.40".into(),
            ..Host::default()
        };
        let address: IpAddr = "192.0.2.40".parse().unwrap();
        assert_eq!(host_line(&bare, &[], &[address]), "");
        let mut ports = BTreeSet::new();
        ports.insert((443, "TCP".to_owned()));
        ports.insert((443, "UDP".to_owned()));
        assert_eq!(service_line("Web".into(), &ports), "443/TCP, 443/UDP · Web");
    }

    #[test]
    fn cut_program_names_are_completed_from_the_path() {
        assert_eq!(
            program_name("evolution-calen", "/usr/libexec/evolution-calendar-factory"),
            "evolution-calendar-factory"
        );
        // shorter names are whole, other files are kept apart
        assert_eq!(
            program_name("chrome", "/opt/google/chrome/chrome"),
            "chrome"
        );
        assert_eq!(
            program_name("abcdefghijklmno", "/usr/bin/other"),
            "abcdefghijklmno"
        );
        assert_eq!(program_name("abcdefghijklmno", ""), "abcdefghijklmno");
    }
}
