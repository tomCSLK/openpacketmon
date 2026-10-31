//! What `openpacketmon-capture` sends to openpacketmon over its standard output.
//!
//! The helper is the only part of openpacketmon with the rights to capture packets. It opens the
//! adapter, gives up its rights and passes on what it reads, without looking into the packets.
//! Standard input carries the commands `pause` and `resume`, one per line; when it closes, the
//! helper ends, so it cannot outlive openpacketmon.
//!
//! Every message starts with a tag byte; numbers are little endian. The functions here only fail
//! when the stream does or when it carries something else than these messages.

#![allow(clippy::missing_errors_doc)]

use std::io::{self, Error, ErrorKind, Read, Write};

/// Name of the helper's binary, without the suffix of the platform
pub const BINARY: &str = "openpacketmon-capture";
/// How long the helper waits for packets before it reports [`Frame::Timeout`]
pub const TIMEOUT_MS: i32 = 150;
/// Largest packet accepted from the helper; it captures at most 65535 bytes of a packet
pub const MAX_PACKET_BYTES: usize = 262_144;
/// Largest error message accepted from the helper
pub const MAX_TEXT_BYTES: usize = 4096;

const READY: u8 = 1;
const PACKET: u8 = 2;
const STATS: u8 = 3;
const TIMEOUT: u8 = 4;
const FAILED: u8 = 5;

/// Counters of a live capture
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    pub received: u32,
    pub dropped: u32,
    pub if_dropped: u32,
}

/// A message of the helper; [`read_frame`] puts the bytes of a packet or an error message aside
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Frame {
    /// The adapter is open and the rights are gone; `link_type` is the `pcap` data link type
    Ready {
        link_type: i32,
    },
    /// A packet: its time and its length on the wire
    Packet {
        secs: i64,
        usecs: i64,
        len: u32,
    },
    Stats(Stats),
    /// No packet arrived for [`TIMEOUT_MS`]
    Timeout,
    /// The helper stops; the bytes are the reason as UTF-8
    Failed,
}

pub fn write_ready(w: &mut impl Write, link_type: i32) -> io::Result<()> {
    w.write_all(&[READY])?;
    w.write_all(&link_type.to_le_bytes())
}

pub fn write_packet(
    w: &mut impl Write,
    secs: i64,
    usecs: i64,
    len: u32,
    data: &[u8],
) -> io::Result<()> {
    let captured = u32::try_from(data.len()).map_err(|_| too_long())?;
    w.write_all(&[PACKET])?;
    w.write_all(&secs.to_le_bytes())?;
    w.write_all(&usecs.to_le_bytes())?;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&captured.to_le_bytes())?;
    w.write_all(data)
}

pub fn write_stats(w: &mut impl Write, stats: Stats) -> io::Result<()> {
    w.write_all(&[STATS])?;
    w.write_all(&stats.received.to_le_bytes())?;
    w.write_all(&stats.dropped.to_le_bytes())?;
    w.write_all(&stats.if_dropped.to_le_bytes())
}

pub fn write_timeout(w: &mut impl Write) -> io::Result<()> {
    w.write_all(&[TIMEOUT])
}

pub fn write_failed(w: &mut impl Write, reason: &str) -> io::Result<()> {
    let mut end = reason.len().min(MAX_TEXT_BYTES);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    let text = &reason.as_bytes()[..end];
    let length = u32::try_from(text.len()).map_err(|_| too_long())?;
    w.write_all(&[FAILED])?;
    w.write_all(&length.to_le_bytes())?;
    w.write_all(text)
}

/// Reads the next message. The bytes of a packet or of a reason end up in `data`.
pub fn read_frame(r: &mut impl Read, data: &mut Vec<u8>) -> io::Result<Frame> {
    data.clear();
    match read_array::<1>(r)?[0] {
        READY => Ok(Frame::Ready {
            link_type: i32::from_le_bytes(read_array(r)?),
        }),
        PACKET => {
            let secs = i64::from_le_bytes(read_array(r)?);
            let usecs = i64::from_le_bytes(read_array(r)?);
            let len = u32::from_le_bytes(read_array(r)?);
            read_bytes(r, data, MAX_PACKET_BYTES)?;
            Ok(Frame::Packet { secs, usecs, len })
        }
        STATS => Ok(Frame::Stats(Stats {
            received: u32::from_le_bytes(read_array(r)?),
            dropped: u32::from_le_bytes(read_array(r)?),
            if_dropped: u32::from_le_bytes(read_array(r)?),
        })),
        TIMEOUT => Ok(Frame::Timeout),
        FAILED => {
            read_bytes(r, data, MAX_TEXT_BYTES)?;
            Ok(Frame::Failed)
        }
        tag => Err(Error::new(
            ErrorKind::InvalidData,
            format!("unknown message {tag}"),
        )),
    }
}

fn read_array<const N: usize>(r: &mut impl Read) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    r.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// Reads a length and that many bytes
fn read_bytes(r: &mut impl Read, data: &mut Vec<u8>, max: usize) -> io::Result<()> {
    let length = usize::try_from(u32::from_le_bytes(read_array(r)?)).map_err(|_| too_long())?;
    if length > max {
        return Err(too_long());
    }
    data.resize(length, 0);
    r.read_exact(data)
}

fn too_long() -> Error {
    Error::new(ErrorKind::InvalidData, "message too long")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{
        Frame, MAX_PACKET_BYTES, MAX_TEXT_BYTES, PACKET, Stats, read_frame, write_failed,
        write_packet, write_ready, write_stats, write_timeout,
    };
    use std::io::{Cursor, ErrorKind};

    #[test]
    fn every_message_comes_back() {
        let stats = Stats {
            received: 7,
            dropped: 2,
            if_dropped: 1,
        };
        let mut wire = Vec::new();
        write_ready(&mut wire, 113).unwrap();
        write_packet(&mut wire, 1_800_000_000, 999_999, 1514, b"abc").unwrap();
        write_stats(&mut wire, stats).unwrap();
        write_timeout(&mut wire).unwrap();
        write_failed(&mut wire, "kein Adapter").unwrap();

        let mut r = Cursor::new(wire);
        let mut data = Vec::new();
        assert_eq!(
            read_frame(&mut r, &mut data).unwrap(),
            Frame::Ready { link_type: 113 }
        );
        assert_eq!(
            read_frame(&mut r, &mut data).unwrap(),
            Frame::Packet {
                secs: 1_800_000_000,
                usecs: 999_999,
                len: 1514
            }
        );
        assert_eq!(data, b"abc");
        assert_eq!(read_frame(&mut r, &mut data).unwrap(), Frame::Stats(stats));
        assert!(data.is_empty());
        assert_eq!(read_frame(&mut r, &mut data).unwrap(), Frame::Timeout);
        assert_eq!(read_frame(&mut r, &mut data).unwrap(), Frame::Failed);
        assert_eq!(data, "kein Adapter".as_bytes());
        assert_eq!(
            read_frame(&mut r, &mut data).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn a_long_reason_is_cut_between_characters() {
        let mut wire = Vec::new();
        write_failed(&mut wire, &"ä".repeat(MAX_TEXT_BYTES)).unwrap();
        let mut data = Vec::new();
        read_frame(&mut Cursor::new(wire), &mut data).unwrap();
        assert!(data.len() <= MAX_TEXT_BYTES);
        assert!(std::str::from_utf8(&data).is_ok());
    }

    #[test]
    fn oversized_and_unknown_messages_are_refused() {
        let mut wire = vec![PACKET];
        wire.extend_from_slice(&[0; 20]);
        wire.extend_from_slice(&u32::try_from(MAX_PACKET_BYTES + 1).unwrap().to_le_bytes());
        let mut data = Vec::new();
        let error = read_frame(&mut Cursor::new(wire), &mut data).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        assert!(data.capacity() <= MAX_PACKET_BYTES);

        let error = read_frame(&mut Cursor::new(vec![200]), &mut data).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn broken_input_ends_in_an_error() {
        // the helper is trusted, but a cut stream must not panic or read past the limits
        let mut wire = Vec::new();
        write_packet(&mut wire, 1, 2, 3, &[9; 40]).unwrap();
        let mut data = Vec::new();
        for end in 0..wire.len() {
            assert!(read_frame(&mut Cursor::new(&wire[..end]), &mut data).is_err());
        }
    }
}
