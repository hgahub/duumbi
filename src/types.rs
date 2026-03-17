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
/// Phase 9a-1 ops: ConstString, String*, Array*, Struct*, PrintString.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Variants used as parser/compiler are extended
pub enum Op {
    /// Integer constant: `duumbi:Const` with `resultType: "i64"`.
    Const(i64),
    /// Float constant: `duumbi:Const` with `resultType: "f64"`.
    ConstF64(f64),
    /// Boolean constant: `duumbi:Const` with `resultType: "bool"`.
    ConstBool(bool),
    /// String constant: `duumbi:Const` with `resultType: "string"`.
    ConstString(String),
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
    /// Print string to stdout: `duumbi:PrintString`
    PrintString,
    /// Return value from function: `duumbi:Return`
    Return,

    // -- String operations (Phase 9a-1) --
    /// Concatenate two strings: `duumbi:StringConcat`
    StringConcat,
    /// String equality check → bool: `duumbi:StringEquals`
    StringEquals,
    /// Lexicographic string comparison: `duumbi:StringCompare`
    StringCompare(CompareOp),
    /// String length → i64: `duumbi:StringLength`
    StringLength,
    /// Substring extraction: `duumbi:StringSlice`
    StringSlice,
    /// Check if string contains substring → bool: `duumbi:StringContains`
    StringContains,
    /// Find index of substring → i64 (-1 if not found): `duumbi:StringFind`
    StringFind,
    /// Convert i64 to string: `duumbi:StringFromI64`
    StringFromI64,

    // -- Array operations (Phase 9a-1) --
    /// Create empty array: `duumbi:ArrayNew`
    ArrayNew,
    /// Append element to array: `duumbi:ArrayPush`
    ArrayPush,
    /// Get element at index (panic on OOB): `duumbi:ArrayGet`
    ArrayGet,
    /// Set element at index (panic on OOB): `duumbi:ArraySet`
    ArraySet,
    /// Array length → i64: `duumbi:ArrayLength`
    ArrayLength,
    /// Safe get → Option<T> (no panic): `duumbi:ArrayTryGet`
    ArrayTryGet,

    // -- Struct operations (Phase 9a-1) --
    /// Create new struct instance: `duumbi:StructNew`
    StructNew {
        /// Name of the struct type to instantiate.
        struct_name: String,
    },
    /// Get field value from struct: `duumbi:FieldGet`
    FieldGet {
        /// Name of the field to read.
        field_name: String,
    },
    /// Set field value on struct: `duumbi:FieldSet`
    FieldSet {
        /// Name of the field to write.
        field_name: String,
    },

    // -- Ownership operations (Phase 9a-2) --
    /// Allocate heap value: `duumbi:Alloc`
    Alloc {
        /// Type to allocate.
        alloc_type: DuumbiType,
    },
    /// Move ownership: `duumbi:Move`
    Move {
        /// Source node ID to move from.
        source: String,
    },
    /// Borrow (shared or mutable): `duumbi:Borrow` / `duumbi:BorrowMut`
    Borrow {
        /// Source node ID to borrow from.
        source: String,
        /// Whether the borrow is mutable.
        mutable: bool,
    },
    /// Drop heap value: `duumbi:Drop`
    Drop {
        /// Target node ID to drop.
        target: String,
    },
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Const(v) => write!(f, "Const({v})"),
            Op::ConstF64(v) => write!(f, "ConstF64({v})"),
            Op::ConstBool(v) => write!(f, "ConstBool({v})"),
            Op::ConstString(v) => write!(f, "ConstString(\"{v}\")"),
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
            Op::PrintString => f.write_str("PrintString"),
            Op::Return => f.write_str("Return"),
            Op::StringConcat => f.write_str("StringConcat"),
            Op::StringEquals => f.write_str("StringEquals"),
            Op::StringCompare(op) => write!(f, "StringCompare({op})"),
            Op::StringLength => f.write_str("StringLength"),
            Op::StringSlice => f.write_str("StringSlice"),
            Op::StringContains => f.write_str("StringContains"),
            Op::StringFind => f.write_str("StringFind"),
            Op::StringFromI64 => f.write_str("StringFromI64"),
            Op::ArrayNew => f.write_str("ArrayNew"),
            Op::ArrayPush => f.write_str("ArrayPush"),
            Op::ArrayGet => f.write_str("ArrayGet"),
            Op::ArraySet => f.write_str("ArraySet"),
            Op::ArrayLength => f.write_str("ArrayLength"),
            Op::ArrayTryGet => f.write_str("ArrayTryGet"),
            Op::StructNew { struct_name } => write!(f, "StructNew({struct_name})"),
            Op::FieldGet { field_name } => write!(f, "FieldGet({field_name})"),
            Op::FieldSet { field_name } => write!(f, "FieldSet({field_name})"),
            Op::Alloc { alloc_type } => write!(f, "Alloc({alloc_type})"),
            Op::Move { source } => write!(f, "Move({source})"),
            Op::Borrow {
                source,
                mutable: true,
            } => write!(f, "BorrowMut({source})"),
            Op::Borrow {
                source,
                mutable: false,
            } => write!(f, "Borrow({source})"),
            Op::Drop { target } => write!(f, "Drop({target})"),
        }
    }
}

impl Op {
    /// Resolves the output type of this operation.
    ///
    /// For ops whose output type depends on context (e.g. `Const`, `Add`, `Load`),
    /// the `result_type` from the graph node is returned. For ops with a fixed
    /// output type (e.g. `Compare` → Bool, `Print` → Void), the fixed type is
    /// returned regardless of `result_type`.
    ///
    /// Returns `None` for `Return` and `Branch` (no output value).
    #[must_use]
    pub fn output_type(&self, result_type: &Option<DuumbiType>) -> Option<DuumbiType> {
        match self {
            Op::Const(_)
            | Op::ConstF64(_)
            | Op::ConstBool(_)
            | Op::ConstString(_)
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Load { .. }
            | Op::Call { .. }
            | Op::ArrayNew
            | Op::ArrayGet
            | Op::ArrayTryGet
            | Op::StructNew { .. }
            | Op::FieldGet { .. }
            | Op::Alloc { .. }
            | Op::Move { .. }
            | Op::Borrow { .. } => result_type.clone(),
            Op::Compare(_) | Op::StringEquals | Op::StringContains => Some(DuumbiType::Bool),
            Op::StringCompare(_) => Some(DuumbiType::Bool),
            Op::StringConcat | Op::StringSlice | Op::StringFromI64 => Some(DuumbiType::String),
            Op::StringLength | Op::StringFind | Op::ArrayLength => Some(DuumbiType::I64),
            Op::Print
            | Op::PrintString
            | Op::Store { .. }
            | Op::ArrayPush
            | Op::ArraySet
            | Op::FieldSet { .. }
            | Op::Drop { .. } => Some(DuumbiType::Void),
            Op::Return | Op::Branch => None,
        }
    }
}

/// Type in the duumbi type system.
///
/// Primitive types (`I64`, `F64`, `Bool`, `Void`) are stack-allocated.
/// Heap types (`String`, `Array`, `Struct`) are pointer-sized at the
/// Cranelift level and require runtime allocation/deallocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DuumbiType {
    /// 64-bit signed integer.
    I64,
    /// 64-bit floating point.
    F64,
    /// Boolean (true/false).
    Bool,
    /// No return value (used by Print).
    Void,
    /// Heap-allocated UTF-8 string.
    #[allow(dead_code)] // Used starting from Phase 9a-1 string ops
    String,
    /// Homogeneous dynamic array, generic over element type.
    #[allow(dead_code)] // Used starting from Phase 9a-1 array ops
    Array(Box<DuumbiType>),
    /// Named struct type. Field definitions are stored in the struct registry,
    /// not in this enum — only the struct name is carried here.
    #[allow(dead_code)] // Used starting from Phase 9a-1 struct ops
    Struct(std::string::String),
    /// Shared reference to a value.
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership ops
    Ref(Box<DuumbiType>),
    /// Mutable reference to a value.
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership ops
    RefMut(Box<DuumbiType>),
}

impl DuumbiType {
    /// Returns `true` if this type requires heap allocation.
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-1 codegen
    pub fn is_heap_type(&self) -> bool {
        matches!(
            self,
            DuumbiType::String | DuumbiType::Array(_) | DuumbiType::Struct(_)
        )
    }

    /// Returns `true` if this type is a reference (`&T` or `&mut T`).
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership checks
    pub fn is_reference(&self) -> bool {
        matches!(self, DuumbiType::Ref(_) | DuumbiType::RefMut(_))
    }

    /// Returns the inner type for references, or `self` for non-references.
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership checks
    pub fn inner_type(&self) -> &DuumbiType {
        match self {
            DuumbiType::Ref(inner) | DuumbiType::RefMut(inner) => inner,
            other => other,
        }
    }

    /// Returns `true` if this type is a mutable reference (`&mut T`).
    #[must_use]
    #[allow(dead_code)] // Used starting from Phase 9a-2 ownership checks
    pub fn is_mutable_ref(&self) -> bool {
        matches!(self, DuumbiType::RefMut(_))
    }
}

impl fmt::Display for DuumbiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuumbiType::I64 => f.write_str("i64"),
            DuumbiType::F64 => f.write_str("f64"),
            DuumbiType::Bool => f.write_str("bool"),
            DuumbiType::Void => f.write_str("void"),
            DuumbiType::String => f.write_str("string"),
            DuumbiType::Array(elem) => write!(f, "array<{elem}>"),
            DuumbiType::Struct(name) => write!(f, "struct<{name}>"),
            DuumbiType::Ref(inner) => write!(f, "&{inner}"),
            DuumbiType::RefMut(inner) => write!(f, "&mut {inner}"),
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

    #[test]
    fn duumbi_type_display_heap_types() {
        assert_eq!(DuumbiType::String.to_string(), "string");
        assert_eq!(
            DuumbiType::Array(Box::new(DuumbiType::I64)).to_string(),
            "array<i64>"
        );
        assert_eq!(
            DuumbiType::Array(Box::new(DuumbiType::String)).to_string(),
            "array<string>"
        );
        assert_eq!(
            DuumbiType::Struct("Point".to_string()).to_string(),
            "struct<Point>"
        );
    }

    #[test]
    fn duumbi_type_is_heap_type() {
        assert!(!DuumbiType::I64.is_heap_type());
        assert!(!DuumbiType::F64.is_heap_type());
        assert!(!DuumbiType::Bool.is_heap_type());
        assert!(!DuumbiType::Void.is_heap_type());
        assert!(DuumbiType::String.is_heap_type());
        assert!(DuumbiType::Array(Box::new(DuumbiType::I64)).is_heap_type());
        assert!(DuumbiType::Struct("Point".to_string()).is_heap_type());
    }

    #[test]
    fn duumbi_type_equality() {
        assert_eq!(DuumbiType::String, DuumbiType::String);
        assert_eq!(
            DuumbiType::Array(Box::new(DuumbiType::I64)),
            DuumbiType::Array(Box::new(DuumbiType::I64))
        );
        assert_ne!(
            DuumbiType::Array(Box::new(DuumbiType::I64)),
            DuumbiType::Array(Box::new(DuumbiType::F64))
        );
        assert_eq!(
            DuumbiType::Struct("Point".to_string()),
            DuumbiType::Struct("Point".to_string())
        );
        assert_ne!(
            DuumbiType::Struct("Point".to_string()),
            DuumbiType::Struct("Vec2".to_string())
        );
    }

    #[test]
    fn op_display_ownership_variants() {
        assert_eq!(
            Op::Alloc {
                alloc_type: DuumbiType::String
            }
            .to_string(),
            "Alloc(string)"
        );
        assert_eq!(
            Op::Move {
                source: "x".to_string()
            }
            .to_string(),
            "Move(x)"
        );
        assert_eq!(
            Op::Borrow {
                source: "x".to_string(),
                mutable: false
            }
            .to_string(),
            "Borrow(x)"
        );
        assert_eq!(
            Op::Borrow {
                source: "x".to_string(),
                mutable: true
            }
            .to_string(),
            "BorrowMut(x)"
        );
        assert_eq!(
            Op::Drop {
                target: "x".to_string()
            }
            .to_string(),
            "Drop(x)"
        );
    }

    #[test]
    fn duumbi_type_display_references() {
        assert_eq!(
            DuumbiType::Ref(Box::new(DuumbiType::String)).to_string(),
            "&string"
        );
        assert_eq!(
            DuumbiType::RefMut(Box::new(DuumbiType::String)).to_string(),
            "&mut string"
        );
        assert_eq!(
            DuumbiType::Ref(Box::new(DuumbiType::Array(Box::new(DuumbiType::I64)))).to_string(),
            "&array<i64>"
        );
    }

    #[test]
    fn duumbi_type_reference_helpers() {
        let shared = DuumbiType::Ref(Box::new(DuumbiType::String));
        assert!(shared.is_reference());
        assert!(!shared.is_mutable_ref());
        assert_eq!(*shared.inner_type(), DuumbiType::String);

        let mutable = DuumbiType::RefMut(Box::new(DuumbiType::I64));
        assert!(mutable.is_reference());
        assert!(mutable.is_mutable_ref());
        assert_eq!(*mutable.inner_type(), DuumbiType::I64);

        let plain = DuumbiType::I64;
        assert!(!plain.is_reference());
        assert!(!plain.is_mutable_ref());
        assert_eq!(*plain.inner_type(), DuumbiType::I64);
    }

    #[test]
    fn duumbi_type_ref_not_heap() {
        assert!(!DuumbiType::Ref(Box::new(DuumbiType::String)).is_heap_type());
        assert!(!DuumbiType::RefMut(Box::new(DuumbiType::String)).is_heap_type());
    }

    #[test]
    fn ownership_op_output_types() {
        let alloc = Op::Alloc {
            alloc_type: DuumbiType::String,
        };
        assert_eq!(
            alloc.output_type(&Some(DuumbiType::String)),
            Some(DuumbiType::String)
        );

        let mv = Op::Move {
            source: "x".to_string(),
        };
        assert_eq!(
            mv.output_type(&Some(DuumbiType::String)),
            Some(DuumbiType::String)
        );

        let borrow = Op::Borrow {
            source: "x".to_string(),
            mutable: false,
        };
        assert_eq!(
            borrow.output_type(&Some(DuumbiType::Ref(Box::new(DuumbiType::String)))),
            Some(DuumbiType::Ref(Box::new(DuumbiType::String)))
        );

        let drop = Op::Drop {
            target: "x".to_string(),
        };
        assert_eq!(drop.output_type(&None), Some(DuumbiType::Void));
    }
}
