//! Hosts, services and programs with their traffic (lists and the top entries of the overview).

use crate::config::favorite::{Favorite, FavoriteItem, FavoriteKey};
use crate::engine::state::EngineState;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::host::Host;
use crate::networking::types::program::Program;
use crate::networking::types::service::Service;
use crate::networking::types::service_category::ServiceCategory;
use crate::networking::types::traffic_type::TrafficType;
use crate::report::types::sort_type::SortType;
use crate::view::i18n::Lang;
use crate::view::{Jump, format};

/// Maximum number of entries shown in a list
const MAX_ROWS: usize = 500;

/// A host, service or program in a list
pub struct EntryRow {
    pub label: String,
    /// Second line: AS and country, service category, program path
    pub sub: String,
    /// Small labels (local, multicast, …)
    pub tags: Vec<String>,
    pub total: String,
    pub incoming: String,
    pub outgoing: String,
    /// Widths of the incoming and outgoing parts of the bar (0–100, relative to the largest entry)
    pub bar_in: u32,
    pub bar_out: u32,
    /// The connections of this entry
    pub jump: Jump,
    pub favorite: bool,
    /// What marks the entry as a favorite; services cannot be favorites
    pub key: Option<FavoriteKey>,
}

impl EntryRow {
    /// Whether the row gets a star
    pub fn favorable(&self) -> bool {
        self.key.is_some()
    }
}

pub fn kind_code(kind: Favorite) -> &'static str {
    match kind {
        Favorite::Host => "host",
        Favorite::Service => "service",
        Favorite::Program => "program",
    }
}

fn category_id(category: ServiceCategory) -> &'static str {
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

/// Rows of entries in the unit chosen by the user; bars are relative to the largest entry
pub fn entry_rows(t: Lang, state: &EngineState, items: &[FavoriteItem]) -> Vec<EntryRow> {
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
                    (label, host_sub(host), tags, favorites.contains_host(host))
                }
                FavoriteItem::Service((service, _)) => {
                    let category = service.category();
                    let sub = if matches!(service, Service::Name(_)) {
                        t.get(category_id(category))
                    } else {
                        String::new()
                    };
                    (service_label(t, service), sub, Vec::new(), false)
                }
                FavoriteItem::Program((program, _)) => (
                    program_label(t, program),
                    program.path(),
                    Vec::new(),
                    favorites.contains_program(program),
                ),
                FavoriteItem::Address((address, _)) => (
                    address.to_string(),
                    String::new(),
                    Vec::new(),
                    favorites.contains_address(address),
                ),
            };
            #[allow(clippy::cast_possible_truncation)]
            let width = |v: u128| (v * 100 / max) as u32;
            EntryRow {
                label,
                sub,
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
    pub title: String,
    pub subtitle: String,
    pub rows: Vec<EntryRow>,
    /// All entries found; the rows stop at [`MAX_ROWS`]
    pub total: usize,
    /// Programs are only known for network adapters
    pub unsupported: bool,
}

/// The entries of `kind` whose name contains `text`
pub fn entry_list(
    t: Lang,
    state: &EngineState,
    kind: Favorite,
    ascending: bool,
    only_favorites: bool,
    text: &str,
) -> EntryList {
    let sort = if ascending {
        SortType::Ascending
    } else {
        SortType::Descending
    };
    let repr = DataRepr {
        data_unit: state.conf.data_repr.data_unit,
        per_second: false,
    };
    let needle = text.trim().to_lowercase();
    let items: Vec<FavoriteItem> = state
        .favorite_entries(kind, repr, sort, only_favorites)
        .into_iter()
        .filter(|i| needle.is_empty() || i.to_entry_string().to_lowercase().contains(&needle))
        .collect();
    let total = items.len();
    let code = kind_code(kind);
    EntryList {
        title: t.get(&format!("nav-{code}s")),
        subtitle: t.get(&format!("{code}s-subtitle")),
        rows: entry_rows(t, state, &items[..total.min(MAX_ROWS)]),
        total,
        unsupported: kind == Favorite::Program
            && state.is_running()
            && !state.capture_source.supports_programs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
