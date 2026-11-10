//! NTP (RFC 5905) and the unencrypted header of QUIC (RFC 9000, 9369)

use std::fmt::Write;
use std::ops::Range;

use super::{Dissection, Field, hex_bytes, ipv4_at, text, u8_at, u32_at};

/// Seconds from 1900 (NTP era 0) to 1970
const NTP_UNIX_OFFSET: i64 = 2_208_988_800;

/// An NTP timestamp (seconds since 1900 and their fraction) at `at`, as a time in UTC
pub(super) fn ntp_time(data: &[u8], at: usize) -> String {
    let (Some(secs), Some(frac)) = (u32_at(data, at), u32_at(data, at + 4)) else {
        return String::new();
    };
    if secs == 0 && frac == 0 {
        return "(not set)".into();
    }
    let micros = (u64::from(frac) * 1_000_000) >> 32;
    jiff::Timestamp::from_microsecond(
        (i64::from(secs) - NTP_UNIX_OFFSET) * 1_000_000 + i64::try_from(micros).unwrap_or(0),
    )
    .map_or_else(|_| secs.to_string(), |t| format!("{t:.6}"))
}

/// NTP over UDP 123
pub(super) fn ntp(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let p = payload.start;
    let m = &data[payload.clone()];
    if m.len() < 48 {
        return false;
    }
    let li = m[0] >> 6;
    let version = (m[0] >> 3) & 0x07;
    let mode = m[0] & 0x07;
    if !(1..=4).contains(&version) {
        return false;
    }
    let mode_name = match mode {
        1 => "symmetric active",
        2 => "symmetric passive",
        3 => "client",
        4 => "server",
        5 => "broadcast",
        6 => "control",
        7 => "private",
        _ => "reserved",
    };
    let stratum = m[1];
    let reference = if stratum < 2 {
        text(m[12..16].split(|b| *b == 0).next().unwrap_or_default())
    } else {
        ipv4_at(m, 12).map(|a| a.to_string()).unwrap_or_default()
    };
    let fields = vec![
        Field::new(
            "Leap indicator",
            match li {
                0 => "no warning",
                1 => "last minute has 61 seconds",
                2 => "last minute has 59 seconds",
                _ => "clock not synchronized",
            },
            p..p + 1,
        ),
        Field::new("Version", version.to_string(), p..p + 1),
        Field::new("Mode", format!("{mode_name} ({mode})"), p..p + 1),
        Field::new("Stratum", stratum.to_string(), p + 1..p + 2),
        Field::new(
            "Poll interval",
            format!("{} s", 1u64 << m[2].min(17)),
            p + 2..p + 3,
        ),
        Field::new(
            "Precision",
            format!("2^{}", i8::from_be_bytes([m[3]])),
            p + 3..p + 4,
        ),
        Field::new("Reference ID", reference, p + 12..p + 16),
        Field::new("Reference timestamp", ntp_time(m, 16), p + 16..p + 24),
        Field::new("Origin timestamp", ntp_time(m, 24), p + 24..p + 32),
        Field::new("Receive timestamp", ntp_time(m, 32), p + 32..p + 40),
        Field::new("Transmit timestamp", ntp_time(m, 40), p + 40..p + 48),
    ];
    d.protocol = "NTP";
    d.has("ntp");
    d.num("ntp.version", version);
    d.num("ntp.mode", mode);
    d.num("ntp.stratum", stratum);
    d.add(
        format!("Network Time Protocol (NTP Version {version}, {mode_name})"),
        payload,
        fields,
    );
    d.info = format!("NTP Version {version}, {mode_name}");
    true
}

fn quic_version(v: u32) -> String {
    match v {
        0 => "Version Negotiation".into(),
        1 => "1".into(),
        0x6b33_43cf => "2".into(),
        0xff00_0000..=0xff00_00ff => format!("draft-{}", v & 0xff),
        _ if v & 0x0f0f_0f0f == 0x0a0a_0a0a => "GREASE".into(),
        _ => format!("0x{v:08x}"),
    }
}

/// QUIC over UDP 443: the header, without decrypting the packet
#[allow(clippy::too_many_lines)] // long and short header, version negotiation
pub(super) fn quic(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let p = payload.start;
    let m = &data[payload.clone()];
    let Some(&first) = m.first() else {
        return false;
    };
    if first & 0x80 == 0 {
        // short header: the connection ID has no length field, all else is encrypted
        if first & 0x40 == 0 {
            return false;
        }
        d.protocol = "QUIC";
        d.has("quic");
        d.add(
            "QUIC (short header)".into(),
            payload.clone(),
            vec![
                Field::new("Header form", "Short header", p..p + 1),
                Field::new("Protected payload", format!("{} bytes", m.len()), payload),
            ],
        );
        d.info = "Protected Payload".into();
        return true;
    }
    let Some(version) = u32_at(m, 1) else {
        return false;
    };
    if version != 0 && first & 0x40 == 0 {
        return false;
    }
    let Some(dcid_len) = u8_at(m, 5).map(usize::from).filter(|l| *l <= 20) else {
        return false;
    };
    let Some(dcid) = m.get(6..6 + dcid_len) else {
        return false;
    };
    let at = 6 + dcid_len;
    let Some(scid_len) = u8_at(m, at).map(usize::from).filter(|l| *l <= 20) else {
        return false;
    };
    let Some(scid) = m.get(at + 1..at + 1 + scid_len) else {
        return false;
    };
    let after = at + 1 + scid_len;
    let kind = if version == 0 {
        "Version Negotiation"
    } else {
        let bits = (first >> 4) & 0x03;
        // QUIC version 2 rotates the type numbers (RFC 9369 3.2)
        let bits = if version == 0x6b33_43cf {
            (bits + 3) % 4
        } else {
            bits
        };
        ["Initial", "0-RTT", "Handshake", "Retry"][usize::from(bits)]
    };
    let mut fields = vec![
        Field::new("Header form", "Long header", p..p + 1),
        Field::new("Packet type", kind, p..p + 1),
        Field::new("Version", quic_version(version), p + 1..p + 5),
        Field::new(
            "Destination connection ID length",
            dcid_len.to_string(),
            p + 5..p + 6,
        ),
        Field::new("Destination connection ID", hex_bytes(dcid), p + 6..p + at),
        Field::new(
            "Source connection ID length",
            scid_len.to_string(),
            p + at..p + at + 1,
        ),
        Field::new(
            "Source connection ID",
            hex_bytes(scid),
            p + at + 1..p + after,
        ),
    ];
    if version == 0 {
        let versions: Vec<String> = (after..m.len())
            .step_by(4)
            .filter_map(|i| u32_at(m, i).map(quic_version))
            .collect();
        fields.push(Field::new(
            "Supported versions",
            versions.join(", "),
            p + after..payload.end,
        ));
    } else {
        fields.push(Field::new(
            "Protected payload",
            format!("{} bytes", m.len() - after),
            p + after..payload.end,
        ));
    }
    d.protocol = "QUIC";
    d.has("quic");
    d.num("quic.version", version);
    d.text("quic.dcid", hex_bytes(dcid));
    d.text("quic.type", kind);
    d.add(format!("QUIC ({kind})"), payload, fields);
    d.info = format!("{kind}, DCID={}", hex_bytes(dcid));
    if !scid.is_empty() {
        let _ = write!(d.info, ", SCID={}", hex_bytes(scid));
    }
    true
}
