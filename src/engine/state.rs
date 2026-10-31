// Modified for openpacketmon (2026), see NOTICE. Based on `src/gui/sniffer.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

//! State of the traffic monitor, independent of any user interface.
//!
//! This takes over the capture-related parts of Sniffnet's `Sniffer` (`src/gui/sniffer.rs`):
//! starting and stopping captures, merging backend messages into the traffic data,
//! host and program resolution, notifications and favorites.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::thread;

use async_channel::Receiver;

use crate::config::conf::Conf;
use crate::config::favorite::{Favorite, FavoriteItem, FavoriteKey};
use crate::countries::types::country::Country;
use crate::dissect::Value;
use crate::engine::packets::SharedPacketStore;
use crate::engine::previews::{DevicePreview, apply_traffic_preview};
use crate::engine::traffic_history::TrafficHistory;
use crate::filter::{self, Filter};
use crate::location;
use crate::mmdb::geo::{self, GeoKind};
use crate::mmdb::types::mmdb_reader::{MmdbReaders, SharedReader};
use crate::networking::capture::{BackendTrafficMessage, spawn_capture_thread};
use crate::networking::manage_packets::get_address_to_lookup;
use crate::networking::traffic_preview::TrafficPreview;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::capture_context::{
    CaptureContext, CaptureError, CaptureSource, CaptureSourcePicklist, MyIpfixCollector,
    MyPcapImport,
};
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::host::{Host, HostMessage};
use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
use crate::networking::types::info_traffic::InfoTraffic;
use crate::networking::types::latency::LatencyStatus;
use crate::networking::types::my_device::MyDevice;
use crate::networking::types::program::Program;
use crate::networking::types::program_lookup::{ProgramLookup, lookup_program};
use crate::networking::types::service::Service;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::notifications::notify_and_log::notify_and_log;
use crate::notifications::types::logged_notification::LoggedNotifications;
use crate::notifications::types::sound::Sound;
use crate::report::types::sort_type::SortType;
use crate::utils::check_updates::UpdateStatus;
use crate::utils::error_logger::{ErrorLogger, Location};
use sniffnet_packet_parser::Protocol;

pub struct EngineState {
    /// Application settings, stored on disk
    pub conf: Conf,
    /// Source of the next or current capture
    pub capture_source: CaptureSource,
    /// Id of the current capture; messages of previous captures are ignored
    capture_id: usize,
    /// Channel of the current capture, `None` while no capture is running
    capture_rx: Option<Receiver<BackendTrafficMessage>>,
    /// Error of the current capture, if any
    pub capture_error: Option<CaptureError>,
    /// Traffic of the current capture
    pub info_traffic: InfoTraffic,
    /// Resolved addresses with their reverse DNS name and host
    pub addresses_resolved: HashMap<IpAddr, (String, Host)>,
    /// Per-second traffic of the current capture
    pub traffic_history: TrafficHistory,
    /// Notifications emitted during the current capture
    pub logged_notifications: LoggedNotifications,
    /// Notifications not yet seen by the user
    pub unread_notifications: usize,
    /// Latest notification sound to signal, with a counter that increases on every new sound
    pub sound_event: (u64, Sound),
    /// Packet counts of all adapters while no capture is running
    pub previews: Vec<DevicePreview>,
    /// Channel of the adapter previews, `None` while a capture is running
    preview_rx: Option<Receiver<TrafficPreview>>,
    /// Geolocation and ASN databases
    pub mmdb_readers: MmdbReaders,
    /// Results of latency measurements
    pub latency_statuses: HashMap<IpAddr, LatencyStatus>,
    /// Whether the current capture is paused
    pub frozen: bool,
    freeze_tx: Option<tokio::sync::broadcast::Sender<()>>,
    /// Program lookup, if supported by the capture source
    pub program_lookup: Option<ProgramLookup>,
    /// Packets of the current capture, if they are kept (not for IPFIX)
    pub packets: Option<SharedPacketStore>,
    /// Result of the check for a newer release
    pub update_status: UpdateStatus,
}

impl EngineState {
    pub fn new(conf: Conf) -> Self {
        let capture_source = CaptureSource::from_conf(&conf);
        let previews = pcap::Device::list()
            .unwrap_or_default()
            .into_iter()
            .map(|dev| DevicePreview {
                device: MyDevice::from_pcap_device(dev),
                packets: std::collections::VecDeque::new(),
            })
            .collect();
        let mmdb_readers = MmdbReaders {
            country: SharedReader::new(geo::open(GeoKind::Country, &conf.settings.mmdb_country)),
            asn: SharedReader::new(geo::open(GeoKind::Asn, &conf.settings.mmdb_asn)),
        };
        Self {
            conf,
            capture_source,
            capture_id: 0,
            capture_rx: None,
            capture_error: None,
            info_traffic: InfoTraffic::default(),
            addresses_resolved: HashMap::new(),
            traffic_history: TrafficHistory::new(true),
            logged_notifications: LoggedNotifications::default(),
            unread_notifications: 0,
            sound_event: (0, Sound::None),
            previews,
            preview_rx: None,
            mmdb_readers,
            latency_statuses: HashMap::new(),
            frozen: false,
            freeze_tx: None,
            program_lookup: None,
            packets: None,
            update_status: UpdateStatus::default(),
        }
    }

    // capture source --------------------------------------------------------------------------

    pub fn set_capture_source(&mut self, cs_pick: CaptureSourcePicklist) {
        self.conf.capture_source_picklist = cs_pick;
        match cs_pick {
            CaptureSourcePicklist::File => {
                self.set_pcap_import(self.conf.import_pcap_path.clone());
            }
            CaptureSourcePicklist::Ipfix => {
                self.capture_source =
                    CaptureSource::Ipfix(MyIpfixCollector::new(self.conf.ipfix_socket.clone()));
            }
            CaptureSourcePicklist::Device => {
                self.select_device(&self.conf.device.device_name.clone());
            }
        }
    }

    /// Selects a network adapter by name; returns `false` if no such adapter is known
    pub fn select_device(&mut self, name: &str) -> bool {
        let Some(preview) = self.previews.iter().find(|p| p.device.get_name().eq(name)) else {
            return false;
        };
        self.conf.device.device_name = name.to_string();
        self.capture_source = CaptureSource::Device(preview.device.clone());
        true
    }

    pub fn set_pcap_import(&mut self, path: String) {
        if !path.is_empty() {
            self.conf.import_pcap_path.clone_from(&path);
            self.capture_source = CaptureSource::File(MyPcapImport::new(path));
        }
    }

    pub fn set_ipfix_addr(&mut self, addr: String) {
        self.conf.ipfix_socket.set_addr(addr);
        self.capture_source =
            CaptureSource::Ipfix(MyIpfixCollector::new(self.conf.ipfix_socket.clone()));
    }

    pub fn set_ipfix_port(&mut self, port: String) {
        self.conf.ipfix_socket.set_port(port);
        self.capture_source =
            CaptureSource::Ipfix(MyIpfixCollector::new(self.conf.ipfix_socket.clone()));
    }

    pub fn is_capture_source_consistent(&self) -> bool {
        matches!(
            (self.conf.capture_source_picklist, &self.capture_source),
            (CaptureSourcePicklist::Device, CaptureSource::Device(_))
                | (CaptureSourcePicklist::File, CaptureSource::File(_))
                | (CaptureSourcePicklist::Ipfix, CaptureSource::Ipfix(_))
        )
    }

    pub fn is_running(&self) -> bool {
        self.capture_rx.is_some()
    }

    /// Id of the current capture; changes with every start and stop
    pub fn capture_id(&self) -> usize {
        self.capture_id
    }

    // settings ------------------------------------------------------------------------------

    pub fn set_custom_country_db(&mut self, db: String) {
        self.conf.settings.mmdb_country = db;
        self.reload_geo();
    }

    pub fn set_custom_asn_db(&mut self, db: String) {
        self.conf.settings.mmdb_asn = db;
        self.reload_geo();
    }

    /// Opens the geolocation databases again (after a download or a change of the settings);
    /// running captures use them for the addresses found from then on
    pub fn reload_geo(&mut self) {
        let settings = &self.conf.settings;
        let readers = &self.mmdb_readers;
        readers
            .country
            .set(geo::open(GeoKind::Country, &settings.mmdb_country));
        readers.asn.set(geo::open(GeoKind::Asn, &settings.mmdb_asn));
    }

    pub fn add_or_remove_favorite(&mut self, fav: &FavoriteKey, add: bool) {
        if add {
            self.conf.favorites.insert(fav);
        } else {
            self.conf.favorites.remove(fav);
        }
    }

    pub fn mark_notifications_read(&mut self) {
        self.unread_notifications = 0;
    }

    pub fn clear_notifications(&mut self) {
        self.logged_notifications.clear_notifications();
        self.unread_notifications = 0;
    }

    // capture lifecycle ---------------------------------------------------------------------

    /// Starts a capture from the current capture source.
    ///
    /// Returns the channel the engine has to drain into [`EngineState::handle_backend_message`],
    /// or `None` if the capture could not be started (see `capture_error`).
    pub fn start(&mut self) -> Option<Receiver<BackendTrafficMessage>> {
        if self.is_running() || !self.is_capture_source_consistent() {
            return None;
        }
        self.stop_previews();

        if matches!(&self.capture_source, CaptureSource::Device(_)) {
            let current_device_name = self.capture_source.get_name();
            self.select_device(&current_device_name);
        }
        self.conf.export_pcap.sanitize_file_name();
        let pcap_path = self.conf.export_pcap.full_path();
        let capture_context = CaptureContext::new(
            &self.capture_source,
            pcap_path.as_ref(),
            &self.conf.filters,
            self.conf.packet_store.full_packets(),
        );
        self.capture_error = capture_context
            .error()
            .map(|e| CaptureError::Fatal(e.to_string()));

        if let CaptureContext::Ipfix(udp_socket) = &capture_context
            && let CaptureSource::Ipfix(collector) = &mut self.capture_source
            && let Ok(local) = udp_socket.local_addr()
        {
            collector.set_actually_bind_addr(local);
        }

        if capture_context.error().is_some() {
            return None;
        }

        self.capture_source
            .set_link_type(capture_context.link_type());
        self.capture_source.set_addresses();
        self.traffic_history = TrafficHistory::new(self.capture_source.supports_live_chart());

        self.packets = match (capture_context.link_type(), &capture_context) {
            (
                Some(link_type),
                CaptureContext::Live(_)
                | CaptureContext::LiveWithSavefile(_)
                | CaptureContext::Offline(_),
            ) if self.conf.packet_store.enabled => {
                Some(SharedPacketStore::new(self.conf.packet_store, link_type))
            }
            _ => None,
        };

        let (tx, rx) = async_channel::unbounded();
        let (freeze_tx, freeze_rx) = tokio::sync::broadcast::channel(1_048_575);
        let freeze_rx2 = freeze_tx.subscribe();
        spawn_capture_thread(
            self.capture_id,
            self.capture_source.clone(),
            capture_context,
            self.mmdb_readers.clone(),
            self.conf.filters.clone(),
            tx,
            (freeze_rx, freeze_rx2),
            self.packets.clone(),
        );
        self.capture_rx = Some(rx.clone());
        self.freeze_tx = Some(freeze_tx);

        if self.capture_source.supports_programs() {
            let (port_tx, port_rx) = std::sync::mpsc::channel();
            let (program_tx, program_rx) = std::sync::mpsc::channel();
            let _ = thread::Builder::new()
                .name("thread_lookup_program".to_string())
                .spawn(move || {
                    lookup_program(&port_rx, &program_tx);
                })
                .log_err(location!());
            self.program_lookup = Some(ProgramLookup::new(port_tx, program_rx));
        }

        Some(rx)
    }

    /// Stops the current capture and clears its data
    pub fn stop(&mut self) {
        // closing the channel stops the capture threads
        if let Some(rx) = &self.capture_rx {
            rx.close();
        }
        // increment capture id to ignore pending messages from previous captures
        self.capture_id += 1;
        self.capture_rx = None;
        self.info_traffic = InfoTraffic::default();
        self.addresses_resolved = HashMap::new();
        self.latency_statuses = HashMap::new();
        self.logged_notifications = LoggedNotifications::default();
        self.unread_notifications = 0;
        self.capture_error = None;
        self.traffic_history = TrafficHistory::new(true);
        self.frozen = false;
        self.freeze_tx = None;
        self.program_lookup = None;
        self.packets = None;
        // reset IPFIX collector's socket to match configuration
        // (particularly relevant for port = 0 that was rewritten with the actual UDP bind)
        if matches!(self.capture_source, CaptureSource::Ipfix(_))
            && self.conf.capture_source_picklist == CaptureSourcePicklist::Ipfix
        {
            self.capture_source =
                CaptureSource::Ipfix(MyIpfixCollector::new(self.conf.ipfix_socket.clone()));
        }
    }

    /// Pauses or resumes the current capture
    pub fn toggle_freeze(&mut self) {
        self.frozen = !self.frozen;
        if let Some(tx) = &self.freeze_tx {
            let _ = tx.send(());
        }
    }

    // adapter previews ----------------------------------------------------------------------

    /// Takes the channel of new adapter previews; previews still running are stopped
    pub(super) fn set_preview_rx(&mut self, rx: Receiver<TrafficPreview>) {
        if let Some(old) = self.preview_rx.replace(rx) {
            old.close();
        }
    }

    fn stop_previews(&mut self) {
        // closing the channel stops the preview thread
        if let Some(rx) = &self.preview_rx {
            rx.close();
        }
        self.preview_rx = None;
        self.previews.iter_mut().for_each(|p| p.packets.clear());
    }

    pub fn handle_traffic_preview(&mut self, msg: TrafficPreview) {
        if self.preview_rx.is_some() {
            apply_traffic_preview(&mut self.previews, msg);
        }
    }

    // backend messages ----------------------------------------------------------------------

    pub fn handle_backend_message(&mut self, msg: BackendTrafficMessage) {
        match msg {
            BackendTrafficMessage::TickRun(cap_id, msg, host_msgs, no_more_packets) => {
                self.tick_run(cap_id, msg, host_msgs, no_more_packets);
            }
            BackendTrafficMessage::PendingHosts(cap_id, host_msgs) => {
                if cap_id == self.capture_id {
                    for host_msg in host_msgs {
                        self.handle_new_host(host_msg);
                    }
                }
            }
            BackendTrafficMessage::OfflineGap(cap_id, gap) => {
                if cap_id == self.capture_id {
                    self.traffic_history.push_offline_gap(gap);
                    self.info_traffic.start_interval(&mut self.program_lookup);
                }
            }
            BackendTrafficMessage::IpfixUndecodable(cap_id) => {
                if cap_id == self.capture_id {
                    self.capture_error = Some(CaptureError::IpfixUndecodable);
                }
            }
        }
    }

    fn tick_run(
        &mut self,
        cap_id: usize,
        msg: InfoTraffic,
        host_msgs: Vec<HostMessage>,
        no_more_packets: bool,
    ) {
        if cap_id != self.capture_id {
            return;
        }
        for host_msg in host_msgs {
            self.handle_new_host(host_msg);
        }
        if let Some(program_lookup) = &mut self.program_lookup {
            for program_res in program_lookup.pending_results() {
                program_lookup.update(program_res, &mut self.info_traffic.map);
            }
        }
        self.capture_source.set_addresses();
        self.refresh_data(msg, no_more_packets);
    }

    fn refresh_data(&mut self, mut msg: InfoTraffic, no_more_packets: bool) {
        self.info_traffic
            .refresh(&mut msg, &mut self.program_lookup);
        if self
            .info_traffic
            .tot_data_info
            .tot_data(DataRepr::packets(false))
            == 0
        {
            return;
        }
        let (emitted_notifications, sound) = notify_and_log(
            &mut self.logged_notifications,
            &self.conf.settings.notifications,
            &msg,
            &self.conf.favorites,
            self.program_lookup.as_ref(),
            &self.capture_source,
        );
        self.unread_notifications += emitted_notifications;
        if sound != Sound::None {
            self.sound_event = (self.sound_event.0 + 1, sound);
        }
        self.traffic_history.push(&msg, no_more_packets);
    }

    fn handle_new_host(&mut self, host_msg: HostMessage) {
        let HostMessage {
            host,
            mut data_info_host,
            address_to_lookup,
            rdns,
        } = host_msg;

        // DNS results carry historical totals, not a new capture interval.
        data_info_host.data_info.start_interval();
        self.info_traffic
            .hosts
            .entry(host.clone())
            .and_modify(|d| {
                d.refresh(&data_info_host);
            })
            .or_insert(data_info_host);

        self.addresses_resolved
            .insert(address_to_lookup, (rdns, host));
    }

    // queries -------------------------------------------------------------------------------

    /// Connections that satisfy the search, sorted, for the given page (starting at 1),
    /// together with the number of matching connections and their total traffic
    /// The connections the filter selects (all without one), sorted, one page of them; with their
    /// number and traffic
    pub fn searched_entries(
        &self,
        filter: Option<&Filter>,
        sort_type: SortType,
        data_repr: DataRepr,
        page: usize,
        page_size: usize,
    ) -> (
        Vec<(&AddressPortPair, &InfoAddressPortPair)>,
        usize,
        DataInfo,
    ) {
        let mut agglomerate = DataInfo::default();
        let mut values = Vec::new();
        let mut all_results: Vec<(&AddressPortPair, &InfoAddressPortPair)> = self
            .info_traffic
            .map
            .iter()
            .filter(|(key, value)| {
                filter.is_none_or(|filter| self.selects(filter, key, value, &mut values))
            })
            .map(|(key, val)| {
                agglomerate.refresh(val.data_info());
                (key, val)
            })
            .collect();

        all_results.sort_by(|&(_, a), &(_, b)| a.compare(b, sort_type, data_repr));

        let tot = all_results.len();
        let start = page.saturating_sub(1).saturating_mul(page_size).min(tot);
        let end = start.saturating_add(page_size).min(tot);
        all_results.truncate(end);
        all_results.drain(..start);
        (all_results, tot, agglomerate)
    }

    /// The connections the filter selects
    pub fn selected_connections(&self, filter: &Filter) -> HashSet<AddressPortPair> {
        let mut values = Vec::new();
        self.info_traffic
            .map
            .iter()
            .filter(|(key, value)| self.selects(filter, key, value, &mut values))
            .map(|(key, _)| *key)
            .collect()
    }

    /// The values of the connections under these keys that contain `part` (any case), with how
    /// many connections have them, the most frequent first
    pub fn connection_field_values(
        &self,
        keys: &[&str],
        part: &str,
        limit: usize,
    ) -> Vec<(String, usize)> {
        let part = part.to_lowercase();
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut values = Vec::new();
        for (key, info) in &self.info_traffic.map {
            self.connection_values(key, info, &mut values);
            for (name, value) in &values {
                if !keys.contains(name) {
                    continue;
                }
                let text = match value {
                    Value::Num(n) => n.to_string(),
                    Value::Ip(ip) => ip.to_string(),
                    Value::Text(text) => text.clone(),
                    Value::Mac(mac) => mac
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(":"),
                };
                if text.to_lowercase().contains(&part) {
                    *counts.entry(text).or_default() += 1;
                }
            }
        }
        let mut found: Vec<(String, usize)> = counts.into_iter().collect();
        found.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        found.truncate(limit);
        found
    }

    fn selects(
        &self,
        filter: &Filter,
        key: &AddressPortPair,
        info: &InfoAddressPortPair,
        values: &mut Vec<(&'static str, Value)>,
    ) -> bool {
        let protocols = self.connection_values(key, info, values);
        filter.matches_connection(&filter::Connection {
            protocols: &protocols,
            values,
        })
    }

    /// The protocols and values of a connection for the connection filter (`values` is reused)
    fn connection_values(
        &self,
        key: &AddressPortPair,
        info: &InfoAddressPortPair,
        values: &mut Vec<(&'static str, Value)>,
    ) -> [&'static str; 2] {
        values.clear();
        let ip = if key.src_ip.is_ipv4() { "ip" } else { "ipv6" };
        let proto = match key.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Icmpv4 => "icmp",
            Protocol::Icmpv6 => "icmpv6",
            Protocol::Igmp => "igmp",
            Protocol::Arp => "arp",
        };
        values.push(("ip.src", Value::Ip(key.src_ip)));
        values.push(("ip.dst", Value::Ip(key.dst_ip)));
        let remote = get_address_to_lookup(key, info.traffic_direction);
        values.push(("remote", Value::Ip(remote)));
        values.push(("proto", Value::Text(key.protocol.to_string())));
        let ports = match key.protocol {
            Protocol::Tcp => Some(("tcp.srcport", "tcp.dstport")),
            Protocol::Udp => Some(("udp.srcport", "udp.dstport")),
            _ => None,
        };
        if let Some((src, dst)) = ports {
            if let Some(port) = key.src_port {
                values.push((src, Value::Num(u64::from(port))));
            }
            if let Some(port) = key.dst_port {
                values.push((dst, Value::Num(u64::from(port))));
            }
        }
        if let Some(mac) = info.src_mac {
            values.push(("eth.src", Value::Mac(mac)));
        }
        if let Some(mac) = info.dst_mac {
            values.push(("eth.dst", Value::Mac(mac)));
        }
        if let Some(vlan) = info.vlan_id {
            values.push(("vlan.id", Value::Num(u64::from(vlan))));
        }
        values.push((
            "direction",
            Value::Text(
                match info.traffic_direction {
                    TrafficDirection::Outgoing => "outgoing",
                    TrafficDirection::Incoming => "incoming",
                }
                .to_owned(),
            ),
        ));
        if info.service != Service::NotApplicable {
            values.push(("service", Value::Text(info.service.to_string())));
        }
        let favorites = &self.conf.favorites;
        let mut favorite =
            favorites.contains_address(&key.src_ip) || favorites.contains_address(&key.dst_ip);
        if let Some((rdns, host)) = self.addresses_resolved.get(&remote) {
            favorite |= favorites.contains_host(host);
            if !host.domain.is_empty() {
                values.push(("domain", Value::Text(host.domain.clone())));
            }
            if !rdns.is_empty() && *rdns != host.domain && *rdns != remote.to_string() {
                values.push(("domain", Value::Text(rdns.clone())));
            }
            if !host.asn.name.is_empty() {
                values.push(("as.name", Value::Text(host.asn.name.clone())));
            }
            if let Ok(number) = host.asn.code.parse::<u64>() {
                values.push(("as.number", Value::Num(number)));
            }
            if host.country != Country::ZZ {
                values.push(("country", Value::Text(host.country.to_string())));
            }
        }
        if let Program::NamePath((name, path)) = &info.program {
            values.push(("program", Value::Text(name.clone())));
            values.push(("program.path", Value::Text(path.clone())));
        } else if info.program == Program::Unknown {
            values.push(("program", Value::Text("?".to_owned())));
        }
        if self.program_lookup.is_some() && favorites.contains_program(&info.program) {
            favorite = true;
        }
        if let Some(exporter) = key.exporter {
            values.push(("exporter", Value::Ip(exporter.addr)));
        }
        values.push((
            "packets",
            Value::Num(u64::try_from(info.packets).unwrap_or(u64::MAX)),
        ));
        values.push((
            "bytes",
            Value::Num(u64::try_from(info.bytes).unwrap_or(u64::MAX)),
        ));
        if favorite {
            values.push(("favorite", Value::Num(1)));
        }
        [ip, proto]
    }

    /// Hosts, services or programs with their traffic, sorted
    pub fn favorite_entries(
        &self,
        kind: Favorite,
        data_repr: DataRepr,
        sort_type: SortType,
        favorites_filter: bool,
    ) -> Vec<FavoriteItem> {
        kind.get_entries(
            &self.info_traffic,
            self.program_lookup.as_ref(),
            &self.conf.favorites,
            data_repr,
            sort_type,
            favorites_filter,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::net::Ipv4Addr;

    use super::*;
    use crate::config::favorite::Favorites;
    use crate::networking::types::data_info_host::DataInfoHost;
    use crate::notifications::types::logged_notification::LoggedNotification;

    #[test]
    fn delayed_host_resolution_preserves_rates_without_counting_history() {
        let mut state = EngineState::new(Conf::default());
        let host = Host {
            domain: "example.test".into(),
            ..Default::default()
        };
        let historical = HostMessage {
            host: host.clone(),
            data_info_host: DataInfoHost {
                data_info: DataInfo::new_for_tests(10, 0, 1000, 0),
                ..Default::default()
            },
            address_to_lookup: IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
            rdns: host.domain.clone(),
        };
        state.handle_new_host(historical.clone());
        let data = state.info_traffic.hosts[&host].data_info;
        assert_eq!(data.tot_data(DataRepr::packets(false)), 10);
        assert_eq!(data.tot_data(DataRepr::packets(true)), 0);

        let mut interval = InfoTraffic::default();
        interval.hosts.insert(
            host.clone(),
            DataInfoHost {
                data_info: DataInfo::new_for_tests(3, 0, 300, 0),
                ..Default::default()
            },
        );
        state.info_traffic.refresh(&mut interval, &mut None);
        // Another resolved address can merge history into the same host mid-interval.
        state.handle_new_host(historical);
        let data = state.info_traffic.hosts[&host].data_info;
        assert_eq!(data.tot_data(DataRepr::packets(false)), 23);
        assert_eq!(data.tot_data(DataRepr::packets(true)), 3);
        assert_eq!(data.tot_data(DataRepr::bytes(true)), 300);
    }

    #[test]
    fn test_add_or_remove_favorite() {
        let mut state = EngineState::new(Conf::default());

        let fav_host = FavoriteKey::Host(Host {
            domain: "1.1".to_string(),
            asn: Default::default(),
            country: Country::US,
        });
        let fav_address = FavoriteKey::Address(IpAddr::from([192, 168, 1, 20]));
        let fav_program = FavoriteKey::Program(Program::NamePath((
            "Chrome".to_string(),
            "/usr/bin/chrome".to_string(),
        )));

        state.add_or_remove_favorite(&fav_host, false);
        assert_eq!(state.conf.favorites, Favorites::default());
        state.add_or_remove_favorite(&fav_address, true);
        assert_eq!(state.conf.favorites, Favorites::from([fav_address.clone()]));
        // adding twice changes nothing
        state.add_or_remove_favorite(&fav_address, true);
        assert_eq!(state.conf.favorites, Favorites::from([fav_address.clone()]));
        state.add_or_remove_favorite(&fav_host, true);
        state.add_or_remove_favorite(&fav_program, true);
        assert_eq!(
            state.conf.favorites,
            Favorites::from([fav_host.clone(), fav_address.clone(), fav_program.clone()])
        );
        state.add_or_remove_favorite(&fav_address, false);
        state.add_or_remove_favorite(&fav_program, false);
        assert_eq!(state.conf.favorites, Favorites::from([fav_host.clone()]));
        state.add_or_remove_favorite(&fav_host, false);
        assert_eq!(state.conf.favorites, Favorites::default());
    }

    #[test]
    fn test_clear_all_notifications() {
        let mut state = EngineState::new(Conf::default());
        state
            .logged_notifications
            .set_notifications(VecDeque::from([LoggedNotification {
                favorite: FavoriteItem::Address((IpAddr::from([10, 0, 0, 1]), DataInfo::default())),
                timestamp: String::new(),
            }]));
        state.unread_notifications = 1;

        assert_eq!(state.logged_notifications.notifications().len(), 1);
        state.clear_notifications();
        assert_eq!(state.logged_notifications.notifications().len(), 0);
        assert_eq!(state.unread_notifications, 0);
    }

    #[test]
    fn messages_of_previous_captures_are_ignored() {
        let mut state = EngineState::new(Conf::default());
        let old_capture = state.capture_id;
        state.stop();
        let mut interval = InfoTraffic::default();
        interval.tot_data_info = DataInfo::new_for_tests(5, 0, 500, 0);
        state.handle_backend_message(BackendTrafficMessage::TickRun(
            old_capture,
            interval,
            Vec::new(),
            false,
        ));
        assert_eq!(
            state
                .info_traffic
                .tot_data_info
                .tot_data(DataRepr::packets(false)),
            0
        );
        assert_eq!(state.traffic_history.ticks(), 0);
    }

    #[test]
    fn select_device_only_accepts_known_adapters() {
        let mut state = EngineState::new(Conf::default());
        assert!(!state.select_device("no-such-adapter-0"));
        assert_ne!(state.conf.device.device_name, "no-such-adapter-0");
    }
}
