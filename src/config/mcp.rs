//! Settings of the access for AI assistants (see `crate::mcp`)

use serde::{Deserialize, Serialize};

use crate::config::conf::deserialize_or_default;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(default)]
pub struct McpConf {
    /// Whether the window answers AI assistants; off unless the user turns it on
    #[serde(deserialize_with = "deserialize_or_default")]
    pub enabled: bool,
}
