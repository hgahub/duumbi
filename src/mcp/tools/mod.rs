//! MCP tool handler modules.
//!
//! Each submodule implements one category of DUUMBI tools exposed via the
//! MCP protocol. Tools operate synchronously on the workspace filesystem.

pub mod build;
pub mod deps;
pub mod graph;
pub mod intent;
