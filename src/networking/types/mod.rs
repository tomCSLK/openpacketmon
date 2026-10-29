// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/mod.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

pub mod address_port_pair;
pub mod asn;
pub mod bogon;
pub mod capture_context;
pub mod data_info;
pub mod data_info_host;
pub mod data_representation;
pub mod dropped_packets;
pub mod host;
pub mod info_address_port_pair;
pub mod info_traffic;
pub mod ipfix_exporter;
pub mod latency;
pub mod message_type;
pub mod my_device;
pub mod program;
pub mod program_lookup;
pub mod service;
pub mod service_category;
pub mod service_query;
pub mod traffic_direction;
pub mod traffic_type;
