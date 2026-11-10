//! Transport layer: TCP, UDP, ICMP, `ICMPv6`, IGMP

use std::fmt::Write;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ops::Range;

use super::net::ip_protocol_name;
use super::{Dissection, Field, TransportInfo, hex16, u8_at, u16_at, u32_at};

/// Dissects the header of `protocol` in `data[start..end]`; returns the range of its payload
pub(super) fn dissect(
    data: &[u8],
    start: usize,
    end: usize,
    protocol: u8,
    d: &mut Dissection,
) -> Option<Range<usize>> {
    match protocol {
        6 => tcp(data, start, end, d),
        17 => udp(data, start, end, d),
        1 => icmp(data, start, end, d),
        58 => icmpv6(data, start, end, d),
        2 => {
            igmp(data, start, end, d);
            None
        }
        other => {
            let name = ip_protocol_name(other);
            d.protocol = if name == "unknown" { "IP" } else { name };
            d.info = format!("{name} ({other})");
            Some(start..end)
        }
    }
}

/// Adds the layer even without all its bytes and marks the packet as cut off
fn short(d: &mut Dissection, title: &str, start: usize, end: usize) -> Option<Range<usize>> {
    d.add(title.into(), start..end, Vec::new());
    d.cut(end);
    None
}

const TCP_FLAGS: [(u16, &str); 9] = [
    (0x100, "NS"),
    (0x080, "CWR"),
    (0x040, "ECE"),
    (0x020, "URG"),
    (0x010, "ACK"),
    (0x008, "PSH"),
    (0x004, "RST"),
    (0x002, "SYN"),
    (0x001, "FIN"),
];

/// Display filter keys of the TCP flags, in the order of [`TCP_FLAGS`]
const TCP_FLAG_KEYS: [&str; 9] = [
    "tcp.flags.ns",
    "tcp.flags.cwr",
    "tcp.flags.ece",
    "tcp.flags.urg",
    "tcp.flags.ack",
    "tcp.flags.psh",
    "tcp.flags.rst",
    "tcp.flags.syn",
    "tcp.flags.fin",
];

#[allow(clippy::too_many_lines)] // header, flags, options and the info line
fn tcp(data: &[u8], start: usize, end: usize, d: &mut Dissection) -> Option<Range<usize>> {
    d.protocol = "TCP";
    d.has("tcp");
    let title = "Transmission Control Protocol";
    if end < start + 20 {
        return short(d, title, start, end);
    }
    let (sport, dport) = (u16_at(data, start)?, u16_at(data, start + 2)?);
    let (seq, ack) = (u32_at(data, start + 4)?, u32_at(data, start + 8)?);
    let offset_flags = u16_at(data, start + 12)?;
    let header_len = usize::from(offset_flags >> 12) * 4;
    let flags = offset_flags & 0x01ff;
    let window = u16_at(data, start + 14)?;
    let names: Vec<&str> = TCP_FLAGS
        .iter()
        .rev()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(_, name)| *name)
        .collect();
    let flag_text = if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    };
    let payload_start = (start + header_len).min(end);
    let len = end - payload_start;
    d.num("tcp.srcport", sport);
    d.num("tcp.dstport", dport);
    d.num("tcp.seq", seq);
    d.num("tcp.ack", ack);
    d.num("tcp.window", window);
    d.num("tcp.len", u64::try_from(len).unwrap_or(0));
    for ((bit, _), key) in TCP_FLAGS.iter().zip(TCP_FLAG_KEYS) {
        d.num(key, u8::from(flags & bit != 0));
    }
    let mut fields = vec![
        Field::new("Source port", sport.to_string(), start..start + 2),
        Field::new("Destination port", dport.to_string(), start + 2..start + 4),
        Field::new("Sequence number", seq.to_string(), start + 4..start + 8),
        Field::new(
            "Acknowledgment number",
            ack.to_string(),
            start + 8..start + 12,
        ),
        Field::new(
            "Header length",
            format!("{header_len} bytes"),
            start + 12..start + 13,
        ),
        Field::new(
            "Flags",
            format!("0x{flags:03x} ({flag_text})"),
            start + 12..start + 14,
        )
        .with_children(
            TCP_FLAGS
                .iter()
                .map(|(bit, name)| {
                    Field::new(
                        *name,
                        if flags & bit == 0 { "Not set" } else { "Set" },
                        start + 12..start + 14,
                    )
                })
                .collect(),
        ),
        Field::new("Window", window.to_string(), start + 14..start + 16),
        Field::new(
            "Checksum",
            hex16(u16_at(data, start + 16)?),
            start + 16..start + 18,
        ),
        Field::new(
            "Urgent pointer",
            u16_at(data, start + 18)?.to_string(),
            start + 18..start + 20,
        ),
    ];
    if header_len > 20 {
        let options_end = (start + header_len).min(end);
        fields.push(
            Field::new(
                "Options",
                format!("{} bytes", header_len - 20),
                start + 20..options_end,
            )
            .with_children(tcp_options(data, start + 20, options_end)),
        );
    }
    d.add(
        format!("{title}, Src Port: {sport}, Dst Port: {dport}, Len: {len}"),
        start..payload_start,
        fields,
    );
    d.info = format!("{sport} → {dport} [{flag_text}] Seq={seq}");
    if flags & 0x010 != 0 {
        let _ = write!(d.info, " Ack={ack}");
    }
    let _ = write!(d.info, " Win={window} Len={len}");
    if header_len < 20 {
        d.info = format!("{sport} → {dport} TCP header with an invalid length");
        return None;
    }
    if start + header_len > end {
        d.cut(end);
        return None;
    }
    d.transport = Some(TransportInfo {
        tcp: true,
        sport,
        dport,
        seq,
        payload: payload_start..end,
    });
    Some(payload_start..end)
}

fn tcp_options(data: &[u8], mut at: usize, end: usize) -> Vec<Field> {
    let mut options = Vec::new();
    while at < end {
        let Some(kind) = u8_at(data, at) else { break };
        match kind {
            0 => {
                options.push(Field::new("End of option list", "", at..end));
                break;
            }
            1 => {
                options.push(Field::new("No-Operation", "", at..at + 1));
                at += 1;
                continue;
            }
            _ => {}
        }
        let Some(len) = u8_at(data, at + 1).map(usize::from) else {
            break;
        };
        if len < 2 || at + len > end {
            options.push(Field::new(
                "Invalid option",
                format!("kind {kind}"),
                at..end,
            ));
            break;
        }
        let range = at..at + len;
        let option = match (kind, len) {
            (2, 4) => Field::new(
                "Maximum segment size",
                format!("{} bytes", u16_at(data, at + 2).unwrap_or(0)),
                range,
            ),
            (3, 3) => {
                let shift = u8_at(data, at + 2).unwrap_or(0);
                Field::new(
                    "Window scale",
                    format!("{shift} (multiply by {})", 1u32 << shift.min(14)),
                    range,
                )
            }
            (4, 2) => Field::new("SACK permitted", "", range),
            (5, _) => {
                let blocks = (at + 2..at + len)
                    .step_by(8)
                    .filter_map(|b| Some(format!("{}-{}", u32_at(data, b)?, u32_at(data, b + 4)?)))
                    .collect::<Vec<_>>()
                    .join(", ");
                Field::new("SACK", blocks, range)
            }
            (8, 10) => Field::new(
                "Timestamps",
                format!(
                    "TSval {}, TSecr {}",
                    u32_at(data, at + 2).unwrap_or(0),
                    u32_at(data, at + 6).unwrap_or(0)
                ),
                range,
            ),
            (30, _) => Field::new("Multipath TCP", format!("{len} bytes"), range),
            (34, _) => Field::new("TCP Fast Open cookie", format!("{len} bytes"), range),
            _ => Field::new("Option", format!("kind {kind}, {len} bytes"), range),
        };
        options.push(option);
        at += len;
    }
    options
}

fn udp(data: &[u8], start: usize, end: usize, d: &mut Dissection) -> Option<Range<usize>> {
    d.protocol = "UDP";
    d.has("udp");
    let title = "User Datagram Protocol";
    if end < start + 8 {
        return short(d, title, start, end);
    }
    let (sport, dport) = (u16_at(data, start)?, u16_at(data, start + 2)?);
    let length = usize::from(u16_at(data, start + 4)?);
    let checksum = u16_at(data, start + 6)?;
    let payload_end = if length >= 8 {
        (start + length).min(end)
    } else {
        end
    };
    let len = length.saturating_sub(8);
    d.num("udp.srcport", sport);
    d.num("udp.dstport", dport);
    d.num("udp.len", u64::try_from(len).unwrap_or(0));
    d.add(
        format!("{title}, Src Port: {sport}, Dst Port: {dport}"),
        start..start + 8,
        vec![
            Field::new("Source port", sport.to_string(), start..start + 2),
            Field::new("Destination port", dport.to_string(), start + 2..start + 4),
            Field::new("Length", length.to_string(), start + 4..start + 6),
            Field::new(
                "Checksum",
                if checksum == 0 {
                    "0x0000 [none]".to_string()
                } else {
                    hex16(checksum)
                },
                start + 6..start + 8,
            ),
        ],
    );
    d.info = format!("{sport} → {dport} Len={len}");
    d.transport = Some(TransportInfo {
        tcp: false,
        sport,
        dport,
        seq: 0,
        payload: start + 8..payload_end,
    });
    Some(start + 8..payload_end)
}

fn icmp_type_name(t: u8) -> &'static str {
    match t {
        0 => "Echo (ping) reply",
        3 => "Destination unreachable",
        4 => "Source quench",
        5 => "Redirect",
        8 => "Echo (ping) request",
        9 => "Router advertisement",
        10 => "Router solicitation",
        11 => "Time-to-live exceeded",
        12 => "Parameter problem",
        13 => "Timestamp request",
        14 => "Timestamp reply",
        _ => "unknown",
    }
}

fn unreachable_name(code: u8) -> &'static str {
    match code {
        0 => "Network unreachable",
        1 => "Host unreachable",
        2 => "Protocol unreachable",
        3 => "Port unreachable",
        4 => "Fragmentation needed",
        9 | 10 | 13 => "Administratively prohibited",
        _ => "",
    }
}

fn icmp(data: &[u8], start: usize, end: usize, d: &mut Dissection) -> Option<Range<usize>> {
    d.protocol = "ICMP";
    let title = "Internet Control Message Protocol";
    if end < start + 8 {
        return short(d, title, start, end);
    }
    let (kind, code) = (data[start], data[start + 1]);
    d.has("icmp");
    d.num("icmp.type", kind);
    d.num("icmp.code", code);
    let name = icmp_type_name(kind);
    let mut fields = vec![
        Field::new("Type", format!("{kind} ({name})"), start..start + 1),
        Field::new("Code", code.to_string(), start + 1..start + 2),
        Field::new(
            "Checksum",
            hex16(u16_at(data, start + 2)?),
            start + 2..start + 4,
        ),
    ];
    d.info = name.to_string();
    match kind {
        0 | 8 | 13 | 14 => {
            let (id, seq) = (u16_at(data, start + 4)?, u16_at(data, start + 6)?);
            fields.push(Field::new(
                "Identifier",
                format!("{id} ({})", hex16(id)),
                start + 4..start + 6,
            ));
            fields.push(Field::new(
                "Sequence number",
                seq.to_string(),
                start + 6..start + 8,
            ));
            d.info = format!("{name} id={} seq={seq}", hex16(id));
        }
        3 => {
            let reason = unreachable_name(code);
            if !reason.is_empty() {
                d.info = format!("{name} ({reason})");
            }
        }
        5 => {
            if let Some(gw) = data.get(start + 4..start + 8) {
                let gw = Ipv4Addr::new(gw[0], gw[1], gw[2], gw[3]);
                fields.push(Field::new(
                    "Gateway address",
                    gw.to_string(),
                    start + 4..start + 8,
                ));
            }
        }
        _ => {}
    }
    d.add(title.into(), start..start + 8, fields);
    Some(start + 8..end)
}

fn icmpv6_type_name(t: u8) -> &'static str {
    match t {
        1 => "Destination unreachable",
        2 => "Packet too big",
        3 => "Time exceeded",
        4 => "Parameter problem",
        128 => "Echo (ping) request",
        129 => "Echo (ping) reply",
        130 => "Multicast listener query",
        131 => "Multicast listener report",
        132 => "Multicast listener done",
        133 => "Router solicitation",
        134 => "Router advertisement",
        135 => "Neighbor solicitation",
        136 => "Neighbor advertisement",
        137 => "Redirect",
        143 => "Multicast listener report v2",
        _ => "unknown",
    }
}

fn icmpv6(data: &[u8], start: usize, end: usize, d: &mut Dissection) -> Option<Range<usize>> {
    d.protocol = "ICMPv6";
    let title = "Internet Control Message Protocol v6";
    if end < start + 8 {
        return short(d, title, start, end);
    }
    let (kind, code) = (data[start], data[start + 1]);
    d.has("icmpv6");
    d.num("icmpv6.type", kind);
    d.num("icmpv6.code", code);
    let name = icmpv6_type_name(kind);
    let mut fields = vec![
        Field::new("Type", format!("{kind} ({name})"), start..start + 1),
        Field::new("Code", code.to_string(), start + 1..start + 2),
        Field::new(
            "Checksum",
            hex16(u16_at(data, start + 2)?),
            start + 2..start + 4,
        ),
    ];
    d.info = name.to_string();
    let mut header_end = start + 8;
    match kind {
        128 | 129 => {
            let (id, seq) = (u16_at(data, start + 4)?, u16_at(data, start + 6)?);
            fields.push(Field::new(
                "Identifier",
                format!("{id} ({})", hex16(id)),
                start + 4..start + 6,
            ));
            fields.push(Field::new(
                "Sequence number",
                seq.to_string(),
                start + 6..start + 8,
            ));
            d.info = format!("{name} id={} seq={seq}", hex16(id));
        }
        2 => {
            let mtu = u32_at(data, start + 4)?;
            fields.push(Field::new("MTU", mtu.to_string(), start + 4..start + 8));
            d.info = format!("{name} (MTU {mtu})");
        }
        135..=137 => {
            if let Some(b) = data.get(start + 8..start + 24)
                && start + 24 <= end
            {
                let b: [u8; 16] = b.try_into().ok()?;
                let target = Ipv6Addr::from(b);
                fields.push(Field::new(
                    "Target address",
                    target.to_string(),
                    start + 8..start + 24,
                ));
                d.info = match kind {
                    135 => format!("{name} for {target}"),
                    136 => format!("{name} {target}"),
                    _ => format!("{name} to {target}"),
                };
                header_end = start + 24;
            }
        }
        _ => {}
    }
    d.add(title.into(), start..header_end, fields);
    Some(header_end..end)
}

fn igmp(data: &[u8], start: usize, end: usize, d: &mut Dissection) {
    d.protocol = "IGMP";
    let title = "Internet Group Management Protocol";
    if end < start + 8 {
        short(d, title, start, end);
        return;
    }
    let kind = data[start];
    d.has("igmp");
    d.num("igmp.type", kind);
    let name = match kind {
        0x11 => "Membership query",
        0x12 => "Membership report v1",
        0x16 => "Membership report v2",
        0x17 => "Leave group",
        0x22 => "Membership report v3",
        _ => "unknown",
    };
    let mut fields = vec![
        Field::new("Type", format!("{name} (0x{kind:02x})"), start..start + 1),
        Field::new(
            "Checksum",
            u16_at(data, start + 2).map(hex16).unwrap_or_default(),
            start + 2..start + 4,
        ),
    ];
    d.info = name.to_string();
    if kind == 0x22 {
        let records = u16_at(data, start + 6).unwrap_or(0);
        fields.push(Field::new(
            "Group records",
            records.to_string(),
            start + 6..start + 8,
        ));
    } else if let Some(g) = data.get(start + 4..start + 8) {
        let group = Ipv4Addr::new(g[0], g[1], g[2], g[3]);
        fields.push(Field::new(
            "Group address",
            group.to_string(),
            start + 4..start + 8,
        ));
        if !group.is_unspecified() {
            d.info = format!("{name} {group}");
        }
    }
    d.add(title.into(), start..end, fields);
}
