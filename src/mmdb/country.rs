// Modified for openpacketmon (2026), see NOTICE. Based on `src/mmdb/country.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::countries::types::country::Country;
use crate::mmdb::types::mmdb_country_entry::MmdbCountryEntry;
use crate::mmdb::types::mmdb_reader::MmdbReader;
use std::net::IpAddr;

#[allow(clippy::module_name_repetitions)]
pub fn get_country(address: &IpAddr, country_db_reader: &MmdbReader) -> Country {
    if let Some(res) = country_db_reader.lookup::<MmdbCountryEntry>(*address) {
        return res.get_country();
    }
    Country::ZZ // unknown
}

#[cfg(test)]
mod tests {
    use crate::countries::types::country::Country;
    use crate::mmdb::country::get_country;
    use crate::mmdb::types::mmdb_country_entry::MmdbCountryEntry;
    use crate::mmdb::types::mmdb_reader::MmdbReader;
    use std::net::IpAddr;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn test_get_country_without_database() {
        assert!(MmdbReader::open(Path::new("unknown path")).is_none());
        assert!(MmdbReader::open(Path::new("static/favicon.svg")).is_none());
        let res = get_country(&IpAddr::from([8, 8, 8, 8]), &MmdbReader::Empty);
        assert_eq!(res, Country::ZZ);
    }

    #[test]
    fn test_country_entry_of_maxmind_and_dbip() {
        // GeoLite2-Country and DB-IP "IP to Country Lite" use this structure
        let entry: MmdbCountryEntry = serde_json::from_str(
            r#"{"continent":{"code":"EU"},"country":{"iso_code":"DE","names":{"en":"Germany"}}}"#,
        )
        .unwrap();
        assert_eq!(entry.get_country(), Country::DE);
    }

    #[test]
    fn test_get_country_with_custom_ipinfo_single_reader() {
        let reader =
            MmdbReader::open(Path::new("resources/test/ipinfo_location_sample.mmdb")).unwrap();

        // known IP
        let res = get_country(&IpAddr::from([1, 0, 6, 99]), &reader);
        assert_eq!(res, Country::AU);

        // another known IP
        let res = get_country(&IpAddr::from([1, 0, 8, 0]), &reader);
        assert_eq!(res, Country::CN);

        // unknown IP
        let res = get_country(&IpAddr::from([127, 0, 0, 1]), &reader);
        assert_eq!(res, Country::ZZ);

        // unknown IPv6
        let res = get_country(&IpAddr::from_str("::1").unwrap(), &reader);
        assert_eq!(res, Country::ZZ);
    }

    #[test]
    fn test_get_country_with_custom_ipinfo_combined_reader() {
        let reader = MmdbReader::open(Path::new("resources/test/ipinfo_lite_sample.mmdb")).unwrap();

        // known IP
        let res = get_country(&IpAddr::from([1, 0, 65, 1]), &reader);
        assert_eq!(res, Country::JP);

        // another known IP
        let res = get_country(&IpAddr::from([1, 6, 230, 0]), &reader);
        assert_eq!(res, Country::IN);

        // unknown IP
        let res = get_country(&IpAddr::from([127, 0, 0, 1]), &reader);
        assert_eq!(res, Country::ZZ);

        // unknown IPv6
        let res = get_country(&IpAddr::from_str("::1").unwrap(), &reader);
        assert_eq!(res, Country::ZZ);
    }
}
