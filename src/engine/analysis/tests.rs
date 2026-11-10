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
            analysis.feed(&packet, &d).unwrap().record
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
        let note = Note {
            record: records[i],
            voip: None,
        };
        annotate(&note, &packets[i].1, &mut d);
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

// ---- VoIP --------------------------------------------------------------------------------------

mod calls {
    use super::*;
    use crate::dissect::tests::{rtcp_report, rtp_bytes, sdp_offer, sip_message, udp_between};
    use voip::Mark;

    const PHONE: [u8; 4] = [192, 0, 2, 20];
    const PROVIDER: [u8; 4] = [203, 0, 113, 60];
    const MEDIA: [u8; 4] = [203, 0, 113, 61];
    const CALL_ID: &str = "3f2a9c1e7b54@192.0.2.20";

    fn sip(
        from: [u8; 4],
        to: [u8; 4],
        first: &str,
        cseq: &str,
        call: &str,
        sdp: Option<&str>,
    ) -> Vec<u8> {
        let m = sip_message(
            first,
            &[
                ("From", "<sip:anna@sip.example.net>;tag=c81d07"),
                ("To", "<sip:support@example.com>"),
                ("Call-ID", call),
                ("CSeq", cseq),
            ],
            sdp,
        );
        udp_between(from, 5060, to, 5060, &m)
    }

    /// Analyzes packets given with their time in milliseconds
    fn analyze(packets: &[(u32, Vec<u8>)]) -> Analysis {
        let mut analysis = Analysis::default();
        for (i, (ms, data)) in packets.iter().enumerate() {
            let packet = StoredPacket {
                number: i as u64 + 1,
                timestamp: Timestamp::new(1000, i64::from(*ms) * 1000),
                orig_len: u32::try_from(data.len()).unwrap(),
                data: Arc::from(data.as_slice()),
                conn: None,
            };
            let d = dissect::dissect(&packet.data, ETH);
            analysis.feed(&packet, &d);
        }
        analysis
    }

    fn mark(analysis: &Analysis, n: u64) -> Option<Mark> {
        analysis.get(n).unwrap().voip
    }

    /// A packet dissected with what the analysis found
    fn annotated(analysis: &Analysis, data: &[u8], n: u64) -> Dissection {
        let mut d = dissect::dissect(data, ETH);
        annotate(&analysis.note(n).unwrap(), data, &mut d);
        d
    }

    fn call_id(d: &Dissection) -> Option<&str> {
        d.values.iter().find_map(|(k, v)| match v {
            Value::Text(t) if *k == "voip.call_id" => Some(t.as_str()),
            _ => None,
        })
    }

    fn call() -> Vec<(u32, Vec<u8>)> {
        let offer = sdp_offer("192.0.2.20", 7078, &[8, 0, 101]);
        let answer = sdp_offer("203.0.113.61", 31456, &[8, 101]);
        vec![
            (
                0,
                sip(
                    PHONE,
                    PROVIDER,
                    "INVITE sip:support@example.com SIP/2.0",
                    "1 INVITE",
                    CALL_ID,
                    Some(&offer),
                ),
            ),
            (
                40,
                sip(
                    PROVIDER,
                    PHONE,
                    "SIP/2.0 100 Trying",
                    "1 INVITE",
                    CALL_ID,
                    None,
                ),
            ),
            (
                2000,
                sip(
                    PROVIDER,
                    PHONE,
                    "SIP/2.0 200 OK",
                    "1 INVITE",
                    CALL_ID,
                    Some(&answer),
                ),
            ),
            (
                2010,
                sip(
                    PHONE,
                    PROVIDER,
                    "ACK sip:support@example.com SIP/2.0",
                    "1 ACK",
                    CALL_ID,
                    None,
                ),
            ),
            (
                2020,
                udp_between(
                    PHONE,
                    7078,
                    MEDIA,
                    31456,
                    &rtp_bytes(8, 100, 0, 0x5e1f_02a7, true, 160),
                ),
            ),
            (
                2030,
                udp_between(
                    MEDIA,
                    31456,
                    PHONE,
                    7078,
                    &rtp_bytes(8, 900, 0, 0x0b77_c3d1, true, 160),
                ),
            ),
            (
                3000,
                udp_between(
                    PHONE,
                    7079,
                    MEDIA,
                    31457,
                    &rtcp_report(0x5e1f_02a7, 0x0b77_c3d1, "anna@192.0.2.20"),
                ),
            ),
            (
                5000,
                sip(
                    PHONE,
                    PROVIDER,
                    "BYE sip:support@example.com SIP/2.0",
                    "2 BYE",
                    CALL_ID,
                    None,
                ),
            ),
            (
                5040,
                sip(PROVIDER, PHONE, "SIP/2.0 200 OK", "2 BYE", CALL_ID, None),
            ),
            (
                6000,
                sip(
                    PHONE,
                    PROVIDER,
                    "REGISTER sip:sip.example.net SIP/2.0",
                    "1 REGISTER",
                    "reg@192.0.2.20",
                    None,
                ),
            ),
            (
                6050,
                sip(
                    PROVIDER,
                    PHONE,
                    "SIP/2.0 200 OK",
                    "1 REGISTER",
                    "reg@192.0.2.20",
                    None,
                ),
            ),
        ]
    }

    #[test]
    fn a_call_ties_its_sip_messages_and_its_rtp_together() {
        let packets = call();
        let a = analyze(&packets);
        let sip = |call, answers, status| {
            Some(Mark::Sip {
                call,
                answers,
                status,
            })
        };
        assert_eq!(mark(&a, 1), sip(Some(0), None, None));
        assert_eq!(mark(&a, 2), sip(Some(0), Some((1, 40_000)), Some(100)));
        assert_eq!(mark(&a, 3), sip(Some(0), Some((1, 2_000_000)), Some(200)));
        assert_eq!(mark(&a, 4), sip(Some(0), None, None));
        // RTP to the address of the answer, back to the one of the offer, RTCP next to it
        assert_eq!(
            mark(&a, 5),
            Some(Mark::Media {
                media: 1,
                rtcp: false
            })
        );
        assert_eq!(
            mark(&a, 6),
            Some(Mark::Media {
                media: 0,
                rtcp: false
            })
        );
        assert_eq!(
            mark(&a, 7),
            Some(Mark::Media {
                media: 1,
                rtcp: true
            })
        );
        assert_eq!(mark(&a, 8), sip(Some(0), None, None));
        assert_eq!(mark(&a, 9), sip(Some(0), Some((8, 40_000)), Some(200)));
        // a registration is no call
        assert_eq!(mark(&a, 10), sip(None, None, None));
        assert_eq!(mark(&a, 11), sip(None, Some((10, 50_000)), Some(200)));

        let rtp = annotated(&a, &packets[4].1, 5);
        assert_eq!(rtp.protocol, "RTP");
        assert!(
            rtp.info
                .starts_with("PT=ITU-T G.711 PCMA, SSRC=0x5E1F02A7, Seq=100")
        );
        assert_eq!(call_id(&rtp), Some(CALL_ID));
        let note = a.note(5).unwrap();
        assert_eq!(note.voip.unwrap().media.unwrap().setup, Some(3));
        let rtcp = annotated(&a, &packets[6].1, 7);
        assert_eq!(rtcp.protocol, "RTCP");
        assert_eq!(call_id(&rtcp), Some(CALL_ID));
        assert_eq!(call_id(&annotated(&a, &packets[0].1, 1)), Some(CALL_ID));
        assert_eq!(call_id(&annotated(&a, &packets[9].1, 10)), None);
        // the protocol hierarchy counts RTP and RTCP
        let path = |n| a.paths()[usize::from(a.get(n).unwrap().path)].clone();
        assert_eq!(path(5).last(), Some(&"rtp"));
        assert_eq!(path(7).last(), Some(&"rtcp"));
        assert_eq!(path(1).last(), Some(&"sdp"));
    }

    #[test]
    fn the_filter_finds_calls_and_their_streams() {
        let packets = call();
        let a = analyze(&packets);
        let matching = |text: &str| -> Vec<u64> {
            let filter = Filter::parse(text).unwrap().unwrap();
            (1..=packets.len() as u64)
                .filter(|n| {
                    let data = &packets[usize::try_from(*n).unwrap() - 1].1;
                    let d = annotated(&a, data, *n);
                    filter.matches(&crate::filter::Packet {
                        number: *n,
                        len: 100,
                        captured: data.len(),
                        d: &d,
                    })
                })
                .collect()
        };
        assert_eq!(matching("sip or rtp"), [1, 2, 3, 4, 5, 6, 8, 9, 10, 11]);
        assert_eq!(matching("rtcp"), [7]);
        assert_eq!(matching("sdp"), [1, 3]);
        assert_eq!(
            matching(&format!("voip.call_id == \"{CALL_ID}\"")),
            [1, 2, 3, 4, 5, 6, 7, 8, 9]
        );
        assert_eq!(matching("sip.method == INVITE"), [1]);
        assert_eq!(matching("sip.status_code >= 200"), [3, 9, 11]);
        assert_eq!(
            matching("sip.from contains anna"),
            [1, 2, 3, 4, 8, 9, 10, 11]
        );
        assert_eq!(matching("rtp.ssrc == 0x5E1F02A7"), [5]);
        assert_eq!(matching("rtp.ssrc in {0x5e1f02a7 0x0b77c3d1}"), [5, 6]);
        assert_eq!(matching("rtp.p_type == 8 and rtp.seq > 500"), [6]);
        // the rows of calls have a color of their own
        for n in [1, 5, 7] {
            let data = &packets[usize::try_from(n).unwrap() - 1].1;
            let d = annotated(&a, data, n);
            let packet = crate::filter::Packet {
                number: n,
                len: 100,
                captured: data.len(),
                d: &d,
            };
            assert_eq!(crate::filter::color(&packet), Some("voip"), "packet {n}");
        }
    }

    /// A flow of RTP without SIP (a video conference in a browser), from port 52714 to 10000
    fn stream(from: u16, count: u16) -> Vec<Vec<u8>> {
        (0..count)
            .map(|i| {
                let m = rtp_bytes(111, from + i, u32::from(i) * 960, 0x9b2e_7714, i == 0, 120);
                udp_between(PHONE, 52714, [198, 51, 100, 7], 10000, &m)
            })
            .collect()
    }

    #[test]
    fn rtp_without_sip_is_found_by_its_form() {
        let mut packets: Vec<(u32, Vec<u8>)> = Vec::new();
        for (i, rtp) in stream(40981, 12).into_iter().enumerate() {
            let ms = u32::try_from(i).unwrap() * 20;
            packets.push((ms, rtp));
            // a flow of another kind whose first byte looks like RTP: its SSRC changes
            let other = rtp_bytes(8, 1, 0, u32::try_from(i).unwrap(), false, 20);
            packets.push((
                ms + 5,
                udp_between(PHONE, 40000, [198, 51, 100, 8], 40000, &other),
            ));
        }
        // its RTCP on the next ports, and a datagram there that is not
        packets.push((
            300,
            udp_between(
                PHONE,
                52715,
                [198, 51, 100, 7],
                10001,
                &rtcp_report(0x9b2e_7714, 1, "x"),
            ),
        ));
        packets.push((
            310,
            udp_between(
                PHONE,
                52715,
                [198, 51, 100, 7],
                10001,
                &[0x81, 200, 0, 9, 1, 2, 3],
            ),
        ));
        let a = analyze(&packets);
        let media = Some(Mark::Media {
            media: 0,
            rtcp: false,
        });
        // the first nine were decided with the tenth
        for n in (1..=23).step_by(2) {
            assert_eq!(mark(&a, n), media, "packet {n}");
        }
        for n in (2..=22).step_by(2) {
            assert_eq!(mark(&a, n), None, "packet {n}");
        }
        assert_eq!(mark(&a, 24), Some(Mark::Candidate));
        assert_eq!(
            mark(&a, 25),
            Some(Mark::Media {
                media: 0,
                rtcp: true
            })
        );
        assert_eq!(mark(&a, 26), None);
        let note = a.note(1).unwrap().voip.unwrap();
        assert_eq!(note.call_id, None);
        assert_eq!(note.media.unwrap().setup, None);
        let first = annotated(&a, &packets[0].1, 1);
        assert_eq!(first.protocol, "RTP");
        assert!(
            first
                .info
                .starts_with("PT=dynamic (111), SSRC=0x9B2E7714, Seq=40981")
        );
        assert_eq!(
            a.paths()[usize::from(a.get(1).unwrap().path)].last(),
            Some(&"rtp")
        );
    }

    #[test]
    fn a_flow_that_breaks_off_is_no_rtp() {
        let mut packets: Vec<(u32, Vec<u8>)> = stream(100, 9).into_iter().map(|p| (0, p)).collect();
        // a jump of the sequence number starts anew
        packets.extend(stream(500, 9).into_iter().map(|p| (0, p)));
        let a = analyze(&packets);
        for n in 1..=9 {
            assert_eq!(mark(&a, n), None, "packet {n}");
        }
        for n in 10..=18 {
            assert_eq!(mark(&a, n), Some(Mark::Candidate), "packet {n}");
        }
        // well-known ports are left alone
        let low: Vec<(u32, Vec<u8>)> = (0..12)
            .map(|i| {
                (
                    0,
                    udp_between(
                        PHONE,
                        5004,
                        [198, 51, 100, 7],
                        999,
                        &rtp_bytes(0, i, 0, 5, false, 20),
                    ),
                )
            })
            .collect();
        let a = analyze(&low);
        assert!((1..=12).all(|n| mark(&a, n).is_none()));
    }
}
