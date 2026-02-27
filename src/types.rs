//! Core types shared across all duumbi modules.
//!
//! Defines newtypes for identifiers, the Op enum for operations,
//! `CompareOp` for comparison operators, and the `DuumbiType` representation.

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

/// Comparison operator for `Compare` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants used in Phase 1 parser and compiler
pub enum CompareOp {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Eq => f.write_str("eq"),
            CompareOp::Ne => f.write_str("ne"),
            CompareOp::Lt => f.write_str("lt"),
            CompareOp::Le => f.write_str("le"),
            CompareOp::Gt => f.write_str("gt"),
            CompareOp::Ge => f.write_str("ge"),
        }
    }
}

/// Operation types in the duumbi instruction set.
///
/// Each variant corresponds to a `duumbi:` prefixed `@type` in JSON-LD.
/// Phase 0 ops: Const, Add, Sub, Mul, Div, Print, Return.
/// Phase 1 ops: ConstF64, ConstBool, Compare, Branch, Call, Load, Store.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Phase 1 variants used once parser/compiler are extended
pub enum Op {
    /// Integer constant: `duumbi:Const` with `resultType: "i64"`.
    Const(i64),
    /// Float constant: `duumbi:Const` with `resultType: "f64"`.
    ConstF64(f64),
    /// Boolean constant: `duumbi:Const` with `resultType: "bool"`.
    ConstBool(bool),
    /// Addition: `duumbi:Add`
    Add,
    /// Subtraction: `duumbi:Sub`
    Sub,
    /// Multiplication: `duumbi:Mul`
    Mul,
    /// Division: `duumbi:Div`
    Div,
    /// Comparison: `duumbi:Compare`
    Compare(CompareOp),
    /// Conditional branch: `duumbi:Branch`
    Branch,
    /// Function call: `duumbi:Call`
    Call {
        /// Name of the function to call.
        function: String,
    },
    /// Load variable: `duumbi:Load`
    Load {
        /// Name of the variable to load.
        variable: String,
    },
    /// Store variable: `duumbi:Store`
    Store {
        /// Name of the variable to store into.
        variable: String,
    },
    /// Print value to stdout: `duumbi:Print`
    Print,
    /// Return value from function: `duumbi:Return`
    Return,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Const(v) => write!(f, "Const({v})"),
            Op::ConstF64(v) => write!(f, "ConstF64({v})"),
            Op::ConstBool(v) => write!(f, "ConstBool({v})"),
            Op::Add => f.write_str("Add"),
            Op::Sub => f.write_str("Sub"),
            Op::Mul => f.write_str("Mul"),
            Op::Div => f.write_str("Div"),
            Op::Compare(op) => write!(f, "Compare({op})"),
            Op::Branch => f.write_str("Branch"),
            Op::Call { function } => write!(f, "Call({function})"),
            Op::Load { variable } => write!(f, "Load({variable})"),
            Op::Store { variable } => write!(f, "Store({variable})"),
            Op::Print => f.write_str("Print"),
            Op::Return => f.write_str("Return"),
        }
    }
}

/// Type in the duumbi type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Phase 1 variants used once parser/compiler are extended
pub enum DuumbiType {
    /// 64-bit signed integer.
    I64,
    /// 64-bit floating point.
    F64,
    /// Boolean (true/false).
    Bool,
    /// No return value (used by Print).
    Void,
}

impl fmt::Display for DuumbiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuumbiType::I64 => f.write_str("i64"),
            DuumbiType::F64 => f.write_str("f64"),
            DuumbiType::Bool => f.write_str("bool"),
            DuumbiType::Void => f.write_str("void"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_op_display() {
        assert_eq!(CompareOp::Eq.to_string(), "eq");
        assert_eq!(CompareOp::Ne.to_string(), "ne");
        assert_eq!(CompareOp::Lt.to_string(), "lt");
        assert_eq!(CompareOp::Le.to_string(), "le");
        assert_eq!(CompareOp::Gt.to_string(), "gt");
        assert_eq!(CompareOp::Ge.to_string(), "ge");
    }

    #[test]
    fn op_display_phase1_variants() {
        assert_eq!(Op::ConstF64(2.5).to_string(), "ConstF64(2.5)");
        assert_eq!(Op::ConstBool(true).to_string(), "ConstBool(true)");
        assert_eq!(Op::Compare(CompareOp::Lt).to_string(), "Compare(lt)");
        assert_eq!(Op::Branch.to_string(), "Branch");
        assert_eq!(
            Op::Call {
                function: "fib".to_string()
            }
            .to_string(),
            "Call(fib)"
        );
        assert_eq!(
            Op::Load {
                variable: "x".to_string()
            }
            .to_string(),
            "Load(x)"
        );
        assert_eq!(
            Op::Store {
                variable: "x".to_string()
            }
            .to_string(),
            "Store(x)"
        );
    }

    #[test]
    fn duumbi_type_display_phase1() {
        assert_eq!(DuumbiType::F64.to_string(), "f64");
        assert_eq!(DuumbiType::Bool.to_string(), "bool");
    }
}
