//! The settings as the interface shows and changes them (language, notifications, geolocation
//! databases, update check, packet store).

use std::net::IpAddr;

use crate::config::favorite::{FavoriteKey, MAX_ADDRESSES};
use crate::config::packet_store::PacketStoreConf;
use crate::engine::state::EngineState;
use crate::mmdb::geo;
use crate::mmdb::types::mmdb_reader::MmdbReader;
use crate::notifications::types::sound::Sound;
use crate::view::connections::{DEFAULT_PAGE_SIZE, PAGE_SIZES};
use crate::view::format;
use crate::view::i18n::Lang;

/// An entry of a selection
pub struct Choice {
    pub value: &'static str,
    pub label: String,
    pub selected: bool,
}

pub fn sound_code(sound: Sound) -> &'static str {
    match sound {
        Sound::Gulp => "gulp",
        Sound::Pop => "pop",
        Sound::Swhoosh => "swhoosh",
        Sound::None => "none",
    }
}

pub fn parse_sound(code: &str) -> Sound {
    Sound::ALL
        .into_iter()
        .find(|s| sound_code(*s) == code)
        .unwrap_or(Sound::None)
}

fn sound_choices(t: Lang, selected: Sound) -> Vec<Choice> {
    Sound::ALL
        .into_iter()
        .map(|s| Choice {
            value: sound_code(s),
            label: t.get(&format!("sound-{}", sound_code(s))),
            selected: s == selected,
        })
        .collect()
}

/// The favorite addresses, one per line, sorted
fn addresses_text(addresses: &std::collections::HashSet<IpAddr>) -> String {
    let mut sorted: Vec<&IpAddr> = addresses.iter().collect();
    sorted.sort();
    sorted
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Addresses typed by the user: separated by lines, spaces or commas; the words that are no
/// address are returned apart. Both stop at their limits (the first [`MAX_ADDRESSES`] addresses
/// are kept, [`MAX_INVALID`] wrong words are named).
pub fn parse_addresses(text: &str) -> (Vec<IpAddr>, Vec<String>) {
    let mut addresses = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut invalid = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        if word.is_empty() {
            continue;
        }
        match word.parse::<IpAddr>() {
            Ok(ip) => {
                if addresses.len() < MAX_ADDRESSES && seen.insert(ip) {
                    addresses.push(ip);
                }
            }
            Err(_) => {
                if invalid.len() < MAX_INVALID {
                    invalid.push(word.chars().take(60).collect());
                }
            }
        }
    }
    (addresses, invalid)
}

/// Most wrong words named after saving the favorite addresses
const MAX_INVALID: usize = 20;

/// A geolocation database in use
pub struct GeoRow {
    pub label: String,
    /// The file and its description, `None` without a database
    pub file: Option<(String, String)>,
    /// Build date and age in days
    pub built: String,
    pub old: bool,
}

fn geo_row(t: Lang, label: &str, reader: &MmdbReader) -> GeoRow {
    let label = t.get(label);
    let Some((path, metadata)) = reader.source() else {
        return GeoRow {
            label,
            file: None,
            built: String::new(),
            old: false,
        };
    };
    let age = geo::age_days(metadata.build_epoch);
    let built = jiff::Timestamp::from_second(metadata.build_epoch.cast_signed())
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::system()).date().to_string())
        .unwrap_or_default();
    GeoRow {
        label,
        file: Some((path.display().to_string(), metadata.database_type.clone())),
        built: t.get2("settings-geo-built", "date", &built, "days", age),
        old: age > geo::OLD_AFTER_DAYS,
    }
}

/// The settings as they are now
#[allow(clippy::struct_excessive_bools)] // switches of the settings
pub struct Settings {
    pub languages: Vec<Choice>,
    pub page_sizes: Vec<(usize, bool)>,
    pub favorite_active: bool,
    pub favorite_sounds: Vec<Choice>,
    pub favorite_addresses: String,
    pub volume: u8,
    pub mmdb_country: String,
    pub mmdb_asn: String,
    pub geo_rows: Vec<GeoRow>,
    pub geo_dir: String,
    pub update_checks: bool,
    pub packets_enabled: bool,
    pub packets_max: Vec<(String, usize, bool)>,
    pub packets_headers: bool,
}

pub fn settings(t: Lang, engine: &EngineState) -> Settings {
    let conf = &engine.conf;
    let notifications = &conf.settings.notifications;
    let mut languages = vec![Choice {
        value: "",
        label: t.get("language-browser"),
        selected: Lang::parse(&conf.language).is_none(),
    }];
    languages.extend(Lang::ALL.into_iter().map(|l| Choice {
        value: l.code(),
        label: l.native_name().to_owned(),
        selected: conf.language == l.code(),
    }));
    let page_size = if conf.page_size == 0 {
        DEFAULT_PAGE_SIZE
    } else {
        conf.page_size
    };
    Settings {
        languages,
        page_sizes: PAGE_SIZES
            .into_iter()
            .map(|s| (s, s == page_size))
            .collect(),
        favorite_active: notifications.favorite_notification.is_active,
        favorite_sounds: sound_choices(t, notifications.favorite_notification.sound),
        favorite_addresses: addresses_text(conf.favorites.addresses()),
        volume: notifications.volume,
        mmdb_country: conf.settings.mmdb_country.clone(),
        mmdb_asn: conf.settings.mmdb_asn.clone(),
        geo_rows: vec![
            geo_row(
                t,
                "settings-geo-country",
                &engine.mmdb_readers.country.get(),
            ),
            geo_row(t, "settings-geo-asn", &engine.mmdb_readers.asn.get()),
        ],
        geo_dir: geo::geo_dir()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default(),
        update_checks: !conf.updates.disable_checks(),
        packets_enabled: conf.packet_store.enabled,
        packets_max: packets_max_choices(t, conf.packet_store),
        packets_headers: conf.packet_store.headers_only,
    }
}

/// The choices for the number of stored packets: label, number, selected
pub fn packets_max_choices(t: Lang, conf: PacketStoreConf) -> Vec<(String, usize, bool)> {
    PacketStoreConf::MAX_PACKETS_CHOICES
        .into_iter()
        .map(|n| (format::count(t, n as u128), n, n == conf.max_packets()))
        .collect()
}

/// New values of the settings
#[allow(clippy::struct_excessive_bools)] // switches of the settings
pub struct SettingsChange {
    /// Language code; anything unknown follows the system
    pub language: String,
    /// Left as it is if it is not one of the sizes offered
    pub page_size: usize,
    pub favorite_active: bool,
    pub favorite_sound: Sound,
    /// All favorite addresses
    pub addresses: Vec<IpAddr>,
    pub volume: u8,
    pub mmdb_country: String,
    pub mmdb_asn: String,
    pub update_checks: bool,
    pub packets_enabled: bool,
    /// Left as it is if it is not one of the choices
    pub packets_max: usize,
    pub packets_headers: bool,
}

/// Puts new values into the settings; saving them is up to the caller
pub fn apply_settings(engine: &mut EngineState, change: &SettingsChange) {
    let conf = &mut engine.conf;
    conf.language = Lang::parse(&change.language)
        .map(|l| l.code().to_owned())
        .unwrap_or_default();
    if PAGE_SIZES.contains(&change.page_size) {
        conf.page_size = change.page_size;
    }

    let notifications = &mut conf.settings.notifications;
    notifications.favorite_notification.is_active = change.favorite_active;
    notifications.favorite_notification.sound = change.favorite_sound;
    notifications.volume = change.volume.min(100);
    let old: Vec<IpAddr> = conf.favorites.addresses().iter().copied().collect();
    for ip in old.iter().filter(|ip| !change.addresses.contains(ip)) {
        conf.favorites.remove(&FavoriteKey::Address(*ip));
    }
    for ip in &change.addresses {
        conf.favorites.insert(&FavoriteKey::Address(*ip));
    }

    conf.packet_store.enabled = change.packets_enabled;
    if PacketStoreConf::MAX_PACKETS_CHOICES.contains(&change.packets_max) {
        conf.packet_store.max_packets = change.packets_max;
    }
    conf.packet_store.headers_only = change.packets_headers;

    if conf.updates.disable_checks() == change.update_checks {
        conf.updates.toggle_disable_checks();
    }

    let country = change.mmdb_country.trim().to_owned();
    if country != engine.conf.settings.mmdb_country {
        engine.set_custom_country_db(country);
    }
    let asn = change.mmdb_asn.trim().to_owned();
    if asn != engine.conf.settings.mmdb_asn {
        engine.set_custom_asn_db(asn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_and_addresses_are_parsed() {
        assert_eq!(parse_sound("pop"), Sound::Pop);
        assert_eq!(parse_sound("<script>"), Sound::None);
        let (addresses, invalid) =
            parse_addresses("192.168.1.20\n fe80::1, 10.0.0.1;192.168.1.20 example.com\n");
        assert_eq!(
            addresses,
            ["192.168.1.20", "fe80::1", "10.0.0.1"].map(|a| a.parse::<IpAddr>().unwrap())
        );
        assert_eq!(invalid, ["example.com"]);
    }
}
