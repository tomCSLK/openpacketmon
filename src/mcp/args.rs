//! What an assistant can pass to the tools. `openpacketmon mcp` shows it as the schema of a tool,
//! and the window reads it from the question, so the limits are checked in the window.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Longest filter accepted
pub const MAX_FILTER_CHARS: usize = 1000;
/// Packets in an answer unless the assistant asks for another number
pub const DEFAULT_PACKETS: usize = 50;
/// Most packets in an answer
pub const MAX_PACKETS: usize = 200;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct ConnectionsArgs {
    /// Connection filter, for example `port == 443 and country == US`; see `filter_help`.
    /// Empty for all connections.
    #[serde(default)]
    pub filter: String,
    /// Page of the list, from 1; the largest connections come first
    #[serde(default)]
    pub page: Option<usize>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct PacketsArgs {
    /// Display filter, for example `dns` or `tls.sni contains amazon`; see `filter_help`.
    /// Empty for all packets.
    #[serde(default)]
    pub filter: String,
    /// Connection filter that also has to match; see `filter_help` with `connections` set
    #[serde(default)]
    pub connection_filter: String,
    /// Position of the first packet among the matching ones, from 0; without it the newest packets
    #[serde(default)]
    pub offset: Option<usize>,
    /// How many packets, at most 200; 50 without it
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct PacketDetailArgs {
    /// Number of the packet, from the `packets` tool
    pub number: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct StatisticsArgs {
    /// Display filter; see `filter_help`. Empty for all packets.
    #[serde(default)]
    pub filter: String,
    /// Connection filter that also has to match; see `filter_help` with `connections` set
    #[serde(default)]
    pub connection_filter: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct FilterHelpArgs {
    /// The connection filter instead of the display filter
    #[serde(default)]
    pub connections: bool,
}
