//! Duumbi library crate.
//!
//! Exposes internal modules for integration tests and external tooling.
//! The binary entry point is `main.rs`.
//!
//! The compiler and CLI modules are binary-only and live in `main.rs`.

pub mod agents;
pub mod compiler;
pub mod config;
pub mod deps;
pub mod errors;
pub mod examples;
pub mod graph;
pub mod hash;
pub mod intent;
pub mod manifest;
pub mod parser;
pub mod patch;
pub mod registry;
pub mod snapshot;
pub mod tools;
pub mod types;
