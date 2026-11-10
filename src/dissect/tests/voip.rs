//! SIP, SDP, RTP and RTCP, with made-up messages

use super::*;
use crate::dissect::rtp::{self, MediaNote};
use crate::dissect::sip;
use crate::dissect::{SdpMedia, Value, media};

const PHONE: [u8; 4] = [192, 0, 2, 20];
const PROVIDER: [u8; 4] = [203, 0, 113, 60];

fn invite() -> Vec<u8> {
    let sdp = sdp_offer("192.0.2.20", 7078, &[8, 0, 101]);
    sip_message(
        "INVITE sip:support@example.com SIP/2.0",
        &[
            (
                "Via",
                "SIP/2.0/UDP 192.0.2.20:5060;branch=z9hG4bK1a2b3c;rport",
            ),
            ("Max-Forwards", "70"),
            (
                "From",
                "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=c81d07",
            ),
            ("To", "<sip:support@example.com>"),
            ("Call-ID", "3f2a9c1e7b54@192.0.2.20"),
            ("CSeq", "1 INVITE"),
            ("Contact", "<sip:anna@192.0.2.20:5060>"),
        ],
        Some(&sdp),
    )
}

fn text<'a>(d: &'a Dissection, key: &str) -> Option<&'a str> {
    d.values.iter().find_map(|(k, v)| match v {
        Value::Text(t) if *k == key => Some(t.as_str()),
        _ => None,
    })
}

fn num(d: &Dissection, key: &str) -> Option<u64> {
    d.values.iter().find_map(|(k, v)| match v {
        Value::Num(n) if *k == key => Some(*n),
        _ => None,
    })
}

#[test]
fn a_sip_invite_with_a_session_description_is_dissected() {
    let m = invite();
    let p = udp_between(PHONE, 5060, PROVIDER, 5060, &m);
    let d = dissect(&p, ETH);
    assert_eq!(d.protocol, "SIP/SDP");
    assert_eq!(
        d.info,
        "Request: INVITE sip:support@example.com, with session description"
    );
    assert_eq!(d.layers.len(), 5);
    let sip = &d.layers[3];
    assert_eq!(sip.title, "Session Initiation Protocol (INVITE)");
    assert_eq!(sip.fields[0].name, "Request-Line");
    assert_eq!(
        sip.fields[0].value,
        "INVITE sip:support@example.com SIP/2.0"
    );
    assert_eq!(field(&d, 3, "Call-ID").value, "3f2a9c1e7b54@192.0.2.20");
    assert_eq!(field(&d, 3, "CSeq").value, "1 INVITE");
    assert_eq!(
        field(&d, 3, "From").value,
        "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=c81d07"
    );
    // the head ends with its empty line, the description takes the rest
    let body = 42 + m.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    assert_eq!(sip.range, 42..body);
    let sdp = &d.layers[4];
    assert_eq!(sdp.title, "Session Description Protocol");
    assert_eq!(sdp.range, body..p.len());
    assert_eq!(field(&d, 4, "Version (v)").value, "0");
    assert_eq!(
        field(&d, 4, "Media Description (m)").value,
        "audio 7078 RTP/AVP 8 0 101"
    );
    assert_eq!(
        field(&d, 4, "Connection Information (c)").value,
        "IN IP4 192.0.2.20"
    );
    assert_eq!(
        field(&d, 4, "Media Attribute (a)").value,
        "rtpmap:8 PCMA/8000"
    );
    assert_eq!(
        d.sdp,
        vec![SdpMedia {
            kind: "audio".into(),
            addr: Some("192.0.2.20".parse().unwrap()),
            port: 7078,
            rtcp_port: 7079,
            srtp: false,
            formats: vec![
                (8, "PCMA/8000".into()),
                (0, "PCMU/8000".into()),
                (101, "telephone-event/8000".into()),
            ],
        }]
    );
    assert_eq!(text(&d, "sip.method"), Some("INVITE"));
    assert_eq!(text(&d, "sip.call_id"), Some("3f2a9c1e7b54@192.0.2.20"));
    assert_eq!(
        text(&d, "sip.from"),
        Some("\"Anna Beispiel\" <sip:anna@sip.example.net>")
    );
    assert_eq!(text(&d, "sip.to"), Some("<sip:support@example.com>"));
    assert_eq!(num(&d, "sip.cseq.seq"), Some(1));
    assert!(d.protocols.contains(&"sip") && d.protocols.contains(&"sdp"));
    assert!(!d.truncated);
}

#[test]
fn a_sip_response_names_its_status_and_the_method_it_answers() {
    let m = sip_message(
        "SIP/2.0 486 Busy Here",
        &[
            ("Via", "SIP/2.0/UDP 192.0.2.20:5060;branch=z9hG4bK77"),
            ("From", "<sip:anna@sip.example.net>;tag=a3f91c"),
            ("To", "<sip:vertrieb@example.com>;tag=84c2e0d7"),
            ("Call-ID", "7c1e9a40d2b3@192.0.2.20"),
            ("CSeq", "2 INVITE"),
        ],
        None,
    );
    let d = dissect(&udp_between(PROVIDER, 5060, PHONE, 5060, &m), ETH);
    assert_eq!(d.protocol, "SIP");
    assert_eq!(d.info, "Status: 486 Busy Here (INVITE)");
    assert_eq!(d.layers[3].title, "Session Initiation Protocol (486)");
    assert_eq!(d.layers[3].fields[0].name, "Status-Line");
    assert_eq!(num(&d, "sip.status_code"), Some(486));
    assert_eq!(text(&d, "sip.method"), None);
    assert_eq!(text(&d, "sip.cseq.method"), Some("INVITE"));
    assert_eq!(text(&d, "sip.to"), Some("<sip:vertrieb@example.com>"));
    assert!(d.sdp.is_empty());
}

#[test]
fn sip_is_found_on_other_ports_over_tcp_and_with_compact_headers() {
    let m = b"OPTIONS sip:sip.example.net SIP/2.0\r\nv: SIP/2.0/TCP 192.0.2.20:5080\r\n\
        f: <sip:anna@sip.example.net>;tag=1\r\nt: <sip:sip.example.net>\r\ni: abc@192.0.2.20\r\n\
        CSeq: 7 OPTIONS\r\nl: 0\r\n\r\n";
    let d = dissect(&tcp_packet(5080, 5080, 1, m), ETH);
    assert_eq!(d.protocol, "SIP");
    assert_eq!(d.info, "Request: OPTIONS sip:sip.example.net");
    assert_eq!(text(&d, "sip.call_id"), Some("abc@192.0.2.20"));
    assert_eq!(text(&d, "sip.from"), Some("<sip:anna@sip.example.net>"));
    assert_eq!(field(&d, 3, "i").value, "abc@192.0.2.20");
}

#[test]
fn display_names_keep_their_letters() {
    let m = sip_message(
        "BYE sip:anna@192.0.2.20 SIP/2.0",
        &[
            ("From", "\"Jürgen Müller\" <sip:zentrale@example.org>;tag=9"),
            ("Call-ID", "x@203.0.113.60"),
            ("CSeq", "2 BYE"),
        ],
        None,
    );
    let d = dissect(&udp_between(PROVIDER, 5060, PHONE, 5060, &m), ETH);
    assert_eq!(
        field(&d, 3, "From").value,
        "\"Jürgen Müller\" <sip:zentrale@example.org>;tag=9"
    );
}

#[test]
fn only_a_sip_first_line_is_sip() {
    for (line, sip) in [
        ("INVITE sip:a@example.net SIP/2.0\r\n", true),
        ("SIP/2.0 200 OK\r\n", true),
        ("SIP/2.0 200\r\n", true),
        ("PRACK sips:a@example.net SIP/2.0\n", true),
        ("SIP/2.0 2000 OK\r\n", false),
        ("invite sip:a@example.net SIP/2.0\r\n", false),
        ("INVITE sip:a@example.net SIP/3.0\r\n", false),
        ("INVITE sip:a@example.net  SIP/2.0\r\n", false),
        ("GET / HTTP/1.1\r\n", false),
        ("INVITE sip:a@example.net SIP/2.0", false),
    ] {
        assert_eq!(sip::is_sip(line.as_bytes(), 0), sip, "{line:?}");
    }
    // a keep-alive of SIP is not a message
    let d = dissect(&udp_between(PHONE, 5060, PROVIDER, 5060, b"\r\n\r\n"), ETH);
    assert_eq!(d.protocol, "UDP");
}

#[test]
fn a_huge_content_length_is_only_cut_off() {
    let m = format!(
        "SIP/2.0 200 OK\r\nCall-ID: a\r\nCSeq: 1 INVITE\r\nContent-Length: {}\r\n\r\nv=0\r\n",
        usize::MAX
    );
    let d = dissect(&udp_between(PROVIDER, 5060, PHONE, 5060, m.as_bytes()), ETH);
    assert_eq!(d.protocol, "SIP/SDP");
    assert!(d.truncated);
}

#[test]
fn a_cut_off_body_marks_the_packet_truncated() {
    let mut m = invite();
    m.truncate(m.len() - 20);
    let d = dissect(&tcp_packet(5060, 5060, 1, &m), ETH);
    assert_eq!(d.protocol, "SIP/SDP");
    assert!(d.truncated);
}

fn note(rtcp: bool, formats: &[(u8, String)]) -> MediaNote<'_> {
    MediaNote {
        rtcp,
        formats,
        srtp: false,
        clock: Some(8000),
    }
}

/// A datagram read as RTP or RTCP, as the analysis has it read
fn as_media(payload: &[u8], note: &MediaNote) -> Dissection {
    let p = udp_between(PHONE, 7078, [203, 0, 113, 61], 31456, payload);
    let mut d = dissect(&p, ETH);
    assert_eq!(d.protocol, "UDP");
    media(&p, &mut d, note);
    d
}

#[test]
fn rtp_shows_its_header_and_codec() {
    let formats = vec![
        (8, "PCMA/8000".to_owned()),
        (101, "telephone-event/8000".to_owned()),
    ];
    let d = as_media(
        &rtp_bytes(8, 21043, 1_617_600, 0x5e1f_02a7, true, 160),
        &note(false, &formats),
    );
    assert_eq!(d.protocol, "RTP");
    assert_eq!(
        d.info,
        "PT=ITU-T G.711 PCMA, SSRC=0x5E1F02A7, Seq=21043, Time=1617600, Mark"
    );
    // the RTP layer takes the place of the plain data
    assert_eq!(d.layers.len(), 4);
    assert_eq!(d.layers[3].title, "Real-Time Transport Protocol");
    assert_eq!(d.layers[3].range, 42..42 + 12 + 160);
    assert_eq!(field(&d, 3, "Version").value, "RFC 1889 Version (2)");
    assert_eq!(
        field(&d, 3, "Padding / Extension / CSRC count").value,
        "False / False / 0"
    );
    assert_eq!(field(&d, 3, "Marker").value, "True (start of a talkspurt)");
    assert_eq!(field(&d, 3, "Payload type").value, "ITU-T G.711 PCMA (8)");
    assert_eq!(
        field(&d, 3, "Synchronization Source identifier").value,
        "0x5e1f02a7 (1579090599)"
    );
    let payload = field(&d, 3, "Payload");
    assert_eq!(payload.value, "160 bytes, G.711 A-law, 20 ms of speech");
    assert_eq!(payload.range, 54..54 + 160);
    assert_eq!(text(&d, "rtp.ssrc"), Some("0x5e1f02a7"));
    assert_eq!(num(&d, "rtp.seq"), Some(21043));
    assert_eq!(num(&d, "rtp.p_type"), Some(8));
    assert!(d.protocols.contains(&"rtp"));

    // a named dynamic type, an unknown one, and SRTP
    let event = as_media(&rtp_bytes(101, 5, 160, 1, false, 4), &note(false, &formats));
    assert_eq!(
        event.info,
        "PT=telephone-event (101), SSRC=0x00000001, Seq=5, Time=160"
    );
    assert_eq!(
        field(&event, 3, "Payload type").value,
        "dynamic (101): telephone-event/8000"
    );
    let unknown = as_media(&rtp_bytes(111, 5, 960, 2, false, 90), &note(false, &[]));
    assert!(unknown.info.starts_with("PT=dynamic (111), "));
    assert_eq!(
        field(&unknown, 3, "Payload type").value,
        "dynamic (111), codec unknown"
    );
    assert_eq!(
        field(&unknown, 3, "Payload").value,
        "90 bytes (encrypted or unknown)"
    );
    let srtp = MediaNote {
        srtp: true,
        ..note(false, &formats)
    };
    let secret = as_media(&rtp_bytes(8, 5, 160, 3, false, 170), &srtp);
    assert_eq!(
        field(&secret, 3, "Payload").value,
        "170 bytes (encrypted, SRTP)"
    );
}

#[test]
fn rtcp_shows_each_packet_of_a_compound_packet() {
    let d = as_media(
        &rtcp_report(0x5e1f_02a7, 0x0b77_c3d1, "anna@192.0.2.20"),
        &note(true, &[]),
    );
    assert_eq!(d.protocol, "RTCP");
    assert_eq!(d.info, "Sender Report, Source Description");
    assert_eq!(d.layers.len(), 5);
    assert_eq!(
        d.layers[3].title,
        "Real-time Transport Control Protocol (Sender Report)"
    );
    assert_eq!(d.layers[3].range, 42..42 + 52);
    assert_eq!(
        field(&d, 3, "Version / Reception report count").value,
        "2 / 1"
    );
    assert_eq!(field(&d, 3, "Packet type").value, "Sender Report (200)");
    assert_eq!(field(&d, 3, "Length").value, "12 (52 bytes)");
    assert_eq!(field(&d, 3, "Sender SSRC").value, "0x5e1f02a7");
    assert_eq!(
        field(&d, 3, "Timestamp, MSW / LSW").value,
        "2026-11-09T12:00:00.500000Z"
    );
    assert_eq!(field(&d, 3, "Sender's packet count").value, "1000");
    assert_eq!(field(&d, 3, "Report: SSRC").value, "0x0b77c3d1");
    assert_eq!(
        field(&d, 3, "Fraction lost / Cumulative lost").value,
        "5 / 256 (2.0 %) / 12"
    );
    assert_eq!(
        field(&d, 3, "Interarrival jitter").value,
        "24 (3.0 ms at 8000 Hz)"
    );
    assert_eq!(
        d.layers[4].title,
        "Real-time Transport Control Protocol (Source description)"
    );
    assert_eq!(field(&d, 4, "SSRC").value, "0x5e1f02a7");
    assert_eq!(field(&d, 4, "CNAME").value, "anna@192.0.2.20");
    assert!(d.protocols.contains(&"rtcp"));
}

#[test]
fn the_form_of_rtp_and_rtcp() {
    let good = rtp_bytes(8, 1, 160, 7, false, 160);
    assert!(rtp::rtp_header(&good).is_some());
    let mut v1 = good.clone();
    v1[0] = 0x40;
    assert!(rtp::rtp_header(&v1).is_none());
    // payload types 72 to 76 belong to RTCP
    assert!(rtp::rtp_header(&rtp_bytes(73, 1, 0, 7, false, 10)).is_none());
    // more CSRCs or padding than the datagram holds
    let mut csrc = good.clone();
    csrc[0] = 0x8f;
    csrc.truncate(40);
    assert!(rtp::rtp_header(&csrc).is_none());
    let mut padding = good.clone();
    padding[0] = 0xa0;
    *padding.last_mut().unwrap() = 0;
    assert!(rtp::rtp_header(&padding).is_none());
    assert!(rtp::rtp_header(&good[..11]).is_none());

    let report = rtcp_report(1, 2, "x@example.net");
    assert!(rtp::is_rtcp(&report, true));
    // a compound packet whose lengths do not add up counts only where SDP announced it
    let mut odd = report.clone();
    odd.extend_from_slice(&[1, 2, 3, 4]);
    assert!(!rtp::is_rtcp(&odd, true));
    assert!(rtp::is_rtcp(&odd, false));
    // RTP is no RTCP
    assert!(!rtp::is_rtcp(&good, false));
}

#[test]
fn encrypted_rtcp_shows_the_rest_as_data() {
    let mut m = rtcp_report(1, 2, "x@example.net");
    m.truncate(52);
    m.extend_from_slice(&[0x9c; 30]);
    let d = as_media(&m, &note(true, &[]));
    assert_eq!(d.protocol, "RTCP");
    assert_eq!(d.info, "Sender Report");
    assert_eq!(
        d.layers.last().unwrap().fields[0].value,
        "30 bytes (encrypted or unknown)"
    );
}
