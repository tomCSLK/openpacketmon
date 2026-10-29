// Modified for openpacketmon (2026), see NOTICE. Based on `src/gui/types/favorite.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::config::conf::deserialize_or_default;
use crate::countries::types::country::Country;
use crate::filter::quote;
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_info_host::DataInfoHost;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::host::Host;
use crate::networking::types::info_traffic::InfoTraffic;
use crate::networking::types::program::Program;
use crate::networking::types::program_lookup::ProgramLookup;
use crate::networking::types::service::Service;
use crate::report::types::sort_type::SortType;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::collections::HashSet;
use std::net::IpAddr;

/// Most favorite addresses (they can be typed in, unlike the other favorites)
pub const MAX_ADDRESSES: usize = 1000;

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Favorites {
    #[serde(deserialize_with = "deserialize_or_default")]
    hosts: HashSet<Host>,
    #[serde(deserialize_with = "deserialize_or_default")]
    programs: HashSet<Program>,
    /// Single IP addresses (an old `services` list of the configuration is ignored)
    #[serde(deserialize_with = "deserialize_or_default")]
    addresses: HashSet<IpAddr>,
}

impl Favorites {
    pub fn contains_host(&self, host: &Host) -> bool {
        self.hosts.contains(host)
    }

    pub fn contains_program(&self, program: &Program) -> bool {
        self.programs.contains(program)
    }

    pub fn contains_address(&self, address: &IpAddr) -> bool {
        self.addresses.contains(address)
    }

    pub fn insert(&mut self, key: &FavoriteKey) {
        match key {
            FavoriteKey::Host(h) => {
                self.hosts.insert(h.clone());
            }
            FavoriteKey::Program(p) => {
                self.programs.insert(p.clone());
            }
            FavoriteKey::Address(a) => {
                if self.addresses.len() < MAX_ADDRESSES {
                    self.addresses.insert(*a);
                }
            }
        }
    }

    pub fn remove(&mut self, key: &FavoriteKey) {
        match key {
            FavoriteKey::Host(h) => {
                self.hosts.remove(h);
            }
            FavoriteKey::Program(p) => {
                self.programs.remove(p);
            }
            FavoriteKey::Address(a) => {
                self.addresses.remove(a);
            }
        }
    }

    pub fn hosts(&self) -> &HashSet<Host> {
        &self.hosts
    }

    pub fn programs(&self) -> &HashSet<Program> {
        &self.programs
    }

    pub fn addresses(&self) -> &HashSet<IpAddr> {
        &self.addresses
    }
}

#[cfg(test)]
impl<const N: usize> From<[FavoriteKey; N]> for Favorites {
    fn from(keys: [FavoriteKey; N]) -> Self {
        let mut favorites = Favorites::default();
        for key in keys {
            favorites.insert(&key);
        }
        favorites
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Favorite {
    Host,
    Service,
    Program,
}

impl Favorite {
    /// Entries of this kind with their traffic, sorted; only favorites if `favorites_filter` is set
    pub fn get_entries(
        self,
        info_traffic: &InfoTraffic,
        program_lookup: Option<&ProgramLookup>,
        favorites: &Favorites,
        data_repr: DataRepr,
        sort_type: SortType,
        favorites_filter: bool,
    ) -> Vec<FavoriteItem> {
        match self {
            Favorite::Host => get_host_entries(
                info_traffic,
                data_repr,
                sort_type,
                favorites_filter,
                &favorites.hosts,
            ),
            // services cannot be favorites
            Favorite::Service => {
                if favorites_filter {
                    Vec::new()
                } else {
                    get_service_entries(info_traffic, data_repr, sort_type)
                }
            }
            Favorite::Program => get_program_entries(
                program_lookup,
                data_repr,
                sort_type,
                favorites_filter,
                &favorites.programs,
            ),
        }
    }
}

#[derive(Clone)]
pub enum FavoriteItem {
    Host((Host, DataInfoHost)),
    Service((Service, DataInfo)),
    Program((Program, DataInfo)),
    /// Only in notifications: the traffic of a favorite address
    Address((IpAddr, DataInfo)),
}

impl FavoriteItem {
    pub fn data_info(&self) -> DataInfo {
        match self {
            FavoriteItem::Host((_, data_info_host)) => data_info_host.data_info,
            FavoriteItem::Service((_, data_info))
            | FavoriteItem::Program((_, data_info))
            | FavoriteItem::Address((_, data_info)) => *data_info,
        }
    }

    pub fn to_entry_string(&self) -> String {
        match self {
            FavoriteItem::Host((host, _)) => host.to_entry_string(),
            FavoriteItem::Service((service, _)) => service.to_string(),
            FavoriteItem::Program((program, _)) => program.to_string(),
            FavoriteItem::Address((address, _)) => address.to_string(),
        }
    }

    /// The connection filter that shows the connections of this entry
    pub fn connection_filter(&self) -> String {
        match self {
            FavoriteItem::Host((host, _)) => {
                let mut parts = Vec::new();
                if !host.domain.is_empty() {
                    parts.push(format!("domain == {}", quote(&host.domain)));
                }
                if !host.asn.name.is_empty() {
                    parts.push(format!("as.name == {}", quote(&host.asn.name)));
                }
                if host.country != Country::ZZ {
                    parts.push(format!("country == {}", host.country));
                }
                parts.join(" and ")
            }
            FavoriteItem::Service((service, _)) => {
                format!("service == {}", quote(&service.to_string()))
            }
            FavoriteItem::Program((program, _)) => {
                format!("program == {}", quote(&program.to_string()))
            }
            FavoriteItem::Address((address, _)) => format!("ip.addr == {address}"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum FavoriteKey {
    Host(Host),
    Program(Program),
    Address(IpAddr),
}

// get entries helpers -----------------------------------------------------------------------------

fn get_host_entries(
    info_traffic: &InfoTraffic,
    data_repr: DataRepr,
    sort_type: SortType,
    favorites_filter: bool,
    favorites: &HashSet<Host>,
) -> Vec<FavoriteItem> {
    let default_data_info_host = DataInfoHost::default();
    let mut sorted_vec: Vec<(&Host, &DataInfoHost)> = if favorites_filter {
        favorites
            .iter()
            .map(|host| {
                let data_info_host = info_traffic
                    .hosts
                    .get(host)
                    .unwrap_or(&default_data_info_host);
                (host, data_info_host)
            })
            .collect()
    } else {
        info_traffic.hosts.iter().collect()
    };

    sorted_vec.sort_by(|&(_, a), &(_, b)| a.data_info.compare(&b.data_info, sort_type, data_repr));

    let n_entry = min(sorted_vec.len(), 30);
    sorted_vec[0..n_entry]
        .iter()
        .map(|&(host, data_info_host)| {
            FavoriteItem::Host((host.to_owned(), data_info_host.to_owned()))
        })
        .collect()
}

fn get_service_entries(
    info_traffic: &InfoTraffic,
    data_repr: DataRepr,
    sort_type: SortType,
) -> Vec<FavoriteItem> {
    let mut sorted_vec: Vec<(&Service, &DataInfo)> = info_traffic
        .services
        .iter()
        .filter(|(service, _)| service != &&Service::NotApplicable)
        .collect();

    sorted_vec.sort_by(|&(_, a), &(_, b)| a.compare(b, sort_type, data_repr));

    let n_entry = min(sorted_vec.len(), 30);
    sorted_vec[0..n_entry]
        .iter()
        .map(|&(service, data_info)| FavoriteItem::Service((*service, *data_info)))
        .collect()
}

fn get_program_entries(
    program_lookup: Option<&ProgramLookup>,
    data_repr: DataRepr,
    sort_type: SortType,
    favorites_filter: bool,
    favorites: &HashSet<Program>,
) -> Vec<FavoriteItem> {
    let Some(program_lookup) = program_lookup else {
        return Vec::new();
    };

    let default_data_info = DataInfo::default();
    let mut sorted_vec: Vec<(&Program, &DataInfo)> = if favorites_filter {
        favorites
            .iter()
            .map(|program| {
                let data_info = program_lookup
                    .programs()
                    .get(program)
                    .unwrap_or(&default_data_info);
                (program, data_info)
            })
            .collect()
    } else {
        program_lookup
            .programs()
            .iter()
            // Unknown may be inserted, and then all of its data could be reassigned to known programs
            .filter(|(_, d)| d.tot_data(DataRepr::packets(false)) > 0)
            .collect()
    };

    sorted_vec.sort_by(|&(p1, a), &(p2, b)| {
        if sort_type == SortType::Neutral
            && a.tot_data(data_repr) > 0
            && b.tot_data(data_repr) > 0
            && a.is_within_same_second(b)
        {
            if p1.is_unknown() {
                return std::cmp::Ordering::Greater;
            } else if p2.is_unknown() {
                return std::cmp::Ordering::Less;
            }
        }
        a.compare(b, sort_type, data_repr)
    });

    let n_entry = min(sorted_vec.len(), 30);
    sorted_vec[0..n_entry]
        .iter()
        .map(|&(program, data_info)| FavoriteItem::Program((program.to_owned(), *data_info)))
        .collect()
}
