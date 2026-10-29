// Modified for openpacketmon (2026), see NOTICE. Based on `src/notifications/types/notifications.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use serde::{Deserialize, Serialize};

use crate::config::conf::deserialize_or_default;
use crate::notifications::types::sound::Sound;

/// Used to contain the notifications configuration set by the user; older configurations with
/// data thresholds, blacklist and remote notifications still load, those fields are ignored
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct Notifications {
    #[serde(deserialize_with = "deserialize_or_default")]
    pub volume: u8,
    #[serde(deserialize_with = "deserialize_or_default")]
    pub favorite_notification: SimpleNotification,
}

impl Default for Notifications {
    fn default() -> Self {
        Notifications {
            volume: 50,
            favorite_notification: SimpleNotification {
                is_active: false,
                sound: Sound::Pop,
            },
        }
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug, Copy, Default)]
#[serde(default)]
pub struct SimpleNotification {
    /// Flag to determine if this notification is enabled
    #[serde(deserialize_with = "deserialize_or_default")]
    pub is_active: bool,
    /// The sound to emit
    #[serde(deserialize_with = "deserialize_or_default")]
    pub sound: Sound,
}

impl SimpleNotification {
    #[cfg(test)]
    /// Constructor when the notification is in use
    pub fn on(sound: Sound) -> Self {
        Self {
            is_active: true,
            sound,
        }
    }

    #[cfg(test)]
    /// Constructor when the notification is not in use. Note that sound is used here for caching, although it won't actively be used.
    pub fn off(sound: Sound) -> Self {
        Self {
            is_active: false,
            sound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_instantiate_simple_notification() {
        assert_eq!(
            SimpleNotification::on(Sound::Gulp),
            SimpleNotification {
                is_active: true,
                sound: Sound::Gulp
            }
        );
        assert_eq!(
            SimpleNotification::on(Sound::Swhoosh),
            SimpleNotification {
                is_active: true,
                sound: Sound::Swhoosh
            }
        );
        assert_eq!(
            SimpleNotification::off(Sound::Pop),
            SimpleNotification {
                is_active: false,
                sound: Sound::Pop
            }
        );
        assert_eq!(
            SimpleNotification::off(Sound::None),
            SimpleNotification {
                is_active: false,
                sound: Sound::None
            }
        );
    }
}
