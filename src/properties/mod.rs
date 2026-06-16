//! Deterministic property-testing support.
//!
//! The initial module surface contains only generated values and type-driven
//! generators. Predicate evaluation, execution, shrinking, evidence, and CLI
//! dispatch are added in later bounded cycles.

#![allow(dead_code, unused_imports)] // Consumed by later DUUMBI-717 property-runner cycles.

pub mod generator;
pub mod value;

pub use generator::{GeneratorSettings, UnsupportedGenerator, generate_values};
pub use value::PropertyValue;
