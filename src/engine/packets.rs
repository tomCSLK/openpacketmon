//! Packets of the current capture kept in memory for the packet list, the packet details and the
//! PCAP download.
//!
//! The capture thread pushes every packet into a [`PacketStore`] it shares with the user interface.
//! The store is a ring: when the number of packets or their bytes exceed the limits, the oldest
//! packets are dropped. Packets are numbered from 1 in capture order, like in Wireshark; numbers
//! stay with their packets when older ones are dropped.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sniffnet_packet_parser::LinkType;

use crate::config::packet_store::PacketStoreConf;
use crate::engine::analysis::Analysis;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::utils::types::timestamp::Timestamp;

/// A captured packet; cloning it is cheap (the bytes are shared)
#[derive(Clone)]
pub struct StoredPacket {
    /// Position in the capture, starting at 1
    pub number: u64,
    pub timestamp: Timestamp,
    /// Length on the wire
    pub orig_len: u32,
    /// The captured bytes (only the headers with `headers_only`)
    pub data: Arc<[u8]>,
    /// The connection the packet belongs to and its direction; `None` for packets without IP
    /// (LLDP, spanning tree …), which only the packet list shows
    pub conn: Option<(AddressPortPair, TrafficDirection)>,
}

pub struct PacketStore {
    /// Different for every store, so the results of a filter are not mixed up between captures
    id: u64,
    packets: VecDeque<StoredPacket>,
    /// Bytes of all stored packets
    bytes: usize,
    max_packets: usize,
    max_bytes: usize,
    /// Bytes kept of each packet, `None` for all of them
    snap: Option<usize>,
    /// Number of the next packet
    next_number: u64,
    /// Time of the first packet of the capture (also when it has been dropped), for relative times
    first_timestamp: Option<Timestamp>,
    link_type: LinkType,
}

/// The store shared by the capture thread and the user interface, with the analysis of its
/// packets; lock the analysis before the store, never the other way round
#[derive(Clone)]
pub struct SharedPacketStore {
    packets: Arc<Mutex<PacketStore>>,
    analysis: Arc<Mutex<Analysis>>,
}

impl SharedPacketStore {
    pub fn new(conf: PacketStoreConf, link_type: LinkType) -> Self {
        Self {
            packets: Arc::new(Mutex::new(PacketStore::new(conf, link_type))),
            analysis: Arc::default(),
        }
    }

    /// The analysis of the packets; it may lock the store meanwhile, but not the other way round
    pub fn analysis(&self) -> MutexGuard<'_, Analysis> {
        self.analysis.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Exclusive access; keep the guard only briefly, the capture waits for it
    pub fn lock(&self) -> MutexGuard<'_, PacketStore> {
        self.packets.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PacketStore {
    pub fn new(conf: PacketStoreConf, link_type: LinkType) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            packets: VecDeque::new(),
            bytes: 0,
            max_packets: conf.max_packets(),
            max_bytes: conf.max_bytes(),
            snap: conf.headers_only.then_some(PacketStoreConf::HEADERS_SNAP),
            next_number: 1,
            first_timestamp: None,
            link_type,
        }
    }

    /// Adds a packet, dropping the oldest ones beyond the limits
    pub fn push(
        &mut self,
        timestamp: Timestamp,
        orig_len: u32,
        mut data: Box<[u8]>,
        conn: Option<(AddressPortPair, TrafficDirection)>,
    ) {
        if let Some(snap) = self.snap
            && data.len() > snap
        {
            data = data[..snap].into();
        }
        self.first_timestamp.get_or_insert(timestamp);
        self.bytes += data.len();
        self.packets.push_back(StoredPacket {
            number: self.next_number,
            timestamp,
            orig_len,
            data: Arc::from(data),
            conn,
        });
        self.next_number += 1;
        while self.packets.len() > self.max_packets || self.bytes > self.max_bytes {
            match self.packets.pop_front() {
                Some(old) => self.bytes -= old.data.len(),
                None => break,
            }
        }
    }

    /// Packets in the store
    pub fn len(&self) -> usize {
        self.packets.len()
    }

    /// Bytes of the packets in the store
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Packets dropped because of the limits
    pub fn dropped(&self) -> u64 {
        self.next_number - 1 - self.packets.len() as u64
    }

    /// Most packets kept
    pub fn max_packets(&self) -> usize {
        self.max_packets
    }

    /// Whether only the headers of the packets are kept
    pub fn headers_only(&self) -> bool {
        self.snap.is_some()
    }

    pub fn link_type(&self) -> LinkType {
        self.link_type
    }

    pub fn first_timestamp(&self) -> Option<Timestamp> {
        self.first_timestamp
    }

    /// The packet with this number, if it is still stored
    pub fn get(&self, number: u64) -> Option<&StoredPacket> {
        let first = self.packets.front()?.number;
        let index = usize::try_from(number.checked_sub(first)?).ok()?;
        self.packets.get(index)
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Number of the newest packet (0 before the first)
    pub fn last_number(&self) -> u64 {
        self.next_number - 1
    }

    /// The stored packets after the one with this number, oldest first
    pub fn after(&self, number: u64) -> impl Iterator<Item = &StoredPacket> {
        let first = self.packets.front().map_or(u64::MAX, |p| p.number);
        let skip =
            usize::try_from(number.saturating_add(1).saturating_sub(first)).unwrap_or(usize::MAX);
        self.packets.iter().skip(skip)
    }

    /// All stored packets, oldest first
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &StoredPacket> + ExactSizeIterator {
        self.packets.iter()
    }

    /// The packets as a PCAP file (the classic format every tool reads, microsecond timestamps)
    pub fn to_pcap<'a>(&self, packets: impl Iterator<Item = &'a StoredPacket>) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0xa1b2_c3d4_u32.to_le_bytes());
        out.extend_from_slice(&2_u16.to_le_bytes());
        out.extend_from_slice(&4_u16.to_le_bytes());
        out.extend_from_slice(&0_i32.to_le_bytes()); // time zone
        out.extend_from_slice(&0_u32.to_le_bytes()); // accuracy
        out.extend_from_slice(&262_144_u32.to_le_bytes()); // snap length
        out.extend_from_slice(&link_type_number(self.link_type).to_le_bytes());
        for p in packets {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (secs, usecs) = (p.timestamp.secs() as u32, p.timestamp.usecs() as u32);
            #[allow(clippy::cast_possible_truncation)]
            let incl_len = p.data.len() as u32;
            out.extend_from_slice(&secs.to_le_bytes());
            out.extend_from_slice(&usecs.to_le_bytes());
            out.extend_from_slice(&incl_len.to_le_bytes());
            out.extend_from_slice(&p.orig_len.to_le_bytes());
            out.extend_from_slice(&p.data);
        }
        out
    }
}

/// The `LINKTYPE_` number of the PCAP file header
fn link_type_number(link_type: LinkType) -> u32 {
    match link_type {
        LinkType::Null(l)
        | LinkType::Ethernet(l)
        | LinkType::RawIp(l)
        | LinkType::Loop(l)
        | LinkType::IPv4(l)
        | LinkType::IPv6(l)
        | LinkType::LinuxSll(l)
        | LinkType::LinuxSll2(l)
        | LinkType::Unsupported(l) => u32::try_from(l.0).unwrap_or(1),
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use pcap::Linktype;
    use sniffnet_packet_parser::Protocol;

    use super::*;

    fn key() -> AddressPortPair {
        AddressPortPair {
            src_ip: IpAddr::from([192, 0, 2, 1]),
            src_port: Some(1234),
            dst_ip: IpAddr::from([198, 51, 100, 7]),
            dst_port: Some(443),
            protocol: Protocol::Tcp,
            exporter: None,
        }
    }

    fn store(max_packets: usize, headers_only: bool) -> PacketStore {
        let mut store = PacketStore::new(
            PacketStoreConf {
                megabytes: 512,
                enabled: true,
                max_packets,
                headers_only,
            },
            LinkType::from_pcap(Linktype::ETHERNET),
        );
        // the tests use small limits
        store.max_packets = max_packets;
        store
    }

    fn push(store: &mut PacketStore, secs: i64, len: usize) {
        store.push(
            Timestamp::new(secs, 250),
            u32::try_from(len).unwrap(),
            vec![0xab; len].into(),
            Some((key(), TrafficDirection::Outgoing)),
        );
    }

    #[test]
    fn the_oldest_packets_are_dropped_and_numbers_stay() {
        let mut s = store(3, false);
        for i in 0..5 {
            push(&mut s, 100 + i, 60);
        }
        assert_eq!(s.len(), 3);
        assert_eq!(s.dropped(), 2);
        assert_eq!(s.bytes(), 180);
        assert!(s.get(2).is_none());
        assert_eq!(s.get(3).unwrap().timestamp.secs(), 102);
        assert_eq!(s.get(5).unwrap().number, 5);
        assert!(s.get(6).is_none());
        assert_eq!(s.first_timestamp().unwrap().secs(), 100);
    }

    #[test]
    fn the_byte_limit_applies_too() {
        let mut s = store(10, false);
        s.max_bytes = 250;
        for i in 0..5 {
            push(&mut s, i, 100);
        }
        assert_eq!(s.len(), 2);
        assert_eq!(s.bytes(), 200);
    }

    #[test]
    fn headers_only_cuts_the_packets() {
        let mut s = store(10, true);
        push(&mut s, 1, 1500);
        let p = s.get(1).unwrap();
        assert_eq!(p.data.len(), PacketStoreConf::HEADERS_SNAP);
        assert_eq!(p.orig_len, 1500);
        assert!(s.headers_only());
    }

    #[test]
    fn packets_are_written_as_pcap() {
        let mut s = store(10, false);
        push(&mut s, 1_700_000_000, 60);
        push(&mut s, 1_700_000_001, 42);
        let pcap = s.to_pcap(s.iter());
        assert_eq!(pcap.len(), 24 + 16 + 60 + 16 + 42);
        assert_eq!(&pcap[..4], &[0xd4, 0xc3, 0xb2, 0xa1]);
        assert_eq!(u32::from_le_bytes(pcap[20..24].try_into().unwrap()), 1); // Ethernet
        assert_eq!(
            u32::from_le_bytes(pcap[24..28].try_into().unwrap()),
            1_700_000_000
        );
        assert_eq!(u32::from_le_bytes(pcap[28..32].try_into().unwrap()), 250);
        assert_eq!(u32::from_le_bytes(pcap[32..36].try_into().unwrap()), 60);

        // the written file can be read again by libpcap
        let dir = std::env::temp_dir().join(format!("openpacketmon-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.pcap");
        std::fs::write(&path, &pcap).unwrap();
        let mut cap = pcap::Capture::from_file(&path).unwrap();
        assert_eq!(cap.get_datalink(), Linktype::ETHERNET);
        assert_eq!(cap.next_packet().unwrap().data.len(), 60);
        assert_eq!(cap.next_packet().unwrap().header.len, 42);
        assert!(cap.next_packet().is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
