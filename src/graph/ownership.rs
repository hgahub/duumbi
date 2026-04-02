//! Ownership state tracking for semantic graph validation.
//!
//! Analyzes ownership flow within functions: tracks which values are
//! owned, moved, borrowed (shared or mutable), or dropped. The validator
//! uses this to detect use-after-move, borrow exclusivity violations,
//! dangling references, and other ownership errors.
//!
//! # AI-AGENT
//!
//! All public types in this module carry `#[allow(dead_code)]`. This is NOT
//! stale — ownership analysis is wired into `graph::validator::validate()` and
//! runs whenever the graph contains Phase 9a-2 ops (Alloc/Move/Borrow/Drop).
//! The `#[allow]` silences clippy for the fields that are read only through
//! pattern matching or by the upcoming Phase 9a-2 checks. Do not remove them.

use std::collections::HashMap;

use crate::errors::{Diagnostic, codes};
use crate::graph::{FunctionInfo, SemanticGraph};
use crate::types::{NodeId, Op};

/// Ownership state of a value at a point in the program.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Used by validator checks C2-C5 (upcoming)
pub enum ValueState {
    /// Value is live and owned by its creating node.
    Owned,
    /// Value has been moved — any use is an error (E021).
    Moved,
    /// Value has `count` active shared borrows.
    Borrowed { count: usize },
    /// Value has an active mutable borrow.
    BorrowedMut,
    /// Value has been dropped — any use is an error (E026).
    Dropped,
}

/// Tracks a single active borrow for borrow exclusivity analysis.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used by validator checks C2-C5 (upcoming)
pub struct ActiveBorrow {
    /// The node ID of the borrow operation.
    pub borrow_node: NodeId,
    /// The node ID of the borrowed source value.
    pub source_node: NodeId,
    /// Whether this is a mutable borrow.
    pub mutable: bool,
    /// The block label where this borrow was created.
    pub block: String,
}

/// Per-function ownership analysis result.
#[derive(Debug)]
#[allow(dead_code)] // Used by validator checks C2-C5 (upcoming)
pub struct OwnershipAnalysis {
    /// Final state of each value (keyed by the node ID that created it).
    pub value_states: HashMap<NodeId, ValueState>,
    /// All active borrows in program order.
    pub borrows: Vec<ActiveBorrow>,
    /// Map from source node ID to list of borrow node IDs.
    pub borrow_sources: HashMap<NodeId, Vec<NodeId>>,
    /// Map from node ID to the block it belongs to.
    pub node_blocks: HashMap<NodeId, String>,
    /// Ordered list of ownership-relevant events for linear analysis.
    pub events: Vec<OwnershipEvent>,
}

/// An ownership-relevant event in program order.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used by validator checks C2-C5 (upcoming)
pub enum OwnershipEvent {
    /// A value was allocated/created.
    Alloc {
        /// Node ID of the allocation.
        node: NodeId,
        /// Block where allocation occurs.
        block: String,
    },
    /// A value was moved.
    Move {
        /// Node ID of the Move op.
        node: NodeId,
        /// The source identifier being moved.
        source: String,
        /// Node ID of the source value (from operand edge).
        source_node: Option<NodeId>,
        /// Block where move occurs.
        block: String,
    },
    /// A value was borrowed.
    Borrow {
        /// Node ID of the Borrow op.
        node: NodeId,
        /// The source identifier.
        source: String,
        /// Node ID of the source value (from operand edge).
        source_node: Option<NodeId>,
        /// Whether mutable.
        mutable: bool,
        /// Block where borrow occurs.
        block: String,
    },
    /// A value was dropped.
    Drop {
        /// Node ID of the Drop op.
        node: NodeId,
        /// The target identifier being dropped.
        target: String,
        /// Node ID of the target value (from operand edge).
        target_node: Option<NodeId>,
        /// Block where drop occurs.
        block: String,
    },
    /// A value is used (Load, or operand to another op).
    Use {
        /// Node ID of the op that uses the value.
        node: NodeId,
        /// Variable name being used (for Load ops).
        variable: Option<String>,
        /// Block where use occurs.
        block: String,
    },
}

/// Analyzes ownership flow for a single function in the graph.
///
/// Walks the function's blocks in order, building an event list and
/// tracking value states. Does not emit diagnostics — that's the
/// validator's job using the returned analysis.
#[must_use]
#[allow(dead_code)] // Used by validator checks C2-C5 (upcoming)
pub fn analyze_function(graph: &SemanticGraph, func_info: &FunctionInfo) -> OwnershipAnalysis {
    let mut analysis = OwnershipAnalysis {
        value_states: HashMap::new(),
        borrows: Vec::new(),
        borrow_sources: HashMap::new(),
        node_blocks: HashMap::new(),
        events: Vec::new(),
    };

    for block_info in &func_info.blocks {
        let block_label = block_info.label.0.clone();

        for &node_idx in &block_info.nodes {
            let node = &graph.graph[node_idx];
            let node_id = node.id.clone();
            analysis
                .node_blocks
                .insert(node_id.clone(), block_label.clone());

            match &node.op {
                Op::Alloc { .. } => {
                    analysis
                        .value_states
                        .insert(node_id.clone(), ValueState::Owned);
                    analysis.events.push(OwnershipEvent::Alloc {
                        node: node_id,
                        block: block_label.clone(),
                    });
                }
                Op::Move { source } => {
                    // Find the operand (source value node) via edges
                    let source_node = find_operand_source(graph, node_idx);
                    if let Some(ref src) = source_node {
                        analysis.value_states.insert(src.clone(), ValueState::Moved);
                    }
                    // The move result is a new owned value
                    analysis
                        .value_states
                        .insert(node_id.clone(), ValueState::Owned);
                    analysis.events.push(OwnershipEvent::Move {
                        node: node_id,
                        source: source.clone(),
                        source_node,
                        block: block_label.clone(),
                    });
                }
                Op::Borrow { source, mutable } => {
                    let source_node = find_operand_source(graph, node_idx);
                    if let Some(ref src) = source_node {
                        if *mutable {
                            analysis
                                .value_states
                                .insert(src.clone(), ValueState::BorrowedMut);
                        } else {
                            let state = analysis
                                .value_states
                                .entry(src.clone())
                                .or_insert(ValueState::Owned);
                            if let ValueState::Borrowed { count } = state {
                                *count += 1;
                            } else if *state == ValueState::Owned {
                                *state = ValueState::Borrowed { count: 1 };
                            }
                        }
                        analysis
                            .borrow_sources
                            .entry(src.clone())
                            .or_default()
                            .push(node_id.clone());
                    }
                    analysis.borrows.push(ActiveBorrow {
                        borrow_node: node_id.clone(),
                        source_node: source_node.clone().unwrap_or_else(|| NodeId(String::new())),
                        mutable: *mutable,
                        block: block_label.clone(),
                    });
                    analysis.events.push(OwnershipEvent::Borrow {
                        node: node_id,
                        source: source.clone(),
                        source_node,
                        mutable: *mutable,
                        block: block_label.clone(),
                    });
                }
                Op::Drop { target } => {
                    let target_node = find_operand_source(graph, node_idx);
                    if let Some(ref tgt) = target_node {
                        analysis
                            .value_states
                            .insert(tgt.clone(), ValueState::Dropped);
                    }
                    analysis.events.push(OwnershipEvent::Drop {
                        node: node_id,
                        target: target.clone(),
                        target_node,
                        block: block_label.clone(),
                    });
                }
                Op::Load { variable } => {
                    analysis.events.push(OwnershipEvent::Use {
                        node: node_id,
                        variable: Some(variable.clone()),
                        block: block_label.clone(),
                    });
                }
                _ => {
                    // Non-ownership ops — no state changes
                }
            }
        }
    }

    analysis
}

/// Returns `true` if the graph contains any ownership ops (Alloc, Move, Borrow, Drop).
///
/// Used to gate ownership validation — Phase 0-8 graphs have no ownership
/// ops and should skip all ownership checks for backward compatibility.
#[must_use]
#[allow(dead_code)] // Used by validator checks C2-C5 (upcoming)
pub fn has_ownership_ops(graph: &SemanticGraph) -> bool {
    graph.graph.node_weights().any(|node| {
        matches!(
            &node.op,
            Op::Alloc { .. } | Op::Move { .. } | Op::Borrow { .. } | Op::Drop { .. }
        )
    })
}

/// Finds the source node of an operand edge (incoming Operand, MovesFrom, BorrowsFrom, or Drops edge).
#[allow(dead_code)] // Used by analyze_function (suppressed at caller level)
fn find_operand_source(
    graph: &SemanticGraph,
    node_idx: petgraph::stable_graph::NodeIndex,
) -> Option<NodeId> {
    use crate::graph::GraphEdge;
    use petgraph::visit::EdgeRef;

    for edge in graph
        .graph
        .edges_directed(node_idx, petgraph::Direction::Incoming)
    {
        match edge.weight() {
            GraphEdge::Operand
            | GraphEdge::MovesFrom
            | GraphEdge::BorrowsFrom
            | GraphEdge::Drops => {
                return Some(graph.graph[edge.source()].id.clone());
            }
            _ => {}
        }
    }
    None
}

/// Checks for use-after-move violations (E021).
///
/// After a Move event, any subsequent use of the moved value's source node
/// (via operand edges or Load of the same variable) is an error.
pub fn check_use_after_move(analysis: &OwnershipAnalysis, diagnostics: &mut Vec<Diagnostic>) {
    // Track which source nodes have been moved, and at which Move node
    let mut moved_sources: HashMap<NodeId, NodeId> = HashMap::new(); // source_node -> move_node

    for event in &analysis.events {
        match event {
            OwnershipEvent::Move {
                node,
                source_node: Some(src),
                ..
            } => {
                moved_sources.insert(src.clone(), node.clone());
            }
            OwnershipEvent::Borrow {
                node,
                source_node: Some(src),
                ..
            } => {
                if moved_sources.contains_key(src) {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E021_USE_AFTER_MOVE,
                            format!(
                                "Use after move: value '{}' was moved and cannot be borrowed",
                                src
                            ),
                        )
                        .with_node(node),
                    );
                }
            }
            OwnershipEvent::Drop {
                node,
                target_node: Some(tgt),
                ..
            } => {
                if moved_sources.contains_key(tgt) {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E021_USE_AFTER_MOVE,
                            format!(
                                "Use after move: value '{}' was moved and cannot be dropped",
                                tgt
                            ),
                        )
                        .with_node(node),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Checks borrow exclusivity — shared XOR mutable (E022).
///
/// At any point, a value can have either:
/// - Multiple shared borrows (`&T`), OR
/// - Exactly one mutable borrow (`&mut T`)
///
/// but not both simultaneously.
pub fn check_borrow_exclusivity(analysis: &OwnershipAnalysis, diagnostics: &mut Vec<Diagnostic>) {
    // Track active borrows per source node
    // Key: source_node_id, Value: list of (borrow_node_id, mutable)
    let mut active_borrows: HashMap<NodeId, Vec<(NodeId, bool)>> = HashMap::new();

    for event in &analysis.events {
        match event {
            OwnershipEvent::Borrow {
                node,
                source_node: Some(src),
                mutable,
                ..
            } => {
                let borrows = active_borrows.entry(src.clone()).or_default();

                if *mutable {
                    // Mutable borrow: no other borrows allowed
                    if !borrows.is_empty() {
                        let existing_kind = if borrows.iter().any(|(_, m)| *m) {
                            "mutable"
                        } else {
                            "shared"
                        };
                        diagnostics.push(
                            Diagnostic::error(
                                codes::E022_BORROW_EXCLUSIVITY,
                                format!(
                                    "Borrow exclusivity violation: cannot take mutable borrow of '{}' \
                                     while {} borrow exists",
                                    src, existing_kind
                                ),
                            )
                            .with_node(node),
                        );
                    }
                } else {
                    // Shared borrow: no mutable borrows allowed
                    if borrows.iter().any(|(_, m)| *m) {
                        diagnostics.push(
                            Diagnostic::error(
                                codes::E022_BORROW_EXCLUSIVITY,
                                format!(
                                    "Borrow exclusivity violation: cannot take shared borrow of '{}' \
                                     while mutable borrow exists",
                                    src
                                ),
                            )
                            .with_node(node),
                        );
                    }
                }

                borrows.push((node.clone(), *mutable));
            }
            OwnershipEvent::Drop {
                target_node: Some(tgt),
                ..
            } => {
                // Dropping a value clears its borrows
                active_borrows.remove(tgt);
            }
            _ => {}
        }
    }
}

/// Checks that borrows don't outlive their owner's scope (E023).
///
/// A borrow created in block B cannot be used in a block that executes
/// after the owner's block ends. For now, this is a simplified check:
/// borrows and owners must be in the same block.
pub fn check_lifetimes(analysis: &OwnershipAnalysis, diagnostics: &mut Vec<Diagnostic>) {
    // Check that borrows and their sources are in the same block
    for event in &analysis.events {
        if let OwnershipEvent::Borrow {
            node,
            source_node: Some(src),
            block,
            ..
        } = event
            && let Some(src_block) = analysis.node_blocks.get(src)
            && src_block != block
        {
            diagnostics.push(
                Diagnostic::error(
                    codes::E023_LIFETIME_EXCEEDED,
                    format!(
                        "Lifetime exceeded: borrow of '{}' in block '{}' \
                         outlives owner in block '{}'",
                        src, block, src_block
                    ),
                )
                .with_node(node),
            );
        }
    }
}

/// Checks for double-free (E025) and dangling references (E026).
///
/// - E025: A value is dropped more than once.
/// - E026: A borrow is created after the source value was dropped.
pub fn check_drop_safety(analysis: &OwnershipAnalysis, diagnostics: &mut Vec<Diagnostic>) {
    // Track dropped source nodes
    let mut dropped: HashMap<NodeId, NodeId> = HashMap::new(); // source_node -> first_drop_node

    for event in &analysis.events {
        match event {
            OwnershipEvent::Drop {
                node,
                target_node: Some(tgt),
                ..
            } => {
                if dropped.contains_key(tgt) {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E025_DOUBLE_FREE,
                            format!("Double free: value '{}' is dropped more than once", tgt),
                        )
                        .with_node(node),
                    );
                } else {
                    dropped.insert(tgt.clone(), node.clone());
                }
            }
            OwnershipEvent::Borrow {
                node,
                source_node: Some(src),
                ..
            } => {
                if dropped.contains_key(src) {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E026_DANGLING_REFERENCE,
                            format!(
                                "Dangling reference: cannot borrow '{}' after it was dropped",
                                src
                            ),
                        )
                        .with_node(node),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Checks that values cannot be moved while borrows are active (E027).
pub fn check_move_while_borrowed(analysis: &OwnershipAnalysis, diagnostics: &mut Vec<Diagnostic>) {
    // Track which nodes have active borrows
    let mut borrowed_nodes: std::collections::HashSet<NodeId> = std::collections::HashSet::new();

    for event in &analysis.events {
        match event {
            OwnershipEvent::Borrow {
                source_node: Some(src),
                ..
            } => {
                borrowed_nodes.insert(src.clone());
            }
            OwnershipEvent::Move {
                node,
                source_node: Some(src),
                ..
            } => {
                if borrowed_nodes.contains(src) {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E027_MOVE_WHILE_BORROWED,
                            format!("Cannot move '{}' while it is borrowed", src),
                        )
                        .with_node(node),
                    );
                }
            }
            OwnershipEvent::Drop {
                target_node: Some(tgt),
                ..
            } => {
                // Dropping clears borrows
                borrowed_nodes.remove(tgt);
            }
            _ => {}
        }
    }
}

/// Checks cross-function lifetime parameters (E028, E029).
///
/// - E028: A function that borrows a parameter must declare lifetime params.
/// - E029: A function returning a reference must tie it to an input lifetime.
pub fn check_lifetime_params(func_info: &FunctionInfo, diagnostics: &mut Vec<Diagnostic>) {
    let has_ref_params = func_info.params.iter().any(|p| p.param_type.is_reference());
    let returns_ref = func_info.return_type.is_reference();

    if has_ref_params && func_info.lifetime_params.is_empty() {
        diagnostics.push(Diagnostic::error(
            codes::E028_LIFETIME_PARAM_MISSING,
            format!(
                "Function '{}' has reference parameters but no lifetime parameters declared",
                func_info.name
            ),
        ));
    }

    if returns_ref && func_info.lifetime_params.is_empty() {
        diagnostics.push(Diagnostic::error(
            codes::E029_RETURN_LIFETIME_MISMATCH,
            format!(
                "Function '{}' returns a reference but has no lifetime parameters — \
                 returned reference must be tied to an input lifetime",
                func_info.name
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::builder::build_graph;
    use crate::parser::parse_jsonld;

    #[test]
    fn analyze_alloc_produces_owned_state() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/1",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/2",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/1"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let analysis = analyze_function(&sg, &sg.functions[0]);

        let alloc_id = NodeId("duumbi:t/main/e/0".to_string());
        assert_eq!(analysis.value_states[&alloc_id], ValueState::Owned);
        assert_eq!(analysis.events.len(), 1); // Only 1 ownership event (Alloc)
    }

    #[test]
    fn analyze_move_marks_source_as_moved() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Move", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/2",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/3",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/2"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let analysis = analyze_function(&sg, &sg.functions[0]);

        let alloc_id = NodeId("duumbi:t/main/e/0".to_string());
        assert_eq!(analysis.value_states[&alloc_id], ValueState::Moved);
    }

    #[test]
    fn analyze_borrow_tracks_shared_count() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let analysis = analyze_function(&sg, &sg.functions[0]);

        let alloc_id = NodeId("duumbi:t/main/e/0".to_string());
        assert_eq!(
            analysis.value_states[&alloc_id],
            ValueState::Borrowed { count: 2 }
        );
        assert_eq!(analysis.borrows.len(), 2);
    }

    #[test]
    fn analyze_drop_marks_value_as_dropped() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Drop", "@id": "duumbi:t/main/e/1",
                         "duumbi:target": "s",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/2",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/3",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/2"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let analysis = analyze_function(&sg, &sg.functions[0]);

        let alloc_id = NodeId("duumbi:t/main/e/0".to_string());
        assert_eq!(analysis.value_states[&alloc_id], ValueState::Dropped);
    }

    #[test]
    fn has_ownership_ops_false_for_plain_graph() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/0",
                         "duumbi:value": 42, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/1",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        assert!(!has_ownership_ops(&sg));
    }

    #[test]
    fn has_ownership_ops_true_with_alloc() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/1",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/2",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/1"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        assert!(has_ownership_ops(&sg));
    }

    #[test]
    fn use_after_move_detected() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Move", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_use_after_move(&analysis, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E021"),
            "Expected E021, got: {diags:?}"
        );
    }

    #[test]
    fn valid_move_no_use_after() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Move", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Drop", "@id": "duumbi:t/main/e/2",
                         "duumbi:target": "s2",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/1"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_use_after_move(&analysis, &mut diags);
        assert!(diags.is_empty(), "Expected no errors, got: {diags:?}");
    }

    #[test]
    fn double_borrow_mut_detected() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:BorrowMut", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "&mut string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:BorrowMut", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&mut string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_borrow_exclusivity(&analysis, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E022"),
            "Expected E022, got: {diags:?}"
        );
    }

    #[test]
    fn shared_borrow_plus_mut_borrow_detected() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:BorrowMut", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&mut string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_borrow_exclusivity(&analysis, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E022"),
            "Expected E022, got: {diags:?}"
        );
    }

    #[test]
    fn multiple_shared_borrows_ok() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_borrow_exclusivity(&analysis, &mut diags);
        assert!(diags.is_empty(), "Expected no E022, got: {diags:?}");
    }

    #[test]
    fn double_free_detected() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Drop", "@id": "duumbi:t/main/e/1",
                         "duumbi:target": "s",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Drop", "@id": "duumbi:t/main/e/2",
                         "duumbi:target": "s",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_drop_safety(&analysis, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E025"),
            "Expected E025, got: {diags:?}"
        );
    }

    #[test]
    fn dangling_reference_detected() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Drop", "@id": "duumbi:t/main/e/1",
                         "duumbi:target": "s",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_drop_safety(&analysis, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E026"),
            "Expected E026, got: {diags:?}"
        );
    }

    #[test]
    fn move_while_borrowed_detected() {
        let json = r#"{
            "@type": "duumbi:Module", "@id": "duumbi:t", "duumbi:name": "t",
            "duumbi:functions": [{
                "@type": "duumbi:Function", "@id": "duumbi:t/main",
                "duumbi:name": "main", "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block", "@id": "duumbi:t/main/e",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {"@type": "duumbi:Alloc", "@id": "duumbi:t/main/e/0",
                         "duumbi:allocType": "string", "duumbi:resultType": "string"},
                        {"@type": "duumbi:Borrow", "@id": "duumbi:t/main/e/1",
                         "duumbi:source": "s", "duumbi:resultType": "&string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Move", "@id": "duumbi:t/main/e/2",
                         "duumbi:source": "s", "duumbi:resultType": "string",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/0"}},
                        {"@type": "duumbi:Const", "@id": "duumbi:t/main/e/3",
                         "duumbi:value": 0, "duumbi:resultType": "i64"},
                        {"@type": "duumbi:Return", "@id": "duumbi:t/main/e/4",
                         "duumbi:operand": {"@id": "duumbi:t/main/e/3"}}
                    ]
                }]
            }]
        }"#;
        let module = parse_jsonld(json).expect("invariant: test JSON must parse");
        let sg = build_graph(&module).expect("invariant: test graph must build");
        let mut diags = Vec::new();
        let analysis = analyze_function(&sg, &sg.functions[0]);
        check_move_while_borrowed(&analysis, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E027"),
            "Expected E027, got: {diags:?}"
        );
    }

    #[test]
    fn lifetime_param_missing_detected() {
        use crate::graph::ParamInfo;
        use crate::types::{DuumbiType, FunctionName};

        let func = FunctionInfo {
            name: FunctionName("borrow_fn".to_string()),
            return_type: DuumbiType::I64,
            params: vec![ParamInfo {
                name: "s".to_string(),
                param_type: DuumbiType::Ref(Box::new(DuumbiType::String)),
                lifetime: None,
            }],
            blocks: vec![],
            lifetime_params: vec![], // Missing!
        };

        let mut diags = Vec::new();
        check_lifetime_params(&func, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E028"),
            "Expected E028, got: {diags:?}"
        );
    }

    #[test]
    fn return_ref_without_lifetime_detected() {
        use crate::types::{DuumbiType, FunctionName};

        let func = FunctionInfo {
            name: FunctionName("get_ref".to_string()),
            return_type: DuumbiType::Ref(Box::new(DuumbiType::String)),
            params: vec![],
            blocks: vec![],
            lifetime_params: vec![], // Missing!
        };

        let mut diags = Vec::new();
        check_lifetime_params(&func, &mut diags);
        assert!(
            diags.iter().any(|d| d.code == "E029"),
            "Expected E029, got: {diags:?}"
        );
    }
}
