//! Benchmark runner for duumbi showcases.
//!
//! Embeds 6 showcase intent specs, runs them against configured LLM providers,
//! and produces a JSON report with success rates and error breakdowns.

pub mod report;
pub mod runner;
pub mod showcases;
