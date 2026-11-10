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

/// How the content of a stream is shown
#[derive(Serialize, Deserialize, JsonSchema, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "lowercase")]
pub enum StreamFormat {
    /// Printable characters, other bytes as `·`
    #[default]
    Text,
    /// Hex dump with the printable characters next to it
    Hex,
}

/// Which directions of a stream are shown
#[derive(Serialize, Deserialize, JsonSchema, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "lowercase")]
pub enum StreamDirection {
    #[default]
    Both,
    /// What the side that started the connection sent
    Client,
    /// What the other side sent
    Server,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct FollowStreamArgs {
    /// Number of any packet of the connection, from the `packets` tool
    pub number: u64,
    /// `text` (the default) or `hex`
    #[serde(default)]
    pub format: StreamFormat,
    /// `both` (the default), `client` or `server`
    #[serde(default)]
    pub direction: StreamDirection,
    /// Bytes of the shown content to skip, from `next_offset` of the answer before
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct PacketBytesArgs {
    /// Number of the packet, from the `packets` tool
    pub number: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[schemars(crate = "rmcp::schemars")]
pub struct CallsArgs {
    /// Id of a call from the list (`sip-3`, `rtp-7`) for its details; without it the list
    #[serde(default)]
    pub call: Option<String>,
}
