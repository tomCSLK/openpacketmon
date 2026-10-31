//! The pages of the kept packets: the list (the browser shows only the rows in view and loads them
//! in blocks), the details of one packet, and the download as a PCAP file.

use askama::Template;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::dissect::Layer;
use crate::filter;
use crate::view::Jump;
use crate::view::packets::{
    FILTER_EXAMPLES, FilterFault, HexLine, PacketList, RowBlock, Window, conversation_filter,
    filter_field_names, filter_problem, packet_detail, packet_list, pcap_file, row_block,
};
use crate::web::auth::Session;
use crate::web::entries::encode;
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::pages::{Layout, render};
use crate::web::{AppState, WebError, format};

/// The filters of the list and the packet to show, as given in the address
#[derive(Deserialize, Default, Clone)]
pub struct PacketQuery {
    /// Display filter
    #[serde(default)]
    pub(crate) f: String,
    /// Connection filter: only packets of the connections it selects
    #[serde(default)]
    pub(crate) c: String,
    /// Show this packet (or the next matching one) and mark it; without it the list follows the
    /// newest packets
    pub(crate) goto: Option<u64>,
}

impl PacketQuery {
    /// An address of this site with the filters
    pub(crate) fn href(&self, path: &str) -> String {
        filters_href(path, &self.f, &self.c)
    }
}

/// An address of this site with a display and a connection filter
pub(crate) fn filters_href(path: &str, display: &str, connections: &str) -> String {
    let pairs: Vec<String> = [("c", connections.trim()), ("f", display.trim())]
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| format!("{name}={}", encode(value)))
        .collect();
    if pairs.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", pairs.join("&"))
    }
}

#[derive(Template)]
#[template(path = "packets.html")]
struct PacketsPage {
    t: Lang,
    layout: Layout,
    running: bool,
    /// Why no packets are kept (store off, IPFIX); empty if they are
    unavailable: String,
    /// The display filter as typed
    filter: String,
    /// The connection filter as typed
    connections: String,
    /// The connection the filter selects: its two ends and its page
    connection: Option<(String, String)>,
    error: Option<FilterFault>,
    /// The list can be shown (packets kept, filter valid)
    list: bool,
    goto: Option<u64>,
    download: String,
    statistics: String,
    foot: Option<String>,
    examples: [&'static str; 8],
    protocols: String,
    fields: String,
}

/// `GET /packets`: the page; the browser loads the rows with [`rows_api`]
pub async fn list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PacketQuery>,
) -> Response {
    let t = i18n::current();
    let layout = Layout::new(&state.engine.lock(), &session);
    let PacketList {
        running,
        unavailable,
        connection,
        error,
        list,
        foot,
    } = packet_list(t, &state.engine, &query.f, &query.c);
    render(&PacketsPage {
        t,
        layout,
        running,
        unavailable,
        filter: query.f.clone(),
        connections: query.c.clone(),
        connection: connection.map(|(ends, key)| (ends, Jump::Connection(key).href())),
        list,
        error,
        goto: query.goto,
        download: query.href("/packets.pcap"),
        statistics: query.href("/statistics"),
        foot,
        examples: FILTER_EXAMPLES,
        protocols: filter::PROTOCOLS.join(" "),
        fields: filter_field_names().join(" "),
    })
}

#[derive(Deserialize)]
pub struct RowsQuery {
    #[serde(default)]
    f: String,
    #[serde(default)]
    c: String,
    from: Option<usize>,
    #[serde(default)]
    count: usize,
    goto: Option<u64>,
    anchor: Option<u64>,
}

/// `GET /api/packets?f=…&from=…&count=…`: rows of the list (`goto` finds a packet, without
/// `from` and `goto` the newest rows)
pub async fn rows_api(
    State(state): State<AppState>,
    _session: Session,
    Query(query): Query<RowsQuery>,
) -> Result<Json<RowBlock>, WebError> {
    let window = Window {
        from: query.from,
        count: query.count,
        goto: query.goto,
        anchor: query.anchor,
    };
    row_block(
        i18n::current(),
        &state.engine,
        &state.packet_index,
        &query.f,
        &query.c,
        window,
    )
    .map(Json)
    .ok_or(WebError::NotFound)
}

#[derive(Deserialize)]
pub struct FilterCheck {
    #[serde(default)]
    f: String,
    /// `connections` for the connection filter, else the packet filter
    #[serde(default)]
    kind: String,
}

/// `GET /api/filter?f=…`: whether a display filter is valid, while it is typed
pub async fn check(_session: Session, Query(query): Query<FilterCheck>) -> Json<Option<String>> {
    Json(filter_problem(
        i18n::current(),
        &query.f,
        query.kind == "connections",
    ))
}

#[derive(Template)]
#[template(path = "packet.html")]
struct PacketPage {
    t: Lang,
    layout: Layout,
    number: u64,
    protocol: &'static str,
    info: String,
    truncated: bool,
    headers_only: bool,
    layers: Vec<Layer>,
    hex: Vec<HexLine>,
    /// Note that the hex dump ends before the packet
    hex_cut: Option<String>,
    /// Note that a message was completed from the following packets
    assembled: Option<String>,
    time: String,
    connection: Option<String>,
    /// Follow the stream (TCP and UDP)
    stream: Option<String>,
    /// The packets and the statistics of the connection
    conversation: Option<String>,
    statistics: Option<String>,
    back: String,
    prev: Option<String>,
    next: Option<String>,
    /// Packets this one answers or acknowledges, or that answer it: label and link
    related: Vec<(String, String)>,
}

#[derive(Deserialize)]
pub struct PacketDetailQuery {
    n: u64,
    #[serde(default)]
    back: String,
}

/// `GET /packet?n=…`: layers and bytes of one packet
pub async fn detail(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PacketDetailQuery>,
) -> Result<Response, WebError> {
    let t = i18n::current();
    let layout = Layout::new(&state.engine.lock(), &session);
    let d = packet_detail(t, &state.engine, query.n).ok_or(WebError::NotFound)?;
    let back = crate::web::capture::back_path(&query.back, "/packets");
    let back_param = encode(&back);
    let link = |n: u64| format!("/packet?n={n}&back={back_param}");
    let connection = d.connection;
    let conversation = |jump: fn(String) -> Jump| {
        connection
            .as_ref()
            .map(|key| jump(conversation_filter(key)).href())
    };
    let page = PacketPage {
        t,
        layout,
        number: d.number,
        protocol: d.protocol,
        info: d.info,
        truncated: d.truncated,
        headers_only: d.headers_only,
        layers: d.layers,
        hex: d.hex,
        hex_cut: d.hex_cut,
        assembled: d.assembled,
        time: format::time_html(d.time),
        connection: connection.map(|key| Jump::Connection(key).href()),
        stream: d.has_stream.then(|| Jump::Stream(d.number).href()),
        conversation: conversation(|filter| Jump::Packets {
            filter,
            connections: String::new(),
        }),
        statistics: conversation(|filter| Jump::Statistics {
            filter,
            connections: String::new(),
        }),
        prev: d.prev.map(link),
        next: d.next.map(link),
        related: d
            .related
            .into_iter()
            .map(|(label, n)| (label, link(n)))
            .collect(),
        back,
    };
    Ok(render(&page))
}

/// `GET /packets.pcap`: the kept packets (of the filter) as a PCAP file
pub async fn download(
    State(state): State<AppState>,
    _session: Session,
    Query(query): Query<PacketQuery>,
) -> Result<Response, WebError> {
    let file = pcap_file(
        i18n::current(),
        &state.engine,
        &state.packet_index,
        &query.f,
        &query.c,
    )
    .ok_or(WebError::NotFound)?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.tcpdump.pcap".to_owned(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", file.name),
            ),
        ],
        file.data,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::web::testutil::*;

    #[test]
    fn addresses_keep_both_filters() {
        assert_eq!(filters_href("/packets", " ", ""), "/packets");
        assert_eq!(filters_href("/packets", "dns", ""), "/packets?f=dns");
        assert_eq!(
            filters_href("/statistics", "tcp", " country == US "),
            "/statistics?c=country%20%3D%3D%20US&f=tcp"
        );
    }

    #[tokio::test]
    async fn without_a_capture_there_are_no_packets() {
        let h = harness().await;
        let reply = send(
            &h.app,
            get("/packets?f=tcp.port%20%3D%3D%20443", Some(&h.cookie)),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK);
        let reply = send(&h.app, get("/api/packets?count=50", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = send(&h.app, get("/packet?n=1", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = send(&h.app, get("/packets.pcap", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = send(&h.app, get("/packets.pcap", None)).await;
        assert_ne!(reply.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_wrong_filter_is_explained() {
        let h = harness().await;
        let reply = send(&h.app, get("/packets?f=tcp.port%20%3D%3D", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(
            reply.body.contains("Error at position 12"),
            "{}",
            reply.body
        );
        // the connection filter has its own field and its own fields
        let reply = send(&h.app, get("/packets?c=country%20%3D%3D", Some(&h.cookie))).await;
        assert!(reply.body.contains("name=\"c\""), "{}", reply.body);
        assert!(reply.body.contains("Error at position"), "{}", reply.body);
        let reply = send(
            &h.app,
            get("/packets?c=country%20%3D%3D%20US", Some(&h.cookie)),
        )
        .await;
        assert!(reply.body.contains("name=\"c\""), "{}", reply.body);
        assert!(!reply.body.contains("Error at position"), "{}", reply.body);
        let reply = send(&h.app, get("/api/filter?f=foo", Some(&h.cookie))).await;
        assert!(
            reply.body.contains("unknown field or protocol"),
            "{}",
            reply.body
        );
        let reply = send(&h.app, get("/api/filter?f=dns", Some(&h.cookie))).await;
        assert_eq!(reply.body, "null");
        let reply = send(&h.app, get("/api/filter?f=dns", None)).await;
        assert_ne!(reply.status, StatusCode::OK);
    }
}
