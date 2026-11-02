// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/my_device.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use pcap::{Address, Device};

use sniffnet_packet_parser::LinkType;

/// Represents the current inspected device.
/// Used to keep in sync the device addresses in case of changes
/// (e.g., device not connected to the internet acquires new IP address)
#[derive(Clone, Debug)]
pub struct MyDevice {
    name: String,
    desc: Option<String>,
    addresses: Vec<Address>,
    link_type: Option<LinkType>,
}

impl MyDevice {
    pub fn from_pcap_device(device: Device) -> Self {
        MyDevice {
            name: device.name,
            desc: device.desc,
            addresses: device.addresses,
            link_type: None,
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_desc(&self) -> Option<&String> {
        self.desc.as_ref()
    }

    /// The description, if the adapter has one that is not empty
    fn non_empty_desc(&self) -> Option<&String> {
        self.desc.as_ref().filter(|d| !d.trim().is_empty())
    }

    /// The name shown to the user. Under Windows the name is `\Device\NPF_{GUID}`, so the
    /// description is shown instead (the name stays in use inside)
    pub fn display_name(&self) -> String {
        if cfg!(windows)
            && let Some(desc) = self.non_empty_desc()
        {
            return desc.clone();
        }
        self.name.clone()
    }

    /// The line shown below [`Self::display_name`]: the description, unless it is the name already
    pub fn display_desc(&self) -> String {
        if cfg!(windows) && self.non_empty_desc().is_some() {
            return String::new();
        }
        self.desc.clone().unwrap_or_default()
    }

    /// Whether the user means this adapter: its name, or under Windows also its description
    pub fn is_called(&self, name: &str) -> bool {
        self.name == name
            || cfg!(windows)
                && self
                    .non_empty_desc()
                    .is_some_and(|d| d.eq_ignore_ascii_case(name))
    }

    pub fn get_addresses(&self) -> &Vec<Address> {
        &self.addresses
    }

    pub fn set_addresses(&mut self, addresses: Vec<Address>) {
        self.addresses = addresses;
    }

    pub fn get_link_type(&self) -> Option<LinkType> {
        self.link_type
    }

    pub fn set_link_type(&mut self, link_type: Option<LinkType>) {
        self.link_type = link_type;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, desc: Option<&str>) -> MyDevice {
        MyDevice {
            name: name.to_owned(),
            desc: desc.map(str::to_owned),
            addresses: Vec::new(),
            link_type: None,
        }
    }

    #[test]
    fn windows_shows_the_description_instead_of_the_name() {
        let npcap = device(r"\Device\NPF_{0D2B}", Some("Intel(R) Wi-Fi 6E AX211"));
        assert!(npcap.is_called(r"\Device\NPF_{0D2B}"));
        if cfg!(windows) {
            assert_eq!(npcap.display_name(), "Intel(R) Wi-Fi 6E AX211");
            assert_eq!(npcap.display_desc(), "");
            assert!(npcap.is_called("intel(r) wi-fi 6e ax211"));
        } else {
            assert_eq!(npcap.display_name(), r"\Device\NPF_{0D2B}");
            assert_eq!(npcap.display_desc(), "Intel(R) Wi-Fi 6E AX211");
            assert!(!npcap.is_called("Intel(R) Wi-Fi 6E AX211"));
        }
        // without a description the name is all there is
        for desc in [None, Some(""), Some("  ")] {
            let plain = device("eth0", desc);
            assert_eq!(plain.display_name(), "eth0");
            assert!(!plain.is_called(""));
        }
    }
}
