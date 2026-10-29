//! The capture page: choose what to analyze (network adapter, capture file, IPFIX collector),
//! start, pause and stop the capture.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use sniffnet_packet_parser::LinkType;

use crate::config::packet_store::PacketStoreConf;
use crate::engine::state::EngineState;
use crate::networking::types::capture_context::{
    CaptureError, CaptureSource, CaptureSourcePicklist,
};
use crate::networking::types::data_representation::DataRepr;
use crate::web::auth::Session;
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, flash_message, render};
use crate::web::settings::packets_max_choices;
use crate::web::{AppState, WebError, format};

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

#[derive(Template)]
#[template(path = "capture.html")]
struct CapturePage {
    t: Lang,
    layout: Layout,
    flash: Option<String>,
    error: Option<String>,
    running: Option<RunningCapture>,
    source: &'static str,
    adapters: Vec<AdapterRow>,
    file: String,
    ipfix_addr: String,
    ipfix_port: String,
    bpf: String,
    export_enabled: bool,
    export_dir: String,
    export_name: String,
    /// `full`, `headers` or `off`
    packets: &'static str,
    packets_max: Vec<(String, usize, bool)>,
}

#[derive(Deserialize)]
pub struct PageQuery {
    msg: Option<String>,
}

/// `GET /`: the capture page
pub async fn home(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> Response {
    let t = i18n::current();
    let engine = state.engine.lock();
    let page = capture_page(
        t,
        &engine,
        &session,
        flash_message(t, query.msg.as_deref()),
        None,
    );
    render(&page)
}

fn capture_page(
    t: Lang,
    state: &EngineState,
    session: &Session,
    flash: Option<String>,
    error: Option<String>,
) -> CapturePage {
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
    CapturePage {
        t,
        layout: Layout::new(state, session),
        flash,
        error,
        running,
        source: match conf.capture_source_picklist {
            CaptureSourcePicklist::Device => "device",
            CaptureSourcePicklist::File => "file",
            CaptureSourcePicklist::Ipfix => "ipfix",
        },
        adapters,
        file: conf.import_pcap_path.clone(),
        ipfix_addr: conf.ipfix_socket.addr().to_owned(),
        ipfix_port: conf.ipfix_socket.port().to_owned(),
        bpf: conf.filters.bpf().to_owned(),
        export_enabled: conf.export_pcap.enabled(),
        export_dir: conf.export_pcap.directory().to_owned(),
        export_name: conf.export_pcap.file_name().to_owned(),
        packets: match conf.packet_store {
            c if !c.enabled => "off",
            c if c.headers_only => "headers",
            _ => "full",
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

#[derive(Deserialize)]
pub struct StartForm {
    #[serde(default)]
    csrf_token: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    device: String,
    #[serde(default)]
    file: String,
    #[serde(default)]
    ipfix_addr: String,
    #[serde(default)]
    ipfix_port: String,
    #[serde(default)]
    bpf: String,
    #[serde(default)]
    export: Option<String>,
    #[serde(default)]
    export_dir: String,
    #[serde(default)]
    export_name: String,
    /// What the packet store keeps: `full`, `headers` or `off`
    #[serde(default)]
    packets: String,
    #[serde(default)]
    packets_max: usize,
}

/// `POST /capture/start`: applies the chosen source and options and starts the capture
pub async fn start(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<StartForm>,
) -> Result<Response, WebError> {
    session.check_csrf(&form.csrf_token)?;
    let t = i18n::current();
    let problem = {
        let mut engine = state.engine.lock();
        if engine.is_running() {
            return Ok(Redirect::to("/overview").into_response());
        }
        apply_start_form(&mut engine, &form)
    };
    let started = problem.is_none() && state.engine.start();
    if started {
        state.engine.store_conf();
        return Ok(Redirect::to("/overview").into_response());
    }
    let engine = state.engine.lock();
    let error = problem.map(|id| t.get(id)).or_else(|| {
        Some(match &engine.capture_error {
            Some(CaptureError::Fatal(e)) => t.get1("capture-error", "error", e.as_str()),
            _ => t.get("capture-error-unknown"),
        })
    });
    Ok(render(&capture_page(t, &engine, &session, None, error)))
}

/// Puts the form into the settings; returns the message id of a problem found before starting
fn apply_start_form(engine: &mut EngineState, form: &StartForm) -> Option<&'static str> {
    let bpf = form.bpf.trim().to_owned();
    engine.conf.filters.expanded = !bpf.is_empty();
    engine.conf.filters.set_bpf(bpf);
    if form.source != "ipfix" {
        apply_packet_store(&mut engine.conf.packet_store, form);
    }
    match form.source.as_str() {
        "file" => {
            let path = form.file.trim();
            engine.set_capture_source(CaptureSourcePicklist::File);
            if path.is_empty() {
                return Some("capture-file-missing");
            }
            engine.set_pcap_import(path.to_owned());
        }
        "ipfix" => {
            engine.set_capture_source(CaptureSourcePicklist::Ipfix);
            engine.set_ipfix_addr(form.ipfix_addr.trim().to_owned());
            engine.set_ipfix_port(form.ipfix_port.trim().to_owned());
        }
        _ => {
            engine.set_capture_source(CaptureSourcePicklist::Device);
            if !engine.select_device(&form.device) {
                return Some("capture-adapter-missing");
            }
            let export = &mut engine.conf.export_pcap;
            export.set_enabled(form.export.is_some());
            export.set_directory(form.export_dir.trim().to_owned());
            export.set_file_name(form.export_name.trim().to_owned());
        }
    }
    None
}

/// Puts the packet store choice of the form into `conf` (left as it is without a choice)
fn apply_packet_store(conf: &mut PacketStoreConf, form: &StartForm) {
    match form.packets.as_str() {
        "full" | "headers" => {
            conf.enabled = true;
            conf.headers_only = form.packets == "headers";
        }
        "off" => conf.enabled = false,
        _ => {}
    }
    if PacketStoreConf::MAX_PACKETS_CHOICES.contains(&form.packets_max) {
        conf.max_packets = form.packets_max;
    }
}

#[derive(Deserialize)]
pub struct ActionForm {
    #[serde(default)]
    csrf_token: String,
    /// The page to return to (a path of this site)
    #[serde(default)]
    back: String,
}

/// A path of this site to return to, `fallback` for anything else (browsers drop tabs and line
/// breaks from addresses, so `/<tab>/host` would lead to another site)
pub fn back_path(back: &str, fallback: &'static str) -> String {
    if back.starts_with('/')
        && !back.starts_with("//")
        && !back.contains('\\')
        && !back.chars().any(char::is_control)
    {
        back.to_owned()
    } else {
        fallback.to_owned()
    }
}

/// `POST /capture/stop`: stops the capture and discards its data
pub async fn stop(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<ActionForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    if state.engine.lock().is_running() {
        state.engine.stop();
        state.engine.store_conf();
        return Ok(Redirect::to("/?msg=capture_stopped"));
    }
    Ok(Redirect::to("/"))
}

/// `POST /capture/pause`: pauses or resumes the capture
pub async fn pause(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<ActionForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    let mut engine = state.engine.lock();
    if engine.is_running() {
        engine.toggle_freeze();
    }
    Ok(Redirect::to(&back_path(&form.back, "/overview")))
}

#[cfg(test)]
mod tests {
    use axum::http::{StatusCode, header};

    use super::*;
    use crate::web::testutil::*;

    #[test]
    fn only_paths_of_this_site_are_returned_to() {
        assert_eq!(back_path("/connections?page=2", "/"), "/connections?page=2");
        assert_eq!(back_path("//evil.example", "/"), "/");
        assert_eq!(back_path("https://evil.example", "/x"), "/x");
        assert_eq!(back_path("/\\evil.example", "/"), "/");
        assert_eq!(back_path("/\t/evil.example", "/"), "/");
        assert_eq!(back_path("/\n/evil.example", "/"), "/");
    }

    #[tokio::test]
    async fn the_start_page_lists_the_sources() {
        let h = harness().await;
        let reply = send(&h.app, get("/", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.contains("name=\"source\" value=\"device\""));
        assert!(reply.body.contains("name=\"file\""));
        assert!(
            reply
                .body
                .contains("name=\"packets\" value=\"full\" checked")
        );
        assert!(reply.body.contains(&h.csrf));
    }

    #[tokio::test]
    async fn the_start_form_chooses_what_the_packet_store_keeps() {
        let h = harness().await;
        let start = |packets: &str| {
            format!(
                "csrf_token={}&source=file&file=%2Fnonexistent%2Fx.pcap&packets={packets}&packets_max=10000",
                h.csrf
            )
        };
        send(&h.app, post("/capture/start", &h.cookie, &start("headers"))).await;
        let conf = h.state.engine.lock().conf.packet_store;
        assert!(conf.enabled && conf.headers_only && !conf.full_packets());
        assert_eq!(conf.max_packets, 10_000);
        let reply = send(&h.app, get("/", Some(&h.cookie))).await;
        assert!(
            reply
                .body
                .contains("name=\"packets\" value=\"headers\" checked")
        );

        send(&h.app, post("/capture/start", &h.cookie, &start("off"))).await;
        assert!(!h.state.engine.lock().conf.packet_store.enabled);
        send(&h.app, post("/capture/start", &h.cookie, &start("full"))).await;
        assert!(h.state.engine.lock().conf.packet_store.full_packets());
    }

    #[tokio::test]
    async fn a_missing_file_is_reported() {
        let h = harness().await;
        let form = format!(
            "csrf_token={}&source=file&file=%2Fnonexistent%2Fx.pcap",
            h.csrf
        );
        let reply = send(&h.app, post("/capture/start", &h.cookie, &form)).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.body.contains("class=\"error\""), "{}", reply.body);
        assert!(!h.state.engine.lock().is_running());

        let form = format!("csrf_token={}&source=device&device=no-such-adapter", h.csrf);
        let reply = send(&h.app, post("/capture/start", &h.cookie, &form)).await;
        assert!(
            reply
                .body
                .contains(&Lang::En.get("capture-adapter-missing"))
        );
        assert!(reply.headers.get(header::LOCATION).is_none());
    }
}
