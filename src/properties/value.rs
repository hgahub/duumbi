//! Property-runner value model.

use std::collections::BTreeMap;

/// A generated value used by property checks.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// Signed 64-bit integer.
    I64(i64),
    /// Finite 64-bit floating-point value.
    F64(f64),
    /// Boolean value.
    Bool(bool),
    /// UTF-8 string value.
    String(String),
    /// JSON value.
    Json(serde_json::Value),
    /// Homogeneous array.
    Array(Vec<PropertyValue>),
    /// Struct value with deterministic field ordering.
    Struct {
        /// Struct type name.
        name: String,
        /// Struct fields by name.
        fields: BTreeMap<String, PropertyValue>,
    },
    /// Optional value.
    Option(Option<Box<PropertyValue>>),
    /// `Result::Ok` payload.
    ResultOk(Box<PropertyValue>),
    /// `Result::Err` payload.
    ResultErr(Box<PropertyValue>),
}

impl PropertyValue {
    /// Returns a compact type label for evidence and diagnostics.
    #[must_use]
    pub fn type_label(&self) -> &'static str {
        match self {
            PropertyValue::I64(_) => "i64",
            PropertyValue::F64(_) => "f64",
            PropertyValue::Bool(_) => "bool",
            PropertyValue::String(_) => "string",
            PropertyValue::Json(_) => "json",
            PropertyValue::Array(_) => "array",
            PropertyValue::Struct { .. } => "struct",
            PropertyValue::Option(_) => "option",
            PropertyValue::ResultOk(_) | PropertyValue::ResultErr(_) => "result",
        }
    }
}
