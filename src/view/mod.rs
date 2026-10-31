//! What the pages of the user interface show, without the user interface: the data of every page,
//! translated and formatted, as plain Rust values. The browser pages render them to HTML; nothing
//! here knows about HTML, addresses of a site or a web server, so another interface can use it.
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
    /// The stream a packet belongs to
    Stream(u64),
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
    /// This module is what another interface builds on: it knows no web server, no HTML templates
    /// and no addresses of the site
    #[test]
    fn the_view_knows_nothing_of_the_browser_pages() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/view");
        let mut files = 0;
        for file in std::fs::read_dir(dir).unwrap() {
            let path = file.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            // the words are put together, so this test does not find itself
            for word in [
                ["ax", "um"].concat(),
                ["ask", "ama"].concat(),
                ["crate::", "web"].concat(),
                ["hr", "ef"].concat(),
                ["<", "a "].concat(),
            ] {
                assert!(!text.contains(&word), "{} has {word}", path.display());
            }
            files += 1;
        }
        assert!(files > 10);
    }
}
