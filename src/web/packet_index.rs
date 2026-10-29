//! The packets that pass a display filter, kept per filter and brought up to date as packets
//! arrive: the list, its live updates and the download then dissect only the new packets.
//!
//! The store is locked only to copy a chunk of packets (their bytes are shared); the dissection
//! runs without the lock, so the capture thread is never held up for long. The packets are
//! analyzed on the way ([`analysis`]), so the filter sees the findings of the TCP analysis.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::engine::analysis;
use crate::engine::packets::SharedPacketStore;
use crate::filter::Filter;
use crate::web::packets::{FOLLOWUP_PACKETS, dissect_following, filter_packet};

/// Filters kept at a time (the list, the download, a second tab …)
const ENTRIES: usize = 4;
/// Packets dissected per lock of the store
const CHUNK: usize = 2048;
/// How long a message split over TCP segments waits for its next segment before the packet is
/// judged without it
const PENDING: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct FilterIndex(Mutex<Vec<Entry>>);

struct Entry {
    store: u64,
    filter: String,
    /// Numbers of the matching packets still in the store, oldest first
    matches: VecDeque<u64>,
    /// The packets up to this number have been judged
    checked: u64,
    /// Since when the next packet waits for the rest of its message
    pending_since: Option<Instant>,
    used: Instant,
}

impl FilterIndex {
    /// Calls `f` with the numbers of the stored packets that pass `filter` (whose text is `text`)
    pub fn with_matches<R>(
        &self,
        store: &SharedPacketStore,
        text: &str,
        filter: &Filter,
        f: impl FnOnce(&VecDeque<u64>) -> R,
    ) -> R {
        let mut entries = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let id = store.lock().id();
        // results of finished captures are of no use any more
        entries.retain(|e| e.store == id);
        let i = if let Some(i) = entries.iter().position(|e| e.filter == text) {
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
                matches: VecDeque::new(),
                checked: 0,
                pending_since: None,
                used: Instant::now(),
            });
            entries.len() - 1
        };
        let entry = &mut entries[i];
        entry.used = Instant::now();
        update(entry, store, filter);
        f(&entry.matches)
    }
}

fn update(entry: &mut Entry, store: &SharedPacketStore, filter: &Filter) {
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
            if let Some(record) = store.analysis().feed(packet, &dissected.d) {
                analysis::annotate(&record, &mut dissected.d);
            }
            if dissected.d.followup.is_some()
                && last - packet.number < FOLLOWUP_PACKETS as u64
                && entry
                    .pending_since
                    .get_or_insert_with(Instant::now)
                    .elapsed()
                    < PENDING
            {
                // the rest of the message may still come
                return;
            }
            entry.pending_since = None;
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
