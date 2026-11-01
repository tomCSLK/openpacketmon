// Modified for openpacketmon (2026), see NOTICE. Contains code adapted from `src/chart/types/traffic_chart.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

//! Per-second traffic samples of the running capture: the data behind the live traffic charts

use std::collections::VecDeque;

use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::info_traffic::InfoTraffic;
use crate::utils::types::timestamp::Timestamp;

/// Maximum number of samples kept.
/// Live captures drop the oldest samples (one hour of history),
/// offline captures merge neighbouring samples so the whole file stays visible.
pub const MAX_SAMPLES: usize = 3600;

/// Maximum number of zero-traffic samples pushed to represent a gap between two offline packets
const MAX_OFFLINE_GAP_POINTS: u32 = 86_400; // one day

/// Traffic of one time interval
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrafficSample {
    /// Seconds since the first packet of the capture
    pub second: u32,
    pub in_bytes: u128,
    pub out_bytes: u128,
    pub in_packets: u128,
    pub out_packets: u128,
}

impl TrafficSample {
    fn average(a: &TrafficSample, b: &TrafficSample) -> TrafficSample {
        TrafficSample {
            second: a.second,
            in_bytes: u128::midpoint(a.in_bytes, b.in_bytes),
            out_bytes: u128::midpoint(a.out_bytes, b.out_bytes),
            in_packets: u128::midpoint(a.in_packets, b.in_packets),
            out_packets: u128::midpoint(a.out_packets, b.out_packets),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrafficHistory {
    /// Number of intervals seen so far
    ticks: u32,
    samples: VecDeque<TrafficSample>,
    is_live_capture: bool,
    /// Whether this is a terminated offline capture
    no_more_packets: bool,
    /// Timestamp of the first packet of the capture
    first_packet_timestamp: Timestamp,
}

impl TrafficHistory {
    #[cfg(test)]
    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    pub fn new(is_live_capture: bool) -> Self {
        Self {
            ticks: 0,
            samples: VecDeque::new(),
            is_live_capture,
            no_more_packets: false,
            first_packet_timestamp: Timestamp::default(),
        }
    }

    /// Adds the traffic of the last interval
    pub fn push(&mut self, info_traffic_msg: &InfoTraffic, no_more_packets: bool) {
        self.no_more_packets = no_more_packets;
        if self.ticks == 0 {
            self.first_packet_timestamp = info_traffic_msg.last_packet_timestamp;
        }
        let data = &info_traffic_msg.tot_data_info;
        let sample = TrafficSample {
            second: self.ticks,
            in_bytes: data.incoming_data(DataRepr::bytes(false)),
            out_bytes: data.outgoing_data(DataRepr::bytes(false)),
            in_packets: data.incoming_data(DataRepr::packets(false)),
            out_packets: data.outgoing_data(DataRepr::packets(false)),
        };
        self.ticks += 1;
        self.add(sample);
    }

    /// Adds an idle period of an offline capture
    pub fn push_offline_gap(&mut self, gap: u32) {
        // an idle period is a flat line: it doesn't need a sample per second,
        // and a capture containing a huge time jump must not exhaust memory
        let step = gap.div_ceil(MAX_OFFLINE_GAP_POINTS).max(1);
        for i in (0..gap).step_by(step as usize) {
            self.add(TrafficSample {
                second: self.ticks + i,
                ..TrafficSample::default()
            });
        }
        self.ticks += gap;
    }

    fn add(&mut self, sample: TrafficSample) {
        self.samples.push_back(sample);
        if self.samples.len() > MAX_SAMPLES {
            if self.is_live_capture {
                self.samples.pop_front();
            } else {
                self.compact();
            }
        }
    }

    /// Halves the number of samples by averaging neighbouring pairs
    fn compact(&mut self) {
        let old = std::mem::take(&mut self.samples);
        let mut iter = old.iter();
        while let Some(a) = iter.next() {
            match iter.next() {
                Some(b) => self.samples.push_back(TrafficSample::average(a, b)),
                None => self.samples.push_back(*a),
            }
        }
    }

    pub fn samples(&self) -> &VecDeque<TrafficSample> {
        &self.samples
    }

    pub fn no_more_packets(&self) -> bool {
        self.no_more_packets
    }

    pub fn first_packet_timestamp(&self) -> Timestamp {
        self.first_packet_timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::types::data_info::DataInfo;

    /// One interval with a single packet per direction that carried traffic
    fn interval(in_bytes: u128, out_bytes: u128) -> InfoTraffic {
        let mut info_traffic = InfoTraffic::default();
        info_traffic.tot_data_info = DataInfo::new_for_tests(
            u128::from(in_bytes > 0),
            u128::from(out_bytes > 0),
            in_bytes,
            out_bytes,
        );
        info_traffic
    }

    #[test]
    fn push_records_one_sample_per_interval() {
        let mut history = TrafficHistory::new(true);
        history.push(&interval(100, 40), false);
        history.push(&interval(0, 0), false);
        history.push(&interval(7, 0), false);

        assert_eq!(history.ticks(), 3);
        let samples: Vec<_> = history.samples().iter().copied().collect();
        assert_eq!(
            samples[0],
            TrafficSample {
                second: 0,
                in_bytes: 100,
                out_bytes: 40,
                in_packets: 1,
                out_packets: 1,
            }
        );
        assert_eq!(samples[1].in_bytes + samples[1].out_bytes, 0);
        assert_eq!(samples[2].second, 2);
        assert_eq!(samples[2].in_bytes, 7);
    }

    #[test]
    fn live_capture_keeps_the_most_recent_samples() {
        let mut history = TrafficHistory::new(true);
        for _ in 0..MAX_SAMPLES + 10 {
            history.push(&interval(1, 0), false);
        }
        assert_eq!(history.samples().len(), MAX_SAMPLES);
        assert_eq!(history.samples().front().map(|s| s.second), Some(10));
    }

    #[test]
    fn offline_capture_is_compacted_instead_of_cut() {
        let mut history = TrafficHistory::new(false);
        for _ in 0..=MAX_SAMPLES {
            history.push(&interval(10, 0), false);
        }
        assert!(history.samples().len() <= MAX_SAMPLES);
        assert_eq!(history.samples().front().map(|s| s.second), Some(0));
        assert!(history.samples().iter().all(|s| s.in_bytes == 10));
    }

    #[test]
    fn offline_gap_adds_idle_seconds() {
        let mut history = TrafficHistory::new(false);
        history.push(&interval(5, 5), false);
        history.push_offline_gap(4);
        history.push(&interval(5, 5), true);

        assert_eq!(history.ticks(), 6);
        assert_eq!(history.samples().len(), 6);
        assert_eq!(history.samples().back().map(|s| s.second), Some(5));
        assert!(history.no_more_packets());
    }

    #[test]
    fn huge_offline_gap_is_sampled_sparsely() {
        let mut history = TrafficHistory::new(false);
        history.push_offline_gap(MAX_OFFLINE_GAP_POINTS * 10);
        assert!(history.samples().len() <= MAX_SAMPLES);
        assert_eq!(history.ticks(), MAX_OFFLINE_GAP_POINTS * 10);
    }
}
