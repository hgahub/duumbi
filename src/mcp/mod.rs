//! MCP (Model Context Protocol) server for DUUMBI.
//!
//! Exposes graph query/mutation, build, deps, and intent tools
//! via JSON-RPC 2.0 over stdio transport.
//!
//! The protocol is JSON-RPC 2.0: each request is a newline-delimited JSON
//! object read from stdin; each response is written as a newline-delimited
//! JSON object to stdout.

pub mod client;
pub mod server;
pub mod tools;
