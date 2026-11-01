//! What the filter fields suggest while a filter is typed: the names of the filter with their
//! kinds of value and operators, and for the connection filter the values of the capture.

use std::collections::HashMap;

use crate::engine::state::EngineState;
use crate::filter::{self, Kind};
use crate::view::i18n::Lang;

/// Most values suggested
const VALUES: usize = 12;
/// Fields whose values are of no use as suggestions (sums, a yes or no)
const NO_VALUES: [&str; 3] = ["packets", "bytes", "favorite"];

const KINDS: [(Kind, &str); 5] = [
    (Kind::Num, "num"),
    (Kind::Ip, "ip"),
    (Kind::Text, "text"),
    (Kind::Mac, "mac"),
    (Kind::Time, "time"),
];

fn kind_code(kind: Kind) -> &'static str {
    KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or("text", |(_, code)| code)
}

/// A name of the filter
pub struct Name {
    pub n: &'static str,
    /// Kind of value, none for a protocol
    pub k: Option<&'static str>,
    /// What it means, for the names most in use
    pub h: Option<String>,
}

/// What a kind of value allows
pub struct KindInfo {
    pub ops: &'static [&'static str],
    /// What such a value looks like
    pub hint: String,
}

/// All a filter field needs to know
pub struct Catalog {
    pub names: Vec<Name>,
    pub kinds: HashMap<&'static str, KindInfo>,
    /// The meaning of `not`
    pub not: String,
}

/// The explanation of a name: message `filter-hint-tls-sni` for `tls.sni`
fn hint(t: Lang, name: &str) -> Option<String> {
    let id = format!("filter-hint-{}", name.replace(['.', '_'], "-"));
    let text = t.get(&id);
    (text != id).then_some(text)
}

pub fn catalog(t: Lang, connections: bool) -> Catalog {
    let name = |n, k| Name {
        n,
        k,
        h: hint(t, n),
    };
    let (protocols, fields): (&[&'static str], _) = if connections {
        (&filter::CONNECTION_PROTOCOLS, filter::CONNECTION_FIELDS)
    } else {
        (&filter::PROTOCOLS, filter::FIELDS)
    };
    let mut names: Vec<Name> = protocols.iter().map(|p| name(p, None)).collect();
    names.extend(
        fields
            .iter()
            .map(|(n, _, kind)| name(n, Some(kind_code(*kind)))),
    );
    if !connections {
        names.extend(["frame.number", "frame.len", "frame.cap_len"].map(|n| name(n, Some("num"))));
    }
    // the explained ones are the ones most in use
    names.sort_by_key(|n| n.h.is_none());
    Catalog {
        names,
        kinds: KINDS
            .iter()
            .map(|(kind, code)| {
                let info = KindInfo {
                    ops: filter::operators(*kind),
                    hint: t.get(&format!("filter-value-{code}")),
                };
                (*code, info)
            })
            .collect(),
        not: t.get("filter-hint-not"),
    }
}

/// A value of the capture
pub struct ValueRow {
    pub text: String,
    /// As it is written in a filter
    pub insert: String,
    /// Connections that have it
    pub count: usize,
}

/// Values of a field of the connection filter that contain `part`, the most frequent first
pub fn value_rows(engine: &EngineState, field: &str, part: &str) -> Vec<ValueRow> {
    let Some(keys) = filter::connection_keys(field).filter(|_| !NO_VALUES.contains(&field)) else {
        return Vec::new();
    };
    engine
        .connection_field_values(keys, part.trim(), VALUES)
        .into_iter()
        .map(|(text, count)| ValueRow {
            insert: filter::literal(&text),
            text,
            count,
        })
        .collect()
}

// ---- the text before the cursor -----------------------------------------------------------------

/// Most lines of names shown
const LINES: usize = 60;
const CONNECTORS: [&str; 2] = ["and", "or"];
const OPERATORS: [&str; 15] = [
    "==", "=", "eq", "!=", "ne", "<", "lt", "<=", "le", ">", "gt", ">=", "ge", "contains", "in",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TokenKind {
    /// A string without its closing quote
    Open,
    Text,
    Symbol,
    Word,
}

struct Token<'a> {
    kind: TokenKind,
    text: &'a str,
    start: usize,
    end: usize,
}

/// The same words as the tokenizer of the filter (`src/filter/mod.rs`)
fn tokens(text: &str) -> Vec<Token<'_>> {
    const TWO: [&str; 6] = ["==", "!=", "<=", ">=", "&&", "||"];
    let stops_word = |c: char| c.is_whitespace() || "\"(){}=!<>&|,".contains(c);
    let mut list = Vec::new();
    let mut i = 0;
    while let Some(c) = text[i..].chars().next() {
        let rest = &text[i..];
        if c.is_whitespace() || c == ',' {
            i += c.len_utf8();
        } else if c == '"' {
            let mut end = None;
            let mut chars = rest.char_indices().skip(1);
            while let Some((at, d)) = chars.next() {
                if d == '"' {
                    end = Some(i + at + 1);
                    break;
                }
                if d == '\\' {
                    chars.next();
                }
            }
            let Some(end) = end else {
                list.push(Token {
                    kind: TokenKind::Open,
                    text: rest,
                    start: i,
                    end: text.len(),
                });
                return list;
            };
            list.push(Token {
                kind: TokenKind::Text,
                text: &text[i..end],
                start: i,
                end,
            });
            i = end;
        } else if let Some(two) = TWO.iter().find(|s| rest.starts_with(**s)) {
            list.push(Token {
                kind: TokenKind::Symbol,
                text: two,
                start: i,
                end: i + 2,
            });
            i += 2;
        } else if "=<>!(){}".contains(c) {
            list.push(Token {
                kind: TokenKind::Symbol,
                text: &rest[..1],
                start: i,
                end: i + 1,
            });
            i += 1;
        } else if stops_word(c) {
            i += c.len_utf8();
        } else {
            let len = rest.find(stops_word).unwrap_or(rest.len());
            list.push(Token {
                kind: TokenKind::Word,
                text: &rest[..len],
                start: i,
                end: i + len,
            });
            i += len;
        }
    }
    list
}

/// What can come next in a filter
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expect {
    /// A name
    Term,
    Operator,
    /// The `{` after `in`
    Brace,
    Value,
    /// The values of `in`
    Set,
    /// `and`, `or`
    Next,
}

/// Where the cursor is in a filter
pub struct Position<'a> {
    pub expect: Expect,
    /// The field a value or operator belongs to
    pub field: Option<&'a Name>,
    /// The word being typed
    pub partial: String,
    /// Where that word starts, in bytes
    pub start: usize,
    depth: i32,
}

/// Follows the grammar over the finished words before the cursor
pub fn position<'a>(before: &str, catalog: &'a Catalog) -> Position<'a> {
    let mut list = tokens(before);
    let (mut partial, mut start) = (String::new(), before.len());
    if let Some(last) = list.last()
        && last.end == before.len()
        && matches!(last.kind, TokenKind::Word | TokenKind::Open)
    {
        last.text.clone_into(&mut partial);
        start = last.start;
        list.pop();
    }
    let mut expect = Expect::Term;
    let mut field = None;
    let mut depth = 0;
    for token in &list {
        let word = if token.kind == TokenKind::Word {
            token.text.to_lowercase()
        } else {
            token.text.to_owned()
        };
        let word = word.as_str();
        let connector = matches!(word, "and" | "or" | "&&" | "||");
        match expect {
            Expect::Term => {
                if word == "(" {
                    depth += 1;
                } else if word != "not" && word != "!" {
                    field = catalog.names.iter().find(|n| n.n == word && n.k.is_some());
                    expect = if field.is_some() {
                        Expect::Operator
                    } else {
                        Expect::Next
                    };
                }
            }
            Expect::Operator if OPERATORS.contains(&word) => {
                expect = if word == "in" {
                    Expect::Brace
                } else {
                    Expect::Value
                };
            }
            Expect::Brace => {
                expect = if word == "{" {
                    Expect::Set
                } else {
                    Expect::Next
                };
            }
            Expect::Value => expect = Expect::Next,
            Expect::Set => {
                if word == "}" {
                    expect = Expect::Next;
                }
            }
            _ if connector => expect = Expect::Term,
            _ => {
                if word == ")" {
                    depth -= 1;
                }
                expect = Expect::Next;
            }
        }
    }
    Position {
        expect,
        field,
        partial,
        start,
        depth,
    }
}

/// A line of the list of suggestions
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Line {
    pub name: String,
    /// What the line puts into the field
    pub insert: String,
    pub hint: Option<String>,
}

impl Line {
    fn plain(word: &str) -> Self {
        Self {
            name: word.to_owned(),
            insert: word.to_owned(),
            hint: None,
        }
    }
}

/// Lines that start with the text come first, then those with a part that does, then the rest
fn rank(name: &str, text: &str) -> Option<usize> {
    let at = name.find(text)?;
    if at == 0 {
        return Some(0);
    }
    Some(if matches!(name.as_bytes()[at - 1], b'.' | b'_') {
        1
    } else {
        2
    })
}

/// What can come at this position
pub fn lines(position: &Position, catalog: &Catalog) -> Vec<Line> {
    let text = position.partial.to_lowercase();
    let starting = |words: &[&str]| -> Vec<Line> {
        words
            .iter()
            .filter(|w| w.starts_with(&text))
            .map(|w| Line::plain(w))
            .collect()
    };
    match position.expect {
        Expect::Term => {
            let mut found: [Vec<Line>; 3] = Default::default();
            let names = catalog
                .names
                .iter()
                .map(|n| (n.n, n.h.clone()))
                .chain([("not", Some(catalog.not.clone()))]);
            for (name, hint) in names {
                if let Some(r) = rank(name, &text) {
                    found[r].push(Line {
                        name: name.to_owned(),
                        insert: name.to_owned(),
                        hint,
                    });
                }
            }
            found.into_iter().flatten().take(LINES).collect()
        }
        Expect::Operator => {
            let ops = position
                .field
                .and_then(|f| f.k)
                .and_then(|k| catalog.kinds.get(k))
                .map_or(&[][..], |k| k.ops);
            let mut lines = starting(ops);
            for line in &mut lines {
                if line.name == "in" {
                    "in {".clone_into(&mut line.insert);
                }
            }
            lines.extend(starting(&CONNECTORS));
            lines
        }
        Expect::Brace => vec![Line::plain("{")],
        Expect::Set => starting(&["}"]),
        Expect::Next => {
            let mut lines = starting(&CONNECTORS);
            if position.depth > 0 {
                lines.extend(starting(&[")"]));
            }
            lines
        }
        Expect::Value => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::Filter;

    #[test]
    fn every_explanation_belongs_to_a_name() {
        for (lang, file) in [
            (Lang::De, include_str!("../../locales/de/app.ftl")),
            (Lang::En, include_str!("../../locales/en/app.ftl")),
        ] {
            let mut known: Vec<String> = [catalog(lang, false), catalog(lang, true)]
                .iter()
                .flat_map(|c| c.names.iter().filter(|n| n.h.is_some()))
                .map(|n| format!("filter-hint-{}", n.n.replace(['.', '_'], "-")))
                .collect();
            known.push("filter-hint-not".to_owned());
            let explained = file
                .lines()
                .filter_map(|l| l.split_once(" = "))
                .map(|(id, _)| id)
                .filter(|id| id.starts_with("filter-hint-"));
            for id in explained {
                assert!(known.iter().any(|k| k == id), "{id} names nothing");
            }
        }
    }

    #[test]
    fn the_catalog_offers_what_the_filter_takes() {
        for connections in [false, true] {
            let c = catalog(Lang::En, connections);
            assert!(c.names[0].h.is_some());
            for name in &c.names {
                let parsed = if connections {
                    Filter::parse_connections(name.n)
                } else {
                    Filter::parse(name.n)
                };
                assert!(parsed.is_ok(), "{}", name.n);
                let Some(code) = name.k else { continue };
                let value = match code {
                    "ip" => "10.0.0.1",
                    "mac" => "aa:bb:cc:dd:ee:ff",
                    "text" => "x",
                    _ => "1",
                };
                for op in c.kinds[code].ops {
                    let text = if *op == "in" {
                        format!("{} in {{{value}}}", name.n)
                    } else {
                        format!("{} {op} {value}", name.n)
                    };
                    let parsed = if connections {
                        Filter::parse_connections(&text)
                    } else {
                        Filter::parse(&text)
                    };
                    assert!(parsed.is_ok(), "{text}");
                }
            }
        }
    }

    #[test]
    fn the_cursor_says_what_can_come_next() {
        let cat = catalog(Lang::En, false);
        let at = |text: &str| position(text, &cat).expect;
        assert_eq!(at(""), Expect::Term);
        assert_eq!(at("tc"), Expect::Term);
        assert_eq!(at("tcp.port "), Expect::Operator);
        assert_eq!(at("tcp.port == "), Expect::Value);
        assert_eq!(at("tcp.port == 80 "), Expect::Next);
        assert_eq!(at("tcp.port == 80 and "), Expect::Term);
        assert_eq!(at("tcp.port in "), Expect::Brace);
        assert_eq!(at("tcp.port in { 80 "), Expect::Set);
        assert_eq!(at("not tcp.port "), Expect::Operator);
        assert_eq!(at("dns "), Expect::Next);
        assert_eq!(at("domain contains \"open"), Expect::Value);
    }

    #[test]
    fn the_word_being_typed_is_replaced() {
        let cat = catalog(Lang::En, false);
        let p = position("tcp.port == 80 and tls.s", &cat);
        assert_eq!(p.partial, "tls.s");
        assert_eq!(p.start, 19);
        let names = lines(&p, &cat);
        assert_eq!(names[0].name, "tls.sni");
        assert_eq!(names[0].insert, "tls.sni");
    }

    #[test]
    fn operators_and_connectors_follow_a_field() {
        let cat = catalog(Lang::En, false);
        let names: Vec<String> = lines(&position("tcp.port ", &cat), &cat)
            .into_iter()
            .map(|l| l.insert)
            .collect();
        assert!(names.contains(&"==".to_owned()), "{names:?}");
        assert!(names.contains(&"in {".to_owned()), "{names:?}");
        assert!(names.contains(&"and".to_owned()), "{names:?}");
        let inside = lines(&position("(dns ", &cat), &cat);
        assert!(inside.iter().any(|l| l.name == ")"));
        let outside = lines(&position("dns ", &cat), &cat);
        assert!(!outside.iter().any(|l| l.name == ")"));
    }

    #[test]
    fn text_outside_of_ascii_does_not_break_the_tokens() {
        let cat = catalog(Lang::En, false);
        let p = position("domain contains \"müller\" and tö", &cat);
        assert_eq!(p.partial, "tö");
        assert_eq!(p.expect, Expect::Term);
        assert_eq!(position("&| é ", &cat).expect, Expect::Next);
    }
}
