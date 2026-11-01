// Modified for openpacketmon (2026), see NOTICE. Based on `src/gui/types/settings.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use serde::{Deserialize, Serialize};

use crate::config::conf::deserialize_or_default;
use crate::notifications::types::notifications::Notifications;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Settings {
    #[serde(deserialize_with = "deserialize_or_default")]
    pub mmdb_country: String,
    #[serde(deserialize_with = "deserialize_or_default")]
    pub mmdb_asn: String,
    // ---------------------------------------------------------------------------------------------
    #[serde(deserialize_with = "deserialize_or_default")]
    pub notifications: Notifications,
    // ---------------------------------------------------------------------------------------------
    /// Light or dark design of the window
    #[serde(deserialize_with = "deserialize_or_default")]
    pub theme: ThemeChoice,
    /// The packet list of the window without the colors of the packet kinds
    #[serde(deserialize_with = "deserialize_or_default")]
    pub plain_packets: bool,
}

/// The design of the window
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeChoice {
    /// Follows the desktop
    #[default]
    System,
    Light,
    Dark,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_of_older_versions_still_load() {
        // the IP blacklist and the threshold, blacklist and remote notifications were removed
        let settings: Settings = toml::from_str(
            r#"
            mmdb_country = "country.mmdb"
            ip_blacklist = "/home/user/blacklist.txt"

            [notifications]
            volume = 30
            remote_notifications = { is_active = true, url = "https://example.com" }
            "#,
        )
        .unwrap();
        assert_eq!(settings.mmdb_country, "country.mmdb");
        assert_eq!(settings.notifications.volume, 30);
    }
}
