//! Display filter: a small language of its own that selects packets by the values of their
//! dissection, for example `ip.addr == 192.168.1.10 and tcp.port == 443`,
//! `dns.qry.name contains amazon` or `not (arp or mdns)`.
//!
//! Written for openpacketmon; the field names follow the habits of packet analyzers so they are
//! easy to guess, the grammar and its implementation are our own.
//!
//! - A protocol name alone (`dns`, `tcp`) selects the packets that contain it; a field alone
//!   (`tls.sni`, `tcp.flags.syn`) the packets that have it (a number other than 0).
//! - Comparisons: `==` (or `=`, `eq`), `!=` (`ne`), `<`, `<=`, `>`, `>=` (`lt` …), `contains`,
//!   `in {80 443}`. A field may occur several times in a packet (`ip.addr`, `dns.a`): `==` is true
//!   if one of the values matches, `!=` if the field is there and none matches.
//! - Text is compared without regard to case; addresses also as networks (`ip.addr == 10.0.0.0/8`).
//! - Times are written in seconds (`dns.time > 0.1`) or with a unit (`100ms`, `800us`).
//! - `and` (`&&`), `or` (`||`), `not` (`!`) and parentheses; `and` binds stronger than `or`.
//!
//! The same language selects connections ([`Filter::parse_connections`]), with fields of their own
//! (`ip.addr`, `port`, `service`, `domain`, `program`, `favorite` …).

mod fields;
#[cfg(test)]
mod tests;

use std::net::IpAddr;

use crate::dissect::{Dissection, Value};
pub use fields::{CONNECTION_FIELDS, CONNECTION_PROTOCOLS, FIELDS, Kind, PROTOCOLS};
use fields::{FrameField, Source, lookup, lookup_connection};

/// What a packet offers to the filter
pub struct Packet<'a> {
    pub number: u64,
    /// Length on the wire
    pub len: u32,
    pub captured: usize,
    pub d: &'a Dissection,
}

/// What a connection offers to the filter: its protocols and values by the keys of
/// [`CONNECTION_FIELDS`]
pub struct Connection<'a> {
    pub protocols: &'a [&'static str],
    pub values: &'a [(&'static str, Value)],
}

/// What the filter reads: a packet or a connection
trait Subject {
    fn protocols(&self) -> &[&'static str];
    fn values(&self) -> &[(&'static str, Value)];
    fn frame(&self, field: FrameField) -> u64;
}

impl Subject for Packet<'_> {
    fn protocols(&self) -> &[&'static str] {
        &self.d.protocols
    }

    fn values(&self) -> &[(&'static str, Value)] {
        &self.d.values
    }

    fn frame(&self, field: FrameField) -> u64 {
        field.value(self)
    }
}

impl Subject for Connection<'_> {
    fn protocols(&self) -> &[&'static str] {
        self.protocols
    }

    fn values(&self) -> &[(&'static str, Value)] {
        self.values
    }

    /// Connections have no frame fields ([`lookup_connection`] does not offer them)
    fn frame(&self, _field: FrameField) -> u64 {
        0
    }
}

/// A parsed display filter
#[derive(Debug)]
pub struct Filter {
    expr: Expr,
}

/// Why a filter could not be parsed, with the byte position in the text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// A name that is neither a field nor a protocol
    UnknownField(String),
    /// The comparison lacks its value
    MissingValue,
    /// The value does not fit the field (a word for a number field, for example)
    BadValue { field: String, value: String },
    /// The operator does not work with this field (`contains` with a port)
    BadOperator { field: String, op: String },
    /// Something else where a field, a value or `and`/`or` was expected
    Unexpected(String),
    /// An opening parenthesis or brace lacks its closing one
    MissingClose,
    /// A string in quotes does not end
    UnclosedString,
    /// Longer or more deeply nested than [`MAX_LEN`] and [`MAX_DEPTH`]
    TooComplex,
}

/// Longest filter text, in bytes
pub const MAX_LEN: usize = 4096;
/// Deepest nesting of parentheses and `not`
const MAX_DEPTH: usize = 64;

#[derive(Debug)]
enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    /// A protocol or field is there
    Has(Source),
    Compare(Source, Op, Vec<Operand>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    In,
}

#[derive(Debug)]
enum Operand {
    Num(u64),
    /// An address with a prefix length (32 or 128 for a single address)
    Net(IpAddr, u8),
    /// Lowercase
    Text(String),
    Mac([u8; 6]),
}

impl Filter {
    /// Parses a packet filter; an empty text is `Ok(None)` (all packets)
    pub fn parse(text: &str) -> Result<Option<Self>, Error> {
        Self::parse_with(text, lookup)
    }

    /// Parses a connection filter; an empty text is `Ok(None)` (all connections)
    pub fn parse_connections(text: &str) -> Result<Option<Self>, Error> {
        Self::parse_with(text, lookup_connection)
    }

    fn parse_with(text: &str, lookup: fn(&str) -> Option<Source>) -> Result<Option<Self>, Error> {
        if text.len() > MAX_LEN {
            return Err(Error {
                kind: ErrorKind::TooComplex,
                pos: MAX_LEN,
            });
        }
        let tokens = tokenize(text)?;
        if tokens.is_empty() {
            return Ok(None);
        }
        let mut parser = Parser {
            tokens,
            pos: 0,
            end: text.len(),
            depth: 0,
            lookup,
        };
        let expr = parser.or()?;
        if let Some((token, pos)) = parser.tokens.get(parser.pos) {
            return Err(Error {
                kind: ErrorKind::Unexpected(token.text()),
                pos: *pos,
            });
        }
        Ok(Some(Self { expr }))
    }

    pub fn matches(&self, packet: &Packet) -> bool {
        self.expr.eval(packet)
    }

    pub fn matches_connection(&self, connection: &Connection) -> bool {
        self.expr.eval(connection)
    }
}

/// A text as a filter value: in quotes, with `"` and `\\` escaped
pub fn quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for c in text.chars() {
        if c == '"' || c == '\\' {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    quoted
}

/// A value as it is written in a filter: a plain word as it is, anything else in quotes
pub fn literal(text: &str) -> String {
    match tokenize(text).as_deref() {
        Ok([(token @ Token::Word(_), _)]) if !token.is(KEYWORDS) => text.to_owned(),
        _ => quote(text),
    }
}

/// The operators that work with a kind of value, the most useful first
pub fn operators(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Num | Kind::Time => &["==", "!=", ">", ">=", "<", "<=", "in"],
        Kind::Text => &["contains", "==", "!=", "in"],
        Kind::Ip | Kind::Mac => &["==", "!=", "in"],
    }
}

/// The keys of the values of a field of the connection filter
pub fn connection_keys(name: &str) -> Option<&'static [&'static str]> {
    CONNECTION_FIELDS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, keys, _)| *keys)
}

// ---- colors -------------------------------------------------------------------------------

/// The coloring rules of the packet list: CSS class and filter, the first match wins
pub const COLOR_RULES: [(&str, &str); 9] = [
    (
        "bad",
        "tcp.analysis.flags or tcp.flags.rst or icmp.type in {3 4 5 11 12} or icmpv6.type < 128 \
         or dns.flags.rcode > 0 or tls.alert or http.response.code >= 400",
    ),
    ("tcp-control", "tcp.flags.syn or tcp.flags.fin"),
    ("dns", "dns"),
    ("local", "arp or dhcp or dhcpv6 or ssdp or igmp or ntp"),
    ("tls", "tls or quic"),
    ("http", "http"),
    ("icmp", "icmp or icmpv6"),
    ("voip", "sip or rtp or rtcp"),
    ("udp", "udp"),
];

/// The CSS class of the first coloring rule the packet matches
pub fn color(packet: &Packet) -> Option<&'static str> {
    static RULES: std::sync::OnceLock<Vec<(&'static str, Filter)>> = std::sync::OnceLock::new();
    let rules = RULES.get_or_init(|| {
        COLOR_RULES
            .iter()
            .filter_map(|(class, text)| Some((*class, Filter::parse(text).ok()??)))
            .collect()
    });
    rules
        .iter()
        .find(|(_, filter)| filter.matches(packet))
        .map(|(class, _)| *class)
}

// ---- tokens --------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Str(String),
    Sym(&'static str),
}

impl Token {
    fn text(&self) -> String {
        match self {
            Token::Word(w) => w.clone(),
            Token::Str(s) => format!("\"{s}\""),
            Token::Sym(s) => (*s).to_owned(),
        }
    }

    /// A keyword (`and`, `contains` …) or symbol
    fn is(&self, words: &[&str]) -> bool {
        match self {
            Token::Word(w) => words.iter().any(|k| w.eq_ignore_ascii_case(k)),
            Token::Sym(s) => words.contains(s),
            Token::Str(_) => false,
        }
    }
}

const SYMBOLS: [&str; 14] = [
    "==", "!=", ">=", "<=", "&&", "||", ">", "<", "=", "!", "(", ")", "{", "}",
];

fn tokenize(text: &str) -> Result<Vec<(Token, usize)>, Error> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some(&(pos, c)) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
            continue;
        }
        if c == '"' {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next() {
                    None => {
                        return Err(Error {
                            kind: ErrorKind::UnclosedString,
                            pos,
                        });
                    }
                    Some((_, '"')) => break,
                    Some((_, '\\')) => {
                        if let Some((_, escaped)) = chars.next() {
                            s.push(escaped);
                        }
                    }
                    Some((_, other)) => s.push(other),
                }
            }
            tokens.push((Token::Str(s), pos));
            continue;
        }
        if let Some(sym) = SYMBOLS.iter().find(|s| text[pos..].starts_with(**s)) {
            for _ in 0..sym.len() {
                chars.next();
            }
            tokens.push((Token::Sym(sym), pos));
            continue;
        }
        let mut word = String::new();
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() || "\"(){}=!<>&|,".contains(c) {
                break;
            }
            word.push(c);
            chars.next();
        }
        if word.is_empty() {
            return Err(Error {
                kind: ErrorKind::Unexpected(c.to_string()),
                pos,
            });
        }
        tokens.push((Token::Word(word), pos));
    }
    Ok(tokens)
}

// ---- parser --------------------------------------------------------------------------------

struct Parser {
    tokens: Vec<(Token, usize)>,
    pos: usize,
    /// Length of the text, the position of errors at its end
    end: usize,
    /// Current nesting of parentheses and `not`
    depth: usize,
    /// The names this filter knows (packets or connections)
    lookup: fn(&str) -> Option<Source>,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn here(&self) -> usize {
        self.tokens.get(self.pos).map_or(self.end, |(_, p)| *p)
    }

    fn eat(&mut self, words: &[&str]) -> bool {
        if self.peek().is_some_and(|t| t.is(words)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn or(&mut self) -> Result<Expr, Error> {
        let mut left = self.and()?;
        while self.eat(&["or", "||"]) {
            left = Expr::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, Error> {
        let mut left = self.unary()?;
        while self.eat(&["and", "&&"]) {
            left = Expr::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.depth >= MAX_DEPTH {
            return Err(Error {
                kind: ErrorKind::TooComplex,
                pos: self.here(),
            });
        }
        if self.eat(&["not", "!"]) {
            self.depth += 1;
            let inner = self.unary()?;
            self.depth -= 1;
            return Ok(Expr::Not(Box::new(inner)));
        }
        let open = self.here();
        if self.eat(&["("]) {
            self.depth += 1;
            let inner = self.or()?;
            self.depth -= 1;
            if !self.eat(&[")"]) {
                return Err(Error {
                    kind: ErrorKind::MissingClose,
                    pos: open,
                });
            }
            return Ok(inner);
        }
        self.test()
    }

    fn test(&mut self) -> Result<Expr, Error> {
        let pos = self.here();
        let name = match self.tokens.get(self.pos) {
            Some((Token::Word(w), _)) if !Token::Word(w.clone()).is(KEYWORDS) => w.clone(),
            Some((token, _)) => {
                return Err(Error {
                    kind: ErrorKind::Unexpected(token.text()),
                    pos,
                });
            }
            None => {
                return Err(Error {
                    kind: ErrorKind::MissingValue,
                    pos,
                });
            }
        };
        let Some(source) = (self.lookup)(&name.to_ascii_lowercase()) else {
            return Err(Error {
                kind: ErrorKind::UnknownField(name),
                pos,
            });
        };
        self.pos += 1;
        let op_pos = self.here();
        let op = match self.peek() {
            Some(t) if t.is(&["==", "=", "eq"]) => Op::Eq,
            Some(t) if t.is(&["!=", "ne"]) => Op::Ne,
            Some(t) if t.is(&["<", "lt"]) => Op::Lt,
            Some(t) if t.is(&["<=", "le"]) => Op::Le,
            Some(t) if t.is(&[">", "gt"]) => Op::Gt,
            Some(t) if t.is(&[">=", "ge"]) => Op::Ge,
            Some(t) if t.is(&["contains"]) => Op::Contains,
            Some(t) if t.is(&["in"]) => Op::In,
            _ => return Ok(Expr::Has(source)),
        };
        let op_text = self.peek().map(Token::text).unwrap_or_default();
        self.pos += 1;
        let kind = source.kind();
        let allowed = match op {
            Op::Eq | Op::Ne | Op::In => kind.is_some(),
            Op::Lt | Op::Le | Op::Gt | Op::Ge => matches!(kind, Some(Kind::Num | Kind::Time)),
            Op::Contains => kind == Some(Kind::Text),
        };
        let Some(kind) = kind.filter(|_| allowed) else {
            return Err(Error {
                kind: ErrorKind::BadOperator {
                    field: name,
                    op: op_text,
                },
                pos: op_pos,
            });
        };
        let mut values = Vec::new();
        if op == Op::In {
            let open = self.here();
            if !self.eat(&["{"]) {
                return Err(Error {
                    kind: ErrorKind::Unexpected(self.peek().map(Token::text).unwrap_or_default()),
                    pos: open,
                });
            }
            while !self.eat(&["}"]) {
                if self.peek().is_none() {
                    return Err(Error {
                        kind: ErrorKind::MissingClose,
                        pos: open,
                    });
                }
                values.push(self.value(&name, kind)?);
            }
            if values.is_empty() {
                return Err(Error {
                    kind: ErrorKind::MissingValue,
                    pos: open,
                });
            }
        } else {
            values.push(self.value(&name, kind)?);
        }
        Ok(Expr::Compare(source, op, values))
    }

    fn value(&mut self, field: &str, kind: Kind) -> Result<Operand, Error> {
        let pos = self.here();
        let text = match self.tokens.get(self.pos) {
            Some((Token::Word(w), _)) if !Token::Word(w.clone()).is(KEYWORDS) => w.clone(),
            Some((Token::Str(s), _)) => s.clone(),
            _ => {
                return Err(Error {
                    kind: ErrorKind::MissingValue,
                    pos,
                });
            }
        };
        self.pos += 1;
        parse_value(field, kind, &text).ok_or(Error {
            kind: ErrorKind::BadValue {
                field: field.to_owned(),
                value: text,
            },
            pos,
        })
    }
}

/// Words that cannot be field names or bare values
const KEYWORDS: &[&str] = &[
    "and", "or", "not", "eq", "ne", "lt", "le", "gt", "ge", "contains", "in",
];

fn parse_value(field: &str, kind: Kind, text: &str) -> Option<Operand> {
    match kind {
        Kind::Num => {
            let number = if let Some(hex) = text.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).ok()
            } else {
                text.parse().ok()
            };
            number
                .or_else(|| named_number(field, text))
                .map(Operand::Num)
        }
        Kind::Ip => {
            let (addr, prefix) = match text.split_once('/') {
                Some((a, p)) => (a.parse::<IpAddr>().ok()?, Some(p.parse::<u8>().ok()?)),
                None => (text.parse::<IpAddr>().ok()?, None),
            };
            let max = if addr.is_ipv4() { 32 } else { 128 };
            let prefix = prefix.unwrap_or(max);
            (prefix <= max).then_some(Operand::Net(addr, prefix))
        }
        Kind::Time => parse_time(text).map(Operand::Num),
        Kind::Text => Some(Operand::Text(text.to_lowercase())),
        Kind::Mac => {
            let parts: Vec<u8> = text
                .split([':', '-'])
                .map(|p| u8::from_str_radix(p, 16).ok().filter(|_| p.len() <= 2))
                .collect::<Option<_>>()?;
            Some(Operand::Mac(parts.try_into().ok()?))
        }
    }
}

/// A duration in microseconds: seconds (`0.25`, `2`) or with a unit (`250ms`, `800us`, `1s`)
fn parse_time(text: &str) -> Option<u64> {
    let lower = text.to_ascii_lowercase();
    let (number, factor) = if let Some(n) = lower.strip_suffix("ms") {
        (n, 1_000.0)
    } else if let Some(n) = lower
        .strip_suffix("us")
        .or_else(|| lower.strip_suffix("µs"))
    {
        (n, 1.0)
    } else if let Some(n) = lower.strip_suffix('s') {
        (n, 1_000_000.0)
    } else {
        (lower.as_str(), 1_000_000.0)
    };
    let value: f64 = number.trim().parse().ok()?;
    if !value.is_finite() || value < 0.0 || value > 1e7 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((value * factor).round() as u64)
}

/// Names for the numbers of some fields (`dns.qry.type == AAAA`, `dhcp.type == request`)
fn named_number(field: &str, text: &str) -> Option<u64> {
    let lower = text.to_ascii_lowercase();
    let pick = |names: &[(&str, u64)]| names.iter().find(|(n, _)| *n == lower).map(|(_, v)| *v);
    match field {
        "dns.qry.type" | "dns.resp.type" => crate::dissect::dns_type_number(text).map(u64::from),
        "dhcp.type" => pick(&[
            ("discover", 1),
            ("offer", 2),
            ("request", 3),
            ("decline", 4),
            ("ack", 5),
            ("nak", 6),
            ("release", 7),
            ("inform", 8),
        ]),
        "tls.handshake.type" => pick(&[
            ("client_hello", 1),
            ("server_hello", 2),
            ("new_session_ticket", 4),
            ("certificate", 11),
            ("finished", 20),
        ]),
        "tls.version" | "tls.record.version" => pick(&[
            ("1.0", 0x0301),
            ("1.1", 0x0302),
            ("1.2", 0x0303),
            ("1.3", 0x0304),
        ]),
        "ntp.mode" => pick(&[("client", 3), ("server", 4), ("broadcast", 5)]),
        "arp.opcode" => pick(&[("request", 1), ("reply", 2)]),
        _ => None,
    }
}

// ---- evaluation ----------------------------------------------------------------------------

impl Expr {
    fn eval<S: Subject>(&self, p: &S) -> bool {
        match self {
            Expr::And(a, b) => a.eval(p) && b.eval(p),
            Expr::Or(a, b) => a.eval(p) || b.eval(p),
            Expr::Not(a) => !a.eval(p),
            Expr::Has(source) => match source {
                Source::Protocol(name) => p.protocols().contains(name),
                Source::Frame(_) => true,
                Source::Keys(keys, _) => values(p, keys).any(|v| match v {
                    Value::Num(n) => *n != 0,
                    _ => true,
                }),
            },
            Expr::Compare(source, op, operands) => {
                let mut found = false;
                let mut any = |v: &Value| {
                    found = true;
                    compare(v, *op, operands)
                };
                let hit = match source {
                    Source::Frame(field) => any(&Value::Num(p.frame(*field))),
                    Source::Keys(keys, _) => values(p, keys).any(&mut any),
                    Source::Protocol(_) => false,
                };
                if *op == Op::Ne {
                    // the field is there and none of its values is equal
                    found && !hit
                } else {
                    hit
                }
            }
        }
    }
}

fn values<'a, S: Subject>(
    p: &'a S,
    keys: &'static [&'static str],
) -> impl Iterator<Item = &'a Value> {
    p.values()
        .iter()
        .filter(move |(k, _)| keys.contains(k))
        .map(|(_, v)| v)
}

/// For `!=` this is the test for equality; [`Expr::eval`] negates it over all values
fn compare(value: &Value, op: Op, operands: &[Operand]) -> bool {
    operands.iter().any(|operand| match (value, operand) {
        (Value::Num(v), Operand::Num(x)) => match op {
            Op::Eq | Op::Ne | Op::In => v == x,
            Op::Lt => v < x,
            Op::Le => v <= x,
            Op::Gt => v > x,
            Op::Ge => v >= x,
            Op::Contains => false,
        },
        (Value::Ip(v), Operand::Net(net, prefix)) => in_network(*v, *net, *prefix),
        (Value::Text(v), Operand::Text(x)) => {
            if op == Op::Contains {
                v.to_lowercase().contains(x.as_str())
            } else {
                v.eq_ignore_ascii_case(x)
            }
        }
        (Value::Mac(v), Operand::Mac(x)) => v == x,
        _ => false,
    })
}

fn in_network(addr: IpAddr, net: IpAddr, prefix: u8) -> bool {
    match (addr, net) {
        (IpAddr::V4(a), IpAddr::V4(n)) => {
            let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
            u32::from(a) & mask == u32::from(n) & mask
        }
        (IpAddr::V6(a), IpAddr::V6(n)) => {
            let mask = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
            u128::from(a) & mask == u128::from(n) & mask
        }
        _ => false,
    }
}
