//! The overview of the running capture: tiles, what stands out ("Auffällig"), the top hosts,
//! services and programs (`docs/entwurf/assets/pages.js`, `pageOverview`).

use std::collections::HashMap;
use std::net::IpAddr;

use crate::config::favorite::{Favorite, FavoriteItem};
use crate::engine::Engine;
use crate::engine::analysis::{self, flag};
use crate::engine::state::EngineState;
use crate::networking::manage_packets::get_address_to_lookup;
use crate::networking::types::capture_context::CaptureSource;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::service::Service;
use crate::networking::types::traffic_direction::TrafficDirection;
use crate::report::types::sort_type::SortType;
use crate::view::entries::{EntryRow, entry_rows, program_label};
use crate::view::i18n::Lang;
use crate::view::{Jump, format};

/// Entries in each top list
const TOP: usize = 7;
/// Names in a row of "Auffällig"
const NAMES: usize = 3;

/// The capture shown in the page head: where, since when, how long
pub struct CaptureHead {
    /// `adapter`, `file` or `ipfix`
    pub source: &'static str,
    pub name: String,
    /// Time of day of the first packet
    pub since: String,
    /// Duration as a clock (`12:07`)
    pub duration: String,
}

/// The numbers of the tiles, written in the unit of the page
pub struct Tiles {
    pub total: String,
    pub packets: String,
    pub incoming: String,
    /// Share of all data, in percent
    pub in_share: u32,
    pub outgoing: String,
    pub out_share: u32,
    pub connections: String,
    pub hosts: String,
    pub favorite_hosts: usize,
    /// `None` when the source does not report them (files)
    pub dropped: Option<u128>,
}

/// The kind of a row of "Auffällig"
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttentionKind {
    /// Packets with a finding of the TCP analysis
    Tcp,
    /// Favorites exchange data
    Favorites,
    /// Web without encryption
    PlainWeb,
    /// A call with a bad connection
    Call,
}

/// A row of "Auffällig": a title, a text and where its button leads
pub struct Attention {
    pub kind: AttentionKind,
    pub title: String,
    pub text: String,
    pub jump: Jump,
}

/// What the overview shows
pub struct Overview {
    pub capture: Option<CaptureHead>,
    pub tiles: Tiles,
    pub attention: Vec<Attention>,
    pub hosts: Vec<EntryRow>,
    pub services: Vec<EntryRow>,
    /// Not for capture sources without programs
    pub programs: Option<Vec<EntryRow>>,
}

pub(crate) fn capture_head(state: &EngineState) -> Option<CaptureHead> {
    if !state.is_running() {
        return None;
    }
    let source = &state.capture_source;
    let first = state.traffic_history.first_packet_timestamp();
    Some(CaptureHead {
        source: match source {
            CaptureSource::Device(_) => "adapter",
            CaptureSource::File(_) => "file",
            CaptureSource::Ipfix(_) => "ipfix",
        },
        name: match source {
            CaptureSource::File(_) => crate::view::file_name(&source.display_name()),
            _ => source.display_name(),
        },
        since: jiff::Timestamp::from_second(first.secs()).map_or_else(
            |_| String::new(),
            |at| {
                at.to_zoned(jiff::tz::TimeZone::system())
                    .strftime("%H:%M:%S")
                    .to_string()
            },
        ),
        duration: format::clock(u64::from(state.traffic_history.ticks())),
    })
}

#[allow(clippy::cast_possible_truncation)] // percent
fn tiles(t: Lang, state: &EngineState) -> Tiles {
    let unit = state.conf.data_repr.data_unit;
    let repr = DataRepr {
        data_unit: unit,
        per_second: false,
    };
    let data = &state.info_traffic.tot_data_info;
    let bytes = DataRepr::bytes(false);
    let all = data.tot_data(bytes).max(1);
    let share = |part: u128| (part * 100 / all) as u32;
    Tiles {
        total: format::amount(t, unit, data.tot_data(repr)),
        packets: format::count(t, data.tot_data(DataRepr::packets(false))),
        incoming: format::amount(t, unit, data.incoming_data(repr)),
        in_share: share(data.incoming_data(bytes)),
        outgoing: format::amount(t, unit, data.outgoing_data(repr)),
        out_share: share(data.outgoing_data(bytes)),
        connections: format::count(t, state.info_traffic.map.len() as u128),
        hosts: format::count(t, state.info_traffic.hosts.len() as u128),
        favorite_hosts: state
            .favorite_entries(Favorite::Host, repr, SortType::Neutral, true)
            .len(),
        dropped: state
            .info_traffic
            .dropped_packets
            .map(|d| d.total(DataRepr::packets(false), *data)),
    }
}

fn top(t: Lang, state: &EngineState, kind: Favorite) -> Vec<EntryRow> {
    let repr = DataRepr {
        data_unit: state.conf.data_repr.data_unit,
        per_second: false,
    };
    let mut items = state.favorite_entries(kind, repr, SortType::Descending, false);
    items.truncate(TOP);
    entry_rows(t, state, &items)
}

/// `a, b und c`
fn and_list(t: Lang, names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} {} {last}", rest.join(", "), t.get("attn-and")),
    }
}

/// The first names and how many more there are: `a, b, c und 2 weitere`
fn some_names(t: Lang, names: &[String]) -> String {
    if names.len() <= NAMES {
        return and_list(t, names);
    }
    let mut shown = names[..NAMES].to_vec();
    shown.push(t.get1("attn-more", "count", names.len() - NAMES));
    and_list(t, &shown)
}

/// The name of a remote host: its domain, else its address
fn host_name(state: &EngineState, address: IpAddr) -> String {
    match state.addresses_resolved.get(&address) {
        Some((rdns, host)) if !host.domain.is_empty() || !rdns.is_empty() => {
            if host.domain.is_empty() {
                rdns.clone()
            } else {
                host.domain.clone()
            }
        }
        _ => address.to_string(),
    }
}

/// Favorites with traffic in this capture
fn favorites(t: Lang, state: &EngineState) -> Option<Attention> {
    let repr = DataRepr::bytes(false);
    let hosts: Vec<String> = state
        .favorite_entries(Favorite::Host, repr, SortType::Descending, true)
        .iter()
        .map(|item| match item {
            FavoriteItem::Host((host, _)) if !host.domain.is_empty() => host.domain.clone(),
            other => other.to_entry_string(),
        })
        .collect();
    let programs: Vec<String> = state
        .favorite_entries(Favorite::Program, repr, SortType::Descending, true)
        .iter()
        .filter_map(|item| match item {
            FavoriteItem::Program((program, _)) => Some(program_label(t, program)),
            _ => None,
        })
        .collect();
    if hosts.is_empty() && programs.is_empty() {
        return None;
    }
    let programs = (!programs.is_empty()).then(|| {
        t.get2(
            "attn-fav-programs",
            "count",
            programs.len(),
            "names",
            some_names(t, &programs),
        )
    });
    let text = match (hosts.is_empty(), programs) {
        (false, Some(programs)) => {
            format!("{} {} {programs}", some_names(t, &hosts), t.get("attn-and"))
        }
        (false, None) => some_names(t, &hosts),
        (true, Some(programs)) => programs,
        (true, None) => String::new(),
    };
    Some(Attention {
        kind: AttentionKind::Favorites,
        title: t.get("attn-fav-title"),
        text,
        jump: Jump::Notifications,
    })
}

/// Web without encryption: the host with the most HTTP traffic, its port and program
fn plain_web(t: Lang, state: &EngineState) -> Option<Attention> {
    let mut hosts: HashMap<IpAddr, (u128, u16, String)> = HashMap::new();
    for (key, info) in &state.info_traffic.map {
        if info.service != Service::Name("http") {
            continue;
        }
        let remote = get_address_to_lookup(key, info.traffic_direction);
        let port = if info.traffic_direction == TrafficDirection::Outgoing {
            key.dst_port
        } else {
            key.src_port
        };
        let entry = hosts
            .entry(remote)
            .or_insert_with(|| (0, port.unwrap_or(80), program_label(t, &info.program)));
        entry.0 += info.bytes;
    }
    let count = hosts.len();
    let (address, (_, port, program)) = hosts.into_iter().max_by_key(|(_, v)| v.0)?;
    let name = host_name(state, address);
    let title = if count > 1 {
        t.get2("attn-web-title-more", "host", name, "count", count - 1)
    } else {
        t.get1("attn-web-title", "host", name)
    };
    let known =
        !program.is_empty() && program != t.get("unknown") && program != t.get("not-applicable");
    let text = if known {
        t.get2("attn-web-text-program", "port", port, "program", program)
    } else {
        t.get1("attn-web-text", "port", port)
    };
    Some(Attention {
        kind: AttentionKind::PlainWeb,
        title,
        text,
        jump: Jump::Packets {
            filter: "http".to_owned(),
            connections: String::new(),
        },
    })
}

/// Counts of the findings of the TCP analysis (all stored packets) and the remote hosts they
/// concern most
#[derive(Default, Debug, PartialEq, Eq)]
pub struct TcpFindings {
    pub packets: u64,
    pub retransmissions: u64,
    pub duplicate_acks: u64,
    pub out_of_order: u64,
    pub resets: u64,
    /// Remote addresses with the most findings, most first
    pub remotes: Vec<IpAddr>,
}

const SHOWN_FINDINGS: u16 =
    flag::RETRANSMISSION | flag::DUPLICATE_ACK | flag::OUT_OF_ORDER | flag::RESET;

fn tcp_findings(engine: &Engine) -> TcpFindings {
    let Some(store) = engine.lock().packets.clone() else {
        return TcpFindings::default();
    };
    let last = store.lock().last_number();
    analysis::advance(&store, last);
    let mut found = TcpFindings::default();
    let mut numbers = Vec::new();
    {
        let analysis = store.analysis();
        for (number, record) in analysis.after(0) {
            let flags = record.flags & SHOWN_FINDINGS;
            if flags == 0 {
                continue;
            }
            found.packets += 1;
            found.retransmissions += u64::from(flags & flag::RETRANSMISSION != 0);
            found.duplicate_acks += u64::from(flags & flag::DUPLICATE_ACK != 0);
            found.out_of_order += u64::from(flags & flag::OUT_OF_ORDER != 0);
            found.resets += u64::from(flags & flag::RESET != 0);
            numbers.push(number);
        }
    }
    let mut remotes: HashMap<IpAddr, u64> = HashMap::new();
    {
        let packets = store.lock();
        for number in numbers {
            if let Some((key, direction)) = packets.get(number).and_then(|p| p.conn.as_ref()) {
                *remotes
                    .entry(get_address_to_lookup(key, *direction))
                    .or_default() += 1;
            }
        }
    }
    let mut remotes: Vec<_> = remotes.into_iter().collect();
    remotes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    found.remotes = remotes.into_iter().take(2).map(|(a, _)| a).collect();
    found
}

fn tcp_row(t: Lang, state: &EngineState, f: &TcpFindings) -> Option<Attention> {
    if f.packets == 0 {
        return None;
    }
    let n = |v: u64| format::count(t, u128::from(v)).into();
    let mut text = t.fmt(
        "attn-tcp-text",
        &[
            ("retransmissions", n(f.retransmissions)),
            ("dupacks", n(f.duplicate_acks)),
            ("ooo", n(f.out_of_order)),
            ("resets", n(f.resets)),
        ],
    );
    if !f.remotes.is_empty() {
        let names: Vec<String> = f.remotes.iter().map(|a| host_name(state, *a)).collect();
        text.push_str(&t.get1("attn-tcp-mostly", "hosts", and_list(t, &names)));
    }
    Some(Attention {
        kind: AttentionKind::Tcp,
        title: t.get1(
            "attn-tcp-title",
            "count",
            format::count(t, u128::from(f.packets)),
        ),
        text,
        jump: Jump::Statistics {
            filter: "tcp.analysis.flags".to_owned(),
            connections: String::new(),
        },
    })
}

pub fn overview(t: Lang, engine: &Engine) -> Overview {
    let findings = tcp_findings(engine);
    let calls = crate::view::calls::bad_calls(t, engine);
    let state = engine.lock();
    let mut attention: Vec<Attention> = tcp_row(t, &state, &findings).into_iter().collect();
    attention.extend(calls.into_iter().map(|call| Attention {
        kind: AttentionKind::Call,
        title: t.get("attn-call-title"),
        text: call.text,
        jump: Jump::Call(call.key),
    }));
    attention.extend(
        [favorites(t, &state), plain_web(t, &state)]
            .into_iter()
            .flatten(),
    );
    Overview {
        capture: capture_head(&state),
        tiles: tiles(t, &state),
        attention,
        hosts: top(t, &state, Favorite::Host),
        services: top(t, &state, Favorite::Service),
        programs: (!state.is_running() || state.capture_source.supports_programs())
            .then(|| top(t, &state, Favorite::Program)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_joined_with_and() {
        let names = |n: &[&str]| n.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(and_list(Lang::De, &names(&["a"])), "a");
        assert_eq!(and_list(Lang::De, &names(&["a", "b", "c"])), "a, b und c");
        assert_eq!(and_list(Lang::En, &names(&["a", "b"])), "a and b");
        assert_eq!(
            some_names(Lang::De, &names(&["a", "b", "c", "d", "e"])),
            "a, b, c und 2 weitere"
        );
    }
}
