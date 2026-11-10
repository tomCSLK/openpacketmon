use pcap::Linktype;
use sniffnet_packet_parser::LinkType;

use super::{Dissection, Field, Followup, dissect, dissect_with, tcp_payload};

pub(crate) const ETH: LinkType = LinkType::Ethernet(Linktype::ETHERNET);

fn ethernet(ether_type: u16) -> Vec<u8> {
    let mut p = vec![
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
    ];
    p.extend_from_slice(&ether_type.to_be_bytes());
    p
}

/// An IPv4 header with a correct checksum
fn ipv4(protocol: u8, payload_len: usize, src: [u8; 4], dst: [u8; 4]) -> Vec<u8> {
    let total = u16::try_from(20 + payload_len).unwrap();
    let mut h = vec![0x45, 0];
    h.extend_from_slice(&total.to_be_bytes());
    h.extend_from_slice(&[0x12, 0x34, 0x40, 0x00, 64, protocol, 0, 0]);
    h.extend_from_slice(&src);
    h.extend_from_slice(&dst);
    let mut sum: u32 = h
        .chunks(2)
        .map(|c| u32::from(u16::from_be_bytes([c[0], c[1]])))
        .sum();
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    h[10..12].copy_from_slice(&(!(sum as u16)).to_be_bytes());
    h
}

fn tcp_syn() -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(&51000u16.to_be_bytes());
    t.extend_from_slice(&443u16.to_be_bytes());
    t.extend_from_slice(&1000u32.to_be_bytes());
    t.extend_from_slice(&0u32.to_be_bytes());
    t.extend_from_slice(&[0x80, 0x02]); // 32 bytes header, SYN
    t.extend_from_slice(&64240u16.to_be_bytes());
    t.extend_from_slice(&[0, 0, 0, 0]);
    // MSS 1460, NOP, window scale 7, SACK permitted, NOP, NOP, NOP, NOP
    t.extend_from_slice(&[2, 4, 0x05, 0xb4, 1, 3, 3, 7, 4, 2, 1, 1]);
    t
}

fn field<'a>(d: &'a Dissection, layer: usize, name: &str) -> &'a Field {
    d.layers[layer]
        .fields
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no field {name} in layer {layer}"))
}

#[test]
fn a_tcp_syn_over_ipv4_is_dissected() {
    let tcp = tcp_syn();
    let mut p = ethernet(0x0800);
    p.extend(ipv4(6, tcp.len(), [192, 168, 1, 10], [1, 1, 1, 1]));
    p.extend(&tcp);
    let d = dissect(&p, ETH);
    assert_eq!(d.protocol, "TCP");
    assert_eq!(d.src, "192.168.1.10");
    assert_eq!(d.dst, "1.1.1.1");
    assert_eq!(d.info, "51000 → 443 [SYN] Seq=1000 Win=64240 Len=0");
    assert!(!d.truncated);
    assert_eq!(d.layers.len(), 3);
    assert_eq!(d.layers[0].range, 0..14);
    assert_eq!(d.layers[1].range, 14..34);
    assert_eq!(d.layers[2].range, 34..66);
    assert!(field(&d, 1, "Header checksum").value.ends_with("[correct]"));
    assert_eq!(field(&d, 1, "Time to live").value, "64");
    let options = &field(&d, 2, "Options").children;
    assert_eq!(options[0].name, "Maximum segment size");
    assert_eq!(options[0].value, "1460 bytes");
    assert!(
        options
            .iter()
            .any(|o| o.name == "Window scale" && o.value.starts_with("7 "))
    );
    assert!(options.iter().any(|o| o.name == "SACK permitted"));
    let flags = &field(&d, 2, "Flags").children;
    assert_eq!(flags.iter().filter(|f| f.value == "Set").count(), 1);
}

#[test]
fn a_wrong_ipv4_checksum_is_reported() {
    let mut p = ethernet(0x0800);
    let mut ip = ipv4(17, 8, [10, 0, 0, 1], [10, 0, 0, 2]);
    ip[10] ^= 0xff;
    p.extend(ip);
    p.extend_from_slice(&[0, 53, 0, 53, 0, 8, 0, 0]);
    let d = dissect(&p, ETH);
    assert!(field(&d, 1, "Header checksum").value.contains("incorrect"));
}

#[test]
fn udp_payload_ends_with_the_ip_packet_not_with_the_padding() {
    let payload = b"hello";
    let mut udp = Vec::new();
    udp.extend_from_slice(&5353u16.to_be_bytes());
    udp.extend_from_slice(&5353u16.to_be_bytes());
    udp.extend_from_slice(&13u16.to_be_bytes());
    udp.extend_from_slice(&[0, 0]);
    udp.extend_from_slice(payload);
    let mut p = ethernet(0x0800);
    p.extend(ipv4(17, udp.len(), [10, 0, 0, 1], [224, 0, 0, 251]));
    p.extend(&udp);
    p.extend_from_slice(&[0; 13]); // Ethernet padding up to 60 bytes
    let d = dissect(&p, ETH);
    assert_eq!(d.protocol, "UDP");
    assert_eq!(d.info, "5353 → 5353 Len=5");
    let data = d.layers.last().unwrap();
    assert_eq!(data.title, "Data (5 bytes)");
    assert_eq!(&p[data.range.clone()], payload);
}

#[test]
fn an_arp_request_and_reply_get_their_info() {
    let mut p = ethernet(0x0806);
    p.extend_from_slice(&[0, 1, 0x08, 0, 6, 4, 0, 1]);
    p.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 192, 168, 1, 10]);
    p.extend_from_slice(&[0, 0, 0, 0, 0, 0, 192, 168, 1, 1]);
    let d = dissect(&p, ETH);
    assert_eq!(d.protocol, "ARP");
    assert_eq!(d.info, "Who has 192.168.1.1? Tell 192.168.1.10");
    p[21] = 2;
    let d = dissect(&p, ETH);
    assert_eq!(d.info, "192.168.1.10 is at aa:bb:cc:dd:ee:ff");
    // without IP the list shows the MAC addresses
    assert_eq!(d.src, "aa:bb:cc:dd:ee:ff");
}

#[test]
fn vlan_tags_are_followed() {
    let mut p = ethernet(0x8100);
    p.extend_from_slice(&[0x20, 0x0a, 0x08, 0x00]); // priority 1, VLAN 10, IPv4
    p.extend(ipv4(1, 8, [10, 0, 0, 1], [10, 0, 0, 2]));
    p.extend_from_slice(&[8, 0, 0, 0, 0x00, 0x01, 0x00, 0x07]);
    let d = dissect(&p, ETH);
    assert_eq!(d.layers[1].title, "802.1Q Virtual LAN, PRI: 1, ID: 10");
    assert_eq!(d.protocol, "ICMP");
    assert_eq!(d.info, "Echo (ping) request id=0x0001 seq=7");
}

#[test]
fn an_ipv6_neighbor_solicitation_after_a_hop_by_hop_header() {
    let target: [u8; 16] = "fe80::1".parse::<std::net::Ipv6Addr>().unwrap().octets();
    let mut icmp = vec![135, 0, 0, 0, 0, 0, 0, 0];
    icmp.extend_from_slice(&target);
    let mut p = ethernet(0x86dd);
    p.extend_from_slice(&[0x60, 0, 0, 0]);
    p.extend_from_slice(&u16::try_from(8 + icmp.len()).unwrap().to_be_bytes());
    p.extend_from_slice(&[0, 255]); // next: hop-by-hop
    p.extend_from_slice(&"fe80::2".parse::<std::net::Ipv6Addr>().unwrap().octets());
    p.extend_from_slice(
        &"ff02::1:ff00:1"
            .parse::<std::net::Ipv6Addr>()
            .unwrap()
            .octets(),
    );
    p.extend_from_slice(&[58, 0, 5, 2, 0, 0, 1, 0]); // hop-by-hop, next: ICMPv6
    p.extend(&icmp);
    let d = dissect(&p, ETH);
    assert_eq!(d.protocol, "ICMPv6");
    assert_eq!(d.src, "fe80::2");
    assert_eq!(d.info, "Neighbor solicitation for fe80::1");
    assert_eq!(d.layers[2].title, "IPv6 Hop-by-Hop Options");
    assert_eq!(d.layers[2].range, 54..62);
}

#[test]
fn a_cut_packet_is_marked_as_truncated() {
    let tcp = tcp_syn();
    let mut p = ethernet(0x0800);
    p.extend(ipv4(6, tcp.len() + 100, [10, 0, 0, 1], [10, 0, 0, 2]));
    p.extend(&tcp);
    let d = dissect(&p, ETH);
    assert!(d.truncated);
    assert_eq!(d.protocol, "TCP");
    let d = dissect(&p[..40], ETH);
    assert!(d.truncated);
    assert!(
        d.layers
            .last()
            .unwrap()
            .fields
            .iter()
            .any(|f| f.name == "[Truncated]")
    );
}

#[test]
fn linux_cooked_capture_and_raw_ip() {
    let udp = [0, 67, 0, 68, 0, 8, 0, 0];
    let ip = ipv4(17, udp.len(), [10, 0, 0, 1], [255, 255, 255, 255]);
    let mut sll = vec![
        0, 1, 0, 1, 0, 6, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0, 0, 0x08, 0x00,
    ];
    sll.extend(&ip);
    sll.extend_from_slice(&udp);
    let d = dissect(&sll, LinkType::LinuxSll(Linktype::LINUX_SLL));
    assert_eq!(
        d.layers[0].title,
        "Linux cooked capture v1, Src: aa:bb:cc:dd:ee:ff"
    );
    assert_eq!(d.protocol, "UDP");
    assert_eq!(d.dst, "255.255.255.255");
    let d = dissect(&sll[16..], LinkType::RawIp(Linktype(101)));
    assert_eq!(d.layers[0].range, 0..20);
    assert_eq!(d.info, "67 → 68 Len=0");
}

/// No input may panic, however short or garbled
#[test]
fn every_prefix_and_random_bytes_are_handled() {
    let tcp = tcp_syn();
    let mut packets = vec![{
        let mut p = ethernet(0x0800);
        p.extend(ipv4(6, tcp.len(), [10, 0, 0, 1], [10, 0, 0, 2]));
        p.extend(&tcp);
        p
    }];
    let mut seed: u32 = 0x1234_5678;
    for _ in 0..300 {
        let len = 14 + (seed % 120) as usize;
        let mut p = ethernet(match seed % 5 {
            0 => 0x0800,
            1 => 0x86dd,
            2 => 0x0806,
            3 => 0x8100,
            _ => 0x88cc,
        });
        for _ in 0..len {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            p.push((seed >> 16) as u8);
        }
        // a plausible IP version, so the random bytes reach the transport layer
        p[14] = if seed % 2 == 0 { 0x45 } else { 0x60 };
        packets.push(p);
    }
    let link_types = [
        ETH,
        LinkType::LinuxSll(Linktype::LINUX_SLL),
        LinkType::LinuxSll2(Linktype::LINUX_SLL2),
        LinkType::Null(Linktype::NULL),
        LinkType::RawIp(Linktype(101)),
    ];
    for p in &packets {
        for end in 0..=p.len() {
            for link_type in link_types {
                let d = dissect(&p[..end], link_type);
                for layer in &d.layers {
                    assert!(layer.range.start <= end, "{:?}", layer.range);
                }
            }
        }
    }
}

// ---- application protocols -------------------------------------------------------------------

pub(crate) fn udp_packet(sport: u16, dport: u16, payload: &[u8]) -> Vec<u8> {
    let mut p = ethernet(0x0800);
    p.extend(ipv4(
        17,
        8 + payload.len(),
        [192, 168, 1, 10],
        [192, 168, 1, 1],
    ));
    p.extend_from_slice(&sport.to_be_bytes());
    p.extend_from_slice(&dport.to_be_bytes());
    p.extend_from_slice(&u16::try_from(8 + payload.len()).unwrap().to_be_bytes());
    p.extend_from_slice(&[0, 0]);
    p.extend_from_slice(payload);
    p
}

pub(crate) fn tcp_packet(sport: u16, dport: u16, seq: u32, payload: &[u8]) -> Vec<u8> {
    let mut p = ethernet(0x0800);
    p.extend(ipv4(
        6,
        20 + payload.len(),
        [192, 168, 1, 10],
        [10, 0, 0, 2],
    ));
    p.extend_from_slice(&sport.to_be_bytes());
    p.extend_from_slice(&dport.to_be_bytes());
    p.extend_from_slice(&seq.to_be_bytes());
    p.extend_from_slice(&1u32.to_be_bytes());
    p.extend_from_slice(&[0x50, 0x18]); // 20 bytes header, PSH ACK
    p.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0]);
    p.extend_from_slice(payload);
    p
}

fn dns_name(name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for label in name.split('.') {
        out.push(u8::try_from(label.len()).unwrap());
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    out
}

pub(crate) fn dns_query(id: u16, flags: u16, name: &str, qtype: u16, qclass: u16) -> Vec<u8> {
    let mut m = Vec::new();
    m.extend_from_slice(&id.to_be_bytes());
    m.extend_from_slice(&flags.to_be_bytes());
    m.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0]);
    m.extend(dns_name(name));
    m.extend_from_slice(&qtype.to_be_bytes());
    m.extend_from_slice(&qclass.to_be_bytes());
    m
}

/// The last layer (the application protocol)
fn last(d: &Dissection) -> &super::Layer {
    d.layers.last().unwrap()
}

fn last_field<'a>(d: &'a Dissection, name: &str) -> &'a Field {
    field(d, d.layers.len() - 1, name)
}

#[test]
fn a_dns_query_and_its_compressed_answer() {
    let query = dns_query(0x1a2b, 0x0100, "github.com", 1, 1);
    let d = dissect(&udp_packet(53001, 53, &query), ETH);
    assert_eq!(d.protocol, "DNS");
    assert_eq!(d.info, "Query 0x1a2b A github.com");
    let queries = &last_field(&d, "Queries").children;
    assert_eq!(queries[0].value, "github.com: type A, class IN");
    // the name lies after the 42 bytes of headers and the 12 bytes of the DNS header
    assert_eq!(queries[0].children[0].range, 54..66);

    let mut answer = dns_query(0x1a2b, 0x8180, "github.com", 1, 1);
    answer[7] = 2; // two answers
    for ip in [[140, 82, 121, 4], [140, 82, 121, 3]] {
        answer.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
        answer.extend_from_slice(&ip);
    }
    let d = dissect(&udp_packet(53, 53001, &answer), ETH);
    assert_eq!(
        d.info,
        "Response 0x1a2b A github.com: 140.82.121.4, 140.82.121.3"
    );
    let answers = &last_field(&d, "Answers").children;
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0].children[0].value, "github.com");
    assert_eq!(answers[0].children[3].value, "60 s");
    assert!(last(&d).title.contains("response"));

    let mut missing = dns_query(0x0042, 0x8183, "nothing.example", 28, 1);
    missing[2] = 0x81;
    let d = dissect(&udp_packet(53, 53001, &missing), ETH);
    assert_eq!(d.info, "Response 0x0042 AAAA nothing.example: No such name");
}

#[test]
fn dns_over_tcp_has_a_length_prefix() {
    let query = dns_query(7, 0x0100, "example.org", 65, 1);
    let mut payload = u16::try_from(query.len()).unwrap().to_be_bytes().to_vec();
    payload.extend(&query);
    let d = dissect(&tcp_packet(40000, 53, 1, &payload), ETH);
    assert_eq!(d.protocol, "DNS");
    assert_eq!(d.info, "Query 0x0007 HTTPS example.org");
}

#[test]
fn a_dns_loop_of_pointers_ends() {
    let mut m = dns_query(1, 0x0100, "a", 1, 1);
    // the name points to itself
    m[12] = 0xc0;
    m[13] = 12;
    let d = dissect(&udp_packet(53001, 53, &m), ETH);
    assert_eq!(d.protocol, "DNS");
    assert!(last_field(&d, "[Incomplete]").value.contains("ends early"));
}

#[test]
fn multicast_dns_shows_the_unicast_bit_and_records() {
    let mut m = dns_query(0, 0x8400, "_googlecast._tcp.local", 12, 0x8001);
    m[7] = 1;
    // PTR answer, cache flush, pointing into the question
    m.extend_from_slice(&[0xc0, 0x0c, 0, 12, 0x80, 0x01, 0, 0, 0x11, 0x94, 0, 9]);
    m.extend_from_slice(&[6, b'k', b'u', b'e', b'c', b'h', b'e', 0xc0, 0x0c]);
    let d = dissect(&udp_packet(5353, 5353, &m), ETH);
    assert_eq!(d.protocol, "mDNS");
    assert_eq!(
        d.info,
        "Response PTR _googlecast._tcp.local: kueche._googlecast._tcp.local"
    );
    let query = &last_field(&d, "Queries").children[0];
    assert!(query.value.ends_with("class IN, unicast response"));
    let answer = &last_field(&d, "Answers").children[0];
    assert_eq!(answer.children[2].value, "IN, cache flush");
}

pub(crate) fn dhcp_message(op: u8, options: &[u8]) -> Vec<u8> {
    let mut m = vec![op, 1, 6, 0, 0xde, 0xad, 0xbe, 0xef, 0, 0, 0x80, 0];
    m.extend_from_slice(&[0; 16]);
    m.extend_from_slice(&[0x02, 0x42, 0xac, 0x11, 0, 2]);
    m.resize(236, 0);
    m.extend_from_slice(&[0x63, 0x82, 0x53, 0x63]);
    m.extend_from_slice(options);
    m.push(255);
    m
}

#[test]
fn a_dhcp_request_shows_host_name_and_wanted_address() {
    let mut options = vec![53, 1, 3, 50, 4, 192, 168, 1, 20, 12, 6];
    options.extend_from_slice(b"laptop");
    options.extend_from_slice(&[55, 3, 1, 3, 6]);
    let d = dissect(&udp_packet(68, 67, &dhcp_message(1, &options)), ETH);
    assert_eq!(d.protocol, "DHCP");
    assert_eq!(
        d.info,
        "DHCP Request - Transaction ID 0xdeadbeef - laptop (wants 192.168.1.20)"
    );
    assert_eq!(
        last_field(&d, "Client MAC address").value,
        "02:42:ac:11:00:02"
    );
    let options: Vec<&str> = last(&d)
        .fields
        .iter()
        .filter(|f| f.name == "Option")
        .map(|f| f.value.as_str())
        .collect();
    assert_eq!(options[0], "(53) DHCP message type: Request (3)");
    assert_eq!(options[3], "(55) Parameter request list: 1, 3, 6");
    assert_eq!(options[4], "(255) End");
}

/// A TLS record with a Client Hello; `padding` bytes of the padding extension come before the
/// server name, so a large value moves it into the next segment
pub(crate) fn client_hello(server: &str, padding: usize) -> Vec<u8> {
    let mut ext = Vec::new();
    if padding > 0 {
        ext.extend_from_slice(&21u16.to_be_bytes());
        ext.extend_from_slice(&u16::try_from(padding).unwrap().to_be_bytes());
        ext.resize(ext.len() + padding, 0);
    }
    let n = u16::try_from(server.len()).unwrap();
    ext.extend_from_slice(&0u16.to_be_bytes());
    ext.extend_from_slice(&(n + 5).to_be_bytes());
    ext.extend_from_slice(&(n + 3).to_be_bytes());
    ext.push(0);
    ext.extend_from_slice(&n.to_be_bytes());
    ext.extend_from_slice(server.as_bytes());
    ext.extend_from_slice(&[0, 16, 0, 5, 0, 3, 2, b'h', b'2']); // ALPN h2
    ext.extend_from_slice(&[0, 43, 0, 5, 4, 0x03, 0x04, 0x03, 0x03]); // TLS 1.3, 1.2
    let mut hello = vec![0x03, 0x03];
    hello.extend_from_slice(&[0x5a; 32]);
    hello.push(0);
    hello.extend_from_slice(&[0, 4, 0x13, 0x01, 0x0a, 0x0a, 1, 0]);
    hello.extend_from_slice(&u16::try_from(ext.len()).unwrap().to_be_bytes());
    hello.extend(ext);
    let mut handshake = vec![1];
    handshake.extend_from_slice(&u32::try_from(hello.len()).unwrap().to_be_bytes()[1..]);
    handshake.extend(hello);
    let mut record = vec![22, 3, 1];
    record.extend_from_slice(&u16::try_from(handshake.len()).unwrap().to_be_bytes());
    record.extend(handshake);
    record
}

#[test]
fn a_tls_client_hello_names_the_server() {
    let d = dissect(
        &tcp_packet(51000, 443, 1, &client_hello("example.org", 0)),
        ETH,
    );
    assert_eq!(d.protocol, "TLSv1.3");
    assert_eq!(d.info, "Client Hello (SNI=example.org)");
    assert_eq!(d.followup, None);
    let hello = &last(&d).fields[3];
    assert_eq!(hello.name, "Handshake");
    let suites = hello
        .children
        .iter()
        .find(|f| f.name == "Cipher suites")
        .unwrap();
    assert_eq!(suites.children[0].value, "TLS_AES_128_GCM_SHA256 (0x1301)");
    assert_eq!(suites.children[1].value, "GREASE (0x0a0a)");
    let values: Vec<&str> = hello
        .children
        .iter()
        .filter(|f| f.name == "Extension")
        .map(|f| f.value.as_str())
        .collect();
    assert_eq!(
        values,
        [
            "server_name: example.org",
            "application_layer_protocol_negotiation: h2",
            "supported_versions: TLS 1.3, TLS 1.2"
        ]
    );
}

#[test]
fn a_client_hello_over_two_segments_is_completed() {
    let record = client_hello("split.example.org", 1600);
    let (first, second) = record.split_at(1400);
    let p1 = tcp_packet(51000, 443, 5000, first);
    let p2 = tcp_packet(51000, 443, 6400, second);
    let d = dissect(&p1, ETH);
    assert_eq!(d.info, "Client Hello");
    assert_eq!(
        d.followup,
        Some(Followup {
            next_seq: 6400,
            missing: second.len()
        })
    );
    let (seq, range) = tcp_payload(&p2, ETH).unwrap();
    assert_eq!(seq, 6400);
    let d = dissect_with(&p1, ETH, &p2[range]);
    assert_eq!(d.info, "Client Hello (SNI=split.example.org)");
    assert_eq!(d.followup, None);
    // the following segment alone is only data of the connection
    let d = dissect(&p2, ETH);
    assert_eq!(d.info, "Continuation Data");
}

#[test]
fn several_tls_records_in_one_segment() {
    let mut payload = vec![20, 3, 3, 0, 1, 1];
    payload.extend_from_slice(&[23, 3, 3, 0, 4, 1, 2, 3, 4]);
    payload.extend_from_slice(&[21, 3, 3, 0, 2, 2, 40]);
    let d = dissect(&tcp_packet(443, 51000, 1, &payload), ETH);
    assert_eq!(d.protocol, "TLSv1.2");
    assert_eq!(
        d.info,
        "Change Cipher Spec, Application Data, Alert (Handshake Failure)"
    );
    assert_eq!(d.layers.len(), 6);
}

#[test]
fn an_http_request_and_its_response() {
    let request = b"GET /index.html HTTP/1.1\r\nHost: example.org\r\nUser-Agent: test\r\n\r\n";
    let d = dissect(&tcp_packet(51000, 80, 1, request), ETH);
    assert_eq!(d.protocol, "HTTP");
    assert_eq!(d.info, "GET /index.html HTTP/1.1 (Host: example.org)");
    assert_eq!(last(&d).fields.len(), 3);
    assert_eq!(last(&d).fields[1].value, "Host: example.org");

    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nhello";
    let d = dissect(&tcp_packet(80, 51000, 1, response), ETH);
    assert_eq!(d.info, "HTTP/1.1 200 OK");
    assert_eq!(last(&d).title, "Message body (5 bytes)");
    assert_eq!(last_field(&d, "Content type").value, "text/plain");
}

#[test]
fn ssdp_notify_and_search() {
    let notify = b"NOTIFY * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nNT: upnp:rootdevice\r\nNTS: ssdp:alive\r\nSERVER: Linux UPnP/1.0\r\n\r\n";
    let d = dissect(&udp_packet(1900, 1900, notify), ETH);
    assert_eq!(d.protocol, "SSDP");
    assert_eq!(
        d.info,
        "NOTIFY * HTTP/1.1 upnp:rootdevice (ssdp:alive) – Linux UPnP/1.0"
    );
    let search = b"M-SEARCH * HTTP/1.1\r\nST: ssdp:all\r\nMAN: \"ssdp:discover\"\r\n\r\n";
    let d = dissect(&udp_packet(50000, 1900, search), ETH);
    assert_eq!(d.info, "M-SEARCH * HTTP/1.1 ssdp:all");
}

#[test]
fn an_ntp_client_request() {
    let mut m = vec![0x23, 0, 6, 0xe9];
    m.resize(40, 0);
    // transmit timestamp 2026-01-01 00:00:00 UTC
    m.extend_from_slice(&(1_767_225_600u32 + 2_208_988_800).to_be_bytes());
    m.extend_from_slice(&[0x80, 0, 0, 0]);
    let d = dissect(&udp_packet(50123, 123, &m), ETH);
    assert_eq!(d.protocol, "NTP");
    assert_eq!(d.info, "NTP Version 4, client");
    assert_eq!(
        last_field(&d, "Transmit timestamp").value,
        "2026-01-01T00:00:00.500000Z"
    );
    assert_eq!(last_field(&d, "Origin timestamp").value, "(not set)");
}

#[test]
fn a_quic_initial_packet() {
    let mut m = vec![0xc3, 0, 0, 0, 1, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0];
    m.resize(1200, 0x77);
    let d = dissect(&udp_packet(51000, 443, &m), ETH);
    assert_eq!(d.protocol, "QUIC");
    assert_eq!(d.info, "Initial, DCID=0102030405060708");
    assert_eq!(last_field(&d, "Version").value, "1");
    let d = dissect(&udp_packet(443, 51000, &[0x45, 1, 2, 3, 4]), ETH);
    assert_eq!(d.info, "Protected Payload");
}

/// Garbled payloads on the ports of the application protocols may not panic
#[test]
fn garbled_application_payloads_are_handled() {
    let mut seed: u32 = 0x9e37_79b9;
    let mut random = |len: usize| -> Vec<u8> {
        (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (seed >> 16) as u8
            })
            .collect()
    };
    let samples = [
        dns_query(1, 0x8180, "a.example.org", 33, 1),
        dhcp_message(2, &[53, 1, 5, 51, 4, 0, 0, 0x0e, 0x10]),
        client_hello("x.example", 40),
        b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec(),
    ];
    let ports = [53, 5353, 5355, 67, 547, 1900, 123, 443, 80];
    for port in ports {
        let mut payloads: Vec<Vec<u8>> = samples.to_vec();
        for i in 0..40 {
            payloads.push(random(i * 7));
            // a sample with some bytes garbled
            let mut garbled = samples[i % samples.len()].clone();
            for _ in 0..4 {
                let at = (random(1)[0] as usize * 7) % garbled.len();
                garbled[at] = random(1)[0];
            }
            payloads.push(garbled);
        }
        for payload in &payloads {
            for end in 0..=payload.len() {
                let _ = dissect(&udp_packet(40000, port, &payload[..end]), ETH);
                let _ = dissect(&tcp_packet(40000, port, 1, &payload[..end]), ETH);
            }
        }
    }
}

// ---- VoIP: made-up messages and streams, no real numbers or captures ---------------------------

/// A UDP datagram between two IPv4 addresses
pub(crate) fn udp_between(
    src: [u8; 4],
    sport: u16,
    dst: [u8; 4],
    dport: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut p = ethernet(0x0800);
    p.extend(ipv4(17, 8 + payload.len(), src, dst));
    p.extend_from_slice(&sport.to_be_bytes());
    p.extend_from_slice(&dport.to_be_bytes());
    p.extend_from_slice(&u16::try_from(8 + payload.len()).unwrap().to_be_bytes());
    p.extend_from_slice(&[0, 0]);
    p.extend_from_slice(payload);
    p
}

/// A SIP message with these header lines and a session description as its body
pub(crate) fn sip_message(first: &str, headers: &[(&str, &str)], sdp: Option<&str>) -> Vec<u8> {
    let mut m = format!("{first}\r\n");
    for (name, value) in headers {
        m.push_str(&format!("{name}: {value}\r\n"));
    }
    let body = sdp.unwrap_or_default();
    if sdp.is_some() {
        m.push_str("Content-Type: application/sdp\r\n");
    }
    m.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    m.into_bytes()
}

/// A session description offering audio at this address and port with these payload types
pub(crate) fn sdp_offer(addr: &str, port: u16, formats: &[u8]) -> String {
    let names = |pt: u8| match pt {
        0 => "PCMU/8000",
        8 => "PCMA/8000",
        101 => "telephone-event/8000",
        _ => "opus/48000/2",
    };
    let list: Vec<String> = formats.iter().map(u8::to_string).collect();
    let mut s = format!(
        "v=0\r\no=anna 1234 1234 IN IP4 {addr}\r\ns=-\r\nc=IN IP4 {addr}\r\nt=0 0\r\nm=audio {port} RTP/AVP {}\r\n",
        list.join(" ")
    );
    for pt in formats {
        s.push_str(&format!("a=rtpmap:{pt} {}\r\n", names(*pt)));
    }
    s.push_str("a=ptime:20\r\na=sendrecv\r\n");
    s
}

/// An RTP packet with `len` bytes of payload (A-law silence)
pub(crate) fn rtp_bytes(pt: u8, seq: u16, ts: u32, ssrc: u32, marker: bool, len: usize) -> Vec<u8> {
    let mut m = vec![0x80, (u8::from(marker) << 7) | pt];
    m.extend_from_slice(&seq.to_be_bytes());
    m.extend_from_slice(&ts.to_be_bytes());
    m.extend_from_slice(&ssrc.to_be_bytes());
    m.extend(std::iter::repeat_n(0xd5, len));
    m
}

/// An RTCP sender report on `about` with a source description (CNAME)
pub(crate) fn rtcp_report(ssrc: u32, about: u32, cname: &str) -> Vec<u8> {
    rtcp_report_on(ssrc, about, 5, 24, cname)
}

/// An RTCP sender report: `about` lost `fraction`/256 of its packets and has this jitter (in
/// units of the clock)
pub(crate) fn rtcp_report_on(
    ssrc: u32,
    about: u32,
    fraction: u8,
    jitter: u32,
    cname: &str,
) -> Vec<u8> {
    let mut m = vec![0x81, 200, 0, 12];
    m.extend_from_slice(&ssrc.to_be_bytes());
    // NTP time 2026-11-09 12:00:00 UTC and its fraction
    m.extend_from_slice(&4_003_214_400_u32.to_be_bytes());
    m.extend_from_slice(&0x8000_0000_u32.to_be_bytes());
    m.extend_from_slice(&160_000_u32.to_be_bytes());
    m.extend_from_slice(&1000_u32.to_be_bytes());
    m.extend_from_slice(&160_000_u32.to_be_bytes());
    // the report block: the fraction lost, 12 in all, the jitter
    m.extend_from_slice(&about.to_be_bytes());
    m.extend_from_slice(&[fraction, 0, 0, 12]);
    m.extend_from_slice(&3010_u32.to_be_bytes());
    m.extend_from_slice(&jitter.to_be_bytes());
    m.extend_from_slice(&[0; 8]);
    let mut chunk = ssrc.to_be_bytes().to_vec();
    chunk.push(1);
    chunk.push(u8::try_from(cname.len()).unwrap());
    chunk.extend_from_slice(cname.as_bytes());
    chunk.push(0);
    while chunk.len() % 4 != 0 {
        chunk.push(0);
    }
    m.extend_from_slice(&[0x81, 202]);
    m.extend_from_slice(&u16::try_from(chunk.len() / 4).unwrap().to_be_bytes());
    m.extend(chunk);
    m
}

mod voip;
