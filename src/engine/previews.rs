//! Packet counts of all network adapters while no capture is running,
//! so the user can see which adapter carries traffic before choosing one

use std::collections::VecDeque;

use crate::networking::traffic_preview::TrafficPreview;
use crate::networking::types::my_device::MyDevice;

/// Number of seconds of history kept per adapter
pub const PREVIEW_SECONDS: usize = 30;

#[derive(Debug, Clone)]
pub struct DevicePreview {
    pub device: MyDevice,
    /// Packets per second, oldest first
    pub packets: VecDeque<u128>,
    /// The adapter carries no IP traffic the program understands (its link type is not
    /// supported); it is shown greyed out and cannot be chosen
    pub no_ip: bool,
}

impl DevicePreview {
    fn new(device: MyDevice) -> Self {
        Self {
            device,
            packets: VecDeque::with_capacity(PREVIEW_SECONDS),
            no_ip: false,
        }
    }

    fn push(&mut self, packets: u128) {
        if self.packets.len() == PREVIEW_SECONDS {
            self.packets.pop_front();
        }
        self.packets.push_back(packets);
    }

    /// Packets seen in the kept history
    pub fn tot_packets(&self) -> u128 {
        self.packets.iter().sum()
    }
}

/// Applies one second of preview data: adapters that disappeared are dropped,
/// new ones are added, and the list is sorted by traffic (busiest first, adapters without IP
/// traffic last)
pub fn apply_traffic_preview(previews: &mut Vec<DevicePreview>, msg: TrafficPreview) {
    previews.retain(|p| {
        msg.data
            .iter()
            .any(|(d, _)| d.get_name().eq(p.device.get_name()))
    });
    for (dev, packets) in msg.data {
        let no_ip = msg.no_ip.contains(dev.get_name());
        if let Some(preview) = previews
            .iter_mut()
            .find(|p| p.device.get_name().eq(dev.get_name()))
        {
            preview.device = dev;
            // a failed start may have found it too (see `EngineState::start`)
            preview.no_ip |= no_ip;
            preview.push(packets);
        } else {
            let mut preview = DevicePreview::new(dev);
            preview.no_ip = no_ip;
            preview.push(packets);
            previews.push(preview);
        }
    }
    previews.sort_by_key(|p| (p.no_ip, std::cmp::Reverse(p.tot_packets())));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str) -> MyDevice {
        MyDevice::from_pcap_device(pcap::Device::from(name))
    }

    #[test]
    fn previews_follow_adapters_and_sort_by_traffic() {
        let mut previews = Vec::new();
        apply_traffic_preview(
            &mut previews,
            TrafficPreview {
                data: vec![(device("eth0"), 1), (device("wlan0"), 5)],
                ..TrafficPreview::default()
            },
        );
        assert_eq!(previews[0].device.get_name(), "wlan0");

        apply_traffic_preview(
            &mut previews,
            TrafficPreview {
                data: vec![(device("eth0"), 20), (device("lo"), 0)],
                ..TrafficPreview::default()
            },
        );
        let names: Vec<_> = previews
            .iter()
            .map(|p| p.device.get_name().clone())
            .collect();
        assert_eq!(names, ["eth0", "lo"]);
        assert_eq!(previews[0].tot_packets(), 21);
    }

    #[test]
    fn adapters_without_ip_traffic_are_marked_and_come_last() {
        let mut previews = Vec::new();
        apply_traffic_preview(
            &mut previews,
            TrafficPreview {
                data: vec![
                    (device("bluetooth0"), 0),
                    (device("eth0"), 0),
                    (device("nflog"), 0),
                ],
                no_ip: ["bluetooth0", "nflog"].map(String::from).into(),
            },
        );
        let rows: Vec<_> = previews
            .iter()
            .map(|p| (p.device.get_name().as_str(), p.no_ip))
            .collect();
        assert_eq!(
            rows,
            [("eth0", false), ("bluetooth0", true), ("nflog", true)]
        );
    }

    #[test]
    fn preview_history_is_bounded() {
        let mut preview = DevicePreview::new(device("eth0"));
        for i in 0..(PREVIEW_SECONDS as u128 + 5) {
            preview.push(i);
        }
        assert_eq!(preview.packets.len(), PREVIEW_SECONDS);
        assert_eq!(preview.packets.front(), Some(&5));
    }
}
