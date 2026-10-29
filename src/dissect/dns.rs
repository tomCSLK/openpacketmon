//! DNS (RFC 1035, 3596, 2782, 6891, 9460), multicast DNS (RFC 6762) and LLMNR (RFC 4795)

use std::fmt::Write;
use std::ops::Range;

use super::{Dissection, Field, hex16, ipv4_at, ipv6_at, text, u16_at, u32_at};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Dns,
    Mdns,
    Llmnr,
}

pub(super) fn type_name(t: u16) -> String {
    match t {
        1 => "A",
        2 => "NS",
        5 => "CNAME",
        6 => "SOA",
        12 => "PTR",
        13 => "HINFO",
        15 => "MX",
        16 => "TXT",
        28 => "AAAA",
        33 => "SRV",
        35 => "NAPTR",
        41 => "OPT",
        43 => "DS",
        46 => "RRSIG",
        47 => "NSEC",
        48 => "DNSKEY",
        50 => "NSEC3",
        64 => "SVCB",
        65 => "HTTPS",
        255 => "ANY",
        257 => "CAA",
        _ => return format!("TYPE{t}"),
    }
    .to_owned()
}

fn class_name(c: u16) -> String {
    match c {
        1 => "IN".to_owned(),
        3 => "CH".to_owned(),
        255 => "ANY".to_owned(),
        other => format!("CLASS{other}"),
    }
}

fn rcode_name(r: u16) -> &'static str {
    match r {
        0 => "No error",
        1 => "Format error",
        2 => "Server failure",
        3 => "No such name",
        4 => "Not implemented",
        5 => "Refused",
        _ => "Error",
    }
}

/// A message, with the position of its first byte in the packet (for the byte ranges of the fields)
struct Msg<'a> {
    m: &'a [u8],
    base: usize,
}

impl Msg<'_> {
    fn r(&self, range: Range<usize>) -> Range<usize> {
        self.base + range.start..self.base + range.end
    }

    /// A (possibly compressed) name at `at`; the name and the position after it
    fn name(&self, at: usize) -> Option<(String, usize)> {
        let mut labels = Vec::new();
        let mut pos = at;
        let mut end = None;
        let mut jumps = 0;
        let mut total = 0;
        loop {
            let len = usize::from(*self.m.get(pos)?);
            if len == 0 {
                pos += 1;
                break;
            }
            if len & 0xc0 == 0xc0 {
                let pointer = ((len & 0x3f) << 8) | usize::from(*self.m.get(pos + 1)?);
                end.get_or_insert(pos + 2);
                jumps += 1;
                if jumps > 64 {
                    return None;
                }
                pos = pointer;
                continue;
            }
            if len & 0xc0 != 0 {
                return None;
            }
            total += len + 1;
            if total > 255 {
                return None;
            }
            labels.push(text(self.m.get(pos + 1..pos + 1 + len)?));
            pos += 1 + len;
        }
        let name = if labels.is_empty() {
            "<Root>".to_owned()
        } else {
            labels.join(".")
        };
        Some((name, end.unwrap_or(pos)))
    }

    /// The data of a record as text
    fn rdata(&self, kind: u16, at: usize, len: usize) -> String {
        let data = || self.m.get(at..at + len);
        let value = match kind {
            1 if len == 4 => ipv4_at(self.m, at).map(|a| a.to_string()),
            28 if len == 16 => ipv6_at(self.m, at).map(|a| a.to_string()),
            2 | 5 | 12 => self.name(at).map(|(n, _)| n),
            15 => u16_at(self.m, at).and_then(|pref| {
                self.name(at + 2)
                    .map(|(n, _)| format!("{n} (preference {pref})"))
            }),
            16 => data().and_then(|d| {
                let mut out = Vec::new();
                let mut i = 0;
                while let Some(&l) = d.get(i) {
                    let l = usize::from(l);
                    out.push(format!(
                        "\"{}\"",
                        text(d.get(i + 1..(i + 1 + l).min(d.len()))?)
                    ));
                    i += 1 + l;
                }
                Some(out.join(" "))
            }),
            33 => (|| {
                let (prio, weight, port) = (
                    u16_at(self.m, at)?,
                    u16_at(self.m, at + 2)?,
                    u16_at(self.m, at + 4)?,
                );
                let (target, _) = self.name(at + 6)?;
                Some(format!(
                    "{target}:{port} (priority {prio}, weight {weight})"
                ))
            })(),
            6 => (|| {
                let (mname, next) = self.name(at)?;
                let (rname, next) = self.name(next)?;
                Some(format!(
                    "{mname}, {rname}, serial {}",
                    u32_at(self.m, next)?
                ))
            })(),
            64 | 65 => self.svcb(at, len),
            _ => None,
        };
        let mut value = value.unwrap_or_else(|| format!("{len} bytes"));
        if value.len() > 300 {
            let cut = (0..=300)
                .rev()
                .find(|i| value.is_char_boundary(*i))
                .unwrap_or(0);
            value.truncate(cut);
            value.push('…');
        }
        value
    }

    /// SVCB/HTTPS: priority, target and the usual parameters (ALPN, address hints)
    fn svcb(&self, at: usize, len: usize) -> Option<String> {
        let prio = u16_at(self.m, at)?;
        let (target, mut pos) = self.name(at + 2)?;
        let mut out = format!("priority {prio}, target {target}");
        while pos + 4 <= at + len {
            let (key, l) = (u16_at(self.m, pos)?, usize::from(u16_at(self.m, pos + 2)?));
            let value = self.m.get(pos + 4..pos + 4 + l)?;
            match key {
                1 => {
                    let mut alpn = Vec::new();
                    let mut i = 0;
                    while let Some(&n) = value.get(i) {
                        alpn.push(text(value.get(i + 1..i + 1 + usize::from(n))?));
                        i += 1 + usize::from(n);
                    }
                    let _ = write!(out, ", alpn {}", alpn.join(","));
                }
                4 => {
                    let hints: Vec<String> = (0..l / 4)
                        .filter_map(|i| ipv4_at(value, i * 4).map(|a| a.to_string()))
                        .collect();
                    let _ = write!(out, ", ipv4hint {}", hints.join(","));
                }
                5 => out.push_str(", ech"),
                6 => {
                    let hints: Vec<String> = (0..l / 16)
                        .filter_map(|i| ipv6_at(value, i * 16).map(|a| a.to_string()))
                        .collect();
                    let _ = write!(out, ", ipv6hint {}", hints.join(","));
                }
                _ => {}
            }
            pos += 4 + l;
        }
        Some(out)
    }
}

/// DNS over UDP (and mDNS, LLMNR)
pub(super) fn dissect(data: &[u8], payload: Range<usize>, kind: Kind, d: &mut Dissection) -> bool {
    let msg = Msg {
        m: &data[payload.clone()],
        base: payload.start,
    };
    message(&msg, kind, None, d)
}

/// DNS over TCP: each message starts with its length (RFC 1035 4.2.2)
pub(super) fn dissect_tcp(data: &[u8], payload: Range<usize>, d: &mut Dissection) -> bool {
    // the length lies in the payload, not in bytes after it (an Ethernet trailer)
    let Some(len) = u16_at(&data[payload.clone()], 0) else {
        return false;
    };
    let start = payload.start + 2;
    let end = (start + usize::from(len)).min(payload.end);
    let msg = Msg {
        m: &data[start..end],
        base: start,
    };
    let length = Field::new("Length", len.to_string(), payload.start..start);
    message(&msg, Kind::Dns, Some(length), d)
}

/// A record of the answer, authority or additional section
struct Record {
    name: String,
    kind: u16,
    value: String,
    field: Field,
}

#[allow(clippy::too_many_lines)] // header, questions and three sections
fn message(msg: &Msg, kind: Kind, length: Option<Field>, d: &mut Dissection) -> bool {
    let m = msg.m;
    let (Some(id), Some(flags)) = (u16_at(m, 0), u16_at(m, 2)) else {
        return false;
    };
    let counts: Vec<u16> = (0..4).filter_map(|i| u16_at(m, 4 + i * 2)).collect();
    if counts.len() < 4 || counts.iter().map(|c| usize::from(*c)).sum::<usize>() > 1000 {
        return false;
    }
    let response = flags & 0x8000 != 0;
    let opcode = (flags >> 11) & 0x0f;
    let rcode = flags & 0x000f;
    let bit = |mask: u16| if flags & mask == 0 { "Not set" } else { "Set" };
    let mut fields: Vec<Field> = length.into_iter().collect();
    fields.push(Field::new("Transaction ID", hex16(id), msg.r(0..2)));
    let mut flag_fields = vec![
        Field::new(
            "Response",
            if response { "Response" } else { "Query" },
            msg.r(2..3),
        ),
        Field::new(
            "Opcode",
            match opcode {
                0 => "Standard query (0)".to_owned(),
                4 => "Notify (4)".to_owned(),
                5 => "Update (5)".to_owned(),
                o => o.to_string(),
            },
            msg.r(2..3),
        ),
    ];
    if response {
        flag_fields.push(Field::new("Authoritative", bit(0x0400), msg.r(2..3)));
    }
    flag_fields.push(Field::new("Truncated", bit(0x0200), msg.r(2..3)));
    if kind == Kind::Dns {
        flag_fields.push(Field::new("Recursion desired", bit(0x0100), msg.r(2..3)));
        if response {
            flag_fields.push(Field::new("Recursion available", bit(0x0080), msg.r(3..4)));
        }
    }
    if response {
        flag_fields.push(Field::new(
            "Reply code",
            format!("{} ({rcode})", rcode_name(rcode)),
            msg.r(3..4),
        ));
    }
    fields.push(Field::new("Flags", hex16(flags), msg.r(2..4)).with_children(flag_fields));
    for (i, name) in ["Questions", "Answer RRs", "Authority RRs", "Additional RRs"]
        .into_iter()
        .enumerate()
    {
        fields.push(Field::new(
            name,
            counts[i].to_string(),
            msg.r(4 + i * 2..6 + i * 2),
        ));
    }

    let mut pos = 12;
    let mut complete = true;
    let mut questions = Vec::new();
    let mut query_fields = Vec::new();
    for _ in 0..counts[0] {
        let Some((name, p)) = msg.name(pos) else {
            complete = false;
            break;
        };
        let (Some(qtype), Some(qclass)) = (u16_at(m, p), u16_at(m, p + 2)) else {
            complete = false;
            break;
        };
        let class_text = if kind == Kind::Mdns && qclass & 0x8000 != 0 {
            format!("{}, unicast response", class_name(qclass & 0x7fff))
        } else {
            class_name(qclass)
        };
        query_fields.push(
            Field::new(
                "Query",
                format!("{name}: type {}, class {class_text}", type_name(qtype)),
                msg.r(pos..p + 4),
            )
            .with_children(vec![
                Field::new("Name", name.clone(), msg.r(pos..p)),
                Field::new(
                    "Type",
                    format!("{} ({qtype})", type_name(qtype)),
                    msg.r(p..p + 2),
                ),
                Field::new("Class", class_text, msg.r(p + 2..p + 4)),
            ]),
        );
        questions.push((name, qtype));
        pos = p + 4;
    }
    if !query_fields.is_empty() {
        fields.push(
            Field::new("Queries", questions.len().to_string(), msg.r(12..pos))
                .with_children(query_fields),
        );
    }

    let mut answers = Vec::new();
    for (section, count) in [
        ("Answers", counts[1]),
        ("Authoritative nameservers", counts[2]),
        ("Additional records", counts[3]),
    ] {
        if !complete || count == 0 {
            continue;
        }
        let start = pos;
        let mut records = Vec::new();
        for _ in 0..count {
            let Some((r, next)) = record(msg, pos, kind) else {
                complete = false;
                break;
            };
            records.push(r);
            pos = next;
        }
        if section == "Answers" {
            answers = records
                .iter()
                .map(|r| (r.name.clone(), r.kind, r.value.clone()))
                .collect();
        }
        if !records.is_empty() {
            fields.push(
                Field::new(section, records.len().to_string(), msg.r(start..pos))
                    .with_children(records.into_iter().map(|r| r.field).collect()),
            );
        }
    }
    if !complete {
        fields.push(Field::new(
            "[Incomplete]",
            "the message ends early or is not valid",
            msg.r(pos..m.len()),
        ));
    }

    let (protocol, title) = match kind {
        Kind::Dns => ("DNS", "Domain Name System"),
        Kind::Mdns => ("mDNS", "Multicast Domain Name System"),
        Kind::Llmnr => ("LLMNR", "Link-local Multicast Name Resolution"),
    };
    d.protocol = protocol;
    d.has("dns");
    match kind {
        Kind::Mdns => d.has("mdns"),
        Kind::Llmnr => d.has("llmnr"),
        Kind::Dns => {}
    }
    d.num("dns.id", id);
    d.num("dns.flags.response", u8::from(response));
    d.num("dns.flags.rcode", rcode);
    d.num("dns.count.answers", counts[1]);
    for (name, qtype) in &questions {
        d.text("dns.qry.name", name.clone());
        d.num("dns.qry.type", *qtype);
    }
    for (name, rtype, value) in &answers {
        d.text("dns.resp.name", name.clone());
        d.num("dns.resp.type", *rtype);
        match rtype {
            1 | 28 => {
                if let Ok(addr) = value.parse::<std::net::IpAddr>() {
                    d.ip(if *rtype == 1 { "dns.a" } else { "dns.aaaa" }, addr);
                }
            }
            5 => d.text("dns.cname", value.clone()),
            12 => d.text("dns.ptr", value.clone()),
            _ => {}
        }
    }
    d.add(
        format!("{title} ({})", if response { "response" } else { "query" }),
        msg.r(0..m.len()),
        fields,
    );
    d.info = info(kind, id, response, rcode, &questions, &answers);
    true
}

fn record(msg: &Msg, pos: usize, kind: Kind) -> Option<(Record, usize)> {
    let m = msg.m;
    let (name, p) = msg.name(pos)?;
    let rtype = u16_at(m, p)?;
    let class = u16_at(m, p + 2)?;
    let ttl = u32_at(m, p + 4)?;
    let len = usize::from(u16_at(m, p + 8)?);
    let at = p + 10;
    if at + len > m.len() {
        return None;
    }
    let (value, field) = if rtype == 41 {
        // EDNS: the class is the UDP payload size
        let value = format!("UDP payload size {class}");
        (
            value.clone(),
            Field::new("EDNS", value, msg.r(pos..at + len)),
        )
    } else {
        let class_text = if kind == Kind::Mdns && class & 0x8000 != 0 {
            format!("{}, cache flush", class_name(class & 0x7fff))
        } else {
            class_name(class)
        };
        let value = msg.rdata(rtype, at, len);
        let field = Field::new(
            "Record",
            format!("{name}: type {}, {value}", type_name(rtype)),
            msg.r(pos..at + len),
        )
        .with_children(vec![
            Field::new("Name", name.clone(), msg.r(pos..p)),
            Field::new(
                "Type",
                format!("{} ({rtype})", type_name(rtype)),
                msg.r(p..p + 2),
            ),
            Field::new("Class", class_text, msg.r(p + 2..p + 4)),
            Field::new("Time to live", format!("{ttl} s"), msg.r(p + 4..p + 8)),
            Field::new("Data length", len.to_string(), msg.r(p + 8..p + 10)),
            Field::new("Data", value.clone(), msg.r(at..at + len)),
        ]);
        (value, field)
    };
    Some((
        Record {
            name,
            kind: rtype,
            value,
            field,
        },
        at + len,
    ))
}

/// `Query 0x1a2b A github.com`, `Response 0x1a2b A github.com: 140.82.121.4`
fn info(
    kind: Kind,
    id: u16,
    response: bool,
    rcode: u16,
    questions: &[(String, u16)],
    answers: &[(String, u16, String)],
) -> String {
    let mut out = String::from(if response { "Response" } else { "Query" });
    if kind != Kind::Mdns {
        let _ = write!(out, " {}", hex16(id));
    }
    if let Some((name, qtype)) = questions.first() {
        let _ = write!(out, " {} {name}", type_name(*qtype));
        if questions.len() > 1 {
            let _ = write!(out, " (+{})", questions.len() - 1);
        }
    }
    if response && rcode != 0 {
        let _ = write!(out, ": {}", rcode_name(rcode));
    } else if !answers.is_empty() {
        let shown: Vec<String> = answers
            .iter()
            .filter(|(_, t, _)| *t != 41)
            .take(3)
            .map(|(name, t, value)| {
                if questions.is_empty() {
                    format!("{} {name} {value}", type_name(*t))
                } else {
                    value.clone()
                }
            })
            .collect();
        let more = answers.len().saturating_sub(shown.len());
        let _ = write!(out, ": {}", shown.join(", "));
        if more > 0 {
            let _ = write!(out, " (+{more})");
        }
    }
    out
}
