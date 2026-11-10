//! Random tests of the parts that read data from the network or from the user: the dissectors
//! with garbled packets of every protocol, the display and connection filters with random and
//! deeply nested texts. The access for AI assistants has its own in `src/mcp/` (`fuzz_…`), with
//! the helpers from here. `FUZZ_ITERATIONS` sets the number of rounds (a quick run by default;
//! a long one: `FUZZ_ITERATIONS=2000000 cargo test --release fuzz`).

use pcap::Linktype;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sniffnet_packet_parser::LinkType;

use crate::dissect::rtp::MediaNote;
use crate::dissect::tests::{
    ETH, client_hello, dhcp_message, dns_query, rtcp_report, rtp_bytes, sdp_offer, sip_message,
    tcp_packet, udp_between, udp_packet,
};
use crate::dissect::{Dissection, dissect, dissect_with, transport_payload};
use crate::engine::analysis::{Analysis, annotate};
use crate::engine::packets::StoredPacket;
use crate::filter::{CONNECTION_FIELDS, FIELDS, Filter, MAX_LEN, PROTOCOLS, Packet};
use crate::utils::types::timestamp::Timestamp;

pub fn iterations(default: usize) -> usize {
    std::env::var("FUZZ_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn rng() -> StdRng {
    let seed = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0x5eed);
    StdRng::seed_from_u64(seed)
}

/// Application payloads with their ports, the starting points of the mutations
pub fn samples() -> Vec<(u16, bool, Vec<u8>)> {
    let mut dns_answer = dns_query(7, 0x8180, "www.example.org", 1, 1);
    dns_answer[7] = 2; // two answers, both names compressed
    for ip in [[93, 184, 216, 34], [93, 184, 216, 35]] {
        dns_answer.extend_from_slice(&[0xc0, 12, 0, 1, 0, 1, 0, 0, 0x0e, 0x10, 0, 4]);
        dns_answer.extend_from_slice(&ip);
    }
    let mut dns_tcp = u16::try_from(dns_answer.len())
        .unwrap()
        .to_be_bytes()
        .to_vec();
    dns_tcp.extend_from_slice(&dns_answer);
    let mut dhcp_options = vec![53, 1, 3, 50, 4, 192, 168, 1, 20, 12, 6];
    dhcp_options.extend_from_slice(b"laptop");
    dhcp_options.extend_from_slice(&[55, 3, 1, 3, 6, 61, 7, 1, 2, 3, 4, 5, 6, 7]);
    let dhcpv6 = vec![
        1, 0x12, 0x34, 0x56, 0, 1, 0, 10, 0, 3, 0, 1, 2, 3, 4, 5, 6, 7, 0, 6, 0, 4, 0, 23, 0, 24,
    ];
    let mut ntp = vec![0x23, 0, 6, 0xec];
    ntp.resize(48, 0);
    let mut quic = vec![0xc3, 0, 0, 0, 1, 8];
    quic.extend_from_slice(&[0x11; 8]);
    quic.push(4);
    quic.extend_from_slice(&[0x22; 4]);
    quic.extend_from_slice(&[0, 0x40, 0x20]);
    quic.resize(64, 0x33);
    vec![
        (53, false, dns_query(1, 0x0100, "a.example.org", 28, 1)),
        (53, false, dns_answer.clone()),
        (53, true, dns_tcp),
        (5353, false, dns_answer.clone()),
        (5355, false, dns_answer),
        (67, false, dhcp_message(1, &dhcp_options)),
        (547, false, dhcpv6),
        (443, true, client_hello("www.example.org", 0)),
        (443, true, client_hello("x.example", 300)),
        (
            80,
            true,
            b"GET /a HTTP/1.1\r\nHost: example.org\r\nUser-Agent: x\r\n\r\n".to_vec(),
        ),
        (
            80,
            true,
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nServer: y\r\n\r\nabc".to_vec(),
        ),
        (
            1900,
            false,
            b"M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nST: ssdp:all\r\n\r\n".to_vec(),
        ),
        (123, false, ntp),
        (443, false, quic),
        (5060, false, invite()),
        (
            5060,
            true,
            b"SIP/2.0 180 Ringing\r\ni: a@example.net\r\nCSeq: 1 INVITE\r\nl: 0\r\n\r\n".to_vec(),
        ),
    ]
}

/// A SIP INVITE with a session description
fn invite() -> Vec<u8> {
    let mut sdp = sdp_offer("192.0.2.20", 7078, &[8, 0, 101, 111]);
    sdp.push_str("a=rtcp:9000 IN IP4 192.0.2.20\r\na=rtcp-mux\r\nm=video 0 RTP/SAVPF 96\r\n");
    sip_message(
        "INVITE sip:support@example.com SIP/2.0",
        &[
            ("Via", "SIP/2.0/UDP 192.0.2.20:5060;branch=z9hG4bK1"),
            ("From", "\"Anna\" <sip:anna@sip.example.net>;tag=1"),
            ("To", "<sip:support@example.com>"),
            ("Call-ID", "fuzz@192.0.2.20"),
            ("CSeq", "1 INVITE"),
        ],
        Some(&sdp),
    )
}

/// Changes a few bytes, ranges or length fields of `data`
pub fn mutate(rng: &mut StdRng, data: &mut Vec<u8>) {
    for _ in 0..rng.random_range(1..=6) {
        let len = data.len();
        match rng.random_range(0..8) {
            0 | 1 if len > 0 => {
                let at = rng.random_range(0..len);
                data[at] = rng.random();
            }
            2 if len > 0 => {
                let at = rng.random_range(0..len);
                data[at] = *[0, 0xff, 0xc0, 0x3f, 0x80, 0x7f]
                    .get(rng.random_range(0..6))
                    .unwrap();
            }
            3 if len > 1 => {
                let at = rng.random_range(0..len - 1);
                let v: u16 = *[0, 1, 0xffff, 0x7fff, 0x8000, 0xc00c]
                    .get(rng.random_range(0..6))
                    .unwrap();
                data[at..at + 2].copy_from_slice(&v.to_be_bytes());
            }
            4 if len > 0 => {
                data.truncate(rng.random_range(0..len));
            }
            5 => {
                let at = rng.random_range(0..=len);
                let n = rng.random_range(1..64);
                let bytes: Vec<u8> = (0..n).map(|_| rng.random()).collect();
                data.splice(at..at, bytes);
            }
            6 if len > 0 => {
                let start = rng.random_range(0..len);
                let end = rng.random_range(start..=len);
                data.drain(start..end);
            }
            _ if len > 0 => {
                // repeat a part, as in nested or looping structures
                let start = rng.random_range(0..len);
                let end = rng.random_range(start..=len.min(start + 64));
                let part = data[start..end].to_vec();
                let at = rng.random_range(0..=data.len());
                data.splice(at..at, part);
            }
            _ => data.push(rng.random()),
        }
    }
}

const LINK_TYPES: [LinkType; 5] = [
    ETH,
    LinkType::LinuxSll(Linktype::LINUX_SLL),
    LinkType::LinuxSll2(Linktype::LINUX_SLL2),
    LinkType::Null(Linktype::NULL),
    LinkType::RawIp(Linktype(101)),
];

/// The payload that the stream and the joined TLS messages cut from the packet lies in its bytes
/// (a range beyond them panics); layers and fields may name bytes that were not captured, but
/// no range is reversed
fn check(d: &Dissection, packet: &[u8], link_type: LinkType) {
    fn fields(fs: &[crate::dissect::Field], packet: &[u8]) {
        for f in fs {
            assert!(
                f.range.start <= f.range.end,
                "{} {:?} in {packet:02x?}",
                f.name,
                f.range
            );
            fields(&f.children, packet);
        }
    }
    for layer in &d.layers {
        assert!(
            layer.range.start <= layer.range.end,
            "{} {:?} in {packet:02x?}",
            layer.title,
            layer.range
        );
        fields(&layer.fields, packet);
    }
    if let Some(p) = transport_payload(packet, link_type) {
        assert!(
            packet.get(p.range.clone()).is_some(),
            "payload {:?} in {packet:02x?}",
            p.range
        );
    }
}

#[test]
fn fuzz_dissectors_with_garbled_packets() {
    let mut rng = rng();
    let samples = samples();
    let filters: Vec<Filter> = ["domain contains \"ex\"", "tls.sni", "http or dns"]
        .iter()
        .map(|f| Filter::parse(f).unwrap().unwrap())
        .collect();
    for _ in 0..iterations(20_000) {
        let (port, tcp, sample) = &samples[rng.random_range(0..samples.len())];
        let mut payload = sample.clone();
        mutate(&mut rng, &mut payload);
        let (sport, dport) = if rng.random() {
            (40_000, *port)
        } else {
            (*port, 40_000)
        };
        let mut packet = if *tcp {
            tcp_packet(sport, dport, rng.random(), &payload)
        } else {
            udp_packet(sport, dport, &payload)
        };
        // sometimes garble the headers as well, or read the bytes with another link type
        if rng.random_ratio(1, 4) {
            mutate(&mut rng, &mut packet);
        }
        let link_type = if rng.random_ratio(1, 8) {
            LINK_TYPES[rng.random_range(0..LINK_TYPES.len())]
        } else {
            ETH
        };
        let d = if *tcp && rng.random_ratio(1, 3) {
            let mut more = sample.clone();
            mutate(&mut rng, &mut more);
            let d = dissect_with(&packet, link_type, &more);
            check(&d, &packet, link_type);
            d
        } else {
            let d = dissect(&packet, link_type);
            check(&d, &packet, link_type);
            d
        };
        let p = Packet {
            number: 1,
            len: u32::try_from(packet.len()).unwrap(),
            captured: packet.len(),
            d: &d,
        };
        for f in &filters {
            let _ = f.matches(&p);
        }
        let _ = crate::filter::color(&p);
    }
}

#[test]
fn fuzz_rtp_rtcp_and_the_analysis_of_calls() {
    let mut rng = rng();
    let samples = [
        rtp_bytes(8, 1, 160, 7, true, 160),
        rtcp_report(1, 2, "anna@example.net"),
        invite(),
    ];
    let formats = [
        (8, "PCMA/8000".to_owned()),
        (111, "opus/48000/2".to_owned()),
    ];
    let mut analysis = Analysis::default();
    let mut time = 0;
    for n in 1..=iterations(20_000) as u64 {
        let mut payload = samples[rng.random_range(0..samples.len())].clone();
        if rng.random_ratio(3, 4) {
            mutate(&mut rng, &mut payload);
        }
        let ports = [5060, 7078, 7079, 9000, 40_000, 40_001];
        let sport = ports[rng.random_range(0..ports.len())];
        let dport = ports[rng.random_range(0..ports.len())];
        let packet = udp_between([192, 0, 2, 20], sport, [203, 0, 113, 61], dport, &payload);
        // read as RTP and as RTCP, whatever it is
        for rtcp in [false, true] {
            let mut d = dissect(&packet, ETH);
            let note = MediaNote {
                rtcp,
                formats: &formats,
                srtp: rng.random(),
                clock: Some(8000),
            };
            crate::dissect::media(&packet, &mut d, &note);
            check(&d, &packet, ETH);
        }
        // and as the analysis finds it
        time += rng.random_range(0..40_000);
        let stored = StoredPacket {
            number: n,
            timestamp: Timestamp::new(1000 + time / 1_000_000, time % 1_000_000),
            orig_len: u32::try_from(packet.len()).unwrap(),
            data: packet.clone().into(),
            conn: None,
        };
        let mut d = dissect(&packet, ETH);
        if let Some(note) = analysis.feed(&stored, &d) {
            annotate(&note, &packet, &mut d);
            check(&d, &packet, ETH);
        }
    }
}

/// A random filter text from the words of the grammar and some random characters
pub fn random_filter(rng: &mut StdRng) -> String {
    const SYMBOLS: [&str; 24] = [
        "and",
        "or",
        "not",
        "!",
        "&&",
        "||",
        "(",
        ")",
        "{",
        "}",
        "==",
        "!=",
        "<",
        ">=",
        "contains",
        "in",
        "\"",
        "\\",
        "\"x\"",
        "1",
        "-1",
        "192.168.1.0/24",
        "::1",
        "aa:bb:cc:dd:ee:ff",
    ];
    let mut text = String::new();
    for _ in 0..rng.random_range(0..40) {
        let word = match rng.random_range(0..6) {
            0 => FIELDS[rng.random_range(0..FIELDS.len())].0.to_owned(),
            1 => CONNECTION_FIELDS[rng.random_range(0..CONNECTION_FIELDS.len())]
                .0
                .to_owned(),
            2 => PROTOCOLS[rng.random_range(0..PROTOCOLS.len())].to_owned(),
            3 => rng.random::<u64>().to_string(),
            4 => char::from_u32(rng.random_range(0..0x3000))
                .unwrap_or('x')
                .to_string(),
            _ => SYMBOLS[rng.random_range(0..SYMBOLS.len())].to_owned(),
        };
        text.push_str(&word);
        if rng.random_ratio(4, 5) {
            text.push(' ');
        }
    }
    text
}

#[test]
fn fuzz_filters_with_random_texts() {
    let mut rng = rng();
    let packet = tcp_packet(51_000, 443, 1, &client_hello("www.example.org", 0));
    let d = dissect(&packet, ETH);
    let p = Packet {
        number: 3,
        len: u32::try_from(packet.len()).unwrap(),
        captured: packet.len(),
        d: &d,
    };
    for _ in 0..iterations(20_000) {
        let text = random_filter(&mut rng);
        if let Ok(Some(f)) = Filter::parse(&text) {
            let _ = f.matches(&p);
        }
        let _ = Filter::parse_connections(&text);
    }
}

/// The longest filters that nest or chain as much as possible are refused or evaluated quickly,
/// without running out of stack
#[test]
fn deeply_nested_and_long_filters_are_handled() {
    let packet = udp_packet(40_000, 53, &dns_query(1, 0x0100, "a.example.org", 1, 1));
    let d = dissect(&packet, ETH);
    let p = Packet {
        number: 1,
        len: u32::try_from(packet.len()).unwrap(),
        captured: packet.len(),
        d: &d,
    };
    let fill = |unit: &str, tail: &str| {
        let mut text = unit.repeat((MAX_LEN - tail.len()) / unit.len());
        text.push_str(tail);
        text
    };
    for text in [
        fill("(", "dns"),
        fill("!", "dns"),
        fill("not ", "dns"),
        fill("ip||", "ip"),
        fill("ip&&", "ip"),
        fill("!ip||", "ip"),
        fill("(ip)||", "ip"),
        fill("ip.addr in {1.2.3.4 ", "}"),
        fill("x", ""),
        "(".repeat(MAX_LEN + 1),
    ] {
        let started = std::time::Instant::now();
        for parse in [Filter::parse, Filter::parse_connections] {
            if let Ok(Some(f)) = parse(&text) {
                let _ = f.matches(&p);
            }
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "{}…",
            &text[..20]
        );
    }
}
