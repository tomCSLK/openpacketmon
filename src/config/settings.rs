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
    // ---------------------------------------------------------------------------------------------
    /// How calls (SIP, RTP) are found
    #[serde(deserialize_with = "deserialize_or_default")]
    pub calls: Calls,
}

/// Settings → Calls
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct Calls {
    /// RTP is also recognized without SIP, by the form of a flow
    pub rtp_without_sip: bool,
    /// SIP is also recognized on other ports than 5060, by its first line
    pub sip_any_port: bool,
    /// The day the note on § 201 of the criminal code was confirmed (`2026-11-10`), empty while it was not
    pub law_confirmed: String,
}

impl Default for Calls {
    fn default() -> Self {
        Self {
            rtp_without_sip: true,
            sip_any_port: true,
            law_confirmed: String::new(),
        }
    }
}

impl Calls {
    /// Makes the dissection and the analysis follow these settings
    pub fn apply(&self) {
        crate::engine::analysis::voip::set_without_sip(self.rtp_without_sip);
        crate::dissect::sip::set_any_port(self.sip_any_port);
    }
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
        // the IP blacklist, the threshold, blacklist and remote notifications and the packet list
        // without colors (0.4) were removed
        let settings: Settings = toml::from_str(
            r#"
            mmdb_country = "country.mmdb"
            ip_blacklist = "/home/user/blacklist.txt"
            plain_packets = true

            [notifications]
            volume = 30
            remote_notifications = { is_active = true, url = "https://example.com" }
            "#,
        )
        .unwrap();
        assert_eq!(settings.mmdb_country, "country.mmdb");
        assert_eq!(settings.notifications.volume, 30);
        // both ways of finding calls are on unless turned off
        assert_eq!(settings.calls, Calls::default());
        assert!(settings.calls.rtp_without_sip && settings.calls.sip_any_port);
        let off: Settings = toml::from_str("[calls]\nrtp_without_sip = false\n").unwrap();
        assert!(!off.calls.rtp_without_sip && off.calls.sip_any_port);
    }
}
