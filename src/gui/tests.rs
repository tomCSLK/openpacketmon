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
use crate::engine::tests::{frame, write_pcap};
use crate::networking::capture::BackendTrafficMessage;
use crate::networking::types::capture_context::CaptureSourcePicklist;

struct Fixture {
    // the jobs of the engine run here
    _runtime: tokio::runtime::Runtime,
    engine: Engine,
}

fn sample_file(rows: usize) -> PathBuf {
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
    let dir = std::env::temp_dir().join(format!("openpacketmon-gui-{}", std::process::id()));
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
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let engine = Engine::new(Conf::default(), runtime.handle().clone());
    if rows > 0 {
        let path = sample_file(rows);
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
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, height))
        .with_theme(theme)
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = App::new(&cc.egui_ctx, engine);
            app.t = lang;
            // so that the warnings of egui fail the test: their icon is in no font
            cc.egui_ctx
                .set_missing_glyph_policy(egui::MissingGlyphPolicy::Panic);
            app
        });
    harness.state_mut().pages = pages::Pages::default();
    harness
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
    settle(harness);
    let image = harness.render().unwrap();
    image.save(folder.join(format!("{name}.png"))).unwrap();
}

#[test]
#[serial]
#[ignore = "writes pictures for a person to look at"]
fn shots() {
    let folder = PathBuf::from(std::env::var("GUI_SHOTS").unwrap());
    std::fs::create_dir_all(&folder).unwrap();
    let idle = engine_with(0);
    let busy = engine_with(2_000);
    for (theme, suffix) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut harness = window(&idle, theme, Lang::De);
        shot(&mut harness, &folder, &format!("capture-{suffix}"));

        let mut harness = window(&busy, theme, Lang::De);
        harness.state_mut().page = Page::Capture;
        shot(&mut harness, &folder, &format!("running-{suffix}"));

        harness.state_mut().go(Page::Packets {
            filter: String::new(),
            connections: String::new(),
            goto: None,
        });
        shot(&mut harness, &folder, &format!("packets-{suffix}"));
        settle(&mut harness);
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
            ("settings", Page::Settings),
            ("licenses", Page::Licenses),
        ];
        for (name, page) in pages {
            harness.state_mut().go(page);
            settle(&mut harness);
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

    // what lies below the first screen
    let mut harness = window_of_size(&busy, Theme::Light, Lang::De, 1600.0);
    harness.state_mut().go(Page::Statistics {
        filter: String::new(),
        connections: String::new(),
    });
    shot(&mut harness, &folder, "statistics-tall");
    harness.state_mut().go(Page::Settings);
    shot(&mut harness, &folder, "settings-tall");

    let mut harness = window(&busy, Theme::Dark, Lang::En);
    for (name, page) in [
        ("overview", Page::Overview),
        ("settings", Page::Settings),
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

/// The right edge of the content of a window of 1280 points
const CONTENT_RIGHT: f32 = 1_262.0;

/// However long the texts of the buttons are, the row of a filter ends where the page ends
#[test]
#[serial]
fn the_rows_of_the_filters_end_at_the_edge_of_the_page() {
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
    for (lang, apply, reset) in [
        (Lang::De, "Anwenden", "Zurücksetzen"),
        (Lang::En, "Apply", "Reset"),
    ] {
        let mut harness = window(&fixture, Theme::Light, lang);
        for (page, last) in [
            (packets("", ""), apply),
            (packets("tcp", ""), reset),
            (packets("tcp", "port == 443"), reset),
            (statistics("", ""), apply),
            (statistics("tcp", ""), reset),
            (statistics("tcp", "port == 443"), reset),
            (connections(""), apply),
            (connections("port == 443"), reset),
        ] {
            let name = format!("{page:?}");
            harness.state_mut().go(page);
            settle(&mut harness);
            let right = harness.get_by_label(last).rect().right();
            assert!(
                (right - CONTENT_RIGHT).abs() <= 1.0,
                "{last} on {name} ends at {right}",
            );
        }
        // the row does not move with the order chosen
        for sort in 0..3 {
            harness.state_mut().pages.connections.set_sort(sort);
            settle(&mut harness);
            let right = harness.get_by_label(reset).rect().right();
            assert!(
                (right - CONTENT_RIGHT).abs() <= 1.0,
                "{reset} with order {sort} ends at {right}",
            );
        }
    }
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

    harness.key_press(egui::Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(row_at_top(&harness), 1);
    harness.key_press(egui::Key::PageDown);
    settle(&mut harness);
    let page = harness.state().pages.packets.visible() - 1;
    assert_eq!(row_at_top(&harness), 1 + page);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::End);
    settle(&mut harness);
    let top = row_at_top(&harness);
    assert!(top + page >= 1_999, "row {top} at the top");

    // back from the details, the list is where it was
    harness.state_mut().go(Page::Packet(1_990));
    settle(&mut harness);
    harness.state_mut().back();
    settle(&mut harness);
    assert_eq!(row_at_top(&harness), top);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Home);
    settle(&mut harness);
    assert_eq!(row_at_top(&harness), 0);
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
    assert_eq!(harness.state().pages.packets.target(), Some(1_000));

    // the last packet: never past the end, that would show an empty area
    harness.state_mut().go(packets(Some(2_000)));
    settle(&mut harness);
    settle(&mut harness);
    let top = row_at_top(&harness);
    let visible = harness.state().pages.packets.visible();
    assert!(top + visible >= 2_000, "row {top}, {visible} in view");
    assert!(top + visible <= 2_002, "row {top}, {visible} in view");
    assert_eq!(harness.state().pages.packets.target(), Some(2_000));
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
    assert_eq!(harness.state().page, Page::Settings);
    harness.state_mut().back();
    assert_eq!(harness.state().page, Page::Capture);
    assert!(!harness.state().can_go_back());
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
    std::thread::sleep(Duration::from_millis(350));
    settle(&mut harness);
    settle(&mut harness);
    assert!(harness.query_by_label_contains("helper gone").is_some());
}
