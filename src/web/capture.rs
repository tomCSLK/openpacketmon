//! The capture page: choose what to analyze, start, pause and stop the capture.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::engine::state::EngineState;
use crate::networking::types::capture_context::CaptureSourcePicklist;
use crate::view::capture::{
    AdapterRow, PacketChoice, RunningCapture, StartRequest, apply_start, capture_setup, start_error,
};
use crate::web::auth::Session;
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, flash_message, render};
use crate::web::{AppState, WebError};

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
    let setup = capture_setup(t, state);
    CapturePage {
        t,
        layout: Layout::new(state, session),
        flash,
        error,
        running: setup.running,
        source: match setup.source {
            CaptureSourcePicklist::Device => "device",
            CaptureSourcePicklist::File => "file",
            CaptureSourcePicklist::Ipfix => "ipfix",
        },
        adapters: setup.adapters,
        file: setup.file,
        ipfix_addr: setup.ipfix_addr,
        ipfix_port: setup.ipfix_port,
        bpf: setup.bpf,
        export_enabled: setup.export_enabled,
        export_dir: setup.export_dir,
        export_name: setup.export_name,
        packets: match setup.packets {
            PacketChoice::Off => "off",
            PacketChoice::Headers => "headers",
            PacketChoice::Full => "full",
        },
        packets_max: setup.packets_max,
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

impl StartForm {
    fn request(self) -> StartRequest {
        StartRequest {
            source: match self.source.as_str() {
                "file" => CaptureSourcePicklist::File,
                "ipfix" => CaptureSourcePicklist::Ipfix,
                _ => CaptureSourcePicklist::Device,
            },
            packets: match self.packets.as_str() {
                "full" => Some(PacketChoice::Full),
                "headers" => Some(PacketChoice::Headers),
                "off" => Some(PacketChoice::Off),
                _ => None,
            },
            device: self.device,
            file: self.file,
            ipfix_addr: self.ipfix_addr,
            ipfix_port: self.ipfix_port,
            bpf: self.bpf,
            export: self.export.is_some(),
            export_dir: self.export_dir,
            export_name: self.export_name,
            packets_max: self.packets_max,
        }
    }
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
        apply_start(&mut engine, &form.request())
    };
    let started = problem.is_none() && state.engine.start();
    if started {
        state.engine.store_conf();
        return Ok(Redirect::to("/overview").into_response());
    }
    let engine = state.engine.lock();
    let error = start_error(t, &engine, problem);
    Ok(render(&capture_page(
        t,
        &engine,
        &session,
        None,
        Some(error),
    )))
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
