// Modified for openpacketmon (2026), see NOTICE. Based on `lib/sniffnet-packet-parser/src/lib.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

#![doc = include_str!("../README.md")]

#[cfg(feature = "full")]
mod arp_type;
#[cfg(feature = "full")]
mod headers;
#[cfg(feature = "full")]
mod icmp_type;
#[cfg(feature = "full")]
mod igmp_type;
// the right import of three functions of the standard library in the test programs under Windows
#[cfg(all(test, windows, target_env = "msvc"))]
#[path = "../../../capture/src/link_order.rs"]
mod link_order;
#[cfg(feature = "full")]
mod link_type;
#[cfg(feature = "full")]
mod packet;
mod protocol;

#[cfg(feature = "full")]
pub use arp_type::ArpType;
#[cfg(feature = "full")]
pub use headers::{LinkInfo, NetInfo, TransportInfo};
#[cfg(feature = "full")]
pub use icmp_type::{IcmpType, IcmpTypeV4, IcmpTypeV6};
#[cfg(feature = "full")]
pub use igmp_type::IgmpType;
#[cfg(feature = "full")]
pub use link_type::LinkType;
#[cfg(feature = "full")]
pub use packet::ParsedPacket;
pub use protocol::Protocol;
