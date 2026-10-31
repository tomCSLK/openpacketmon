//! Numbers, data volumes and times as the program shows them, in the language of the interface.
//! `static/format.js` formats the live values of the browser pages the same way.

use std::net::IpAddr;

use crate::networking::types::data_representation::DataUnit;
use crate::utils::types::timestamp::Timestamp;
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
fn decimal(t: Lang, v: f64, decimals: usize) -> String {
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    s.replace('.', &t.decimal_separator().to_string())
}

/// A data volume in decimal units (1 kB = 1000 B), e.g. `1,5 MB`
#[allow(clippy::cast_precision_loss)]
pub fn bytes(t: Lang, b: u128) -> String {
    let v = b as f64;
    if v >= 1e12 {
        format!("{} TB", decimal(t, v / 1e12, 2))
    } else if v >= 1e9 {
        format!("{} GB", decimal(t, v / 1e9, 2))
    } else if v >= 1e6 {
        format!("{} MB", decimal(t, v / 1e6, 2))
    } else if v >= 1e3 {
        format!("{} kB", decimal(t, v / 1e3, 1))
    } else {
        format!("{b} B")
    }
}

/// Bits in decimal units, e.g. `12 Mbit`
#[allow(clippy::cast_precision_loss)]
pub fn bits(t: Lang, b: u128) -> String {
    let v = b as f64;
    if v >= 1e12 {
        format!("{} Tbit", decimal(t, v / 1e12, 2))
    } else if v >= 1e9 {
        format!("{} Gbit", decimal(t, v / 1e9, 2))
    } else if v >= 1e6 {
        format!("{} Mbit", decimal(t, v / 1e6, 2))
    } else if v >= 1e3 {
        format!("{} kbit", decimal(t, v / 1e3, 1))
    } else {
        format!("{b} bit")
    }
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

/// `2026-06-15 19:32:10 UTC`
pub fn time_utc(ts: Timestamp) -> String {
    let (y, mo, d, h, mi, s) = civil_utc(ts.secs());
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
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
        assert_eq!(bytes(Lang::De, 1_500), "1,5 kB");
        assert_eq!(bytes(Lang::En, 2_000_000), "2 MB");
        assert_eq!(bytes(Lang::En, 3_456_000_000), "3.46 GB");
        assert_eq!(bits(Lang::En, 8_000), "8 kbit");
        assert_eq!(amount(Lang::De, DataUnit::Packets, 12_000), "12.000");
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
        assert_eq!(
            time_utc(Timestamp::new(1_781_551_930, 0)),
            "2026-06-15 19:32:10 UTC"
        );
    }
}
