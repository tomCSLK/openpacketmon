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
    pub name: String,
    pub desc: String,
    pub addresses: String,
    pub selected: bool,
    /// Packets seen in the last 30 seconds
    pub packets: String,
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
            desc: p.device.get_desc().cloned().unwrap_or_default(),
            addresses: p
                .device
                .get_addresses()
                .iter()
                .map(|a| a.addr.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            selected: *p.device.get_name() == selected_device,
            packets: format::count(t, p.tot_packets()),
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
        name: source.get_name(),
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
        None => None,
    }
}
