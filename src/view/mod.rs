//! What the pages of the user interface show, without the user interface: the data of every page,
//! translated and formatted, as plain Rust values. [`crate::gui`] draws them.
//!
//! Links are [`Jump`]s: they say where a link leads, and the interface decides how to get there.

use crate::engine::state::EngineState;
use crate::mmdb::geo;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::utils::check_updates::UpdateStatus;

pub mod capture;
pub mod connections;
pub mod entries;
pub mod filter_index;
pub mod format;
pub mod i18n;
pub mod notifications;
pub mod overview;
pub mod packets;
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
}

/// What the frame around every page shows
pub struct Status {
    /// Notifications not seen yet
    pub unread: usize,
    /// A newer release, if the update check found one
    pub update: Option<String>,
    /// There is no country database (hint on the pages that show countries)
    pub geo_missing: bool,
    /// A database of DB-IP is used: link to db-ip.com, as its license asks
    pub geo_dbip: bool,
}

impl Status {
    pub fn read(state: &EngineState) -> Self {
        let country = state.mmdb_readers.country.get();
        let asn = state.mmdb_readers.asn.get();
        Self {
            unread: state.unread_notifications,
            update: match &state.update_status {
                UpdateStatus::UpdateAvailable(version) => Some(version.clone()),
                _ => None,
            },
            geo_missing: country.source().is_none(),
            geo_dbip: geo::is_dbip(&country) || geo::is_dbip(&asn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LICENSES;

    #[test]
    fn the_licenses_name_what_the_program_is_built_on() {
        assert!(LICENSES.starts_with("openpacketmon\n"));
        for part in ["Giuliano Bellini", "Inter", "IANA", "DB-IP", "eframe "] {
            assert!(LICENSES.contains(part), "{part}");
        }
    }
}
