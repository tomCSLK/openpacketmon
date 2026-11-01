//! What an assistant can ask. It runs in the window, and it only reads.

use std::path::Path;

use serde_json::{Map, Value, json};

use crate::engine::Engine;
use crate::engine::state::EngineState;
use crate::mcp::protocol::Answer;
use crate::networking::types::capture_context::CaptureSource;
use crate::networking::types::data_representation::DataRepr;
use crate::view::capture::capture_problem;
use crate::view::i18n::Lang;

pub const CAPTURE_STATUS: &str = "capture_status";

/// Answers one question; the state stays locked only as long as the capture can wait
pub fn call(engine: &Engine, tool: &str) -> Answer {
    match tool {
        CAPTURE_STATUS => Answer::Ok(capture_status(&engine.lock())),
        _ => Answer::Error("unknown tool".to_owned()),
    }
}

fn number(value: u128) -> Value {
    json!(u64::try_from(value).unwrap_or(u64::MAX))
}

fn capture_state(state: &EngineState) -> &'static str {
    if !state.is_running() {
        "none"
    } else if state.capture_error.is_some() {
        "error"
    } else if state.traffic_history.no_more_packets() {
        "finished"
    } else if state.frozen {
        "paused"
    } else {
        "running"
    }
}

/// The file alone: its folder says nothing about the traffic, but it does name the user
fn source_name(source: &CaptureSource) -> String {
    let name = source.get_name();
    match source {
        CaptureSource::File(_) => Path::new(&name)
            .file_name()
            .map_or(name.clone(), |n| n.to_string_lossy().into_owned()),
        CaptureSource::Device(_) | CaptureSource::Ipfix(_) => name,
    }
}

fn capture_status(state: &EngineState) -> Value {
    let mut answer = Map::new();
    answer.insert("capture".to_owned(), capture_state(state).into());
    if let Some(problem) = capture_problem(Lang::En, state) {
        answer.insert("problem".to_owned(), problem.into());
    }
    if !state.is_running() {
        return answer.into();
    }
    let source = &state.capture_source;
    let kind = match source {
        CaptureSource::Device(_) => "adapter",
        CaptureSource::File(_) => "file",
        CaptureSource::Ipfix(_) => "ipfix",
    };
    let data = state.info_traffic.tot_data_info;
    let packets = DataRepr::packets(false);
    answer.insert(
        "source".to_owned(),
        json!({ "kind": kind, "name": source_name(source) }),
    );
    answer.insert("packets".to_owned(), number(data.tot_data(packets)));
    answer.insert(
        "bytes".to_owned(),
        number(data.tot_data(DataRepr::bytes(false))),
    );
    if let Some(dropped) = state.info_traffic.dropped_packets {
        answer.insert(
            "dropped".to_owned(),
            json!({
                "by_adapter": number(dropped.by_adapter(packets, data)),
                "by_openpacketmon": number(dropped.by_sniffnet(packets, data)),
            }),
        );
    }
    if let Some(store) = &state.packets {
        let store = store.lock();
        answer.insert(
            "stored_packets".to_owned(),
            json!({ "count": store.len(), "headers_only": store.headers_only() }),
        );
    }
    answer.into()
}
