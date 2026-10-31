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
