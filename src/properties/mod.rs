//! Deterministic property-testing support.
//!
//! This module owns the local, provider-free property testing pieces. Native
//! generated-case execution is added by later bounded cycles.

#![allow(dead_code, unused_imports)] // Consumed by later DUUMBI-717 property-runner cycles.

pub mod evidence;
pub mod generator;
pub mod predicate;
pub mod runner;
pub mod shrink;
pub mod value;

pub use evidence::{
    FailureEvidence, FunctionEvidence, FunctionEvidenceStatus, PROPERTY_EVIDENCE_SCHEMA_VERSION,
    PropertyEvidence, PropertyEvidenceSettings, PropertyEvidenceSummary, UnsupportedEvidence,
};
pub use generator::{GeneratorSettings, UnsupportedGenerator, generate_values};
pub use predicate::{PredicateContext, PredicateEvalError, eval_predicate};
pub use runner::{PropertyRunOptions, PropertyRunReport, run_properties};
pub use shrink::shrink_candidates;
pub use value::PropertyValue;
