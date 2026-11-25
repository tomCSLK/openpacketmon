//! The structure of a packet (`Aufbau` of the draft, `.dg`): the header of each layer in rows of
//! 32 bits, as the RFCs draw them, with every field at its bits.
//!
//! Written for openpacketmon. The widths of the fields come from the specifications (IEEE 802.3
//! and 802.1Q, RFC 826 ARP, RFC 791 IPv4, RFC 8200 IPv6, RFC 9293 TCP, RFC 768 UDP, RFC 792 ICMP,
//! RFC 4443 `ICMPv6`, RFC 3376 IGMP, RFC 1035 DNS, RFC 4795 LLMNR, RFC 8446 TLS, RFC 9000 QUIC,
//! RFC 5905 NTP, RFC 2131 DHCP, RFC 8415 `DHCPv6`, RFC 3550 RTP and RTCP); nothing comes from
//! other packet analyzers. The layers come from the dissection, the bits from the bytes. Fields
//! longer than 16 bytes and the rest of a layer (its payload) take one row with broken edges,
//! which keeps the drawing short; text protocols are one block.

use std::fmt::Write;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ops::Range;

use crate::dissect::Layer;
use crate::view::i18n::Lang;

/// The bits of a row
pub const ROW_BITS: u32 = 32;
/// Fields longer than this take one broken row
const LONG: u32 = 128;

/// How the value of a field is written
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fmt {
    Dec,
    /// A signed byte (the precision of NTP)
    Signed,
    Hex,
    Bin,
    Mac,
    Ip4,
    Ip6,
    /// A length in words of 32 bits (`5 (20 bytes)`)
    Words,
    /// Bits hidden by the header protection of QUIC
    Protected,
    /// Only the size
    Size,
}

/// A field as the specification draws it
struct Spec {
    name: &'static str,
    bits: u32,
    fmt: Fmt,
    /// A short name for narrow fields (`IHL`, `TTL`)
    short: Option<&'static str>,
    /// What the name stands for, in the tooltip
    note: Option<&'static str>,
    /// The rest of the layer, drawn as one broken row
    rest: bool,
}

fn f(name: &'static str, bits: u32, fmt: Fmt) -> Spec {
    Spec {
        name,
        bits,
        fmt,
        short: None,
        note: None,
        rest: false,
    }
}

impl Spec {
    fn short(mut self, short: &'static str) -> Self {
        self.short = Some(short);
        self
    }

    fn note(mut self, note: &'static str) -> Self {
        self.note = Some(note);
        self
    }
}

/// A single bit with a short name
fn bit(name: &'static str, short: &'static str) -> Spec {
    f(name, 1, Fmt::Dec).short(short)
}

/// A part of a row: the bits `from..to` of row `row`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub row: usize,
    pub from: u32,
    pub to: u32,
    /// A row for a long field or the rest of the layer, with broken edges
    pub broken: bool,
}

/// A field of the drawing
#[derive(Clone, Debug)]
pub struct DiagramField {
    pub name: &'static str,
    pub short: Option<&'static str>,
    pub value: String,
    /// Name, meaning, bits and value
    pub tip: String,
    /// Its bytes in the packet
    pub range: Range<usize>,
    pub segments: Vec<Segment>,
    /// The segment with the label: the widest
    pub main: usize,
    /// Drawn as one broken row
    pub block: bool,
}

impl DiagramField {
    /// The text of the field: its name or its value
    pub fn label(&self, values: bool) -> String {
        match (self.block, values) {
            (true, false) => format!("{} ({})", self.name, self.value),
            (_, true) => self.value.clone(),
            (false, false) => self.name.to_owned(),
        }
    }

    /// The text for assistive technology
    pub fn line(&self) -> String {
        format!("{}: {}", self.name, self.value)
    }
}

/// A layer of the drawing
#[derive(Clone, Debug)]
pub struct DiagramLayer {
    /// The index of the layer in the details
    pub layer: usize,
    pub name: String,
    /// `20 bytes`
    pub size: String,
    pub rows: usize,
    pub fields: Vec<DiagramField>,
}

/// A line of the frame: horizontal on the edge `at` (a row) from bit `from` to `to`, or vertical
/// on the edge `at` (a bit) from row `from` to `to`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub at: usize,
    pub from: usize,
    pub to: usize,
}

impl DiagramLayer {
    /// The lines between fields and around them: where two neighbouring cells belong to
    /// different fields (or one to none)
    pub fn lines(&self) -> (Vec<Line>, Vec<Line>) {
        const COLS: usize = ROW_BITS as usize;
        // with a border of empty cells all round: cell (r, c) at [r + 1][c + 1]
        let mut own = vec![[None; COLS + 2]; self.rows + 2];
        for (k, field) in self.fields.iter().enumerate() {
            for s in &field.segments {
                for c in s.from as usize..s.to as usize {
                    own[s.row + 1][c + 1] = Some(k);
                }
            }
        }
        let (mut across, mut down) = (Vec::new(), Vec::new());
        // above row r: between the cells of rows r - 1 and r
        for r in 0..=self.rows {
            join(r, COLS, |c| own[r][c + 1] != own[r + 1][c + 1], &mut across);
        }
        // left of bit c: between the cells of bits c - 1 and c
        for c in 0..=COLS {
            join(
                c,
                self.rows,
                |r| own[r + 1][c] != own[r + 1][c + 1],
                &mut down,
            );
        }
        (across, down)
    }
}

/// Joins the edges to draw (`draw(i)` for `i` in `0..len`) on the edge `at` into lines
fn join(at: usize, len: usize, draw: impl Fn(usize) -> bool, lines: &mut Vec<Line>) {
    let mut start = None;
    for i in 0..=len {
        match (i < len && draw(i), start) {
            (true, None) => start = Some(i),
            (false, Some(from)) => {
                lines.push(Line { at, from, to: i });
                start = None;
            }
            _ => {}
        }
    }
}

/// The drawings of the layers of a packet; `bytes` are those the ranges of the layers point
/// into (with the following segments of an assembled message)
pub fn diagram(t: Lang, layers: &[Layer], bytes: &[u8]) -> Vec<DiagramLayer> {
    layers
        .iter()
        .enumerate()
        .filter_map(|(i, layer)| draw_layer(t, i, layer, bytes))
        .collect()
}

fn draw_layer(t: Lang, index: usize, layer: &Layer, bytes: &[u8]) -> Option<DiagramLayer> {
    let s = layer.range.start;
    let end = layer.range.end.min(bytes.len());
    if end <= s || layer.title.starts_with("Frame ") || layer.title.starts_with('[') {
        return None;
    }
    let len = end - s;
    let specs = specs(&layer.title, &bytes[s..end]);
    let total = u32::try_from(len.saturating_mul(8)).unwrap_or(u32::MAX);
    let mut fields = Vec::new();
    let (mut bit, mut row, mut col) = (0_u32, 0_usize, 0_u32);
    for spec in specs {
        let bits = if spec.rest {
            total.saturating_sub(bit)
        } else {
            spec.bits
        };
        if bits == 0 || bit >= total {
            continue;
        }
        let cut = bit + bits > total;
        let bits = bits.min(total - bit);
        let block = spec.rest || bits > LONG;
        let segments = place(bits, block, &mut row, &mut col);
        let main = segments
            .iter()
            .enumerate()
            .max_by_key(|(i, sg)| (sg.broken, sg.to - sg.from, std::cmp::Reverse(*i)))
            .map_or(0, |(i, _)| i);
        let first = s * 8 + bit as usize;
        let range = first / 8..(first + bits as usize).div_ceil(8);
        let mut value = value(t, &spec, block, bytes, first, bits);
        if cut {
            let _ = write!(value, " ({})", t.get("packet-diagram-cut"));
        }
        let place = if bits == 1 {
            t.get1("packet-diagram-bit", "from", f64::from(bit))
        } else {
            t.fmt(
                "packet-diagram-bits",
                &[
                    ("from", f64::from(bit).into()),
                    ("to", f64::from(bit + bits - 1).into()),
                    ("count", f64::from(bits).into()),
                ],
            )
        };
        let mut tip = spec.name.to_owned();
        if let Some(note) = spec.note {
            let _ = write!(tip, " – {note}");
        }
        let _ = write!(
            tip,
            "\n{place}\n{}",
            t.get1("packet-diagram-value", "value", value.clone())
        );
        fields.push(DiagramField {
            name: spec.name,
            short: spec.short,
            value,
            tip,
            range,
            segments,
            main,
            block,
        });
        bit += bits;
    }
    if fields.is_empty() {
        return None;
    }
    Some(DiagramLayer {
        layer: index,
        name: layer
            .title
            .split(", ")
            .next()
            .unwrap_or_default()
            .to_owned(),
        size: size(t, len),
        rows: row + usize::from(col > 0),
        fields,
    })
}

/// The segments of a field of `bits` that starts at bit `col` of row `row`; moves both to its end
fn place(bits: u32, block: bool, row: &mut usize, col: &mut u32) -> Vec<Segment> {
    let mut segments = Vec::new();
    if block {
        // the rest of the row, then one broken row
        if *col > 0 {
            segments.push(Segment {
                row: *row,
                from: *col,
                to: ROW_BITS,
                broken: false,
            });
            *row += 1;
            *col = 0;
        }
        segments.push(Segment {
            row: *row,
            from: 0,
            to: ROW_BITS,
            broken: true,
        });
        *row += 1;
        return segments;
    }
    let mut left = bits;
    while left > 0 {
        let take = (ROW_BITS - *col).min(left);
        segments.push(Segment {
            row: *row,
            from: *col,
            to: *col + take,
            broken: false,
        });
        *col += take;
        left -= take;
        if *col == ROW_BITS {
            *row += 1;
            *col = 0;
        }
    }
    segments
}

/// `20 bytes`
fn size(t: Lang, bytes: usize) -> String {
    t.get1(
        "packet-bytes-count",
        "count",
        f64::from(u32::try_from(bytes).unwrap_or(u32::MAX)),
    )
}

/// `n` bits from bit `first` of `bytes` (missing bytes count as zero)
fn num(bytes: &[u8], first: usize, n: u32) -> u64 {
    (0..n as usize).fold(0, |v, k| {
        let i = first + k;
        let b = bytes.get(i / 8).copied().unwrap_or(0);
        (v << 1) | u64::from((b >> (7 - i % 8)) & 1)
    })
}

fn value(t: Lang, spec: &Spec, block: bool, bytes: &[u8], first: usize, bits: u32) -> String {
    let at = first / 8;
    let whole = bytes
        .get(at..(at + bits as usize / 8).min(bytes.len()))
        .unwrap_or_default();
    let hex = |b: &[u8], sep: &str| {
        b.iter()
            .map(|x| format!("{x:02x}"))
            .collect::<Vec<_>>()
            .join(sep)
    };
    if block || spec.fmt == Fmt::Size {
        return size(t, bits.div_ceil(8) as usize);
    }
    match spec.fmt {
        Fmt::Mac => hex(&whole[..whole.len().min(6)], ":"),
        Fmt::Ip4 => <[u8; 4]>::try_from(whole)
            .map_or_else(|_| hex(whole, " "), |b| Ipv4Addr::from(b).to_string()),
        Fmt::Ip6 => <[u8; 16]>::try_from(whole)
            .map_or_else(|_| hex(whole, " "), |b| Ipv6Addr::from(b).to_string()),
        Fmt::Protected => t.get("packet-diagram-protected"),
        Fmt::Hex if bits > 64 => format!("0x{}", hex(whole, "")),
        Fmt::Hex => format!(
            "0x{:0width$x}",
            num(bytes, first, bits),
            width = bits.div_ceil(4) as usize
        ),
        Fmt::Bin => format!(
            "{:0width$b}",
            num(bytes, first, bits),
            width = bits as usize
        ),
        Fmt::Words => {
            let words = num(bytes, first, bits);
            format!(
                "{words} ({})",
                size(t, usize::try_from(words * 4).unwrap_or(usize::MAX))
            )
        }
        Fmt::Signed => whole
            .first()
            .map_or(0, |b| i8::from_be_bytes([*b]))
            .to_string(),
        Fmt::Dec | Fmt::Size => num(bytes, first, bits).to_string(),
    }
}

/// The rest of a layer from byte `from`
fn rest(name: &'static str) -> Spec {
    Spec {
        rest: true,
        ..f(name, 0, Fmt::Size)
    }
}

/// The fields of a layer by its title; `b` are its bytes
#[allow(clippy::too_many_lines)] // one table for each protocol
fn specs(title: &str, b: &[u8]) -> Vec<Spec> {
    use Fmt::{Bin, Dec, Hex, Ip4, Ip6, Mac, Protected, Signed, Size, Words};
    let byte = |i: usize| b.get(i).copied().unwrap_or(0);
    let starts = |p: &str| title.starts_with(p);
    if starts("Ethernet II") {
        vec![
            f("Destination", 48, Mac),
            f("Source", 48, Mac),
            f("Type", 16, Hex),
        ]
    } else if starts("IEEE 802.3 Ethernet") {
        vec![
            f("Destination", 48, Mac),
            f("Source", 48, Mac),
            f("Length", 16, Dec),
        ]
    } else if starts("802.1Q Virtual LAN") {
        vec![
            f("Priority", 3, Dec)
                .short("PCP")
                .note("Priority Code Point"),
            bit("Drop Eligible Indicator", "DEI"),
            f("VLAN ID", 12, Dec).short("VID"),
            f("Type", 16, Hex),
        ]
    } else if starts("Linux cooked capture v1") {
        vec![
            f("Packet Type", 16, Dec),
            f("Link-layer Address Type", 16, Dec),
            f("Address Length", 16, Dec),
            f("Source", 64, Hex),
            f("Protocol", 16, Hex),
        ]
    } else if starts("Linux cooked capture v2") {
        vec![
            f("Protocol", 16, Hex),
            f("Reserved", 16, Hex),
            f("Interface Index", 32, Dec),
            f("Link-layer Address Type", 16, Dec),
            f("Packet Type", 8, Dec),
            f("Address Length", 8, Dec),
            f("Source", 64, Hex),
        ]
    } else if starts("Null/Loopback") {
        vec![f("Address Family", 32, Hex)]
    } else if starts("Address Resolution Protocol") {
        let (hw, proto) = (u32::from(byte(4)) * 8, u32::from(byte(5)) * 8);
        let (hw_fmt, proto_fmt) = (
            if hw == 48 { Mac } else { Hex },
            if proto == 32 { Ip4 } else { Hex },
        );
        vec![
            f("Hardware Type", 16, Dec),
            f("Protocol Type", 16, Hex),
            f("Hardware Size", 8, Dec),
            f("Protocol Size", 8, Dec),
            f("Opcode", 16, Dec),
            f("Sender MAC Address", hw, hw_fmt),
            f("Sender IP Address", proto, proto_fmt),
            f("Target MAC Address", hw, hw_fmt),
            f("Target IP Address", proto, proto_fmt),
        ]
    } else if starts("Internet Protocol Version 4") {
        let ihl = u32::from(byte(0) & 0x0f);
        let mut v = vec![
            f("Version", 4, Dec),
            f("Header Length", 4, Words).short("IHL"),
            f("DSCP", 6, Dec).note("Differentiated Services Code Point"),
            f("ECN", 2, Bin).note("Explicit Congestion Notification"),
            f("Total Length", 16, Dec),
            f("Identification", 16, Hex),
            f("Flags", 3, Bin).note("Reserved, Don't Fragment, More Fragments"),
            f("Fragment Offset", 13, Dec),
            f("Time to Live", 8, Dec).short("TTL"),
            f("Protocol", 8, Dec),
            f("Header Checksum", 16, Hex),
            f("Source Address", 32, Ip4),
            f("Destination Address", 32, Ip4),
        ];
        if ihl > 5 {
            v.push(f("Options", (ihl - 5) * 32, Size));
        }
        v
    } else if starts("Internet Protocol Version 6") {
        vec![
            f("Version", 4, Dec),
            f("Traffic Class", 8, Hex),
            f("Flow Label", 20, Hex),
            f("Payload Length", 16, Dec),
            f("Next Header", 8, Dec),
            f("Hop Limit", 8, Dec),
            f("Source Address", 128, Ip6),
            f("Destination Address", 128, Ip6),
        ]
    } else if starts("IPv6 Fragment") {
        vec![
            f("Next Header", 8, Dec),
            f("Reserved", 8, Hex),
            f("Fragment Offset", 13, Dec),
            f("Reserved", 2, Bin).short("Res"),
            bit("More Fragments", "M"),
            f("Identification", 32, Hex),
        ]
    } else if starts("IPv6 Hop-by-Hop") || starts("IPv6 Routing") || starts("IPv6 Destination") {
        vec![
            f("Next Header", 8, Dec),
            f("Header Extension Length", 8, Dec).short("Hdr Ext Len"),
            rest("Options"),
        ]
    } else if starts("Transmission Control Protocol") {
        let offset = u32::from(byte(12) >> 4);
        let mut v = vec![
            f("Source Port", 16, Dec),
            f("Destination Port", 16, Dec),
            f("Sequence Number", 32, Dec),
            f("Acknowledgment Number", 32, Dec),
            f("Data Offset", 4, Words).short("Offset"),
            f("Reserved", 4, Bin).short("Res."),
            bit("CWR", "C").note("Congestion Window Reduced"),
            bit("ECE", "E").note("ECN-Echo"),
            bit("URG", "U").note("Urgent"),
            bit("ACK", "A").note("Acknowledgment"),
            bit("PSH", "P").note("Push"),
            bit("RST", "R").note("Reset"),
            bit("SYN", "S").note("Synchronize"),
            bit("FIN", "F").note("Finish"),
            f("Window", 16, Dec),
            f("Checksum", 16, Hex),
            f("Urgent Pointer", 16, Dec),
        ];
        if offset > 5 {
            v.push(f("Options", (offset - 5) * 32, Size));
        }
        v
    } else if starts("User Datagram Protocol") {
        vec![
            f("Source Port", 16, Dec),
            f("Destination Port", 16, Dec),
            f("Length", 16, Dec),
            f("Checksum", 16, Hex),
        ]
    } else if starts("Internet Control Message Protocol v6") {
        let mut v = vec![f("Type", 8, Dec), f("Code", 8, Dec), f("Checksum", 16, Hex)];
        match byte(0) {
            128 | 129 => {
                v.push(f("Identifier", 16, Hex));
                v.push(f("Sequence Number", 16, Dec));
            }
            2 => v.push(f("MTU", 32, Dec)),
            136 => {
                v.push(bit("Router", "R"));
                v.push(bit("Solicited", "S"));
                v.push(bit("Override", "O"));
                v.push(f("Reserved", 29, Hex));
                v.push(f("Target Address", 128, Ip6));
            }
            135 | 137 => {
                v.push(f("Reserved", 32, Hex));
                v.push(f("Target Address", 128, Ip6));
            }
            _ => v.push(f("Rest of Header", 32, Hex)),
        }
        v.push(rest("Data"));
        v
    } else if starts("Internet Control Message Protocol") {
        let mut v = vec![f("Type", 8, Dec), f("Code", 8, Dec), f("Checksum", 16, Hex)];
        match byte(0) {
            0 | 8 | 13 | 14 => {
                v.push(f("Identifier", 16, Hex));
                v.push(f("Sequence Number", 16, Dec));
            }
            5 => v.push(f("Gateway Address", 32, Ip4)),
            _ => v.push(f("Rest of Header", 32, Hex)),
        }
        v.push(rest("Data"));
        v
    } else if starts("Internet Group Management Protocol") {
        let mut v = vec![
            f("Type", 8, Hex),
            f("Max Resp Time", 8, Dec),
            f("Checksum", 16, Hex),
        ];
        if byte(0) == 0x22 {
            v.push(f("Reserved", 16, Hex));
            v.push(f("Number of Group Records", 16, Dec));
            v.push(rest("Group Records"));
        } else {
            v.push(f("Group Address", 32, Ip4));
            v.push(rest("Data"));
        }
        v
    } else if starts("Domain Name System") || starts("Multicast Domain Name System") {
        vec![
            f("Transaction ID", 16, Hex),
            bit("Response", "QR").note("Query (0) or Response (1)"),
            f("Opcode", 4, Dec),
            bit("Authoritative Answer", "AA"),
            bit("Truncated", "TC"),
            bit("Recursion Desired", "RD"),
            bit("Recursion Available", "RA"),
            bit("Z", "Z").note("Reserved"),
            bit("Authentic Data", "AD"),
            bit("Checking Disabled", "CD"),
            f("Rcode", 4, Dec).note("Response Code"),
            f("Questions", 16, Dec),
            f("Answer RRs", 16, Dec),
            f("Authority RRs", 16, Dec),
            f("Additional RRs", 16, Dec),
            rest("Queries and Answers"),
        ]
    } else if starts("Link-local Multicast Name Resolution") {
        vec![
            f("Transaction ID", 16, Hex),
            bit("Response", "QR").note("Query (0) or Response (1)"),
            f("Opcode", 4, Dec),
            bit("Conflict", "C"),
            bit("Truncated", "TC"),
            bit("Tentative", "T"),
            f("Reserved", 4, Bin).short("Z"),
            f("Rcode", 4, Dec).note("Response Code"),
            f("Questions", 16, Dec),
            f("Answer RRs", 16, Dec),
            f("Authority RRs", 16, Dec),
            f("Additional RRs", 16, Dec),
            rest("Queries and Answers"),
        ]
    } else if starts("Transport Layer Security (continuation data)") {
        vec![rest("Data")]
    } else if starts("Transport Layer Security") {
        let content = match byte(0) {
            20 => "Change Cipher Spec Message",
            21 => "Alert Message",
            22 => "Handshake",
            23 => "Encrypted Application Data",
            _ => "Data",
        };
        vec![
            f("Content Type", 8, Dec),
            f("Version", 16, Hex),
            f("Length", 16, Dec),
            rest(content),
        ]
    } else if starts("QUIC (short header)") {
        vec![
            bit("Header Form", "H").note("Long (1) or Short Header (0)"),
            bit("Fixed Bit", "F"),
            bit("Spin Bit", "S"),
            f("Protected Bits", 5, Protected)
                .note("Reserved Bits, Key Phase and Packet Number Length, hidden by the header protection"),
            rest("Destination Connection ID and Protected Payload"),
        ]
    } else if starts("QUIC (") {
        let version = num(b, 8, 32);
        let dcid = u32::from(byte(5)) * 8;
        let scid = u32::from(byte(6 + usize::from(byte(5)))) * 8;
        let mut v = vec![bit("Header Form", "H").note("Long (1) or Short Header (0)")];
        if version == 0 {
            v.push(f("Unused", 7, Bin));
        } else {
            v.push(bit("Fixed Bit", "F"));
            v.push(f("Long Packet Type", 2, Dec).short("Type"));
            if title.starts_with("QUIC (Retry)") {
                v.push(f("Unused", 4, Bin));
            } else {
                v.push(f("Type-Specific Bits", 4, Protected).note(
                    "Reserved Bits and Packet Number Length, hidden by the header protection",
                ));
            }
        }
        v.extend([
            f("Version", 32, Hex),
            f("Destination Connection ID Length", 8, Dec).short("DCID Len"),
            f("Destination Connection ID", dcid, Hex),
            f("Source Connection ID Length", 8, Dec).short("SCID Len"),
            f("Source Connection ID", scid, Hex),
        ]);
        v.push(rest(if version == 0 {
            "Supported Versions"
        } else if title.starts_with("QUIC (Retry)") {
            "Retry Token and Integrity Tag"
        } else {
            "Protected Payload"
        }));
        v
    } else if starts("Network Time Protocol") {
        vec![
            f("Leap Indicator", 2, Dec).short("LI"),
            f("Version", 3, Dec).short("VN"),
            f("Mode", 3, Dec),
            f("Stratum", 8, Dec),
            f("Poll", 8, Signed),
            f("Precision", 8, Signed),
            f("Root Delay", 32, Hex),
            f("Root Dispersion", 32, Hex),
            f("Reference ID", 32, Hex),
            f("Reference Timestamp", 64, Hex),
            f("Origin Timestamp", 64, Hex),
            f("Receive Timestamp", 64, Hex),
            f("Transmit Timestamp", 64, Hex),
            rest("Extensions"),
        ]
    } else if starts("Dynamic Host Configuration Protocol") {
        vec![
            f("Message Type", 8, Dec).short("Op"),
            f("Hardware Type", 8, Dec).short("HType"),
            f("Hardware Address Length", 8, Dec).short("HLen"),
            f("Hops", 8, Dec),
            f("Transaction ID", 32, Hex),
            f("Seconds", 16, Dec),
            f("Flags", 16, Hex),
            f("Client IP Address", 32, Ip4),
            f("Your IP Address", 32, Ip4),
            f("Server IP Address", 32, Ip4),
            f("Relay IP Address", 32, Ip4),
            f(
                "Client Hardware Address",
                128,
                if byte(2) == 6 { Mac } else { Hex },
            ),
            f("Server Host Name", 512, Size),
            f("Boot File Name", 1024, Size),
            f("Magic Cookie", 32, Hex),
            rest("Options"),
        ]
    } else if starts("DHCPv6") {
        vec![
            f("Message Type", 8, Dec),
            f("Transaction ID", 24, Hex),
            rest("Options"),
        ]
    } else if starts("Real-Time Transport Protocol") {
        let csrc = u32::from(byte(0) & 0x0f);
        let mut v = vec![
            f("Version", 2, Dec).short("V"),
            bit("Padding", "P"),
            bit("Extension", "X"),
            f("CSRC Count", 4, Dec).short("CC"),
            bit("Marker", "M"),
            f("Payload Type", 7, Dec).short("PT"),
            f("Sequence Number", 16, Dec),
            f("Timestamp", 32, Dec),
            f("SSRC", 32, Hex).note("Synchronization Source"),
        ];
        for _ in 0..csrc {
            v.push(f("CSRC", 32, Hex).note("Contributing Source"));
        }
        if byte(0) & 0x10 != 0 {
            let at = 12 + 4 * csrc as usize;
            let words = u32::try_from(num(b, (at + 2) * 8, 16)).unwrap_or(0);
            v.push(f("Extension Profile", 16, Hex));
            v.push(f("Extension Length", 16, Dec));
            v.push(f("Extension", words * 32, Size));
        }
        v.push(rest("Payload"));
        v
    } else if starts("Real-time Transport Control Protocol") {
        let count = byte(0) & 0x1f;
        let pt = byte(1);
        let mut v = vec![
            f("Version", 2, Dec).short("V"),
            bit("Padding", "P"),
            f(
                match pt {
                    200 | 201 => "Reception Report Count",
                    202 | 203 => "Source Count",
                    _ => "Count",
                },
                5,
                Dec,
            )
            .short(if pt == 200 || pt == 201 { "RC" } else { "SC" }),
            f("Packet Type", 8, Dec).short("PT"),
            f("Length", 16, Dec),
        ];
        match pt {
            200 | 201 => {
                v.push(f("SSRC of Sender", 32, Hex));
                if pt == 200 {
                    v.extend([
                        f("NTP Timestamp", 64, Hex),
                        f("RTP Timestamp", 32, Dec),
                        f("Sender's Packet Count", 32, Dec),
                        f("Sender's Octet Count", 32, Dec),
                    ]);
                }
                for _ in 0..count {
                    v.extend([
                        f("SSRC of Source", 32, Hex),
                        f("Fraction Lost", 8, Dec),
                        f("Cumulative Number of Packets Lost", 24, Dec),
                        f("Extended Highest Sequence Number", 32, Dec),
                        f("Interarrival Jitter", 32, Dec),
                        f("Last SR", 32, Hex).short("LSR"),
                        f("Delay since Last SR", 32, Dec).short("DLSR"),
                    ]);
                }
                v.push(rest("Profile-specific Extensions"));
            }
            202 => {
                v.push(f("SSRC", 32, Hex));
                v.push(rest("SDES Items"));
            }
            203 => {
                v.push(f("SSRC", 32, Hex));
                v.push(rest("More Sources and Reason"));
            }
            _ => v.push(rest("Data")),
        }
        v
    } else if starts("Hypertext Transfer Protocol")
        || starts("Session Initiation Protocol")
        || starts("Session Description Protocol")
        || starts("Simple Service Discovery Protocol")
    {
        vec![rest("Text")]
    } else if starts("Message body") {
        vec![rest("Message Body")]
    } else {
        vec![rest("Data")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(title: &str, range: Range<usize>) -> Layer {
        Layer {
            title: title.to_owned(),
            range,
            fields: Vec::new(),
        }
    }

    fn one(title: &str, bytes: &[u8]) -> DiagramLayer {
        let layers = [layer(title, 0..bytes.len())];
        diagram(Lang::En, &layers, bytes).remove(0)
    }

    fn field<'a>(d: &'a DiagramLayer, name: &str) -> &'a DiagramField {
        d.fields.iter().find(|f| f.name == name).unwrap()
    }

    /// An IPv4 header of 20 bytes from 192.0.2.1 to 198.51.100.7, TTL 64, UDP
    const IPV4: [u8; 20] = [
        0x45, 0x00, 0x00, 0x3c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x11, 0xb1, 0xe6, 192, 0, 2, 1, 198,
        51, 100, 7,
    ];

    #[test]
    fn ethernet_takes_three_and_a_half_rows() {
        let mut bytes = vec![0x00, 0x00, 0x5e, 0x00, 0x53, 0x01];
        bytes.extend([0x00, 0x00, 0x5e, 0x00, 0x53, 0x14, 0x08, 0x00]);
        let d = one("Ethernet II, Src: a, Dst: b", &bytes);
        assert_eq!(d.name, "Ethernet II");
        assert_eq!(d.size, "14 bytes");
        assert_eq!(d.rows, 4);
        let dst = field(&d, "Destination");
        assert_eq!(dst.value, "00:00:5e:00:53:01");
        assert_eq!(dst.range, 0..6);
        assert_eq!(
            dst.segments,
            [
                Segment {
                    row: 0,
                    from: 0,
                    to: 32,
                    broken: false
                },
                Segment {
                    row: 1,
                    from: 0,
                    to: 16,
                    broken: false
                },
            ]
        );
        assert_eq!(field(&d, "Type").value, "0x0800");
        assert_eq!(field(&d, "Type").segments[0].row, 3);
        // the step between the addresses: a line under the first half of row 1 only
        let (across, down) = d.lines();
        assert!(across.contains(&Line {
            at: 2,
            from: 0,
            to: 16
        }));
        assert!(across.contains(&Line {
            at: 1,
            from: 16,
            to: 32
        }));
        assert!(down.contains(&Line {
            at: 16,
            from: 1,
            to: 2
        }));
        // the outline: left edge all the way, right edge of the last row only to bit 16
        assert!(down.contains(&Line {
            at: 0,
            from: 0,
            to: 4
        }));
        assert!(down.contains(&Line {
            at: 16,
            from: 3,
            to: 4
        }));
        assert!(across.contains(&Line {
            at: 4,
            from: 0,
            to: 16
        }));
    }

    #[test]
    fn ipv4_has_its_fields_at_their_bits() {
        let d = one("Internet Protocol Version 4, Src: x", &IPV4);
        assert_eq!(d.rows, 5);
        let ihl = field(&d, "Header Length");
        assert_eq!(
            (ihl.value.as_str(), ihl.range.clone()),
            ("5 (20 bytes)", 0..1)
        );
        assert_eq!(ihl.short, Some("IHL"));
        assert_eq!(field(&d, "Version").range, 0..1);
        assert_eq!(field(&d, "Flags").value, "010");
        assert_eq!(field(&d, "Fragment Offset").range, 6..8);
        let ttl = field(&d, "Time to Live");
        assert_eq!((ttl.value.as_str(), ttl.range.clone()), ("64", 8..9));
        assert!(ttl.tip.contains("Bits 64–71 (8 bits)"), "{}", ttl.tip);
        assert_eq!(field(&d, "Source Address").value, "192.0.2.1");
        assert_eq!(field(&d, "Header Checksum").value, "0xb1e6");
    }

    #[test]
    fn ipv4_options_and_a_cut_header() {
        let mut bytes = IPV4.to_vec();
        bytes[0] = 0x46;
        bytes.extend([1, 1, 1, 0]);
        let d = one("Internet Protocol Version 4", &bytes);
        assert_eq!(field(&d, "Options").value, "4 bytes");
        assert_eq!(d.rows, 6);
        // captured up to the middle of the source address
        let d = one("Internet Protocol Version 4", &IPV4[..14]);
        let src = field(&d, "Source Address");
        assert_eq!(src.range, 12..14);
        assert!(src.value.ends_with("(truncated)"), "{}", src.value);
        assert!(!d.fields.iter().any(|f| f.name == "Destination Address"));
    }

    #[test]
    fn tcp_flags_are_single_bits_and_options_follow() {
        let mut tcp = vec![0x01, 0xbb, 0xc3, 0x50, 0, 0, 0, 1, 0, 0, 0, 2, 0x60, 0x12];
        tcp.extend([0xff, 0xff, 0, 0, 0, 0, 2, 4, 5, 0xb4]);
        let d = one("Transmission Control Protocol, Src Port: 443", &tcp);
        assert_eq!(field(&d, "Source Port").value, "443");
        assert_eq!(field(&d, "Data Offset").value, "6 (24 bytes)");
        for (name, value) in [("ACK", "1"), ("SYN", "1"), ("FIN", "0"), ("CWR", "0")] {
            let flag = field(&d, name);
            let one = &flag.segments[0];
            assert_eq!(
                (flag.value.as_str(), one.to - one.from),
                (value, 1),
                "{name}"
            );
        }
        assert_eq!(
            field(&d, "SYN").segments[0],
            Segment {
                row: 3,
                from: 14,
                to: 15,
                broken: false
            }
        );
        assert!(field(&d, "SYN").tip.contains("Bit 110"));
        assert_eq!(field(&d, "Options").range, 20..24);
        assert_eq!(d.rows, 6);
    }

    #[test]
    fn dns_flags_and_the_rest_in_one_broken_row() {
        let mut dns = vec![0x12, 0x34, 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0];
        dns.extend([0; 40]);
        let d = one("Domain Name System (response)", &dns);
        assert_eq!(field(&d, "Response").value, "1");
        assert_eq!(field(&d, "Recursion Desired").value, "1");
        assert_eq!(field(&d, "Recursion Available").value, "1");
        assert_eq!(field(&d, "Authoritative Answer").value, "0");
        let rest = field(&d, "Queries and Answers");
        assert!(rest.block);
        assert_eq!(rest.value, "40 bytes");
        assert_eq!(rest.label(false), "Queries and Answers (40 bytes)");
        assert_eq!(
            rest.segments,
            [Segment {
                row: 3,
                from: 0,
                to: 32,
                broken: true
            }]
        );
        assert_eq!(d.rows, 4);
    }

    #[test]
    fn rtp_and_a_block_after_half_a_row() {
        let mut rtp = vec![
            0x80, 0x08, 0x00, 0x2a, 0, 0, 0x03, 0x20, 0xde, 0xad, 0xbe, 0xef,
        ];
        rtp.extend([0xd5; 160]);
        let d = one("Real-Time Transport Protocol", &rtp);
        assert_eq!(field(&d, "Version").value, "2");
        assert_eq!(field(&d, "Payload Type").value, "8");
        assert_eq!(field(&d, "SSRC").value, "0xdeadbeef");
        assert_eq!(field(&d, "Payload").value, "160 bytes");
        assert_eq!(d.rows, 4);
    }

    #[test]
    fn a_block_after_half_a_row_fills_the_row_first() {
        let mut hop = vec![17, 0];
        hop.extend([1, 4, 0, 0, 0, 0]);
        let d = one("IPv6 Hop-by-Hop Options", &hop);
        let options = field(&d, "Options");
        assert_eq!(options.range, 2..8);
        assert_eq!(
            options.segments,
            [
                Segment {
                    row: 0,
                    from: 16,
                    to: 32,
                    broken: false
                },
                Segment {
                    row: 1,
                    from: 0,
                    to: 32,
                    broken: true
                },
            ]
        );
        assert_eq!(options.main, 1);
        assert_eq!(d.rows, 2);
    }

    #[test]
    fn text_and_unknown_layers_are_one_block_and_the_frame_none() {
        let text = one(
            "Hypertext Transfer Protocol (request)",
            b"GET / HTTP/1.1\r\n",
        );
        assert_eq!(text.fields.len(), 1);
        assert_eq!(text.fields[0].label(true), "16 bytes");
        let data = one("Data (3 bytes)", &[1, 2, 3]);
        assert_eq!(data.fields[0].name, "Data");
        let layers = [
            layer("Frame 1: 60 bytes on wire", 0..60),
            layer("Empty", 4..4),
        ];
        assert!(diagram(Lang::En, &layers, &[0; 60]).is_empty());
    }

    #[test]
    fn a_layer_beyond_the_captured_bytes_ends_with_them() {
        let layers = [layer("User Datagram Protocol", 0..8)];
        let d = diagram(Lang::De, &layers, &[0x13, 0x88, 0x00]);
        let d = &d[0];
        assert_eq!(d.size, "3 Bytes");
        assert_eq!(d.fields.len(), 2);
        assert!(d.fields[1].value.ends_with("(abgeschnitten)"));
    }

    #[test]
    fn the_label_follows_names_or_values() {
        let d = one(
            "User Datagram Protocol",
            &[0x13, 0x88, 0, 53, 0, 8, 0xab, 0xcd],
        );
        let port = field(&d, "Destination Port");
        assert_eq!(port.label(false), "Destination Port");
        assert_eq!(port.label(true), "53");
        assert_eq!(port.line(), "Destination Port: 53");
    }
}
