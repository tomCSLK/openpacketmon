use super::*;
use crate::dissect::dissect;
use crate::dissect::tests::{ETH, client_hello, tcp_packet};

fn matches(filter: &str, data: &[u8]) -> bool {
    let d = dissect(data, ETH);
    let packet = Packet {
        number: 7,
        len: u32::try_from(data.len()).unwrap(),
        captured: data.len(),
        d: &d,
    };
    Filter::parse(filter)
        .unwrap_or_else(|e| panic!("{filter}: {e:?}"))
        .unwrap()
        .matches(&packet)
}

fn error(filter: &str) -> Error {
    Filter::parse(filter).unwrap_err()
}

#[test]
fn protocols_fields_and_comparisons() {
    // 192.168.1.10:51000 → 10.0.0.2:443, TLS Client Hello for example.org
    let p = tcp_packet(51000, 443, 1, &client_hello("www.Example.org", 0));
    for yes in [
        "tcp",
        "tls",
        "ip",
        "ipv4",
        "frame",
        "eth",
        "ip.addr == 10.0.0.2",
        "ip.src == 192.168.1.0/24",
        "ip.dst != 192.168.1.10",
        "tcp.port == 443",
        "port in {80, 443}",
        "tcp.dstport >= 443 && tcp.srcport > 50000",
        "tls.sni == www.example.org",
        "tls.sni contains EXAMPLE",
        "domain contains \"example\"",
        "tls.handshake.type == client_hello",
        "tls.version == 1.3",
        "tls.alpn == h2",
        "tls.sni and not udp",
        "tcp.flags.psh and !tcp.flags.syn",
        "frame.number == 7 and frame.len > 100",
        "not (dns or arp)",
        "udp or tcp and tls",
        "eth.src == aa:bb:cc:dd:ee:ff",
        "eth.addr == 11-22-33-44-55-66",
    ] {
        assert!(matches(yes, &p), "{yes}");
    }
    for no in [
        "udp",
        "dns",
        "ipv6",
        "ip.addr == 10.0.0.3",
        "ip.src == 10.0.0.0/8",
        "ip.addr != 10.0.0.2",
        "tcp.port == 80",
        "tcp.flags.syn",
        "tls.sni contains amazon",
        "dns.qry.name contains example",
        "ip.addr == fe80::1",
        "(udp or dns) and tcp",
        "tls.alert",
    ] {
        assert!(!matches(no, &p), "{no}");
    }
}

#[test]
fn dns_names_and_types() {
    let mut query = vec![0x12, 0x34, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    query.extend_from_slice(&[
        6, b'g', b'i', b't', b'h', b'u', b'b', 3, b'c', b'o', b'm', 0,
    ]);
    query.extend_from_slice(&[0, 28, 0, 1]);
    let p = crate::dissect::tests::udp_packet(53001, 53, &query);
    assert!(matches(
        "dns.qry.type == AAAA and dns.qry.name == github.com",
        &p
    ));
    assert!(matches(
        "domain == GitHub.com and dns.flags.response == 0",
        &p
    ));
    assert!(matches("udp.port == 53 and port == 53", &p));
    assert!(!matches("dns.flags.response", &p));
    assert!(!matches("dns.a", &p));
}

#[test]
fn an_empty_filter_selects_everything() {
    assert!(Filter::parse("").unwrap().is_none());
    assert!(Filter::parse("   ").unwrap().is_none());
}

#[test]
fn errors_name_the_problem_and_its_place() {
    assert_eq!(
        error("tcp and foo.bar == 1"),
        Error {
            kind: ErrorKind::UnknownField("foo.bar".into()),
            pos: 8
        }
    );
    assert_eq!(error("tcp.port ==").kind, ErrorKind::MissingValue);
    assert_eq!(error("tcp.port ==").pos, 11);
    assert_eq!(
        error("tcp.port == https").kind,
        ErrorKind::BadValue {
            field: "tcp.port".into(),
            value: "https".into()
        }
    );
    assert_eq!(
        error("ip.addr contains 10").kind,
        ErrorKind::BadOperator {
            field: "ip.addr".into(),
            op: "contains".into()
        }
    );
    assert_eq!(error("(tcp or udp").kind, ErrorKind::MissingClose);
    assert_eq!(error("tls.sni == \"abc").kind, ErrorKind::UnclosedString);
    assert_eq!(error("tcp udp").kind, ErrorKind::Unexpected("udp".into()));
    assert_eq!(error("ip.addr == 10.0.0.0/33").pos, 11);
    assert_eq!(error("tcp and").kind, ErrorKind::MissingValue);
    assert_eq!(error("port in {}").kind, ErrorKind::MissingValue);
    assert_eq!(
        error("dns contains x").kind,
        ErrorKind::BadOperator {
            field: "dns".into(),
            op: "contains".into()
        }
    );
}

#[test]
fn the_color_rules_parse_and_apply() {
    for (class, text) in COLOR_RULES {
        assert!(Filter::parse(text).is_ok(), "{class}: {text}");
    }
    let d = dissect(
        &tcp_packet(51000, 443, 1, &client_hello("a.example", 0)),
        ETH,
    );
    let packet = Packet {
        number: 1,
        len: 100,
        captured: 100,
        d: &d,
    };
    assert_eq!(color(&packet), Some("tls"));
}

/// No input may panic
#[test]
fn garbage_is_rejected_without_panic() {
    for text in [
        "==",
        "(",
        ")",
        "!",
        "&&",
        "{",
        "\"",
        "ip.addr == ::",
        "port in {1 2",
        "a=b=c",
        "äöü == 1",
        "tcp.port == 0xzz",
        "eth.src == 1:2:3",
        "ip.addr == 1.2.3.4/x",
        "not not not",
    ] {
        let _ = Filter::parse(text);
    }
    // deep nesting and long texts are refused instead of exhausting the stack
    assert_eq!(error(&"(".repeat(100_000)).kind, ErrorKind::TooComplex);
    assert_eq!(error(&"not ".repeat(1000)).kind, ErrorKind::TooComplex);
    assert_eq!(error(&"!".repeat(4000)).kind, ErrorKind::TooComplex);
    let long = vec!["tcp"; 2000].join(" or ");
    assert_eq!(error(&long).kind, ErrorKind::TooComplex);
    let nested = format!("{}tcp{}", "(".repeat(60), ")".repeat(60));
    assert!(Filter::parse(&nested).unwrap().is_some());
}

#[test]
fn times_in_seconds_or_with_a_unit() {
    use super::parse_time;
    assert_eq!(parse_time("0.25"), Some(250_000));
    assert_eq!(parse_time("2"), Some(2_000_000));
    assert_eq!(parse_time("1s"), Some(1_000_000));
    assert_eq!(parse_time("100ms"), Some(100_000));
    assert_eq!(parse_time("800us"), Some(800));
    assert_eq!(parse_time("1.5MS"), Some(1_500));
    for bad in ["", "ms", "-1", "nan", "inf", "1e99", "fast"] {
        assert_eq!(parse_time(bad), None, "{bad}");
    }
    assert!(Filter::parse("dns.time > 100ms").is_ok());
    assert!(Filter::parse("dns.time contains 1").is_err());
    assert!(Filter::parse("tcp.analysis.ack_rtt >= 0.2 or tcp.analysis.retransmission").is_ok());
}

#[test]
fn connections_have_fields_of_their_own() {
    let protocols = ["ip", "tcp"];
    let values = [
        ("ip.src", Value::Ip([192, 168, 1, 20].into())),
        ("ip.dst", Value::Ip([142, 250, 1, 1].into())),
        ("remote", Value::Ip([142, 250, 1, 1].into())),
        ("proto", Value::Text("TCP".into())),
        ("tcp.srcport", Value::Num(51000)),
        ("tcp.dstport", Value::Num(443)),
        ("service", Value::Text("https".into())),
        ("domain", Value::Text("google.com".into())),
        ("as.name", Value::Text("GOOGLE".into())),
        ("country", Value::Text("US".into())),
        ("program", Value::Text("firefox".into())),
        (
            "program.path",
            Value::Text("/usr/lib/firefox/firefox".into()),
        ),
        ("direction", Value::Text("outgoing".into())),
        ("bytes", Value::Num(5000)),
        ("favorite", Value::Num(1)),
    ];
    let connection = Connection {
        protocols: &protocols,
        values: &values,
    };
    let matches = |text: &str| {
        Filter::parse_connections(text)
            .unwrap_or_else(|e| panic!("{text}: {e:?}"))
            .unwrap()
            .matches_connection(&connection)
    };
    for yes in [
        "tcp",
        "ipv4",
        "ip.addr == 192.168.1.0/24",
        "addr == 142.250.1.1",
        "port == 443 and proto == tcp",
        "srcport > 50000",
        "service == HTTPS",
        "domain contains google and country == us",
        "as contains goo",
        "program == firefox",
        "program contains /usr/lib",
        "direction == outgoing",
        "bytes > 1000",
        "favorite",
        "remote == 142.250.0.0/16",
    ] {
        assert!(matches(yes), "{yes}");
    }
    for no in [
        "udp",
        "ipv6",
        "port == 80",
        "udp.port == 443",
        "not favorite",
        "country != US",
        "direction == incoming",
    ] {
        assert!(!matches(no), "{no}");
    }
    // packet fields are unknown to the connection filter and the other way round
    assert!(matches!(
        Filter::parse_connections("tls.sni contains x")
            .unwrap_err()
            .kind,
        ErrorKind::UnknownField(_)
    ));
    assert!(matches!(
        Filter::parse("favorite").unwrap_err().kind,
        ErrorKind::UnknownField(_)
    ));
    assert_eq!(quote(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    assert!(matches(&format!("domain == {}", quote("google.com"))));
}

#[test]
fn values_are_written_so_that_the_filter_reads_them_back() {
    assert_eq!(literal("DE"), "DE");
    assert_eq!(literal("192.168.1.20"), "192.168.1.20");
    assert_eq!(literal("aa:bb:cc:dd:ee:ff"), "aa:bb:cc:dd:ee:ff");
    assert_eq!(literal("Amazon Data Services"), "\"Amazon Data Services\"");
    assert_eq!(literal("in"), "\"in\"");
    assert_eq!(literal("a=b"), "\"a=b\"");
    assert_eq!(literal("say \"hi\""), "\"say \\\"hi\\\"\"");
    assert_eq!(literal(""), "\"\"");
    assert!(Filter::parse_connections(&format!("program == {}", literal("my \"app"))).is_ok());
}
