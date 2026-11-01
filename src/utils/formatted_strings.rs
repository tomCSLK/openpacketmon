// Modified for openpacketmon (2026), see NOTICE. Based on `src/utils/formatted_strings.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use std::net::IpAddr;

use crate::utils::types::timestamp::Timestamp;
use jiff::tz::TimeZone;

/// Application version number (to be displayed in gui footer)
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

// /// Computes the String representing the percentage of filtered bytes/packets
// pub fn get_percentage_string(observed: u128, filtered: u128) -> String {
//     #[allow(clippy::cast_precision_loss)]
//     let filtered_float = filtered as f32;
//     #[allow(clippy::cast_precision_loss)]
//     let observed_float = observed as f32;
//     if format!("{:.1}", 100.0 * filtered_float / observed_float).eq("0.0") {
//         "<0.1%".to_string()
//     } else {
//         format!("{:.1}%", 100.0 * filtered_float / observed_float)
//     }
// }

pub fn get_domain_from_r_dns(r_dns: String) -> String {
    if r_dns.parse::<IpAddr>().is_ok() || r_dns.is_empty() {
        // rDNS is equal to the corresponding IP address (can't be empty but checking it to be safe)
        r_dns
    } else {
        let parts: Vec<&str> = r_dns.split('.').collect();
        let len = parts.len();
        if len >= 2 {
            let last = parts.get(len - 1).unwrap_or(&"");
            let second_last = parts.get(len - 2).unwrap_or(&"");
            if last.len() > 3 || second_last.len() > 3 {
                format!("{second_last}.{last}")
            } else {
                let third_last_opt = len.checked_sub(3).and_then(|i| parts.get(i));
                match third_last_opt {
                    Some(third_last) => format!("{third_last}.{second_last}.{last}"),
                    None => format!("{second_last}.{last}"),
                }
            }
        } else {
            r_dns
        }
    }
}

pub fn get_formatted_timestamp(t: Timestamp) -> String {
    let date_opt = t
        .to_usecs()
        .and_then(|usecs| jiff::Timestamp::from_microsecond(usecs).ok())
        .map(|ts| TimeZone::system().to_datetime(ts));
    if let Some(date) = date_opt {
        date.strftime("%Y/%m/%d %H:%M:%S").to_string()
    } else {
        "?".to_string()
    }
}

#[allow(dead_code)]
#[cfg(windows)]
pub fn get_logs_file_path() -> Option<String> {
    let mut conf = confy::get_configuration_file_path(crate::APP_NAME, "logs").ok()?;
    conf.set_extension("txt");
    Some(conf.to_str()?.to_string())
}

#[cfg(all(windows, not(debug_assertions)))]
pub fn redirect_stdout_stderr_to_file()
-> Option<(gag::Redirect<std::fs::File>, gag::Redirect<std::fs::File>)> {
    if let Ok(logs_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_logs_file_path()?)
    {
        return Some((
            gag::Redirect::stdout(logs_file.try_clone().ok()?).ok()?,
            gag::Redirect::stderr(logs_file).ok()?,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_formatted_timestamp() {
        // 2023/11/14 22:13:20 UTC: the rendered day is 14 or 15 depending on the system time zone
        let formatted = get_formatted_timestamp(Timestamp::new(1_700_000_000, 123_456));
        assert!(formatted.starts_with("2023/11/1"), "{formatted}");
        assert!(
            formatted.chars().enumerate().all(|(i, c)| match i {
                4 | 7 => c == '/',
                10 => c == ' ',
                13 | 16 => c == ':',
                _ => c.is_ascii_digit(),
            }) && formatted.len() == 19,
            "{formatted}"
        );

        // microseconds overflow
        assert_eq!(get_formatted_timestamp(Timestamp::new(i64::MAX, 0)), "?");
        assert_eq!(get_formatted_timestamp(Timestamp::new(i64::MIN, 0)), "?");
        // out of the range of dates that can be represented
        assert_eq!(
            get_formatted_timestamp(Timestamp::new(1_800_000_000_000, 0)),
            "?"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_logs_file_path() {
        let file_path = std::path::PathBuf::from(get_logs_file_path().unwrap());
        assert!(file_path.is_absolute());
        assert_eq!(file_path.file_name().unwrap(), "logs.txt");
    }

    #[test]
    fn test_get_domain_from_r_dns() {
        let f = |s: &str| get_domain_from_r_dns(s.to_string());
        assert_eq!(f(""), "");
        assert_eq!(f("8.8.8.8"), "8.8.8.8");
        assert_eq!(f("a.b.c.d"), "b.c.d");
        assert_eq!(f("ciao.xyz"), "ciao.xyz");
        assert_eq!(f("bye.ciao.xyz"), "ciao.xyz");
        assert_eq!(f("ciao.bye.xyz"), "ciao.bye.xyz");
        assert_eq!(f("hola.ciao.bye.xyz"), "ciao.bye.xyz");
        assert_eq!(f(".bye.xyz"), ".bye.xyz");
        assert_eq!(f("bye.xyz"), "bye.xyz");
        assert_eq!(f("hola.ciao.b"), "ciao.b");
        assert_eq!(f("hola.b.ciao"), "b.ciao");
        assert_eq!(f("ciao."), "ciao.");
        assert_eq!(f("ciao.."), "ciao..");
        assert_eq!(f(".ciao."), "ciao.");
        assert_eq!(f("ciao.bye."), "ciao.bye.");
        assert_eq!(f("ciao..."), "..");
        assert_eq!(f("..bye"), "..bye");
        assert_eq!(f("ciao..bye"), "ciao..bye");
        assert_eq!(f("..ciao"), ".ciao");
        assert_eq!(f("bye..ciao"), ".ciao");
        assert_eq!(f("."), ".");
        assert_eq!(f(".."), "..");
        assert_eq!(f("..."), "..");
        assert_eq!(f("no_dots_in_this"), "no_dots_in_this");
    }
}
