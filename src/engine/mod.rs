//! The traffic monitor without a user interface.
//!
//! [`Engine`] owns the [`EngineState`] behind a mutex and runs the background work
//! (capture, adapter previews, latency measurements, update checks).
//! A user interface reads the state with [`Engine::lock`] and changes it through the
//! methods of [`EngineState`] or of the engine itself.

pub mod analysis;
pub mod packets;
pub mod previews;
pub mod state;
pub mod traffic_history;

use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use tokio::runtime::Handle;

use crate::config::conf::Conf;
use crate::engine::state::EngineState;
use crate::location;
use crate::networking::traffic_preview::traffic_preview;
use crate::networking::types::latency::{LatencyStatus, measure_latency};
use crate::utils::check_updates::{UpdateStatus, is_newer_release_available};
use crate::utils::error_logger::{ErrorLogger, Location};

/// Longest wait in [`Engine::stop`] for the capture threads to end
const CAPTURE_END_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone)]
pub struct Engine {
    state: Arc<Mutex<EngineState>>,
    runtime: Handle,
}

impl Engine {
    /// Creates the engine and starts the adapter previews.
    /// `runtime` runs the asynchronous jobs (latency, update checks).
    pub fn new(conf: Conf, runtime: Handle) -> Self {
        let engine = Self {
            state: Arc::new(Mutex::new(EngineState::new(conf))),
            runtime,
        };
        engine.start_previews();
        engine
    }

    /// Exclusive access to the state; keep the guard only briefly, the capture waits for it
    pub fn lock(&self) -> MutexGuard<'_, EngineState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts a capture from the selected source; returns `false` if it could not be started
    pub fn start(&self) -> bool {
        let started = self.lock().start();
        let Some(rx) = started else {
            // a failed start has already stopped the adapter previews
            if !self.lock().is_running() {
                self.start_previews();
            }
            return false;
        };
        let state = self.state.clone();
        let _ = thread::Builder::new()
            .name("thread_engine_capture".to_string())
            .spawn(move || {
                while let Ok(msg) = rx.recv_blocking() {
                    state
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .handle_backend_message(msg);
                }
            })
            .log_err(location!());
        true
    }

    /// Stops the current capture, clears its data and shows the adapter previews again
    pub fn stop(&self) {
        let ended = {
            let mut state = self.lock();
            state.stop();
            state.take_capture_ended()
        };
        // The previews start a helper per adapter. Under Windows a process inherits the files
        // open at its start, so the capture must have closed the file it imports or writes first,
        // or the file stays locked as long as the helpers run. The capture threads notice the stop
        // within 150 ms (the timeout of the helper and of the packet channel); never wait forever.
        if let Some(ended) = ended {
            let _ = ended.recv_timeout(CAPTURE_END_TIMEOUT);
        }
        self.start_previews();
    }

    fn start_previews(&self) {
        let (tx, rx) = async_channel::unbounded();
        let _ = thread::Builder::new()
            .name("thread_traffic_preview".to_string())
            .spawn(move || {
                traffic_preview(&tx);
            })
            .log_err(location!());
        self.lock().set_preview_rx(rx.clone());
        let state = self.state.clone();
        let _ = thread::Builder::new()
            .name("thread_engine_preview".to_string())
            .spawn(move || {
                while let Ok(msg) = rx.recv_blocking() {
                    state
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .handle_traffic_preview(msg);
                }
            })
            .log_err(location!());
    }

    /// Measures the latency to `ip` in the background
    pub fn measure_latency(&self, ip: IpAddr) {
        self.lock()
            .latency_statuses
            .insert(ip, LatencyStatus::Measuring);
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let status = measure_latency(ip).await;
            state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .latency_statuses
                .insert(ip, status);
        });
    }

    /// Checks GitHub for a newer release in the background, unless disabled in the settings
    pub fn check_newer_release(&self) {
        {
            let mut state = self.lock();
            if state.conf.updates.disable_checks() {
                return;
            }
            state.update_status = UpdateStatus::InProgress;
        }
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let status = is_newer_release_available().await;
            state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .update_status = status;
        });
    }

    /// Writes the settings to disk
    /// Downloads the geolocation databases and waits for them; not on a thread of the runtime
    pub fn download_geo(&self) -> Result<(), crate::mmdb::geo::DownloadError> {
        self.runtime.block_on(crate::mmdb::geo::download_dbip())
    }

    pub fn store_conf(&self) {
        // tests share one settings file (`Conf::test_path`), and only the tests of `Conf` may write it
        #[cfg(not(test))]
        {
            let conf = self.lock().conf.clone();
            let _ = conf.store();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::{Duration, Instant};

    use serial_test::serial;

    use super::*;
    use crate::networking::types::capture_context::CaptureSourcePicklist;
    use crate::networking::types::data_representation::DataRepr;
    use crate::networking::types::service::Service;

    /// Removes the folder of a test and fails the test if that does not work. Under Windows a file
    /// may stay locked for a moment after it was closed (virus scanner, indexer), so it tries again
    /// for a while. During a panic it only tries once, a second panic would abort all tests.
    pub(crate) fn remove_test_dir(dir: &std::path::Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match std::fs::remove_dir_all(dir) {
                Ok(()) => return,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
                Err(_) if std::thread::panicking() => return,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => panic!("could not remove {}: {e}", dir.display()),
            }
        }
    }

    /// Ethernet frame with an IPv4 header and a TCP (`tcp = true`) or UDP header
    pub(crate) fn frame(
        src: [u8; 4],
        dst: [u8; 4],
        src_port: u16,
        dst_port: u16,
        tcp: bool,
        payload: usize,
    ) -> Vec<u8> {
        let l4_len = if tcp { 20 } else { 8 };
        let ip_len = u16::try_from(20 + l4_len + payload).unwrap();
        let mut f = Vec::new();
        f.extend_from_slice(&[0x02, 0, 0, 0, 0, 0x01, 0x02, 0, 0, 0, 0, 0x02, 0x08, 0x00]);
        f.extend_from_slice(&[
            0x45,
            0,
            0,
            0,
            0,
            0,
            0x40,
            0,
            64,
            if tcp { 6 } else { 17 },
            0,
            0,
        ]);
        f[16..18].copy_from_slice(&ip_len.to_be_bytes());
        f.extend_from_slice(&src);
        f.extend_from_slice(&dst);
        f.extend_from_slice(&src_port.to_be_bytes());
        f.extend_from_slice(&dst_port.to_be_bytes());
        if tcp {
            // sequence, acknowledgment, data offset 5, flags ACK, window, checksum, urgent pointer
            f.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 0x50, 0x10, 0xff, 0xff, 0, 0, 0, 0]);
        } else {
            let udp_len = u16::try_from(8 + payload).unwrap();
            f.extend_from_slice(&udp_len.to_be_bytes());
            f.extend_from_slice(&[0, 0]);
        }
        f.resize(f.len() + payload, 0xab);
        f
    }

    /// Writes a classic PCAP file (Ethernet link type) with the given (seconds, frame) records
    pub(crate) fn write_pcap(path: &std::path::Path, packets: &[(u32, Vec<u8>)]) {
        let mut out = Vec::new();
        out.extend_from_slice(&0xa1b2_c3d4_u32.to_le_bytes());
        out.extend_from_slice(&2_u16.to_le_bytes());
        out.extend_from_slice(&4_u16.to_le_bytes());
        out.extend_from_slice(&[0; 8]); // timezone, accuracy
        out.extend_from_slice(&65_535_u32.to_le_bytes());
        out.extend_from_slice(&1_u32.to_le_bytes()); // Ethernet
        for (secs, data) in packets {
            let len = u32::try_from(data.len()).unwrap();
            out.extend_from_slice(&secs.to_le_bytes());
            out.extend_from_slice(&0_u32.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(data);
        }
        std::fs::write(path, out).unwrap();
    }

    #[test]
    #[serial]
    fn pcap_import_fills_the_state() {
        let local = [192, 168, 1, 10];
        let dns = [192, 168, 1, 1];
        let web = [192, 168, 1, 20];
        let packets = vec![
            (1_000, frame(local, dns, 50_000, 53, false, 30)),
            (1_000, frame(dns, local, 53, 50_000, false, 60)),
            (1_001, frame(local, web, 40_000, 443, true, 100)),
            (1_003, frame(web, local, 443, 40_000, true, 1_000)),
        ];
        let tot_bytes: usize = packets.iter().map(|(_, f)| f.len()).sum();
        let dir = std::env::temp_dir().join(format!("openpacketmon-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.pcap");
        write_pcap(&path, &packets);

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        {
            let mut state = engine.lock();
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import(path.to_string_lossy().to_string());
        }
        assert!(engine.start());
        assert!(engine.lock().is_running());

        let deadline = Instant::now() + Duration::from_secs(30);
        while !engine.lock().traffic_history.no_more_packets() {
            assert!(
                Instant::now() < deadline,
                "the capture file was not processed"
            );
            thread::sleep(Duration::from_millis(50));
        }

        {
            let state = engine.lock();
            assert!(state.capture_error.is_none());
            let data = &state.info_traffic.tot_data_info;
            assert_eq!(data.tot_data(DataRepr::packets(false)), 4);
            assert_eq!(
                data.tot_data(DataRepr::bytes(false)),
                u128::try_from(tot_bytes).unwrap()
            );
            assert_eq!(state.info_traffic.map.len(), 4);
            assert!(
                state
                    .info_traffic
                    .services
                    .contains_key(&Service::Name("https"))
            );
            assert!(
                state
                    .info_traffic
                    .services
                    .contains_key(&Service::Name("domain"))
            );
            // one sample per second of the file: 1000, 1001, 1002 (idle), 1003
            assert_eq!(state.traffic_history.ticks(), 4);
            let (entries, tot, agglomerate, _) = state.searched_entries(
                None,
                crate::report::types::sort_type::SortType::Neutral,
                DataRepr::bytes(false),
                1,
                3,
            );
            assert_eq!((entries.len(), tot), (3, 4));
            assert_eq!(agglomerate.tot_data(DataRepr::packets(false)), 4);

            // the packet store keeps every packet with its bytes and connection
            let store = state.packets.as_ref().unwrap().lock();
            assert_eq!(store.len(), 4);
            let third = store.get(3).unwrap();
            assert_eq!(*third.data, *packets[2].1);
            let (key, _) = third.conn.unwrap();
            assert_eq!(key.dst_port, Some(443));
            let d = crate::dissect::dissect(&third.data, store.link_type());
            // the payload on port 443 is data of a TLS connection
            assert_eq!(d.protocol, "TLS");
            assert_eq!(d.src, "192.168.1.10");
            assert_eq!(d.info, "Continuation Data");
            assert!(
                d.layers[2]
                    .title
                    .starts_with("Transmission Control Protocol")
            );
            let pcap = store.to_pcap(store.iter());
            assert_eq!(pcap.len(), 24 + 4 * 16 + tot_bytes);
        }

        engine.stop();
        {
            let state = engine.lock();
            assert!(!state.is_running());
            assert_eq!(state.info_traffic.map.len(), 0);
        }
        // the file is closed once the capture has stopped, also under Windows, where the helpers of
        // the adapter previews would otherwise inherit it and keep it locked
        remove_test_dir(&dir);
    }

    /// Under Windows the helpers of the adapter previews inherit the files open when they start;
    /// stopping must close the imported file before it starts them, or the file stays locked as
    /// long as they run. Needs real helpers (Npcap under Windows) and no other test at the same
    /// time: `cargo test --bin openpacketmon the_imported_file -- --ignored`
    #[test]
    #[serial]
    #[ignore = "starts real capture helpers, run it alone"]
    fn the_imported_file_is_free_right_after_stopping() {
        use crate::networking::types::capture_helper::tests::REAL_HELPERS;
        use std::sync::atomic::Ordering;

        REAL_HELPERS.store(true, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("openpacketmon-stop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("long.pcap");
        // long enough to be still reading when stopped
        let packets: Vec<_> = (0..200_000_u32)
            .map(|i| {
                let port = 40_000 + u16::try_from(i % 1_000).unwrap();
                (
                    1_000 + i / 1_000,
                    frame([192, 168, 1, 10], [192, 168, 1, 20], port, 443, true, 100),
                )
            })
            .collect();
        write_pcap(&path, &packets);

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        // the previews are running, with their helpers
        std::thread::sleep(Duration::from_secs(2));
        let import = |path: &std::path::Path| {
            let mut state = engine.lock();
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import(path.to_string_lossy().to_string());
        };
        // locked by a helper, the file would stay so for as long as the previews run
        let free = |path: &std::path::Path| {
            let deadline = Instant::now() + Duration::from_secs(2);
            while let Err(e) = std::fs::remove_file(path) {
                assert!(Instant::now() < deadline, "still locked: {e}");
                std::thread::sleep(Duration::from_millis(50));
            }
        };

        // stopped while reading
        import(&path);
        assert!(engine.start());
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            !engine.lock().traffic_history.no_more_packets(),
            "already read"
        );
        engine.stop();
        free(&path);

        // read to the end, with many addresses still waiting for their names (documentation
        // addresses, nobody answers quickly for them)
        let short = dir.join("short.pcap");
        let remote: Vec<_> = (0..500_u32)
            .map(|i| {
                let host = u8::try_from(i % 250 + 1).unwrap();
                let net = if i < 250 {
                    [198, 51, 100]
                } else {
                    [203, 0, 113]
                };
                (
                    1_000 + i / 100,
                    frame(
                        [192, 168, 1, 10],
                        [net[0], net[1], net[2], host],
                        40_000,
                        443,
                        true,
                        100,
                    ),
                )
            })
            .collect();
        write_pcap(&short, &remote);
        import(&short);
        assert!(engine.start());
        let deadline = Instant::now() + Duration::from_secs(30);
        while !engine.lock().traffic_history.no_more_packets() {
            assert!(Instant::now() < deadline, "not read");
            std::thread::sleep(Duration::from_millis(20));
        }
        engine.stop();
        free(&short);

        remove_test_dir(&dir);
        REAL_HELPERS.store(false, Ordering::Relaxed);
    }

    #[test]
    #[serial]
    fn start_reports_fatal_errors() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        {
            let mut state = engine.lock();
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import("/nonexistent/openpacketmon.pcap".to_string());
        }
        assert!(!engine.start());
        let state = engine.lock();
        assert!(!state.is_running());
        assert!(matches!(
            state.capture_error,
            Some(crate::networking::types::capture_context::CaptureError::Fatal(_))
        ));
    }

    #[test]
    #[serial]
    fn a_source_without_ip_traffic_is_not_started() {
        // an empty capture file of Bluetooth HCI (link type 201), as `bluetooth0` delivers
        let dir = std::env::temp_dir().join(format!("openpacketmon-noip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bluetooth.pcap");
        let mut header = vec![0xd4, 0xc3, 0xb2, 0xa1, 2, 0, 4, 0];
        header.extend_from_slice(&[0; 8]);
        header.extend_from_slice(&65_535_u32.to_le_bytes());
        header.extend_from_slice(&201_u32.to_le_bytes());
        std::fs::write(&path, header).unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        {
            let mut state = engine.lock();
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import(path.display().to_string());
        }
        assert!(!engine.start());
        {
            let state = engine.lock();
            assert!(!state.is_running());
            assert!(matches!(
                &state.capture_error,
                Some(crate::networking::types::capture_context::CaptureError::NoIp(link))
                    if link.starts_with("BLUETOOTH_HCI_H4_WITH_PHDR")
            ));
        }
        remove_test_dir(&dir);
    }
}
