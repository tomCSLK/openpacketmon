//! The settings as the interface shows and changes them (language, notifications, geolocation
//! databases, update check, packet store, about).

use std::net::IpAddr;

use crate::config::favorite::{FavoriteKey, MAX_ADDRESSES};
use crate::config::packet_store::PacketStoreConf;
use crate::engine::state::EngineState;
use crate::mmdb::geo;
use crate::mmdb::types::mmdb_reader::MmdbReader;
use crate::notifications::types::sound::Sound;
use crate::utils::check_updates::UpdateStatus;
use crate::utils::formatted_strings::APP_VERSION;
use crate::view::connections::{DEFAULT_PAGE_SIZE, PAGE_SIZES};
use crate::view::format;
use crate::view::i18n::Lang;
use crate::view::release_notes::{self, Block};

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

/// A geolocation database in use (a row of `dl.facts` in the panel "Geodaten")
pub struct GeoRow {
    pub label: String,
    /// The name of the database (`DB-IP Lite`), `None` without one
    pub name: Option<String>,
    /// Build date and age in days
    pub built: String,
    /// The file
    pub path: String,
    pub old: bool,
}

fn geo_row(t: Lang, label: &str, reader: &MmdbReader) -> GeoRow {
    let label = t.get(label);
    let Some((path, metadata)) = reader.source() else {
        return GeoRow {
            label,
            name: None,
            built: String::new(),
            path: String::new(),
            old: false,
        };
    };
    let age = geo::age_days(metadata.build_epoch);
    let built = jiff::Timestamp::from_second(metadata.build_epoch.cast_signed())
        .map(|ts| format::date(t, ts))
        .unwrap_or_default();
    let name = if metadata.database_type.starts_with("DBIP") {
        "DB-IP Lite".to_owned()
    } else {
        metadata.database_type.clone()
    };
    GeoRow {
        label,
        name: Some(name),
        built: t.get2("settings-geo-built", "date", &built, "days", age),
        path: path.display().to_string(),
        old: age > geo::OLD_AFTER_DAYS,
    }
}

/// The newest version, as far as it is known
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Latest {
    /// Not searched for (yet)
    Unknown,
    Searching,
    /// The installed one is the newest
    Current(String),
    Newer {
        version: String,
        /// When it was published, empty if GitHub did not say
        date: String,
    },
}

/// The panel "Aktualisierungen"
pub struct Updates {
    pub installed: &'static str,
    pub latest: Latest,
    /// When openpacketmon last asked GitHub (`heute, 09:12 (beim Start)`)
    pub checked: Option<String>,
    /// The release notes of the newest version: its number and the notes
    pub notes: Option<(String, Vec<Block>)>,
}

pub fn updates(t: Lang, status: &UpdateStatus) -> Updates {
    let latest = match status {
        UpdateStatus::Unknown => Latest::Unknown,
        UpdateStatus::InProgress => Latest::Searching,
        UpdateStatus::UpToDate(release) => Latest::Current(release.version.clone()),
        UpdateStatus::UpdateAvailable(release) => Latest::Newer {
            version: release.version.clone(),
            date: release
                .published
                .map(|at| format::date(t, at))
                .unwrap_or_default(),
        },
    };
    let release = status.release();
    Updates {
        installed: APP_VERSION,
        latest,
        checked: release.map(|r| checked(t, r.checked, jiff::Timestamp::now())),
        notes: release
            .map(|r| (r.version.clone(), release_notes::blocks(&r.notes)))
            .filter(|(_, blocks)| !blocks.is_empty()),
    }
}

/// When the check ran: `heute, 09:12 (beim Start)` or with the day
fn checked(t: Lang, at: jiff::Timestamp, now: jiff::Timestamp) -> String {
    let zone = jiff::tz::TimeZone::system();
    let (at, now) = (at.to_zoned(zone.clone()), now.to_zoned(zone));
    let day = if at.date() == now.date() {
        t.get("settings-today")
    } else {
        format::date(t, at.timestamp())
    };
    let time = at.strftime("%H:%M").to_string();
    t.get2("settings-checked-at", "day", &day, "time", &time)
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
    /// Where the databases of DB-IP are put, the home folder as `~`
    pub geo_dir: String,
    pub update_checks: bool,
    pub updates: Updates,
    pub packets_enabled: bool,
    pub packets_max: usize,
    pub packets_headers: bool,
    pub calls_rtp: bool,
    pub calls_sip_any: bool,
    /// When the note on § 201 of the criminal code was confirmed, as written in the language; empty: not yet
    pub calls_law: String,
}

pub fn settings(t: Lang, engine: &EngineState) -> Settings {
    let conf = &engine.conf;
    let notifications = &conf.settings.notifications;
    let mut languages = vec![Choice {
        value: "",
        label: t.get("gui-language-system"),
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
            .map(|dir| home_as_tilde(&dir))
            .unwrap_or_default(),
        update_checks: !conf.updates.disable_checks(),
        updates: updates(t, &engine.update_status),
        packets_enabled: conf.packet_store.enabled,
        packets_max: conf.packet_store.max_packets(),
        packets_headers: conf.packet_store.headers_only,
        calls_rtp: conf.settings.calls.rtp_without_sip,
        calls_sip_any: conf.settings.calls.sip_any_port,
        calls_law: law_date(t, &conf.settings.calls.law_confirmed),
    }
}

/// The day of a confirmation (`2026-11-10`) as written in the language (`10.11.2026`)
fn law_date(t: Lang, day: &str) -> String {
    match day.parse::<jiff::civil::Date>() {
        Ok(date) => date
            .strftime(match t {
                Lang::De => "%d.%m.%Y",
                Lang::En => "%Y-%m-%d",
            })
            .to_string(),
        Err(_) => String::new(),
    }
}

/// A folder with the home folder written as `~` (`~/.local/share/openpacketmon/geo`)
fn home_as_tilde(dir: &std::path::Path) -> String {
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").filter(|h| !h.is_empty())
        && let Ok(rest) = dir.strip_prefix(&home)
    {
        return std::path::Path::new("~").join(rest).display().to_string();
    }
    dir.display().to_string()
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
    /// Put into the range offered
    pub packets_max: usize,
    /// The memory of the packet store in MiB; `None` leaves it as it is
    pub packets_memory: Option<usize>,
    pub packets_headers: bool,
    pub calls_rtp: bool,
    pub calls_sip_any: bool,
    pub mcp_enabled: bool,
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
    conf.packet_store.max_packets = change
        .packets_max
        .clamp(PacketStoreConf::MIN_PACKETS, PacketStoreConf::MAX_PACKETS);
    if let Some(megabytes) = change.packets_memory
        && PacketStoreConf::MEGABYTES_CHOICES.contains(&megabytes)
    {
        conf.packet_store.megabytes = megabytes;
    }
    conf.packet_store.headers_only = change.packets_headers;
    conf.settings.calls.rtp_without_sip = change.calls_rtp;
    conf.settings.calls.sip_any_port = change.calls_sip_any;
    conf.settings.calls.apply();
    conf.mcp.enabled = change.mcp_enabled;

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
    use serial_test::serial;

    use super::*;
    use crate::config::conf::Conf;
    use crate::engine::Engine;

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

    fn engine() -> (tokio::runtime::Runtime, Engine) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = Engine::new(Conf::default(), runtime.handle().clone());
        (runtime, engine)
    }

    fn change() -> SettingsChange {
        SettingsChange {
            language: "de".to_owned(),
            page_size: 100,
            favorite_active: true,
            favorite_sound: Sound::Gulp,
            addresses: vec![[10, 0, 0, 2].into(), [192, 168, 1, 20].into()],
            volume: 150,
            mmdb_country: String::new(),
            mmdb_asn: String::new(),
            update_checks: false,
            packets_enabled: false,
            packets_max: 250_000,
            packets_memory: Some(2048),
            packets_headers: true,
            // the defaults: the switches act on the whole program, also on other tests
            calls_rtp: true,
            calls_sip_any: true,
            mcp_enabled: true,
        }
    }

    #[test]
    #[serial]
    fn the_settings_are_put_into_the_configuration() {
        let (_runtime, engine) = engine();
        apply_settings(&mut engine.lock(), &change());
        let state = engine.lock();
        let conf = &state.conf;
        assert_eq!(conf.language, "de");
        assert_eq!(conf.page_size, 100);
        let notifications = &conf.settings.notifications;
        assert!(notifications.favorite_notification.is_active);
        assert_eq!(notifications.favorite_notification.sound, Sound::Gulp);
        assert_eq!(notifications.volume, 100);
        assert_eq!(conf.favorites.addresses().len(), 2);
        assert!(conf.favorites.contains_address(&[192, 168, 1, 20].into()));
        assert!(conf.updates.disable_checks());
        assert!(!conf.packet_store.enabled);
        assert_eq!(conf.packet_store.max_packets, 250_000);
        assert_eq!(conf.packet_store.megabytes, 2048);
        assert!(conf.packet_store.headers_only);
    }

    #[test]
    #[serial]
    fn values_that_are_not_offered_change_nothing() {
        let (_runtime, engine) = engine();
        let (size, memory) = {
            let state = engine.lock();
            let store = state.conf.packet_store;
            (state.conf.page_size, store.megabytes)
        };
        let mut wrong = change();
        wrong.language = "xx".to_owned();
        wrong.page_size = 7;
        wrong.packets_max = 7;
        wrong.packets_memory = Some(7);
        apply_settings(&mut engine.lock(), &wrong);
        let state = engine.lock();
        assert_eq!(state.conf.language, "");
        assert_eq!(state.conf.page_size, size);
        // the number of packets is put into the range offered
        assert_eq!(state.conf.packet_store.max_packets, 1_000);
        assert_eq!(state.conf.packet_store.megabytes, memory);
        drop(state);
        wrong.packets_max = 9_000_000;
        apply_settings(&mut engine.lock(), &wrong);
        assert_eq!(engine.lock().conf.packet_store.max_packets, 2_000_000);
    }

    #[test]
    fn the_last_check_says_the_day_only_if_it_was_not_today() {
        let now: jiff::Timestamp = "2026-11-10T12:00:00Z".parse().unwrap();
        let today = checked(Lang::En, now, now);
        assert!(today.starts_with("today, "), "{today}");
        assert!(today.ends_with(" (at start)"), "{today}");
        let before = checked(Lang::De, "2026-11-01T12:00:00Z".parse().unwrap(), now);
        assert!(before.starts_with("01.11.2026, "), "{before}");
    }

    #[test]
    #[serial]
    fn addresses_that_are_gone_are_no_favorites_any_more() {
        let (_runtime, engine) = engine();
        apply_settings(&mut engine.lock(), &change());
        let mut fewer = change();
        fewer.addresses = vec![[10, 0, 0, 2].into()];
        apply_settings(&mut engine.lock(), &fewer);
        let state = engine.lock();
        assert_eq!(state.conf.favorites.addresses().len(), 1);
        assert!(
            !state
                .conf
                .favorites
                .contains_address(&[192, 168, 1, 20].into())
        );
    }
}
