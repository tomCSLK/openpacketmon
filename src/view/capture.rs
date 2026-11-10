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

/// A network adapter in the selection
pub struct AdapterRow {
    /// Identifies the adapter, not shown under Windows (`\Device\NPF_{GUID}`)
    pub name: String,
    /// Shown in bold: the name, under Windows the description
    pub label: String,
    /// Shown below, may be empty
    pub desc: String,
    /// One per line
    pub addresses: Vec<String>,
    pub selected: bool,
    /// No IP traffic on this adapter: shown greyed out, cannot be chosen
    pub no_ip: bool,
}

/// How the running capture is doing (the state in the head of its card)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Running,
    Paused,
    /// A file read to its end
    Finished,
    Failed,
}

/// The running capture, shown instead of the selection (`pageCapture` of the draft)
pub struct RunningCapture {
    pub state: RunState,
    /// The text below the title: where and since when
    pub subtitle: String,
    /// "Dauer 12:08 min"
    pub duration: String,
    /// The adapter, file or address, in bold
    pub source: String,
    /// The description of the adapter, after the name
    pub source_desc: String,
    /// Link type of the adapter or file (Ethernet, Linux cooked capture …)
    pub link_type: Option<String>,
    /// Addresses an IPFIX collector bound to the unspecified address listens on
    pub listening: Vec<String>,
    /// The BPF filter, empty without one; `None` for sources without filters
    pub filter: Option<String>,
    /// What the packet store keeps (not shown for IPFIX, which has no packets)
    pub packet_store: Option<String>,
    /// The PCAP file written, empty without one; `None` for sources that write none
    pub export: Option<String>,
    /// Packets dropped by the adapter and by openpacketmon (buffer full)
    pub dropped: Option<(String, String)>,
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
    /// What each choice of the packet store keeps, with the most packets of the settings
    pub packets_lines: [String; 3],
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
                .collect(),
            selected: *p.device.get_name() == selected_device,
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
        packets_lines: {
            let max = format::count(t, conf.packet_store.max_packets() as u128);
            [
                t.get1("capture-packets-full-max", "max", max.as_str()),
                t.get1("capture-packets-headers-max", "max", max.as_str()),
                t.get("capture-packets-none"),
            ]
        },
    }
}

/// How the running capture is doing
fn run_state(state: &EngineState) -> RunState {
    if state.capture_error.is_some() {
        RunState::Failed
    } else if state.traffic_history.no_more_packets() {
        RunState::Finished
    } else if state.frozen {
        RunState::Paused
    } else {
        RunState::Running
    }
}

/// The name of a link type as the draft writes it (`Ethernet`): its description, else its name
fn link_name(link: LinkType) -> String {
    let text = link.description();
    text.split_once(" (")
        .and_then(|(_, desc)| desc.strip_suffix(')'))
        .map_or_else(|| text.clone(), str::to_owned)
}

fn running_capture(t: Lang, state: &EngineState) -> RunningCapture {
    let source = &state.capture_source;
    let head = crate::view::overview::capture_head(state);
    let (subtitle, duration) = head.map_or_else(Default::default, |h| {
        let id = match source {
            CaptureSource::Device(_) => "capture-sub-adapter",
            CaptureSource::File(_) => "capture-sub-file",
            CaptureSource::Ipfix(_) => "capture-sub-ipfix",
        };
        let subtitle = t.fmt(
            id,
            &[
                ("name", h.name.clone().into()),
                ("time", h.since.clone().into()),
            ],
        );
        let duration = t.get1("capture-duration", "duration", h.duration.as_str());
        (subtitle, duration)
    });
    let (source_name, source_desc) = match source {
        CaptureSource::Device(device) => (device.display_name(), device.display_desc()),
        _ => (source.display_name(), String::new()),
    };
    RunningCapture {
        state: run_state(state),
        subtitle,
        duration,
        source: source_name,
        source_desc,
        link_type: source.get_link_type().map(link_name),
        listening: source
            .get_ipfix_unspecified_bound_addresses()
            .iter()
            .map(|a| a.addr.to_string())
            .collect(),
        filter: source.supports_filters().then(|| {
            if state.conf.filters.is_some_filter_active() {
                state.conf.filters.bpf().to_owned()
            } else {
                String::new()
            }
        }),
        packet_store: (!matches!(source, CaptureSource::Ipfix(_))).then(|| {
            state.packets.as_ref().map_or_else(
                || t.get("capture-packets-none"),
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
        export: source
            .supports_export_pcap()
            .then(|| state.conf.export_pcap.full_path().unwrap_or_default()),
        dropped: state.info_traffic.dropped_packets.map(|dropped| {
            let packets = DataRepr::packets(false);
            let data = state.info_traffic.tot_data_info;
            (
                format::count(t, dropped.by_adapter(packets, data)),
                format::count(t, dropped.by_sniffnet(packets, data)),
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
