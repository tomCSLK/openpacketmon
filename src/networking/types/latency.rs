// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/latency.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use std::net::IpAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use surge_ping::{Client, Config, ICMP, PingIdentifier, PingSequence, SurgeError};

const PING_TIMEOUT: Duration = Duration::from_secs(2);
const PING_PAYLOAD: [u8; 8] = [0; 8];
const PING_COUNT: usize = 3;

static IPV4_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();
static IPV6_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();
static PING_SEQUENCE: AtomicU16 = AtomicU16::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LatencyStatus {
    Measuring,
    Measured(Duration),
    Failed(String),
}

pub async fn measure_latency(ip: IpAddr) -> LatencyStatus {
    let client = match client_for(ip) {
        Ok(client) => client,
        Err(error) => return LatencyStatus::Failed(error),
    };

    let mut pinger = client.pinger(ip, ping_identifier()).await;
    pinger.timeout(PING_TIMEOUT);

    let mut sum = Duration::ZERO;
    let mut received: u32 = 0;
    let mut last_error = None;
    for _ in 0..PING_COUNT {
        match pinger.ping(next_sequence(), &PING_PAYLOAD).await {
            Ok((_, latency)) => {
                sum += latency;
                received += 1;
            }
            Err(error @ SurgeError::Timeout { .. }) => last_error = Some(error.to_string()),
            Err(error) => {
                last_error = Some(error.to_string());
                break;
            }
        }
    }

    match received {
        0 => LatencyStatus::Failed(last_error.unwrap_or_else(|| "No reply".to_string())),
        n => LatencyStatus::Measured(sum / n),
    }
}

fn client_for(ip: IpAddr) -> Result<Arc<Client>, String> {
    let (cell, kind) = match ip {
        IpAddr::V4(_) => (&IPV4_CLIENT, ICMP::V4),
        IpAddr::V6(_) => (&IPV6_CLIENT, ICMP::V6),
    };

    if let Some(client) = cell.get() {
        return Ok(Arc::clone(client));
    }

    let client = latency_client(kind)?;
    Ok(Arc::clone(cell.get_or_init(|| client)))
}

fn latency_client(kind: ICMP) -> Result<Arc<Client>, String> {
    let config = match kind {
        ICMP::V4 => Config::default(),
        ICMP::V6 => Config::builder().kind(ICMP::V6).build(),
    };

    Client::new(&config)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

fn ping_identifier() -> PingIdentifier {
    #[allow(clippy::cast_possible_truncation)]
    PingIdentifier(std::process::id() as u16)
}

fn next_sequence() -> PingSequence {
    next_from(&PING_SEQUENCE)
}

/// The next number of a counter, starting again at 0 after `u16::MAX`
fn next_from(counter: &AtomicU16) -> PingSequence {
    PingSequence(counter.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use std::sync::atomic::AtomicU16;

    use super::{LatencyStatus, measure_latency, next_from, ping_identifier};

    #[test]
    fn test_uses_process_id_as_ping_identifier() {
        assert_eq!(ping_identifier().0, std::process::id() as u16);
    }

    #[test]
    fn test_increments_ping_sequence() {
        // a counter of its own: the shared one also counts the pings of tests running at the same time
        let counter = AtomicU16::new(0);
        assert_eq!(next_from(&counter).0, 0);
        assert_eq!(next_from(&counter).0, 1);
        assert_eq!(next_from(&counter).0, 2);
        let counter = AtomicU16::new(u16::MAX);
        assert_eq!(next_from(&counter).0, u16::MAX);
        assert_eq!(next_from(&counter).0, 0);
    }

    #[tokio::test]
    async fn test_measures_ipv4_loopback_latency() {
        let status = measure_latency(IpAddr::V4(Ipv4Addr::LOCALHOST)).await;
        assert!(matches!(status, LatencyStatus::Measured(_)));
    }

    #[tokio::test]
    async fn test_measures_ipv6_loopback_latency() {
        let status = measure_latency(IpAddr::V6(Ipv6Addr::LOCALHOST)).await;
        assert!(matches!(status, LatencyStatus::Measured(_)));
    }
}
