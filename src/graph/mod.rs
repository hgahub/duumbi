//! Semantic graph module.
//!
//! Builds and validates a `petgraph::StableGraph` representation of
//! the program from the parsed AST. The graph is the central IR —
//! all transformations are graph-to-graph.

pub mod builder;
pub mod validator;

use std::collections::HashMap;

use petgraph::stable_graph::{NodeIndex, StableGraph};
use thiserror::Error;

use crate::types::{BlockLabel, DuumbiType, FunctionName, NodeId, Op};

/// Errors that can occur during graph construction.
#[derive(Debug, Error)]
pub enum GraphError {
    /// A duplicate `@id` was found in the graph.
    #[error("[{code}] Duplicate @id: '{node_id}'")]
    DuplicateId {
        /// Error code for diagnostics.
        code: &'static str,
        /// The duplicated node ID.
        node_id: String,
    },

    /// A reference points to a non-existent `@id`.
    #[error("[{code}] Orphan reference to '{target}' from node '{from_node}'")]
    OrphanRef {
        /// Error code for diagnostics.
        code: &'static str,
        /// The node containing the dangling reference.
        from_node: String,
        /// The referenced `@id` that does not exist.
        target: String,
    },

    /// No `main` function was found.
    #[error("[{code}] No entry function 'main' found")]
    NoEntry {
        /// Error code for diagnostics.
        code: &'static str,
    },
}

impl GraphError {
    /// Returns the error code for this graph error.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            GraphError::DuplicateId { code, .. }
            | GraphError::OrphanRef { code, .. }
            | GraphError::NoEntry { code, .. } => code,
        }
    }
}

/// A node in the semantic graph.
#[allow(dead_code)] // Fields used in future compilation phases
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Unique identifier from JSON-LD `@id`.
    pub id: NodeId,
    /// The operation this node represents.
    pub op: Op,
    /// Result type of this node, if applicable.
    pub result_type: Option<DuumbiType>,
    /// Which function this node belongs to.
    pub function: FunctionName,
    /// Which block this node belongs to.
    pub block: BlockLabel,
}

/// Edge label in the semantic graph.
#[derive(Debug, Clone)]
pub enum GraphEdge {
    /// Left operand of a binary operation.
    Left,
    /// Right operand of a binary operation.
    Right,
    /// Single operand (for Print, Return).
    Operand,
}

/// Information about a function in the graph.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Function name.
    pub name: FunctionName,
    /// Declared return type.
    pub return_type: DuumbiType,
    /// Blocks in this function, in order.
    pub blocks: Vec<BlockInfo>,
}

/// Information about a block in the graph.
#[allow(dead_code)] // Fields used in future compilation phases
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// Block label.
    pub label: BlockLabel,
    /// Node indices in this block, in order.
    pub nodes: Vec<NodeIndex>,
}

/// The semantic graph — central data structure of duumbi.
///
/// Contains the petgraph `StableGraph` plus metadata about functions
/// and blocks, and a lookup map from `NodeId` to `NodeIndex`.
#[allow(dead_code)] // Fields used in future compilation phases
#[derive(Debug)]
pub struct SemanticGraph {
    /// The underlying petgraph stable graph.
    pub graph: StableGraph<GraphNode, GraphEdge>,
    /// Map from `NodeId` to `NodeIndex` for O(1) lookups.
    pub node_map: HashMap<NodeId, NodeIndex>,
    /// Function metadata, in order.
    pub functions: Vec<FunctionInfo>,
}
