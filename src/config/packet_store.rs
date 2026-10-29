//! Settings of the packet store (packet list, packet details, PCAP download)

use serde::{Deserialize, Serialize};

use crate::config::conf::deserialize_or_default;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct PacketStoreConf {
    /// Whether the packets of a capture are kept in memory
    #[serde(deserialize_with = "deserialize_or_default")]
    pub enabled: bool,
    /// Most packets kept; older ones are dropped first
    #[serde(deserialize_with = "deserialize_or_default")]
    pub max_packets: usize,
    /// Keep only the first bytes of each packet (the headers), not the content
    #[serde(deserialize_with = "deserialize_or_default")]
    pub headers_only: bool,
}

impl PacketStoreConf {
    /// Choices for the number of packets kept
    pub const MAX_PACKETS_CHOICES: [usize; 4] = [10_000, 100_000, 250_000, 500_000];
    /// Most memory the packet bytes may take, whatever the number of packets
    pub const MAX_BYTES: usize = 512 * 1024 * 1024;
    /// Bytes kept of each packet with `headers_only`: enough for all headers up to TCP with options
    pub const HEADERS_SNAP: usize = 128;

    /// Whether the store keeps whole packets, so the capture has to take them in full
    pub fn full_packets(self) -> bool {
        self.enabled && !self.headers_only
    }

    /// The number of packets to keep, one of the choices
    pub fn max_packets(self) -> usize {
        if Self::MAX_PACKETS_CHOICES.contains(&self.max_packets) {
            self.max_packets
        } else {
            Self::default().max_packets
        }
    }
}

impl Default for PacketStoreConf {
    fn default() -> Self {
        Self {
            enabled: true,
            max_packets: 100_000,
            headers_only: false,
        }
    }
}
