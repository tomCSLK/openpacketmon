// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/capture_context.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

//! Capture helper of openpacketmon, the only part of it with the rights to capture packets.
//!
//! It opens one adapter, gives up its rights and writes what it reads to standard output (see the
//! library of this crate). It does not look into the packets, keeps no files and ends when
//! standard input closes. Keep it small: everything added here runs with these rights.

use std::io::{BufRead, BufWriter, Write, stdin, stdout};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use openpacketmon_capture::{
    Stats, TIMEOUT_MS, write_failed, write_packet, write_ready, write_stats, write_timeout,
};
use pcap::{Active, Capture, Device, Error};

/// Largest kernel buffer accepted on the command line
const MAX_BUFFER_BYTES: i32 = 64 * 1024 * 1024;
/// Longest BPF filter accepted on the command line
const MAX_BPF_BYTES: usize = 4096;
/// How often the counters are reported while packets keep arriving
const STATS_EVERY: Duration = Duration::from_millis(500);
/// Bytes kept in the pipe to openpacketmon, to survive short delays of the reader
#[cfg(target_os = "linux")]
const PIPE_BYTES: usize = 1024 * 1024;

/// Set by the thread reading standard input, applied by the capture loop
static PAUSED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Eq, PartialEq)]
struct Options {
    device: String,
    snaplen: i32,
    buffer: i32,
    bpf: String,
    /// Asked for promiscuous mode; the adapter may refuse it
    promisc: bool,
}

impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let (mut device, mut snaplen, mut buffer, mut bpf) = (None, None, None, String::new());
        let mut promisc = false;
        while let Some(name) = args.next() {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            match name.as_str() {
                "--device" => device = Some(value),
                "--snaplen" => snaplen = Some(number(&value, 65_535)?),
                "--buffer" => buffer = Some(number(&value, MAX_BUFFER_BYTES)?),
                "--bpf" if value.len() <= MAX_BPF_BYTES => bpf = value,
                "--bpf" => return Err("the BPF filter is too long".to_string()),
                "--promisc" => {
                    promisc = match value.as_str() {
                        "on" => true,
                        "off" => false,
                        _ => return Err(format!("--promisc is on or off, not {value}")),
                    }
                }
                _ => return Err(format!("unknown option {name}")),
            }
        }
        Ok(Self {
            device: device.ok_or("--device is missing")?,
            snaplen: snaplen.ok_or("--snaplen is missing")?,
            buffer: buffer.ok_or("--buffer is missing")?,
            bpf,
            promisc,
        })
    }
}

fn number(value: &str, max: i32) -> Result<i32, String> {
    match value.parse() {
        Ok(n) if (1..=max).contains(&n) => Ok(n),
        _ => Err(format!("{value} is not a number from 1 to {max}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            let mut out = stdout();
            let _ = write_failed(&mut out, &reason);
            let _ = out.flush();
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args().skip(1))?;
    let (mut cap, promiscuous) = open(&options)?;
    // nothing below needs the rights any more: the open socket keeps working without them
    drop_rights()?;
    // the filter is compiled from text only now, so a flaw in the BPF compiler of libpcap gets no rights
    if !options.bpf.is_empty() {
        cap.filter(&options.bpf, true).map_err(|e| e.to_string())?;
    }
    enlarge_pipe();
    let link_type = cap.get_datalink().0;
    let mut out = BufWriter::with_capacity(64 * 1024, stdout().lock());
    if write_ready(&mut out, link_type, promiscuous)
        .and_then(|()| out.flush())
        .is_err()
    {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("commands".to_string())
        .spawn(read_commands)
        .map_err(|e| e.to_string())?;
    capture(&mut cap, &options.bpf, &mut out)
}

/// Opens the adapter with the settings of openpacketmon (without the filter, which comes after the
/// rights are gone); only an existing adapter is accepted. Tells whether it is in promiscuous mode:
/// an adapter that refuses it (some Wi-Fi drivers under Npcap, the `any` device of Linux) is opened
/// without it rather than not at all.
fn open(options: &Options) -> Result<(Capture<Active>, bool), String> {
    let device = Device::list()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|d| d.name == options.device)
        .ok_or_else(|| format!("unknown adapter {}", options.device))?;
    if options.promisc
        && let Ok(cap) = open_device(device.clone(), options, true)
    {
        return Ok((cap, true));
    }
    Ok((open_device(device, options, false)?, false))
}

/// `pcap` takes the warnings of libpcap for errors, also "promiscuous mode not supported"
fn open_device(
    device: Device,
    options: &Options,
    promisc: bool,
) -> Result<Capture<Active>, String> {
    let cap = Capture::from_device(device)
        .map_err(|e| e.to_string())?
        .promisc(promisc)
        .buffer_size(options.buffer)
        .snaplen(options.snaplen)
        .immediate_mode(false)
        .timeout(TIMEOUT_MS)
        .open()
        .map_err(|e| e.to_string())?;
    // with no matching packets a blocking read may never return: libpcap leaves the timeout to the
    // kernel, which does not hand over an empty block. A paused capture would stay paused for good.
    #[cfg(target_os = "linux")]
    let cap = cap.setnonblock().map_err(|e| e.to_string())?;
    Ok(cap)
}

/// Gives up all capabilities for good; called while this is the only thread
#[cfg(target_os = "linux")]
fn drop_rights() -> Result<(), String> {
    use rustix::thread::{CapabilitySet, CapabilitySets, set_capabilities, set_no_new_privs};

    let reason = |e: rustix::io::Errno| format!("could not give up the capture rights: {e}");
    set_no_new_privs(true).map_err(reason)?;
    let none = CapabilitySet::empty();
    set_capabilities(
        None,
        CapabilitySets {
            effective: none,
            permitted: none,
            inheritable: none,
        },
    )
    .map_err(reason)
}

/// Other systems grant the capture through their driver, there is nothing to give up
#[cfg(not(target_os = "linux"))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the same signature as under Linux, where giving up the rights can fail"
)]
fn drop_rights() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn enlarge_pipe() {
    // only helps with bursts; the default size also works, and a file as output has none
    let _ = rustix::pipe::fcntl_setpipe_size(stdout(), PIPE_BYTES);
}

#[cfg(not(target_os = "linux"))]
fn enlarge_pipe() {}

/// Follows the commands until standard input closes, which ends the helper
fn read_commands() {
    for line in stdin().lock().lines() {
        match line.as_deref() {
            Ok("pause") => PAUSED.store(true, Ordering::Relaxed),
            Ok("resume") => PAUSED.store(false, Ordering::Relaxed),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    std::process::exit(0);
}

fn capture(cap: &mut Capture<Active>, bpf: &str, out: &mut impl Write) -> Result<(), String> {
    let mut paused = false;
    let mut reported = Instant::now();
    loop {
        let wanted = PAUSED.load(Ordering::Relaxed);
        if wanted != paused {
            paused = wanted;
            apply_filter(cap, paused, bpf);
        }

        let sent = match cap.next_packet() {
            Ok(packet) => {
                #[allow(clippy::useless_conversion)]
                let (secs, usecs) = (
                    i64::from(packet.header.ts.tv_sec),
                    i64::from(packet.header.ts.tv_usec),
                );
                write_packet(out, secs, usecs, packet.header.len, packet.data)
            }
            Err(Error::TimeoutExpired) => idle(cap, out, &mut reported),
            Err(e) if nothing_yet(&e) => idle(cap, out, &mut reported),
            Err(e) => return Err(e.to_string()),
        };
        // openpacketmon went away: there is nobody left to report to
        if sent.is_err() {
            return Ok(());
        }
        if reported.elapsed() >= STATS_EVERY {
            reported = Instant::now();
            if report(cap, out).and_then(|()| out.flush()).is_err() {
                return Ok(());
            }
        }
    }
}

/// Everything that was waiting has been read: hands it on and waits for more
fn idle(
    cap: &mut Capture<Active>,
    out: &mut impl Write,
    reported: &mut Instant,
) -> std::io::Result<()> {
    out.flush()?;
    if wait_for_packets(cap) {
        return Ok(());
    }
    *reported = Instant::now();
    report(cap, out)?;
    write_timeout(out)?;
    out.flush()
}

/// Sources that are not network adapters, such as `nfqueue`, turn the "nothing yet" of a
/// non-blocking read into an error, which libpcap only hands over as text
fn nothing_yet(error: &Error) -> bool {
    matches!(error, Error::PcapError(text) if text.ends_with("Resource temporarily unavailable"))
}

/// Waits for packets, at most [`TIMEOUT_MS`], and tells if there are any
#[cfg(target_os = "linux")]
fn wait_for_packets(cap: &Capture<Active>) -> bool {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};

    let timeout = Timespec {
        tv_sec: 0,
        tv_nsec: i64::from(TIMEOUT_MS) * 1_000_000,
    };
    let mut adapter = [PollFd::new(cap, PollFlags::IN)];
    // an interrupted wait counts as nothing arrived, the capture loop just comes back here
    poll(&mut adapter, Some(&timeout)).is_ok_and(|ready| ready > 0)
}

/// Elsewhere the read itself waits for [`TIMEOUT_MS`]
#[cfg(not(target_os = "linux"))]
fn wait_for_packets(_cap: &Capture<Active>) -> bool {
    false
}

fn report(cap: &mut Capture<Active>, out: &mut impl Write) -> std::io::Result<()> {
    match cap.stats() {
        Ok(s) => write_stats(
            out,
            Stats {
                received: s.received,
                dropped: s.dropped,
                if_dropped: s.if_dropped,
            },
        ),
        Err(_) => Ok(()),
    }
}

/// While paused, a filter that matches nothing keeps the packets out of the capture
fn apply_filter(cap: &mut Capture<Active>, paused: bool, bpf: &str) {
    let done = if paused {
        cap.filter("less 2", true)
    } else if bpf.is_empty() {
        cap.filter("", true)
            .or_else(|_| cap.filter("greater 0", true))
    } else {
        cap.filter(bpf, true)
    };
    // a failed change keeps the previous filter, and the capture goes on
    drop(done);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{MAX_BPF_BYTES, Options, nothing_yet};
    use pcap::Error;

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn accepts_what_openpacketmon_sends() {
        let options = parse(&[
            "--device",
            "wlp2s0",
            "--snaplen",
            "200",
            "--buffer",
            "2000000",
            "--bpf",
            "tcp",
        ])
        .unwrap();
        assert_eq!(
            options,
            Options {
                device: "wlp2s0".to_string(),
                snaplen: 200,
                buffer: 2_000_000,
                bpf: "tcp".to_string(),
                promisc: false,
            }
        );
        let promisc = ["--device", "eth0", "--snaplen", "1", "--buffer", "1"];
        let on = parse(&[&promisc[..], &["--promisc", "on"]].concat()).unwrap();
        assert!(on.promisc);
        let off = parse(&[&promisc[..], &["--promisc", "off"]].concat()).unwrap();
        assert!(!off.promisc);
    }

    #[test]
    fn an_empty_source_is_not_a_failure() {
        let empty = "Can't receive packet: Resource temporarily unavailable";
        assert!(nothing_yet(&Error::PcapError(empty.to_string())));
        assert!(!nothing_yet(&Error::PcapError(
            "The interface went down".to_string()
        )));
    }

    #[test]
    fn the_filter_is_optional() {
        let args = ["--device", "lo", "--snaplen", "1", "--buffer", "1"];
        let options = parse(&args).unwrap();
        assert_eq!(options.bpf, "");
        // without the option the adapter is not put into promiscuous mode
        assert!(!options.promisc);
    }

    #[test]
    fn refuses_anything_else() {
        for args in [
            &["--device", "lo", "--snaplen", "200"][..],
            &["--device", "lo", "--snaplen", "0", "--buffer", "1"],
            &["--device", "lo", "--snaplen", "70000", "--buffer", "1"],
            &["--device", "lo", "--snaplen", "1", "--buffer", "-1"],
            &["--device", "lo", "--snaplen", "1", "--buffer", "x"],
            &[
                "--device",
                "lo",
                "--snaplen",
                "1",
                "--buffer",
                "1",
                "--out",
                "/etc/passwd",
            ],
            &["--device"],
            &[
                "--device",
                "lo",
                "--snaplen",
                "1",
                "--buffer",
                "1",
                "--promisc",
                "yes",
            ],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
        let long = "x".repeat(MAX_BPF_BYTES + 1);
        let args = [
            "--device",
            "lo",
            "--snaplen",
            "1",
            "--buffer",
            "1",
            "--bpf",
            &long,
        ];
        assert!(parse(&args).is_err());
    }
}
