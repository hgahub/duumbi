//! Duumbi library crate.
//!
//! Exposes internal modules for integration tests and external tooling.
//! The binary entry point is `main.rs`.
//!
//! The compiler and CLI modules are binary-only and live in `main.rs`.

pub mod agents;
pub mod config;
pub mod errors;
pub mod graph;
pub mod parser;
pub mod patch;
pub mod snapshot;
pub mod tools;
pub mod types;
pub mod web;
