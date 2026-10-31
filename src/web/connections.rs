//! The pages of the connections: the list in pages and the details of one connection.

use std::fmt::Write;
use std::net::IpAddr;

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;

use crate::engine::state::EngineState;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
use crate::report::types::sort_type::SortType;
use crate::view::Jump;
use crate::view::connections::{
    ConnectionRow, FILTER_EXAMPLES, Fact, FactValue, ListRequest, connection_detail,
    connection_list, filter_field_names, latency_allowed,
};
use crate::web::auth::Session;
use crate::web::entries::encode;
use crate::web::format::TimeHtml;
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::packets::filters_href;
use crate::web::pager::Pager;
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError};

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
    let l = connection_list(
        t,
        &engine,
        &ListRequest {
            filter: &query.f,
            sort: query.sort_type(),
            page: query.page,
            size: query.size,
        },
    );
    render(&ConnectionsPage {
        t,
        layout: Layout::new(&engine, &session),
        running: l.running,
        filtered: query.is_filtered(),
        hidden_filters: query.hidden_fields(),
        error: l.error,
        examples: FILTER_EXAMPLES,
        protocols: crate::filter::CONNECTION_PROTOCOLS.join(" "),
        fields: filter_field_names().join(" "),
        rows: l.rows,
        total: l.total,
        matching_data: l.matching_data,
        pages: l.pages,
        first_row: l.first_row,
        last_row: l.last_row,
        pager: Pager::new(l.page, l.pages, |p| query.href(p), "/connections", {
            let mut fields = query.hidden_fields();
            if let Some(size) = query.size {
                fields.push(("size", size.to_string()));
            }
            fields
        }),
        sizes: l.sizes,
        programs: l.programs,
        unit: crate::web::api::unit_code(l.unit),
        back: query.href(l.page),
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
    address_jump: Jump,
    remote_favorite: bool,
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
    let d = connection_detail(t, &engine, key, info);
    let page = ConnectionPage {
        t,
        layout: Layout::new(&engine, &session),
        title: d.title,
        facts: d.facts,
        remote: d.remote.to_string(),
        host_facts: d.host_facts,
        message_types: d.message_types,
        latency: d.latency,
        latency_supported: d.latency_supported,
        measuring: d.measuring,
        self_query: connection_query(key),
        address_jump: d.address_jump,
        remote_favorite: d.remote_favorite,
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
    if !latency_allowed(&state.engine.lock(), ip) {
        return Err(WebError::NotFound);
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
