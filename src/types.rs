//! Core types shared across all duumbi modules.
//!
//! Defines newtypes for identifiers, the Op enum for Phase 0 operations,
//! and the `DuumbiType` representation.

use std::fmt;

/// Unique identifier for a node in the semantic graph.
///
/// Wraps the `@id` string from JSON-LD (e.g. `"duumbi:main/main/entry/0"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Unique identifier for an edge in the semantic graph.
#[allow(dead_code)] // Will be used in future phases for edge tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeId(pub String);

/// Label for a basic block within a function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockLabel(pub String);

impl fmt::Display for BlockLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Name of a function in the semantic graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionName(pub String);

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Name of a module in the semantic graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleName(pub String);

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Phase 0 operation types.
///
/// Each variant corresponds to a `duumbi:` prefixed `@type` in JSON-LD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Integer constant: `duumbi:Const`
    Const(i64),
    /// Addition: `duumbi:Add`
    Add,
    /// Subtraction: `duumbi:Sub`
    Sub,
    /// Multiplication: `duumbi:Mul`
    Mul,
    /// Division: `duumbi:Div`
    Div,
    /// Print value to stdout: `duumbi:Print`
    Print,
    /// Return value from function: `duumbi:Return`
    Return,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Const(v) => write!(f, "Const({v})"),
            Op::Add => f.write_str("Add"),
            Op::Sub => f.write_str("Sub"),
            Op::Mul => f.write_str("Mul"),
            Op::Div => f.write_str("Div"),
            Op::Print => f.write_str("Print"),
            Op::Return => f.write_str("Return"),
        }
    }
}

/// Type in the duumbi type system.
///
/// Phase 0 supports only `I64`. Future phases add `F64`, `Bool`, `Void`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuumbiType {
    /// 64-bit signed integer.
    I64,
    /// No return value (used by Print).
    Void,
}

impl fmt::Display for DuumbiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuumbiType::I64 => f.write_str("i64"),
            DuumbiType::Void => f.write_str("void"),
        }
    }
}
