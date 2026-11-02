//! A live capture read from `openpacketmon-capture`, the only part of openpacketmon with the
//! rights to capture packets. Everything here runs without them.

use std::env;
use std::ffi::OsStr;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::Duration;

use openpacketmon_capture::{BINARY, Frame, TIMEOUT_MS, read_frame};
use pcap::{Error, Packet, PacketHeader, Stat};

/// Overrides where the helper is looked for
const HELPER_VARIABLE: &str = "OPENPACKETMON_CAPTURE_HELPER";

pub struct CaptureHelper {
    /// `None` only in tests
    child: Option<Child>,
    commands: Option<ChildStdin>,
    output: Box<dyn Read + Send>,
    link_type: i32,
    header: PacketHeader,
    data: Vec<u8>,
    stats: Stat,
    /// Why the helper stopped, once it has
    ended: Option<String>,
}

impl CaptureHelper {
    /// Starts the helper and waits until it has opened the adapter
    pub fn start(
        device: &str,
        snaplen: i32,
        buffer: i32,
        bpf: Option<&str>,
    ) -> Result<Self, String> {
        let path = helper_path();
        let mut command = Command::new(&path);
        command
            .arg("--device")
            .arg(device)
            .arg("--snaplen")
            .arg(snaplen.to_string())
            .arg("--buffer")
            .arg(buffer.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if let Some(bpf) = bpf {
            command.arg("--bpf").arg(bpf);
        }
        // the window has no console, so each helper would get a console window of its own
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let commands = child.stdin.take();
        let output = child.stdout.take().ok_or("the helper has no output")?;
        let mut helper = Self::new(Box::new(BufReader::with_capacity(64 * 1024, output)));
        helper.child = Some(child);
        helper.commands = commands;
        helper.wait_until_ready()?;
        Ok(helper)
    }

    fn new(output: Box<dyn Read + Send>) -> Self {
        Self {
            child: None,
            commands: None,
            output,
            link_type: 0,
            header: PacketHeader {
                ts: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                caplen: 0,
                len: 0,
            },
            data: Vec::new(),
            stats: Stat {
                received: 0,
                dropped: 0,
                if_dropped: 0,
            },
            ended: None,
        }
    }

    fn wait_until_ready(&mut self) -> Result<(), String> {
        match read_frame(&mut self.output, &mut self.data) {
            Ok(Frame::Ready { link_type }) => {
                self.link_type = link_type;
                Ok(())
            }
            Ok(Frame::Failed) => Err(String::from_utf8_lossy(&self.data).into_owned()),
            Ok(_) => Err("the helper sent an unexpected message".to_string()),
            Err(e) => Err(format!("the helper stopped: {e}")),
        }
    }

    /// The `pcap` data link type of the adapter
    pub fn link_type(&self) -> i32 {
        self.link_type
    }

    pub fn next_packet(&mut self) -> Result<Packet<'_>, Error> {
        if let Some(reason) = &self.ended {
            let reason = reason.clone();
            // the capture thread keeps asking; a stopped helper must not make it spin
            std::thread::sleep(Duration::from_millis(u64::from(TIMEOUT_MS.unsigned_abs())));
            return Err(Error::PcapError(reason));
        }
        loop {
            match read_frame(&mut self.output, &mut self.data) {
                Ok(Frame::Packet { secs, usecs, len }) => {
                    // the field types differ: `time_t`/`suseconds_t` on Unix, 32-bit `c_long`
                    // under Windows, as in Npcap's own `timeval`
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_possible_wrap,
                        clippy::unnecessary_cast
                    )]
                    let ts = libc::timeval {
                        tv_sec: secs as _,
                        tv_usec: usecs as _,
                    };
                    self.header = PacketHeader {
                        ts,
                        caplen: u32::try_from(self.data.len()).unwrap_or(u32::MAX),
                        len,
                    };
                    return Ok(Packet::new(&self.header, &self.data));
                }
                Ok(Frame::Stats(s)) => {
                    self.stats = Stat {
                        received: s.received,
                        dropped: s.dropped,
                        if_dropped: s.if_dropped,
                    };
                }
                Ok(Frame::Timeout) => return Err(Error::TimeoutExpired),
                Ok(Frame::Ready { .. }) => {}
                Ok(Frame::Failed) => {
                    return Err(self.stop(String::from_utf8_lossy(&self.data).into_owned()));
                }
                Err(e) => return Err(self.stop(format!("the capture helper stopped: {e}"))),
            }
        }
    }

    /// A stopped helper is an event of the running program, like an adapter that disappears, so
    /// this does not use `log_err`, which panics in debug builds
    #[allow(clippy::print_stderr)]
    fn stop(&mut self, reason: String) -> Error {
        eprintln!("openpacketmon capture error: {reason}");
        self.end();
        self.ended = Some(reason.clone());
        Error::PcapError(reason)
    }

    /// Stops the helper and collects its exit status, so that it does not linger as a zombie
    fn end(&mut self) {
        // a closed standard input ends the helper by itself; killing it spares waiting for that
        drop(self.commands.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// The latest counters of the helper
    pub fn stats(&self) -> Stat {
        self.stats
    }

    pub fn pause(&mut self) {
        self.command("pause");
    }

    pub fn resume(&mut self) {
        self.command("resume");
    }

    fn command(&mut self, line: &str) {
        if let Some(commands) = self.commands.as_mut() {
            // a helper that stopped shows up in `next_packet`
            let _ = writeln!(commands, "{line}");
        }
    }
}

impl Drop for CaptureHelper {
    fn drop(&mut self) {
        self.end();
    }
}

/// The helper lies next to the program (next to its folder `deps` when running the tests)
fn helper_path() -> PathBuf {
    if let Some(path) = env::var_os(HELPER_VARIABLE) {
        return path.into();
    }
    let mut folder = env::current_exe().unwrap_or_default();
    folder.pop();
    if folder.file_name() == Some(OsStr::new("deps")) {
        folder.pop();
    }
    folder.join(format!("{BINARY}{}", env::consts::EXE_SUFFIX))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::io::Cursor;

    use openpacketmon_capture::{
        Stats, write_failed, write_packet, write_ready, write_stats, write_timeout,
    };
    use pcap::Error;

    use super::CaptureHelper;

    fn reading(wire: Vec<u8>) -> CaptureHelper {
        CaptureHelper::new(Box::new(Cursor::new(wire)))
    }

    #[test]
    fn hands_on_packets_and_counters() {
        let mut wire = Vec::new();
        write_ready(&mut wire, 1).unwrap();
        write_stats(
            &mut wire,
            Stats {
                received: 9,
                dropped: 4,
                if_dropped: 2,
            },
        )
        .unwrap();
        write_packet(&mut wire, 1_800_000_000, 250, 1514, b"abcd").unwrap();
        write_timeout(&mut wire).unwrap();

        let mut helper = reading(wire);
        helper.wait_until_ready().unwrap();
        assert_eq!(helper.link_type(), 1);
        let packet = helper.next_packet().unwrap();
        assert_eq!(i64::from(packet.header.ts.tv_sec), 1_800_000_000);
        assert_eq!(i64::from(packet.header.ts.tv_usec), 250);
        assert_eq!((packet.header.caplen, packet.header.len), (4, 1514));
        assert_eq!(packet.data, b"abcd");
        assert_eq!(helper.stats().dropped, 4);
        assert_eq!(helper.next_packet().err(), Some(Error::TimeoutExpired));
    }

    #[test]
    fn reports_why_the_adapter_could_not_be_opened() {
        let mut wire = Vec::new();
        write_failed(&mut wire, "no permission").unwrap();
        assert_eq!(
            reading(wire).wait_until_ready().unwrap_err(),
            "no permission"
        );
        assert!(reading(Vec::new()).wait_until_ready().is_err());
    }

    #[test]
    fn a_stopped_helper_stays_an_error() {
        let mut wire = Vec::new();
        write_failed(&mut wire, "adapter gone").unwrap();
        let mut helper = reading(wire);
        for _ in 0..2 {
            assert_eq!(
                helper.next_packet().err(),
                Some(Error::PcapError("adapter gone".to_string()))
            );
        }
    }
}
