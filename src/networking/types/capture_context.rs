// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/capture_context.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::config::conf::Conf;
use crate::config::filters::Filters;
use crate::config::ipfix_socket::MyIpfixSocket;
use crate::location;
use crate::networking::types::capture_helper::CaptureHelper;
use crate::networking::types::my_device::MyDevice;
use crate::utils::error_logger::{ErrorLogger, Location};
use pcap::{Address, Capture, Device, Error, Linktype, Packet, Savefile, Stat};
use serde::{Deserialize, Serialize};
use sniffnet_packet_parser::LinkType;
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub enum CaptureContext {
    Live(Live),
    LiveWithSavefile(LiveWithSavefile),
    Offline(Offline),
    Ipfix(UdpSocket),
    Error(String),
}

/// A problem the capture backend has encountered
pub enum CaptureError {
    /// The capture failed to start (fatal)
    Fatal(String),
    /// The IPFIX collector is running but what reaches it isn't decodable at the moment (warning)
    IpfixUndecodable,
}

impl CaptureContext {
    /// `full_packets`: the packet store keeps whole packets, so they must be captured in full
    pub fn new(
        source: &CaptureSource,
        pcap_out_path: Option<&String>,
        filters: &Filters,
        full_packets: bool,
    ) -> Self {
        let mut cap_type = match source {
            CaptureSource::Device(device) => {
                let full = pcap_out_path.is_some() || full_packets;
                match CaptureType::from_device(device, full, filters) {
                    Ok(c) => c,
                    Err(e) => return Self::Error(e),
                }
            }
            CaptureSource::File(file) => match CaptureType::from_file(file) {
                Ok(c) => c,
                Err(e) => return Self::Error(e),
            },
            CaptureSource::Ipfix(ipfix) => {
                return Self::new_ipfix(&ipfix.socket).unwrap_or_else(Self::Error);
            }
        };

        // only apply BPF filter if it is active, and return an error if it fails to apply
        // (a live capture gets it from the helper)
        if filters.is_some_filter_active()
            && let CaptureType::Offline(cap) = &mut cap_type
            && let Err(e) = cap.filter(filters.bpf(), true)
        {
            return Self::Error(e.to_string());
        }

        let cap = match cap_type {
            CaptureType::Live(cap) => cap,
            CaptureType::Offline(cap) => return Self::new_offline(cap),
        };

        if let Some(out_path) = pcap_out_path {
            let savefile_res =
                Capture::dead(Linktype(cap.link_type())).and_then(|dead| dead.savefile(out_path));
            match savefile_res {
                Ok(s) => Self::new_live_with_savefile(cap, s),
                Err(e) => Self::Error(e.to_string()),
            }
        } else {
            Self::new_live(cap)
        }
    }

    fn new_live(cap: CaptureHelper) -> Self {
        Self::Live(Live { cap })
    }

    fn new_live_with_savefile(cap: CaptureHelper, savefile: Savefile) -> Self {
        Self::LiveWithSavefile(LiveWithSavefile {
            live: Live { cap },
            savefile,
        })
    }

    fn new_offline(cap: Capture<pcap::Offline>) -> Self {
        Self::Offline(Offline { cap })
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Error(e) => Some(e),
            _ => None,
        }
    }

    pub fn consume(self) -> (Option<CaptureType>, Option<Savefile>) {
        match self {
            Self::Live(on) => (Some(CaptureType::Live(on.cap)), None),
            Self::LiveWithSavefile(onws) => {
                (Some(CaptureType::Live(onws.live.cap)), Some(onws.savefile))
            }
            Self::Offline(off) => (Some(CaptureType::Offline(off.cap)), None),
            Self::Ipfix(_) | Self::Error(_) => (None, None),
        }
    }

    pub fn link_type(&self) -> Option<LinkType> {
        match self {
            Self::Live(on) => Some(LinkType::from_pcap(Linktype(on.cap.link_type()))),
            Self::LiveWithSavefile(onws) => {
                Some(LinkType::from_pcap(Linktype(onws.live.cap.link_type())))
            }
            Self::Offline(off) => Some(LinkType::from_pcap(off.cap.get_datalink())),
            Self::Ipfix(_) | Self::Error(_) => None,
        }
    }

    fn new_ipfix(ipfix_socket: &MyIpfixSocket) -> Result<Self, String> {
        const IPFIX_READ_TIMEOUT: Duration = Duration::from_millis(150);
        const RETRY_BIND_DELAY: Duration = Duration::from_millis(20);
        let socket_addr = ipfix_socket.socket_addr()?;

        // restarting a capture on the same port races the just finished collector
        // retry across its read window rather than failing on the socket that is about to go away
        let give_up_at = Instant::now() + IPFIX_READ_TIMEOUT * 3;
        let bind_socket = |socket_addr: SocketAddr| loop {
            match UdpSocket::bind(socket_addr) {
                Err(e)
                    if e.kind() == std::io::ErrorKind::AddrInUse && Instant::now() < give_up_at =>
                {
                    std::thread::sleep(RETRY_BIND_DELAY);
                }
                result => return result,
            }
        };

        let socket = bind_socket(socket_addr).map_err(|e| e.to_string())?;
        socket
            .set_read_timeout(Some(IPFIX_READ_TIMEOUT))
            .map_err(|e| e.to_string())?;

        Ok(Self::Ipfix(socket))
    }
}

pub struct Live {
    cap: CaptureHelper,
}

pub struct LiveWithSavefile {
    live: Live,
    savefile: Savefile,
}

pub struct Offline {
    cap: Capture<pcap::Offline>,
}

pub enum CaptureType {
    Live(CaptureHelper),
    Offline(Capture<pcap::Offline>),
}

impl CaptureType {
    // TODO: consider calling PCAP's dispatch() instead of next_packet() (needs benchmarking first)
    pub fn next_packet(&mut self) -> Result<Packet<'_>, Error> {
        match self {
            Self::Live(on) => on.next_packet(),
            Self::Offline(off) => off.next_packet(),
        }
    }

    pub fn stats(&mut self) -> Result<Stat, Error> {
        match self {
            Self::Live(on) => Ok(on.stats()),
            Self::Offline(off) => off.stats(),
        }
    }

    /// `full`: capture whole packets (for the PCAP export or the packet store), not only the
    /// first bytes the traffic statistics need
    fn from_device(device: &MyDevice, full: bool, filters: &Filters) -> Result<Self, String> {
        let (snaplen, buffer_size) = live_limits(full);
        let bpf = filters.is_some_filter_active().then(|| filters.bpf());
        let helper = CaptureHelper::start(device.get_name(), snaplen, buffer_size, bpf)?;
        Ok(Self::Live(helper))
    }

    fn from_file(file: &MyPcapImport) -> Result<Self, String> {
        Ok(Self::Offline(
            Capture::from_file(&file.path).map_err(|e| e.to_string())?,
        ))
    }

    pub fn pause(&mut self) {
        if let Self::Live(cap) = self {
            cap.pause();
        }
    }

    pub fn resume(&mut self) {
        if let Self::Live(cap) = self {
            cap.resume();
        }
    }
}

/// Snapshot length and kernel buffer size of a live capture: whole packets with a larger buffer
/// when they are kept or exported, otherwise only the first 200 bytes the statistics need (more
/// of them fit in the buffer)
fn live_limits(full: bool) -> (i32, i32) {
    if full {
        // 16 MB: more than a second of a busy gigabit link, the packet store copies the bytes
        (i32::from(u16::MAX), 16_000_000)
    } else {
        // 2 MB -> 10k packets of 200 bytes
        (200, 2_000_000)
    }
}

#[derive(Clone)]
pub enum CaptureSource {
    Device(MyDevice),
    File(MyPcapImport),
    Ipfix(MyIpfixCollector),
}

impl CaptureSource {
    pub fn from_conf(conf: &Conf) -> Self {
        match conf.capture_source_picklist {
            CaptureSourcePicklist::Device => {
                let device = conf.device.to_my_device();
                Self::Device(device)
            }
            CaptureSourcePicklist::File => {
                let path = conf.import_pcap_path.clone();
                Self::File(MyPcapImport::new(path))
            }
            CaptureSourcePicklist::Ipfix => {
                let socket = conf.ipfix_socket.clone();
                Self::Ipfix(MyIpfixCollector::new(socket))
            }
        }
    }

    /// Addresses the collector listens on when bound to the unspecified address
    /// (only used for display purposes)
    pub fn get_ipfix_unspecified_bound_addresses(&self) -> &[Address] {
        match self {
            Self::Ipfix(collector) => &collector.unspecified_bound_addresses,
            Self::Device(_) | Self::File(_) => &[],
        }
    }

    pub fn get_addresses(&self) -> &[Address] {
        match self {
            Self::Device(device) => device.get_addresses(),
            Self::File(_) | Self::Ipfix(_) => &[],
        }
    }

    pub fn set_addresses(&mut self) {
        match self {
            Self::Device(my_device) => {
                let mut addresses = Vec::new();
                for dev in Device::list().log_err(location!()).unwrap_or_default() {
                    if matches!(
                        my_device.get_link_type(),
                        Some(LinkType::LinuxSll(_) | LinkType::LinuxSll2(_))
                    ) {
                        addresses.extend(dev.addresses);
                    } else if dev.name.eq(my_device.get_name()) {
                        addresses.extend(dev.addresses);
                        break;
                    }
                }
                my_device.set_addresses(addresses);
            }
            Self::Ipfix(collector) => collector.set_unspecified_bound_addresses(),
            Self::File(_) => {}
        }
    }

    pub fn get_link_type(&self) -> Option<LinkType> {
        match self {
            Self::Device(device) => device.get_link_type(),
            Self::File(file) => file.link_type,
            Self::Ipfix(_) => None,
        }
    }

    pub fn set_link_type(&mut self, link_type: Option<LinkType>) {
        match self {
            Self::Device(device) => device.set_link_type(link_type),
            Self::File(file) => file.link_type = link_type,
            Self::Ipfix(_) => {}
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            Self::Device(device) => device.get_name().clone(),
            Self::File(file) => file.path.clone(),
            Self::Ipfix(collector) => collector.socket.display_name(),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn get_desc(&self) -> Option<String> {
        match self {
            Self::Device(device) => device.get_desc().cloned(),
            Self::File(_) | Self::Ipfix(_) => None,
        }
    }

    pub fn supports_filters(&self) -> bool {
        match self {
            Self::Device(_) | Self::File(_) => true,
            Self::Ipfix(_) => false,
        }
    }

    pub fn supports_export_pcap(&self) -> bool {
        match self {
            Self::Device(_) => true,
            Self::Ipfix(_) | Self::File(_) => false,
        }
    }

    pub fn supports_latency(&self) -> bool {
        match self {
            Self::Device(_) => true,
            Self::Ipfix(_) | Self::File(_) => false,
        }
    }

    pub fn supports_live_chart(&self) -> bool {
        match self {
            Self::Device(_) | Self::Ipfix(_) => true,
            Self::File(_) => false,
        }
    }

    pub fn supports_programs(&self) -> bool {
        match self {
            Self::Device(_) => true,
            Self::File(_) | Self::Ipfix(_) => false,
        }
    }

    pub fn supports_notification_sound(&self) -> bool {
        match self {
            Self::Device(_) | Self::Ipfix(_) => true,
            Self::File(_) => false,
        }
    }
}

#[derive(Clone)]
pub struct MyIpfixCollector {
    socket: MyIpfixSocket,
    /// Addresses the collector listens on when bound to the unspecified address
    /// (only used for display purposes)
    unspecified_bound_addresses: Vec<Address>,
}

impl MyIpfixCollector {
    pub fn new(socket: MyIpfixSocket) -> Self {
        Self {
            socket,
            unspecified_bound_addresses: Vec::new(),
        }
    }

    fn set_unspecified_bound_addresses(&mut self) {
        let Some(bound_to) = self.socket.unspecified_addr() else {
            self.unspecified_bound_addresses.clear();
            return;
        };

        let mut seen = HashSet::new();
        self.unspecified_bound_addresses = Device::list()
            .log_err(location!())
            .unwrap_or_default()
            .into_iter()
            .flat_map(|dev| dev.addresses)
            // a socket bound to an unspecified IP only receives datagrams of the same family
            .filter(|addr| addr.addr.is_ipv4() == bound_to.is_ipv4())
            // exporters can't reach loopback, multicast, broadcast, and link-local addresses
            .filter(|addr| match addr.addr {
                IpAddr::V4(v4) => {
                    !v4.is_loopback()
                        && !v4.is_link_local()
                        && !v4.is_unspecified()
                        && !v4.is_broadcast()
                        && !v4.is_multicast()
                }
                IpAddr::V6(v6) => {
                    !v6.is_loopback()
                        && !v6.is_unicast_link_local()
                        && !v6.is_unspecified()
                        && !v6.is_multicast()
                }
            })
            // remove duplicates
            .filter(|addr| seen.insert(addr.addr))
            .collect();
    }

    /// Update the collector's bind address and port to the actual ones used by the socket
    /// (particularly relevant for port = 0, which lets the OS pick a free port)
    pub(crate) fn set_actually_bind_addr(&mut self, local_addr: SocketAddr) {
        self.socket.set_addr(local_addr.ip().to_string());
        self.socket.set_port(local_addr.port().to_string());
    }
}

#[derive(Clone)]
pub struct MyPcapImport {
    path: String,
    link_type: Option<LinkType>,
}

impl MyPcapImport {
    pub fn new(path: String) -> Self {
        Self {
            path,
            link_type: None,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Copy, Default, Serialize, Deserialize)]
pub enum CaptureSourcePicklist {
    #[default]
    Device,
    File,
    Ipfix,
}

#[cfg(test)]
mod tests {
    use super::live_limits;

    #[test]
    fn whole_packets_only_when_kept_or_exported() {
        assert_eq!(live_limits(true), (65535, 16_000_000));
        assert_eq!(live_limits(false), (200, 2_000_000));
    }
}
