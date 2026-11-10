//! The names the display filter knows: protocols and fields with the keys of the dissection
//! they read (several for `ip.addr`, `tcp.port`, `domain`)

use super::Packet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Num,
    Ip,
    Text,
    Mac,
    /// A duration, stored in microseconds and written in seconds (`0.25`) or with a unit
    /// (`250ms`, `800us`)
    Time,
}

/// Values of the frame itself, not of a protocol
#[derive(Debug, Clone, Copy)]
pub(super) enum FrameField {
    Number,
    Len,
    Captured,
}

impl FrameField {
    pub(super) fn value(self, p: &Packet) -> u64 {
        match self {
            FrameField::Number => p.number,
            FrameField::Len => u64::from(p.len),
            FrameField::Captured => u64::try_from(p.captured).unwrap_or(u64::MAX),
        }
    }
}

/// Where a name of the filter gets its values
#[derive(Debug, Clone, Copy)]
pub(super) enum Source {
    Protocol(&'static str),
    Keys(&'static [&'static str], Kind),
    Frame(FrameField),
}

impl Source {
    pub(super) fn kind(self) -> Option<Kind> {
        match self {
            Source::Protocol(_) => None,
            Source::Keys(_, kind) => Some(kind),
            Source::Frame(_) => Some(Kind::Num),
        }
    }
}

/// The protocols, by the names the dissection notes
pub const PROTOCOLS: [&str; 27] = [
    "eth", "vlan", "sll", "loopback", "arp", "ip", "ipv6", "tcp", "udp", "icmp", "icmpv6", "igmp",
    "dns", "mdns", "llmnr", "dhcp", "dhcpv6", "tls", "http", "ssdp", "ntp", "quic", "sip", "sdp",
    "rtp", "rtcp", "frame",
];

use Kind::{Ip, Mac, Num, Text, Time};

/// The fields: name, the keys of the dissection it reads, kind of value
pub const FIELDS: &[(&str, &[&str], Kind)] = &[
    ("eth.addr", &["eth.src", "eth.dst"], Mac),
    ("eth.src", &["eth.src"], Mac),
    ("eth.dst", &["eth.dst"], Mac),
    ("eth.type", &["eth.type"], Num),
    ("vlan.id", &["vlan.id"], Num),
    ("arp.opcode", &["arp.opcode"], Num),
    ("arp.src.ip", &["arp.src.ip"], Ip),
    ("arp.dst.ip", &["arp.dst.ip"], Ip),
    ("arp.src.mac", &["arp.src.mac"], Mac),
    ("arp.dst.mac", &["arp.dst.mac"], Mac),
    ("ip.addr", &["ip.src", "ip.dst"], Ip),
    ("ip.src", &["ip.src"], Ip),
    ("ip.dst", &["ip.dst"], Ip),
    ("ip.ttl", &["ip.ttl"], Num),
    ("ip.proto", &["ip.proto"], Num),
    ("ip.len", &["ip.len"], Num),
    ("ip.id", &["ip.id"], Num),
    ("ipv6.addr", &["ipv6.src", "ipv6.dst"], Ip),
    ("ipv6.src", &["ipv6.src"], Ip),
    ("ipv6.dst", &["ipv6.dst"], Ip),
    (
        "port",
        &["tcp.srcport", "tcp.dstport", "udp.srcport", "udp.dstport"],
        Num,
    ),
    ("tcp.port", &["tcp.srcport", "tcp.dstport"], Num),
    ("tcp.srcport", &["tcp.srcport"], Num),
    ("tcp.dstport", &["tcp.dstport"], Num),
    ("tcp.seq", &["tcp.seq"], Num),
    ("tcp.ack", &["tcp.ack"], Num),
    ("tcp.len", &["tcp.len"], Num),
    ("tcp.window", &["tcp.window"], Num),
    ("tcp.flags.syn", &["tcp.flags.syn"], Num),
    ("tcp.flags.ack", &["tcp.flags.ack"], Num),
    ("tcp.flags.fin", &["tcp.flags.fin"], Num),
    ("tcp.flags.rst", &["tcp.flags.rst"], Num),
    ("tcp.flags.reset", &["tcp.flags.rst"], Num),
    ("tcp.flags.psh", &["tcp.flags.psh"], Num),
    ("tcp.flags.push", &["tcp.flags.psh"], Num),
    ("tcp.flags.urg", &["tcp.flags.urg"], Num),
    ("tcp.flags.ece", &["tcp.flags.ece"], Num),
    ("tcp.flags.cwr", &["tcp.flags.cwr"], Num),
    ("tcp.analysis.flags", &["tcp.analysis.flags"], Num),
    (
        "tcp.analysis.retransmission",
        &["tcp.analysis.retransmission"],
        Num,
    ),
    (
        "tcp.analysis.out_of_order",
        &["tcp.analysis.out_of_order"],
        Num,
    ),
    (
        "tcp.analysis.lost_segment",
        &["tcp.analysis.lost_segment"],
        Num,
    ),
    (
        "tcp.analysis.duplicate_ack",
        &["tcp.analysis.duplicate_ack"],
        Num,
    ),
    (
        "tcp.analysis.zero_window",
        &["tcp.analysis.zero_window"],
        Num,
    ),
    ("tcp.analysis.keep_alive", &["tcp.analysis.keep_alive"], Num),
    (
        "tcp.analysis.keep_alive_ack",
        &["tcp.analysis.keep_alive_ack"],
        Num,
    ),
    (
        "tcp.analysis.window_update",
        &["tcp.analysis.window_update"],
        Num,
    ),
    ("tcp.analysis.ack_rtt", &["tcp.analysis.ack_rtt"], Time),
    ("tcp.analysis.acks_frame", &["tcp.analysis.acks_frame"], Num),
    ("tcp.analysis.handshake", &["tcp.analysis.handshake"], Time),
    ("udp.port", &["udp.srcport", "udp.dstport"], Num),
    ("udp.srcport", &["udp.srcport"], Num),
    ("udp.dstport", &["udp.dstport"], Num),
    ("udp.len", &["udp.len"], Num),
    ("icmp.type", &["icmp.type"], Num),
    ("icmp.code", &["icmp.code"], Num),
    ("icmpv6.type", &["icmpv6.type"], Num),
    ("icmpv6.code", &["icmpv6.code"], Num),
    ("igmp.type", &["igmp.type"], Num),
    ("domain", &["dns.qry.name", "tls.sni", "http.host"], Text),
    ("dns.id", &["dns.id"], Num),
    ("dns.flags.response", &["dns.flags.response"], Num),
    ("dns.flags.rcode", &["dns.flags.rcode"], Num),
    ("dns.count.answers", &["dns.count.answers"], Num),
    ("dns.qry.name", &["dns.qry.name"], Text),
    ("dns.qry.type", &["dns.qry.type"], Num),
    ("dns.resp.name", &["dns.resp.name"], Text),
    ("dns.resp.type", &["dns.resp.type"], Num),
    ("dns.a", &["dns.a"], Ip),
    ("dns.aaaa", &["dns.aaaa"], Ip),
    ("dns.cname", &["dns.cname"], Text),
    ("dns.ptr", &["dns.ptr"], Text),
    ("dns.time", &["dns.time"], Time),
    ("dns.response_to", &["dns.response_to"], Num),
    ("dhcp.type", &["dhcp.type"], Num),
    ("dhcp.hostname", &["dhcp.hostname"], Text),
    ("dhcp.requested_ip", &["dhcp.requested_ip"], Ip),
    ("dhcp.your_ip", &["dhcp.your_ip"], Ip),
    ("dhcp.client_mac", &["dhcp.client_mac"], Mac),
    ("dhcp.vendor_class", &["dhcp.vendor_class"], Text),
    ("dhcp.option", &["dhcp.option"], Num),
    ("dhcpv6.type", &["dhcpv6.type"], Num),
    ("tls.sni", &["tls.sni"], Text),
    ("tls.alpn", &["tls.alpn"], Text),
    ("tls.version", &["tls.version"], Num),
    ("tls.record.version", &["tls.record.version"], Num),
    ("tls.handshake.type", &["tls.handshake.type"], Num),
    ("tls.alert", &["tls.alert"], Num),
    ("http.request.method", &["http.request.method"], Text),
    ("http.request.uri", &["http.request.uri"], Text),
    ("http.response.code", &["http.response.code"], Num),
    ("http.host", &["http.host"], Text),
    ("http.user_agent", &["http.user_agent"], Text),
    ("http.server", &["http.server"], Text),
    ("http.content_type", &["http.content_type"], Text),
    ("ssdp.nt", &["ssdp.nt"], Text),
    ("ssdp.st", &["ssdp.st"], Text),
    ("ssdp.server", &["ssdp.server"], Text),
    ("ssdp.user_agent", &["ssdp.user_agent"], Text),
    ("ssdp.location", &["ssdp.location"], Text),
    ("ntp.version", &["ntp.version"], Num),
    ("ntp.mode", &["ntp.mode"], Num),
    ("ntp.stratum", &["ntp.stratum"], Num),
    ("quic.version", &["quic.version"], Num),
    ("quic.type", &["quic.type"], Text),
    ("quic.dcid", &["quic.dcid"], Text),
    ("sip.method", &["sip.method"], Text),
    ("sip.status_code", &["sip.status_code"], Num),
    ("sip.call_id", &["sip.call_id"], Text),
    ("sip.from", &["sip.from"], Text),
    ("sip.to", &["sip.to"], Text),
    ("sip.cseq.seq", &["sip.cseq.seq"], Num),
    ("sip.cseq.method", &["sip.cseq.method"], Text),
    ("rtp.ssrc", &["rtp.ssrc"], Text),
    ("rtp.seq", &["rtp.seq"], Num),
    ("rtp.p_type", &["rtp.p_type"], Num),
    ("rtp.timestamp", &["rtp.timestamp"], Num),
    ("rtp.marker", &["rtp.marker"], Num),
    ("rtcp.pt", &["rtcp.pt"], Num),
    ("voip.call_id", &["voip.call_id"], Text),
];

/// The source of a name (lowercase)
pub(super) fn lookup(name: &str) -> Option<Source> {
    match name {
        "frame" | "frame.number" => return Some(Source::Frame(FrameField::Number)),
        "frame.len" => return Some(Source::Frame(FrameField::Len)),
        "frame.cap_len" => return Some(Source::Frame(FrameField::Captured)),
        "ipv4" => return Some(Source::Protocol("ip")),
        _ => {}
    }
    if let Some(p) = PROTOCOLS.iter().find(|p| **p == name) {
        return Some(Source::Protocol(p));
    }
    FIELDS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, keys, kind)| Source::Keys(keys, *kind))
}

/// The protocols of a connection
pub const CONNECTION_PROTOCOLS: [&str; 8] =
    ["ip", "ipv6", "tcp", "udp", "icmp", "icmpv6", "igmp", "arp"];

/// The fields of a connection: name, the keys it reads, kind of value
pub const CONNECTION_FIELDS: &[(&str, &[&str], Kind)] = &[
    ("ip.addr", &["ip.src", "ip.dst"], Ip),
    ("ip.src", &["ip.src"], Ip),
    ("ip.dst", &["ip.dst"], Ip),
    ("remote", &["remote"], Ip),
    ("proto", &["proto"], Text),
    (
        "port",
        &["tcp.srcport", "tcp.dstport", "udp.srcport", "udp.dstport"],
        Num,
    ),
    ("srcport", &["tcp.srcport", "udp.srcport"], Num),
    ("dstport", &["tcp.dstport", "udp.dstport"], Num),
    ("tcp.port", &["tcp.srcport", "tcp.dstport"], Num),
    ("tcp.srcport", &["tcp.srcport"], Num),
    ("tcp.dstport", &["tcp.dstport"], Num),
    ("udp.port", &["udp.srcport", "udp.dstport"], Num),
    ("udp.srcport", &["udp.srcport"], Num),
    ("udp.dstport", &["udp.dstport"], Num),
    ("eth.addr", &["eth.src", "eth.dst"], Mac),
    ("eth.src", &["eth.src"], Mac),
    ("eth.dst", &["eth.dst"], Mac),
    ("vlan.id", &["vlan.id"], Num),
    ("direction", &["direction"], Text),
    ("service", &["service"], Text),
    ("domain", &["domain"], Text),
    ("as", &["as.name"], Text),
    ("as.name", &["as.name"], Text),
    ("as.number", &["as.number"], Num),
    ("country", &["country"], Text),
    ("program", &["program", "program.path"], Text),
    ("program.name", &["program"], Text),
    ("program.path", &["program.path"], Text),
    ("exporter", &["exporter"], Ip),
    ("packets", &["packets"], Num),
    ("bytes", &["bytes"], Num),
    ("favorite", &["favorite"], Num),
];

/// The source of a name of the connection filter (lowercase)
pub(super) fn lookup_connection(name: &str) -> Option<Source> {
    let name = match name {
        "ipv4" => "ip",
        "icmpv4" => "icmp",
        "addr" | "ipv6.addr" => "ip.addr",
        "ipv6.src" => "ip.src",
        "ipv6.dst" => "ip.dst",
        "protocol" => "proto",
        other => other,
    };
    if let Some(p) = CONNECTION_PROTOCOLS.iter().find(|p| **p == name) {
        return Some(Source::Protocol(p));
    }
    CONNECTION_FIELDS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, keys, kind)| Source::Keys(keys, *kind))
}
