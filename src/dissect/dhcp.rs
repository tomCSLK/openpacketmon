//! DHCP (RFC 2131, 2132) and the message type of `DHCPv6` (RFC 8415)

use std::fmt::Write;
use std::ops::Range;

use super::{Dissection, Field, Value, hex_bytes, ipv4_at, mac, text, u8_at, u16_at, u32_at};

const MAGIC_COOKIE: u32 = 0x6382_5363;

fn message_type(t: u8) -> &'static str {
    match t {
        1 => "Discover",
        2 => "Offer",
        3 => "Request",
        4 => "Decline",
        5 => "ACK",
        6 => "NAK",
        7 => "Release",
        8 => "Inform",
        _ => "Unknown",
    }
}

fn option_name(code: u8) -> &'static str {
    match code {
        1 => "Subnet mask",
        3 => "Router",
        6 => "Domain name server",
        12 => "Host name",
        15 => "Domain name",
        28 => "Broadcast address",
        42 => "NTP servers",
        50 => "Requested IP address",
        51 => "IP address lease time",
        53 => "DHCP message type",
        54 => "DHCP server identifier",
        55 => "Parameter request list",
        57 => "Maximum DHCP message size",
        58 => "Renewal time",
        59 => "Rebinding time",
        60 => "Vendor class identifier",
        61 => "Client identifier",
        81 => "Client fully qualified domain name",
        119 => "Domain search",
        121 => "Classless static route",
        _ => "Option",
    }
}

/// Value of an option as text
fn option_value(code: u8, v: &[u8]) -> String {
    let addresses = || {
        (0..v.len() / 4)
            .filter_map(|i| ipv4_at(v, i * 4).map(|a| a.to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match code {
        53 if v.len() == 1 => format!("{} ({})", message_type(v[0]), v[0]),
        1 | 3 | 6 | 28 | 42 | 50 | 54 if v.len() >= 4 && v.len().is_multiple_of(4) => addresses(),
        51 | 58 | 59 if v.len() == 4 => format!("{} s", u32_at(v, 0).unwrap_or_default()),
        57 if v.len() == 2 => u16_at(v, 0).unwrap_or_default().to_string(),
        12 | 15 | 60 => text(v),
        55 => v.iter().map(u8::to_string).collect::<Vec<_>>().join(", "),
        61 if v.len() == 7 && v[0] == 1 => format!("{} (Ethernet)", mac(&v[1..])),
        81 if v.len() > 3 => text(&v[3..]),
        _ => format!("{} bytes: {}", v.len(), hex_bytes(&v[..v.len().min(32)])),
    }
}

/// DHCP over UDP 67/68
#[allow(clippy::too_many_lines)] // the fixed header, then the options
pub(super) fn dissect(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let base = payload.start;
    let m = &data[payload.clone()];
    if m.len() < 240 || u32_at(m, 236) != Some(MAGIC_COOKIE) {
        return false;
    }
    let span = |range: Range<usize>| base + range.start..base + range.end;
    let op = m[0];
    let hlen = usize::from(m[2]).min(16);
    let xid = u32_at(m, 4).unwrap_or_default();
    let flags = u16_at(m, 10).unwrap_or_default();
    let addr = |at: usize| ipv4_at(m, at).map(|a| a.to_string()).unwrap_or_default();
    let client_mac = mac(&m[28..28 + hlen]);
    let mut fields = vec![
        Field::new(
            "Message type",
            match op {
                1 => "Boot Request (1)".to_owned(),
                2 => "Boot Reply (2)".to_owned(),
                o => o.to_string(),
            },
            span(0..1),
        ),
        Field::new("Hardware type", m[1].to_string(), span(1..2)),
        Field::new("Hardware address length", m[2].to_string(), span(2..3)),
        Field::new("Hops", m[3].to_string(), span(3..4)),
        Field::new("Transaction ID", format!("0x{xid:08x}"), span(4..8)),
        Field::new(
            "Seconds elapsed",
            u16_at(m, 8).unwrap_or_default().to_string(),
            span(8..10),
        ),
        Field::new(
            "Flags",
            if flags & 0x8000 == 0 {
                "Unicast"
            } else {
                "Broadcast"
            },
            span(10..12),
        ),
        Field::new("Client IP address", addr(12), span(12..16)),
        Field::new("Your (client) IP address", addr(16), span(16..20)),
        Field::new("Next server IP address", addr(20), span(20..24)),
        Field::new("Relay agent IP address", addr(24), span(24..28)),
        Field::new(
            "Client MAC address",
            client_mac.clone(),
            span(28..28 + hlen),
        ),
    ];
    let sname = m[44..108].split(|b| *b == 0).next().unwrap_or_default();
    if !sname.is_empty() {
        fields.push(Field::new("Server host name", text(sname), span(44..108)));
    }
    let file = m[108..236].split(|b| *b == 0).next().unwrap_or_default();
    if !file.is_empty() {
        fields.push(Field::new("Boot file name", text(file), span(108..236)));
    }
    fields.push(Field::new("Magic cookie", "DHCP", span(236..240)));

    let mut kind = None;
    let mut hostname = None;
    let mut requested = None;
    let mut i = 240;
    while let Some(code) = u8_at(m, i) {
        match code {
            0 => {
                i += 1;
                continue;
            }
            255 => {
                fields.push(Field::new("Option", "(255) End", span(i..i + 1)));
                break;
            }
            _ => {}
        }
        let Some(len) = u8_at(m, i + 1).map(usize::from) else {
            break;
        };
        let Some(v) = m.get(i + 2..i + 2 + len) else {
            fields.push(Field::new(
                "[Incomplete]",
                "the option ends after the packet",
                span(i..m.len()),
            ));
            break;
        };
        let value = option_value(code, v);
        match code {
            53 => kind = v.first().copied(),
            12 => hostname = Some(text(v)),
            50 => requested = ipv4_at(v, 0),
            60 => d.text("dhcp.vendor_class", text(v)),
            _ => {}
        }
        d.num("dhcp.option", code);
        fields.push(Field::new(
            "Option",
            format!("({code}) {}: {value}", option_name(code)),
            span(i..i + 2 + len),
        ));
        i += 2 + len;
    }

    let name = kind.map_or("BOOTP", message_type);
    d.protocol = if kind.is_some() { "DHCP" } else { "BOOTP" };
    d.has("dhcp");
    if let Some(k) = kind {
        d.num("dhcp.type", k);
    }
    if let Some(h) = &hostname {
        d.text("dhcp.hostname", h.clone());
    }
    if let Some(ip) = requested {
        d.ip("dhcp.requested_ip", ip);
    }
    if let Some(ip) = ipv4_at(m, 16) {
        d.ip("dhcp.your_ip", ip);
    }
    if hlen == 6
        && let Ok(bytes) = m[28..34].try_into()
    {
        d.set("dhcp.client_mac", Value::Mac(bytes));
    }
    d.add(
        format!(
            "Dynamic Host Configuration Protocol ({})",
            kind.map_or("BOOTP", message_type)
        ),
        payload,
        fields,
    );
    let mut info = format!("DHCP {name} - Transaction ID 0x{xid:08x}");
    if let Some(h) = hostname {
        let _ = write!(info, " - {h}");
    }
    if let Some(ip) = requested {
        let _ = write!(info, " (wants {ip})");
    }
    let yiaddr = addr(16);
    if op == 2 && yiaddr != "0.0.0.0" {
        let _ = write!(info, " → {yiaddr}");
    }
    d.info = info;
    true
}

/// `DHCPv6` over UDP 546/547: the message type and the transaction ID
pub(super) fn dissect_v6(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let p = payload.start;
    let m = &data[payload.clone()];
    if m.len() < 4 {
        return false;
    }
    let kind = match m[0] {
        1 => "Solicit",
        2 => "Advertise",
        3 => "Request",
        4 => "Confirm",
        5 => "Renew",
        6 => "Rebind",
        7 => "Reply",
        8 => "Release",
        9 => "Decline",
        10 => "Reconfigure",
        11 => "Information-request",
        12 => "Relay-forw",
        13 => "Relay-repl",
        _ => "Unknown",
    };
    let xid = u32::from_be_bytes([0, m[1], m[2], m[3]]);
    d.protocol = "DHCPv6";
    d.has("dhcpv6");
    d.num("dhcpv6.type", m[0]);
    d.add(
        format!("DHCPv6 ({kind})"),
        payload,
        vec![
            Field::new("Message type", format!("{kind} ({})", m[0]), p..p + 1),
            Field::new("Transaction ID", format!("0x{xid:06x}"), p + 1..p + 4),
            Field::new(
                "Options",
                format!("{} bytes", m.len() - 4),
                p + 4..p + m.len(),
            ),
        ],
    );
    d.info = format!("{kind} XID: 0x{xid:06x}");
    true
}
