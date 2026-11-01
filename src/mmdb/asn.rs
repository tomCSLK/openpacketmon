// Modified for openpacketmon (2026), see NOTICE. Based on `src/mmdb/asn.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::mmdb::types::mmdb_asn_entry::MmdbAsnEntry;
use crate::mmdb::types::mmdb_reader::MmdbReader;
use crate::networking::types::asn::Asn;
use std::net::IpAddr;

#[allow(clippy::module_name_repetitions)]
pub fn get_asn(address: &IpAddr, asn_db_reader: &MmdbReader) -> Asn {
    if let Some(res) = asn_db_reader.lookup::<MmdbAsnEntry>(*address) {
        return res.get_asn();
    }
    Asn::default()
}

#[cfg(test)]
mod tests {
    use crate::mmdb::asn::get_asn;
    use crate::mmdb::types::mmdb_asn_entry::MmdbAsnEntry;
    use crate::mmdb::types::mmdb_reader::MmdbReader;
    use std::net::IpAddr;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn test_get_asn_without_database() {
        assert!(MmdbReader::open(Path::new("unknown path")).is_none());
        assert!(MmdbReader::open(Path::new("")).is_none());
        assert!(MmdbReader::open(Path::new("Cargo.toml")).is_none());
        let res = get_asn(&IpAddr::from([8, 8, 8, 8]), &MmdbReader::Empty);
        assert_eq!(res.code, "");
        assert_eq!(res.name, "");
    }

    #[test]
    fn test_asn_entry_of_maxmind_and_dbip() {
        // GeoLite2-ASN and DB-IP "IP to ASN Lite" (compatible with GeoLite2-ASN) use these fields
        let entry: MmdbAsnEntry = serde_json::from_str(
            r#"{"autonomous_system_number":15169,"autonomous_system_organization":"Google LLC"}"#,
        )
        .unwrap();
        let asn = entry.get_asn();
        assert_eq!(asn.code, "15169");
        assert_eq!(asn.name, "Google LLC");
    }

    #[test]
    fn test_get_asn_with_custom_ipinfo_single_reader() {
        let reader = MmdbReader::open(Path::new("resources/test/ipinfo_asn_sample.mmdb")).unwrap();

        // known IP
        let res = get_asn(&IpAddr::from([185, 72, 2, 28]), &reader);
        assert_eq!(res.code, "AS202583");
        assert_eq!(res.name, "AVATEL TELECOM, SA");

        // another known IP
        let res = get_asn(&IpAddr::from([89, 187, 198, 0]), &reader);
        assert_eq!(res.code, "AS210367");
        assert_eq!(res.name, "Krajska zdravotni, a.s.");

        // known IPv6
        let res = get_asn(&IpAddr::from_str("2408:8957:6280::").unwrap(), &reader);
        assert_eq!(res.code, "AS17622");
        assert_eq!(res.name, "China Unicom Guangzhou network");

        // unknown IP
        let res = get_asn(&IpAddr::from([127, 0, 0, 1]), &reader);
        assert_eq!(res.code, "");
        assert_eq!(res.name, "");

        // unknown IPv6
        let res = get_asn(&IpAddr::from_str("::1").unwrap(), &reader);
        assert_eq!(res.code, "");
        assert_eq!(res.name, "");
    }

    #[test]
    fn test_get_asn_with_custom_ipinfo_combined_reader() {
        let reader = MmdbReader::open(Path::new("resources/test/ipinfo_lite_sample.mmdb")).unwrap();

        // known IP
        let res = get_asn(&IpAddr::from([1, 0, 65, 1]), &reader);
        assert_eq!(res.code, "AS18144");
        assert_eq!(res.name, "Enecom,Inc.");

        // another known IP
        let res = get_asn(&IpAddr::from([1, 6, 230, 0]), &reader);
        assert_eq!(res.code, "AS4755");
        assert_eq!(res.name, "TATA Communications formerly VSNL is Leading ISP");

        // unknown IP
        let res = get_asn(&IpAddr::from([127, 0, 0, 1]), &reader);
        assert_eq!(res.code, "");
        assert_eq!(res.name, "");

        // unknown IPv6
        let res = get_asn(&IpAddr::from_str("::1").unwrap(), &reader);
        assert_eq!(res.code, "");
        assert_eq!(res.name, "");
    }
}
