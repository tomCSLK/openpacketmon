//! SIP (RFC 3261) over UDP and TCP, with the session description (SDP, RFC 8866) in its body.
//! SIP over TLS (port 5061) stays a TLS connection: its messages cannot be read.

use std::fmt::Write;
use std::net::IpAddr;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Dissection, Field, SdpMedia};

/// SIP is also recognized on other ports than 5060, by its first line (Settings → Calls)
static ANY_PORT: AtomicBool = AtomicBool::new(true);

/// Longest first line looked at
const MAX_LINE: usize = 1024;

/// Header names and their compact forms (RFC 3261 7.3.3 and the extensions that define one)
const COMPACT: [(&str, &str); 17] = [
    ("Accept-Contact", "a"),
    ("Referred-By", "b"),
    ("Content-Type", "c"),
    ("Content-Encoding", "e"),
    ("From", "f"),
    ("Call-ID", "i"),
    ("Supported", "k"),
    ("Content-Length", "l"),
    ("Contact", "m"),
    ("Event", "o"),
    ("Refer-To", "r"),
    ("Subject", "s"),
    ("To", "t"),
    ("Allow-Events", "u"),
    ("Via", "v"),
    ("Session-Expires", "x"),
    ("Identity", "y"),
];

/// Whether SIP is recognized on any port, not only on 5060
pub fn any_port() -> bool {
    ANY_PORT.load(Ordering::Relaxed)
}

/// Sets whether SIP is recognized on any port (Settings → Calls)
pub fn set_any_port(on: bool) {
    ANY_PORT.store(on, Ordering::Relaxed);
}

/// The bytes at `at` start a SIP request (`INVITE sip:… SIP/2.0`) or response (`SIP/2.0 200 OK`)
pub(super) fn is_sip(data: &[u8], at: usize) -> bool {
    let rest = data.get(at..).unwrap_or_default();
    let Some(end) = rest
        .iter()
        .take(MAX_LINE)
        .position(|b| *b == b'\r' || *b == b'\n')
    else {
        return false;
    };
    let line = &rest[..end];
    if let Some(status) = line.strip_prefix(b"SIP/2.0 ") {
        return status.len() >= 3
            && status[..3].iter().all(u8::is_ascii_digit)
            && status.get(3).is_none_or(|b| *b == b' ');
    }
    let mut words = line.split(|b| *b == b' ');
    let (Some(method), Some(uri), Some(version), None) =
        (words.next(), words.next(), words.next(), words.next())
    else {
        return false;
    };
    !method.is_empty()
        && method.iter().all(|b| b.is_ascii_uppercase() || *b == b'-')
        && uri.contains(&b':')
        && version == b"SIP/2.0"
}

/// Text of a SIP line: UTF-8 (display names may hold any letters), control characters as `·`
fn utf8(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .map(|c| if c.is_control() { '·' } else { c })
        .collect()
}

/// The value of a header, also under its compact form
fn header<'a>(head: &'a [(String, Range<usize>)], name: &str) -> Option<&'a str> {
    super::text::header(head, name).or_else(|| {
        let (_, short) = COMPACT
            .iter()
            .find(|(long, _)| long.eq_ignore_ascii_case(name))?;
        super::text::header(head, short)
    })
}

/// A From or To header without its parameters (`"Anna" <sip:anna@example.net>`)
fn address(value: &str) -> &str {
    match value.find('>') {
        Some(end) => &value[..=end],
        None => value.split(';').next().unwrap_or(value).trim(),
    }
}

/// A SIP message; `false` if the payload is none
pub(super) fn dissect(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let (head, body_start) = super::text::lines_as(data, &payload, utf8);
    let Some((first, _)) = head.first() else {
        return false;
    };
    let response = first.starts_with("SIP/2.0 ");
    let mut words = first.splitn(3, ' ');
    let (one, two, three) = (
        words.next().unwrap_or_default(),
        words.next().unwrap_or_default(),
        words.next().unwrap_or_default(),
    );
    let fields = head_fields(&head, response);
    let head_end = body_start.unwrap_or(payload.end);
    let cseq = header(&head, "CSeq").map(|v| {
        let mut parts = v.split_whitespace();
        (
            parts.next().and_then(|n| n.parse::<u32>().ok()),
            parts.next().unwrap_or_default().to_owned(),
        )
    });
    d.protocol = "SIP";
    d.has("sip");
    let status = if response {
        two.parse::<u16>().ok()
    } else {
        None
    };
    if let Some(code) = status {
        d.num("sip.status_code", code);
    } else if !response {
        d.text("sip.method", one);
    }
    if let Some(id) = header(&head, "Call-ID") {
        d.text("sip.call_id", id);
    }
    if let Some(from) = header(&head, "From") {
        d.text("sip.from", address(from));
    }
    if let Some(to) = header(&head, "To") {
        d.text("sip.to", address(to));
    }
    if let Some((number, method)) = &cseq {
        if let Some(n) = number {
            d.num("sip.cseq.seq", *n);
        }
        d.text("sip.cseq.method", method.clone());
    }
    d.add(
        format!(
            "Session Initiation Protocol ({})",
            if response { two } else { one }
        ),
        payload.start..head_end,
        fields,
    );
    let sdp_found = body_start
        .filter(|b| *b < payload.end)
        .is_some_and(|body| self::body(data, &head, body..payload.end, d));
    d.info = if let Some(code) = status {
        let mut info = format!("Status: {code}");
        if !three.is_empty() {
            let _ = write!(info, " {three}");
        }
        if let Some((_, method)) = cseq.as_ref().filter(|(_, m)| !m.is_empty()) {
            let _ = write!(info, " ({method})");
        }
        info
    } else {
        format!("Request: {one} {two}")
    };
    if sdp_found {
        d.protocol = "SIP/SDP";
        d.info.push_str(", with session description");
    }
    true
}

/// The fields of the head: the first line, then each header line by its name
fn head_fields(head: &[(String, Range<usize>)], response: bool) -> Vec<Field> {
    let mut fields = Vec::with_capacity(head.len());
    for (i, (line, range)) in head.iter().enumerate() {
        match line.split_once(':') {
            _ if i == 0 => fields.push(Field::new(
                if response {
                    "Status-Line"
                } else {
                    "Request-Line"
                },
                line.clone(),
                range.clone(),
            )),
            Some((name, value)) if !line.starts_with([' ', '\t']) && !name.trim().is_empty() => {
                fields.push(Field::new(
                    name.trim().to_owned(),
                    value.trim().to_owned(),
                    range.clone(),
                ));
            }
            // the continuation of the line before
            _ => fields.push(Field::new("Header", line.clone(), range.clone())),
        }
    }
    fields
}

/// The body from `whole.start` on, as long as Content-Length says (over TCP more messages may
/// follow); `true` if it is a session description
fn body(
    data: &[u8],
    head: &[(String, Range<usize>)],
    whole: Range<usize>,
    d: &mut Dissection,
) -> bool {
    let body = whole.start;
    let len = header(head, "Content-Length")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(whole.end - body);
    let end = body.saturating_add(len).min(whole.end);
    let kind = header(head, "Content-Type");
    let is_sdp = kind.map_or_else(
        || data[body..end].starts_with(b"v=0"),
        |k| {
            k.split(';')
                .next()
                .is_some_and(|m| m.trim().eq_ignore_ascii_case("application/sdp"))
        },
    );
    if end > body && is_sdp {
        sdp(data, body..end, d);
    } else if end > body {
        let mut body_fields = vec![Field::new("Length", (end - body).to_string(), body..end)];
        if let Some(kind) = kind {
            body_fields.push(Field::new("Content type", kind.to_owned(), body..body));
        }
        d.add(
            format!("Message body ({} bytes)", end - body),
            body..end,
            body_fields,
        );
    }
    if body.saturating_add(len) > whole.end {
        // the rest of the body was not captured, or follows in the next TCP segment
        d.cut(whole.end);
    }
    is_sdp && end > body
}

/// The names of the SDP lines (RFC 8866 5); `a` and `i` differ between session and media
fn sdp_name(kind: u8, media: bool) -> &'static str {
    match (kind, media) {
        (b'v', _) => "Version (v)",
        (b'o', _) => "Owner/Creator (o)",
        (b's', _) => "Session Name (s)",
        (b'i', false) => "Session Information (i)",
        (b'i', true) => "Media Title (i)",
        (b'u', _) => "URI (u)",
        (b'e', _) => "Email Address (e)",
        (b'p', _) => "Phone Number (p)",
        (b'c', _) => "Connection Information (c)",
        (b'b', _) => "Bandwidth Information (b)",
        (b't', _) => "Time Description (t)",
        (b'r', _) => "Repeat Time (r)",
        (b'z', _) => "Time Zone (z)",
        (b'k', _) => "Encryption Key (k)",
        (b'a', false) => "Session Attribute (a)",
        (b'a', true) => "Media Attribute (a)",
        (b'm', _) => "Media Description (m)",
        _ => "Line",
    }
}

/// The address of a `c=` line (`IN IP4 192.0.2.20`, multicast with `/ttl`)
fn connection(value: &str) -> Option<IpAddr> {
    let mut parts = value.split_whitespace();
    let (Some("IN"), Some(_), Some(addr)) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    addr.split('/').next()?.parse().ok()
}

/// The session description: a layer with its lines, and the media it offers in [`Dissection::sdp`]
fn sdp(data: &[u8], body: Range<usize>, d: &mut Dissection) {
    let mut fields = Vec::new();
    let mut session_addr = None;
    let mut media: Vec<SdpMedia> = Vec::new();
    let mut pos = body.start;
    while pos < body.end && fields.len() < 200 {
        let rest = &data[pos..body.end];
        let nl = rest.iter().position(|b| *b == b'\n');
        let mut end = nl.map_or(body.end, |n| pos + n);
        let next = nl.map_or(body.end, |n| pos + n + 1);
        if end > pos && data[end - 1] == b'\r' {
            end -= 1;
        }
        let line = utf8(&data[pos..end]);
        if let Some((kind, value)) = line
            .split_once('=')
            .filter(|(k, _)| k.len() == 1 && k.as_bytes()[0].is_ascii_lowercase())
        {
            let kind = kind.as_bytes()[0];
            fields.push(Field::new(
                sdp_name(kind, !media.is_empty()),
                value.to_owned(),
                pos..end,
            ));
            match kind {
                b'c' => {
                    let addr = connection(value);
                    match media.last_mut() {
                        Some(m) => m.addr = addr.or(m.addr),
                        None => session_addr = addr,
                    }
                }
                b'm' => {
                    if let Some(m) = media_line(value, session_addr) {
                        media.push(m);
                    } else {
                        // a media line that cannot be read: its attributes belong to no stream
                        media.push(SdpMedia {
                            port: 0,
                            ..SdpMedia::default()
                        });
                    }
                }
                b'a' => {
                    if let Some(m) = media.last_mut() {
                        attribute(value, m);
                    }
                }
                _ => {}
            }
        } else if !line.is_empty() {
            fields.push(Field::new("Line", line, pos..end));
        }
        pos = next;
    }
    d.has("sdp");
    d.add("Session Description Protocol".into(), body, fields);
    d.sdp.extend(
        media
            .into_iter()
            .filter(|m| m.port != 0 && m.addr.is_some()),
    );
}

/// A media line: `audio 7078 RTP/AVP 8 0 101`
fn media_line(value: &str, session_addr: Option<IpAddr>) -> Option<SdpMedia> {
    let mut parts = value.split_whitespace();
    let kind = parts.next()?.to_owned();
    let port: u16 = parts.next()?.split('/').next()?.parse().ok()?;
    let profile = parts.next()?;
    let formats = parts
        .filter_map(|f| f.parse::<u8>().ok())
        .filter(|f| *f < 128)
        .map(|f| (f, String::new()))
        .collect();
    Some(SdpMedia {
        kind,
        addr: session_addr,
        port,
        rtcp_port: port.checked_add(1).unwrap_or(port),
        srtp: profile.contains("SAVP"),
        formats,
    })
}

/// The attributes that matter for the media: `rtpmap`, `rtcp` and `rtcp-mux`
fn attribute(value: &str, m: &mut SdpMedia) {
    if let Some(map) = value.strip_prefix("rtpmap:") {
        let mut parts = map.split_whitespace();
        if let (Some(pt), Some(name)) = (
            parts.next().and_then(|p| p.parse::<u8>().ok()),
            parts.next(),
        ) && let Some(format) = m.formats.iter_mut().find(|(f, _)| *f == pt)
        {
            name.clone_into(&mut format.1);
        }
    } else if let Some(port) = value.strip_prefix("rtcp:") {
        if let Some(port) = port
            .split_whitespace()
            .next()
            .and_then(|p| p.parse::<u16>().ok())
        {
            m.rtcp_port = port;
        }
    } else if value == "rtcp-mux" {
        m.rtcp_port = m.port;
    }
}
