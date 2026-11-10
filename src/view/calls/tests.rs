//! Made-up calls after the draft (`docs/entwurf/`), as one capture file: a registration, a
//! stream of RTP without SIP, a rejected call, an ended one, a bad one, a missed one and one that
//! still runs at the end of the file. No real numbers, addresses from the documentation ranges.

use std::path::PathBuf;

use serial_test::serial;

use super::audio::{CallAudio, Channel, alaw, call_audio, ulaw};
use super::*;
use crate::config::conf::Conf;
use crate::dissect::tests::{rtcp_report_on, rtp_bytes, sdp_offer, sip_message, udp_between};
use crate::engine::tests::remove_test_dir;
use crate::networking::types::capture_context::CaptureSourcePicklist;

pub(crate) const PHONE: [u8; 4] = [192, 0, 2, 20];
const PROVIDER: [u8; 4] = [203, 0, 113, 60];
const MEDIA: [u8; 4] = [203, 0, 113, 61];
const MEET: [u8; 4] = [198, 51, 100, 90];
/// 2026-11-10 14:36:00 in Berlin
const BASE: u64 = 1_794_317_760;
const S: u64 = 1_000_000;
const MS: u64 = 1_000;

/// The call IDs, in the order of the calls
pub(crate) const REJECTED: &str = "7c1e9a40d2b3@192.0.2.20";
pub(crate) const ENDED: &str = "3f2a9c1e7b54@192.0.2.20";
pub(crate) const BAD: &str = "b2c7f3e9-1d4a@203.0.113.60";
pub(crate) const MISSED: &str = "4a7e2c91f0b3@203.0.113.60";
pub(crate) const RUNNING: &str = "9e4b1f7a20c6@192.0.2.20";

/// The same numbers on every run
struct Dice(u64);

impl Dice {
    /// A whole number below `n`
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n.max(1)
    }
}

/// A number from 0 to 1 for `x`, the same on every run (SplitMix64)
fn unit(x: u64) -> f64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    #[allow(clippy::cast_precision_loss)]
    let unit = (z >> 11) as f64 / (1_u64 << 53) as f64;
    unit
}

/// How loud `speaker` (0 the phone, 1 the other side) is at `t` seconds of a call: turns of 2 to
/// 8 seconds, words of a third of a second, a few pauses (like the made-up curve of the draft)
fn loudness(speaker: u64, t: f64) -> f64 {
    let (mut end, mut turn) = (0.0, 0_u64);
    loop {
        end += 2.0 + 6.0 * unit(turn);
        if t < end {
            break;
        }
        turn += 1;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let word = (t / 0.36) as u64;
    if turn % 2 == speaker && unit(word * 7 + speaker + 1_000) > 0.12 {
        0.3 + 0.7 * unit(word * 13 + speaker + 2_000)
    } else {
        0.03 + 0.03 * unit(word * 5 + speaker + 3_000)
    }
}

/// Fills a payload of G.711 with a voice-like tone from sample `first` on
fn speech(law: audio::Law, speaker: u64, first: u64, payload: &mut [u8]) {
    #[allow(clippy::cast_precision_loss)]
    let at = |n: u64| n as f64 / f64::from(audio::RATE);
    let level = loudness(speaker, at(first));
    for (k, byte) in (first..).zip(payload.iter_mut()) {
        let t = at(k);
        let wave = 0.7 * (std::f64::consts::TAU * 170.0 * t).sin()
            + 0.3 * (std::f64::consts::TAU * 340.0 * t).sin();
        #[allow(clippy::cast_possible_truncation)]
        let value = (level * 16_000.0 * wave) as i16;
        *byte = match law {
            audio::Law::A => alaw_encode(value),
            audio::Law::Mu => ulaw_encode(value),
        };
    }
}

/// Linear to A-law (G.711): 13 bits, segment by the highest bit, four bits of step
pub(crate) fn alaw_encode(value: i16) -> u8 {
    const ENDS: [i32; 8] = [0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff];
    let mut v = i32::from(value) >> 3;
    let mask = if v >= 0 {
        0xd5
    } else {
        v = -v - 1;
        0x55
    };
    let Some(segment) = ENDS.iter().position(|end| v <= *end) else {
        return 0x7f ^ mask;
    };
    let shift = if segment < 2 { 1 } else { segment };
    let byte = (i32::try_from(segment).unwrap() << 4) | ((v >> shift) & 0x0f);
    u8::try_from(byte).unwrap() ^ mask
}

/// Linear to µ-law (G.711): 14 bits, the bias of 33 added, segment and step
pub(crate) fn ulaw_encode(value: i16) -> u8 {
    const ENDS: [i32; 8] = [0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff];
    let mut v = i32::from(value) >> 2;
    let mask = if v < 0 {
        v = -v;
        0x7f
    } else {
        0xff
    };
    v = v.min(8_159) + 33;
    let Some(segment) = ENDS.iter().position(|end| v <= *end) else {
        return 0x7f ^ mask;
    };
    let byte = (i32::try_from(segment).unwrap() << 4) | ((v >> (segment + 1)) & 0x0f);
    u8::try_from(byte).unwrap() ^ mask
}

/// One SIP dialog: who called whom
struct Dialog {
    id: &'static str,
    from: &'static str,
    to: &'static str,
}

#[derive(Default)]
struct File {
    /// Microseconds after `BASE`, and the frame
    packets: Vec<(u64, Vec<u8>)>,
}

impl File {
    /// A SIP message from the phone (`by_phone`) or from the provider
    #[allow(clippy::too_many_arguments)]
    fn sip(
        &mut self,
        at: u64,
        d: &Dialog,
        by_phone: bool,
        first: &str,
        cseq: &str,
        extra: &[(&str, &str)],
        sdp: Option<&str>,
    ) {
        // a response carries the Via of the request, from the other side
        let response = first.starts_with("SIP/2.0");
        let via = if by_phone != response {
            "SIP/2.0/UDP 192.0.2.20:5060;branch=z9hG4bK3f9a1c;rport"
        } else {
            "SIP/2.0/UDP 203.0.113.60:5060;branch=z9hG4bKsbc81d2"
        };
        let mut headers = vec![
            ("Via", via),
            ("From", d.from),
            ("To", d.to),
            ("Call-ID", d.id),
            ("CSeq", cseq),
        ];
        headers.extend_from_slice(extra);
        headers.push(if by_phone {
            ("User-Agent", "Linphone/5.2 (belle-sip)")
        } else {
            ("Server", "Beispiel-SBC/3.1")
        });
        let m = sip_message(first, &headers, sdp);
        let frame = if by_phone {
            udp_between(PHONE, 5060, PROVIDER, 5060, &m)
        } else {
            udp_between(PROVIDER, 5060, PHONE, 5060, &m)
        };
        self.packets.push((at, frame));
    }

    /// A stream of RTP: `count` sequence numbers every `step`, without those in `lost`; each
    /// packet is late by up to `late` microseconds, the ones in `spikes` by `spike` more
    #[allow(clippy::too_many_arguments)]
    fn rtp(
        &mut self,
        (src, sport): ([u8; 4], u16),
        (dst, dport): ([u8; 4], u16),
        (pt, ssrc, len): (u8, u32, usize),
        start: u64,
        (count, step, ticks): (u16, u64, u32),
        late: u64,
        lost: &dyn Fn(u16) -> bool,
        spikes: (&[u16], u64),
        dice: &mut Dice,
    ) {
        let first_seq = u16::try_from(ssrc % 30_000).unwrap();
        let first_ts = ssrc / 7;
        for i in 0..count {
            if lost(i) {
                continue;
            }
            let seq = first_seq.wrapping_add(i);
            let ts = first_ts.wrapping_add(u32::from(i) * ticks);
            let mut at = start + u64::from(i) * step + dice.below(late + 1);
            if spikes.0.contains(&i) {
                at += spikes.1;
            }
            let mut m = rtp_bytes(pt, seq, ts, ssrc, i == 0, len);
            // G.711 gets made-up speech: the phone speaks first, then the sides take turns
            let law = match pt {
                0 => Some(audio::Law::Mu),
                8 => Some(audio::Law::A),
                _ => None,
            };
            if let Some(law) = law {
                let speaker = u64::from(src != PHONE);
                speech(law, speaker, u64::from(i) * u64::from(ticks), &mut m[12..]);
            }
            self.packets
                .push((at, udp_between(src, sport, dst, dport, &m)));
        }
    }

    /// RTCP every ten seconds from both sides
    fn rtcp(
        &mut self,
        (near, far): ((u16, u32), (u16, u32)),
        far_ip: [u8; 4],
        start: u64,
        reports: u64,
        far_jitter: u32,
    ) {
        for i in 0..reports {
            let at = start + 5 * S + i * 10 * S;
            let mine = rtcp_report_on(near.1, far.1, 0, 40, "anna@192.0.2.20");
            self.packets
                .push((at, udp_between(PHONE, near.0, far_ip, far.0, &mine)));
            let theirs = rtcp_report_on(far.1, near.1, 0, far_jitter, "sbc@203.0.113.61");
            self.packets.push((
                at + 31 * MS,
                udp_between(far_ip, far.0, PHONE, near.0, &theirs),
            ));
        }
    }

    fn write(mut self) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        self.packets.sort_by_key(|(at, _)| *at);
        let dir = std::env::temp_dir().join(format!(
            "openpacketmon-calls-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("calls.pcap");
        let mut out = Vec::new();
        out.extend_from_slice(&0xa1b2_c3d4_u32.to_le_bytes());
        out.extend_from_slice(&2_u16.to_le_bytes());
        out.extend_from_slice(&4_u16.to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&65_535_u32.to_le_bytes());
        out.extend_from_slice(&1_u32.to_le_bytes());
        for (us, data) in &self.packets {
            let len = u32::try_from(data.len()).unwrap();
            let secs = u32::try_from(BASE + us / S).unwrap();
            out.extend_from_slice(&secs.to_le_bytes());
            out.extend_from_slice(&u32::try_from(us % S).unwrap().to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(data);
        }
        std::fs::write(&path, out).unwrap();
        path
    }
}

/// The capture file of the draft's calls, in a folder of its own
#[allow(clippy::too_many_lines)]
pub(crate) fn calls_file() -> PathBuf {
    let mut f = File::default();
    let mut dice = Dice(0x5eed_cafe_f00d_0001);
    let none = |_: u16| false;
    let contact = [("Contact", "<sip:anna@192.0.2.20:5060>")];
    let sbc = [("Contact", "<sip:203.0.113.60:5060>")];
    let auth = (
        "Proxy-Authorization",
        "Digest username=\"anna\", realm=\"sip.example.net\", nonce=\"5f1c2e9a\", \
         uri=\"sip:support@example.com\", response=\"0e5b3a9c7d21f4e8a6b0c2d4e6f80a1b\"",
    );

    // 14:36:09 the phone registers, after a challenge
    let reg = Dialog {
        id: "5b0e2d8c41a7@192.0.2.20",
        from: "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=r81d",
        to: "<sip:anna@sip.example.net>",
    };
    let t = 9 * S + 120 * MS;
    let expires = [
        ("Contact", "<sip:anna@192.0.2.20:5060>"),
        ("Expires", "3600"),
    ];
    let first = "REGISTER sip:sip.example.net SIP/2.0";
    f.sip(t, &reg, true, first, "1 REGISTER", &expires, None);
    let challenge = [(
        "WWW-Authenticate",
        "Digest realm=\"sip.example.net\", nonce=\"5f1c2e9a\", algorithm=MD5",
    )];
    f.sip(
        t + 21 * MS,
        &reg,
        false,
        "SIP/2.0 401 Unauthorized",
        "1 REGISTER",
        &challenge,
        None,
    );
    let answer = [
        ("Contact", "<sip:anna@192.0.2.20:5060>"),
        ("Expires", "3600"),
        (
            "Authorization",
            "Digest username=\"anna\", realm=\"sip.example.net\", nonce=\"5f1c2e9a\", \
             uri=\"sip:sip.example.net\", response=\"7d21f4e80e5b3a9ca6b0c2d4e6f80a1b\"",
        ),
    ];
    f.sip(t + 40 * MS, &reg, true, first, "2 REGISTER", &answer, None);
    let ok = [("Contact", "<sip:anna@192.0.2.20:5060>;expires=3600")];
    f.sip(
        t + 63 * MS,
        &reg,
        false,
        "SIP/2.0 200 OK",
        "2 REGISTER",
        &ok,
        None,
    );

    // 14:37:20 a video conference in the browser: RTP without SIP, 1:30
    let start = 80 * S + 428 * MS;
    f.rtp(
        (PHONE, 52_714),
        (MEET, 10_000),
        (111, 0x9b2e_7714, 140),
        start,
        (1_000, 90 * MS, 4_320),
        7 * MS,
        &none,
        (&[], 0),
        &mut dice,
    );
    f.rtp(
        (MEET, 10_000),
        (PHONE, 52_714),
        (111, 0xc40a_3f86, 150),
        start + 13 * MS,
        (1_003, 90 * MS, 4_320),
        40 * MS,
        &|i| matches!(i, 211 | 640 | 641),
        (&[], 0),
        &mut dice,
    );

    // 14:39:06 to sales, after a challenge: busy
    let d = Dialog {
        id: REJECTED,
        from: "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=a40d2b",
        to: "<sip:vertrieb@example.com>",
    };
    let t = 186 * S + 41 * MS;
    let offer = sdp_offer("192.0.2.20", 7076, &[8, 0, 101]);
    let first = "INVITE sip:vertrieb@example.com SIP/2.0";
    f.sip(t, &d, true, first, "1 INVITE", &contact, Some(&offer));
    let challenge = [(
        "Proxy-Authenticate",
        "Digest realm=\"sip.example.net\", nonce=\"9a2c7e11\", algorithm=MD5",
    )];
    let required = "SIP/2.0 407 Proxy Authentication Required";
    f.sip(
        t + 8 * MS,
        &d,
        false,
        required,
        "1 INVITE",
        &challenge,
        None,
    );
    let ack = "ACK sip:vertrieb@example.com SIP/2.0";
    f.sip(t + 25 * MS, &d, true, ack, "1 ACK", &[], None);
    let with_auth = [contact[0], auth];
    f.sip(
        t + 40 * MS,
        &d,
        true,
        first,
        "2 INVITE",
        &with_auth,
        Some(&offer),
    );
    f.sip(
        t + 62 * MS,
        &d,
        false,
        "SIP/2.0 100 Trying",
        "2 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 144 * MS,
        &d,
        false,
        "SIP/2.0 486 Busy Here",
        "2 INVITE",
        &[],
        None,
    );
    f.sip(t + 167 * MS, &d, true, ack, "2 ACK", &[], None);

    // 14:40:57 to support: answered after 8.9 s, 1:31 of G.711 A-law, hung up here
    let d = Dialog {
        id: ENDED,
        from: "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=c81d07",
        to: "<sip:support@example.com>",
    };
    let t = 297 * S + 679 * MS;
    let offer = sdp_offer("192.0.2.20", 7078, &[8, 0, 101]);
    let answer = sdp_offer("203.0.113.61", 31_456, &[8, 101]);
    let first = "INVITE sip:support@example.com SIP/2.0";
    f.sip(t, &d, true, first, "1 INVITE", &with_auth, Some(&offer));
    f.sip(
        t + 22 * MS,
        &d,
        false,
        "SIP/2.0 100 Trying",
        "1 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 78 * MS,
        &d,
        false,
        "SIP/2.0 180 Ringing",
        "1 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 8_902 * MS,
        &d,
        false,
        "SIP/2.0 200 OK",
        "1 INVITE",
        &sbc,
        Some(&answer),
    );
    let ack = "ACK sip:support@example.com SIP/2.0";
    f.sip(t + 8_915 * MS, &d, true, ack, "1 ACK", &[], None);
    let rtp = t + 8_939 * MS;
    f.rtp(
        (PHONE, 7_078),
        (MEDIA, 31_456),
        (8, 0x5e1f_02a7, 480),
        rtp,
        (1_502, 60 * MS, 480),
        2 * MS,
        &none,
        (&[], 0),
        &mut dice,
    );
    f.rtp(
        (MEDIA, 31_456),
        (PHONE, 7_078),
        (8, 0x0b77_c3d1, 480),
        rtp + 7 * MS,
        (1_502, 60 * MS, 480),
        19 * MS,
        &none,
        (&[], 0),
        &mut dice,
    );
    f.rtcp(
        ((7_079, 0x5e1f_02a7), (31_457, 0x0b77_c3d1)),
        MEDIA,
        rtp,
        9,
        5,
    );
    let bye = "BYE sip:support@example.com SIP/2.0";
    f.sip(t + 99_586 * MS, &d, true, bye, "2 BYE", &[], None);
    f.sip(
        t + 99_592 * MS,
        &d,
        false,
        "SIP/2.0 200 OK",
        "2 BYE",
        &[],
        None,
    );

    // 14:43:45 from a company: 1:00 of G.711 µ-law, bad on the way back, hung up there
    let d = Dialog {
        id: BAD,
        from: "\"Beispiel GmbH\" <sip:zentrale@example.org>;tag=7f3e9",
        to: "<sip:anna@sip.example.net>",
    };
    let t = 465 * S + 149 * MS;
    let offer = sdp_offer("203.0.113.61", 40_112, &[0, 8, 101]);
    let answer = sdp_offer("192.0.2.20", 7_080, &[0, 101]);
    let first = "INVITE sip:anna@192.0.2.20:5060 SIP/2.0";
    f.sip(t, &d, false, first, "1 INVITE", &sbc, Some(&offer));
    f.sip(
        t + 13 * MS,
        &d,
        true,
        "SIP/2.0 100 Trying",
        "1 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 31 * MS,
        &d,
        true,
        "SIP/2.0 180 Ringing",
        "1 INVITE",
        &contact,
        None,
    );
    f.sip(
        t + 4_528 * MS,
        &d,
        true,
        "SIP/2.0 200 OK",
        "1 INVITE",
        &contact,
        Some(&answer),
    );
    let ack = "ACK sip:anna@192.0.2.20:5060 SIP/2.0";
    f.sip(t + 4_545 * MS, &d, false, ack, "1 ACK", &[], None);
    let rtp = t + 4_556 * MS;
    f.rtp(
        (PHONE, 7_080),
        (MEDIA, 40_112),
        (0, 0x2c90_e415, 480),
        rtp,
        (1_000, 60 * MS, 480),
        2 * MS,
        &none,
        (&[], 0),
        &mut dice,
    );
    // nine gaps of five packets, three that come too late
    f.rtp(
        (MEDIA, 40_112),
        (PHONE, 7_080),
        (0, 0x7a33_61f0, 480),
        rtp + 9 * MS,
        (1_044, 60 * MS, 480),
        58 * MS,
        &|i| i < 900 && (40..45).contains(&(i % 100)),
        (&[123, 456, 789], 150 * MS),
        &mut dice,
    );
    f.rtcp(
        ((7_081, 0x2c90_e415), (40_113, 0x7a33_61f0)),
        MEDIA,
        rtp,
        6,
        6,
    );
    let bye = "BYE sip:anna@192.0.2.20:5060 SIP/2.0";
    f.sip(t + 64_874 * MS, &d, false, bye, "2 BYE", &[], None);
    f.sip(
        t + 64_888 * MS,
        &d,
        true,
        "SIP/2.0 200 OK",
        "2 BYE",
        &[],
        None,
    );

    // 14:46:08 from a hidden number: rang for six seconds, then the caller gave up
    let d = Dialog {
        id: MISSED,
        from: "\"Anonymous\" <sip:anonymous@anonymous.invalid>;tag=e2c91",
        to: "<sip:anna@sip.example.net>",
    };
    let t = 608 * S + 2 * MS;
    let offer = sdp_offer("203.0.113.61", 40_200, &[8, 0, 101]);
    let first = "INVITE sip:anna@192.0.2.20:5060 SIP/2.0";
    let hidden = [sbc[0], ("Privacy", "id")];
    f.sip(t, &d, false, first, "1 INVITE", &hidden, Some(&offer));
    f.sip(
        t + 12 * MS,
        &d,
        true,
        "SIP/2.0 100 Trying",
        "1 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 40 * MS,
        &d,
        true,
        "SIP/2.0 180 Ringing",
        "1 INVITE",
        &contact,
        None,
    );
    let cancel = "CANCEL sip:anna@192.0.2.20:5060 SIP/2.0";
    f.sip(t + 6_010 * MS, &d, false, cancel, "1 CANCEL", &[], None);
    f.sip(
        t + 6_022 * MS,
        &d,
        true,
        "SIP/2.0 200 OK",
        "1 CANCEL",
        &[],
        None,
    );
    let terminated = "SIP/2.0 487 Request Terminated";
    f.sip(t + 6_025 * MS, &d, true, terminated, "1 INVITE", &[], None);
    let ack = "ACK sip:anna@192.0.2.20:5060 SIP/2.0";
    f.sip(t + 6_040 * MS, &d, false, ack, "1 ACK", &[], None);

    // 14:47:42 to the technicians: still talking when the file ends, 0:26 so far
    let d = Dialog {
        id: RUNNING,
        from: "\"Anna Beispiel\" <sip:anna@sip.example.net>;tag=4b1f7a",
        to: "<sip:technik@example.org>",
    };
    let t = 702 * S + 311 * MS;
    let offer = sdp_offer("192.0.2.20", 7_082, &[8, 0, 101]);
    let answer = sdp_offer("203.0.113.61", 31_610, &[8, 101]);
    let first = "INVITE sip:technik@example.org SIP/2.0";
    f.sip(t, &d, true, first, "1 INVITE", &with_auth, Some(&offer));
    f.sip(
        t + 20 * MS,
        &d,
        false,
        "SIP/2.0 100 Trying",
        "1 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 70 * MS,
        &d,
        false,
        "SIP/2.0 180 Ringing",
        "1 INVITE",
        &[],
        None,
    );
    f.sip(
        t + 2_400 * MS,
        &d,
        false,
        "SIP/2.0 200 OK",
        "1 INVITE",
        &sbc,
        Some(&answer),
    );
    let ack = "ACK sip:technik@example.org SIP/2.0";
    f.sip(t + 2_410 * MS, &d, true, ack, "1 ACK", &[], None);
    let rtp = t + 2_430 * MS;
    f.rtp(
        (PHONE, 7_082),
        (MEDIA, 31_610),
        (8, 0x31d8_a5c2, 480),
        rtp,
        (434, 60 * MS, 480),
        2 * MS,
        &none,
        (&[], 0),
        &mut dice,
    );
    f.rtp(
        (MEDIA, 31_610),
        (PHONE, 7_082),
        (8, 0x6f02_b9e4, 480),
        rtp + 8 * MS,
        (434, 60 * MS, 480),
        12 * MS,
        &none,
        (&[], 0),
        &mut dice,
    );
    f.rtcp(
        ((7_083, 0x31d8_a5c2), (31_611, 0x6f02_b9e4)),
        MEDIA,
        rtp,
        2,
        4,
    );
    f.write()
}

/// An engine that has read the file of the draft's calls
pub(crate) struct Calls {
    _runtime: tokio::runtime::Runtime,
    pub(crate) engine: Engine,
    dir: PathBuf,
}

impl Drop for Calls {
    fn drop(&mut self) {
        self.engine.stop();
        remove_test_dir(&self.dir);
    }
}

pub(crate) fn engine_with_calls() -> Calls {
    let path = calls_file();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let engine = Engine::new(Conf::default(), runtime.handle().clone());
    {
        let mut state = engine.lock();
        state.set_capture_source(CaptureSourcePicklist::File);
        state.set_pcap_import(path.to_string_lossy().to_string());
    }
    assert!(engine.start());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !engine.lock().traffic_history.no_more_packets() {
        assert!(
            std::time::Instant::now() < deadline,
            "the file was not read"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Calls {
        _runtime: runtime,
        engine,
        dir: path.parent().unwrap().to_path_buf(),
    }
}

fn row<'a>(list: &'a CallList, label: &str) -> &'a CallRow {
    list.rows
        .iter()
        .find(|r| r.to.name == label || r.from.name == label)
        .unwrap_or_else(|| panic!("no row for {label}"))
}

#[test]
#[serial]
fn the_list_shows_the_calls_of_the_draft() {
    let calls = engine_with_calls();
    let t = Lang::De;
    let list = call_list(t, &calls.engine);
    assert!(!list.live);
    assert_eq!(count(&calls.engine), 5);
    // newest first, the stream without SIP last
    let states: Vec<State> = list.rows.iter().map(|r| r.status.state).collect();
    assert_eq!(
        states,
        [
            State::Running,
            State::Missed,
            State::Ended,
            State::Ended,
            State::Rejected,
            State::Rtp
        ]
    );
    let tiles: Vec<(&str, &str)> = list
        .tiles
        .iter()
        .map(|tile| (tile.value.as_str(), tile.sub.as_str()))
        .collect();
    assert_eq!(tiles[0], ("5", "davon 1 läuft"));
    // 1:31 + 1:00 + 0:26
    assert_eq!(tiles[1], ("3", "Gesprächszeit 2:57"));
    assert_eq!(tiles[2].0, "2");
    assert_eq!(tiles[3].0, "1");
    assert_eq!(tiles[4].0, "1");

    let ended = row(&list, "support@example.com");
    assert!(ended.outgoing);
    assert_eq!(ended.duration, "1:31");
    assert_eq!(ended.codec, "G.711 A-law");
    assert_eq!(ended.rate, Some(Rate::Good));
    assert_eq!(ended.from.name, "Anna Beispiel");
    assert_eq!(ended.status.sub, "selbst aufgelegt");
    assert_eq!(ended.packets, "3.029");
    assert!(ended.playable);

    let bad = row(&list, "Beispiel GmbH");
    assert!(!bad.outgoing);
    assert_eq!(bad.duration, "1:00");
    assert_eq!(bad.codec, "G.711 µ-law");
    assert_eq!(bad.rate, Some(Rate::Bad));
    assert_eq!(bad.status.sub, "Gegenseite hat aufgelegt");
    assert_eq!(bad.to.name, "Anna Beispiel");

    let rejected = row(&list, "vertrieb@example.com");
    assert_eq!(rejected.status.sub, "486 Busy Here (besetzt)");
    assert_eq!(rejected.duration_sub, "nach 0,1 s");
    assert_eq!(rejected.packets, "7");
    assert!(!rejected.playable);

    let missed = list
        .rows
        .iter()
        .find(|r| r.status.state == State::Missed)
        .unwrap();
    assert!(!missed.outgoing);
    assert_eq!(missed.duration_sub, "geklingelt 6,0 s");
    assert_eq!(missed.from.name, "Anonymous");

    let running = row(&list, "technik@example.org");
    assert_eq!(running.duration, "0:26");

    let rtp = list.rows.last().unwrap();
    assert!(rtp.outgoing);
    assert_eq!(rtp.from.name, "192.0.2.20");
    assert_eq!(rtp.duration, "1:30");
    assert_eq!(rtp.codec, "unbekannt");
    assert_eq!(rtp.packets, "2.000");

    let registration = list.registration.as_ref().unwrap();
    assert!(registration.account.contains("anna@sip.example.net"));
}

#[test]
#[serial]
fn a_call_page_shows_its_flow_and_its_streams() {
    let calls = engine_with_calls();
    let t = Lang::De;
    let list = call_list(t, &calls.engine);
    let key = row(&list, "support@example.com").key;
    let page = call_page(t, &calls.engine, key).unwrap();
    assert!(page.sip);
    assert_eq!(page.from, "Anna Beispiel");
    assert_eq!(page.to, "support@example.com");
    assert!(page.notice.is_none());
    assert_eq!(page.filter, format!("voip.call_id == \"{ENDED}\""));
    let labels: Vec<&str> = page.facts.iter().map(|f| f.label.as_str()).collect();
    for label in [
        "Zustand",
        "Von",
        "An",
        "Richtung",
        "Codec",
        "Call-ID",
        "Anmeldung",
        "Pakete",
    ] {
        assert!(labels.contains(&label), "{label} in {labels:?}");
    }
    // the ladder: seven messages and one band of RTP
    let messages = page
        .flow
        .rows
        .iter()
        .filter(|r| matches!(r, FlowRow::Message { .. }))
        .count();
    let bands = page.flow.rows.len() - messages;
    assert_eq!((messages, bands), (7, 1));
    assert_eq!(page.flow.lanes.len(), 3);
    // sent first, then received; nothing lost
    assert_eq!(page.streams.len(), 2);
    assert_eq!(page.streams[0].packets, "1.502");
    assert!(page.streams[0].sent);
    assert!(!page.streams[1].sent);
    assert!(page.streams.iter().all(|s| s.rate == Some(Rate::Good)));
    assert!(page.rtcp.contains("18"), "{}", page.rtcp);

    // the bad call: the notice names the received stream
    let key = row(&list, "Beispiel GmbH").key;
    let page = call_page(t, &calls.engine, key).unwrap();
    let (_, notice) = page.notice.unwrap();
    assert!(notice.contains("4,3 %"), "{notice}");
    let received = page.streams.iter().find(|s| !s.sent).unwrap();
    assert_eq!(received.packets, "999");
    assert_eq!(received.rate, Some(Rate::Bad));
    assert_eq!(received.late, "3");
    assert!(received.lost.as_ref().unwrap().0.starts_with("45"));

    // the rejected call: no streams, a reason why
    let key = row(&list, "vertrieb@example.com").key;
    let page = call_page(t, &calls.engine, key).unwrap();
    assert!(page.streams.is_empty());
    assert!(page.no_streams.is_some());
    assert!(matches!(page.listen, Listen::Off { .. }));
}

#[test]
#[serial]
fn a_stream_without_sip_has_a_page_of_its_own() {
    let calls = engine_with_calls();
    let t = Lang::De;
    let list = call_list(t, &calls.engine);
    let rtp = list.rows.last().unwrap();
    assert!(matches!(rtp.key, CallKey::Rtp(_)));
    let page = call_page(t, &calls.engine, rtp.key).unwrap();
    assert!(!page.sip);
    assert_eq!(page.from, "192.0.2.20");
    assert!(page.filter.contains("0x9b2e7714"), "{}", page.filter);
    assert!(matches!(page.listen, Listen::Off { .. }));
    assert_eq!(page.streams.len(), 2);
    assert!(page.streams[0].sent);
    assert_eq!(page.streams[0].jitter, None);
    let lost = page.streams[1].lost.as_ref().unwrap();
    assert!(lost.0.starts_with('3'), "{lost:?}");
}

#[test]
#[serial]
fn packets_know_their_call_and_bad_calls_are_named() {
    let calls = engine_with_calls();
    let t = Lang::De;
    let bad = bad_calls(t, &calls.engine);
    assert_eq!(bad.len(), 1);
    assert!(bad[0].text.contains("4,3 %"), "{}", bad[0].text);
    // the first INVITE of the call to support
    let list = call_list(t, &calls.engine);
    let key = row(&list, "support@example.com").key;
    let number = {
        let store = calls.engine.lock().packets.clone().unwrap();
        crate::engine::analysis::advance(&store, u64::MAX);
        let analysis = store.analysis();
        (1..=10_000)
            .find(|n| key_of(&analysis, *n) == Some(key))
            .unwrap()
    };
    let (found, label) = call_of(t, &calls.engine, number).unwrap();
    assert_eq!(found, key);
    assert_eq!(label, "Anna Beispiel → support@example.com");
}

#[test]
fn an_engine_without_packets_has_no_calls() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let engine = Engine::new(Conf::default(), runtime.handle().clone());
    assert_eq!(count(&engine), 0);
    let list = call_list(Lang::De, &engine);
    assert!(list.rows.is_empty());
    assert!(list.registration.is_none());
}

#[test]
fn g711_decodes_after_the_tables_of_the_recommendation() {
    // A-law: the smallest steps either side of zero and the largest values
    assert_eq!(alaw(0xd5), 8);
    assert_eq!(alaw(0x55), -8);
    assert_eq!(alaw(0xaa), 32_256);
    assert_eq!(alaw(0x2a), -32_256);
    // µ-law: zero, the largest values, and the second zero
    assert_eq!(ulaw(0xff), 0);
    assert_eq!(ulaw(0x7f), 0);
    assert_eq!(ulaw(0x80), 32_124);
    assert_eq!(ulaw(0x00), -32_124);
    // every value of a code lies in the middle of its step: encoded again it gives the same code
    for byte in 0..=u8::MAX {
        assert_eq!(alaw_encode(alaw(byte)), byte, "A-law {byte:#04x}");
        if byte != 0x7f {
            assert_eq!(ulaw_encode(ulaw(byte)), byte, "µ-law {byte:#04x}");
        }
    }
    // and both grow with the value
    let a: Vec<i16> = (0..128).map(|b| alaw(0x80 | (b ^ 0x55))).collect();
    assert!(a.windows(2).all(|w| w[0] < w[1]), "{a:?}");
    let u: Vec<i16> = (0..128_u8).map(|b| ulaw(0xff - b)).collect();
    assert!(u.windows(2).all(|w| w[0] < w[1]), "{u:?}");
}

fn tiny_audio() -> CallAudio {
    CallAudio {
        sent: std::sync::Arc::from([1_000, -2_000, 30_000]),
        received: std::sync::Arc::from([4_000]),
        seconds: 3.0 / 8_000.0,
        sent_bars: Vec::new(),
        received_bars: Vec::new(),
        lost: false,
        sent_name: String::new(),
        received_name: String::new(),
        running: false,
        file_name: String::new(),
    }
}

#[test]
fn a_wav_file_has_both_sides_at_8_khz() {
    let wav = tiny_audio().wav();
    assert_eq!(wav.len(), 44 + 3 * 4);
    let u16_at = |i: usize| u16::from_le_bytes([wav[i], wav[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([wav[i], wav[i + 1], wav[i + 2], wav[i + 3]]);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(u32_at(4), 36 + 12);
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    assert_eq!(u32_at(16), 16);
    // PCM, stereo, 8000 Hz, 32000 bytes per second, 4 bytes per frame, 16 bit
    assert_eq!((u16_at(20), u16_at(22)), (1, 2));
    assert_eq!((u32_at(24), u32_at(28)), (8_000, 32_000));
    assert_eq!((u16_at(32), u16_at(34)), (4, 16));
    assert_eq!(&wav[36..40], b"data");
    assert_eq!(u32_at(40), 12);
    // left sent, right received, the shorter side filled with silence
    let samples: Vec<i16> = wav[44..]
        .chunks(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    assert_eq!(samples, [1_000, 4_000, -2_000, 0, 30_000, 0]);
}

#[test]
fn the_output_hears_one_side_or_both() {
    let audio = tiny_audio();
    let scale = |v: &[i32]| v.iter().map(|v| *v as f32 / 32_768.0).collect::<Vec<_>>();
    assert_eq!(
        audio.samples(Channel::Both, 0.0),
        scale(&[5_000, -2_000, 30_000])
    );
    assert_eq!(
        audio.samples(Channel::Sent, 0.0),
        scale(&[1_000, -2_000, 30_000])
    );
    assert_eq!(audio.samples(Channel::Received, 0.0), scale(&[4_000, 0, 0]));
    // from a time on; sums stay within 16 bit
    assert_eq!(
        audio.samples(Channel::Sent, 2.0 / 8_000.0),
        scale(&[30_000])
    );
    let loud = CallAudio {
        received: std::sync::Arc::from([0, 0, 30_000]),
        ..tiny_audio()
    };
    assert_eq!(loud.samples(Channel::Both, 0.0)[2], 32_767.0 / 32_768.0);
}

fn rms(samples: &[i16]) -> f64 {
    let sum: f64 = samples.iter().map(|v| f64::from(*v).powi(2)).sum();
    #[allow(clippy::cast_precision_loss)]
    let n = samples.len().max(1) as f64;
    (sum / n).sqrt()
}

#[test]
#[serial]
fn a_call_with_g711_can_be_heard() {
    let calls = engine_with_calls();
    let t = Lang::De;
    let list = call_list(t, &calls.engine);

    // the ended call: A-law, 1502 packets of 60 ms each way, nothing lost; the timeline is the
    // talk time from the answer to the BYE (8.902 s to 99.586 s), 1:31 as in the draft
    let key = row(&list, "support@example.com").key;
    let audio = call_audio(t, &calls.engine, key).unwrap();
    assert!((audio.seconds - 90.684).abs() < 0.001, "{}", audio.seconds);
    // the first voice 37 ms after the answer
    let lead = audio.sent.iter().take_while(|v| **v == 0).count();
    assert!((290..=320).contains(&lead), "{lead}");
    assert!(audio.sent.len() >= lead + 1_502 * 480);
    assert!(!audio.lost && !audio.running);
    assert_eq!(audio.sent_name, "Anna Beispiel");
    assert_eq!(audio.received_name, "support@example.com");
    assert!(audio.file_name.starts_with("openpacketmon-call-2026111"));
    assert!(audio.file_name.ends_with(".wav"));
    // the phone speaks first: the sides are not swapped
    let second = 8_000;
    assert!(rms(&audio.sent[..second]) > 4.0 * rms(&audio.received[..second]));
    // the waveform: 150 bars that follow the speech, none red
    assert_eq!(audio.sent_bars.len(), 150);
    assert_eq!(audio.received_bars.len(), 150);
    assert!(
        audio
            .sent_bars
            .iter()
            .all(|b| !b.gap && (0.0..=1.0).contains(&b.level))
    );
    assert!(audio.sent_bars.iter().any(|b| b.level > 0.6));
    assert!(audio.received_bars.iter().any(|b| b.level < 0.1));

    // the bad call: µ-law, the lost packets of the received side are silent and red
    let key = row(&list, "Beispiel GmbH").key;
    let audio = call_audio(t, &calls.engine, key).unwrap();
    assert!(audio.lost);
    assert_eq!(audio.sent_name, "Anna Beispiel");
    assert!(audio.sent_bars.iter().all(|b| !b.gap));
    let red = audio.received_bars.iter().filter(|b| b.gap).count();
    assert!((9..=18).contains(&red), "{red}");
    // 9 gaps of 5 packets of 480 samples: silence in the middle of speech
    let silent = audio.received.iter().filter(|v| **v == 0).count();
    assert!(silent >= 45 * 480, "{silent}");

    // the running call: what is there so far
    let key = row(&list, "technik@example.org").key;
    let audio = call_audio(t, &calls.engine, key).unwrap();
    assert!(audio.running);

    // nothing to hear: rejected, missed, RTP without SIP
    for label in ["vertrieb@example.com"] {
        let key = row(&list, label).key;
        assert!(call_audio(t, &calls.engine, key).is_none(), "{label}");
    }
    let rtp = list.rows.last().unwrap();
    assert!(call_audio(t, &calls.engine, rtp.key).is_none());
    let missed = list
        .rows
        .iter()
        .find(|r| r.status.state == State::Missed)
        .unwrap();
    assert!(call_audio(t, &calls.engine, missed.key).is_none());
}
