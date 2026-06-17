//! MCP tool handler modules.
//!
//! Each submodule implements one category of DUUMBI tools exposed via the
//! MCP protocol. Tools operate synchronously on the workspace filesystem.

pub mod approval;
pub mod build;
pub mod deps;
pub mod graph;
pub mod intent;
pub mod model_telemetry;
pub mod query;
pub mod rewrite;
pub mod status;
