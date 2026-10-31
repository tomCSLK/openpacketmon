//! "Follow stream": the payload of a TCP or UDP connection in both directions as text or hex,
//! put together from the stored packets (TCP in sequence order, retransmissions left out).

use std::fmt::Write;

use crate::dissect;
use crate::engine::Engine;
use crate::engine::packets::StoredPacket;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::view::format;
use crate::view::i18n::Lang;

/// Payload bytes shown at most; the PCAP file has all of them
const MAX_BYTES: usize = 1024 * 1024;

/// A run of bytes in one direction
#[derive(Debug, PartialEq, Eq)]
pub struct Chunk {
    /// Sent by the side that started the connection
    pub client: bool,
    pub data: Vec<u8>,
    /// Bytes missing before this chunk (not captured or dropped from the store)
    pub gap: u64,
    /// Number of the first packet of the chunk
    pub packet: u64,
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
            }
            _ => chunks.push(Chunk {
                client,
                data: bytes.to_vec(),
                gap,
                packet: p.number,
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

/// Bytes as text: printable ASCII and line breaks, other bytes as `·`
fn as_text(data: &[u8]) -> String {
    data.iter()
        .map(|b| match b {
            b'\n' | b'\t' => char::from(*b),
            b'\r' => '\u{0}',
            b if b.is_ascii_graphic() || *b == b' ' => char::from(*b),
            _ => '·',
        })
        .filter(|c| *c != '\u{0}')
        .collect()
}

/// Bytes as hex dump lines
fn as_hex(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, line) in data.chunks(16).enumerate() {
        let _ = write!(out, "{:08x}  ", i * 16);
        for (j, b) in line.iter().enumerate() {
            let _ = write!(out, "{b:02x} ");
            if j == 7 {
                out.push(' ');
            }
        }
        for j in line.len()..16 {
            out.push_str("   ");
            if j == 7 {
                out.push(' ');
            }
        }
        out.push(' ');
        out.extend(line.iter().map(|b| {
            if b.is_ascii_graphic() || *b == b' ' {
                char::from(*b)
            } else {
                '.'
            }
        }));
        out.push('\n');
    }
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
    pub gap: Option<String>,
    pub packet: u64,
}

/// The stream of a connection
pub struct Stream {
    pub title: String,
    pub client: String,
    pub server: String,
    pub client_counts: String,
    pub server_counts: String,
    pub chunks: Vec<ShownChunk>,
    pub cut: bool,
    pub headers_only: bool,
    /// The connection of the packet asked for
    pub connection: AddressPortPair,
}

/// The connection of packet `n` as a stream, if it is kept and belongs to one
pub fn stream(
    t: Lang,
    engine: &Engine,
    n: u64,
    view: StreamView,
    side: StreamSide,
) -> Option<Stream> {
    let store = engine.lock().packets.clone()?;
    let (packets, link_type, headers_only, key) = {
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
        (packets, store.link_type(), store.headers_only(), key)
    };
    // the side of the first packet started the connection
    let client_key = packets.first().and_then(|p| p.conn).map_or(key, |(k, _)| k);
    let (chunks, counts, cut) = assemble(link_type, &packets, &client_key);
    let shown = chunks
        .into_iter()
        .filter(|c| match side {
            StreamSide::Both => true,
            StreamSide::Client => c.client,
            StreamSide::Server => !c.client,
        })
        .map(|c| ShownChunk {
            client: c.client,
            text: match view {
                StreamView::Hex => as_hex(&c.data),
                StreamView::Text => as_text(&c.data),
            },
            gap: (c.gap > 0).then(|| t.get1("stream-gap", "bytes", c.gap)),
            packet: c.packet,
        })
        .collect();
    let counts_text = |bytes: u64, packets: u64| {
        t.get2(
            "stream-counts",
            "bytes",
            format::bytes(t, u128::from(bytes)),
            "packets",
            packets,
        )
    };
    let client = format::socket(&client_key.src_ip, client_key.src_port);
    let server = format::socket(&client_key.dst_ip, client_key.dst_port);
    Some(Stream {
        title: format!("{client} ⇄ {server} ({})", client_key.protocol),
        client,
        server,
        client_counts: counts_text(counts[0], counts[1]),
        server_counts: counts_text(counts[2], counts[3]),
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
        assert_eq!((chunks[0].client, chunks[0].packet), (true, 1));
        assert_eq!(chunks[1].data, b"HTTP/1.1 200 OK\r\nhello");
        assert!(!chunks[1].client);
        assert_eq!((chunks[2].gap, chunks[2].data.as_slice()), (10, &b"!"[..]));
        assert_eq!(counts, [18, 2, 23, 3]);
        assert_eq!(as_text(&chunks[0].data), "GET / HTTP/1.1\n\n");
        assert!(as_hex(b"GET").starts_with("00000000  47 45 54 "));
    }
}
