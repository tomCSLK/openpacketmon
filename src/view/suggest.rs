//! What the filter fields suggest while a filter is typed: the names of the filter with their
//! kinds of value and operators, and for the connection filter the values of the capture.

use std::collections::HashMap;

use serde::Serialize;

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
#[derive(Serialize)]
pub struct Name {
    pub n: &'static str,
    /// Kind of value, none for a protocol
    pub k: Option<&'static str>,
    /// What it means, for the names most in use
    pub h: Option<String>,
}

/// What a kind of value allows
#[derive(Serialize)]
pub struct KindInfo {
    pub ops: &'static [&'static str],
    /// What such a value looks like
    pub hint: String,
}

/// All a filter field needs to know
#[derive(Serialize)]
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
#[derive(Serialize)]
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
}
