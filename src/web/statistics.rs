//! The page with the statistics of the stored packets.

use askama::Template;
use axum::extract::{Query, State};
use axum::response::Response;

use crate::view::Jump;
use crate::view::packets::FilterFault;
use crate::view::statistics::{FindingRow, HierarchyRow, TimeRow, statistics};
use crate::web::AppState;
use crate::web::auth::Session;
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::packets::PacketQuery;
use crate::web::pages::{Layout, render};

#[derive(Template)]
#[template(path = "statistics.html")]
struct StatisticsPage {
    t: Lang,
    layout: Layout,
    running: bool,
    unavailable: String,
    filter: String,
    /// The connection filter as typed
    connections: String,
    /// The connection the filter selects: its two ends and its page
    connection: Option<(String, String)>,
    error: Option<FilterFault>,
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

/// `GET /statistics?f=…&c=…`
pub async fn page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PacketQuery>,
) -> Response {
    let t = i18n::current();
    let layout = Layout::new(&state.engine.lock(), &session);
    let s = statistics(t, &state.engine, &state.packet_index, &query.f, &query.c);
    render(&StatisticsPage {
        t,
        layout,
        running: s.running,
        live: s.live,
        unavailable: s.unavailable,
        filter: query.f.clone(),
        connections: query.c.clone(),
        connection: s
            .connection
            .map(|(ends, key)| (ends, Jump::Connection(key).href())),
        error: s.error,
        summary: s.summary,
        hierarchy: s.hierarchy,
        findings: s.findings,
        times: s.times,
        chart: serde_json::to_string(&s.chart)
            .unwrap_or_else(|_| "{}".to_owned())
            .replace('<', "\\u003c"),
        packets_href: query.href("/packets"),
        empty: s.empty,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::view::statistics::packets_jump;
    use crate::web::testutil::*;

    #[tokio::test]
    async fn the_page_answers_without_a_capture() {
        let h = harness().await;
        let reply = send(&h.app, get("/statistics?f=tcp", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        let reply = send(&h.app, get("/statistics?f=tcp.port%20==", Some(&h.cookie))).await;
        assert!(reply.body.contains("filter-error"), "{}", reply.body);
    }

    #[tokio::test]
    async fn the_connection_filter_comes_along() {
        let h = harness().await;
        let reply = send(
            &h.app,
            get("/connections?f=country%20%3D%3D%20US", Some(&h.cookie)),
        )
        .await;
        assert!(
            reply
                .body
                .contains("href=\"/statistics?c=country%20%3D%3D%20US\""),
            "{}",
            reply.body
        );
        let reply = send(&h.app, get("/connections", Some(&h.cookie))).await;
        assert!(
            reply
                .body
                .contains("href=\"/statistics\" data-filter-carry")
        );

        // a field of the connection filter is no display filter, and the links keep it
        let reply = send(
            &h.app,
            get("/statistics?c=country%20%3D%3D%20US", Some(&h.cookie)),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(!reply.body.contains("Error at position"), "{}", reply.body);
        assert!(reply.body.contains("name=\"c\""), "{}", reply.body);
        assert!(
            reply
                .body
                .contains("href=\"/packets?c=country%20%3D%3D%20US\""),
            "{}",
            reply.body
        );
        let reply = send(
            &h.app,
            get("/statistics?c=country%20%3D%3D", Some(&h.cookie)),
        )
        .await;
        assert!(reply.body.contains("Error at position"), "{}", reply.body);
        assert_eq!(
            packets_jump("", "port == 443", "tcp").href(),
            "/packets?c=port%20%3D%3D%20443&f=tcp"
        );
    }
}
