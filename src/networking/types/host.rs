// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/host.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::countries::types::country::Country;
use crate::networking::types::asn::Asn;
use crate::networking::types::data_info_host::DataInfoHost;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Struct to represent a network host
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct Host {
    /// Hostname (domain). Obtained from the reverse DNS.
    pub domain: String,
    /// Autonomous System which operates the host
    pub asn: Asn,
    /// Country
    pub country: Country,
}

impl Host {
    /// Used in the host bars
    pub fn to_entry_string(&self) -> String {
        let mut ret_val = self.domain.clone();
        if !self.asn.name.is_empty() {
            ret_val.push_str(" - ");
            ret_val.push_str(&self.asn.name);
        }
        ret_val
    }
}

#[derive(Clone, Debug)]
pub struct HostMessage {
    pub host: Host,
    pub data_info_host: DataInfoHost,
    pub address_to_lookup: IpAddr,
    pub rdns: String,
}

#[cfg(test)]
mod tests {
    use crate::networking::types::asn::Asn;
    use crate::networking::types::host::Host;

    fn host_for_tests(domain: &str, asn: &str) -> Host {
        Host {
            domain: domain.to_string(),
            asn: Asn {
                name: asn.to_string(),
                code: "512".to_string(),
            },
            country: Default::default(),
        }
    }

    #[test]
    fn test_host_to_entry_string() {
        let host = host_for_tests("iphone-di-doofenshmirtz.local", "AS1234");
        assert_eq!(
            host.to_entry_string(),
            "iphone-di-doofenshmirtz.local - AS1234"
        );

        let host = host_for_tests("", "");
        assert_eq!(host.to_entry_string(), "");

        let host = host_for_tests("192.168.1.113", "AS1234");
        assert_eq!(host.to_entry_string(), "192.168.1.113 - AS1234");

        let host = host_for_tests("192.168.1.113", "");
        assert_eq!(host.to_entry_string(), "192.168.1.113");

        let host = host_for_tests("", "FASTLY");
        assert_eq!(host.to_entry_string(), " - FASTLY");

        let host = host_for_tests("::", "GOOGLE");
        assert_eq!(host.to_entry_string(), ":: - GOOGLE");

        let host = host_for_tests("::f", "AKAMAI-TECHNOLOGIES-INCORPORATED");
        assert_eq!(
            host.to_entry_string(),
            "::f - AKAMAI-TECHNOLOGIES-INCORPORATED"
        );

        let host = host_for_tests("::g", "GOOGLE");
        assert_eq!(host.to_entry_string(), "::g - GOOGLE");

        let host = host_for_tests(" ", "GOOGLE");
        assert_eq!(host.to_entry_string(), "  - GOOGLE");
    }
}
