//! The packets that pass a display filter or belong to the connections of a connection filter,
//! kept per filter and brought up to date as packets arrive: the list, its live updates and the
//! export then dissect only the new packets.
//!
//! The store is locked only to copy a chunk of packets (their bytes are shared); the dissection
//! runs without the lock, so the capture thread is never held up for long. The packets are
//! analyzed on the way ([`analysis`]), so the filter sees the findings of the TCP analysis.

use std::collections::{HashSet, VecDeque};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::dissect::Dissection;
use crate::engine::analysis::voip::Mark;
use crate::engine::analysis::{self, Note};
use crate::engine::packets::SharedPacketStore;
use crate::filter::Filter;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::view::packets::{FOLLOWUP_PACKETS, dissect_following, filter_packet};

/// Filters kept at a time (the list, the download, a second tab, both filters of a selection …)
const ENTRIES: usize = 6;
/// Packets dissected per lock of the store
const CHUNK: usize = 2048;
/// How long a message split over TCP segments waits for its next segment, or a datagram with the
/// form of RTP for its flow to be decided, before the packet is judged without
const PENDING: Duration = Duration::from_secs(1);
/// How far the analysis looks ahead for the flow of a datagram with the form of RTP
const LOOKAHEAD: u64 = 20_000;

pub struct FilterIndex {
    entries: Mutex<Vec<Entry>>,
    /// Changes the values of a packet before the filter sees them
    values: fn(&mut Dissection),
}

impl Default for FilterIndex {
    fn default() -> Self {
        Self::with_values(|_| {})
    }
}

struct Entry {
    store: u64,
    filter: String,
    /// The connections selected at the last look, for a connection filter
    connections: Option<HashSet<AddressPortPair>>,
    /// Numbers of the matching packets still in the store, oldest first
    matches: VecDeque<u64>,
    /// The packets up to this number have been judged
    checked: u64,
    /// Since when the next packet waits for the rest of its message
    pending_since: Option<Instant>,
    used: Instant,
}

impl FilterIndex {
    /// An index whose filters see the values of a packet as `values` leaves them: the access for
    /// AI assistants filters on what its answers show, so that a filter cannot find out what they hide
    pub fn with_values(values: fn(&mut Dissection)) -> Self {
        Self {
            entries: Mutex::default(),
            values,
        }
    }

    /// Calls `f` with the numbers of the stored packets that pass `filter` (whose text is `text`)
    pub fn with_matches<R>(
        &self,
        store: &SharedPacketStore,
        text: &str,
        filter: &Filter,
        f: impl FnOnce(&VecDeque<u64>) -> R,
    ) -> R {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = entry(&mut entries, store, text, false);
        update(entry, store, filter, self.values);
        f(&entry.matches)
    }

    /// Calls `f` with the numbers of the stored packets of `connections`, the ones a connection
    /// filter with the text `text` selects now. New connections and values like the volume change
    /// that selection; the packets are then judged anew, which needs no dissection.
    pub fn with_connections<R>(
        &self,
        store: &SharedPacketStore,
        text: &str,
        connections: &HashSet<AddressPortPair>,
        f: impl FnOnce(&VecDeque<u64>) -> R,
    ) -> R {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = entry(&mut entries, store, text, true);
        if entry.connections.as_ref() != Some(connections) {
            entry.connections = Some(connections.clone());
            entry.matches.clear();
            entry.checked = 0;
        }
        update_connections(entry, store, connections);
        f(&entry.matches)
    }
}

/// The entry of a filter, a new one if there is none
fn entry<'a>(
    entries: &'a mut Vec<Entry>,
    store: &SharedPacketStore,
    text: &str,
    connections: bool,
) -> &'a mut Entry {
    let id = store.lock().id();
    // results of finished captures are of no use any more
    entries.retain(|e| e.store == id);
    let found = entries
        .iter()
        .position(|e| e.filter == text && e.connections.is_some() == connections);
    let i = if let Some(i) = found {
        i
    } else {
        if entries.len() >= ENTRIES
            && let Some(oldest) = (0..entries.len()).min_by_key(|i| entries[*i].used)
        {
            entries.remove(oldest);
        }
        entries.push(Entry {
            store: id,
            filter: text.to_owned(),
            connections: connections.then(HashSet::new),
            matches: VecDeque::new(),
            checked: 0,
            pending_since: None,
            used: Instant::now(),
        });
        entries.len() - 1
    };
    let entry = &mut entries[i];
    entry.used = Instant::now();
    entry
}

fn update_connections(
    entry: &mut Entry,
    store: &SharedPacketStore,
    connections: &HashSet<AddressPortPair>,
) {
    loop {
        let store = store.lock();
        let first = store.iter().next().map_or(u64::MAX, |p| p.number);
        while entry.matches.front().is_some_and(|n| *n < first) {
            entry.matches.pop_front();
        }
        let mut judged = 0;
        for packet in store.after(entry.checked).take(CHUNK) {
            judged += 1;
            if packet
                .conn
                .is_some_and(|(key, _)| connections.contains(&key))
            {
                entry.matches.push_back(packet.number);
            }
            entry.checked = packet.number;
        }
        if judged < CHUNK {
            return;
        }
    }
}

fn update(
    entry: &mut Entry,
    store: &SharedPacketStore,
    filter: &Filter,
    values: fn(&mut Dissection),
) {
    loop {
        let (chunk, first, last, link_type) = {
            let store = store.lock();
            (
                store
                    .after(entry.checked)
                    .take(CHUNK + FOLLOWUP_PACKETS)
                    .cloned()
                    .collect::<Vec<_>>(),
                store.iter().next().map_or(u64::MAX, |p| p.number),
                store.last_number(),
                store.link_type(),
            )
        };
        while entry.matches.front().is_some_and(|n| *n < first) {
            entry.matches.pop_front();
        }
        let judged = chunk.len().min(CHUNK);
        for (i, packet) in chunk.iter().take(judged).enumerate() {
            let mut dissected = dissect_following(link_type, packet, chunk[i + 1..].iter());
            let mut note = store.analysis().feed(packet, &dissected.d);
            let undecided = |note: &Option<Note>| {
                note.as_ref()
                    .is_some_and(|n| n.record.voip == Some(Mark::Candidate))
            };
            if undecided(&note) {
                // the flow may still turn out to be RTP: the analysis looks further ahead
                analysis::advance(store, packet.number.saturating_add(LOOKAHEAD).min(last));
                note = store.analysis().note(packet.number);
            }
            if let Some(note) = &note {
                analysis::annotate(note, &packet.data, &mut dissected.d);
            }
            let waits = dissected.d.followup.is_some()
                && last - packet.number < FOLLOWUP_PACKETS as u64
                || undecided(&note) && last - packet.number < LOOKAHEAD;
            if waits
                && entry
                    .pending_since
                    .get_or_insert_with(Instant::now)
                    .elapsed()
                    < PENDING
            {
                // the rest of the message may still come, or the flow be decided
                return;
            }
            entry.pending_since = None;
            values(&mut dissected.d);
            if filter.matches(&filter_packet(packet, &dissected.d)) {
                entry.matches.push_back(packet.number);
            }
            entry.checked = packet.number;
        }
        if judged < CHUNK {
            return;
        }
    }
}
