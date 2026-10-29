//! The overview of the running capture (tiles, live chart, top hosts, services and programs).

use askama::Template;
use axum::extract::State;
use axum::response::Response;

use crate::config::favorite::Favorite;
use crate::engine::state::EngineState;
use crate::networking::types::data_representation::DataRepr;
use crate::report::types::sort_type::SortType;
use crate::web::api::unit_code;
use crate::web::auth::Session;
use crate::web::capture::{capture_state, source_kind};
use crate::web::entries::{EntryRow, entry_rows};
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, render};
use crate::web::{AppState, format};

/// Entries in each top list
const TOP: usize = 5;

/// The capture shown in the page head
pub struct CaptureHead {
    pub kind: String,
    pub name: String,
    pub state: String,
    pub frozen: bool,
    pub finished: bool,
}

/// A value tile; `live` names the value `static/live.js` keeps current
pub struct Tile {
    pub label: String,
    pub value: String,
    pub live: &'static str,
}

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

fn capture_head(t: Lang, state: &EngineState) -> Option<CaptureHead> {
    state.is_running().then(|| CaptureHead {
        kind: source_kind(t, &state.capture_source),
        name: state.capture_source.get_name(),
        state: capture_state(t, state),
        frozen: state.frozen,
        finished: state.traffic_history.no_more_packets(),
    })
}

fn tiles(t: Lang, state: &EngineState) -> Vec<Tile> {
    let unit = state.conf.data_repr.data_unit;
    let repr = DataRepr {
        data_unit: unit,
        per_second: false,
    };
    let data = &state.info_traffic.tot_data_info;
    let mut tiles = vec![
        Tile {
            label: t.get("tile-total"),
            value: format::amount(t, unit, data.tot_data(repr)),
            live: "total",
        },
        Tile {
            label: t.get("tile-in"),
            value: format::amount(t, unit, data.incoming_data(repr)),
            live: "in",
        },
        Tile {
            label: t.get("tile-out"),
            value: format::amount(t, unit, data.outgoing_data(repr)),
            live: "out",
        },
        Tile {
            label: t.get("tile-connections"),
            value: format::count(t, state.info_traffic.map.len() as u128),
            live: "connections",
        },
        Tile {
            label: t.get("tile-hosts"),
            value: format::count(t, state.info_traffic.hosts.len() as u128),
            live: "hosts",
        },
    ];
    if let Some(dropped) = state.info_traffic.dropped_packets {
        tiles.push(Tile {
            label: t.get("tile-dropped"),
            value: format::count(t, dropped.total(DataRepr::packets(false), *data)),
            live: "dropped",
        });
    }
    tiles
}

fn top(t: Lang, state: &EngineState, kind: Favorite) -> Vec<EntryRow> {
    let repr = DataRepr {
        data_unit: state.conf.data_repr.data_unit,
        per_second: false,
    };
    let mut items = state.favorite_entries(kind, repr, SortType::Descending, false);
    items.truncate(TOP);
    entry_rows(t, state, &items)
}

/// `GET /overview`
pub async fn page(State(state): State<AppState>, session: Session) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let programs = (!engine.is_running() || engine.capture_source.supports_programs())
        .then(|| top(t, &engine, Favorite::Program));
    render(&OverviewPage {
        t,
        layout: Layout::new(&engine, &session),
        capture: capture_head(t, &engine),
        tiles: tiles(t, &engine),
        hosts: top(t, &engine, Favorite::Host),
        services: top(t, &engine, Favorite::Service),
        programs,
        unit: unit_code(engine.conf.data_repr.data_unit),
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
