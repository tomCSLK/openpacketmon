//! Access for AI assistants over the Model Context Protocol.
//!
//! An assistant starts `openpacketmon mcp` ([`server`]), which speaks MCP on its standard streams
//! and holds no data. For every question it connects to the open window ([`access`]) on
//! `127.0.0.1`, and both prove that they know a token from a file only this user can read,
//! without sending it ([`protocol`]). Such a connection carries a hello, a challenge, one short
//! JSON question and its answer, so nothing in the window has to understand MCP, and the window
//! can open and close at any time.
//!
//! The access is off unless the user turns it on in the settings. What an assistant can ask is
//! all in [`tools`], and it only reads.

pub mod access;
mod args;
mod privacy;
mod protocol;
pub mod server;
mod tools;

pub use access::{Access, Status, access_path};
