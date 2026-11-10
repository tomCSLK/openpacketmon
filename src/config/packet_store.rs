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
    /// Most memory the packet bytes may take, in MiB, whatever the number of packets
    #[serde(deserialize_with = "deserialize_or_default")]
    pub megabytes: usize,
}

impl PacketStoreConf {
    /// Fewest packets that can be set as the limit
    pub const MIN_PACKETS: usize = 1_000;
    /// Most packets that can be set as the limit
    pub const MAX_PACKETS: usize = 2_000_000;
    /// Choices for the memory of the packet bytes, in MiB
    pub const MEGABYTES_CHOICES: [usize; 4] = [256, 512, 1024, 2048];
    const DEFAULT_MEGABYTES: usize = 512;
    /// Bytes kept of each packet with `headers_only`: enough for all headers up to TCP with options
    pub const HEADERS_SNAP: usize = 128;

    /// Whether the store keeps whole packets, so the capture has to take them in full
    pub fn full_packets(self) -> bool {
        self.enabled && !self.headers_only
    }

    /// The memory of the packet bytes in MiB, one of the choices
    pub fn max_megabytes(self) -> usize {
        if Self::MEGABYTES_CHOICES.contains(&self.megabytes) {
            self.megabytes
        } else {
            Self::DEFAULT_MEGABYTES
        }
    }

    /// The memory of the packet bytes
    pub fn max_bytes(self) -> usize {
        self.max_megabytes() * 1024 * 1024
    }

    /// The number of packets to keep, within the range offered
    pub fn max_packets(self) -> usize {
        if (Self::MIN_PACKETS..=Self::MAX_PACKETS).contains(&self.max_packets) {
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
            megabytes: Self::DEFAULT_MEGABYTES,
        }
    }
}
