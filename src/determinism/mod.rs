//! Determinism replay evidence, digests, and metrics.
//!
//! The first determinism slice is CLI-first replay measurement. This module
//! keeps the schema and low-level deterministic helpers separate from the later
//! provider-backed runner and CLI dispatch code.

pub mod digest;
pub mod evidence;
pub mod metrics;
