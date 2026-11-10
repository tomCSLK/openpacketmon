//! What the pages of the user interface show, without the user interface: the data of every page,
//! translated and formatted, as plain Rust values. [`crate::gui`] draws them.
//!
//! Links are [`Jump`]s: they say where a link leads, and the interface decides how to get there.

use crate::engine::state::EngineState;
use crate::mmdb::geo;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::capture_context::CaptureSource;
use crate::networking::types::data_representation::DataRepr;
use crate::utils::check_updates::UpdateStatus;
use crate::view::i18n::Lang;

pub mod calls;
pub mod capture;
pub mod connections;
pub mod entries;
pub mod filter_index;
pub mod format;
pub mod i18n;
pub mod notifications;
pub mod overview;
pub mod packets;
pub mod release_notes;
pub mod settings;
pub mod statistics;
pub mod stream;
pub mod suggest;

/// The licenses of openpacketmon and of the software and data it uses
pub const LICENSES: &str = concat!(
    include_str!("../../NOTICE"),
    "\n\n",
    include_str!("../../THIRD-PARTY-LICENSES.txt")
);

/// Where a link on a page leads; the interface decides how to get there
#[derive(Debug, Clone, PartialEq)]
pub enum Jump {
    /// The details of a connection
    Connection(AddressPortPair),
    /// The connections of a connection filter
    Connections { filter: String },
    /// The packets of a display and a connection filter
    Packets { filter: String, connections: String },
    /// The statistics of a display and a connection filter
    Statistics { filter: String, connections: String },
    /// The notifications
    Notifications,
    /// The page of a call
    Call(calls::CallKey),
}

/// What the frame around every page shows
pub struct Status {
    /// Notifications not seen yet
    pub unread: usize,
    /// A newer release, if the update check found one
    pub update: Option<NewRelease>,
    /// There is no country database (hint on the pages that show countries)
    pub geo_missing: bool,
    /// A database of DB-IP is used: link to db-ip.com, as its license asks
    pub geo_dbip: bool,
    /// The capture at the top right; `None` without a capture
    pub capture: Option<CaptureChip>,
    /// The counts of the status bar, while a capture runs
    pub counts: Option<Counts>,
}

/// State of the capture in the header
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaptureMode {
    Running,
    Paused,
    /// A file has been read to its end
    Done,
}

/// The capture in the header: state, source and how long it runs
pub struct CaptureChip {
    pub mode: CaptureMode,
    /// The adapter, the file or the collector
    pub source: String,
    /// Time of the capture, like `4:05`
    pub time: String,
}

/// The numbers of the status bar
pub struct Counts {
    /// Packets of the capture
    pub packets: String,
    /// Packets in the packet store and its limit, with whether it keeps whole packets; `None` without a
    /// packet store
    pub store: Option<(String, String, bool)>,
    /// Packets dropped by the adapter and by openpacketmon
    pub dropped: String,
    /// Connections of the capture
    pub connections: String,
}

/// A newer release, for the note in the status bar
pub struct NewRelease {
    pub version: String,
    /// The day it was published
    pub date: String,
}

impl Status {
    pub fn read(t: Lang, state: &EngineState) -> Self {
        let country = state.mmdb_readers.country.get();
        let asn = state.mmdb_readers.asn.get();
        let running = state.is_running();
        Self {
            unread: state.unread_notifications,
            update: match &state.update_status {
                UpdateStatus::UpdateAvailable(release) => Some(NewRelease {
                    version: release.version.clone(),
                    date: release
                        .published
                        .map(|at| format::date(t, at))
                        .unwrap_or_default(),
                }),
                _ => None,
            },
            geo_missing: country.source().is_none(),
            geo_dbip: geo::is_dbip(&country) || geo::is_dbip(&asn),
            capture: running.then(|| CaptureChip {
                mode: if state.traffic_history.no_more_packets() {
                    CaptureMode::Done
                } else if state.frozen {
                    CaptureMode::Paused
                } else {
                    CaptureMode::Running
                },
                source: chip_source(&state.capture_source),
                time: format::clock(u64::from(state.traffic_history.ticks())),
            }),
            counts: running.then(|| counts(t, state)),
        }
    }
}

/// The source in the header: the adapter, the name of the file without its folder, the collector
fn chip_source(source: &CaptureSource) -> String {
    let name = source.display_name();
    match source {
        CaptureSource::File(_) => file_name(&name),
        _ => name,
    }
}

/// The name of a file without its folder
pub fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

fn counts(t: Lang, state: &EngineState) -> Counts {
    let data = state.info_traffic.tot_data_info;
    let packets = DataRepr::packets(false);
    Counts {
        packets: format::count(t, data.tot_data(packets)),
        store: state.packets.as_ref().map(|store| {
            let store = store.lock();
            (
                format::count(t, store.len() as u128),
                format::count(t, store.max_packets() as u128),
                !store.headers_only(),
            )
        }),
        dropped: format::count(
            t,
            state
                .info_traffic
                .dropped_packets
                .map_or(0, |dropped| dropped.total(packets, data)),
        ),
        connections: format::count(t, state.info_traffic.map.len() as u128),
    }
}

#[cfg(test)]
mod tests {
    use super::LICENSES;

    #[test]
    fn the_licenses_name_what_the_program_is_built_on() {
        assert!(LICENSES.starts_with("openpacketmon\n"));
        for part in [
            "Giuliano Bellini",
            "Fira Sans",
            "Fira Code",
            "IANA",
            "DB-IP",
            "eframe ",
        ] {
            assert!(LICENSES.contains(part), "{part}");
        }
    }
}
