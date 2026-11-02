//! What must not leave the window with an answer. Everything an assistant receives goes to its AI
//! service, and much of it was written by other devices: a host name can hold an instruction, and
//! a request can hold a password.
//!
//! The fields of the dissectors are texts ([`hide_header`], [`hide_query`]); the content of packets
//! is bytes, so [`Masker`] overwrites the same secrets there, byte for byte, before they become
//! text or a hex dump.

use serde_json::Value;

use crate::dissect::{self, Dissection};

/// Longest text in an answer
const MAX_CHARS: usize = 500;
const HIDDEN: &str = "[hidden]";

/// Parts of a header name that mark a value as a secret
const SECRET_HEADERS: [&str; 8] = [
    "cookie", "auth", "token", "secret", "password", "session", "api-key", "apikey",
];
/// Filter fields whose values hold a URL, and with it a query the answers hide
const URL_FIELDS: [&str; 2] = ["http.request.uri", "ssdp.location"];
/// What takes the place of a hidden byte
const MASK: u8 = b'x';
/// Longest first word or header name a line is looked at for
const MAX_NAME: usize = 64;
/// First words of a line that carries a URL: HTTP and its relatives SSDP and GENA
const URL_COMMANDS: [&[u8]; 13] = [
    b"GET",
    b"POST",
    b"PUT",
    b"HEAD",
    b"DELETE",
    b"OPTIONS",
    b"PATCH",
    b"CONNECT",
    b"TRACE",
    b"NOTIFY",
    b"M-SEARCH",
    b"SUBSCRIBE",
    b"UNSUBSCRIBE",
];
/// First words after which the rest of the line is a password: FTP, POP3, SMTP
const SECRET_COMMANDS: [&[u8]; 2] = [b"PASS", b"AUTH"];

/// Characters a person does not see but a language model reads: format characters that hide or
/// turn around text, and the tag characters that spell out whole sentences
fn is_invisible(c: char) -> bool {
    matches!(c as u32,
        0xAD | 0x34F | 0x61C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x180B..=0x180F
        | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0x3164 | 0xFE00..=0xFE0F
        | 0xFEFF | 0xFFA0 | 0xFFF0..=0xFFFB | 0x1D173..=0x1D17A | 0xE0000..=0xE0FFF)
}

/// A text that is safe to hand on: on one line, without what cannot be seen, of a limited length
pub fn clean_text(text: &str) -> String {
    let mut clean = String::with_capacity(text.len().min(MAX_CHARS));
    let mut chars = text.chars().filter(|c| !is_invisible(*c));
    for c in chars.by_ref().take(MAX_CHARS) {
        clean.push(if c.is_control() { ' ' } else { c });
    }
    if chars.next().is_some() {
        clean.push('…');
    }
    clean
}

/// Cleans all texts of an answer
pub fn clean(value: &mut Value) {
    match value {
        Value::String(text) => *text = clean_text(text),
        Value::Array(items) => items.iter_mut().for_each(clean),
        Value::Object(fields) => fields.values_mut().for_each(clean),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// `GET /login?user=me&pass=1 HTTP/1.1` without its query, and the same for a URL in a header
pub fn hide_query(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find('?') {
        out.push_str(&rest[..=at]);
        out.push_str(HIDDEN);
        rest = &rest[at..];
        rest = rest
            .find(char::is_whitespace)
            .map_or("", |end| &rest[end..]);
    }
    out.push_str(rest);
    out
}

/// A line of an HTTP header without secrets: `Cookie: [hidden]`
pub fn hide_header(line: &str) -> String {
    // the continuation of the line before, whatever that was
    if line.starts_with([' ', '\t']) {
        return HIDDEN.to_owned();
    }
    let Some((name, _)) = line.split_once(':') else {
        return hide_query(line);
    };
    if is_secret_name(name) {
        format!("{name}: {HIDDEN}")
    } else {
        hide_query(line)
    }
}

/// The values of a packet as the filter of an assistant sees them: without what the answers hide,
/// so that trying filters cannot find it out either
pub fn hide_in_values(d: &mut Dissection) {
    for (key, value) in &mut d.values {
        if URL_FIELDS.contains(key)
            && let dissect::Value::Text(text) = value
        {
            *text = hide_query(text);
        }
    }
}

fn is_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_HEADERS.iter().any(|secret| lower.contains(secret))
}

/// Where in a line the [`Masker`] is
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Line {
    Start,
    /// The first word, or the name of a header
    Name,
    /// The value of a secret, up to the end of the line
    Secret,
    /// A line that can hold a URL, whose queries are hidden
    Url,
    Query,
    /// Nothing to hide
    Plain,
}

/// Hides the secrets of text protocols in bytes as they went over the wire, keeping their
/// length: the values of headers such as `Cookie` and `Authorization`, the queries of URLs in
/// requests and headers, and what follows `PASS` and `AUTH`. It goes through a direction of a
/// connection piece by piece and remembers where a line began, so a secret cut in two by the
/// packets stays hidden.
#[derive(Clone, Debug)]
pub struct Masker {
    line: Line,
    name: Vec<u8>,
    /// The line before was a secret, so its continuation is too
    secret_before: bool,
}

impl Masker {
    /// At the start of a line
    pub fn new() -> Self {
        Self {
            line: Line::Start,
            name: Vec::new(),
            secret_before: false,
        }
    }

    /// After bytes that were not seen: the rest of the line could be anything, so it is hidden
    pub fn unknown() -> Self {
        Self {
            line: Line::Secret,
            name: Vec::new(),
            secret_before: true,
        }
    }

    pub fn mask(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            self.next(byte);
        }
    }

    fn next(&mut self, byte: &mut u8) {
        match *byte {
            b'\n' => {
                self.secret_before = self.line == Line::Secret;
                self.line = Line::Start;
                self.name.clear();
                return;
            }
            b'\r' => return,
            _ => {}
        }
        if self.line == Line::Start {
            self.line = if *byte == b' ' || *byte == b'\t' {
                if self.secret_before {
                    Line::Secret
                } else {
                    Line::Url
                }
            } else {
                Line::Name
            };
        }
        match self.line {
            Line::Name => self.name_byte(*byte),
            Line::Secret => {
                if *byte != b' ' && *byte != b'\t' {
                    *byte = MASK;
                }
            }
            Line::Url => {
                if *byte == b'?' {
                    self.line = Line::Query;
                }
            }
            Line::Query => {
                if *byte == b' ' || *byte == b'\t' {
                    self.line = Line::Url;
                } else {
                    *byte = MASK;
                }
            }
            Line::Start | Line::Plain => {}
        }
    }

    /// A byte of the first word: a colon ends the name of a header, a space a command
    fn name_byte(&mut self, byte: u8) {
        match byte {
            b':' => {
                let name = String::from_utf8_lossy(&self.name);
                self.line = if self.name.is_empty() {
                    Line::Plain
                } else if is_secret_name(&name) {
                    Line::Secret
                } else {
                    Line::Url
                };
            }
            b' ' | b'\t' => {
                let word = self.name.as_slice();
                self.line = if URL_COMMANDS.contains(&word) {
                    Line::Url
                } else if SECRET_COMMANDS.iter().any(|c| c.eq_ignore_ascii_case(word)) {
                    Line::Secret
                } else {
                    Line::Plain
                };
            }
            b if (b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && self.name.len() < MAX_NAME =>
            {
                self.name.push(b);
            }
            _ => self.line = Line::Plain,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn what_cannot_be_seen_does_not_reach_the_model() {
        assert_eq!(clean_text("a\u{202E}b\u{200B}c\u{FEFF}d"), "abcd");
        // "ignore" in tag characters, invisible on the screen
        let hidden: String = "ignore"
            .chars()
            .map(|c| char::from_u32(0xE0000 + c as u32).unwrap())
            .collect();
        assert_eq!(clean_text(&format!("host{hidden}.example")), "host.example");
    }

    #[test]
    fn a_text_stays_on_its_line_and_within_its_length() {
        assert_eq!(
            clean_text("one\r\ntwo\tthree\u{1b}[0m"),
            "one  two three [0m"
        );
        let long = clean_text(&"x".repeat(MAX_CHARS + 1));
        assert_eq!(long.chars().count(), MAX_CHARS + 1);
        assert!(long.ends_with('…'));
        assert_eq!(
            clean_text(&"x".repeat(MAX_CHARS)).chars().count(),
            MAX_CHARS
        );
    }

    #[test]
    fn every_text_of_an_answer_is_cleaned() {
        let mut answer = json!({ "a": ["x\ny", { "b": "c\u{200B}d" }], "n": 1, "t": true });
        clean(&mut answer);
        assert_eq!(
            answer,
            json!({ "a": ["x y", { "b": "cd" }], "n": 1, "t": true })
        );
    }

    #[test]
    fn a_query_is_hidden() {
        assert_eq!(
            hide_query("GET /login?user=me&pass=1 HTTP/1.1"),
            "GET /login?[hidden] HTTP/1.1"
        );
        assert_eq!(hide_query("GET /a?b=c"), "GET /a?[hidden]");
        assert_eq!(
            hide_query("GET /a?b HTTP/1.1 /c?d"),
            "GET /a?[hidden] HTTP/1.1 /c?[hidden]"
        );
        assert_eq!(
            hide_query("GET /index.html HTTP/1.1"),
            "GET /index.html HTTP/1.1"
        );
    }

    fn masked(pieces: &[&str]) -> String {
        let mut masker = Masker::new();
        let mut out = Vec::new();
        for piece in pieces {
            let mut bytes = piece.as_bytes().to_vec();
            masker.mask(&mut bytes);
            assert_eq!(bytes.len(), piece.len());
            out.extend(bytes);
        }
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn the_secrets_of_a_request_are_overwritten_in_its_bytes() {
        let request = "GET /login?user=me&pass=1 HTTP/1.1\r\nHost: example.com\r\n\
            Cookie: id=1; b=2\r\nAuthorization: Basic dXNl\r\n  cjpw\r\n\
            Referer: https://example.com/a?id=1\r\nX-Api-Key: 123\r\n\r\nbody?a=b";
        assert_eq!(
            masked(&[request]),
            "GET /login?xxxxxxxxxxxxxx HTTP/1.1\r\nHost: example.com\r\n\
            Cookie: xxxxx xxx\r\nAuthorization: xxxxx xxxx\r\n  xxxx\r\n\
            Referer: https://example.com/a?xxxx\r\nX-Api-Key: xxx\r\n\r\nbody?a=b"
        );
        let response = "HTTP/1.1 302 Found\r\nSet-Cookie: id=1\r\nLocation: /n?c=2\r\n\r\n";
        assert_eq!(
            masked(&[response]),
            "HTTP/1.1 302 Found\r\nSet-Cookie: xxxx\r\nLocation: /n?xxx\r\n\r\n"
        );
        assert_eq!(
            masked(&["USER me\r\nPASS hunter2\r\nAUTH PLAIN AGZvbwBiYXI=\r\n"]),
            "USER me\r\nPASS xxxxxxx\r\nAUTH xxxxx xxxxxxxxxxxx\r\n"
        );
    }

    #[test]
    fn a_secret_cut_by_the_packets_stays_hidden() {
        assert_eq!(
            masked(&["GET / HTTP/1.1\r\nCoo", "kie: id=SEC", "RET\r\nHost: a\r\n"]),
            "GET / HTTP/1.1\r\nCookie: xxxxxxxxx\r\nHost: a\r\n"
        );
        assert_eq!(masked(&["GET /a?b", "=c HTTP/1.1"]), "GET /a?xxx HTTP/1.1");
    }

    #[test]
    fn after_unseen_bytes_the_rest_of_the_line_is_hidden() {
        let mut bytes = b"id=SECRET\r\nHost: a\r\n".to_vec();
        Masker::unknown().mask(&mut bytes);
        assert_eq!(bytes, b"xxxxxxxxx\r\nHost: a\r\n");
    }

    #[test]
    fn other_content_stays_as_it_is() {
        for text in [
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<p>Hello? Yes.</p>",
            "{\"a\": \"b?c\"}",
            "SSH-2.0-OpenSSH_9.6\r\n",
        ] {
            assert_eq!(masked(&[text]), text);
        }
        let binary: Vec<u8> = (0..=255).collect();
        let mut copy = binary.clone();
        Masker::new().mask(&mut copy);
        assert_eq!(copy, binary);
    }

    #[test]
    fn a_secret_in_a_header_is_hidden() {
        for line in [
            "Cookie: id=1",
            "Set-Cookie: id=1; Path=/",
            "Authorization: Basic dXNlcjpwYXNz",
            "proxy-authorization: Bearer x",
            "X-Api-Key: 123",
            "X-Auth-Token: 123",
            "X-Session-Id: 123",
        ] {
            let name = line.split(':').next().unwrap();
            assert_eq!(hide_header(line), format!("{name}: [hidden]"));
        }
        assert_eq!(hide_header("Host: example.com"), "Host: example.com");
        assert_eq!(hide_header(" id=1; other=2"), "[hidden]");
        assert_eq!(
            hide_header("Referer: https://example.com/a?id=1"),
            "Referer: https://example.com/a?[hidden]"
        );
    }

    /// Random texts of header lines, URLs, commands and bytes, masked whole and in random pieces:
    /// the pieces change nothing, the length stays, and only `x` takes the place of a byte
    #[test]
    fn fuzz_masker_with_random_pieces() {
        use rand::Rng;

        const PARTS: [&[u8]; 22] = [
            b"GET /a?b=c HTTP/1.1",
            b"HTTP/1.1 200 OK",
            b"NOTIFY * HTTP/1.1",
            b"Cookie: id=1",
            b"Set-Cookie: s=2",
            b"Authorization: Basic abc",
            b"X-Api-Key: k",
            b"Location: /n?code=3",
            b"Host: example.org",
            b"PASS hunter2",
            b"AUTH PLAIN abc",
            b"\r\n",
            b"\n",
            b"\r",
            b" ",
            b"\t",
            b":",
            b"?",
            b"=",
            b"x-token",
            b"\xff\x00",
            b"user",
        ];
        let mut rng = crate::fuzz::rng();
        for _ in 0..crate::fuzz::iterations(20_000) {
            let mut text = Vec::new();
            for _ in 0..rng.random_range(0..40) {
                if rng.random_ratio(1, 6) {
                    text.push(rng.random());
                } else {
                    text.extend_from_slice(PARTS[rng.random_range(0..PARTS.len())]);
                }
            }
            let start = if rng.random() {
                Masker::new()
            } else {
                Masker::unknown()
            };
            let mut whole = text.clone();
            start.clone().mask(&mut whole);
            let mut pieces = text.clone();
            let mut masker = start;
            let mut rest = &mut pieces[..];
            while !rest.is_empty() {
                let n = rng.random_range(0..=rest.len());
                let (piece, after) = rest.split_at_mut(n);
                masker.mask(piece);
                rest = after;
            }
            assert_eq!(whole, pieces, "{:?}", String::from_utf8_lossy(&text));
            assert_eq!(whole.len(), text.len());
            for (before, after) in text.iter().zip(&whole) {
                assert!(
                    before == after || (*after == b'x' && !b"\r\n".contains(before)),
                    "{:?}",
                    String::from_utf8_lossy(&text)
                );
            }
        }
    }
}
