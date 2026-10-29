// Modified for openpacketmon (2026), see NOTICE. Based on `src/report/types/sort_type.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SortType {
    Ascending,
    Descending,
    #[default]
    Neutral,
}

impl SortType {}
