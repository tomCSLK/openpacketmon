//! The capture as the interface shows it: what to analyze (network adapter, capture file, IPFIX
//! collector) and the running capture.

use sniffnet_packet_parser::LinkType;

use crate::config::packet_store::PacketStoreConf;
use crate::engine::state::EngineState;
use crate::networking::types::capture_context::{
    CaptureError, CaptureSource, CaptureSourcePicklist,
};
use crate::networking::types::data_representation::DataRepr;
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::settings::packets_max_choices;

/// A network adapter in the selection
pub struct AdapterRow {
    /// Identifies the adapter, not shown under Windows (`\Device\NPF_{GUID}`)
    pub name: String,
    /// Shown in bold: the name, under Windows the description
    pub label: String,
    /// Shown below, may be empty
    pub desc: String,
    pub addresses: String,
    pub selected: bool,
    /// Packets seen in the last 30 seconds
    pub packets: String,
    /// No IP traffic on this adapter: shown greyed out, cannot be chosen
    pub no_ip: bool,
}

/// The running capture, shown instead of the selection
pub struct RunningCapture {
    pub kind: String,
    pub name: String,
    pub state: String,
    pub frozen: bool,
    pub filter: String,
    pub export: Option<String>,
    /// Link type of the adapter or file (Ethernet, Linux cooked capture …)
    pub link_type: Option<String>,
    /// Addresses an IPFIX collector bound to the unspecified address listens on
    pub listening: Vec<String>,
    /// Packets dropped by the adapter and by openpacketmon (buffer full)
    pub dropped: Option<(String, String)>,
    /// What the packet store keeps (not shown for IPFIX, which has no packets)
    pub packet_store: Option<String>,
}

/// What the packet store keeps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketChoice {
    Full,
    Headers,
    Off,
}

/// What can be chosen before a capture, and the running capture
pub struct CaptureSetup {
    pub running: Option<RunningCapture>,
    pub source: CaptureSourcePicklist,
    pub adapters: Vec<AdapterRow>,
    pub file: String,
    pub ipfix_addr: String,
    pub ipfix_port: String,
    pub bpf: String,
    pub export_enabled: bool,
    pub export_dir: String,
    pub export_name: String,
    pub packets: PacketChoice,
    pub packets_max: Vec<(String, usize, bool)>,
}

pub fn capture_setup(t: Lang, state: &EngineState) -> CaptureSetup {
    let conf = &state.conf;
    let selected_device = match &state.capture_source {
        CaptureSource::Device(device) => device.get_name().clone(),
        _ => conf.device.device_name.clone(),
    };
    let adapters = state
        .previews
        .iter()
        .map(|p| AdapterRow {
            name: p.device.get_name().clone(),
            label: p.device.display_name(),
            desc: p.device.display_desc(),
            addresses: p
                .device
                .get_addresses()
                .iter()
                .map(|a| a.addr.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            selected: *p.device.get_name() == selected_device,
            packets: format::count(t, p.tot_packets()),
            no_ip: p.no_ip,
        })
        .collect();
    let running = state.is_running().then(|| running_capture(t, state));
    CaptureSetup {
        running,
        source: conf.capture_source_picklist,
        adapters,
        file: conf.import_pcap_path.clone(),
        ipfix_addr: conf.ipfix_socket.addr().to_owned(),
        ipfix_port: conf.ipfix_socket.port().to_owned(),
        bpf: conf.filters.bpf().to_owned(),
        export_enabled: conf.export_pcap.enabled(),
        export_dir: conf.export_pcap.directory().to_owned(),
        export_name: conf.export_pcap.file_name().to_owned(),
        packets: match conf.packet_store {
            c if !c.enabled => PacketChoice::Off,
            c if c.headers_only => PacketChoice::Headers,
            _ => PacketChoice::Full,
        },
        packets_max: packets_max_choices(t, conf.packet_store),
    }
}

/// Kind of the capture source, as shown to the user
pub fn source_kind(t: Lang, source: &CaptureSource) -> String {
    t.get(match source {
        CaptureSource::Device(_) => "source-device",
        CaptureSource::File(_) => "source-file",
        CaptureSource::Ipfix(_) => "source-ipfix",
    })
}

/// State of the running capture, as shown to the user
pub fn capture_state(t: Lang, state: &EngineState) -> String {
    t.get(if state.capture_error.is_some() {
        "capture-state-error"
    } else if state.traffic_history.no_more_packets() {
        "capture-state-finished"
    } else if state.frozen {
        "capture-state-paused"
    } else {
        "capture-state-running"
    })
}

fn running_capture(t: Lang, state: &EngineState) -> RunningCapture {
    let source = &state.capture_source;
    RunningCapture {
        kind: source_kind(t, source),
        name: source.display_name(),
        state: capture_state(t, state),
        frozen: state.frozen,
        filter: if source.supports_filters() && state.conf.filters.is_some_filter_active() {
            state.conf.filters.bpf().to_owned()
        } else {
            String::new()
        },
        export: if source.supports_export_pcap() {
            state.conf.export_pcap.full_path()
        } else {
            None
        },
        link_type: source.get_link_type().map(LinkType::description),
        listening: source
            .get_ipfix_unspecified_bound_addresses()
            .iter()
            .map(|a| a.addr.to_string())
            .collect(),
        dropped: state.info_traffic.dropped_packets.map(|dropped| {
            let packets = DataRepr::packets(false);
            let data = state.info_traffic.tot_data_info;
            (
                format::count(t, dropped.by_adapter(packets, data)),
                format::count(t, dropped.by_sniffnet(packets, data)),
            )
        }),
        packet_store: (!matches!(source, CaptureSource::Ipfix(_))).then(|| {
            state.packets.as_ref().map_or_else(
                || t.get("capture-packets-off"),
                |store| {
                    let store = store.lock();
                    t.get1(
                        if store.headers_only() {
                            "capture-packets-headers-max"
                        } else {
                            "capture-packets-full-max"
                        },
                        "max",
                        format::count(t, store.max_packets() as u128),
                    )
                },
            )
        }),
    }
}

/// What a capture is to analyze and how
pub struct StartRequest {
    pub source: CaptureSourcePicklist,
    pub device: String,
    pub file: String,
    pub ipfix_addr: String,
    pub ipfix_port: String,
    pub bpf: String,
    pub export: bool,
    pub export_dir: String,
    pub export_name: String,
    /// What the packet store keeps, left as it is without a choice
    pub packets: Option<PacketChoice>,
    /// Left as it is if it is not one of the choices
    pub packets_max: usize,
}

/// Puts the choices into the settings; returns the message id of a problem found before starting
pub fn apply_start(engine: &mut EngineState, request: &StartRequest) -> Option<&'static str> {
    let bpf = request.bpf.trim().to_owned();
    engine.conf.filters.expanded = !bpf.is_empty();
    engine.conf.filters.set_bpf(bpf);
    if request.source != CaptureSourcePicklist::Ipfix {
        apply_packet_store(&mut engine.conf.packet_store, request);
    }
    match request.source {
        CaptureSourcePicklist::File => {
            let path = request.file.trim();
            engine.set_capture_source(CaptureSourcePicklist::File);
            if path.is_empty() {
                return Some("capture-file-missing");
            }
            engine.set_pcap_import(path.to_owned());
        }
        CaptureSourcePicklist::Ipfix => {
            engine.set_capture_source(CaptureSourcePicklist::Ipfix);
            engine.set_ipfix_addr(request.ipfix_addr.trim().to_owned());
            engine.set_ipfix_port(request.ipfix_port.trim().to_owned());
        }
        CaptureSourcePicklist::Device => {
            engine.set_capture_source(CaptureSourcePicklist::Device);
            if !engine.select_device(&request.device) {
                return Some("capture-adapter-missing");
            }
            if engine
                .previews
                .iter()
                .any(|p| p.no_ip && *p.device.get_name() == request.device)
            {
                return Some("capture-adapter-no-ip");
            }
            let export = &mut engine.conf.export_pcap;
            export.set_enabled(request.export);
            export.set_directory(request.export_dir.trim().to_owned());
            export.set_file_name(request.export_name.trim().to_owned());
        }
    }
    None
}

fn apply_packet_store(conf: &mut PacketStoreConf, request: &StartRequest) {
    match request.packets {
        Some(PacketChoice::Full) => {
            conf.enabled = true;
            conf.headers_only = false;
        }
        Some(PacketChoice::Headers) => {
            conf.enabled = true;
            conf.headers_only = true;
        }
        Some(PacketChoice::Off) => conf.enabled = false,
        None => {}
    }
    if PacketStoreConf::MAX_PACKETS_CHOICES.contains(&request.packets_max) {
        conf.max_packets = request.packets_max;
    }
}

/// Why a capture did not start: the problem found before starting, or the error of the capture
pub fn start_error(t: Lang, engine: &EngineState, problem: Option<&str>) -> String {
    problem.map_or_else(
        || match &engine.capture_error {
            Some(CaptureError::Fatal(e)) => t.get1("capture-error", "error", e.as_str()),
            Some(CaptureError::NoIp(link)) => t.get1("capture-no-ip-source", "link", link.as_str()),
            _ => t.get("capture-error-unknown"),
        },
        |id| t.get(id),
    )
}

/// What went wrong with the running capture
pub fn capture_problem(t: Lang, engine: &EngineState) -> Option<String> {
    match &engine.capture_error {
        Some(CaptureError::Fatal(e)) => Some(e.clone()),
        Some(CaptureError::IpfixUndecodable) => Some(t.get("capture-ipfix-undecodable")),
        Some(CaptureError::Stopped(reason)) => {
            Some(t.get1("capture-stopped", "reason", reason.as_str()))
        }
        Some(CaptureError::NoIp(link)) => {
            Some(t.get1("capture-no-ip-source", "link", link.as_str()))
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    use crate::config::conf::Conf;
    use crate::engine::Engine;

    fn engine() -> (tokio::runtime::Runtime, Engine) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        (runtime, engine)
    }

    fn request(packets: Option<PacketChoice>) -> StartRequest {
        StartRequest {
            source: CaptureSourcePicklist::File,
            device: String::new(),
            file: "/nonexistent/x.pcap".to_owned(),
            ipfix_addr: String::new(),
            ipfix_port: String::new(),
            bpf: String::new(),
            export: false,
            export_dir: String::new(),
            export_name: String::new(),
            packets,
            packets_max: 10_000,
        }
    }

    #[test]
    #[serial]
    fn the_start_request_chooses_what_the_packet_store_keeps() {
        let (_runtime, engine) = engine();
        assert_eq!(
            apply_start(&mut engine.lock(), &request(Some(PacketChoice::Headers))),
            None
        );
        let conf = engine.lock().conf.packet_store;
        assert!(conf.enabled && conf.headers_only && !conf.full_packets());
        assert_eq!(conf.max_packets, 10_000);

        apply_start(&mut engine.lock(), &request(Some(PacketChoice::Off)));
        assert!(!engine.lock().conf.packet_store.enabled);
        apply_start(&mut engine.lock(), &request(Some(PacketChoice::Full)));
        assert!(engine.lock().conf.packet_store.full_packets());
        // without a choice it stays
        apply_start(&mut engine.lock(), &request(None));
        assert!(engine.lock().conf.packet_store.full_packets());
    }

    #[test]
    #[serial]
    fn what_is_missing_is_reported_before_starting() {
        let (_runtime, engine) = engine();
        let mut no_file = request(None);
        no_file.file = "  ".to_owned();
        assert_eq!(
            apply_start(&mut engine.lock(), &no_file),
            Some("capture-file-missing")
        );
        let mut no_adapter = request(None);
        no_adapter.source = CaptureSourcePicklist::Device;
        no_adapter.device = "no-such-adapter".to_owned();
        assert_eq!(
            apply_start(&mut engine.lock(), &no_adapter),
            Some("capture-adapter-missing")
        );
        assert!(!engine.lock().is_running());
    }

    #[test]
    fn an_adapter_without_ip_traffic_cannot_be_chosen() {
        let mut state = EngineState::new(Conf::default());
        state.previews.push(crate::engine::previews::DevicePreview {
            device: crate::networking::types::my_device::MyDevice::from_pcap_device(
                pcap::Device::from("no-ip-test0"),
            ),
            packets: std::collections::VecDeque::new(),
            no_ip: true,
        });
        let mut bluetooth = request(None);
        bluetooth.source = CaptureSourcePicklist::Device;
        bluetooth.device = "no-ip-test0".to_owned();
        assert_eq!(
            apply_start(&mut state, &bluetooth),
            Some("capture-adapter-no-ip")
        );
        let row = capture_setup(Lang::En, &state)
            .adapters
            .into_iter()
            .find(|a| a.name == "no-ip-test0")
            .unwrap();
        assert!(row.no_ip);
        assert_eq!(
            start_error(Lang::En, &state, Some("capture-adapter-no-ip")),
            Lang::En.get("capture-adapter-no-ip")
        );
    }

    #[test]
    #[serial]
    fn a_file_that_cannot_be_read_is_explained() {
        let (_runtime, engine) = engine();
        apply_start(&mut engine.lock(), &request(None));
        assert!(!engine.start());
        let state = engine.lock();
        assert!(!state.is_running());
        assert!(!start_error(Lang::En, &state, None).is_empty());
        assert_eq!(
            start_error(Lang::En, &state, Some("capture-adapter-missing")),
            Lang::En.get("capture-adapter-missing")
        );
    }
}
