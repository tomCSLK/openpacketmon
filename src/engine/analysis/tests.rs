use std::sync::Arc;

use super::*;
use crate::dissect::tests::ETH;
use crate::filter::Filter;
use crate::utils::types::timestamp::Timestamp;

const CLIENT: [u8; 4] = [192, 168, 1, 10];
const SERVER: [u8; 4] = [10, 0, 0, 2];

fn ipv4(protocol: u8, len: usize, src: [u8; 4], dst: [u8; 4]) -> Vec<u8> {
    let mut p = vec![0x45, 0];
    p.extend_from_slice(&u16::try_from(20 + len).unwrap().to_be_bytes());
    p.extend_from_slice(&[0, 0, 0x40, 0, 64, protocol, 0, 0]);
    p.extend_from_slice(&src);
    p.extend_from_slice(&dst);
    p
}

fn ethernet() -> Vec<u8> {
    let mut p = vec![2, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 1];
    p.extend_from_slice(&[0x08, 0x00]);
    p
}

/// A TCP segment of the client (port 51000) or the server (port 443); `flags` as letters
/// (`S` SYN, `A` ACK, `F` FIN, `R` RST, `P` PSH)
fn segment(client: bool, seq: u32, ack: u32, flags: &str, window: u16, payload: &[u8]) -> Vec<u8> {
    let (src, dst, sport, dport) = if client {
        (CLIENT, SERVER, 51000_u16, 443_u16)
    } else {
        (SERVER, CLIENT, 443, 51000)
    };
    let mut bits = 0_u8;
    for (letter, bit) in [('F', 1), ('S', 2), ('R', 4), ('P', 8), ('A', 16)] {
        if flags.contains(letter) {
            bits |= bit;
        }
    }
    let mut p = ethernet();
    p.extend(ipv4(6, 20 + payload.len(), src, dst));
    p.extend_from_slice(&sport.to_be_bytes());
    p.extend_from_slice(&dport.to_be_bytes());
    p.extend_from_slice(&seq.to_be_bytes());
    p.extend_from_slice(&ack.to_be_bytes());
    p.extend_from_slice(&[0x50, bits]);
    p.extend_from_slice(&window.to_be_bytes());
    p.extend_from_slice(&[0, 0, 0, 0]);
    p.extend_from_slice(payload);
    p
}

fn dns(query: bool, id: u16) -> Vec<u8> {
    let (src, dst, sport, dport) = if query {
        (CLIENT, SERVER, 40000_u16, 53_u16)
    } else {
        (SERVER, CLIENT, 53, 40000)
    };
    let mut m = id.to_be_bytes().to_vec();
    m.extend_from_slice(if query { &[1, 0] } else { &[0x81, 0x80] });
    m.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0]);
    m.extend_from_slice(b"\x07example\x03org\x00\x00\x01\x00\x01");
    let mut p = ethernet();
    p.extend(ipv4(17, 8 + m.len(), src, dst));
    p.extend_from_slice(&sport.to_be_bytes());
    p.extend_from_slice(&dport.to_be_bytes());
    p.extend_from_slice(&u16::try_from(8 + m.len()).unwrap().to_be_bytes());
    p.extend_from_slice(&[0, 0]);
    p.extend(m);
    p
}

/// Analyzes packets given with their time in milliseconds; `orig_len` is the length of the bytes
fn run(packets: Vec<(u32, Vec<u8>)>) -> Vec<PacketAnalysis> {
    let mut analysis = Analysis::default();
    packets
        .into_iter()
        .enumerate()
        .map(|(i, (ms, data))| {
            let packet = StoredPacket {
                number: i as u64 + 1,
                timestamp: Timestamp::new(1000, i64::from(ms) * 1000),
                orig_len: u32::try_from(data.len()).unwrap(),
                data: Arc::from(data),
                conn: None,
            };
            let d = dissect::dissect(&packet.data, ETH);
            analysis.feed(&packet, &d).unwrap()
        })
        .collect()
}

fn handshake() -> Vec<(u32, Vec<u8>)> {
    vec![
        (0, segment(true, 100, 0, "S", 64000, &[])),
        (10, segment(false, 500, 101, "SA", 64000, &[])),
        (20, segment(true, 101, 501, "A", 64000, &[])),
    ]
}

#[test]
fn handshake_and_round_trip_times() {
    let mut packets = handshake();
    packets.push((30, segment(true, 101, 501, "PA", 64000, b"0123456789")));
    packets.push((45, segment(false, 501, 111, "A", 64000, &[])));
    let r = run(packets);
    // the SYN-ACK acknowledges the SYN, the ACK the SYN-ACK
    assert_eq!(r[1].acks, Some((1, 10_000)));
    assert_eq!(r[2].handshake, Some(20_000));
    assert_eq!(r[2].acks, Some((2, 10_000)));
    assert_eq!(r[4].acks, Some((4, 15_000)));
    assert!(r.iter().all(|r| r.flags == 0), "{r:?}");
}

#[test]
fn retransmissions_lost_and_late_segments() {
    let mut packets = handshake();
    packets.extend([
        (30, segment(true, 101, 501, "PA", 64000, b"0123456789")),
        // sent again: the ACK after it tells no round trip time
        (230, segment(true, 101, 501, "PA", 64000, b"0123456789")),
        (240, segment(false, 501, 111, "A", 64000, &[])),
        // 10 bytes missing before this one, then they come
        (250, segment(true, 121, 501, "PA", 64000, b"abcdefghij")),
        (251, segment(true, 111, 501, "PA", 64000, b"ABCDEFGHIJ")),
        // a keep-alive one byte before the next sequence number
        (900, segment(true, 130, 501, "A", 64000, &[])),
    ]);
    let r = run(packets);
    assert_eq!(r[3].flags, 0);
    assert_eq!(r[4].flags, flag::RETRANSMISSION);
    assert_eq!(r[5].acks, None);
    assert_eq!(r[6].flags, flag::LOST_SEGMENT);
    assert_eq!(r[7].flags, flag::OUT_OF_ORDER);
    assert_eq!(r[8].flags, flag::KEEP_ALIVE);
}

#[test]
fn keep_alives_and_their_answers() {
    let mut packets = handshake();
    packets.extend([
        (30, segment(false, 501, 101, "A", 64000, &[])),
        (40, segment(true, 100, 501, "A", 64000, &[])),
        (41, segment(false, 501, 101, "A", 64000, &[])),
        // a real duplicate ACK is still one
        (50, segment(false, 501, 101, "A", 64000, &[])),
    ]);
    let r = run(packets);
    assert_eq!(r[4].flags, flag::KEEP_ALIVE);
    assert_eq!(r[5].flags, flag::KEEP_ALIVE_ACK);
    assert_eq!(r[6].flags, flag::DUPLICATE_ACK);
}

#[test]
fn keep_alives_of_a_connection_already_running() {
    let r = run(vec![
        (0, segment(true, 999, 500, "A", 64000, &[])),
        (10, segment(false, 500, 1000, "A", 64000, &[])),
        (1000, segment(true, 999, 500, "A", 64000, &[])),
        (1010, segment(false, 500, 1000, "A", 64000, &[])),
        (2000, segment(true, 999, 500, "A", 64000, &[0])),
        (2010, segment(false, 500, 1000, "A", 64000, &[])),
        // the data goes on behind the keep-alives, nothing was lost
        (3000, segment(true, 1000, 500, "PA", 64000, b"abc")),
    ]);
    assert_eq!(r[0].flags, 0);
    assert_eq!(r[1].flags, 0);
    assert_eq!(r[2].flags, flag::KEEP_ALIVE);
    assert_eq!(r[3].flags, flag::KEEP_ALIVE_ACK);
    assert_eq!(r[4].flags, flag::KEEP_ALIVE);
    assert_eq!(r[5].flags, flag::KEEP_ALIVE_ACK);
    assert_eq!(r[6].flags, 0);
}

#[test]
fn duplicate_acks_window_updates_and_zero_windows() {
    let mut packets = handshake();
    packets.extend([
        (30, segment(false, 501, 101, "A", 64000, &[])),
        (31, segment(false, 501, 101, "A", 64000, &[])),
        (32, segment(false, 501, 101, "A", 32000, &[])),
        (33, segment(false, 501, 101, "A", 0, &[])),
        (34, segment(false, 501, 101, "R", 0, &[])),
    ]);
    let r = run(packets);
    // the SYN-ACK before has the same acknowledgment, but does not count
    assert_eq!(r[3].flags, 0);
    assert_eq!(r[4].flags, flag::DUPLICATE_ACK);
    assert_eq!(r[5].flags, flag::WINDOW_UPDATE);
    assert_eq!(r[6].flags, flag::ZERO_WINDOW);
    assert_eq!(r[7].flags, flag::RESET);
}

#[test]
fn a_syn_sent_again_and_a_new_connection_on_the_same_ports() {
    let r = run(vec![
        (0, segment(true, 100, 0, "S", 64000, &[])),
        (1000, segment(true, 100, 0, "S", 64000, &[])),
        (1010, segment(false, 500, 101, "SA", 64000, &[])),
        (1020, segment(true, 101, 501, "A", 64000, &[])),
        // the same ports later, with other sequence numbers
        (5000, segment(true, 9000, 0, "S", 64000, &[])),
    ]);
    assert_eq!(r[1].flags, flag::RETRANSMISSION);
    assert_eq!(r[3].handshake, Some(1_020_000));
    assert_eq!(r[4].flags, 0);
}

#[test]
fn the_length_on_the_wire_counts_for_packets_with_headers_only() {
    let mut packets = handshake();
    let mut big = segment(true, 101, 501, "PA", 64000, &[b'x'; 1000]);
    big.truncate(128);
    packets.push((30, big));
    packets.push((31, segment(true, 1101, 501, "PA", 64000, b"next")));
    let r = run(packets);
    assert_eq!(r[4].flags, 0);
}

#[test]
fn dns_response_times() {
    let r = run(vec![
        (0, dns(true, 0x1234)),
        (5, dns(true, 0x4321)),
        (25, dns(false, 0x1234)),
        // no query for this one
        (30, dns(false, 0x9999)),
    ]);
    assert_eq!(r[2].dns, Some((1, 25_000)));
    assert_eq!(r[3].dns, None);
}

#[test]
fn findings_go_to_the_filter_the_info_and_the_layers() {
    let mut packets = handshake();
    packets.push((30, segment(true, 101, 501, "PA", 64000, b"0123456789")));
    packets.push((230, segment(true, 101, 501, "PA", 64000, b"0123456789")));
    let records = run(packets.clone());
    let annotated = |i: usize| {
        let mut d = dissect::dissect(&packets[i].1, ETH);
        annotate(&records[i], &mut d);
        d
    };
    let matches = |text: &str, d: &Dissection| {
        Filter::parse(text)
            .unwrap()
            .unwrap()
            .matches(&crate::filter::Packet {
                number: 1,
                len: 60,
                captured: 60,
                d,
            })
    };
    let retransmitted = annotated(4);
    assert!(retransmitted.info.starts_with("[TCP Retransmission] "));
    assert!(matches("tcp.analysis.retransmission", &retransmitted));
    assert!(matches("tcp.analysis.flags", &retransmitted));
    let ack = annotated(2);
    assert!(!matches("tcp.analysis.flags", &ack));
    assert!(matches("tcp.analysis.handshake == 20ms", &ack));
    assert!(matches(
        "tcp.analysis.ack_rtt > 0.005 and tcp.analysis.ack_rtt < 11ms",
        &ack
    ));
    assert!(matches("tcp.analysis.acks_frame == 2", &ack));
    let tcp = ack
        .layers
        .iter()
        .find(|l| l.title.starts_with("Transmission Control Protocol"))
        .unwrap();
    let analysis = tcp.fields.last().unwrap();
    assert_eq!(analysis.name, "[SEQ/ACK analysis]");
    assert!(analysis.children.iter().any(|f| f.value == "0.010000 s"));
}

#[test]
fn records_follow_the_store() {
    let mut analysis = Analysis::default();
    let data: Arc<[u8]> = Arc::from(dns(true, 1));
    let packet = |number| StoredPacket {
        number,
        timestamp: Timestamp::new(1, 0),
        orig_len: 60,
        data: data.clone(),
        conn: None,
    };
    let d = dissect::dissect(&data, ETH);
    for n in 1..=5 {
        analysis.feed(&packet(n), &d);
    }
    assert_eq!(analysis.done(), 5);
    assert!(analysis.feed(&packet(3), &d).is_some());
    analysis.forget_before(3);
    assert!(analysis.get(2).is_none());
    assert_eq!(
        analysis.after(3).map(|(n, _)| n).collect::<Vec<_>>(),
        [4, 5]
    );
    // packets 6 to 9 were dropped from the store before they were analyzed
    analysis.feed(&packet(10), &d);
    assert!(analysis.get(5).is_none());
    assert!(analysis.get(10).is_some());
    assert_eq!(
        analysis.paths()[usize::from(analysis.get(10).unwrap().path)],
        ["eth", "ip", "udp", "dns"]
    );
}
