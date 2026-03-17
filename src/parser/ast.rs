//! Typed AST representation for parsed JSON-LD.
//!
//! These structures mirror the JSON-LD schema but use Rust types for
//! safety. They are the intermediate representation between raw JSON
//! and the semantic graph.

use crate::types::{BlockLabel, DuumbiType, FunctionName, ModuleName, NodeId, Op};

/// A reference to another node by its `@id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRef {
    /// The `@id` of the referenced node.
    pub id: NodeId,
}

/// A parsed function parameter.
#[allow(dead_code)] // Fields used once graph builder handles params
#[derive(Debug, Clone)]
pub struct ParamAst {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub param_type: DuumbiType,
    /// Optional lifetime annotation (e.g. `"'a"`).
    pub lifetime: Option<String>,
}

/// A parsed operation within a block.
#[derive(Debug, Clone)]
pub struct OpAst {
    /// The `@id` of this operation node.
    pub id: NodeId,
    /// The operation type with embedded data (e.g. `Const(42)`).
    pub op: Op,
    /// Result type of this operation, if applicable.
    pub result_type: Option<DuumbiType>,
    /// Left operand reference (for binary ops).
    pub left: Option<NodeRef>,
    /// Right operand reference (for binary ops).
    pub right: Option<NodeRef>,
    /// Single operand reference (for Print, Return, Compare).
    pub operand: Option<NodeRef>,
    /// Condition reference (for Branch).
    pub condition: Option<NodeRef>,
    /// True block label (for Branch).
    pub true_block: Option<BlockLabel>,
    /// False block label (for Branch).
    pub false_block: Option<BlockLabel>,
    /// Argument references (for Call).
    pub args: Vec<NodeRef>,
}

/// A parsed basic block.
#[allow(dead_code)] // Fields used in future compilation phases
#[derive(Debug, Clone)]
pub struct BlockAst {
    /// The `@id` of this block.
    pub id: NodeId,
    /// Block label (e.g. `"entry"`).
    pub label: BlockLabel,
    /// Operations in this block, in order.
    pub ops: Vec<OpAst>,
}

/// A parsed function.
#[allow(dead_code)] // Fields used in future compilation phases
#[derive(Debug, Clone)]
pub struct FunctionAst {
    /// The `@id` of this function.
    pub id: NodeId,
    /// Function name.
    pub name: FunctionName,
    /// Declared return type.
    pub return_type: DuumbiType,
    /// Function parameters.
    pub params: Vec<ParamAst>,
    /// Blocks in this function.
    pub blocks: Vec<BlockAst>,
    /// Lifetime parameters declared on this function (e.g. `["'a", "'b"]`).
    pub lifetime_params: Vec<String>,
}

/// A parsed import declaration on a `duumbi:Module` node.
///
/// Corresponds to one entry in the `duumbi:imports` array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAst {
    /// Logical module name (e.g. `"stdlib/math"`).
    pub module_name: String,
    /// Relative path to the `.jsonld` file (e.g. `"../stdlib/math.jsonld"`).
    pub path: String,
    /// Specific function names to import. An empty list means all exported functions.
    pub functions: Vec<String>,
}

/// A parsed module — the top-level AST node.
#[allow(dead_code)] // Fields used in future compilation phases
#[derive(Debug, Clone)]
pub struct ModuleAst {
    /// The `@id` of this module.
    pub id: NodeId,
    /// Module name.
    pub name: ModuleName,
    /// Functions defined in this module.
    pub functions: Vec<FunctionAst>,
    /// Modules imported by this module (from `duumbi:imports`).
    pub imports: Vec<ImportAst>,
    /// Function names exported by this module (from `duumbi:exports`).
    pub exports: Vec<String>,
}
