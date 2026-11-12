//! How adapters are opened for a live capture.

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct CaptureConf {
    /// Opens the adapter in promiscuous mode, like Wireshark does: it then also hands over frames
    /// for other MAC addresses, which is what a mirror port or a network TAP delivers
    #[serde(deserialize_with = "deserialize_or_on")]
    pub promiscuous: bool,
}

impl Default for CaptureConf {
    fn default() -> Self {
        Self { promiscuous: true }
    }
}

/// An invalid value falls back to the default, which is on (`deserialize_or_default` would turn
/// it off)
#[allow(clippy::unnecessary_wraps, reason = "serde expects a Result")]
fn deserialize_or_on<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(bool::deserialize(deserializer).unwrap_or(true))
}

#[cfg(test)]
mod tests {
    use crate::config::conf::Conf;

    #[test]
    fn promiscuous_mode_is_on_unless_turned_off() {
        assert!(Conf::default().capture.promiscuous);
        // a configuration of an older version has no such setting
        let old: Conf = toml::from_str("language = \"de\"\n").unwrap();
        assert!(old.capture.promiscuous);
        let invalid: Conf = toml::from_str("[capture]\npromiscuous = \"yes\"\n").unwrap();
        assert!(invalid.capture.promiscuous);
        let off: Conf = toml::from_str("[capture]\npromiscuous = false\n").unwrap();
        assert!(!off.capture.promiscuous);
        let written = toml::to_string(&off).unwrap();
        assert!(
            !toml::from_str::<Conf>(&written)
                .unwrap()
                .capture
                .promiscuous
        );
    }
}
