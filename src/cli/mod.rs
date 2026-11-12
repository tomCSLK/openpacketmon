// Modified for openpacketmon (2026), see NOTICE. Based on `src/cli/mod.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::APP_NAME;
use crate::config::conf::{CONF, Conf};
use crate::utils::formatted_strings::APP_VERSION;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = APP_NAME,
    bin_name = APP_NAME,
    version = APP_VERSION,
    about = "Monitor your network traffic",
    after_help = "Commands:\n  mcp  Answer an AI assistant on the standard streams; the assistant starts this itself"
)]
pub(crate) struct Args {
    /// Start capturing from this network adapter right away (on Windows also its description)
    #[arg(short, long, value_name = "NAME", default_missing_value = CONF.device.device_name.as_str(), num_args = 0..=1)]
    pub adapter: Option<String>,
    /// Analyze the packets of this PCAP file right away
    #[arg(short, long, value_name = "FILE", conflicts_with = "adapter")]
    pub import: Option<String>,
    /// List the network adapters and exit
    #[arg(short, long, exclusive = true)]
    pub list_adapters: bool,
    /// Print the path to the configuration file
    #[arg(short, long, exclusive = true)]
    config_path: bool,
    /// Restore default settings
    #[arg(short, long, exclusive = true)]
    restore_default: bool,
}

impl Args {
    /// Handle and return CLI arguments
    #[allow(clippy::print_stdout, clippy::print_stderr)]
    pub fn handle() -> Self {
        let args = Args::parse();

        if args.restore_default {
            if Conf::default().store().is_ok() {
                println!("Restored default settings");
                std::process::exit(0);
            }
            eprintln!("Could not restore default settings");
            std::process::exit(1);
        }

        if args.config_path {
            if let Ok(config_path) = confy::get_configuration_file_path(APP_NAME, Conf::FILE_NAME) {
                println!("{}", config_path.display());
                std::process::exit(0);
            }
            eprintln!("Could not retrieve configuration file path");
            std::process::exit(1);
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::config::conf::Conf;
    use crate::config::config_device::ConfigDevice;
    use crate::config::config_updates::ConfigUpdates;
    use crate::config::export_pcap::ExportPcap;
    use crate::config::favorite::{FavoriteKey, Favorites};
    use crate::config::filters::Filters;
    use crate::config::mcp::McpConf;
    use crate::config::settings::Settings;
    use crate::networking::types::capture_context::CaptureSourcePicklist;
    use crate::networking::types::data_representation::DataRepr;
    use crate::notifications::types::notifications::Notifications;
    use crate::report::types::sort_type::SortType;

    #[test]
    #[serial]
    fn test_restore_default_configs() {
        // initial configs stored are the default ones
        assert_eq!(Conf::load(), Conf::default());
        let modified_conf = Conf {
            settings: Settings {
                mmdb_country: "countrymmdb".to_string(),
                mmdb_asn: "asnmmdb".to_string(),
                notifications: Notifications {
                    volume: 100,
                    favorite_notification: Default::default(),
                },
                ..Settings::default()
            },
            favorites: Favorites::from([FavoriteKey::Address([10, 0, 0, 1].into())]),
            device: ConfigDevice {
                device_name: "hey-hey".to_string(),
            },
            updates: ConfigUpdates::default(),
            capture_source_picklist: CaptureSourcePicklist::File,
            report_sort_type: SortType::Ascending,
            host_favorites_filter: false,
            service_favorites_filter: true,
            program_favorites_filter: false,
            host_sort_type: SortType::Descending,
            service_sort_type: SortType::Neutral,
            program_sort_type: SortType::Neutral,
            mcp: McpConf { enabled: true },
            filters: Filters {
                bpf: "tcp".to_string(),
                expanded: true,
            },
            capture: crate::config::capture::CaptureConf { promiscuous: false },
            import_pcap_path: "whole_day.pcapng".to_string(),
            ipfix_socket: Default::default(),
            export_pcap: ExportPcap::default(),
            data_repr: DataRepr::default(),
            language: "de".to_string(),
            page_size: 100,
            packet_store: crate::config::packet_store::PacketStoreConf {
                megabytes: 1024,
                enabled: false,
                max_packets: 10_000,
                headers_only: true,
            },
        };
        // we want to be sure that modified config is different from defaults
        assert_ne!(Conf::default(), modified_conf);
        //store modified configs
        modified_conf.store().unwrap();
        // assert they've been stored
        assert_eq!(Conf::load(), modified_conf);
        // restore defaults
        Conf::default().store().unwrap();
        // assert that defaults are stored
        assert_eq!(Conf::load(), Conf::default());

        let _ = std::fs::remove_file(Conf::test_path());
    }
}
