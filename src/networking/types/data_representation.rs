// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/data_representation.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::config::conf::deserialize_or_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DataRepr {
    #[serde(deserialize_with = "deserialize_or_default")]
    pub(crate) data_unit: DataUnit,
    #[serde(deserialize_with = "deserialize_or_default")]
    pub(crate) per_second: bool,
}

impl DataRepr {
    pub fn packets(per_second: bool) -> DataRepr {
        DataRepr {
            data_unit: DataUnit::Packets,
            per_second,
        }
    }

    pub fn bytes(per_second: bool) -> DataRepr {
        DataRepr {
            data_unit: DataUnit::Bytes,
            per_second,
        }
    }

    #[cfg(test)]
    pub fn bits(per_second: bool) -> DataRepr {
        DataRepr {
            data_unit: DataUnit::Bits,
            per_second,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DataUnit {
    Packets,
    #[default]
    Bytes,
    Bits,
}
