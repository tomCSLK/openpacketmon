//! What must not leave the window with an answer. Everything an assistant receives goes to its AI
//! service, and much of it was written by other devices: a host name can hold an instruction, and
//! a request can hold a password.
//!
//! The fields of the dissectors are texts ([`hide_header`], [`hide_query`]); the content of packets
//! is bytes, so [`Masker`] overwrites the same secrets there, byte for byte, before they become
//! text or a hex dump.
//!
//! Telephone numbers keep only their last two digits (`sip:+xxxxxxxxxx67@sip.example.net`), in
//! texts ([`hide_numbers`], [`hide_address`]) as in bytes; display names and domains stay.

use serde_json::Value;

use crate::dissect::{self, Dissection, rtp};

/// Longest text in an answer
const MAX_CHARS: usize = 500;
pub(crate) const HIDDEN: &str = "[hidden]";

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
/// Header names whose whole value is a secret: the signed `Identity` of SIP (RFC 8224, short `y`)
/// carries the numbers of both sides
const SECRET_NAMES: [&str; 2] = ["identity", "y"];
/// Attributes of a session description that hold the key of SRTP, with which the voice could be
/// decrypted
const KEY_ATTRIBUTES: [&str; 2] = ["crypto", "key-mgmt"];
/// SIP headers with an address and a display name (`From: "Anna" <sip:…>`), with their short forms
const ADDRESS_HEADERS: [&str; 17] = [
    "from",
    "f",
    "to",
    "t",
    "contact",
    "m",
    "p-asserted-identity",
    "p-preferred-identity",
    "p-called-party-id",
    "remote-party-id",
    "diversion",
    "history-info",
    "referred-by",
    "b",
    "refer-to",
    "r",
    "reply-to",
];

/// Characters a person does not see but a language model reads: format characters that hide or
/// turn around text, and the tag characters that spell out whole sentences
fn is_invisible(c: char) -> bool {
    matches!(c as u32,
        0xAD | 0x34F | 0x61C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x180B..=0x180F
        | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0x3164 | 0xFE00..=0xFE0F
        | 0xFEFF | 0xFFA0 | 0xFFF0..=0xFFFB | 0x1D173..=0x1D17A | 0xE0000..=0xE0FFF)
}

/// A text that is safe to hand on: on one line, without what cannot be seen, of a limited length,
/// and with telephone numbers cut short
pub fn clean_text(text: &str) -> String {
    let text = hide_numbers(text);
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
        let dissect::Value::Text(text) = value else {
            continue;
        };
        if URL_FIELDS.contains(key) {
            *text = hide_query(text);
        } else if *key == "sip.from" || *key == "sip.to" {
            *text = hide_address(text);
        }
    }
}

pub(crate) fn is_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_HEADERS.iter().any(|secret| lower.contains(secret))
        || SECRET_NAMES.contains(&lower.as_str())
}

/// A header of SIP whose value is an address with a display name
pub(crate) fn is_address_header(name: &str) -> bool {
    ADDRESS_HEADERS.contains(&name.to_ascii_lowercase().as_str())
}

/// An attribute of a session description (`crypto:1 AES_CM_128_HMAC_SHA1_80 inline:…`) without
/// the key of SRTP in it
pub(crate) fn hide_key_attribute(value: &str) -> String {
    match value.split_once(':') {
        Some((name, _))
            if KEY_ATTRIBUTES
                .iter()
                .any(|key| key.eq_ignore_ascii_case(name.trim())) =>
        {
            format!("{name}:{HIDDEN}")
        }
        _ => value.to_owned(),
    }
}

/// A text with its telephone numbers cut short to their last two digits: in SIP and `tel:` URIs,
/// in the display names of the SIP headers that carry an address and in the `p=` line of a session
/// description
pub fn hide_numbers(text: &str) -> String {
    Numbers::at(Place::Name).text(text)
}

/// An address as SIP writes it (`"Anna" <sip:anna@example.net>`, `+4930123456@example.net`, a
/// display name alone), with its numbers cut short: a display name that is a number too, the user
/// part before `@` with or without its scheme. An IP address stays as it is.
pub fn hide_address(text: &str) -> String {
    let plain = text.trim();
    if plain.parse::<std::net::IpAddr>().is_ok() || plain.parse::<std::net::SocketAddr>().is_ok() {
        return text.to_owned();
    }
    Numbers::at(Place::Display).text(text)
}

/// A telephone number of a session description (`p=+49 30 1234567`), cut short
pub(crate) fn hide_phone(text: &str) -> String {
    Numbers::at(Place::Phone).text(text)
}

/// The UDP payload of an RTP packet without the voice: its header stays, the rest becomes `x`
pub fn hide_voice(payload: &mut [u8]) {
    let start = rtp::rtp_header(payload).map_or(payload.len().min(12), |h| h.payload.start);
    payload[start..].fill(MASK);
}

/// The UDP payload of an RTCP packet with the numbers in its source descriptions cut short
/// (PHONE, and NAME, CNAME and EMAIL as addresses)
pub fn hide_in_reports(payload: &mut [u8]) {
    for (kind, range) in rtp::sdes_items(payload) {
        let place = match kind {
            1..=3 => Place::Display,
            4 => Place::Phone,
            _ => continue,
        };
        let value = &mut payload[range];
        let mut numbers = Numbers::at(place);
        for i in 0..value.len() {
            numbers.step(value, i, true);
        }
    }
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
    /// The name of an attribute of a session description (`a=crypto:…`)
    Attribute,
    /// Nothing to hide
    Plain,
}

/// Hides the secrets of text protocols in bytes as they went over the wire, keeping their
/// length: the values of headers such as `Cookie` and `Authorization`, the queries of URLs in
/// requests and headers, what follows `PASS` and `AUTH`, the keys of SRTP in a session
/// description, and all but the last two digits of telephone numbers (see [`hide_numbers`]). It
/// goes through a direction of a connection piece by piece and remembers where a line began, so a
/// secret cut in two by the packets stays hidden; a number cut in two may lose its last two digits
/// as well.
#[derive(Clone, Debug)]
pub struct Masker {
    line: Line,
    name: Vec<u8>,
    /// The line before was a secret, so its continuation is too
    secret_before: bool,
    numbers: Numbers,
}

impl Masker {
    /// At the start of a line
    pub fn new() -> Self {
        Self {
            line: Line::Start,
            name: Vec::new(),
            secret_before: false,
            numbers: Numbers::at(Place::Name),
        }
    }

    /// After bytes that were not seen: the rest of the line could be anything, so it is hidden
    pub fn unknown() -> Self {
        Self {
            line: Line::Secret,
            name: Vec::new(),
            secret_before: true,
            numbers: Numbers::at(Place::Other),
        }
    }

    pub fn mask(&mut self, bytes: &mut [u8]) {
        for i in 0..bytes.len() {
            self.next(&mut bytes[i]);
            self.numbers.step(bytes, i, false);
        }
        // the end of a number may follow in the next piece
        self.numbers.cut(bytes);
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
            Line::Attribute => match *byte {
                b':' => {
                    let name = String::from_utf8_lossy(&self.name);
                    self.line = if KEY_ATTRIBUTES.iter().any(|k| k.eq_ignore_ascii_case(&name)) {
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
            },
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
            // the lines of a session description: `k=` is a key, `a=` an attribute
            b'=' if self.name.as_slice() == b"k" => self.line = Line::Secret,
            b'=' if self.name.as_slice() == b"a" => {
                self.name.clear();
                self.line = Line::Attribute;
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

/// What a byte belongs to, for [`Numbers`]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Place {
    /// The first word of a line: the name of a header, or the `p` of `p=`
    Name,
    /// Nothing that holds a number, unless a URI begins
    Other,
    /// The display name of an address header, up to the address
    Display,
    /// The user part of a SIP URI, up to its `@`
    User,
    /// The password of a SIP URI (`sip:alice:secret@…`), hidden whole
    Password,
    /// A `tel:` URI; its parameters (`;ext=12`) are numbers of their own
    Tel,
    /// The value of a `p=` line of a session description
    Phone,
}

/// Where a URI after `sip:` goes: to a user part with its `@`, or right to the host
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ahead {
    User,
    Host,
    /// The bytes at hand end before it shows
    Unknown,
}

/// Longest first word of a line looked at for an address header
const MAX_HEADER: usize = 32;

/// Cuts telephone numbers short to their last two digits, byte for byte. A number is a run of
/// digits: the user part of a SIP URI, a `tel:` URI, a display name or the `p=` line of a session
/// description. Since the end of a number is known only after it, the last two digits of a run
/// are held back; if the bytes at hand end within the run, they are hidden too.
#[derive(Clone, Debug)]
struct Numbers {
    place: Place,
    /// The first word of the line, lowercase, while [`Place::Name`]
    name: Vec<u8>,
    /// The last bytes, lowercase, for the scheme before a `:`
    tail: [u8; 5],
    /// Where in the bytes at hand the last two digits of the run are
    held: [Option<usize>; 2],
}

impl Numbers {
    fn at(place: Place) -> Self {
        Self {
            place,
            name: Vec::new(),
            tail: [b'\n'; 5],
            held: [None; 2],
        }
    }

    /// A text, read whole: its end is the end of every number in it
    fn text(mut self, text: &str) -> String {
        let mut bytes = text.as_bytes().to_vec();
        for i in 0..bytes.len() {
            self.step(&mut bytes, i, true);
        }
        // only ASCII digits and bytes of ASCII became `x`
        String::from_utf8(bytes).unwrap_or_default()
    }

    /// The bytes at hand end within a number: its last digits may not be the last
    fn cut(&mut self, bytes: &mut [u8]) {
        for at in self.held.iter_mut().filter_map(Option::take) {
            bytes[at] = MASK;
        }
    }

    /// The run of a number ends: its last two digits stay
    fn end_run(&mut self) {
        self.held = [None; 2];
    }

    /// A digit of a run: the third last is hidden now
    fn digit(&mut self, bytes: &mut [u8], i: usize) {
        self.held = match self.held {
            [Some(old), Some(last)] => {
                bytes[old] = MASK;
                [Some(last), Some(i)]
            }
            [Some(first), None] => [Some(first), Some(i)],
            _ => [Some(i), None],
        };
    }

    /// Ends the URI of a SIP or tel address
    fn is_end(byte: u8) -> bool {
        matches!(byte, b' ' | b'\t' | b'<' | b'>' | b'"' | b',') || !(0x21..0x7f).contains(&byte)
    }

    /// Belongs to a written number: digits and what is put between them
    fn is_number_char(byte: u8) -> bool {
        byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'(' | b')' | b'/' | b' ')
    }

    /// The scheme that ends with the last byte, before a `:`
    fn scheme(&self) -> Option<Place> {
        let t = &self.tail;
        if &t[1..] == b"sips" && !t[0].is_ascii_alphanumeric()
            || &t[2..] == b"sip" && !t[1].is_ascii_alphanumeric()
        {
            Some(Place::User)
        } else if &t[2..] == b"tel" && !t[1].is_ascii_alphanumeric() {
            Some(Place::Tel)
        } else {
            None
        }
    }

    /// Whether the URI from `from` on has a user part; a text read `whole` has nothing after it
    fn ahead(bytes: &[u8], from: usize, whole: bool) -> Ahead {
        let rest = bytes.get(from..).unwrap_or_default();
        match rest.iter().position(|b| *b == b'@' || Self::is_end(*b)) {
            Some(at) if rest[at] == b'@' => Ahead::User,
            // a global number without a host is still a number
            _ if rest.first() == Some(&b'+') => Ahead::User,
            Some(_) => Ahead::Host,
            None if whole => Ahead::Host,
            None => Ahead::Unknown,
        }
    }

    fn step(&mut self, bytes: &mut [u8], i: usize, whole: bool) {
        let byte = bytes[i];
        let scheme = if byte == b':' { self.scheme() } else { None };
        self.tail.rotate_left(1);
        self.tail[4] = byte.to_ascii_lowercase();
        if byte == b'\n' || byte == b'\r' {
            self.end_run();
            self.place = if byte == b'\n' {
                Place::Name
            } else {
                Place::Other
            };
            self.name.clear();
            return;
        }
        if let Some(place) = scheme
            && matches!(self.place, Place::Name | Place::Other | Place::Display)
        {
            self.end_run();
            self.place = match (place, Self::ahead(bytes, i + 1, whole)) {
                (Place::User, Ahead::Host) => Place::Other,
                _ => place,
            };
            return;
        }
        match self.place {
            Place::Name => match byte {
                b':' => {
                    let name = String::from_utf8_lossy(&self.name);
                    self.place = if is_address_header(&name) {
                        Place::Display
                    } else {
                        Place::Other
                    };
                }
                b'=' => {
                    self.place = if self.name == b"p" {
                        Place::Phone
                    } else {
                        Place::Other
                    };
                }
                b if (b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                    && self.name.len() < MAX_HEADER =>
                {
                    self.name.push(b.to_ascii_lowercase());
                }
                _ => self.place = Place::Other,
            },
            Place::Other => {}
            Place::Display => match byte {
                // the address follows, or a host
                b'<' | b'@' => {
                    self.end_run();
                    self.place = Place::Other;
                }
                _ => self.run(bytes, i),
            },
            Place::Phone => self.run(bytes, i),
            Place::User => match byte {
                b'@' => {
                    self.end_run();
                    self.place = Place::Other;
                }
                b':' => {
                    self.end_run();
                    self.place = Place::Password;
                }
                b if Self::is_end(b) => {
                    self.end_run();
                    self.place = Place::Other;
                }
                b if b.is_ascii_digit() => self.digit(bytes, i),
                _ => {}
            },
            Place::Password => match byte {
                b'@' => self.place = Place::Other,
                b if Self::is_end(b) => self.place = Place::Other,
                _ => bytes[i] = MASK,
            },
            Place::Tel => match byte {
                b';' => self.end_run(),
                b if Self::is_end(b) => {
                    self.end_run();
                    self.place = Place::Other;
                }
                b if b.is_ascii_digit() => self.digit(bytes, i),
                _ => {}
            },
        }
    }

    /// A byte of a display name or a `p=` line: a number is digits and what stands between them
    fn run(&mut self, bytes: &mut [u8], i: usize) {
        let byte = bytes[i];
        if byte.is_ascii_digit() {
            self.digit(bytes, i);
        } else if !Self::is_number_char(byte) {
            self.end_run();
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

    #[test]
    fn a_number_keeps_its_last_two_digits() {
        for (text, hidden) in [
            (
                "INVITE sip:+4930123456789@sip.example.net SIP/2.0",
                "INVITE sip:+xxxxxxxxxxx89@sip.example.net SIP/2.0",
            ),
            (
                "To: <tel:+49-30-1234567;ext=1234>",
                "To: <tel:+xx-xx-xxxxx67;ext=xx34>",
            ),
            (
                "From: \"+49 30 1234567\" <sips:0301234567@192.0.2.1:5061>;tag=12345",
                "From: \"+xx xx xxxxx67\" <sips:xxxxxxxx67@192.0.2.1:5061>;tag=12345",
            ),
            (
                "f: 030 1234567 <sip:anna@example.net>",
                "f: xxx xxxxx67 <sip:anna@example.net>",
            ),
            (
                "P-Asserted-Identity: \"Anna\" <sip:4930123@pbx>",
                "P-Asserted-Identity: \"Anna\" <sip:xxxxx23@pbx>",
            ),
            ("p=+49 30 1234567 (Anna)", "p=+xx xx xxxxx67 (Anna)"),
            (
                "Contact: <sip:alice:secret1@host>",
                "Contact: <sip:alice:xxxxxxx@host>",
            ),
            // a global number without a host is a number too
            (
                "Request: INVITE sip:+4930123",
                "Request: INVITE sip:+xxxxx23",
            ),
        ] {
            assert_eq!(hide_numbers(text), hidden, "{text}");
        }
    }

    #[test]
    fn names_hosts_and_other_digits_stay() {
        for text in [
            "INVITE sip:anna@sip.example.net SIP/2.0",
            "From: \"Anna Beispiel\" <sip:anna@example.net>;tag=1928301774",
            "Via: SIP/2.0/UDP 192.168.1.20:5060;branch=z9hG4bK776asdhds",
            "Contact: <sip:192.168.1.20:5060;transport=udp>;expires=3600",
            "Route: <sip:pbx1.example.net;lr>",
            "Call-ID: 3848276298220188511@atlanta.example.com",
            "Status: 200 OK (INVITE)",
            "o=- 1234567890 1234567890 IN IP4 192.0.2.10",
            "Zimmer 12",
            "gossip:12345",
        ] {
            assert_eq!(hide_numbers(text), text);
        }
    }

    #[test]
    fn an_address_is_cut_short_with_or_without_its_scheme() {
        for (text, hidden) in [
            ("Anna Beispiel", "Anna Beispiel"),
            ("+4930123456@sip.example.net", "+xxxxxxxx56@sip.example.net"),
            ("4930123@192.168.1.1", "xxxxx23@192.168.1.1"),
            ("+49 30 1234567", "+xx xx xxxxx67"),
            (
                "\"+49 30 1234567\" <sip:+49301234567@h>",
                "\"+xx xx xxxxx67\" <sip:+xxxxxxxxx67@h>",
            ),
            ("<tel:+49301234567>", "<tel:+xxxxxxxxx67>"),
            ("anonymous@anonymous.invalid", "anonymous@anonymous.invalid"),
            ("192.168.1.20", "192.168.1.20"),
            ("192.168.1.20:40000", "192.168.1.20:40000"),
        ] {
            assert_eq!(hide_address(text), hidden, "{text}");
        }
        assert_eq!(hide_phone("+49 30 1234567"), "+xx xx xxxxx67");
    }

    #[test]
    fn the_numbers_of_a_sip_message_are_cut_short_in_its_bytes() {
        let message = "INVITE sip:+4930123456@sip.example.net SIP/2.0\r\n\
            From: \"+49 40 7654321\" <sip:+49407654321@sip.example.net>;tag=1\r\n\
            t: <tel:+4930123456>\r\n\
            Authorization: Digest username=\"4940765\", response=\"abc\"\r\n\
            Identity: eyJhbGciOiJFUzI1NiJ9.eyJvcmlnIjp7InRuIjoiMSJ9fQ\r\n\
            Content-Type: application/sdp\r\n\r\n\
            v=0\r\np=+49 40 7654321\r\nk=clear:secret\r\n\
            a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:WVNfX19zZW1jdGwgKCkgewkyMjA7\r\n\
            a=rtpmap:8 PCMA/8000\r\n";
        let hidden = "INVITE sip:+xxxxxxxx56@sip.example.net SIP/2.0\r\n\
            From: \"+xx xx xxxxx21\" <sip:+xxxxxxxxx21@sip.example.net>;tag=1\r\n\
            t: <tel:+xxxxxxxx56>\r\n\
            Authorization: xxxxxx xxxxxxxxxxxxxxxxxxx xxxxxxxxxxxxxx\r\n\
            Identity: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\r\n\
            Content-Type: application/sdp\r\n\r\n\
            v=0\r\np=+xx xx xxxxx21\r\nk=xxxxxxxxxxxx\r\n\
            a=crypto:x xxxxxxxxxxxxxxxxxxxxxxx xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\r\n\
            a=rtpmap:8 PCMA/8000\r\n";
        assert_eq!(masked(&[message]), hidden);
        // cut anywhere, a number loses no more than its last two digits
        for cut in 1..message.len() {
            let pieces = masked(&[&message[..cut], &message[cut..]]);
            for (i, (whole, piece)) in hidden.bytes().zip(pieces.bytes()).enumerate() {
                assert!(
                    whole == piece || (piece == MASK && message.as_bytes()[i].is_ascii_digit()),
                    "{cut}: {pieces}"
                );
            }
        }
    }

    #[test]
    fn a_key_of_srtp_is_hidden_in_an_attribute() {
        assert_eq!(
            hide_key_attribute("crypto:1 AES_CM_128_HMAC_SHA1_80 inline:WVNf"),
            "crypto:[hidden]"
        );
        assert_eq!(
            hide_key_attribute("rtpmap:8 PCMA/8000"),
            "rtpmap:8 PCMA/8000"
        );
    }

    #[test]
    fn the_voice_of_rtp_and_the_numbers_of_rtcp_stay_inside() {
        let mut rtp = crate::dissect::tests::rtp_bytes(8, 1, 160, 7, false, 160);
        hide_voice(&mut rtp);
        assert_eq!(
            rtp[..12],
            crate::dissect::tests::rtp_bytes(8, 1, 160, 7, false, 0)[..]
        );
        assert!(rtp[12..].iter().all(|b| *b == MASK));
        let mut report = crate::dissect::tests::rtcp_report(7, 9, "4930123456@192.0.2.20");
        let mut phone = 7_u32.to_be_bytes().to_vec();
        phone.extend([4, 14]);
        phone.extend_from_slice(b"+49 30 1234567");
        // the end of the items, up to a full word
        phone.extend([0, 0, 0, 0]);
        report.extend([0x81, 202, 0, 6]);
        report.extend(phone);
        hide_in_reports(&mut report);
        let text = String::from_utf8_lossy(&report);
        assert!(text.contains("xxxxxxxx56@192.0.2.20"), "{text}");
        assert!(text.contains("+xx xx xxxxx67"), "{text}");
    }

    /// Random texts of header lines, URLs, commands, SIP and bytes, masked whole and in random
    /// pieces: the pieces hide no less, the length stays, and only `x` takes the place of a byte
    #[test]
    fn fuzz_masker_with_random_pieces() {
        use rand::Rng;

        const PARTS: [&[u8]; 33] = [
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
            b"INVITE sip:+4930123456@sip.example.net SIP/2.0",
            b"From: \"+49 30 123\" <sip:0301234@192.0.2.1>;tag=12",
            b"tel:+49-30-12;ext=12",
            b"sip:",
            b"@",
            b"<",
            b">",
            b"p=+49 30 1234567",
            b"0815",
            b"a=crypto:1 inline:KEY",
            b"k=clear:KEY",
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
            // pieces only hide more: a number cut in two, a URI whose end was not seen
            assert_eq!(pieces.len(), whole.len());
            for (w, p) in whole.iter().zip(&pieces) {
                assert!(w == p || *p == b'x', "{:?}", String::from_utf8_lossy(&text));
            }
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
