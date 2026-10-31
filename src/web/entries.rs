//! The lists of hosts, services and programs, and marking them as favorites.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;
use std::net::IpAddr;

use crate::config::favorite::{Favorite, FavoriteKey};
use crate::engine::state::EngineState;
use crate::view::entries::{EntryList, EntryRow, entry_list, kind_code};
use crate::web::auth::Session;
use crate::web::capture::back_path;
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError};

/// The fields of the form that marks an entry as a favorite; `entry_table.html` calls them
pub trait FavoriteFields {
    fn form_kind(&self) -> &'static str;
    /// Identifies the entry: JSON of the host, program or address
    fn form_key(&self) -> String;
}

impl FavoriteFields for EntryRow {
    fn form_kind(&self) -> &'static str {
        match self.key {
            Some(FavoriteKey::Host(_)) => "host",
            Some(FavoriteKey::Program(_)) => "program",
            Some(FavoriteKey::Address(_)) => "address",
            None => "service",
        }
    }

    fn form_key(&self) -> String {
        match &self.key {
            Some(FavoriteKey::Host(host)) => serde_json::to_string(host),
            Some(FavoriteKey::Program(program)) => serde_json::to_string(program),
            Some(FavoriteKey::Address(address)) => serde_json::to_string(address),
            None => Ok(String::new()),
        }
        .unwrap_or_default()
    }
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

fn list(state: &AppState, session: &Session, kind: Favorite, query: ListQuery) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let ascending = query.sort.as_deref() == Some("asc");
    let only_favorites = query.favorites.is_some();
    let EntryList {
        title,
        subtitle,
        rows,
        total,
        running,
        unsupported,
        unit,
    } = entry_list(t, &engine, kind, ascending, only_favorites, &query.q);
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
        title,
        subtitle,
        rows,
        total,
        running,
        unsupported,
        ascending,
        only_favorites,
        q: query.q,
        unit: crate::web::api::unit_code(unit),
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
