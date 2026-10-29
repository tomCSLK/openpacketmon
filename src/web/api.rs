//! JSON for the scripts: live values of the capture (`static/live.js`) and the activity of the
//! network adapters before a capture (`static/capture.js`).

use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::networking::types::capture_context::CaptureError;
use crate::networking::types::data_representation::{DataRepr, DataUnit};
use crate::notifications::types::sound::Sound;
use crate::web::AppState;
use crate::web::auth::Session;

#[derive(Deserialize)]
pub struct LiveQuery {
    /// Capture the client has samples of
    capture: Option<usize>,
    /// Generation of those samples (see `TrafficHistory::generation`)
    generation: Option<u32>,
    /// The client has all samples up to this second
    since: Option<u32>,
}

#[allow(clippy::struct_excessive_bools)] // switches of a page or a command line
#[derive(Serialize)]
pub struct Live {
    running: bool,
    frozen: bool,
    /// An analyzed file has been read completely
    finished: bool,
    /// The capture stopped with an error, or IPFIX data could not be decoded
    error: Option<String>,
    unread: usize,
    /// Counter that increases with every notification sound, and the sound
    sound: (u64, &'static str),
    /// Notification sounds are wanted at this volume (0–100)
    volume: u8,
    capture: usize,
    generation: u32,
    /// Whether `samples` replaces what the client has (otherwise it continues it)
    reset: bool,
    /// Unix time of the first packet; the samples count seconds from there
    start: i64,
    /// `[second, bytes in, bytes out, packets in, packets out]`
    samples: Vec<[u128; 5]>,
    /// Unit chosen for volumes: `packets`, `bytes` or `bits`
    unit: &'static str,
    totals: Totals,
}

#[derive(Serialize)]
struct Totals {
    in_bytes: u128,
    out_bytes: u128,
    in_packets: u128,
    out_packets: u128,
    /// Packets dropped by the adapter or by openpacketmon (live captures only)
    dropped: Option<u128>,
    connections: usize,
    hosts: usize,
}

pub fn unit_code(unit: DataUnit) -> &'static str {
    match unit {
        DataUnit::Packets => "packets",
        DataUnit::Bytes => "bytes",
        DataUnit::Bits => "bits",
    }
}

fn sound_code(sound: Sound) -> &'static str {
    match sound {
        Sound::Gulp => "gulp",
        Sound::Pop => "pop",
        Sound::Swhoosh => "swhoosh",
        Sound::None => "none",
    }
}

/// `GET /api/live`: state and new traffic samples of the capture
pub async fn live(
    State(state): State<AppState>,
    _session: Session,
    Query(query): Query<LiveQuery>,
) -> Json<Live> {
    let engine = state.engine.lock();
    let history = &engine.traffic_history;
    let capture = engine.capture_id();
    let reset = query.capture != Some(capture) || query.generation != Some(history.generation());
    let samples = history
        .samples()
        .iter()
        .filter(|s| reset || query.since.is_none_or(|since| s.second > since))
        .map(|s| {
            [
                u128::from(s.second),
                s.in_bytes,
                s.out_bytes,
                s.in_packets,
                s.out_packets,
            ]
        })
        .collect();
    let data = &engine.info_traffic.tot_data_info;
    let bytes = DataRepr::bytes(false);
    let packets = DataRepr::packets(false);
    let error = match &engine.capture_error {
        Some(CaptureError::Fatal(e)) => Some(e.clone()),
        Some(CaptureError::IpfixUndecodable) => {
            Some(crate::web::i18n::current().get("capture-ipfix-undecodable"))
        }
        None => None,
    };
    Json(Live {
        running: engine.is_running(),
        frozen: engine.frozen,
        finished: history.no_more_packets(),
        error,
        unread: engine.unread_notifications,
        sound: (engine.sound_event.0, sound_code(engine.sound_event.1)),
        volume: engine.conf.settings.notifications.volume,
        capture,
        generation: history.generation(),
        reset,
        start: history.first_packet_timestamp().secs(),
        samples,
        unit: unit_code(engine.conf.data_repr.data_unit),
        totals: Totals {
            in_bytes: data.incoming_data(bytes),
            out_bytes: data.outgoing_data(bytes),
            in_packets: data.incoming_data(packets),
            out_packets: data.outgoing_data(packets),
            dropped: engine
                .info_traffic
                .dropped_packets
                .map(|d| d.total(packets, *data)),
            connections: engine.info_traffic.map.len(),
            hosts: engine.info_traffic.hosts.len(),
        },
    })
}

#[derive(Serialize)]
pub struct Preview {
    name: String,
    /// Packets per second, oldest first
    packets: Vec<u128>,
    total: u128,
}

/// `GET /api/previews`: packets per second of every adapter in the last 30 seconds
pub async fn previews(State(state): State<AppState>, _session: Session) -> Json<Vec<Preview>> {
    let engine = state.engine.lock();
    Json(
        engine
            .previews
            .iter()
            .map(|p| Preview {
                name: p.device.get_name().clone(),
                packets: p.packets.iter().copied().collect(),
                total: p.tot_packets(),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::web::testutil::*;

    #[tokio::test]
    async fn live_values_without_a_capture() {
        let h = harness().await;
        let reply = send(&h.app, get("/api/live", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_str(&reply.body).unwrap();
        assert_eq!(json["running"], false);
        assert_eq!(json["reset"], true);
        assert_eq!(json["samples"].as_array().unwrap().len(), 0);
        assert_eq!(json["unit"], "bytes");

        let again = send(
            &h.app,
            get(
                &format!(
                    "/api/live?capture={}&generation={}&since=0",
                    json["capture"], json["generation"]
                ),
                Some(&h.cookie),
            ),
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&again.body).unwrap();
        assert_eq!(json["reset"], false);

        let previews = send(&h.app, get("/api/previews", Some(&h.cookie))).await;
        assert!(previews.body.starts_with('['));
    }
}
