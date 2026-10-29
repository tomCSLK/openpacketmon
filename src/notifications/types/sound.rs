// Modified for openpacketmon (2026), see NOTICE. Based on `src/notifications/types/sound.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Enum representing the possible notification sounds.
///
/// openpacketmon does not play sounds itself; the browser UI decides how to signal a notification.
/// The value is kept so the setting survives and stays compatible with existing configuration files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sound {
    Gulp,
    #[default]
    Pop,
    Swhoosh,
    None,
}

impl fmt::Display for Sound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Sound {
    pub(crate) const ALL: [Sound; 4] = [Sound::Gulp, Sound::Pop, Sound::Swhoosh, Sound::None];
}
