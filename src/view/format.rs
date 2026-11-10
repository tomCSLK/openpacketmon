//! Numbers, data volumes and times as the program shows them, in the language of the interface.

use std::net::IpAddr;

use crate::networking::types::data_representation::DataUnit;
use crate::view::i18n::Lang;

/// A count with thousands separators: `12.345` (de), `12,345` (en)
pub fn count(t: Lang, n: u128) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(t.thousands_separator());
        }
        out.push(c);
    }
    out
}

/// A number with at most `decimals` decimals (trailing zeros removed)
pub fn decimal(t: Lang, v: f64, decimals: usize) -> String {
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    s.replace('.', &t.decimal_separator().to_string())
}

/// A number with exactly `decimals` decimals and thousands separators, as the draft writes it
/// (`fnum(v, dg)`): `1.234,5` (de), `1,234.5` (en)
pub fn fixed(t: Lang, v: f64, decimals: usize) -> String {
    let text = format!("{:.decimals$}", v.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut out = String::with_capacity(text.len() + whole.len() / 3 + 1);
    if v < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.push('-');
    }
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            out.push(t.thousands_separator());
        }
        out.push(c);
    }
    if !fraction.is_empty() {
        out.push(t.decimal_separator());
        out.push_str(fraction);
    }
    out
}

/// A value with three digits (`docs/entwurf/assets/data.js`, `fbytes`): two decimals below 10, one
/// below 100, none above (`7,26`, `42,2`, `177`)
fn three(t: Lang, v: f64) -> String {
    let decimals = usize::from(v < 10.0) + usize::from(v < 100.0);
    format!("{v:.decimals$}").replace('.', &t.decimal_separator().to_string())
}

/// A value in steps of 1000 with the units given, e.g. `1,50 KB`; below 1000 as a whole number
#[allow(clippy::cast_precision_loss)]
fn scaled(t: Lang, n: u128, base: &str, units: [&str; 4]) -> String {
    if n < 1000 {
        return format!("{n} {base}");
    }
    let mut v = n as f64;
    let mut unit = units[0];
    for u in units {
        v /= 1000.0;
        unit = u;
        if v < 1000.0 {
            break;
        }
    }
    format!("{} {unit}", three(t, v))
}

/// A data volume in decimal units (1 KB = 1000 B), e.g. `42,2 MB`
pub fn bytes(t: Lang, b: u128) -> String {
    scaled(t, b, "B", ["KB", "MB", "GB", "TB"])
}

/// Bits in decimal units, e.g. `12,0 Mb`
pub fn bits(t: Lang, b: u128) -> String {
    scaled(t, b, "b", ["Kb", "Mb", "Gb", "Tb"])
}

/// An amount in the unit chosen by the user: packets, bytes or bits.
/// `amount` is the value the engine reports for that unit (`DataInfo::tot_data` and friends).
pub fn amount(t: Lang, unit: DataUnit, amount: u128) -> String {
    match unit {
        DataUnit::Packets => count(t, amount),
        DataUnit::Bytes => bytes(t, amount),
        DataUnit::Bits => bits(t, amount),
    }
}

/// Civil date and time of Unix seconds in UTC: `(year, month, day, hour, minute, second)`
pub(crate) fn civil_utc(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil-from-days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rem / 3_600, (rem % 3_600) / 60, rem % 60)
}

/// Milliseconds with up to three decimals (`12.345 ms`)
pub fn millis(t: Lang, us: f64) -> String {
    let text = if us >= 100_000.0 {
        format!("{:.0} ms", us / 1000.0)
    } else if us >= 1000.0 {
        format!("{:.1} ms", us / 1000.0)
    } else {
        format!("{:.3} ms", us / 1000.0)
    };
    text.replace('.', &t.decimal_separator().to_string())
}

/// An address with a port: `192.0.2.1:443`, `[2001:db8::1]:443`, or the address alone
pub fn socket(ip: &IpAddr, port: Option<u16>) -> String {
    match (ip, port) {
        (IpAddr::V6(v6), Some(p)) => format!("[{v6}]:{p}"),
        (ip, Some(p)) => format!("{ip}:{p}"),
        (ip, None) => ip.to_string(),
    }
}

/// A MAC address as `aa:bb:cc:dd:ee:ff`
pub fn mac(mac: Option<[u8; 6]>) -> String {
    mac.map_or_else(
        || "–".to_owned(),
        |m| {
            m.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(":")
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_volumes_follow_the_language() {
        assert_eq!(count(Lang::De, 1_234_567), "1.234.567");
        assert_eq!(count(Lang::En, 1_234_567), "1,234,567");
        assert_eq!(count(Lang::En, 999), "999");
        assert_eq!(bytes(Lang::De, 512), "512 B");
        assert_eq!(bytes(Lang::De, 1_500), "1,50 KB");
        assert_eq!(bytes(Lang::De, 42_230_000), "42,2 MB");
        assert_eq!(bytes(Lang::De, 177_000), "177 KB");
        assert_eq!(bytes(Lang::En, 2_000_000), "2.00 MB");
        assert_eq!(bytes(Lang::En, 3_456_000_000), "3.46 GB");
        assert_eq!(bytes(Lang::En, 5_000_000_000_000_000), "5000 TB");
        assert_eq!(bits(Lang::En, 8_000), "8.00 Kb");
        assert_eq!(bits(Lang::De, 999), "999 b");
        assert_eq!(amount(Lang::De, DataUnit::Packets, 12_000), "12.000");
        assert_eq!(fixed(Lang::De, 1234.56, 1), "1.234,6");
        assert_eq!(fixed(Lang::En, 0.5, 3), "0.500");
        assert_eq!(fixed(Lang::De, 42.0, 0), "42");
        assert_eq!(fixed(Lang::De, -0.01, 1), "0,0");
    }

    #[test]
    fn addresses_and_times() {
        assert_eq!(
            socket(&"2001:db8::1".parse().unwrap(), Some(443)),
            "[2001:db8::1]:443"
        );
        assert_eq!(
            socket(&"192.0.2.1".parse().unwrap(), Some(53)),
            "192.0.2.1:53"
        );
        assert_eq!(socket(&"192.0.2.1".parse().unwrap(), None), "192.0.2.1");
        assert_eq!(mac(Some([0, 1, 2, 0xab, 0xcd, 0xef])), "00:01:02:ab:cd:ef");
    }
}

/// A duration as a clock: `4:05`, from an hour on `1:04:05`
pub fn clock(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// A day in the local time zone, as written in the language
pub fn date(t: Lang, at: jiff::Timestamp) -> String {
    let format = match t {
        Lang::De => "%d.%m.%Y",
        Lang::En => "%Y-%m-%d",
    };
    at.to_zoned(jiff::tz::TimeZone::system())
        .strftime(format)
        .to_string()
}
