//! Network layer: IPv4, IPv6 with extension headers, ARP

use std::net::{Ipv4Addr, Ipv6Addr};

use super::link::ether_type_name;
use super::{Dissection, Field, Value, hex16, mac, u8_at, u16_at, u32_at};

/// What a network header carries: IP protocol number, start and end of its payload
pub(super) type Transport = (u8, usize, usize);

/// Name of an IP protocol number
pub(super) fn ip_protocol_name(p: u8) -> &'static str {
    match p {
        0 => "IPv6 Hop-by-Hop Options",
        1 => "ICMP",
        2 => "IGMP",
        4 => "IPv4 (encapsulated)",
        6 => "TCP",
        17 => "UDP",
        41 => "IPv6 (encapsulated)",
        43 => "IPv6 Routing",
        44 => "IPv6 Fragment",
        47 => "GRE",
        50 => "ESP",
        51 => "AH",
        58 => "ICMPv6",
        59 => "No Next Header",
        60 => "IPv6 Destination Options",
        89 => "OSPF",
        103 => "PIM",
        112 => "VRRP",
        132 => "SCTP",
        _ => "unknown",
    }
}

fn ipv4(data: &[u8], at: usize) -> Option<Ipv4Addr> {
    let b: [u8; 4] = data.get(at..at + 4)?.try_into().ok()?;
    Some(Ipv4Addr::from(b))
}

fn ipv6(data: &[u8], at: usize) -> Option<Ipv6Addr> {
    let b: [u8; 16] = data.get(at..at + 16)?.try_into().ok()?;
    Some(Ipv6Addr::from(b))
}

/// The layer after an `EtherType`
pub(super) fn by_ether_type(
    data: &[u8],
    at: usize,
    ether_type: u16,
    d: &mut Dissection,
) -> Option<Transport> {
    match ether_type {
        0x0800 | 0x86dd => ip(data, at, d),
        0x0806 => {
            arp(data, at, d);
            None
        }
        other => {
            let name = ether_type_name(other);
            if name != "unknown" {
                d.protocol = name;
            }
            d.info = format!("{name} ({})", hex16(other));
            let len = data.len().saturating_sub(at);
            if len > 0 {
                d.add(
                    format!("Data ({len} bytes)"),
                    at..data.len(),
                    vec![Field::new("Length", len.to_string(), at..data.len())],
                );
            }
            None
        }
    }
}

/// An IP header, IPv4 or IPv6 by its version
pub(super) fn ip(data: &[u8], at: usize, d: &mut Dissection) -> Option<Transport> {
    match u8_at(data, at).map(|b| b >> 4) {
        Some(4) => ip_v4(data, at, d),
        Some(6) => ip_v6(data, at, d),
        Some(v) => {
            d.protocol = "IP";
            d.info = format!("IP version {v} is not known");
            None
        }
        None => {
            d.cut(at);
            None
        }
    }
}

/// Whether the one's complement sum of the header is correct
fn ipv4_checksum_ok(header: &[u8]) -> bool {
    let mut sum: u32 = header
        .chunks(2)
        .map(|c| u32::from(u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])))
        .sum();
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    sum == 0xffff
}

#[allow(clippy::too_many_lines)] // one field after the other
fn ip_v4(data: &[u8], at: usize, d: &mut Dissection) -> Option<Transport> {
    d.protocol = "IPv4";
    let Some(header) = data.get(at..at + 20) else {
        d.add(
            "Internet Protocol Version 4".into(),
            at..data.len(),
            Vec::new(),
        );
        d.cut(at);
        return None;
    };
    let ihl = usize::from(header[0] & 0x0f) * 4;
    let total = usize::from(u16_at(header, 2)?);
    let id = u16_at(header, 4)?;
    let flags_frag = u16_at(header, 6)?;
    let (df, mf, frag_offset) = (
        flags_frag & 0x4000 != 0,
        flags_frag & 0x2000 != 0,
        usize::from(flags_frag & 0x1fff) * 8,
    );
    let (ttl, protocol) = (header[8], header[9]);
    let (src, dst) = (ipv4(header, 12)?, ipv4(header, 16)?);
    d.has("ip");
    d.ip("ip.src", src);
    d.ip("ip.dst", dst);
    d.num("ip.ttl", ttl);
    d.num("ip.proto", protocol);
    d.num("ip.len", u64::try_from(total).unwrap_or(0));
    d.num("ip.id", id);
    d.src = src.to_string();
    d.dst = dst.to_string();
    let checksum = u16_at(header, 10)?;
    let checksum_state = match data.get(at..at + ihl) {
        Some(h) if ihl >= 20 => {
            if ipv4_checksum_ok(h) {
                "correct"
            } else {
                "incorrect (often computed by the adapter when sending)"
            }
        }
        _ => "unverified",
    };
    let mut fields = vec![
        Field::new("Version", "4", at..at + 1),
        Field::new("Header length", format!("{ihl} bytes"), at..at + 1),
        Field::new(
            "Differentiated services",
            format!("DSCP {}, ECN {}", header[1] >> 2, header[1] & 0x03),
            at + 1..at + 2,
        ),
        Field::new("Total length", total.to_string(), at + 2..at + 4),
        Field::new(
            "Identification",
            format!("{} ({id})", hex16(id)),
            at + 4..at + 6,
        ),
        Field::new(
            "Flags",
            match (df, mf) {
                (true, true) => "Don't fragment, More fragments",
                (true, false) => "Don't fragment",
                (false, true) => "More fragments",
                (false, false) => "none",
            },
            at + 6..at + 7,
        )
        .with_children(vec![
            Field::new("Don't fragment", set(df), at + 6..at + 7),
            Field::new("More fragments", set(mf), at + 6..at + 7),
        ]),
        Field::new("Fragment offset", frag_offset.to_string(), at + 6..at + 8),
        Field::new("Time to live", ttl.to_string(), at + 8..at + 9),
        Field::new(
            "Protocol",
            format!("{} ({protocol})", ip_protocol_name(protocol)),
            at + 9..at + 10,
        ),
        Field::new(
            "Header checksum",
            format!("{} [{checksum_state}]", hex16(checksum)),
            at + 10..at + 12,
        ),
        Field::new("Source address", src.to_string(), at + 12..at + 16),
        Field::new("Destination address", dst.to_string(), at + 16..at + 20),
    ];
    if ihl > 20 {
        fields.push(Field::new(
            "Options",
            format!("{} bytes", ihl - 20),
            at + 20..at + ihl,
        ));
    }
    d.add(
        format!("Internet Protocol Version 4, Src: {src}, Dst: {dst}"),
        at..at + ihl.max(20),
        fields,
    );
    if ihl < 20 {
        d.info = "IPv4 header with an invalid length".into();
        return None;
    }
    if data.len() < at + ihl {
        d.cut(data.len());
        return None;
    }
    if frag_offset > 0 {
        d.info = format!(
            "Fragmented IP protocol ({}, offset {frag_offset}, id {})",
            ip_protocol_name(protocol),
            hex16(id)
        );
        return None;
    }
    // the payload ends with the IP packet, not with the frame (Ethernet padding)
    let end = if total >= ihl { at + total } else { data.len() };
    Some((protocol, at + ihl, end))
}

fn set(on: bool) -> &'static str {
    if on { "Set" } else { "Not set" }
}

#[allow(clippy::too_many_lines)] // the header fields and the extension headers
fn ip_v6(data: &[u8], at: usize, d: &mut Dissection) -> Option<Transport> {
    d.protocol = "IPv6";
    let Some(header) = data.get(at..at + 40) else {
        d.add(
            "Internet Protocol Version 6".into(),
            at..data.len(),
            Vec::new(),
        );
        d.cut(at);
        return None;
    };
    let first = u32_at(header, 0)?;
    let payload_len = usize::from(u16_at(header, 4)?);
    let mut next = header[6];
    let (src, dst) = (ipv6(header, 8)?, ipv6(header, 24)?);
    d.has("ipv6");
    for (key, addr) in [
        ("ip.src", src),
        ("ipv6.src", src),
        ("ip.dst", dst),
        ("ipv6.dst", dst),
    ] {
        d.ip(key, addr);
    }
    d.num("ip.ttl", header[7]);
    d.src = src.to_string();
    d.dst = dst.to_string();
    d.add(
        format!("Internet Protocol Version 6, Src: {src}, Dst: {dst}"),
        at..at + 40,
        vec![
            Field::new("Version", "6", at..at + 1),
            Field::new(
                "Traffic class",
                format!("0x{:02x}", (first >> 20) & 0xff),
                at..at + 2,
            ),
            Field::new(
                "Flow label",
                format!("0x{:05x}", first & 0x000f_ffff),
                at + 1..at + 4,
            ),
            Field::new("Payload length", payload_len.to_string(), at + 4..at + 6),
            Field::new(
                "Next header",
                format!("{} ({next})", ip_protocol_name(next)),
                at + 6..at + 7,
            ),
            Field::new("Hop limit", header[7].to_string(), at + 7..at + 8),
            Field::new("Source address", src.to_string(), at + 8..at + 24),
            Field::new("Destination address", dst.to_string(), at + 24..at + 40),
        ],
    );
    let end = at + 40 + payload_len;
    let mut offset = at + 40;
    // extension headers
    loop {
        let len = match next {
            0 | 43 | 60 => u8_at(data, offset + 1).map(|l| (usize::from(l) + 1) * 8),
            44 => Some(8),
            51 => u8_at(data, offset + 1).map(|l| (usize::from(l) + 2) * 4),
            _ => break,
        };
        let (Some(len), Some(following)) = (len, u8_at(data, offset)) else {
            d.cut(offset);
            return None;
        };
        let mut fields = vec![Field::new(
            "Next header",
            format!("{} ({following})", ip_protocol_name(following)),
            offset..offset + 1,
        )];
        if next == 44 {
            let frag = u16_at(data, offset + 2).unwrap_or(0);
            fields.push(Field::new(
                "Fragment offset",
                (usize::from(frag >> 3) * 8).to_string(),
                offset + 2..offset + 4,
            ));
            fields.push(Field::new(
                "More fragments",
                set(frag & 1 != 0),
                offset + 3..offset + 4,
            ));
            if frag >> 3 != 0 {
                d.add(ip_protocol_name(next).into(), offset..offset + len, fields);
                d.info = format!("Fragmented IPv6 ({})", ip_protocol_name(following));
                return None;
            }
        } else {
            fields.push(Field::new(
                "Length",
                format!("{len} bytes"),
                offset + 1..offset + 2,
            ));
        }
        if offset + len > data.len() {
            d.add(ip_protocol_name(next).into(), offset..data.len(), fields);
            d.cut(data.len());
            return None;
        }
        d.add(ip_protocol_name(next).into(), offset..offset + len, fields);
        next = following;
        offset += len;
    }
    d.num("ip.proto", next);
    Some((next, offset, end))
}

#[allow(clippy::too_many_lines)] // the fields, the filter values and the info line
fn arp(data: &[u8], at: usize, d: &mut Dissection) {
    d.protocol = "ARP";
    d.has("arp");
    let header = |i| u16_at(data, at + i);
    let (Some(htype), Some(ptype), Some(hlen), Some(plen), Some(op)) = (
        header(0),
        header(2),
        u8_at(data, at + 4),
        u8_at(data, at + 5),
        header(6),
    ) else {
        d.add(
            "Address Resolution Protocol".into(),
            at..data.len(),
            Vec::new(),
        );
        d.cut(at);
        return;
    };
    let (hlen, plen) = (usize::from(hlen), usize::from(plen));
    let sha = at + 8;
    let spa = sha + hlen;
    let tha = spa + plen;
    let tpa = tha + hlen;
    let end = tpa + plen;
    let operation = match op {
        1 => "request",
        2 => "reply",
        3 => "RARP request",
        4 => "RARP reply",
        _ => "unknown",
    };
    let mut fields = vec![
        Field::new(
            "Hardware type",
            if htype == 1 {
                "Ethernet (1)".into()
            } else {
                htype.to_string()
            },
            at..at + 2,
        ),
        Field::new(
            "Protocol type",
            format!("{} ({})", ether_type_name(ptype), hex16(ptype)),
            at + 2..at + 4,
        ),
        Field::new("Hardware size", hlen.to_string(), at + 4..at + 5),
        Field::new("Protocol size", plen.to_string(), at + 5..at + 6),
        Field::new("Opcode", format!("{operation} ({op})"), at + 6..at + 8),
    ];
    if data.len() < end {
        d.add(
            format!("Address Resolution Protocol ({operation})"),
            at..data.len(),
            fields,
        );
        d.cut(data.len());
        return;
    }
    let paddr = |i: usize| {
        if plen == 4 {
            ipv4(data, i).map_or_else(String::new, |a| a.to_string())
        } else {
            mac(&data[i..i + plen])
        }
    };
    let (s_mac, s_ip, t_mac, t_ip) = (
        mac(&data[sha..sha + hlen]),
        paddr(spa),
        mac(&data[tha..tha + hlen]),
        paddr(tpa),
    );
    d.num("arp.opcode", op);
    if hlen == 6 {
        for (key, at) in [("arp.src.mac", sha), ("arp.dst.mac", tha)] {
            if let Ok(bytes) = data[at..at + 6].try_into() {
                d.set(key, Value::Mac(bytes));
            }
        }
    }
    if plen == 4 {
        for (key, at) in [("arp.src.ip", spa), ("arp.dst.ip", tpa)] {
            if let Some(addr) = ipv4(data, at) {
                d.ip(key, addr);
            }
        }
    }
    fields.push(Field::new("Sender MAC address", s_mac.clone(), sha..spa));
    fields.push(Field::new("Sender IP address", s_ip.clone(), spa..tha));
    fields.push(Field::new("Target MAC address", t_mac, tha..tpa));
    fields.push(Field::new("Target IP address", t_ip.clone(), tpa..end));
    d.info = match op {
        1 if s_ip == t_ip => format!("Gratuitous ARP for {s_ip}"),
        1 if s_ip == "0.0.0.0" => format!("Who has {t_ip}? (ARP probe)"),
        1 => format!("Who has {t_ip}? Tell {s_ip}"),
        2 => format!("{s_ip} is at {s_mac}"),
        _ => format!("ARP {operation}"),
    };
    d.add(
        format!("Address Resolution Protocol ({operation})"),
        at..end,
        fields,
    );
}
