//! Deterministic property-testing support.
//!
//! The initial module surface contains only generated values and type-driven
//! generators. Predicate evaluation, execution, shrinking, evidence, and CLI
//! dispatch are added in later bounded cycles.

#![allow(dead_code, unused_imports)] // Consumed by later DUUMBI-717 property-runner cycles.

pub mod evidence;
pub mod generator;
pub mod predicate;
pub mod shrink;
pub mod value;

pub use evidence::{
    FailureEvidence, FunctionEvidence, FunctionEvidenceStatus, PROPERTY_EVIDENCE_SCHEMA_VERSION,
    PropertyEvidence, PropertyEvidenceSettings, PropertyEvidenceSummary, UnsupportedEvidence,
};
pub use generator::{GeneratorSettings, UnsupportedGenerator, generate_values};
pub use predicate::{PredicateContext, PredicateEvalError, eval_predicate};
pub use shrink::shrink_candidates;
pub use value::PropertyValue;
