// Modified for openpacketmon (2026), see NOTICE. Based on `src/gui/types/config_updates.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::config::conf::deserialize_or_default;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct ConfigUpdates {
    #[serde(deserialize_with = "deserialize_or_default")]
    notify_updates: bool,
    #[serde(deserialize_with = "deserialize_or_default")]
    disable_checks: bool,
}

impl ConfigUpdates {
    pub fn disable_checks(self) -> bool {
        self.disable_checks
    }

    pub fn toggle_disable_checks(&mut self) {
        self.disable_checks = !self.disable_checks;
    }
}

impl Default for ConfigUpdates {
    fn default() -> Self {
        Self {
            notify_updates: true,
            disable_checks: false,
        }
    }
}
