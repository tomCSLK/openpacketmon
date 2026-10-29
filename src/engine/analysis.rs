//! Analysis of the stored packets across packets: TCP sequence and acknowledgment analysis
//! (retransmissions, segments out of order or not captured, duplicate ACKs, zero windows,
//! keep-alives, window updates), round trip times of ACKs and handshakes, DNS response times,
//! and the protocol path of every packet for the protocol hierarchy.
//!
//! Written for openpacketmon from the TCP specification (RFC 9293); the rules are simple
//! heuristics of our own, like other packet analyzers have them.
//!
//! The analysis runs when the packets are looked at (packet list, details, statistics), in
//! capture order, and keeps a small record per packet next to the store.

use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;

use crate::dissect::{self, Dissection, Field, Value};
use crate::engine::packets::{SharedPacketStore, StoredPacket};

/// The findings of the TCP analysis, as bits of [`PacketAnalysis::flags`]
pub mod flag {
    /// The segment was sent before (all of its bytes)
    pub const RETRANSMISSION: u16 = 1;
    /// The segment fills a gap left by a lost or late segment
    pub const OUT_OF_ORDER: u16 = 1 << 1;
    /// Bytes before the segment are missing (not captured, or lost)
    pub const LOST_SEGMENT: u16 = 1 << 2;
    /// The same acknowledgment again, without data or a change of the window
    pub const DUPLICATE_ACK: u16 = 1 << 3;
    /// The receiver has no room for data
    pub const ZERO_WINDOW: u16 = 1 << 4;
    /// A segment that only keeps the connection open
    pub const KEEP_ALIVE: u16 = 1 << 5;
    /// The same acknowledgment with a new window
    pub const WINDOW_UPDATE: u16 = 1 << 6;
    /// The connection is reset (RST)
    pub const RESET: u16 = 1 << 7;
    /// Flags that point to a problem of the connection
    pub const PROBLEMS: u16 =
        RETRANSMISSION | OUT_OF_ORDER | LOST_SEGMENT | DUPLICATE_ACK | ZERO_WINDOW;
}

/// The findings with their display filter field and the label of the packet list and details
pub const FLAGS: [(u16, &str, &str); 7] = [
    (
        flag::RETRANSMISSION,
        "tcp.analysis.retransmission",
        "TCP Retransmission",
    ),
    (
        flag::OUT_OF_ORDER,
        "tcp.analysis.out_of_order",
        "TCP Out-Of-Order",
    ),
    (
        flag::LOST_SEGMENT,
        "tcp.analysis.lost_segment",
        "TCP Previous segment not captured",
    ),
    (
        flag::DUPLICATE_ACK,
        "tcp.analysis.duplicate_ack",
        "TCP Dup ACK",
    ),
    (
        flag::ZERO_WINDOW,
        "tcp.analysis.zero_window",
        "TCP ZeroWindow",
    ),
    (
        flag::KEEP_ALIVE,
        "tcp.analysis.keep_alive",
        "TCP Keep-Alive",
    ),
    (
        flag::WINDOW_UPDATE,
        "tcp.analysis.window_update",
        "TCP Window Update",
    ),
];

/// Packets analyzed per lock of the store
const CHUNK: usize = 2048;
/// Connections followed at a time; older ones are forgotten first
const MAX_FLOWS: usize = 100_000;
/// DNS queries waiting for their response
const MAX_QUERIES: usize = 10_000;
/// Gaps in the sequence numbers remembered per direction
const MAX_GAPS: usize = 16;
/// Segments waiting for their ACK per direction, for the round trip times
const MAX_UNACKED: usize = 64;
/// Records kept at most (more than the store can hold)
const MAX_RECORDS: usize = 600_000;

/// What the analysis found about one packet
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PacketAnalysis {
    /// Bits of [`flag`]
    pub flags: u16,
    /// Index of the packet's protocols in [`Analysis::paths`]
    pub path: u16,
    /// Length on the wire
    pub len: u32,
    /// Time of the packet in microseconds since 1970
    pub time: i64,
    /// The segment this packet acknowledges (its packet number) and the time since it was sent,
    /// in microseconds
    pub acks: Option<(u64, u32)>,
    /// This packet completes the TCP handshake: microseconds since the SYN
    pub handshake: Option<u32>,
    /// This DNS response answers the query in this packet, after so many microseconds
    pub dns: Option<(u64, u32)>,
}

/// The analysis of the packets of one store
#[derive(Default)]
pub struct Analysis {
    /// The packets up to this number have been analyzed
    done: u64,
    /// Number of the packet of `records[0]`
    first: u64,
    records: VecDeque<PacketAnalysis>,
    flows: HashMap<FlowKey, Flow>,
    queries: HashMap<QueryKey, (u64, i64)>,
    paths: Vec<Vec<&'static str>>,
    path_ids: HashMap<Vec<&'static str>, u16>,
}

/// The two ends of a TCP connection, the lower one first
type FlowKey = ((IpAddr, u16), (IpAddr, u16));
/// Client, client port, server, server port, query ID
type QueryKey = (IpAddr, u16, IpAddr, u16, u16);

#[derive(Default)]
struct Flow {
    /// The two directions; 0 is the one sent by the lower end of the key
    sides: [Side; 2],
    /// Side and time of the SYN of a handshake under way
    syn: Option<(usize, i64)>,
    /// The SYN has been answered
    syn_ack: bool,
    /// Number of the newest packet
    last: u64,
}

#[derive(Default)]
struct Side {
    /// The next sequence number expected
    next: Option<u32>,
    /// Ranges of sequence numbers not seen yet (from, to)
    gaps: Vec<(u32, u32)>,
    /// The last acknowledgment number and window sent
    last_ack: Option<(u32, u16)>,
    /// Segments without an ACK yet: end of the segment, packet number, time
    unacked: VecDeque<(u32, u64, i64)>,
}

/// The values of a TCP segment the analysis needs
#[allow(clippy::struct_excessive_bools)] // the flags of the segment
struct Segment {
    sport: u16,
    dport: u16,
    seq: u32,
    ack_number: u32,
    len: u32,
    window: u16,
    syn: bool,
    ack: bool,
    fin: bool,
    rst: bool,
}

impl Analysis {
    /// The packets up to this number have been analyzed
    #[cfg(test)]
    pub fn done(&self) -> u64 {
        self.done
    }

    /// The record of a packet, if it has been analyzed and is still kept
    pub fn get(&self, number: u64) -> Option<&PacketAnalysis> {
        let index = usize::try_from(number.checked_sub(self.first)?).ok()?;
        self.records.get(index)
    }

    /// The records after the one of this packet, with their packet numbers
    pub fn after(&self, number: u64) -> impl Iterator<Item = (u64, &PacketAnalysis)> {
        let skip = usize::try_from(number.saturating_add(1).saturating_sub(self.first))
            .unwrap_or(usize::MAX);
        self.records
            .iter()
            .enumerate()
            .skip(skip)
            .map(|(i, r)| (self.first + i as u64, r))
    }

    /// The protocols of the packets (`eth`, `ip`, `tcp`, `tls`), by [`PacketAnalysis::path`]
    pub fn paths(&self) -> &[Vec<&'static str>] {
        &self.paths
    }

    /// Analyzes `packet`, dissected as `d`, if it has not been analyzed yet, and returns its
    /// record; packets must come in capture order (earlier ones that were skipped are gone)
    pub fn feed(&mut self, packet: &StoredPacket, d: &Dissection) -> Option<PacketAnalysis> {
        if packet.number <= self.done {
            return self.get(packet.number).copied();
        }
        if packet.number != self.done + 1 || self.records.is_empty() {
            // the packets in between were dropped from the store before anyone looked
            self.records.clear();
            self.first = packet.number;
        }
        if self.records.len() >= MAX_RECORDS {
            self.records.pop_front();
            self.first += 1;
        }
        let record = self.analyze(packet, d);
        self.records.push_back(record);
        self.done = packet.number;
        Some(record)
    }

    /// Forgets the records of the packets before this number (dropped from the store)
    fn forget_before(&mut self, number: u64) {
        while self.first < number && self.records.pop_front().is_some() {
            self.first += 1;
        }
    }

    fn analyze(&mut self, packet: &StoredPacket, d: &Dissection) -> PacketAnalysis {
        let time = packet.timestamp.to_usecs().unwrap_or(0);
        let mut record = PacketAnalysis {
            path: self.path(&d.protocols),
            len: packet.orig_len,
            time,
            ..PacketAnalysis::default()
        };
        let Some((src, dst)) = addresses(d) else {
            return record;
        };
        if let Some(segment) = Segment::read(d) {
            self.tcp(packet.number, time, src, dst, &segment, &mut record);
        }
        self.dns(packet.number, time, src, dst, d, &mut record);
        record
    }

    /// The index of a list of protocols in [`Self::paths`]
    fn path(&mut self, protocols: &[&'static str]) -> u16 {
        if let Some(id) = self.path_ids.get(protocols) {
            return *id;
        }
        let Ok(id) = u16::try_from(self.paths.len()) else {
            return u16::MAX;
        };
        self.paths.push(protocols.to_vec());
        self.path_ids.insert(protocols.to_vec(), id);
        id
    }

    fn tcp(
        &mut self,
        number: u64,
        time: i64,
        src: IpAddr,
        dst: IpAddr,
        t: &Segment,
        record: &mut PacketAnalysis,
    ) {
        let (key, i) = flow_key(src, t.sport, dst, t.dport);
        if self.flows.len() >= MAX_FLOWS && !self.flows.contains_key(&key) {
            let keep = number.saturating_sub(100_000);
            self.flows.retain(|_, f| f.last >= keep);
            if self.flows.len() >= MAX_FLOWS {
                self.flows.clear();
            }
        }
        let flow = self.flows.entry(key).or_default();
        flow.last = number;
        if t.rst {
            record.flags |= flag::RESET;
        }
        if !flow.handshake(i, number, time, t, record) {
            return;
        }
        flow.sides[i].sequence(t, record);
        if t.window == 0 && !t.syn && !t.fin && !t.rst {
            record.flags |= flag::ZERO_WINDOW;
        }
        flow.acknowledgments(i, number, time, t, record);
    }

    fn dns(
        &mut self,
        number: u64,
        time: i64,
        src: IpAddr,
        dst: IpAddr,
        d: &Dissection,
        record: &mut PacketAnalysis,
    ) {
        let (Some(id), Some(response)) = (num(d, "dns.id"), num(d, "dns.flags.response")) else {
            return;
        };
        let ports = num(d, "udp.srcport")
            .zip(num(d, "udp.dstport"))
            .or_else(|| num(d, "tcp.srcport").zip(num(d, "tcp.dstport")));
        let Some((sport, dport)) = ports else {
            return;
        };
        let (Ok(id), Ok(sport), Ok(dport)) = (
            u16::try_from(id),
            u16::try_from(sport),
            u16::try_from(dport),
        ) else {
            return;
        };
        if response == 0 {
            if self.queries.len() >= MAX_QUERIES {
                let keep = number.saturating_sub(10_000);
                self.queries.retain(|_, (n, _)| *n >= keep);
                if self.queries.len() >= MAX_QUERIES {
                    self.queries.clear();
                }
            }
            self.queries
                .insert((src, sport, dst, dport, id), (number, time));
        } else if let Some((n, sent)) = self.queries.remove(&(dst, dport, src, sport, id)) {
            record.dns = Some((n, micros(time - sent)));
        }
    }
}

impl Flow {
    /// Follows the handshake (SYN, SYN-ACK, ACK) of side `i`; `false` for a SYN sent again,
    /// which needs no further analysis
    fn handshake(
        &mut self,
        i: usize,
        number: u64,
        time: i64,
        t: &Segment,
        record: &mut PacketAnalysis,
    ) -> bool {
        if t.syn && !t.ack {
            if self.sides[i].next == Some(t.seq.wrapping_add(1)) {
                record.flags |= flag::RETRANSMISSION;
                return false;
            }
            // a new connection on these ports
            *self = Flow {
                syn: Some((i, time)),
                last: number,
                ..Flow::default()
            };
        } else if t.syn {
            self.syn_ack = self.syn.is_some_and(|(side, _)| side != i);
        } else if t.ack && self.syn_ack {
            if let Some((side, start)) = self.syn
                && side == i
            {
                record.handshake = Some(micros(time - start));
            }
            self.syn = None;
            self.syn_ack = false;
        }
        true
    }

    /// Duplicate ACKs, window updates and the round trip time of the segment side `i` acknowledges
    fn acknowledgments(
        &mut self,
        i: usize,
        number: u64,
        time: i64,
        t: &Segment,
        record: &mut PacketAnalysis,
    ) {
        let side = &mut self.sides[i];
        let pure_ack = t.ack
            && t.len == 0
            && !t.syn
            && !t.fin
            && !t.rst
            && record.flags & flag::KEEP_ALIVE == 0;
        if pure_ack
            && let Some((ack, window)) = side.last_ack
            && ack == t.ack_number
        {
            if window == t.window {
                record.flags |= flag::DUPLICATE_ACK;
            } else if t.window != 0 {
                // a window closed to zero is a zero window, not an update
                record.flags |= flag::WINDOW_UPDATE;
            }
        }
        if t.ack && !t.syn {
            side.last_ack = Some((t.ack_number, t.window));
        }
        let seg = t.len + u32::from(t.syn) + u32::from(t.fin);
        let unsure = flag::RETRANSMISSION | flag::OUT_OF_ORDER | flag::KEEP_ALIVE;
        if seg > 0 && record.flags & unsure == 0 {
            if side.unacked.len() >= MAX_UNACKED {
                side.unacked.pop_front();
            }
            side.unacked
                .push_back((t.seq.wrapping_add(seg), number, time));
        }
        if !t.ack {
            return;
        }
        let other = &mut self.sides[1 - i];
        let mut acked = None;
        while let Some(&(end, n, sent)) = other.unacked.front() {
            if diff(t.ack_number, end) < 0 {
                break;
            }
            acked = Some((n, sent));
            other.unacked.pop_front();
        }
        if let Some((n, sent)) = acked {
            record.acks = Some((n, micros(time - sent)));
        }
    }
}

impl Side {
    /// Compares the sequence number with the one expected: keep-alive, gap, late segment or
    /// retransmission
    fn sequence(&mut self, t: &Segment, record: &mut PacketAnalysis) {
        let seg = t.len + u32::from(t.syn) + u32::from(t.fin);
        let end = t.seq.wrapping_add(seg);
        let Some(next) = self.next else {
            self.next = Some(end);
            return;
        };
        let ahead = diff(t.seq, next);
        if t.len <= 1 && !t.syn && !t.fin && !t.rst && ahead == -1 {
            record.flags |= flag::KEEP_ALIVE;
            return;
        }
        if seg == 0 {
            return;
        }
        match ahead.cmp(&0) {
            Ordering::Greater => {
                record.flags |= flag::LOST_SEGMENT;
                if self.gaps.len() < MAX_GAPS {
                    self.gaps.push((next, t.seq));
                }
                self.next = Some(end);
            }
            Ordering::Equal => self.next = Some(end),
            Ordering::Less => {
                let gap = self
                    .gaps
                    .iter()
                    .position(|(from, to)| diff(t.seq, *from) >= 0 && diff(t.seq, *to) < 0);
                if let Some(g) = gap {
                    record.flags |= flag::OUT_OF_ORDER;
                    let (from, to) = self.gaps.swap_remove(g);
                    if diff(t.seq, from) > 0 {
                        self.gaps.push((from, t.seq));
                    }
                    if diff(to, end) > 0 {
                        self.gaps.push((end, to));
                    }
                } else {
                    record.flags |= flag::RETRANSMISSION;
                    // its first transmission no longer tells the round trip time
                    self.unacked.retain(|(e, _, _)| diff(*e, t.seq) <= 0);
                }
                if diff(end, next) > 0 {
                    self.next = Some(end);
                }
            }
        }
    }
}

impl Segment {
    fn read(d: &Dissection) -> Option<Self> {
        let flag = |key| num(d, key).is_some_and(|v| v != 0);
        Some(Self {
            sport: u16::try_from(num(d, "tcp.srcport")?).ok()?,
            dport: u16::try_from(num(d, "tcp.dstport")?).ok()?,
            seq: u32::try_from(num(d, "tcp.seq")?).ok()?,
            ack_number: u32::try_from(num(d, "tcp.ack")?).ok()?,
            len: u32::try_from(num(d, "tcp.len")?).ok()?,
            window: u16::try_from(num(d, "tcp.window")?).ok()?,
            syn: flag("tcp.flags.syn"),
            ack: flag("tcp.flags.ack"),
            fin: flag("tcp.flags.fin"),
            rst: flag("tcp.flags.rst"),
        })
    }
}

/// The last value of a number field of the dissection
fn num(d: &Dissection, key: &str) -> Option<u64> {
    d.values.iter().rev().find_map(|(k, v)| match v {
        Value::Num(n) if *k == key => Some(*n),
        _ => None,
    })
}

/// Source and destination of the innermost IP header
fn addresses(d: &Dissection) -> Option<(IpAddr, IpAddr)> {
    let ip = |keys: [&str; 2]| {
        d.values.iter().rev().find_map(|(k, v)| match v {
            Value::Ip(a) if keys.contains(k) => Some(*a),
            _ => None,
        })
    };
    Some((ip(["ip.src", "ipv6.src"])?, ip(["ip.dst", "ipv6.dst"])?))
}

/// The key of a connection and the side of the sender
fn flow_key(src: IpAddr, sport: u16, dst: IpAddr, dport: u16) -> (FlowKey, usize) {
    let (a, b) = ((src, sport), (dst, dport));
    if a <= b { ((a, b), 0) } else { ((b, a), 1) }
}

/// How far sequence number `a` is ahead of `b` (negative: behind), across the wrap
#[allow(clippy::cast_possible_wrap)]
fn diff(a: u32, b: u32) -> i32 {
    a.wrapping_sub(b) as i32
}

fn micros(us: i64) -> u32 {
    u32::try_from(us.max(0)).unwrap_or(u32::MAX)
}

/// Microseconds as seconds (`0.012345 s`)
pub fn seconds(us: u32) -> String {
    format!("{}.{:06} s", us / 1_000_000, us % 1_000_000)
}

/// Analyzes the stored packets up to this number that have not been analyzed yet
pub fn advance(store: &SharedPacketStore, upto: u64) {
    let mut analysis = store.analysis();
    loop {
        let (chunk, first, link_type) = {
            let store = store.lock();
            (
                store
                    .after(analysis.done)
                    .take_while(|p| p.number <= upto)
                    .take(CHUNK)
                    .cloned()
                    .collect::<Vec<_>>(),
                store.iter().next().map_or(u64::MAX, |p| p.number),
                store.link_type(),
            )
        };
        analysis.forget_before(first);
        for packet in &chunk {
            let d = dissect::dissect(&packet.data, link_type);
            analysis.feed(packet, &d);
        }
        if chunk.len() < CHUNK {
            return;
        }
    }
}

/// Adds what the analysis found to a dissection: values for the display filter, a note in front
/// of the info and fields in the TCP and DNS layers
pub fn annotate(record: &PacketAnalysis, d: &mut Dissection) {
    let found: Vec<(&'static str, &'static str)> = FLAGS
        .iter()
        .filter(|(bit, _, _)| record.flags & bit != 0)
        .map(|(_, key, label)| (*key, *label))
        .collect();
    if record.flags & flag::PROBLEMS != 0 {
        d.values.push(("tcp.analysis.flags", Value::Num(1)));
    }
    for (key, _) in &found {
        d.values.push((key, Value::Num(1)));
    }
    let mut fields = Vec::new();
    for (_, label) in &found {
        fields.push((*label, String::new()));
    }
    if let Some((n, rtt)) = record.acks {
        d.values.push(("tcp.analysis.acks_frame", Value::Num(n)));
        d.values
            .push(("tcp.analysis.ack_rtt", Value::Num(u64::from(rtt))));
        fields.push(("This is an ACK to the segment in packet", n.to_string()));
        fields.push(("Time since that segment (RTT)", seconds(rtt)));
    }
    if let Some(us) = record.handshake {
        d.values
            .push(("tcp.analysis.handshake", Value::Num(u64::from(us))));
        fields.push(("Handshake time (SYN to ACK)", seconds(us)));
    }
    if !found.is_empty() {
        let labels: Vec<&str> = found.iter().map(|(_, l)| *l).collect();
        d.info = format!("[{}] {}", labels.join(", "), d.info);
    }
    if !fields.is_empty()
        && let Some(layer) = d
            .layers
            .iter_mut()
            .find(|l| l.title.starts_with("Transmission Control Protocol"))
    {
        let at = layer.range.end..layer.range.end;
        let children = fields
            .into_iter()
            .map(|(name, value)| Field::new(name, value, at.clone()))
            .collect();
        layer
            .fields
            .push(Field::new("[SEQ/ACK analysis]", "", at).with_children(children));
    }
    if let Some((n, us)) = record.dns {
        d.values.push(("dns.response_to", Value::Num(n)));
        d.values.push(("dns.time", Value::Num(u64::from(us))));
        if let Some(layer) = d.layers.iter_mut().rev().find(|l| {
            [
                "Domain Name System",
                "Multicast Domain",
                "Link-local Multicast",
            ]
            .iter()
            .any(|t| l.title.starts_with(t))
        }) {
            let at = layer.range.end..layer.range.end;
            layer.fields.push(Field::new(
                "[Response to the query in packet]",
                n.to_string(),
                at.clone(),
            ));
            layer.fields.push(Field::new("[Time]", seconds(us), at));
        }
    }
}

#[cfg(test)]
mod tests;
