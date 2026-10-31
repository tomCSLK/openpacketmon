//! The page with the overview of the running capture.

use askama::Template;
use axum::extract::State;
use axum::response::Response;

use crate::view::entries::EntryRow;
use crate::view::overview::{CaptureHead, Tile, overview};
use crate::web::AppState;
use crate::web::api::unit_code;
use crate::web::auth::Session;
use crate::web::entries::FavoriteFields;
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::pages::{Layout, render};

#[derive(Template)]
#[template(path = "overview.html")]
struct OverviewPage {
    t: Lang,
    layout: Layout,
    capture: Option<CaptureHead>,
    tiles: Vec<Tile>,
    hosts: Vec<EntryRow>,
    services: Vec<EntryRow>,
    programs: Option<Vec<EntryRow>>,
    unit: &'static str,
    back: &'static str,
}

/// `GET /overview`
pub async fn page(State(state): State<AppState>, session: Session) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let o = overview(t, &engine);
    render(&OverviewPage {
        t,
        layout: Layout::new(&engine, &session),
        capture: o.capture,
        tiles: o.tiles,
        hosts: o.hosts,
        services: o.services,
        programs: o.programs,
        unit: unit_code(o.unit),
        back: "/overview",
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::web::testutil::*;

    #[tokio::test]
    async fn overview_renders_without_a_capture() {
        let h = harness().await;
        let reply = send(&h.app, get("/overview", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.contains("id=\"live-chart\""));
        assert!(reply.body.contains("href=\"/\">"));
        let reply = send(&h.app, get("/mini", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }
}
