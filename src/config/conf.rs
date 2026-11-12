// Modified for openpacketmon (2026), see NOTICE. Based on `src/gui/types/conf.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::config::capture::CaptureConf;
use crate::config::config_device::ConfigDevice;
use crate::config::config_updates::ConfigUpdates;
use crate::config::export_pcap::ExportPcap;
#[cfg(not(test))]
use crate::config::favorite::FavoriteKey;
use crate::config::favorite::Favorites;
use crate::config::filters::Filters;
use crate::config::ipfix_socket::MyIpfixSocket;
use crate::config::mcp::McpConf;
use crate::config::packet_store::PacketStoreConf;
use crate::config::settings::Settings;
use crate::networking::types::capture_context::CaptureSourcePicklist;
use crate::networking::types::data_representation::DataRepr;
#[cfg(not(test))]
use crate::networking::types::program::Program;
use crate::report::types::sort_type::SortType;
#[cfg(not(test))]
use crate::utils::error_logger::{ErrorLogger, Location};
#[cfg(not(test))]
use crate::{APP_NAME, location};
#[cfg(not(test))]
use confy::ConfyError;
use serde::{Deserialize, Deserializer, Serialize};

pub static CONF: std::sync::LazyLock<Conf> = std::sync::LazyLock::new(Conf::load);

/// Application configurations structure
///
/// This structure holds all the configuration parameters for the application,
/// and is serialized/deserialized using `confy` crate to store/load from disk.
///
/// ### IMPORTANT NOTE
///
/// In order to load it in a robust, fault-tolerant, backward-compatible way,
/// there are various constraints to be satisfied when deserializing:
/// - missing fields must be filled with default values
///   - the main struct and all nested structs must implement `Default` and be annotated with `#[serde(default)]`
///   - this populates missing fields from the struct's `Default` implementation
/// - invalid fields must be replaced with default values
///   - all fields must be annotated with `#[serde(deserialize_with = "deserialize_or_default")]`
///   - this populates invalid fields from the field's type `Default` implementation
/// - extra fields must be ignored
///   - this is the default behavior of `serde`
/// - right after deserialization, certain fields must be sanitized
///   - this is to ensure that fields deserialized correctly but with "weird" values are fixed
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Conf {
    /// Capture source picklist, to select the source of the capture
    #[serde(deserialize_with = "deserialize_or_default")]
    pub capture_source_picklist: CaptureSourcePicklist,
    /// Import path for PCAP file
    #[serde(deserialize_with = "deserialize_or_default")]
    pub import_pcap_path: String,
    /// Data representation
    #[serde(deserialize_with = "deserialize_or_default")]
    pub data_repr: DataRepr,
    /// Host sort type (overview page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub host_sort_type: SortType,
    /// Service sort type (overview page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub service_sort_type: SortType,
    /// Program sort type (overview page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub program_sort_type: SortType,
    /// Report sort type (inspect page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub report_sort_type: SortType,
    /// Host favorites filter (overview page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub host_favorites_filter: bool,
    /// Service favorites filter (overview page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub service_favorites_filter: bool,
    /// Program favorites filter (overview page)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub program_favorites_filter: bool,
    // ---------------------------------------------------------------------------------------------
    /// Last selected network device name
    #[serde(deserialize_with = "deserialize_or_default")]
    pub device: ConfigDevice,
    /// Update checks configuration
    #[serde(deserialize_with = "deserialize_or_default")]
    pub updates: ConfigUpdates,
    /// IPFIX collector configuration (bind address and port)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub ipfix_socket: MyIpfixSocket,
    /// BPF filter program to be applied to the capture
    #[serde(deserialize_with = "deserialize_or_default")]
    pub filters: Filters,
    /// How adapters are opened (promiscuous mode)
    #[serde(deserialize_with = "deserialize_or_default")]
    pub capture: CaptureConf,
    /// Information about PCAP file export
    #[serde(deserialize_with = "deserialize_or_default")]
    pub export_pcap: ExportPcap,
    /// Parameters from settings pages
    #[serde(deserialize_with = "deserialize_or_default")]
    pub settings: Settings,
    /// Favorite hosts, services, and programs
    #[serde(deserialize_with = "deserialize_or_default")]
    pub favorites: Favorites,
    /// Language of the browser UI (`de`, `en`); empty to follow the browser
    #[serde(deserialize_with = "deserialize_or_default")]
    pub language: String,
    /// Connections per page in the connection list
    #[serde(deserialize_with = "deserialize_or_default")]
    pub page_size: usize,
    /// Packets kept in memory for the packet list and details
    #[serde(deserialize_with = "deserialize_or_default")]
    pub packet_store: PacketStoreConf,
    /// Access for AI assistants
    #[serde(deserialize_with = "deserialize_or_default")]
    pub mcp: McpConf,
}

impl Conf {
    pub(crate) const FILE_NAME: &'static str = "conf";

    /// This should only be used directly to load fresh configurations;
    /// use `CONF` instead to access the initial instance
    #[cfg(not(test))]
    fn load() -> Self {
        let mut conf = if let Ok(conf) = confy::load::<Conf>(APP_NAME, Self::FILE_NAME) {
            conf
        } else {
            let _ = Conf::default().store();
            Conf::default()
        };

        // sanitize Conf...
        conf.sanitize();

        conf
    }

    #[cfg(not(test))]
    pub fn store(&self) -> Result<(), ConfyError> {
        confy::store(APP_NAME, Self::FILE_NAME, self).log_err(location!())
    }

    #[cfg(not(test))]
    fn sanitize(&mut self) {
        // check sound volume validity
        if !(0..=100).contains(&self.settings.notifications.volume) {
            self.settings.notifications.volume = 50;
        }

        // Program::NotApplicable is not allowed in favorites
        self.favorites
            .remove(&FavoriteKey::Program(Program::NotApplicable));
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn deserialize_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: Deserializer<'de>,
{
    Ok(T::deserialize(deserializer).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use crate::config::conf::Conf;

    impl Conf {
        pub fn test_path() -> String {
            format!("{}/{}.toml", env!("CARGO_MANIFEST_DIR"), Self::FILE_NAME)
        }

        pub fn load() -> Self {
            confy::load_path::<Conf>(Conf::test_path()).unwrap_or_else(|_| Conf::default())
        }

        pub fn store(&self) -> Result<(), confy::ConfyError> {
            confy::store_path(Conf::test_path(), self)
        }
    }
}
