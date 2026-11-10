//! RTP and RTCP (RFC 3550) with the payload types of RFC 3551. RTP runs on ports that SIP
//! negotiates, so which UDP datagrams carry it is decided by the analysis across packets (from the
//! SDP, or by the form of a flow, see `engine::analysis::voip`); here they are only read.

use std::fmt::Write;
use std::ops::Range;

use super::{Dissection, Field, misc, u16_at, u32_at};

/// What the analysis knows about an RTP or RTCP datagram, for its dissection
pub struct MediaNote<'a> {
    pub rtcp: bool,
    /// The payload types the SDP named (`PCMA/8000`)
    pub formats: &'a [(u8, String)],
    /// SRTP was negotiated (`RTP/SAVP`): the payload is encrypted
    pub srtp: bool,
    /// The clock rate of the stream an RTCP datagram reports on, for its jitter in milliseconds
    pub clock: Option<u32>,
}

/// The fixed header of an RTP packet
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RtpHeader {
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: u8,
    pub marker: bool,
    pub payload_type: u8,
    pub seq: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    /// Where the payload starts and ends (after the CSRCs and the extension, before the padding)
    pub payload: Range<usize>,
}

/// The RTP header of a UDP payload, if it has the form of one: version 2, all its parts inside the
/// datagram, a payload type that is not reserved for RTCP (RFC 5761 4)
pub fn rtp_header(m: &[u8]) -> Option<RtpHeader> {
    if m.len() < 12 || m[0] >> 6 != 2 || (192..=223).contains(&m[1]) {
        return None;
    }
    let csrc_count = m[0] & 0x0f;
    let mut start = 12 + 4 * usize::from(csrc_count);
    let extension = m[0] & 0x10 != 0;
    if extension {
        let words = usize::from(u16_at(m, start + 2)?);
        start += 4 + 4 * words;
    }
    let padding = m[0] & 0x20 != 0;
    let pad = if padding { usize::from(*m.last()?) } else { 0 };
    if start > m.len() || (padding && (pad == 0 || start + pad > m.len())) {
        return None;
    }
    let payload_type = m[1] & 0x7f;
    if (72..=76).contains(&payload_type) {
        return None;
    }
    Some(RtpHeader {
        padding,
        extension,
        csrc_count,
        marker: m[1] & 0x80 != 0,
        payload_type,
        seq: u16_at(m, 2)?,
        timestamp: u32_at(m, 4)?,
        ssrc: u32_at(m, 8)?,
        payload: start..m.len() - pad,
    })
}

/// A UDP payload has the form of RTCP: version 2 and a known packet type first; `strict` also
/// wants the lengths of all packets of the compound packet to add up and a report first (RFC 3550
/// 6.1), for datagrams no SDP announced
pub fn is_rtcp(m: &[u8], strict: bool) -> bool {
    if m.len() < 8 || m[0] >> 6 != 2 || !(200..=207).contains(&m[1]) {
        return false;
    }
    if !strict {
        return true;
    }
    if !(200..=201).contains(&m[1]) {
        return false;
    }
    let mut at = 0;
    while at < m.len() {
        let (Some(first), Some(words)) = (m.get(at), u16_at(m, at + 2)) else {
            return false;
        };
        if first >> 6 != 2 || !(200..=207).contains(&m[at + 1]) {
            return false;
        }
        at += 4 * (usize::from(words) + 1);
    }
    at == m.len()
}

/// The payload types of RFC 3551 (tables 4 and 5): name and clock rate
pub fn static_type(pt: u8) -> Option<(&'static str, u32)> {
    Some(match pt {
        0 => ("ITU-T G.711 PCMU", 8000),
        3 => ("GSM 06.10", 8000),
        4 => ("ITU-T G.723", 8000),
        5 => ("DVI4 8000", 8000),
        6 => ("DVI4 16000", 16000),
        7 => ("LPC", 8000),
        8 => ("ITU-T G.711 PCMA", 8000),
        // RFC 3551 4.5.2: the clock of G.722 runs at 8000 Hz although it samples at 16000 Hz
        9 => ("ITU-T G.722", 8000),
        10 => ("L16 stereo", 44100),
        11 => ("L16 mono", 44100),
        12 => ("QCELP", 8000),
        13 => ("Comfort noise", 8000),
        14 => ("MPEG-I/II Audio", 90000),
        15 => ("ITU-T G.728", 8000),
        16 => ("DVI4 11025", 11025),
        17 => ("DVI4 22050", 22050),
        18 => ("ITU-T G.729", 8000),
        25 => ("Sun CellB video", 90000),
        26 => ("JPEG", 90000),
        28 => ("nv", 90000),
        31 => ("ITU-T H.261", 90000),
        32 => ("MPEG-I/II Video", 90000),
        33 => ("MPEG-II transport stream", 90000),
        34 => ("ITU-T H.263", 90000),
        _ => return None,
    })
}

/// The clock rate of a payload type: from RFC 3551, else from the SDP (`opus/48000/2`)
pub fn clock_rate(pt: u8, formats: &[(u8, String)]) -> Option<u32> {
    let named = formats
        .iter()
        .find(|(f, name)| *f == pt && !name.is_empty())
        .and_then(|(_, name)| name.split('/').nth(1)?.parse().ok());
    named.or_else(|| static_type(pt).map(|(_, clock)| clock))
}

/// The name of a payload type for the info (`ITU-T G.711 PCMA`, `telephone-event (101)`,
/// `dynamic (111)`)
fn type_name(pt: u8, formats: &[(u8, String)]) -> String {
    if let Some((name, _)) = static_type(pt) {
        return name.to_owned();
    }
    match formats
        .iter()
        .find(|(f, name)| *f == pt && !name.is_empty())
    {
        Some((_, name)) => format!("{} ({pt})", name.split('/').next().unwrap_or(name)),
        None if pt >= 96 => format!("dynamic ({pt})"),
        None => format!("unassigned ({pt})"),
    }
}

/// `0x5e1f02a7`, as the display filter wants it
pub fn ssrc_text(ssrc: u32) -> String {
    format!("0x{ssrc:08x}")
}

/// The payload type for the details (`ITU-T G.711 PCMA (8)`, `dynamic (101): telephone-event/8000`)
fn type_text(pt: u8, formats: &[(u8, String)]) -> String {
    if static_type(pt).is_some() || pt < 96 {
        format!("{} ({pt})", type_name(pt, formats))
    } else if let Some((_, name)) = formats.iter().find(|(f, n)| *f == pt && !n.is_empty()) {
        format!("dynamic ({pt}): {name}")
    } else {
        format!("dynamic ({pt}), codec unknown")
    }
}

/// What the payload holds (`160 bytes, G.711 A-law, 20 ms of speech`)
fn payload_text(pt: u8, len: usize, note: &MediaNote) -> String {
    let known =
        static_type(pt).is_some() || note.formats.iter().any(|(f, n)| *f == pt && !n.is_empty());
    match pt {
        _ if note.srtp => format!("{len} bytes (encrypted, SRTP)"),
        0 | 8 => format!(
            "{len} bytes, G.711 {}, {} ms of speech",
            if pt == 8 { "A-law" } else { "µ-law" },
            len / 8
        ),
        _ if known => format!("{len} bytes"),
        _ => format!("{len} bytes (encrypted or unknown)"),
    }
}

/// The CSRCs and the header extension of an RTP packet that starts at `p`
fn more_fields(data: &[u8], p: usize, h: &RtpHeader) -> Vec<Field> {
    let mut out = Vec::new();
    if h.csrc_count > 0 {
        let end = p + 12 + 4 * usize::from(h.csrc_count);
        let csrcs = (0..usize::from(h.csrc_count))
            .filter_map(|i| {
                let at = p + 12 + 4 * i;
                u32_at(data, at).map(|c| Field::new("CSRC", ssrc_text(c), at..at + 4))
            })
            .collect();
        out.push(
            Field::new(
                "Contributing Source identifiers",
                h.csrc_count.to_string(),
                p + 12..end,
            )
            .with_children(csrcs),
        );
    }
    if h.extension {
        let at = p + 12 + 4 * usize::from(h.csrc_count);
        let profile = u16_at(data, at).unwrap_or(0);
        out.push(Field::new(
            "Header extension",
            format!(
                "defined by profile 0x{profile:04x}, {} bytes",
                p + h.payload.start - at - 4
            ),
            at..p + h.payload.start,
        ));
    }
    out
}

/// An RTP packet in the UDP payload `payload`; `false` if it has not the form of one
pub(super) fn rtp(
    data: &[u8],
    payload: Range<usize>,
    note: &MediaNote,
    d: &mut Dissection,
) -> bool {
    let p = payload.start;
    let Some(h) = rtp_header(&data[payload.clone()]) else {
        return false;
    };
    let pt = h.payload_type;
    let yes_no = |b: bool| if b { "True" } else { "False" };
    let mut fields = vec![
        Field::new("Version", "RFC 1889 Version (2)", p..p + 1),
        Field::new(
            "Padding / Extension / CSRC count",
            format!(
                "{} / {} / {}",
                yes_no(h.padding),
                yes_no(h.extension),
                h.csrc_count
            ),
            p..p + 1,
        ),
        Field::new(
            "Marker",
            if h.marker {
                "True (start of a talkspurt)"
            } else {
                "False"
            },
            p + 1..p + 2,
        ),
        Field::new("Payload type", type_text(pt, note.formats), p + 1..p + 2),
        Field::new("Sequence number", h.seq.to_string(), p + 2..p + 4),
        Field::new("Timestamp", h.timestamp.to_string(), p + 4..p + 8),
        Field::new(
            "Synchronization Source identifier",
            format!("{} ({})", ssrc_text(h.ssrc), h.ssrc),
            p + 8..p + 12,
        ),
    ];
    fields.extend(more_fields(data, p, &h));
    fields.push(Field::new(
        "Payload",
        payload_text(pt, h.payload.len(), note),
        p + h.payload.start..p + h.payload.end,
    ));
    if h.padding {
        let pad = payload.len() - h.payload.end;
        fields.push(Field::new(
            "Padding",
            format!("{pad} bytes"),
            p + h.payload.end..payload.end,
        ));
    }
    d.protocol = "RTP";
    d.has("rtp");
    d.text("rtp.ssrc", ssrc_text(h.ssrc));
    d.num("rtp.seq", h.seq);
    d.num("rtp.p_type", pt);
    d.num("rtp.timestamp", h.timestamp);
    d.num("rtp.marker", u8::from(h.marker));
    d.add("Real-Time Transport Protocol".into(), payload, fields);
    d.info = format!(
        "PT={}, SSRC=0x{:08X}, Seq={}, Time={}",
        type_name(pt, note.formats),
        h.ssrc,
        h.seq,
        h.timestamp
    );
    if h.marker {
        d.info.push_str(", Mark");
    }
    true
}

/// The name of an RTCP packet type for the info and the title of its layer
fn rtcp_names(pt: u8) -> (&'static str, &'static str) {
    match pt {
        200 => ("Sender Report", "Sender Report"),
        201 => ("Receiver Report", "Receiver Report"),
        202 => ("Source Description", "Source description"),
        203 => ("Goodbye", "Goodbye"),
        204 => ("Application specific", "Application specific"),
        205 => ("Generic RTP Feedback", "Generic RTP feedback"),
        206 => ("Payload-specific Feedback", "Payload-specific feedback"),
        207 => ("Extended Report", "Extended report"),
        _ => ("Unknown", "unknown packet type"),
    }
}

/// A report block of RTCP: how a receiver saw a stream (RFC 3550 6.4.1)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    /// The stream reported on
    pub about: u32,
    /// Lost since the last report, in 256ths
    pub fraction: u8,
    /// Interarrival jitter in units of the stream's clock
    pub jitter: u32,
}

/// The report blocks of the sender and receiver reports of an RTCP packet (also compound)
pub fn reports(m: &[u8]) -> Vec<Report> {
    let mut found = Vec::new();
    let mut at = 0;
    while at + 8 <= m.len() && m[at] >> 6 == 2 {
        let count = usize::from(m[at] & 0x1f);
        let len = (usize::from(u16::from_be_bytes([m[at + 2], m[at + 3]])) + 1) * 4;
        let end = (at + len).min(m.len());
        let first = match m[at + 1] {
            200 => Some(at + 28),
            201 => Some(at + 8),
            _ => None,
        };
        if let Some(mut block) = first {
            for _ in 0..count {
                if block + 24 > end {
                    break;
                }
                let word = |i: usize| u32::from_be_bytes([m[i], m[i + 1], m[i + 2], m[i + 3]]);
                found.push(Report {
                    about: word(block),
                    fraction: m[block + 4],
                    jitter: word(block + 12),
                });
                block += 24;
            }
        }
        at += len;
    }
    found
}

/// The report blocks of a sender or receiver report (RFC 3550 6.4.1), from `at` on
fn report_blocks(
    data: &[u8],
    mut at: usize,
    count: u8,
    end: usize,
    clock: Option<u32>,
) -> Vec<Field> {
    let mut fields = Vec::new();
    for _ in 0..count {
        if at + 24 > end {
            break;
        }
        let (Some(ssrc), Some(lost), Some(highest), Some(jitter), Some(lsr), Some(dlsr)) = (
            u32_at(data, at),
            u32_at(data, at + 4),
            u32_at(data, at + 8),
            u32_at(data, at + 12),
            u32_at(data, at + 16),
            u32_at(data, at + 20),
        ) else {
            break;
        };
        let fraction = lost >> 24;
        // a signed 24-bit number
        let cumulative = ((lost & 0x00ff_ffff) << 8).cast_signed() >> 8;
        fields.push(Field::new("Report: SSRC", ssrc_text(ssrc), at..at + 4));
        fields.push(Field::new(
            "Fraction lost / Cumulative lost",
            format!(
                "{fraction} / 256 ({:.1} %) / {cumulative}",
                f64::from(fraction) * 100.0 / 256.0
            ),
            at + 4..at + 8,
        ));
        fields.push(Field::new(
            "Extended highest sequence number",
            highest.to_string(),
            at + 8..at + 12,
        ));
        let mut jitter_text = jitter.to_string();
        if let Some(rate) = clock.filter(|r| *r > 0) {
            let _ = write!(
                jitter_text,
                " ({:.1} ms at {rate} Hz)",
                f64::from(jitter) * 1000.0 / f64::from(rate)
            );
        }
        fields.push(Field::new(
            "Interarrival jitter",
            jitter_text,
            at + 12..at + 16,
        ));
        let mut since = format!("{lsr} / {dlsr}");
        if dlsr > 0 {
            let _ = write!(since, " ({:.1} ms)", f64::from(dlsr) * 1000.0 / 65536.0);
        }
        fields.push(Field::new(
            "Last SR / Delay since last SR",
            since,
            at + 16..at + 24,
        ));
        at += 24;
    }
    fields
}

/// The items of a source description (RFC 3550 6.5), one layer for all its chunks
fn sdes(data: &[u8], start: usize, count: u8, end: usize) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut at = start + 4;
    for _ in 0..count {
        let Some(ssrc) = u32_at(data, at) else {
            break;
        };
        fields.push(Field::new("SSRC", ssrc_text(ssrc), at..at + 4));
        at += 4;
        while at < end {
            let kind = data[at];
            if kind == 0 {
                // the end of the chunk, padded to a full word
                at = (at - start + 4) / 4 * 4 + start;
                break;
            }
            let Some(&len) = data.get(at + 1).filter(|_| at + 2 <= end) else {
                return fields;
            };
            let value_end = (at + 2 + usize::from(len)).min(end);
            let name = match kind {
                1 => "CNAME",
                2 => "NAME",
                3 => "EMAIL",
                4 => "PHONE",
                5 => "LOC",
                6 => "TOOL",
                7 => "NOTE",
                8 => "PRIV",
                _ => "Item",
            };
            fields.push(Field::new(
                name,
                String::from_utf8_lossy(&data[at + 2..value_end]).into_owned(),
                at..value_end,
            ));
            at = value_end;
        }
    }
    fields
}

/// Where the values of the items of the source descriptions in an RTCP compound packet are (`m`, a
/// UDP payload), with their kind (`1` CNAME, `2` NAME, `3` EMAIL, `4` PHONE …), as [`sdes`] reads
/// them
pub fn sdes_items(m: &[u8]) -> Vec<(u8, Range<usize>)> {
    let mut items = Vec::new();
    let mut at = 0;
    while at + 4 <= m.len() {
        let (first, pt) = (m[at], m[at + 1]);
        let words = usize::from(u16_at(m, at + 2).unwrap_or(0));
        let end = at + 4 * (words + 1);
        if first >> 6 != 2 || !(200..=207).contains(&pt) || end > m.len() {
            break;
        }
        if pt == 202 {
            let mut i = at + 4;
            'chunks: for _ in 0..first & 0x1f {
                if i + 4 > end {
                    break;
                }
                i += 4;
                while i < end {
                    let kind = m[i];
                    if kind == 0 {
                        i = (i - at + 4) / 4 * 4 + at;
                        continue 'chunks;
                    }
                    let Some(&len) = m.get(i + 1).filter(|_| i + 2 <= end) else {
                        break 'chunks;
                    };
                    let value_end = (i + 2 + usize::from(len)).min(end);
                    items.push((kind, i + 2..value_end));
                    i = value_end;
                }
            }
        }
        at = end;
    }
    items
}

/// The fields of one packet of an RTCP compound packet, at `packet`
fn rtcp_fields(data: &[u8], packet: Range<usize>, clock: Option<u32>) -> Vec<Field> {
    let (at, end) = (packet.start, packet.end);
    let (first, pt) = (data[at], data[at + 1]);
    let words = (end - at) / 4 - 1;
    let count = first & 0x1f;
    let (_, title) = rtcp_names(pt);
    let head = vec![
        Field::new(
            match pt {
                200 | 201 => "Version / Reception report count",
                202 | 203 => "Version / Source count",
                204 => "Version / Subtype",
                205 | 206 => "Version / Feedback message type",
                _ => "Version / Count",
            },
            format!("{} / {count}", first >> 6),
            at..at + 1,
        ),
        Field::new("Packet type", format!("{title} ({pt})"), at + 1..at + 2),
        Field::new(
            "Length",
            format!("{words} ({} bytes)", 4 * (words + 1)),
            at + 2..at + 4,
        ),
    ];
    match pt {
        200 => {
            let mut f = head;
            if let Some(ssrc) = u32_at(data, at + 4) {
                f.push(Field::new("Sender SSRC", ssrc_text(ssrc), at + 4..at + 8));
            }
            if at + 28 <= end {
                f.push(Field::new(
                    "Timestamp, MSW / LSW",
                    misc::ntp_time(data, at + 8),
                    at + 8..at + 16,
                ));
                for (name, offset) in [
                    ("RTP timestamp", 16),
                    ("Sender's packet count", 20),
                    ("Sender's octet count", 24),
                ] {
                    let value = u32_at(data, at + offset).unwrap_or(0);
                    f.push(Field::new(
                        name,
                        value.to_string(),
                        at + offset..at + offset + 4,
                    ));
                }
                f.extend(report_blocks(data, at + 28, count, end, clock));
            }
            f
        }
        201 => {
            let mut f = head;
            if let Some(ssrc) = u32_at(data, at + 4) {
                f.push(Field::new("Sender SSRC", ssrc_text(ssrc), at + 4..at + 8));
            }
            f.extend(report_blocks(data, at + 8, count, end, clock));
            f
        }
        202 => sdes(data, at, count, end),
        203 => {
            let mut f = head;
            let mut pos = at + 4;
            for _ in 0..count {
                let Some(ssrc) = u32_at(data, pos).filter(|_| pos + 4 <= end) else {
                    break;
                };
                f.push(Field::new("SSRC", ssrc_text(ssrc), pos..pos + 4));
                pos += 4;
            }
            if let Some(&len) = data.get(pos).filter(|_| pos < end) {
                let reason_end = (pos + 1 + usize::from(len)).min(end);
                f.push(Field::new(
                    "Reason",
                    String::from_utf8_lossy(&data[pos + 1..reason_end]).into_owned(),
                    pos..reason_end,
                ));
            }
            f
        }
        204 => {
            let mut f = head;
            if let Some(ssrc) = u32_at(data, at + 4) {
                f.push(Field::new("SSRC", ssrc_text(ssrc), at + 4..at + 8));
            }
            if at + 12 <= end {
                f.push(Field::new(
                    "Name",
                    super::text(&data[at + 8..at + 12]),
                    at + 8..at + 12,
                ));
            }
            f
        }
        _ => head,
    }
}

/// An RTCP compound packet: a layer for each of its packets
pub(super) fn rtcp(
    data: &[u8],
    payload: Range<usize>,
    clock: Option<u32>,
    d: &mut Dissection,
) -> bool {
    if !is_rtcp(&data[payload.clone()], false) {
        return false;
    }
    let mut names = Vec::new();
    let mut at = payload.start;
    while at + 4 <= payload.end {
        let (first, pt) = (data[at], data[at + 1]);
        let words = usize::from(u16_at(data, at + 2).unwrap_or(0));
        let end = at + 4 * (words + 1);
        if first >> 6 != 2 || !(200..=207).contains(&pt) || end > payload.end {
            // the rest is encrypted (SRTCP) or not RTCP
            let len = payload.end - at;
            d.add(
                format!("Data ({len} bytes)"),
                at..payload.end,
                vec![Field::new(
                    "Data",
                    format!("{len} bytes (encrypted or unknown)"),
                    at..payload.end,
                )],
            );
            break;
        }
        let (name, title) = rtcp_names(pt);
        names.push(name);
        d.num("rtcp.pt", pt);
        let fields = rtcp_fields(data, at..end, clock);
        d.add(
            format!("Real-time Transport Control Protocol ({title})"),
            at..end,
            fields,
        );
        at = end;
    }
    if names.is_empty() {
        return false;
    }
    d.protocol = "RTCP";
    d.has("rtcp");
    d.info = names.join(", ");
    true
}
