//! Dissection of a single packet into its layers and fields, for the packet details and the
//! columns of the packet list (protocol, addresses, info).
//!
//! Written for openpacketmon from the protocol specifications (RFCs, IEEE 802); every field
//! records the bytes it comes from, so the hex dump can mark them. Field and protocol names are
//! the usual English terms, as in other packet analyzers, in every language of the interface.

mod app;
mod dhcp;
mod dns;
mod link;
mod misc;
mod net;
mod text;
mod tls;
mod transport;

use std::fmt::Write;
use std::ops::Range;

use sniffnet_packet_parser::LinkType;

/// A value of a layer, with the bytes it was read from
pub struct Field {
    pub name: &'static str,
    pub value: String,
    pub range: Range<usize>,
    /// Details of the value (the single TCP flags, for example)
    pub children: Vec<Field>,
}

impl Field {
    pub(crate) fn new(name: &'static str, value: impl Into<String>, range: Range<usize>) -> Self {
        Self {
            name,
            value: value.into(),
            range,
            children: Vec::new(),
        }
    }

    pub(crate) fn with_children(mut self, children: Vec<Field>) -> Self {
        self.children = children;
        self
    }
}

/// A protocol header (or the payload) of the packet
pub struct Layer {
    /// Title of the layer, with a short summary (`Transmission Control Protocol, Src Port: …`)
    pub title: String,
    pub range: Range<usize>,
    pub fields: Vec<Field>,
}

/// The dissected packet
pub struct Dissection {
    pub layers: Vec<Layer>,
    /// The highest protocol found, for the packet list
    pub protocol: &'static str,
    /// Short description of the packet, for the packet list
    pub info: String,
    /// Source and destination shown in the packet list (IP addresses, MAC addresses without IP)
    pub src: String,
    pub dst: String,
    /// The captured bytes end before the packet does (headers only, or a short snap length)
    pub truncated: bool,
    /// A message of the application continues in the next TCP segment (a TLS Client Hello with
    /// large key shares); [`dissect_with`] completes it with the following segments
    pub followup: Option<Followup>,
    /// Ports and payload of TCP or UDP
    transport: Option<TransportInfo>,
    /// Values for the display filter (`ip.src`, `dns.qry.name` …), a key may occur several times
    pub values: Vec<(&'static str, Value)>,
    /// The protocols found, by their filter names (`eth`, `ip`, `tcp`, `dns` …)
    pub protocols: Vec<&'static str>,
}

/// A value of a packet for the display filter
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Num(u64),
    Ip(std::net::IpAddr),
    Text(String),
    Mac([u8; 6]),
}

/// Where a message split over TCP segments continues
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Followup {
    /// Sequence number of the next segment
    pub next_seq: u32,
    /// Bytes still missing
    pub missing: usize,
}

/// Ports and payload of a TCP segment or UDP datagram
#[derive(Clone, Debug)]
struct TransportInfo {
    tcp: bool,
    sport: u16,
    dport: u16,
    /// Sequence number (TCP)
    seq: u32,
    payload: Range<usize>,
}

impl TransportInfo {
    fn has_port(&self, port: u16) -> bool {
        self.sport == port || self.dport == port
    }
}

impl Dissection {
    fn new() -> Self {
        Self {
            layers: Vec::new(),
            protocol: "",
            info: String::new(),
            src: String::new(),
            dst: String::new(),
            truncated: false,
            followup: None,
            transport: None,
            values: Vec::new(),
            protocols: Vec::new(),
        }
    }

    /// Adds a value for the display filter
    fn set(&mut self, key: &'static str, value: Value) {
        self.values.push((key, value));
    }

    fn num(&mut self, key: &'static str, value: impl Into<u64>) {
        self.set(key, Value::Num(value.into()));
    }

    fn text(&mut self, key: &'static str, value: impl Into<String>) {
        self.set(key, Value::Text(value.into()));
    }

    fn ip(&mut self, key: &'static str, value: impl Into<std::net::IpAddr>) {
        self.set(key, Value::Ip(value.into()));
    }

    /// Notes a protocol for the display filter
    fn has(&mut self, protocol: &'static str) {
        if !self.protocols.contains(&protocol) {
            self.protocols.push(protocol);
        }
    }

    fn add(&mut self, title: String, range: Range<usize>, fields: Vec<Field>) {
        self.layers.push(Layer {
            title,
            range,
            fields,
        });
    }

    /// Marks the packet as cut off and notes it in the last layer
    fn cut(&mut self, at: usize) {
        self.truncated = true;
        if let Some(layer) = self.layers.last_mut() {
            layer.fields.push(Field::new(
                "[Truncated]",
                "the captured bytes end here",
                at..at,
            ));
        }
    }
}

/// What follows a link layer header
pub(crate) enum Next {
    /// An `EtherType` value
    EtherType(u16),
    /// An IP header, IPv4 or IPv6 by its version
    Ip,
}

/// Dissects the captured bytes of a packet
pub fn dissect(data: &[u8], link_type: LinkType) -> Dissection {
    dissect_with(data, link_type, &[])
}

/// Dissects a packet whose TCP payload continues with `more` (the payloads of the following
/// segments, see [`Dissection::followup`]); fields in these bytes lie beyond the packet's bytes
pub fn dissect_with(data: &[u8], link_type: LinkType, more: &[u8]) -> Dissection {
    layers(data, link_type, more, true)
}

/// The layers of a packet, up to the transport layer only without `app`
fn layers(data: &[u8], link_type: LinkType, more: &[u8], app: bool) -> Dissection {
    let mut d = Dissection::new();
    let next = match link_type {
        LinkType::Ethernet(_) | LinkType::Unsupported(_) => link::ethernet(data, &mut d),
        LinkType::LinuxSll(_) => link::linux_sll(data, &mut d, false),
        LinkType::LinuxSll2(_) => link::linux_sll(data, &mut d, true),
        LinkType::Null(_) | LinkType::Loop(_) => link::null(data, &mut d),
        LinkType::RawIp(_) | LinkType::IPv4(_) | LinkType::IPv6(_) => Some((Next::Ip, 0)),
    };
    let Some((next, offset)) = next else {
        return d;
    };
    let transport = match next {
        Next::EtherType(ether_type) => net::by_ether_type(data, offset, ether_type, &mut d),
        Next::Ip => net::ip(data, offset, &mut d),
    };
    if let Some((protocol, start, end)) = transport {
        // bytes of the packet beyond the captured ones (headers only, or a short snap length)
        let missing = end.saturating_sub(data.len());
        if missing > 0 {
            d.truncated = true;
        }
        let end = end.min(data.len());
        if start > end {
            // a header claims more bytes than were captured
            d.truncated = true;
            return d;
        }
        let payload = transport::dissect(data, start, end, protocol, &mut d);
        if missing > 0 {
            // the payload length on the wire, which the TCP analysis needs
            for (key, value) in &mut d.values {
                if let ("tcp.len", Value::Num(len)) = (*key, value) {
                    *len += missing as u64;
                }
            }
        }
        if let Some(range) = payload
            && !range.is_empty()
            && app
        {
            let handled = match d.transport.clone() {
                Some(t) if t.tcp && !more.is_empty() => {
                    let mut joined = data[..range.end].to_vec();
                    joined.extend_from_slice(more);
                    app::dissect(&joined, range.start..joined.len(), &t, &mut d)
                }
                Some(t) => app::dissect(data, range.clone(), &t, &mut d),
                None => false,
            };
            if !handled {
                let len = range.len();
                d.add(
                    format!("Data ({len} bytes)"),
                    range.clone(),
                    vec![Field::new("Length", len.to_string(), range)],
                );
            }
        }
    }
    d
}

/// The number of a DNS record type by its name (`AAAA` → 28), for the display filter
pub fn dns_type_number(name: &str) -> Option<u16> {
    (1..=300).find(|t| dns::type_name(*t).eq_ignore_ascii_case(name))
}

/// Sequence number and payload of a TCP segment, for joining the segments of a message;
/// `None` for a segment whose payload was not captured completely
pub fn tcp_payload(data: &[u8], link_type: LinkType) -> Option<(u32, Range<usize>)> {
    transport_payload(data, link_type)
        .filter(|p| p.tcp)
        .map(|p| (p.seq, p.range))
}

/// The payload of a TCP segment or UDP datagram
pub struct Payload {
    pub tcp: bool,
    /// Sequence number (TCP)
    pub seq: u32,
    pub range: Range<usize>,
}

/// The payload of a TCP segment or UDP datagram, for following a stream; `None` for other
/// packets and for those whose payload was not captured completely
pub fn transport_payload(data: &[u8], link_type: LinkType) -> Option<Payload> {
    // the application layer has nothing to say here, and a stream goes through many packets
    let d = layers(data, link_type, &[], false);
    if d.truncated {
        return None;
    }
    d.transport.map(|t| Payload {
        tcp: t.tcp,
        seq: t.seq,
        range: t.payload,
    })
}

/// The captured part of the payload of a TCP segment or UDP datagram, also when the rest was not
/// captured; `None` for other packets
pub fn captured_payload(data: &[u8], link_type: LinkType) -> Option<Payload> {
    layers(data, link_type, &[], false)
        .transport
        .map(|t| Payload {
            tcp: t.tcp,
            seq: t.seq,
            range: t.payload,
        })
}

// ---- reading -------------------------------------------------------------------------------

/// Printable text of bytes (other bytes as `·`), for names and headers read from the wire
fn text(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| {
            if b.is_ascii_graphic() || *b == b' ' {
                char::from(*b)
            } else {
                '·'
            }
        })
        .collect()
}

fn ipv4_at(data: &[u8], i: usize) -> Option<std::net::Ipv4Addr> {
    let b: [u8; 4] = data.get(i..i + 4)?.try_into().ok()?;
    Some(std::net::Ipv4Addr::from(b))
}

fn ipv6_at(data: &[u8], i: usize) -> Option<std::net::Ipv6Addr> {
    let b: [u8; 16] = data.get(i..i + 16)?.try_into().ok()?;
    Some(std::net::Ipv6Addr::from(b))
}

fn u8_at(data: &[u8], i: usize) -> Option<u8> {
    data.get(i).copied()
}

fn u16_at(data: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*data.get(i)?, *data.get(i + 1)?]))
}

fn u32_at(data: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(i..i + 4)?.try_into().ok()?))
}

fn mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Bytes as hex digits without separators
fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// `0x0800`
fn hex16(v: u16) -> String {
    format!("0x{v:04x}")
}

#[cfg(test)]
pub(crate) mod tests;
