//! Runs the helper on a real adapter: the loopback adapter of a private network namespace, where
//! it can capture without any rights being set up. Where namespaces are not available (some CI
//! runners), this test only says so. Under Windows the loopback adapter of Npcap, if installed.

#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::fs;
use std::io::{BufReader, Write};
use std::net::UdpSocket;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use openpacketmon_capture::{Frame, read_frame};

const HELPER: &str = env!("CARGO_BIN_EXE_openpacketmon-capture");
const PORT: u16 = 7391;
const OTHER_PORT: u16 = 7392;

#[test]
fn captures_in_a_namespace() {
    let usable = Command::new("unshare")
        .args(["-Urn", "ip", "link", "set", "lo", "up"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !usable {
        eprintln!("skipped: no private network namespace here");
        return;
    }
    let status = Command::new("unshare")
        .args([
            "-Urn",
            "sh",
            "-c",
            r#"ip link set lo up && exec "$@""#,
            "sh",
        ])
        .arg(std::env::current_exe().unwrap())
        .args(["--ignored", "--nocapture"])
        .status()
        .unwrap();
    assert!(status.success());
}

struct Helper {
    child: Child,
    commands: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    data: Vec<u8>,
}

impl Helper {
    fn start(device: &str, bpf: &str) -> Self {
        let mut child = Command::new(HELPER)
            .args([
                "--device",
                device,
                "--snaplen",
                "65535",
                "--buffer",
                "2000000",
            ])
            .args(["--bpf", bpf])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        Self {
            commands: child.stdin.take(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
            data: Vec::new(),
        }
    }

    fn next(&mut self) -> Frame {
        read_frame(&mut self.output, &mut self.data).unwrap()
    }

    fn command(&mut self, line: &str) {
        writeln!(self.commands.as_mut().unwrap(), "{line}").unwrap();
        // the command takes effect between two reads of the adapter
        self.wait_for_timeouts(2);
    }

    fn wait_for_timeouts(&mut self, mut count: u32) -> bool {
        let mut seen = false;
        while count > 0 {
            match self.next() {
                Frame::Timeout => count -= 1,
                Frame::Packet { .. } => seen = true,
                _ => {}
            }
        }
        seen
    }

    /// Reads until the helper was idle a few times; true if `payload` went by in a packet
    fn saw(&mut self, payload: &[u8]) -> bool {
        let mut seen = false;
        let mut idle = 0;
        while idle < 4 {
            match self.next() {
                Frame::Timeout => idle += 1,
                Frame::Packet { .. } => seen |= self.data.ends_with(payload),
                _ => {}
            }
        }
        seen
    }

    fn status(&self, name: &str) -> String {
        let status = fs::read_to_string(format!("/proc/{}/status", self.child.id())).unwrap();
        let line = status.lines().find(|l| l.starts_with(name)).unwrap();
        line[name.len()..].trim().to_string()
    }
}

#[test]
#[ignore = "started by captures_in_a_namespace"]
fn inside_the_namespace() {
    let receiver = UdpSocket::bind(("127.0.0.1", PORT)).unwrap();
    let other_receiver = UdpSocket::bind(("127.0.0.1", OTHER_PORT)).unwrap();

    let mut helper = Helper::start("lo", &format!("udp port {PORT}"));
    assert_eq!(helper.next(), Frame::Ready { link_type: 1 });

    // the rights are gone before the first packet, for good
    assert_eq!(helper.status("CapEff:"), "0000000000000000");
    assert_eq!(helper.status("CapPrm:"), "0000000000000000");
    assert_eq!(helper.status("CapInh:"), "0000000000000000");
    assert_eq!(helper.status("NoNewPrivs:"), "1");

    exercise(helper, &receiver, &other_receiver);
}

/// Under Windows on the loopback adapter of Npcap, which needs no rights of its own
#[cfg(windows)]
#[test]
fn captures_on_the_npcap_loopback_adapter() {
    let system = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    if !system.join("wpcap.dll").is_file() {
        eprintln!("skipped: Npcap is not installed");
        return;
    }
    let receiver = UdpSocket::bind(("127.0.0.1", PORT)).unwrap();
    let other_receiver = UdpSocket::bind(("127.0.0.1", OTHER_PORT)).unwrap();

    let mut helper = Helper::start(r"\Device\NPF_Loopback", &format!("udp port {PORT}"));
    // under Npcap a blocking read without packets might never return; then this test would hang
    let done = watchdog(helper.child.id());
    match helper.next() {
        Frame::Ready { link_type } => eprintln!("loopback adapter, link type {link_type}"),
        Frame::Failed => {
            eprintln!(
                "skipped: no loopback adapter ({})",
                String::from_utf8_lossy(&helper.data)
            );
            done.store(true, std::sync::atomic::Ordering::Relaxed);
            return;
        }
        other => panic!("unexpected {other:?}"),
    }
    exercise(helper, &receiver, &other_receiver);

    let mut unknown = Helper::start("no-such-adapter", "");
    assert_eq!(unknown.next(), Frame::Failed);
    assert!(String::from_utf8_lossy(&unknown.data).contains("no-such-adapter"));
    assert!(!unknown.child.wait().unwrap().success());
    done.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Ends the helper after a minute unless the test is done: a hanging read fails the test then
#[cfg(windows)]
fn watchdog(pid: u32) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = done.clone();
    std::thread::spawn(move || {
        sleep(Duration::from_secs(60));
        if !flag.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .status();
        }
    });
    done
}

/// Filter, pause, resume, statistics and the end with the standard input, on an opened adapter
fn exercise(mut helper: Helper, receiver: &UdpSocket, other_receiver: &UdpSocket) {
    assert_ne!(
        receiver.local_addr().unwrap(),
        other_receiver.local_addr().unwrap()
    );
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    let send = |payload: &[u8], port| {
        sender.send_to(payload, ("127.0.0.1", port)).unwrap();
    };

    send(b"seen-first", PORT);
    send(b"not-in-the-filter", OTHER_PORT);
    assert!(helper.saw(b"seen-first"));

    helper.command("pause");
    send(b"while-paused", PORT);
    assert!(!helper.saw(b"while-paused"));

    helper.command("resume");
    send(b"not-in-the-filter", OTHER_PORT);
    assert!(!helper.saw(b"not-in-the-filter"), "resume lost the filter");
    send(b"seen-again", PORT);
    assert!(helper.saw(b"seen-again"));

    let mut stats = None;
    for _ in 0..4 {
        if let Frame::Stats(s) = helper.next() {
            stats = Some(s);
        }
    }
    assert!(stats.unwrap().received > 0);

    // a closed standard input ends the helper, so it cannot outlive openpacketmon
    drop(helper.commands.take());
    let started = Instant::now();
    loop {
        if let Some(status) = helper.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(2), "still running");
        sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "started by captures_in_a_namespace"]
fn refuses_an_unknown_adapter() {
    let mut helper = Helper::start("no-such-adapter", "");
    assert_eq!(helper.next(), Frame::Failed);
    assert!(String::from_utf8_lossy(&helper.data).contains("no-such-adapter"));
    assert!(!helper.child.wait().unwrap().success());
}

#[test]
#[ignore = "started by captures_in_a_namespace"]
fn refuses_a_broken_filter_after_giving_up_the_rights() {
    // the filter is compiled without rights, and a broken one still stops the helper
    let mut helper = Helper::start("lo", "udp port");
    assert_eq!(helper.next(), Frame::Failed);
    assert!(!helper.data.is_empty());
    assert!(!helper.child.wait().unwrap().success());
}
