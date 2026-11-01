//! What must not leave the window with an answer. Everything an assistant receives goes to its AI
//! service, and much of it was written by other devices: a host name can hold an instruction, and
//! a request can hold a password.

use serde_json::Value;

/// Longest text in an answer
const MAX_CHARS: usize = 500;
const HIDDEN: &str = "[hidden]";

/// Parts of a header name that mark a value as a secret
const SECRET_HEADERS: [&str; 8] = [
    "cookie", "auth", "token", "secret", "password", "session", "api-key", "apikey",
];

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
    let lower = name.to_ascii_lowercase();
    if SECRET_HEADERS.iter().any(|secret| lower.contains(secret)) {
        format!("{name}: {HIDDEN}")
    } else {
        hide_query(line)
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
}
