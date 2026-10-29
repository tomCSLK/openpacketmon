//! Hosts, services and programs with their traffic (lists and the top entries of the overview),
//! and marking them as favorites.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;
use std::net::IpAddr;

use crate::config::favorite::{Favorite, FavoriteItem, FavoriteKey};
use crate::engine::state::EngineState;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::host::Host;
use crate::networking::types::program::Program;
use crate::networking::types::service::Service;
use crate::networking::types::service_category::ServiceCategory;
use crate::networking::types::traffic_type::TrafficType;
use crate::report::types::sort_type::SortType;
use crate::web::auth::Session;
use crate::web::capture::back_path;
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError, format};

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
    /// Query of the connection list showing this entry's connections (`f=…`)
    pub search: String,
    /// Services cannot be favorites
    pub can_favorite: bool,
    pub favorite: bool,
    pub kind: &'static str,
    /// Identifies the entry for the favorite form (JSON of the host, service or program)
    pub key: String,
}

impl EntryRow {
    /// For the template: whether the row gets a star
    pub fn favorable(&self) -> bool {
        self.can_favorite
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

/// Query string of a filter of the connection list (empty for all connections)
pub fn search_query(filter: &str) -> String {
    if filter.is_empty() {
        String::new()
    } else {
        format!("f={}", encode(filter))
    }
}

/// Percent-encodes a query value
pub fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn favorite_key_json(item: &FavoriteItem) -> String {
    match item {
        FavoriteItem::Host((host, _)) => serde_json::to_string(host),
        FavoriteItem::Service((service, _)) => serde_json::to_string(service),
        FavoriteItem::Program((program, _)) => serde_json::to_string(program),
        FavoriteItem::Address((address, _)) => serde_json::to_string(address),
    }
    .unwrap_or_default()
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
                search: search_query(&item.connection_filter()),
                can_favorite: !matches!(item, FavoriteItem::Service(_)),
                favorite,
                kind: match item {
                    FavoriteItem::Host(_) => "host",
                    FavoriteItem::Service(_) => "service",
                    FavoriteItem::Program(_) => "program",
                    FavoriteItem::Address(_) => "address",
                },
                key: favorite_key_json(item),
            }
        })
        .collect()
}

#[derive(Deserialize)]
pub struct ListQuery {
    /// `asc` or `desc` by volume; anything else sorts by volume descending as well
    sort: Option<String>,
    /// Only favorites
    #[serde(default)]
    favorites: Option<String>,
    /// Text filter on the name
    #[serde(default)]
    q: String,
}

#[derive(Template)]
#[template(path = "entries.html")]
#[allow(clippy::struct_excessive_bools)] // switches of a page or a command line
struct EntriesPage {
    t: Lang,
    layout: Layout,
    kind: &'static str,
    title: String,
    subtitle: String,
    rows: Vec<EntryRow>,
    total: usize,
    running: bool,
    /// Programs are only known for network adapters
    unsupported: bool,
    ascending: bool,
    only_favorites: bool,
    q: String,
    unit: &'static str,
    back: String,
}

/// Maximum number of entries shown in a list
const MAX_ROWS: usize = 500;

fn list(state: &AppState, session: &Session, kind: Favorite, query: ListQuery) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let ascending = query.sort.as_deref() == Some("asc");
    let sort = if ascending {
        SortType::Ascending
    } else {
        SortType::Descending
    };
    let only_favorites = query.favorites.is_some();
    let repr = DataRepr {
        data_unit: engine.conf.data_repr.data_unit,
        per_second: false,
    };
    let needle = query.q.trim().to_lowercase();
    let items: Vec<FavoriteItem> = engine
        .favorite_entries(kind, repr, sort, only_favorites)
        .into_iter()
        .filter(|i| needle.is_empty() || i.to_entry_string().to_lowercase().contains(&needle))
        .collect();
    let total = items.len();
    let shown = &items[..total.min(MAX_ROWS)];
    let unsupported = kind == Favorite::Program
        && engine.is_running()
        && !engine.capture_source.supports_programs();
    let code = kind_code(kind);
    let mut back = format!("/{code}s");
    let mut params = Vec::new();
    if ascending {
        params.push("sort=asc".to_owned());
    }
    if only_favorites {
        params.push("favorites=1".to_owned());
    }
    if !query.q.is_empty() {
        params.push(format!("q={}", encode(&query.q)));
    }
    if !params.is_empty() {
        back = format!("{back}?{}", params.join("&"));
    }
    render(&EntriesPage {
        t,
        layout: Layout::new(&engine, session),
        kind: code,
        title: t.get(&format!("nav-{code}s")),
        subtitle: t.get(&format!("{code}s-subtitle")),
        rows: entry_rows(t, &engine, shown),
        total,
        running: engine.is_running(),
        unsupported,
        ascending,
        only_favorites,
        q: query.q,
        unit: crate::web::api::unit_code(repr.data_unit),
        back,
    })
}

pub async fn hosts(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Response {
    list(&state, &session, Favorite::Host, query)
}

pub async fn services(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Response {
    list(&state, &session, Favorite::Service, query)
}

pub async fn programs(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Response {
    list(&state, &session, Favorite::Program, query)
}

#[derive(Deserialize)]
pub struct FavoriteForm {
    #[serde(default)]
    csrf_token: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    key: String,
    /// `1` to add, anything else to remove
    #[serde(default)]
    add: String,
    #[serde(default)]
    back: String,
}

/// The favorite named by a form: looked up among the current entries and the favorites,
/// so no names are created from the request
fn find_key(engine: &EngineState, kind: &str, key: &str) -> Option<FavoriteKey> {
    let same = |json: Result<String, serde_json::Error>| json.is_ok_and(|j| j == key);
    let favorites = &engine.conf.favorites;
    match kind {
        "host" => engine
            .info_traffic
            .hosts
            .keys()
            .chain(favorites.hosts())
            .find(|h| same(serde_json::to_string(h)))
            .cloned()
            .map(FavoriteKey::Host),
        "program" => engine
            .program_lookup
            .iter()
            .flat_map(|l| l.programs().keys())
            .chain(favorites.programs())
            .find(|p| same(serde_json::to_string(p)))
            .cloned()
            .map(FavoriteKey::Program),
        // an address can be typed in the settings as well; it only has to be one
        "address" => key
            .trim_matches('"')
            .parse::<IpAddr>()
            .ok()
            .map(FavoriteKey::Address),
        _ => None,
    }
}

/// `POST /favorites`: adds an entry to the favorites or removes it
pub async fn toggle_favorite(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<FavoriteForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    {
        let mut engine = state.engine.lock();
        let key = find_key(&engine, &form.kind, &form.key).ok_or(WebError::NotFound)?;
        engine.add_or_remove_favorite(&key, form.add == "1");
    }
    state.engine.store_conf();
    Ok(Redirect::to(&back_path(&form.back, "/hosts")))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::web::testutil::*;

    #[test]
    fn search_values_are_encoded() {
        assert_eq!(encode("a b&c=d/é"), "a%20b%26c%3Dd%2F%C3%A9");
        assert_eq!(
            search_query("service == \"https\""),
            "f=service%20%3D%3D%20%22https%22"
        );
        assert_eq!(search_query(""), "");
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

    #[tokio::test]
    async fn the_lists_render_and_unknown_favorites_are_refused() {
        let h = harness().await;
        for path in ["/hosts", "/services?sort=asc", "/programs?favorites=1&q=x"] {
            let reply = send(&h.app, get(path, Some(&h.cookie))).await;
            assert_eq!(reply.status, StatusCode::OK, "{path}");
        }
        let form = format!(
            "csrf_token={}&kind=service&key=%7B%22Name%22%3A%22https%22%7D&add=1",
            h.csrf
        );
        let reply = send(&h.app, post("/favorites", &h.cookie, &form)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }
}
