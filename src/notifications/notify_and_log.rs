// Modified for openpacketmon (2026), see NOTICE. Based on `src/notifications/notify_and_log.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::InfoTraffic;
use crate::config::favorite::{FavoriteItem, Favorites};
use crate::networking::types::capture_context::CaptureSource;
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::program_lookup::ProgramLookup;
use crate::notifications::types::logged_notification::{LoggedNotification, LoggedNotifications};
use crate::notifications::types::notifications::Notifications;
use crate::notifications::types::sound::Sound;
use crate::utils::formatted_strings::get_formatted_timestamp;
use std::collections::BTreeMap;
use std::net::IpAddr;

/// Checks if favorites exchanged data in the last interval and logs a notification for each.
///
/// It returns the number of new notifications emitted
pub fn notify_and_log(
    logged_notifications: &mut LoggedNotifications,
    notifications: &Notifications,
    info_traffic_msg: &InfoTraffic,
    favorites: &Favorites,
    program_lookup: Option<&ProgramLookup>,
    cs: &CaptureSource,
) -> (usize, Sound) {
    let mut sound_to_play = Sound::None;
    let emitted_notifications_prev = logged_notifications.tot();
    let timestamp = info_traffic_msg.last_packet_timestamp;

    if notifications.favorite_notification.is_active {
        let favorites_last_interval =
            favorites_last_interval(info_traffic_msg, favorites, program_lookup);

        if !favorites_last_interval.is_empty() {
            for favorite in favorites_last_interval {
                logged_notifications.push(LoggedNotification {
                    favorite,
                    timestamp: get_formatted_timestamp(timestamp),
                });
            }
            sound_to_play = notifications.favorite_notification.sound;
        }
    }

    // no sound when importing data from pcap file
    if !cs.supports_notification_sound() {
        sound_to_play = Sound::None;
    }

    (
        logged_notifications.tot() - emitted_notifications_prev,
        sound_to_play,
    )
}

fn favorites_last_interval(
    info_traffic_msg: &InfoTraffic,
    favorites: &Favorites,
    program_lookup: Option<&ProgramLookup>,
) -> Vec<FavoriteItem> {
    let hosts = favorites.hosts().iter().filter_map(|h| {
        info_traffic_msg
            .hosts
            .get(h)
            .map(|d| FavoriteItem::Host((h.clone(), *d)))
    });

    let programs = favorites.programs().iter().filter_map(|p| {
        let data_info = program_lookup?.programs().get(p)?.interval_data();
        if data_info.tot_data(DataRepr::packets(false)) > 0 {
            Some(FavoriteItem::Program((p.clone(), data_info)))
        } else {
            None
        }
    });

    // traffic of each favorite address, from the connections of the interval (sorted by address)
    let mut addresses: BTreeMap<IpAddr, DataInfo> = BTreeMap::new();
    if !favorites.addresses().is_empty() {
        for (key, value) in &info_traffic_msg.map {
            for ip in [key.src_ip, key.dst_ip] {
                if favorites.contains_address(&ip) {
                    addresses.entry(ip).or_default().refresh(value.data_info());
                }
                if key.src_ip == key.dst_ip {
                    break;
                }
            }
        }
    }
    let addresses = addresses
        .into_iter()
        .map(|(ip, data_info)| FavoriteItem::Address((ip, data_info)));

    hosts.chain(programs).chain(addresses).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::favorite::FavoriteKey;
    use crate::networking::types::address_port_pair::AddressPortPair;
    use crate::networking::types::capture_context::MyPcapImport;
    use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
    use crate::notifications::types::notifications::SimpleNotification;
    use sniffnet_packet_parser::Protocol;

    #[test]
    fn traffic_of_a_favorite_address_is_notified() {
        let favorite: IpAddr = [192, 168, 1, 20].into();
        let other: IpAddr = [10, 0, 0, 1].into();
        let favorites = Favorites::from([FavoriteKey::Address(favorite)]);
        let mut info = InfoTraffic::default();
        let pair = |src, dst| AddressPortPair {
            src_ip: src,
            src_port: Some(1000),
            dst_ip: dst,
            dst_port: Some(443),
            protocol: Protocol::Tcp,
            exporter: None,
        };
        let conn = InfoAddressPortPair {
            bytes: 500,
            packets: 2,
            ..InfoAddressPortPair::default()
        };
        info.map.insert(pair(other, other), conn.clone());
        let notifications = Notifications {
            volume: 0,
            favorite_notification: SimpleNotification::on(Sound::Pop),
        };
        let mut logged = LoggedNotifications::default();
        let cs = CaptureSource::File(MyPcapImport::new(String::new()));
        let (count, _) = notify_and_log(&mut logged, &notifications, &info, &favorites, None, &cs);
        assert_eq!(count, 0, "no traffic of the favorite yet");

        info.map.insert(pair(other, favorite), conn.clone());
        info.map.insert(pair(favorite, other), conn);
        let (count, _) = notify_and_log(&mut logged, &notifications, &info, &favorites, None, &cs);
        assert_eq!(count, 1, "one notification per address");
        let n = &logged.notifications()[0];
        assert!(matches!(n.favorite, FavoriteItem::Address((ip, _)) if ip == favorite));
        assert_eq!(n.favorite.data_info().tot_data(DataRepr::packets(false)), 4);

        // switched off: nothing
        let off = Notifications::default();
        let (count, _) = notify_and_log(&mut logged, &off, &info, &favorites, None, &cs);
        assert_eq!(count, 0);
    }
}
