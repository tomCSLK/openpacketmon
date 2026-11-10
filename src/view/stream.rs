//! "Follow stream": the payload of a TCP or UDP connection in both directions as text or hex,
//! put together from the stored packets (TCP in sequence order, retransmissions left out).

use std::fmt::Write;

use crate::dissect;
use crate::engine::Engine;
use crate::engine::packets::StoredPacket;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::service::Service;
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::packets::connection_label;

/// Payload bytes shown at most; the PCAP file has all of them
pub const MAX_BYTES: usize = 1024 * 1024;

/// A run of bytes in one direction
#[derive(Debug, PartialEq, Eq)]
pub struct Chunk {
    /// Sent by the side that started the connection
    pub client: bool,
    pub data: Vec<u8>,
    /// Bytes missing before this chunk (not captured or dropped from the store)
    pub gap: u64,
    /// Numbers of the packets of the chunk
    pub packets: Vec<u64>,
}

/// What one direction has sent
#[derive(Default, Clone, Copy)]
struct Side {
    bytes: u64,
    packets: u64,
    /// Next expected TCP sequence number
    next: Option<u32>,
}

/// The payload of the packets of a connection, in capture order; `packets` holds both directions
/// and the first one decides which side is the client
pub fn assemble(
    link_type: sniffnet_packet_parser::LinkType,
    packets: &[StoredPacket],
    client_key: &AddressPortPair,
) -> (Vec<Chunk>, [u64; 4], bool) {
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut sides = [Side::default(); 2];
    let mut total = 0;
    let mut cut = false;
    for p in packets {
        let Some((key, _)) = &p.conn else { continue };
        let client = key == client_key;
        let Some(payload) = dissect::transport_payload(&p.data, link_type) else {
            continue;
        };
        let side = &mut sides[usize::from(!client)];
        let mut bytes = &p.data[payload.range.clone()];
        if bytes.is_empty() {
            continue;
        }
        let mut gap = 0;
        if payload.tcp {
            if let Some(next) = side.next {
                #[allow(clippy::cast_possible_wrap)]
                let diff = payload.seq.wrapping_sub(next) as i32;
                if diff < 0 {
                    // a retransmission, or a segment that overlaps what came before
                    let overlap = diff.unsigned_abs() as usize;
                    if overlap >= bytes.len() {
                        continue;
                    }
                    bytes = &bytes[overlap..];
                } else {
                    gap = u64::from(diff.unsigned_abs());
                }
            }
            side.next = Some(
                payload
                    .seq
                    .wrapping_add(u32::try_from(payload.range.len()).unwrap_or(0)),
            );
        }
        side.bytes += bytes.len() as u64;
        side.packets += 1;
        if cut {
            continue;
        }
        let room = MAX_BYTES - total;
        if bytes.len() > room {
            bytes = &bytes[..room];
            cut = true;
        }
        total += bytes.len();
        match chunks.last_mut() {
            // TCP is a stream: what one side sends in a row belongs together
            Some(last) if payload.tcp && last.client == client && gap == 0 => {
                last.data.extend_from_slice(bytes);
                last.packets.push(p.number);
            }
            _ => chunks.push(Chunk {
                client,
                data: bytes.to_vec(),
                gap,
                packets: vec![p.number],
            }),
        }
    }
    let counts = [
        sides[0].bytes,
        sides[0].packets,
        sides[1].bytes,
        sides[1].packets,
    ];
    (chunks, counts, cut)
}

/// Bytes as text: printable ASCII, line breaks and tabs, other bytes as `.`
pub fn as_text(data: &[u8]) -> String {
    data.iter()
        .filter(|b| **b != b'\r')
        .map(|b| match b {
            b'\n' | b'\t' => char::from(*b),
            b if b.is_ascii_graphic() || *b == b' ' => char::from(*b),
            _ => '.',
        })
        .collect()
}

/// Bytes as hex dump lines, the offsets counted from `base` (`00000010  47 45 …  GE…`)
pub fn as_hex(data: &[u8], base: usize) -> String {
    let mut out = String::new();
    for (i, line) in data.chunks(16).enumerate() {
        let _ = write!(out, "{:08x}  ", base + i * 16);
        let mut hex = String::new();
        for (j, b) in line.iter().enumerate() {
            if j > 0 {
                hex.push(' ');
            }
            let _ = write!(hex, "{b:02x}");
            if j == 7 {
                hex.push(' ');
            }
        }
        let _ = write!(out, "{hex:<49}  ");
        out.extend(line.iter().map(|b| {
            if b.is_ascii_graphic() || *b == b' ' {
                char::from(*b)
            } else {
                '.'
            }
        }));
        out.push('\n');
    }
    out.pop();
    out
}

/// How the bytes are shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamView {
    Text,
    Hex,
}

/// Which directions are shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamSide {
    Both,
    Client,
    Server,
}

/// A chunk as shown
pub struct ShownChunk {
    pub client: bool,
    pub text: String,
    /// The bytes missing before it
    pub gap: Option<String>,
    /// "Packets 1205, 1206", for the pointer
    pub packets: String,
}

/// The stream of a connection
pub struct Stream {
    /// The connection from the side that started it (`connLabel`)
    pub label: String,
    /// Its service, if known (`· http`)
    pub service: Option<String>,
    /// The service is encrypted with TLS: only the handshake can be read
    pub tls: bool,
    pub client: String,
    pub server: String,
    /// "2.69 KB in 3 packets", with "(of … in …)" when one side is shown
    pub counts: String,
    pub chunks: Vec<ShownChunk>,
    pub cut: bool,
    pub headers_only: bool,
    /// The connection of the packet asked for
    pub connection: AddressPortPair,
}

/// Services whose content is encrypted with TLS
const TLS_SERVICES: [&str; 7] = [
    "https",
    "imaps",
    "pop3s",
    "smtps",
    "submissions",
    "ldaps",
    "ftps",
];

/// The kept packets of the connection of packet `n`, both directions in capture order
pub struct Conversation {
    pub packets: Vec<StoredPacket>,
    pub link_type: sniffnet_packet_parser::LinkType,
    pub headers_only: bool,
    /// The direction of packet `n`
    pub key: AddressPortPair,
    /// The direction of the side that started the connection: that of its first kept packet
    pub client_key: AddressPortPair,
}

/// The connection of packet `n`, if it is kept and belongs to one
pub fn conversation(engine: &Engine, n: u64) -> Option<Conversation> {
    let store = engine.lock().packets.clone()?;
    let store = store.lock();
    let packet = store.get(n)?;
    let (key, _) = packet.conn?;
    let reverse = AddressPortPair {
        src_ip: key.dst_ip,
        src_port: key.dst_port,
        dst_ip: key.src_ip,
        dst_port: key.src_port,
        ..key
    };
    let packets: Vec<StoredPacket> = store
        .iter()
        .filter(|p| p.conn.is_some_and(|(k, _)| k == key || k == reverse))
        .cloned()
        .collect();
    let client_key = packets.first().and_then(|p| p.conn).map_or(key, |(k, _)| k);
    Some(Conversation {
        packets,
        link_type: store.link_type(),
        headers_only: store.headers_only(),
        key,
        client_key,
    })
}

/// The connection of packet `n` as a stream, if it is kept and belongs to one
#[allow(clippy::too_many_lines)] // the stream, its chunks and the counts
pub fn stream(
    t: Lang,
    engine: &Engine,
    n: u64,
    view: StreamView,
    side: StreamSide,
) -> Option<Stream> {
    let Conversation {
        packets,
        link_type,
        headers_only,
        key,
        client_key,
    } = conversation(engine, n)?;
    let (chunks, counts, cut) = assemble(link_type, &packets, &client_key);
    let (mut client_at, mut server_at) = (0, 0);
    let (mut bytes, mut shown_packets) = (0, 0);
    let shown = chunks
        .into_iter()
        .filter(|c| match side {
            StreamSide::Both => true,
            StreamSide::Client => c.client,
            StreamSide::Server => !c.client,
        })
        .map(|c| {
            // the offsets of the hex dump count each direction on its own
            let at = if c.client {
                &mut client_at
            } else {
                &mut server_at
            };
            let base = *at;
            *at += c.data.len();
            bytes += c.data.len() as u64;
            shown_packets += c.packets.len() as u64;
            let mut numbers: Vec<String> = c.packets.iter().take(50).map(u64::to_string).collect();
            if c.packets.len() > 50 {
                numbers.push("…".to_owned());
            }
            ShownChunk {
                client: c.client,
                text: match view {
                    StreamView::Hex => as_hex(&c.data, base),
                    // a line break at the end needs no empty line
                    StreamView::Text => {
                        let mut text = as_text(&c.data);
                        if text.ends_with('\n') {
                            text.pop();
                        }
                        text
                    }
                },
                gap: (c.gap > 0).then(|| t.get1("stream-gap", "bytes", c.gap)),
                packets: t.get2(
                    "stream-chunk-packets",
                    "count",
                    c.packets.len(),
                    "numbers",
                    numbers.join(", "),
                ),
            }
        })
        .collect();
    let counts_text = |bytes: u64, packets: u64, key: &str| {
        t.fmt(
            key,
            &[
                ("bytes", format::bytes(t, u128::from(bytes)).into()),
                ("packets", format::count(t, u128::from(packets)).into()),
                ("n", packets.into()),
            ],
        )
    };
    let mut counts_line = counts_text(bytes, shown_packets, "stream-counts");
    if side != StreamSide::Both {
        counts_line.push(' ');
        counts_line.push_str(&counts_text(
            counts[0] + counts[2],
            counts[1] + counts[3],
            "stream-counts-of",
        ));
    }
    let service = {
        let engine = engine.lock();
        let reverse = AddressPortPair {
            src_ip: key.dst_ip,
            src_port: key.dst_port,
            dst_ip: key.src_ip,
            dst_port: key.src_port,
            ..key
        };
        [key, reverse]
            .iter()
            .find_map(|k| engine.info_traffic.map.get(k))
            .and_then(|info| match info.service {
                Service::Name(name) => Some(name.to_owned()),
                _ => None,
            })
    };
    Some(Stream {
        label: connection_label(&client_key),
        tls: service
            .as_deref()
            .is_some_and(|name| TLS_SERVICES.contains(&name)),
        service,
        client: format::socket(&client_key.src_ip, client_key.src_port),
        server: format::socket(&client_key.dst_ip, client_key.dst_port),
        counts: counts_line,
        chunks: shown,
        cut,
        headers_only,
        connection: key,
    })
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::sync::Arc;

    use sniffnet_packet_parser::Protocol;

    use super::*;
    use crate::dissect::tests::{ETH, tcp_packet};
    use crate::networking::types::traffic_direction::TrafficDirection;
    use crate::utils::types::timestamp::Timestamp;

    fn key(client: bool) -> AddressPortPair {
        let (a, b) = (Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(10, 0, 0, 2));
        let (src, sport, dst, dport) = if client {
            (a, 51000, b, 80)
        } else {
            (b, 80, a, 51000)
        };
        AddressPortPair {
            src_ip: src.into(),
            src_port: Some(sport),
            dst_ip: dst.into(),
            dst_port: Some(dport),
            protocol: Protocol::Tcp,
            exporter: None,
        }
    }

    fn segment(number: u64, client: bool, seq: u32, payload: &[u8]) -> StoredPacket {
        let (sport, dport) = if client { (51000, 80) } else { (80, 51000) };
        StoredPacket {
            number,
            timestamp: Timestamp::new(1, 0),
            orig_len: 100,
            data: Arc::from(tcp_packet(sport, dport, seq, payload)),
            conn: Some((key(client), TrafficDirection::Outgoing)),
        }
    }

    #[test]
    fn a_tcp_stream_in_order_without_retransmissions() {
        let packets = [
            segment(1, true, 100, b"GET / HT"),
            segment(2, true, 108, b"TP/1.1\r\n\r\n"),
            // retransmission of the first segment
            segment(3, true, 100, b"GET / HT"),
            segment(4, false, 900, b"HTTP/1.1 200 OK\r\n"),
            // partly new: the first 4 bytes were sent already
            segment(5, false, 913, b"OK\r\nhello"),
            // 10 bytes not captured
            segment(6, false, 932, b"!"),
        ];
        let (chunks, counts, cut) = assemble(ETH, &packets, &key(true));
        assert!(!cut);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].data, b"GET / HTTP/1.1\r\n\r\n");
        assert_eq!(
            (chunks[0].client, chunks[0].packets.clone()),
            (true, vec![1, 2])
        );
        assert_eq!(chunks[1].data, b"HTTP/1.1 200 OK\r\nhello");
        assert!(!chunks[1].client);
        assert_eq!((chunks[2].gap, chunks[2].data.as_slice()), (10, &b"!"[..]));
        assert_eq!(counts, [18, 2, 23, 3]);
        assert_eq!(as_text(&chunks[0].data), "GET / HTTP/1.1\n\n");
        assert_eq!(
            as_hex(b"GET", 0x20),
            format!("00000020  47 45 54{}  GET", " ".repeat(41))
        );
        let line = as_hex(b"0123456789abcdef", 0);
        // after the eighth byte one more space, the characters after two spaces
        assert_eq!(
            line,
            "00000000  30 31 32 33 34 35 36 37  38 39 61 62 63 64 65 66   0123456789abcdef"
        );
    }
}
