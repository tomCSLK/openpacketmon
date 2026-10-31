//! The overview of the running capture (tiles, top hosts, services and programs).

use crate::config::favorite::Favorite;
use crate::engine::state::EngineState;
use crate::networking::types::data_representation::{DataRepr, DataUnit};
use crate::report::types::sort_type::SortType;
use crate::view::capture::{capture_state, source_kind};
use crate::view::entries::{EntryRow, entry_rows};
use crate::view::format;
use crate::view::i18n::Lang;

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

/// A value tile; `id` names the value for updates while the page is open
pub struct Tile {
    pub label: String,
    pub value: String,
    pub id: &'static str,
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
            id: "total",
        },
        Tile {
            label: t.get("tile-in"),
            value: format::amount(t, unit, data.incoming_data(repr)),
            id: "in",
        },
        Tile {
            label: t.get("tile-out"),
            value: format::amount(t, unit, data.outgoing_data(repr)),
            id: "out",
        },
        Tile {
            label: t.get("tile-connections"),
            value: format::count(t, state.info_traffic.map.len() as u128),
            id: "connections",
        },
        Tile {
            label: t.get("tile-hosts"),
            value: format::count(t, state.info_traffic.hosts.len() as u128),
            id: "hosts",
        },
    ];
    if let Some(dropped) = state.info_traffic.dropped_packets {
        tiles.push(Tile {
            label: t.get("tile-dropped"),
            value: format::count(t, dropped.total(DataRepr::packets(false), *data)),
            id: "dropped",
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

/// What the overview shows
pub struct Overview {
    pub capture: Option<CaptureHead>,
    pub tiles: Vec<Tile>,
    pub hosts: Vec<EntryRow>,
    pub services: Vec<EntryRow>,
    /// Not for capture sources without programs
    pub programs: Option<Vec<EntryRow>>,
    pub unit: DataUnit,
}

pub fn overview(t: Lang, state: &EngineState) -> Overview {
    Overview {
        capture: capture_head(t, state),
        tiles: tiles(t, state),
        hosts: top(t, state, Favorite::Host),
        services: top(t, state, Favorite::Service),
        programs: (!state.is_running() || state.capture_source.supports_programs())
            .then(|| top(t, state, Favorite::Program)),
        unit: state.conf.data_repr.data_unit,
    }
}
