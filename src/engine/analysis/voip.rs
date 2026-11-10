//! The analysis of calls: SIP transactions and calls (RFC 3261), the media streams their
//! session descriptions negotiate (RFC 8866), and the RTP and RTCP datagrams of those streams
//! (RFC 3550). Without SIP, a UDP flow counts as RTP by its form: version 2, the same SSRC and
//! consecutive sequence numbers over [`CONFIRM`] packets (Settings → Calls).
//!
//! Written for openpacketmon from the specifications; the rules for finding RTP without SIP are a
//! simple heuristic of our own.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::dissect::{self, Dissection, Value, rtp};

/// RTP is also recognized without SIP, by the form of a flow (Settings → Calls)
static WITHOUT_SIP: AtomicBool = AtomicBool::new(true);

/// Packets of one SSRC with consecutive sequence numbers before a flow without SIP counts as RTP
pub const CONFIRM: usize = 10;
/// Largest step of the sequence number between two packets that still counts as consecutive
/// (a packet or two lost on the way)
const MAX_STEP: u16 = 3;
/// Media streams kept per capture; more are not followed
const MAX_MEDIA: usize = 100_000;
/// Calls kept per capture
const MAX_CALLS: usize = 100_000;
/// UDP flows watched for the form of RTP at a time
const MAX_FLOWS: usize = 50_000;
/// SIP requests waiting for their responses
const MAX_TRANSACTIONS: usize = 10_000;
/// Media addresses announced by session descriptions
const MAX_ENDPOINTS: usize = 100_000;

/// Whether RTP is recognized without SIP
pub fn without_sip() -> bool {
    WITHOUT_SIP.load(Ordering::Relaxed)
}

/// Sets whether RTP is recognized without SIP (Settings → Calls)
pub fn set_without_sip(on: bool) {
    WITHOUT_SIP.store(on, Ordering::Relaxed);
}

/// What the analysis found about a packet of a call
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// A SIP message: the call it belongs to (in [`Voip::calls`]); a response also the request it
    /// answers (packet number, microseconds after it) and its status code
    Sip {
        call: Option<u32>,
        answers: Option<(u64, u32)>,
        status: Option<u16>,
    },
    /// RTP or RTCP of a media stream (in [`Voip::media`])
    Media { media: u32, rtcp: bool },
    /// Has the form of RTP, but its flow is not decided yet
    Candidate,
}

/// A media stream: announced by a session description, or found by its form
pub struct Media {
    /// The SIP packet whose session description announced it; `None` if found by its form
    pub setup: Option<u64>,
    pub call: Option<u32>,
    pub formats: Arc<[(u8, String)]>,
    /// SRTP was negotiated
    pub srtp: bool,
    /// The payload type of its newest RTP packet, for the clock rate of the reports on it
    pub last_type: Option<u8>,
    /// The flow it was found in by its form (source, then destination)
    pub ends: Option<(SocketAddr, SocketAddr)>,
}

/// What [`super::annotate`] adds to a packet of a call
#[derive(Clone, Debug)]
pub struct Info {
    /// The call the packet belongs to (`voip.call_id`)
    pub call_id: Option<Arc<str>>,
    pub media: Option<MediaInfo>,
}

/// An RTP or RTCP packet and its stream
#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub rtcp: bool,
    pub setup: Option<u64>,
    pub srtp: bool,
    pub formats: Arc<[(u8, String)]>,
    pub clock: Option<u32>,
}

/// What a packet changed: its own mark, and earlier packets of a flow now decided
#[derive(Default)]
pub struct Found {
    pub mark: Option<Mark>,
    /// Earlier packets of the flow, now known as RTP of this mark
    pub confirmed: Vec<u64>,
    /// Earlier packets of a flow that turned out not to be RTP
    pub dropped: Vec<u64>,
}

/// A UDP flow in one direction
type FlowKey = (IpAddr, u16, IpAddr, u16);

enum Flow {
    /// Packets of one SSRC so far, with the newest sequence number
    Candidate {
        ssrc: u32,
        seq: u16,
        numbers: Vec<u64>,
    },
    /// RTP of this media stream
    Media(u32),
}

/// The analysis of the calls of one capture
#[derive(Default)]
pub struct Voip {
    /// The Call-IDs of the calls (dialogs started by INVITE)
    calls: Vec<Arc<str>>,
    /// The packet that started each call (its first INVITE)
    starts: Vec<u64>,
    call_index: HashMap<Arc<str>, u32>,
    /// Requests by Call-ID, `CSeq` number and method: packet number and time
    transactions: HashMap<(Arc<str>, u32, String), (u64, i64)>,
    /// Where media is sent, by address and port: the stream and whether it is the port of RTCP
    endpoints: HashMap<(IpAddr, u16), (u32, bool)>,
    flows: HashMap<FlowKey, Flow>,
    media: Vec<Media>,
}

impl Voip {
    /// The Call-ID of a call
    pub fn call_id(&self, call: u32) -> Option<&Arc<str>> {
        self.calls.get(call as usize)
    }

    /// The packet that started a call
    pub fn start(&self, call: u32) -> Option<u64> {
        self.starts.get(call as usize).copied()
    }

    /// A media stream
    pub fn media(&self, media: u32) -> Option<&Media> {
        self.media.get(media as usize)
    }

    /// The stream found by its form in the other direction of the same two ends
    pub fn partner(&self, media: u32) -> Option<u32> {
        let (src, dst) = self.media(media)?.ends?;
        self.media
            .iter()
            .position(|m| m.ends == Some((dst, src)))
            .and_then(|i| u32::try_from(i).ok())
    }

    /// What [`super::annotate`] needs to know about a packet with this mark
    pub fn info(&self, mark: Option<Mark>) -> Option<Info> {
        match mark? {
            Mark::Sip { call, .. } => Some(Info {
                call_id: call.and_then(|c| self.call_id(c)).cloned(),
                media: None,
            }),
            Mark::Media { media, rtcp } => {
                let m = self.media(media)?;
                let clock = m
                    .last_type
                    .or_else(|| m.formats.first().map(|(pt, _)| *pt))
                    .and_then(|pt| rtp::clock_rate(pt, &m.formats));
                Some(Info {
                    call_id: m.call.and_then(|c| self.call_id(c)).cloned(),
                    media: Some(MediaInfo {
                        rtcp,
                        setup: m.setup,
                        srtp: m.srtp,
                        formats: m.formats.clone(),
                        clock,
                    }),
                })
            }
            Mark::Candidate => None,
        }
    }

    /// Looks at a packet from `src` to `dst`, dissected as `d`
    pub fn feed(
        &mut self,
        number: u64,
        time: i64,
        (src, dst): (IpAddr, IpAddr),
        d: &Dissection,
        data: &[u8],
    ) -> Found {
        if d.protocols.contains(&"sip") {
            return Found {
                mark: Some(self.sip(number, time, d)),
                ..Found::default()
            };
        }
        let Some((sport, dport, payload)) = d.plain_udp() else {
            return Found::default();
        };
        let Some(m) = data.get(payload) else {
            return Found::default();
        };
        self.datagram(number, (src, sport), (dst, dport), m)
    }

    /// A SIP message: its transaction, its call and the media of its session description
    fn sip(&mut self, number: u64, time: i64, d: &Dissection) -> Mark {
        let call_id = text(d, "sip.call_id");
        let method = text(d, "sip.cseq.method").unwrap_or_default();
        let cseq = num(d, "sip.cseq.seq").and_then(|n| u32::try_from(n).ok());
        let status = num(d, "sip.status_code").and_then(|n| u16::try_from(n).ok());
        let call = call_id.and_then(|id| {
            if method == "INVITE" {
                self.call(id, number)
            } else {
                self.call_index.get(id).copied()
            }
        });
        let mut answers = None;
        if let (Some(id), Some(cseq)) = (call_id, cseq) {
            let key = (Arc::from(id), cseq, method.to_owned());
            if status.is_some() {
                if let Some((n, sent)) = self.transactions.get(&key) {
                    answers = Some((*n, super::micros(time - sent)));
                }
            } else {
                if self.transactions.len() >= MAX_TRANSACTIONS {
                    let keep = number.saturating_sub(10_000);
                    self.transactions.retain(|_, (n, _)| *n >= keep);
                    if self.transactions.len() >= MAX_TRANSACTIONS {
                        self.transactions.clear();
                    }
                }
                self.transactions.entry(key).or_insert((number, time));
            }
        }
        for m in &d.sdp {
            self.announce(number, call, m);
        }
        Mark::Sip {
            call,
            answers,
            status,
        }
    }

    /// The call of a Call-ID, a new one for a new dialog
    fn call(&mut self, id: &str, number: u64) -> Option<u32> {
        if let Some(call) = self.call_index.get(id) {
            return Some(*call);
        }
        if self.calls.len() >= MAX_CALLS {
            return None;
        }
        let call = u32::try_from(self.calls.len()).ok()?;
        let id: Arc<str> = Arc::from(id);
        self.calls.push(id.clone());
        self.starts.push(number);
        self.call_index.insert(id, call);
        Some(call)
    }

    /// A media stream offered by a session description: datagrams to its address are its RTP
    /// and RTCP
    fn announce(&mut self, number: u64, call: Option<u32>, m: &dissect::SdpMedia) {
        let Some(addr) = m.addr.filter(|a| !a.is_unspecified()) else {
            return;
        };
        if let Some((media, _)) = self.endpoints.get(&(addr, m.port))
            && let Some(known) = self.media.get(*media as usize)
            && known.call == call
            && known.srtp == m.srtp
            && *known.formats == *m.formats
        {
            // the same offer again (a request sent again over UDP)
            return;
        }
        if self.media.len() >= MAX_MEDIA {
            return;
        }
        if self.endpoints.len() >= MAX_ENDPOINTS {
            self.endpoints.clear();
        }
        let Ok(index) = u32::try_from(self.media.len()) else {
            return;
        };
        self.media.push(Media {
            setup: Some(number),
            call,
            formats: Arc::from(m.formats.as_slice()),
            srtp: m.srtp,
            last_type: None,
            ends: None,
        });
        self.endpoints.insert((addr, m.port), (index, false));
        if m.rtcp_port != m.port {
            self.endpoints.insert((addr, m.rtcp_port), (index, true));
        }
    }

    /// A UDP datagram that no protocol claimed: RTP or RTCP of a known stream, or a flow that
    /// may turn out to be RTP
    fn datagram(&mut self, number: u64, src: (IpAddr, u16), dst: (IpAddr, u16), m: &[u8]) -> Found {
        let known = self
            .endpoints
            .get(&dst)
            .or_else(|| self.endpoints.get(&src))
            .copied();
        if let Some((media, rtcp)) = known {
            return Found {
                mark: self.media_datagram(media, rtcp, m),
                ..Found::default()
            };
        }
        if !without_sip() || src.1 < 1024 || dst.1 < 1024 {
            return Found::default();
        }
        let key = (src.0, src.1, dst.0, dst.1);
        match self.flows.get_mut(&key) {
            Some(Flow::Media(media)) => {
                let media = *media;
                Found {
                    mark: self.media_datagram(media, false, m),
                    ..Found::default()
                }
            }
            Some(Flow::Candidate { ssrc, seq, numbers }) => {
                let header = rtp::rtp_header(m);
                if let Some(h) = header.as_ref().filter(|h| {
                    h.ssrc == *ssrc && (1..=MAX_STEP).contains(&h.seq.wrapping_sub(*seq))
                }) {
                    *seq = h.seq;
                    numbers.push(number);
                    if numbers.len() < CONFIRM {
                        return Found {
                            mark: Some(Mark::Candidate),
                            ..Found::default()
                        };
                    }
                    let mut confirmed = std::mem::take(numbers);
                    confirmed.pop();
                    let Some(media) = self.found(src, dst, h.payload_type) else {
                        self.flows.remove(&key);
                        return Found {
                            dropped: confirmed,
                            ..Found::default()
                        };
                    };
                    self.flows.insert(key, Flow::Media(media));
                    return Found {
                        mark: Some(Mark::Media { media, rtcp: false }),
                        confirmed,
                        dropped: Vec::new(),
                    };
                }
                let dropped = std::mem::take(numbers);
                let mark = if let Some(h) = header {
                    *ssrc = h.ssrc;
                    *seq = h.seq;
                    numbers.push(number);
                    Some(Mark::Candidate)
                } else {
                    self.flows.remove(&key);
                    None
                };
                Found {
                    mark,
                    confirmed: Vec::new(),
                    dropped,
                }
            }
            None => {
                let Some(h) = rtp::rtp_header(m) else {
                    return Found::default();
                };
                if self.flows.len() >= MAX_FLOWS {
                    // flows that are not RTP are forgotten first
                    self.flows.retain(|_, f| matches!(f, Flow::Media(_)));
                    if self.flows.len() >= MAX_FLOWS {
                        self.flows.clear();
                    }
                }
                self.flows.insert(
                    key,
                    Flow::Candidate {
                        ssrc: h.ssrc,
                        seq: h.seq,
                        numbers: vec![number],
                    },
                );
                Found {
                    mark: Some(Mark::Candidate),
                    ..Found::default()
                }
            }
        }
    }

    /// A new stream found by its form; the ports above its own (even) ones carry its RTCP
    fn found(&mut self, src: (IpAddr, u16), dst: (IpAddr, u16), payload_type: u8) -> Option<u32> {
        if self.media.len() >= MAX_MEDIA {
            return None;
        }
        let index = u32::try_from(self.media.len()).ok()?;
        self.media.push(Media {
            setup: None,
            call: None,
            formats: Arc::from([]),
            srtp: false,
            last_type: Some(payload_type),
            ends: Some((SocketAddr::from(src), SocketAddr::from(dst))),
        });
        if src.1.is_multiple_of(2) && dst.1.is_multiple_of(2) {
            for (addr, port) in [src, dst] {
                self.endpoints
                    .entry((addr, port + 1))
                    .or_insert((index, true));
            }
        }
        Some(index)
    }

    /// A datagram of a known stream: RTP, or RTCP on its own port or mixed with RTP (RFC 5761)
    fn media_datagram(&mut self, media: u32, rtcp: bool, m: &[u8]) -> Option<Mark> {
        let entry = self.media.get_mut(media as usize)?;
        let mixed = m.get(1).is_some_and(|b| (192..=223).contains(b));
        if rtcp || mixed {
            // without a session description only datagrams of the exact form count
            return rtp::is_rtcp(m, entry.setup.is_none())
                .then_some(Mark::Media { media, rtcp: true });
        }
        let header = rtp::rtp_header(m)?;
        entry.last_type = Some(header.payload_type);
        Some(Mark::Media { media, rtcp: false })
    }
}

/// Reads the payload of an RTP or RTCP packet into the dissection, and the call it belongs to
pub fn annotate(info: &Info, data: &[u8], d: &mut Dissection) {
    if let Some(m) = &info.media {
        dissect::media(
            data,
            d,
            &rtp::MediaNote {
                rtcp: m.rtcp,
                formats: &m.formats,
                srtp: m.srtp,
                clock: m.clock,
            },
        );
    }
    if let Some(id) = &info.call_id {
        d.values.push(("voip.call_id", Value::Text(id.to_string())));
    }
}

fn num(d: &Dissection, key: &str) -> Option<u64> {
    d.values.iter().find_map(|(k, v)| match v {
        Value::Num(n) if *k == key => Some(*n),
        _ => None,
    })
}

fn text<'a>(d: &'a Dissection, key: &str) -> Option<&'a str> {
    d.values.iter().find_map(|(k, v)| match v {
        Value::Text(t) if *k == key => Some(t.as_str()),
        _ => None,
    })
}
