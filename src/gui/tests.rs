//! The window without a window: the pages run against a real engine that reads a real capture
//! file, drawn into memory.
//!
//! `GUI_SHOTS=<folder> cargo test --bin openpacketmon gui::tests::shots -- --ignored` writes a
//! picture of every page in both designs, to look at them.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, Theme};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serial_test::serial;

use super::*;
use crate::config::conf::Conf;
use crate::config::favorite::Favorite;
use crate::engine::tests::{frame, remove_test_dir, write_pcap};
use crate::networking::capture::BackendTrafficMessage;
use crate::networking::types::capture_context::CaptureSourcePicklist;

struct Fixture {
    // the jobs of the engine run here
    _runtime: tokio::runtime::Runtime,
    engine: Engine,
    /// Folder of the capture file, removed at the end
    dir: Option<PathBuf>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(dir) = &self.dir {
            self.engine.stop();
            remove_test_dir(dir);
        }
    }
}

/// A capture file in a folder of its own
fn sample_file(rows: usize) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let local = [192, 168, 1, 10];
    let dns = [192, 168, 1, 1];
    let web = [192, 168, 1, 20];
    let far = [93, 184, 216, 34];
    let mut packets = Vec::new();
    for i in 0..rows {
        let second = 1_000 + (i / 40) as u32;
        let port = 40_000 + (i % 30) as u16;
        packets.push(match i % 4 {
            0 => (second, frame(local, dns, 50_000, 53, false, 30)),
            1 => (second, frame(dns, local, 53, 50_000, false, 60)),
            2 => (second, frame(local, web, port, 443, true, 100)),
            _ => (second, frame(far, local, 443, port, true, 1_000)),
        });
    }
    let dir = std::env::temp_dir().join(format!(
        "openpacketmon-gui-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("sample-{rows}.pcap"));
    write_pcap(&path, &packets);
    path
}

fn wait_for(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn engine_with(rows: usize) -> Fixture {
    engine_reading((rows > 0).then(|| sample_file(rows)))
}

/// A made-up call with SIP, RTP both ways and RTCP, a stream of RTP without SIP, and a
/// registration, between other packets; times in microseconds
fn call_file() -> PathBuf {
    use crate::dissect::tests::{rtcp_report, rtp_bytes, sdp_offer, sip_message, udp_between};
    let phone = [192, 0, 2, 20];
    let provider = [203, 0, 113, 60];
    let media = [203, 0, 113, 61];
    let id = "3f2a9c1e7b54@192.0.2.20";
    let sip = |out: bool, first: &str, cseq: &str, extra: &[(&str, &str)], sdp: Option<&str>| {
        let mut headers = vec![
            (
                "Via",
                "SIP/2.0/UDP 192.0.2.20:5060;branch=z9hG4bK3f9a1c;rport",
            ),
            (
                "From",
                "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=c81d07",
            ),
            ("To", "<sip:support@example.com>"),
            ("Call-ID", id),
            ("CSeq", cseq),
        ];
        headers.extend_from_slice(extra);
        headers.push(if out {
            ("User-Agent", "Linphone/5.2 (belle-sip)")
        } else {
            ("Server", "Beispiel-SBC/3.1")
        });
        let m = sip_message(first, &headers, sdp);
        if out {
            udp_between(phone, 5060, provider, 5060, &m)
        } else {
            udp_between(provider, 5060, phone, 5060, &m)
        }
    };
    let offer = sdp_offer("192.0.2.20", 7078, &[8, 0, 101]);
    let answer = sdp_offer("203.0.113.61", 31456, &[8, 101]);
    let contact = [("Contact", "<sip:anna@192.0.2.20:5060>")];
    let mut packets: Vec<(u64, Vec<u8>)> = vec![
        (
            0,
            frame([192, 168, 1, 10], [192, 168, 1, 1], 50_000, 53, false, 30),
        ),
        (
            1_000,
            sip(
                true,
                "INVITE sip:support@example.com SIP/2.0",
                "1 INVITE",
                &contact,
                Some(&offer),
            ),
        ),
        (
            41_300,
            sip(false, "SIP/2.0 100 Trying", "1 INVITE", &[], None),
        ),
        (
            612_800,
            sip(false, "SIP/2.0 180 Ringing", "1 INVITE", &[], None),
        ),
        (
            4_200_500,
            sip(false, "SIP/2.0 200 OK", "1 INVITE", &contact, Some(&answer)),
        ),
        (
            4_212_000,
            sip(
                true,
                "ACK sip:support@example.com SIP/2.0",
                "1 ACK",
                &[],
                None,
            ),
        ),
    ];
    let start = 4_220_000_u64;
    for i in 0..60_u16 {
        let t = start + u64::from(i) * 20_000;
        let out = rtp_bytes(8, 21_043 + i, u32::from(i) * 160, 0x5e1f_02a7, i == 0, 160);
        let back = rtp_bytes(
            8,
            3_310 + i,
            92_400_160 + u32::from(i) * 160,
            0x0b77_c3d1,
            i == 0,
            160,
        );
        packets.push((t, udp_between(phone, 7078, media, 31_456, &out)));
        packets.push((t + 7_300, udp_between(media, 31_456, phone, 7078, &back)));
        if i == 30 {
            let report = rtcp_report(0x5e1f_02a7, 0x0b77_c3d1, "anna@192.0.2.20");
            packets.push((t + 9_000, udp_between(phone, 7079, media, 31_457, &report)));
        }
        // a video conference in the browser: RTP without SIP
        if i % 3 == 0 {
            let m = rtp_bytes(
                111,
                40_981 + i / 3,
                2_830_080 + u32::from(i) * 320,
                0x9b2e_7714,
                i == 0,
                140,
            );
            packets.push((
                t + 11_000,
                udp_between(phone, 52_714, [198, 51, 100, 7], 10_000, &m),
            ));
        }
    }
    packets.push((
        5_500_000,
        sip(
            true,
            "BYE sip:support@example.com SIP/2.0",
            "2 BYE",
            &[],
            None,
        ),
    ));
    packets.push((5_540_200, sip(false, "SIP/2.0 200 OK", "2 BYE", &[], None)));
    let dir = std::env::temp_dir().join(format!("openpacketmon-gui-call-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("call.pcap");
    let mut out = Vec::new();
    out.extend_from_slice(&0xa1b2_c3d4_u32.to_le_bytes());
    out.extend_from_slice(&2_u16.to_le_bytes());
    out.extend_from_slice(&4_u16.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(&65_535_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    for (us, data) in &packets {
        let len = u32::try_from(data.len()).unwrap();
        out.extend_from_slice(
            &u32::try_from(1_794_225_600 + us / 1_000_000)
                .unwrap()
                .to_le_bytes(),
        );
        out.extend_from_slice(&u32::try_from(us % 1_000_000).unwrap().to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(data);
    }
    std::fs::write(&path, out).unwrap();
    path
}

/// An engine that has read this capture file (none: an engine without a capture)
fn engine_reading(path: Option<PathBuf>) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let engine = Engine::new(Conf::default(), runtime.handle().clone());
    let mut dir = None;
    if let Some(path) = path {
        dir = path.parent().map(Path::to_path_buf);
        {
            let mut state = engine.lock();
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import(path.to_string_lossy().to_string());
        }
        assert!(engine.start());
        wait_for("the capture file was not read", || {
            engine.lock().traffic_history.no_more_packets()
        });
    }
    Fixture {
        _runtime: runtime,
        engine,
        dir,
    }
}

fn window(fixture: &Fixture, theme: Theme, lang: Lang) -> Harness<'static, App> {
    window_of_size(fixture, theme, lang, 820.0)
}

fn window_of_size(
    fixture: &Fixture,
    theme: Theme,
    lang: Lang,
    height: f32,
) -> Harness<'static, App> {
    let engine = fixture.engine.clone();
    let mut builder = Harness::builder()
        .with_size(egui::vec2(1280.0, height))
        .with_theme(theme);
    // only pictures need a renderer, and with it a graphics adapter; the CI has none
    if std::env::var_os("GUI_SHOTS").is_some() {
        builder = builder.wgpu();
    }
    let mut harness = builder.build_eframe(move |cc| {
        // never started, so without a place for its file
        let mcp = crate::mcp::Access::new(engine.clone(), None);
        let mut app = App::new(&cc.egui_ctx, engine, mcp);
        app.t = lang;
        // so that the warnings of egui fail the test: their icon is in no font
        cc.egui_ctx
            .set_missing_glyph_policy(egui::MissingGlyphPolicy::Panic);
        app
    });
    harness.state_mut().pages = pages::Pages::default();
    harness
}

/// The pages of a made-up call: the packet list with `sip or rtp`, the details of SIP, RTP
/// (with SDP and without SIP) and RTCP, the statistics
fn call_shots(folder: &Path) {
    let call = engine_reading(Some(call_file()));
    for (theme, suffix) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut harness = window(&call, theme, Lang::De);
        harness.state_mut().go(Page::Packets {
            filter: "sip or rtp or rtcp".to_owned(),
            connections: String::new(),
            goto: Some(1),
        });
        shot(&mut harness, folder, &format!("voip-packets-{suffix}"));
        harness.state_mut().go(Page::Packets {
            filter: String::new(),
            connections: String::new(),
            goto: Some(1),
        });
        shot(&mut harness, folder, &format!("voip-all-{suffix}"));
        for (name, n) in [
            ("invite", 2),
            ("ok", 5),
            ("rtp", 7),
            ("rtcp", 79),
            ("rtp-form", 16),
        ] {
            harness.state_mut().go(Page::Packet(n));
            shot(&mut harness, folder, &format!("voip-{name}-{suffix}"));
        }
        harness.state_mut().go(Page::Statistics {
            filter: String::new(),
            connections: String::new(),
        });
        harness.set_size(egui::vec2(1280.0, 1300.0));
        shot(&mut harness, folder, &format!("voip-statistics-{suffix}"));
        harness.set_size(egui::vec2(1280.0, 820.0));
    }
    calls_shots(folder);
}

/// The pages of the draft's calls: the list, every kind of call, the settings, the overview
fn calls_shots(folder: &Path) {
    let fixture = crate::view::calls::tests::engine_with_calls();
    let engine = &fixture.engine;
    let list = crate::view::calls::call_list(Lang::De, engine);
    let key = |state: crate::view::calls::State, to: &str| {
        list.rows
            .iter()
            .find(|r| r.status.state == state && (to.is_empty() || r.to.name == to))
            .map(|r| r.key)
            .unwrap()
    };
    use crate::view::calls::State as S;
    let pages = [
        ("ended", key(S::Ended, "support@example.com"), 2_200.0),
        ("bad", key(S::Ended, "Anna Beispiel"), 2_200.0),
        ("rejected", key(S::Rejected, ""), 1_300.0),
        ("missed", key(S::Missed, ""), 1_300.0),
        ("running", key(S::Running, ""), 1_800.0),
        ("rtp", key(S::Rtp, ""), 1_300.0),
    ];
    let wrap = Fixture {
        _runtime: tokio::runtime::Runtime::new().unwrap(),
        engine: engine.clone(),
        dir: None,
    };
    for (theme, suffix) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut harness = window_of_size(&wrap, theme, Lang::De, 1_000.0);
        harness.state_mut().go(Page::Calls);
        shot(&mut harness, folder, &format!("voip-calls-{suffix}"));
        for (name, key, height) in pages {
            harness.set_size(egui::vec2(1280.0, height));
            harness.state_mut().go(Page::Call(key));
            if matches!(name, "ended" | "bad" | "running") {
                wait_for_audio(&mut harness, key);
            }
            shot(&mut harness, folder, &format!("voip-call-{name}-{suffix}"));
        }
        // the player halfway through, only the sent side, and the note on § 201 before listening
        harness.set_size(egui::vec2(1280.0, 1000.0));
        harness.state_mut().go(Page::Call(pages[1].1));
        wait_for_audio(&mut harness, pages[1].1);
        harness
            .state_mut()
            .pages
            .calls
            .player
            .show_at(26.0, crate::view::calls::audio::Channel::Sent);
        shot(&mut harness, folder, &format!("voip-player-{suffix}"));
        harness.state_mut().pages.calls.player.ask = Some(pages::calls::player::Ask::Play);
        shot(&mut harness, folder, &format!("voip-law-{suffix}"));
        harness.state_mut().pages.calls.player.ask = None;
        harness.set_size(egui::vec2(1280.0, 1000.0));
        harness.state_mut().go(Page::Settings(SettingsPanel::Calls));
        shot(&mut harness, folder, &format!("voip-settings-{suffix}"));
        harness.state_mut().go(Page::Overview);
        shot(&mut harness, folder, &format!("voip-overview-{suffix}"));
        harness.state_mut().go(Page::Statistics {
            filter: String::new(),
            connections: String::new(),
        });
        shot(&mut harness, folder, &format!("voip-protocols-{suffix}"));
        // narrow windows hide columns
        harness.set_size(egui::vec2(980.0, 1000.0));
        harness.state_mut().go(Page::Calls);
        shot(&mut harness, folder, &format!("voip-calls-narrow-{suffix}"));
        // from 900 px down the names stand above the waveforms
        harness.set_size(egui::vec2(860.0, 2_600.0));
        harness.state_mut().go(Page::Call(pages[0].1));
        wait_for_audio(&mut harness, pages[0].1);
        shot(&mut harness, folder, &format!("voip-call-narrow-{suffix}"));
    }
    // English
    let mut harness = window_of_size(&wrap, Theme::Light, Lang::En, 1_000.0);
    harness.state_mut().go(Page::Calls);
    shot(&mut harness, folder, "voip-calls-en");
    harness.set_size(egui::vec2(1280.0, 2_200.0));
    harness.state_mut().go(Page::Call(pages[1].1));
    shot(&mut harness, folder, "voip-call-bad-en");
    // without calls
    let empty = engine_with(40);
    let mut harness = window_of_size(&empty, Theme::Light, Lang::De, 1_000.0);
    harness.state_mut().go(Page::Calls);
    shot(&mut harness, folder, "voip-calls-empty");
}

/// Runs frames until the voice of the call in view is loaded
fn wait_for_audio(harness: &mut Harness<'static, App>, call: crate::view::calls::CallKey) {
    for _ in 0..400 {
        harness.step();
        if harness.state().pages.calls.player.has_audio(call) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("the voice of the call was not loaded");
}

/// Runs frames until the background jobs have delivered
fn settle(harness: &mut Harness<'static, App>) {
    for _ in 0..12 {
        harness.step();
        std::thread::sleep(Duration::from_millis(25));
    }
    harness.step();
}

fn shot(harness: &mut Harness<'static, App>, folder: &Path, name: &str) {
    // `GUI_SHOTS_ONLY=overview` takes only the pictures whose name contains the word
    if std::env::var("GUI_SHOTS_ONLY").is_ok_and(|only| !name.contains(&only)) {
        return;
    }
    settle(harness);
    let image = harness.render().unwrap();
    image.save(folder.join(format!("{name}.png"))).unwrap();
}

#[test]
#[serial]
fn the_access_for_assistants_is_turned_on_in_the_settings() {
    let fixture = engine_with(0);
    let mut harness = window_of_size(&fixture, Theme::Light, Lang::En, 2_000.0);
    harness.state_mut().page = Page::Settings(SettingsPanel::Assistants);
    settle(&mut harness);
    assert!(!fixture.engine.lock().conf.mcp.enabled);
    assert_eq!(harness.state().mcp.status(), crate::mcp::Status::Off);

    // the switch takes effect at once, without saving
    harness
        .get_by_label("Allow AI assistants to access openpacketmon")
        .click();
    settle(&mut harness);
    assert!(fixture.engine.lock().conf.mcp.enabled);
    // this window has no place for the file of the access, so it says it could not set it up
    assert_eq!(harness.state().mcp.status(), crate::mcp::Status::Failed);
    let error = harness.state().t.get("settings-mcp-failed");
    let _ = harness.get_by_label(&error);
}

#[test]
#[serial]
fn the_content_of_packets_is_allowed_until_the_access_closes() {
    let fixture = engine_with(0);
    // the whole page in the window: a click does not scroll to what lies below it
    let mut harness = window_of_size(&fixture, Theme::Light, Lang::En, 2_400.0);
    let dir = std::env::temp_dir().join(format!("openpacketmon-gui-mcp-{}", std::process::id()));
    let mcp = crate::mcp::Access::new(fixture.engine.clone(), Some(dir.join("access.json")));
    harness.state_mut().mcp = mcp.clone();
    harness.state_mut().page = Page::Settings(SettingsPanel::Assistants);
    settle(&mut harness);
    let label = |harness: &Harness<'static, App>, id: &str| harness.state().t.get(id);
    assert!(
        harness
            .query_by_label(&label(&harness, "mcp-open"))
            .is_none()
    );

    // both switches take effect at once; the second one is there only while the first is on
    for (click, content) in [
        (&["settings-mcp-enabled", "settings-mcp-content"][..], true),
        (&["settings-mcp-content"][..], false),
    ] {
        for id in click {
            harness.get_by_label(&label(&harness, id)).click();
            settle(&mut harness);
        }
        assert!(matches!(mcp.status(), crate::mcp::Status::Active { .. }));
        assert_eq!(mcp.content_allowed(), content);
        let (note, state) = if content {
            ("mcp-open-content", "settings-mcp-content-on")
        } else {
            ("mcp-open", "settings-mcp-on")
        };
        let _ = harness.get_by_label(&label(&harness, note));
        let _ = harness.get_by_label(&label(&harness, state));
    }

    harness
        .get_by_label(&label(&harness, "settings-mcp-enabled"))
        .click();
    settle(&mut harness);
    assert_eq!(mcp.status(), crate::mcp::Status::Off);
    assert!(!mcp.content_allowed());
    assert!(!fixture.engine.lock().conf.mcp.enabled);
    assert!(
        harness
            .query_by_label(&label(&harness, "mcp-open"))
            .is_none()
    );
    assert!(
        harness
            .query_by_label(&label(&harness, "settings-mcp-content"))
            .is_none()
    );
    remove_test_dir(&dir);
}

#[test]
#[serial]
#[ignore = "writes pictures for a person to look at"]
fn shots() {
    let folder = PathBuf::from(std::env::var("GUI_SHOTS").unwrap());
    std::fs::create_dir_all(&folder).unwrap();
    if std::env::var("GUI_SHOTS_ONLY").is_ok_and(|only| only.contains("voip")) {
        call_shots(&folder);
        return;
    }
    let idle = engine_with(0);
    let busy = engine_with(2_000);
    for (theme, suffix) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        // adapters without IP traffic are only known once the preview could open them
        for preview in &mut idle.engine.lock().previews {
            let name = preview.device.get_name();
            preview.no_ip = name.starts_with("bluetooth") || name.starts_with("nf");
        }
        let mut harness = window(&idle, theme, Lang::De);
        shot(&mut harness, &folder, &format!("capture-{suffix}"));
        if std::env::var("GUI_SHOTS_ONLY").is_ok_and(|only| only.contains("capture")) {
            // made-up packets of the last 30 seconds, to see the curves
            for (i, preview) in idle.engine.lock().previews.iter_mut().enumerate() {
                if !preview.no_ip && i != 1 {
                    preview.packets = (0..30_u128)
                        .map(|k| (k * 7 + i as u128 * 5) % 11 * (40 / (i as u128 + 1)) + 20)
                        .collect();
                }
            }
            harness.set_size(egui::vec2(1280.0, 1150.0));
            shot(&mut harness, &folder, &format!("capture-tall-{suffix}"));
            harness.get_by_label("Aufzeichnungsdatei").click();
            shot(&mut harness, &folder, &format!("capture-file-{suffix}"));
            harness.get_by_label("IPFIX-Kollektor").click();
            shot(&mut harness, &folder, &format!("capture-ipfix-{suffix}"));
        }

        let mut harness = window(&busy, theme, Lang::De);
        harness.state_mut().page = Page::Capture;
        shot(&mut harness, &folder, &format!("running-{suffix}"));

        harness.state_mut().go(Page::Packets {
            filter: String::new(),
            connections: String::new(),
            goto: None,
        });
        shot(&mut harness, &folder, &format!("packets-{suffix}"));
        shot(&mut harness, &folder, &format!("packets2-{suffix}"));

        harness.state_mut().go(Page::Packets {
            filter: "tcp and ip.addr == 192.168.1.20".to_owned(),
            connections: String::new(),
            goto: None,
        });
        shot(&mut harness, &folder, &format!("filtered-{suffix}"));

        harness.state_mut().go(Page::Packets {
            filter: "tcp".to_owned(),
            connections: "port == 443".to_owned(),
            goto: None,
        });
        shot(&mut harness, &folder, &format!("two-filters-{suffix}"));

        harness.state_mut().go(Page::Packets {
            filter: String::new(),
            connections: String::new(),
            goto: Some(1_000),
        });
        shot(&mut harness, &folder, &format!("goto-{suffix}"));

        harness.state_mut().go(Page::Packet(3));
        shot(&mut harness, &folder, &format!("packet-{suffix}"));

        harness.state_mut().go(Page::Stream(3));
        shot(&mut harness, &folder, &format!("stream-{suffix}"));

        let connection = busy
            .engine
            .lock()
            .info_traffic
            .map
            .keys()
            .next()
            .unwrap()
            .clone();
        let pages = [
            ("overview", Page::Overview),
            (
                "connections",
                Page::Connections {
                    filter: String::new(),
                },
            ),
            ("connection", Page::Connection(connection)),
            ("hosts", Page::Entries(Favorite::Host)),
            ("services", Page::Entries(Favorite::Service)),
            ("programs", Page::Entries(Favorite::Program)),
            (
                "statistics",
                Page::Statistics {
                    filter: String::new(),
                    connections: String::new(),
                },
            ),
            ("notifications", Page::Notifications),
            ("settings", Page::Settings(SettingsPanel::Display)),
            ("licenses", Page::Settings(SettingsPanel::About)),
        ];
        for (name, page) in pages {
            harness.state_mut().go(page);
            shot(&mut harness, &folder, &format!("{name}-{suffix}"));
        }
    }
    let mut harness = window(&busy, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Packets {
        filter: String::new(),
        connections: String::new(),
        goto: None,
    });
    settle(&mut harness);
    harness.get_by_label("Hilfe zum Anzeigefilter").click();
    shot(&mut harness, &folder, "packets-help");

    // notifications of favorites (made up)
    {
        use crate::config::favorite::FavoriteItem;
        use crate::networking::types::data_info::DataInfo;
        use crate::notifications::types::logged_notification::LoggedNotification;
        let note = |favorite: FavoriteItem, time: &str| LoggedNotification {
            favorite,
            timestamp: format!("2026/11/10 {time}"),
        };
        let program = crate::networking::types::program::Program::NamePath((
            "thunderbird".to_owned(),
            "/usr/lib/thunderbird/thunderbird".to_owned(),
        ));
        let mut state = busy.engine.lock();
        state.logged_notifications.set_notifications(
            [
                note(
                    FavoriteItem::Program((program, DataInfo::new_for_tests(40, 0, 61_300, 0))),
                    "14:43:55",
                ),
                note(
                    FavoriteItem::Address((
                        "203.0.113.44".parse().unwrap(),
                        DataInfo::new_for_tests(20, 0, 23_800, 0),
                    )),
                    "14:46:12",
                ),
            ]
            .into(),
        );
    }
    let mut harness = window(&busy, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Notifications);
    shot(&mut harness, &folder, "notifications-list");
    busy.engine.lock().clear_notifications();

    // what lies below the first screen
    let mut harness = window_of_size(&busy, Theme::Light, Lang::De, 1600.0);
    harness.state_mut().go(Page::Overview);
    shot(&mut harness, &folder, "overview-tall");
    harness.state_mut().go(Page::Statistics {
        filter: String::new(),
        connections: String::new(),
    });
    shot(&mut harness, &folder, "statistics-tall");
    // a newer version with made-up release notes, as in the draft
    let release = |version: &str, notes: &str| crate::utils::check_updates::Release {
        version: version.to_owned(),
        published: "2026-11-07T10:00:00Z".parse().ok(),
        notes: notes.to_owned(),
        checked: jiff::Timestamp::now(),
    };
    busy.engine.lock().update_status =
        crate::utils::check_updates::UpdateStatus::UpdateAvailable(release(
            "9.9.0",
            "New interface in the style of opennetmon and opensecmon.\n\n### Changed\n\
             - Header bar instead of the sidebar; the recording stays visible at the top right\n\
             - Settings in one place with a sub-navigation\n\n### Added\n\
             - Keyboard shortcuts help (`?`), `/` jumps to the filter field\n\
             - Release notes in Settings → Updates\n\n### Installation\n\
             **Linux:** `sudo apt install ./openpacketmon_9.9.0-1_amd64.deb`. See the \
             [changelog](https://github.com/tomCSLK/openpacketmon/blob/main/CHANGELOG.md).",
        ));
    for panel in SettingsPanel::ALL {
        harness.state_mut().go(Page::Settings(panel));
        shot(
            &mut harness,
            &folder,
            &format!("settings-{panel:?}").to_lowercase(),
        );
    }
    busy.engine.lock().update_status =
        crate::utils::check_updates::UpdateStatus::UpToDate(release(
            VERSION,
            "### Highlights\n- Native interface (egui)\n- Packet list as in Wireshark",
        ));
    harness.state_mut().go(Page::Overview);
    harness
        .state_mut()
        .go(Page::Settings(SettingsPanel::Updates));
    shot(&mut harness, &folder, "settings-updates-current");
    busy.engine.lock().update_status = crate::utils::check_updates::UpdateStatus::Unknown;
    // the navigation of the settings below 1024 and below 900 px
    for width in [960.0, 760.0] {
        harness.set_size(egui::vec2(width, 900.0));
        harness.state_mut().go(Page::Settings(SettingsPanel::Geo));
        harness
            .state_mut()
            .go(Page::Settings(SettingsPanel::Notifications));
        shot(&mut harness, &folder, &format!("settings-narrow-{width}"));
    }

    // while assistants have access, and read packet contents
    let access = folder.join("mcp-access");
    busy.engine.lock().conf.mcp.enabled = true;
    for theme in [Theme::Light, Theme::Dark] {
        let mut harness = window_of_size(&busy, theme, Lang::De, 2_000.0);
        let mcp = crate::mcp::Access::new(busy.engine.clone(), Some(access.join("access.json")));
        mcp.apply(true);
        mcp.allow_content(true);
        harness.state_mut().mcp = mcp.clone();
        harness
            .state_mut()
            .go(Page::Settings(SettingsPanel::Assistants));
        let name = if theme == Theme::Light {
            "light"
        } else {
            "dark"
        };
        shot(&mut harness, &folder, &format!("settings-mcp-{name}"));
        mcp.allow_content(false);
        shot(&mut harness, &folder, &format!("settings-mcp-off-{name}"));
        mcp.apply(false);
    }
    busy.engine.lock().conf.mcp.enabled = false;
    remove_test_dir(&access);

    let mut harness = window(&busy, Theme::Dark, Lang::En);
    for (name, page) in [
        ("overview", Page::Overview),
        ("settings", Page::Settings(SettingsPanel::Display)),
        (
            "statistics",
            Page::Statistics {
                filter: String::new(),
                connections: String::new(),
            },
        ),
    ] {
        harness.state_mut().go(page);
        settle(&mut harness);
        shot(&mut harness, &folder, &format!("{name}-en"));
    }
}

/// The right edge of the toolbar of a filter in a window of 1280 points: the page ends 24 px
/// before the edge of the window, the card has its line and 12 px inside (`.toolbar`)
const TOOLBAR_RIGHT: f32 = 1_243.0;

/// However long the texts are, the toolbar of a filter ends where its card ends: with "Apply",
/// on the packet list with the field to go to a packet
#[test]
#[serial]
fn the_rows_of_the_filters_end_at_the_edge_of_the_card() {
    let packets = |filter: &str, connections: &str| Page::Packets {
        filter: filter.to_owned(),
        connections: connections.to_owned(),
        goto: None,
    };
    let statistics = |filter: &str, connections: &str| Page::Statistics {
        filter: filter.to_owned(),
        connections: connections.to_owned(),
    };
    let connections = |filter: &str| Page::Connections {
        filter: filter.to_owned(),
    };
    let fixture = engine_with(200);
    for (lang, apply, goto) in [
        (Lang::De, "Anwenden", "Gehe zu Paket"),
        (Lang::En, "Apply", "Go to packet"),
    ] {
        let mut harness = window(&fixture, Theme::Light, lang);
        for (page, last) in [
            (packets("", ""), goto),
            (packets("tcp", ""), goto),
            (packets("tcp", "port == 443"), goto),
            (statistics("", ""), apply),
            (statistics("tcp", ""), apply),
            (statistics("tcp", "port == 443"), apply),
            (connections(""), apply),
            (connections("port == 443"), apply),
        ] {
            let name = format!("{page:?}");
            harness.state_mut().go(page);
            settle(&mut harness);
            let mut right = harness.get_by_label(last).rect().right();
            if last == goto {
                // the label, 6 px, then the field of 92 px
                right += 6.0 + 92.0;
            }
            assert!(
                (right - TOOLBAR_RIGHT).abs() <= 1.0,
                "{last} on {name} ends at {right}",
            );
        }
        // the row does not move with the order chosen
        for sort in 0..3 {
            harness.state_mut().pages.connections.set_sort(sort);
            settle(&mut harness);
            let right = harness.get_by_label(apply).rect().right();
            assert!(
                (right - TOOLBAR_RIGHT).abs() <= 1.0,
                "{apply} with order {sort} ends at {right}",
            );
        }
    }
}

/// Each card of the overview has its own clicks: a later card must not undo those of an earlier one
#[test]
#[serial]
fn every_top_list_of_the_overview_opens_its_entries() {
    let fixture = engine_with(200);
    wait_for("the capture has no hosts", || {
        !crate::view::overview::overview(Lang::En, &fixture.engine)
            .hosts
            .is_empty()
    });
    let entries = {
        let view = crate::view::overview::overview(Lang::En, &fixture.engine);
        [&view.hosts, &view.services].map(|rows| (rows[0].label.clone(), rows[0].jump.clone()))
    };
    let mut harness = window_of_size(&fixture, Theme::Light, Lang::En, 1_400.0);
    for (label, jump) in entries {
        harness.state_mut().go(Page::Overview);
        settle(&mut harness);
        harness.get_by_label(&label).click();
        harness.run_steps(3);
        assert_eq!(harness.state().page, Page::from(jump), "{label}");
    }
}

/// What the last frame put on the clipboard
fn copied(harness: &Harness<'static, App>) -> Option<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| match c {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

#[test]
#[serial]
fn the_details_of_a_packet_can_be_copied() {
    let fixture = engine_with(200);
    // tall enough to have the fields of the transport layer in view
    let mut harness = window_of_size(&fixture, Theme::Light, Lang::En, 1_400.0);
    harness.state_mut().go(Page::Packet(3));
    settle(&mut harness);

    // nothing chosen, nothing copied
    harness.event(egui::Event::Copy);
    harness.step();
    assert_eq!(copied(&harness), None);

    let line = "Destination port: 443";
    harness.get_by_label(line).click();
    harness.run_steps(2);
    harness.event(egui::Event::Copy);
    harness.step();
    assert_eq!(copied(&harness).as_deref(), Some(line));

    harness.get_by_label(line).click_secondary();
    harness.run_steps(2);
    harness.get_by_label("Copy value").click();
    harness.step();
    assert_eq!(copied(&harness).as_deref(), Some("443"));

    harness.get_by_label(line).click_secondary();
    harness.run_steps(2);
    harness.get_by_label("Copy bytes as hex").click();
    harness.step();
    assert_eq!(copied(&harness).as_deref(), Some("01 bb"));

    harness.get_by_label(line).click_secondary();
    harness.run_steps(2);
    harness.get_by_label("Copy everything").click();
    harness.step();
    let all = copied(&harness).unwrap();
    assert!(all.contains("  Destination port: 443"), "{all}");

    // the first line of the hex dump: its bytes have no label to find them by
    let at = harness.get_by_label("Bytes").rect().left_bottom() + egui::vec2(90.0, 24.0);
    let right_click = |harness: &mut Harness<'static, App>| {
        harness.event(egui::Event::PointerMoved(at));
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        harness.run_steps(3);
    };
    right_click(&mut harness);
    harness
        .get_by_label("Copy whole packet as hex dump")
        .click();
    harness.step();
    let dump = copied(&harness).unwrap();
    assert!(dump.starts_with("0000  "), "{dump}");
    assert!(dump.lines().count() > 3, "{dump}");

    right_click(&mut harness);
    harness.get_by_label("Copy selection as hex").click();
    harness.step();
    let hex = copied(&harness).unwrap();
    assert!(
        !hex.is_empty() && hex.split(' ').all(|byte| byte.len() == 2),
        "{hex}"
    );
}

#[test]
#[serial]
fn the_filter_field_suggests_and_applies() {
    let fixture = engine_with(200);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Packets {
        filter: String::new(),
        connections: String::new(),
        goto: None,
    });
    settle(&mut harness);
    let inputs = || egui::accesskit::Role::TextInput;
    harness.get_all_by_role(inputs()).next().unwrap().click();
    harness.run_steps(2);
    harness
        .get_all_by_role(inputs())
        .next()
        .unwrap()
        .type_text("tc");
    settle(&mut harness);
    // the arrow chooses a line, Enter takes it into the field
    harness.key_press(egui::Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(harness.state().pages.packets.typed(), "tcp ");
    assert!(matches!(
        &harness.state().page,
        Page::Packets { filter, .. } if filter.is_empty()
    ));
    // Enter without a chosen line applies the filter
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert!(matches!(
        &harness.state().page,
        Page::Packets { filter, .. } if filter == "tcp"
    ));
    settle(&mut harness);
    assert_eq!(harness.state().pages.packets.total(), 100);
}

#[test]
#[serial]
fn the_keys_move_the_packet_list() {
    let fixture = engine_with(2_000);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Packets {
        filter: String::new(),
        connections: String::new(),
        goto: None,
    });
    settle(&mut harness);
    // a file that has been read starts at its first packet
    assert_eq!(row_at_top(&harness), 0);

    // the arrows select a packet, Enter opens it (`keydown` of the draft)
    harness.key_press(egui::Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(harness.state().pages.packets.selected(), Some(1));
    harness.key_press(egui::Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(harness.state().pages.packets.selected(), Some(2));
    let _ = harness.get_by_label("Paket 2");
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Packet(2));
    harness.state_mut().back();
    settle(&mut harness);
    assert_eq!(harness.state().pages.packets.selected(), Some(2));

    harness.key_press(egui::Key::PageDown);
    settle(&mut harness);
    let page = harness.state().pages.packets.visible() - 1;
    assert_eq!(row_at_top(&harness), page);

    // End goes to the newest packets and leaves the selection
    harness.key_press(egui::Key::End);
    settle(&mut harness);
    let top = row_at_top(&harness);
    // without the bar of the selected packet the list is longer
    let visible = harness.state().pages.packets.visible();
    assert!(
        top + visible >= 2_000,
        "row {top} at the top, {visible} in view"
    );
    assert_eq!(harness.state().pages.packets.selected(), None);

    // back from the details, the list is where it was
    harness.state_mut().go(Page::Packet(1_990));
    settle(&mut harness);
    harness.state_mut().back();
    settle(&mut harness);
    assert_eq!(row_at_top(&harness), top);

    harness.key_press(egui::Key::Home);
    settle(&mut harness);
    assert_eq!(row_at_top(&harness), 0);
}

#[test]
#[serial]
fn the_details_page_through_the_packets_of_the_list() {
    let fixture = engine_with(200);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Packets {
        filter: "frame.number in {3 7 9}".to_owned(),
        connections: String::new(),
        goto: None,
    });
    settle(&mut harness);
    harness.state_mut().go(Page::Packet(7));
    settle(&mut harness);
    // the arrows and the buttons go to the neighbors in the list, not in the capture
    harness.key_press(egui::Key::ArrowRight);
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Packet(9));
    harness.key_press(egui::Key::ArrowRight);
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Packet(9));
    harness.get_by_label("Vorheriges").click();
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Packet(7));
    harness.key_press(egui::Key::ArrowLeft);
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Packet(3));

    // the crumb goes back to the list, which shows the packet last seen
    let crumb = harness
        .get_all_by_label("Pakete")
        .find(|node| (60.0..95.0).contains(&node.rect().top()))
        .unwrap();
    crumb.click();
    settle(&mut harness);
    assert!(matches!(
        &harness.state().page,
        Page::Packets { goto: Some(3), .. }
    ));
    assert_eq!(harness.state().pages.packets.selected(), Some(3));
    // it went back, it did not open the list once more
    assert_eq!(harness.state().history.len(), 1);
}

#[test]
#[serial]
fn going_to_a_packet_from_a_shorter_list_stays_there() {
    let fixture = engine_with(2_000);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Packets {
        filter: "tcp and ip.addr == 192.168.1.20".to_owned(),
        connections: String::new(),
        goto: None,
    });
    settle(&mut harness);
    harness.state_mut().go(Page::Packets {
        filter: String::new(),
        connections: String::new(),
        goto: Some(1_000),
    });
    settle(&mut harness);
    settle(&mut harness);
    let top = row_at_top(&harness);
    assert!((900..1_000).contains(&top), "row {top} at the top");
}

/// The scroll position of the list, in rows
fn row_at_top(harness: &Harness<'static, App>) -> usize {
    harness.state().pages.packets.first_row()
}

#[test]
#[serial]
fn going_to_a_packet_finds_it_and_stops_at_the_end_of_the_list() {
    let fixture = engine_with(2_000);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    let packets = |goto| Page::Packets {
        filter: String::new(),
        connections: String::new(),
        goto,
    };
    harness.state_mut().go(packets(None));
    settle(&mut harness);
    assert_eq!(harness.state().pages.packets.total(), 2_000);

    harness.state_mut().go(packets(Some(1_000)));
    settle(&mut harness);
    settle(&mut harness);
    let top = row_at_top(&harness);
    assert!((900..1_000).contains(&top), "row {top} at the top");
    assert_eq!(harness.state().pages.packets.selected(), Some(1_000));

    // the last packet: never past the end, that would show an empty area
    harness.state_mut().go(packets(Some(2_000)));
    settle(&mut harness);
    settle(&mut harness);
    let top = row_at_top(&harness);
    let visible = harness.state().pages.packets.visible();
    assert!(top + visible >= 2_000, "row {top}, {visible} in view");
    assert!(top + visible <= 2_002, "row {top}, {visible} in view");
    assert_eq!(harness.state().pages.packets.selected(), Some(2_000));
}

#[test]
#[serial]
fn the_way_back_retraces_the_steps() {
    let fixture = engine_with(0);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Capture);
    harness.get_by_label("Einstellungen").click();
    harness.run_steps(2);
    assert_eq!(harness.state().page, Page::Settings(SettingsPanel::Display));
    harness.state_mut().back();
    assert_eq!(harness.state().page, Page::Capture);
    assert!(!harness.state().can_go_back());
}

/// The start button of the top bar, above the one at the end of the capture page
fn top_start_button<'a>(harness: &'a Harness<'static, App>) -> egui_kittest::Node<'a> {
    harness
        .get_all_by_label("Aufzeichnung starten")
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .unwrap()
}

#[test]
#[serial]
fn the_start_button_at_the_top_starts_on_the_capture_page() {
    // elsewhere it only opens the capture page
    let fixture = engine_with(0);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Overview);
    settle(&mut harness);
    top_start_button(&harness).click();
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Capture);
    assert!(!fixture.engine.lock().is_running());

    // on the capture page it starts with the choices there, here a capture file
    let path = sample_file(5);
    {
        let mut state = fixture.engine.lock();
        state.set_capture_source(CaptureSourcePicklist::File);
        state.set_pcap_import(path.to_string_lossy().to_string());
    }
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    settle(&mut harness);
    assert_eq!(harness.state().page, Page::Capture);
    assert_eq!(harness.get_all_by_label("Aufzeichnung starten").count(), 2);
    top_start_button(&harness).click();
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(50));
        settle(&mut harness);
        if harness.state().page == Page::Overview {
            break;
        }
    }
    assert_eq!(harness.state().page, Page::Overview);
    assert!(fixture.engine.lock().is_running());
    if let Some(dir) = path.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

fn text_files(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            text_files(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// A character without a glyph is drawn as an empty box
#[test]
fn the_fonts_have_every_character_of_the_texts() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .textures_delta
        .clear();
    let mut files = Vec::new();
    for folder in [
        "src/view",
        "src/gui",
        "src/dissect",
        "src/filter",
        "locales",
    ] {
        text_files(Path::new(folder), &mut files);
    }
    let mut missing = std::collections::BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
            for c in line.chars().filter(|c| !c.is_ascii()) {
                for font in [
                    egui::FontId::proportional(14.0),
                    egui::FontId::monospace(13.0),
                ] {
                    if !ctx.fonts_mut(|f| f.has_glyph(&font, c)) {
                        missing.insert(format!("{c} {:?} {}", font.family, file.display()));
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}

fn request_close(harness: &mut Harness<'static, App>) {
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    harness.run_steps(2);
}

#[test]
#[serial]
fn closing_asks_only_while_a_capture_runs() {
    let fixture = engine_with(0);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    settle(&mut harness);
    request_close(&mut harness);
    assert!(harness.query_by_label("Aufzeichnung läuft").is_none());

    // the next reading of the state of the capture takes this back
    for _ in 0..50 {
        harness.state_mut().live.capturing = true;
        request_close(&mut harness);
        if harness.query_by_label("Aufzeichnung läuft").is_some() {
            break;
        }
    }
    assert!(harness.query_by_label("Aufzeichnung läuft").is_some());
    harness.get_by_label("Abbrechen").click();
    harness.run_steps(2);
    assert!(harness.query_by_label("Aufzeichnung läuft").is_none());
}

#[test]
#[serial]
fn a_capture_that_ends_with_an_error_says_so_on_every_page() {
    let fixture = engine_with(0);
    let mut harness = window(&fixture, Theme::Light, Lang::De);
    harness.state_mut().go(Page::Overview);
    settle(&mut harness);
    {
        let mut state = fixture.engine.lock();
        let capture = state.capture_id();
        state.handle_backend_message(BackendTrafficMessage::CaptureStopped(
            capture,
            "helper gone".to_owned(),
        ));
    }
    // the state of the capture is read every 300 ms; under load it may take longer
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        settle(&mut harness);
        if harness.query_by_label_contains("helper gone").is_some() {
            break;
        }
    }
    assert!(harness.query_by_label_contains("helper gone").is_some());
}

#[cfg(windows)]
#[test]
fn windows_names_the_languages_of_the_user() {
    // the list Windows keeps under Settings > Time & language, never empty
    let languages = super::windows_languages().expect("no display language");
    assert!(
        languages.split(',').all(|tag| tag.len() >= 2),
        "{languages}"
    );
}
