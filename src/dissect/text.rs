//! Text protocols: HTTP 1.x (RFC 9112) and SSDP (`UPnP` device discovery, HTTP over UDP)

use std::fmt::Write;
use std::ops::Range;

use super::{Dissection, Field, text};

const METHODS: [&[u8]; 9] = [
    b"GET ",
    b"POST ",
    b"PUT ",
    b"HEAD ",
    b"DELETE ",
    b"OPTIONS ",
    b"PATCH ",
    b"CONNECT ",
    b"TRACE ",
];

/// Most header lines shown per message
const MAX_HEADERS: usize = 100;

/// The bytes at `at` start an HTTP/1 request or response
pub(super) fn is_http(data: &[u8], at: usize) -> bool {
    let rest = data.get(at..).unwrap_or_default();
    rest.starts_with(b"HTTP/1.") || METHODS.iter().any(|m| rest.starts_with(m))
}

/// The lines of a message head: (line, range of the line without the line break)
fn lines(data: &[u8], payload: &Range<usize>) -> (Vec<(String, Range<usize>)>, Option<usize>) {
    let mut out = Vec::new();
    let mut pos = payload.start;
    while pos < payload.end && out.len() <= MAX_HEADERS {
        let Some(nl) = data[pos..payload.end].iter().position(|b| *b == b'\n') else {
            // the head continues in the next segment
            out.push((text(&data[pos..payload.end]), pos..payload.end));
            return (out, None);
        };
        let mut end = pos + nl;
        if end > pos && data[end - 1] == b'\r' {
            end -= 1;
        }
        if end == pos {
            return (out, Some(pos + nl + 1));
        }
        out.push((text(&data[pos..end]), pos..end));
        pos += nl + 1;
    }
    (out, None)
}

fn header<'a>(headers: &'a [(String, Range<usize>)], name: &str) -> Option<&'a str> {
    headers.iter().skip(1).find_map(|(line, _)| {
        let (n, v) = line.split_once(':')?;
        n.trim().eq_ignore_ascii_case(name).then_some(v.trim())
    })
}

/// HTTP/1.x request or response over TCP
pub(super) fn http(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let (head, body_start) = lines(data, &payload);
    let Some((start_line, start_range)) = head.first() else {
        return false;
    };
    let mut fields = vec![Field::new(
        "Start line",
        start_line.clone(),
        start_range.clone(),
    )];
    for (line, range) in head.iter().skip(1) {
        fields.push(Field::new("Header", line.clone(), range.clone()));
    }
    let head_end = body_start.unwrap_or(payload.end);
    let response = start_line.starts_with("HTTP/");
    d.protocol = "HTTP";
    d.has("http");
    let mut words = start_line.split(' ');
    match (words.next(), words.next()) {
        (Some(_), Some(code)) if response => {
            if let Ok(code) = code.parse::<u16>() {
                d.num("http.response.code", code);
            }
        }
        (Some(method), Some(uri)) => {
            d.text("http.request.method", method);
            d.text("http.request.uri", uri);
        }
        _ => {}
    }
    for (key, name) in [
        ("http.host", "Host"),
        ("http.user_agent", "User-Agent"),
        ("http.server", "Server"),
        ("http.content_type", "Content-Type"),
    ] {
        if let Some(value) = header(&head, name) {
            d.text(key, value);
        }
    }
    d.add(
        format!(
            "Hypertext Transfer Protocol ({})",
            if response { "response" } else { "request" }
        ),
        payload.start..head_end,
        fields,
    );
    if let Some(body) = body_start.filter(|b| *b < payload.end) {
        let len = payload.end - body;
        let mut body_fields = vec![Field::new("Length", len.to_string(), body..payload.end)];
        if let Some(kind) = header(&head, "Content-Type") {
            body_fields.push(Field::new("Content type", kind.to_owned(), body..body));
        }
        d.add(
            format!("Message body ({len} bytes)"),
            body..payload.end,
            body_fields,
        );
    }
    d.info.clone_from(start_line);
    if !response && let Some(host) = header(&head, "Host") {
        let _ = write!(d.info, " (Host: {host})");
    }
    true
}

/// SSDP over UDP 1900: M-SEARCH, NOTIFY or a search response
pub(super) fn ssdp(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    let (head, _) = lines(data, &payload);
    let Some((start_line, start_range)) = head.first() else {
        return false;
    };
    if !(start_line.starts_with("M-SEARCH ")
        || start_line.starts_with("NOTIFY ")
        || start_line.starts_with("HTTP/1."))
    {
        return false;
    }
    let mut fields = vec![Field::new(
        "Start line",
        start_line.clone(),
        start_range.clone(),
    )];
    for (line, range) in head.iter().skip(1) {
        fields.push(Field::new("Header", line.clone(), range.clone()));
    }
    d.protocol = "SSDP";
    d.has("ssdp");
    for (key, name) in [
        ("ssdp.nt", "NT"),
        ("ssdp.st", "ST"),
        ("ssdp.server", "SERVER"),
        ("ssdp.user_agent", "USER-AGENT"),
        ("ssdp.location", "LOCATION"),
    ] {
        if let Some(value) = header(&head, name) {
            d.text(key, value);
        }
    }
    d.add("Simple Service Discovery Protocol".into(), payload, fields);
    let mut info = start_line.clone();
    let target = header(&head, "NT").or_else(|| header(&head, "ST"));
    if let Some(t) = target {
        let _ = write!(info, " {t}");
    }
    if let Some(nts) = header(&head, "NTS") {
        let _ = write!(info, " ({nts})");
    }
    if let Some(agent) = header(&head, "SERVER").or_else(|| header(&head, "USER-AGENT")) {
        let _ = write!(info, " – {agent}");
    }
    d.info = info;
    true
}
