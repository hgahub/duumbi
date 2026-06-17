//! Deterministic rewrite preview engine.

use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;

use crate::graph::builder::build_graph;
use crate::graph::validator::validate;
use crate::graph::{GraphEdge, GraphNode, SemanticGraph};
use crate::parser::parse_jsonld;
use crate::patch::{GraphPatch, PatchOp, apply_patch};
use crate::types::{DuumbiType, Op};

use super::catalog::{BuiltInRuleKind, RewriteCatalog, RuleDefinition};
use super::error::RewriteError;
use super::evidence::{
    ApplyMode, ApplyOptions, CostEvidence, RewriteApplyOutcome, RewriteApplyPlan, RewriteLimits,
    RewriteMatch, RewritePreview, ValidationEvidence,
};
use super::rule::RuleSummary;

/// Provider-free rewrite engine for rule listing and preview matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteEngine {
    catalog: RewriteCatalog,
    limits: RewriteLimits,
}

impl RewriteEngine {
    /// Creates an engine with the supplied catalog and limits.
    #[must_use]
    pub fn new(catalog: RewriteCatalog, limits: RewriteLimits) -> Self {
        Self { catalog, limits }
    }

    /// Returns rule summaries in deterministic catalog order.
    #[must_use]
    pub fn list_rules(&self) -> Vec<RuleSummary> {
        self.catalog.summaries()
    }

    /// Previews one rule against raw JSON-LD source without mutating it.
    ///
    /// # Errors
    ///
    /// Returns [`RewriteError`] when the current source cannot parse, build,
    /// validate, or references an unknown rule.
    pub fn preview_source(
        &self,
        source: &serde_json::Value,
        rule_id: &str,
        max_matches: Option<usize>,
    ) -> Result<RewritePreview, RewriteError> {
        let graph = current_graph_from_source(source)?;
        self.preview_graph(&graph.module_name.to_string(), &graph, rule_id, max_matches)
    }

    /// Previews one rule against an already-built semantic graph.
    ///
    /// # Errors
    ///
    /// Returns [`RewriteError::UnknownRule`] when `rule_id` is not registered.
    pub fn preview_graph(
        &self,
        module: &str,
        graph: &SemanticGraph,
        rule_id: &str,
        max_matches: Option<usize>,
    ) -> Result<RewritePreview, RewriteError> {
        let rule = self
            .catalog
            .find(rule_id)
            .ok_or_else(|| RewriteError::UnknownRule(rule_id.to_string()))?;
        let requested_limit = max_matches.unwrap_or(self.limits.max_matches_per_preview);
        let limit = requested_limit.min(self.limits.max_matches_per_preview);
        let mut considered = 0usize;
        let mut returned = Vec::new();

        for node_idx in ordered_nodes(graph) {
            if let Some(matched) = self.match_rule(module, graph, rule, node_idx, considered) {
                considered += 1;
                if returned.len() < limit {
                    returned.push(matched);
                }
            }
        }

        let cost = aggregate_cost(&returned, considered, self.limits);

        Ok(RewritePreview {
            status: "success".to_string(),
            rule: rule.summary.clone(),
            matches: returned,
            cost,
            warnings: Vec::new(),
        })
    }

    /// Applies a selected rule to cloned JSON-LD source in memory.
    ///
    /// Apply reruns parse, graph build, validation, matching, preconditions,
    /// safety-class checks, patch construction, patch application, and
    /// candidate validation. It never saves a snapshot or writes a file.
    ///
    /// # Errors
    ///
    /// Returns [`RewriteError`] for invalid current graphs, unknown rules,
    /// stale matches, unsupported safety classes, cost-bound failures, patch
    /// failures, and invalid candidates.
    pub fn apply_to_source(
        &self,
        source: &serde_json::Value,
        options: &ApplyOptions,
    ) -> Result<RewriteApplyOutcome, RewriteError> {
        let graph = current_graph_from_source(source)?;
        let module = options
            .module
            .clone()
            .unwrap_or_else(|| graph.module_name.to_string());
        let rule = self
            .catalog
            .find(&options.rule_id)
            .ok_or_else(|| RewriteError::UnknownRule(options.rule_id.clone()))?;

        if !rule.summary.apply_capable {
            return Err(RewriteError::UnsupportedSafetyClass(
                options.rule_id.clone(),
            ));
        }

        let preview = self.preview_graph(
            &module,
            &graph,
            &options.rule_id,
            Some(self.limits.max_matches_per_preview),
        )?;
        let selected = select_matches(&preview.matches, options, self.limits)?;
        let patch = self.patch_for_matches(&graph, rule, &selected)?;

        let candidate = apply_patch(source, &patch)
            .map_err(|err| RewriteError::ValidationFailed(err.to_string()))?;
        validate_candidate_source(&candidate)?;

        let touched_node_ids = selected
            .iter()
            .flat_map(|matched| matched.touched_node_ids.iter().cloned())
            .collect::<Vec<_>>();
        let cost = CostEvidence {
            matches_considered: preview.cost.matches_considered,
            matches_returned: selected.len(),
            matches_truncated: 0,
            touched_node_count: selected
                .iter()
                .map(|matched| matched.cost.touched_node_count)
                .sum(),
            patch_op_count: patch.ops.len(),
            estimated_cost_units: selected
                .iter()
                .map(|matched| matched.cost.estimated_cost_units)
                .sum(),
            limits: self.limits,
        };

        Ok(RewriteApplyOutcome {
            candidate_source: candidate,
            plan: RewriteApplyPlan {
                status: "success".to_string(),
                rule: rule.summary.clone(),
                match_ids: selected
                    .iter()
                    .map(|matched| matched.match_id.clone())
                    .collect(),
                touched_node_ids,
                operation_summary: format!(
                    "Prepared {} rewrite patch operation(s) for {} match(es).",
                    patch.ops.len(),
                    selected.len()
                ),
                validation: ValidationEvidence::valid(),
                cost,
                warnings: Vec::new(),
            },
        })
    }

    fn patch_for_matches(
        &self,
        graph: &SemanticGraph,
        rule: &RuleDefinition,
        matches: &[RewriteMatch],
    ) -> Result<GraphPatch, RewriteError> {
        let mut ops = Vec::new();
        for matched in matches {
            let node_idx = graph
                .node_map
                .get(&crate::types::NodeId(matched.primary_node_id.clone()))
                .copied()
                .ok_or_else(|| RewriteError::StaleMatch(matched.match_id.clone()))?;
            let replacement = replacement_node_id(graph, rule, node_idx)
                .ok_or_else(|| RewriteError::StaleMatch(matched.match_id.clone()))?;
            let mut match_ops = redirect_consumers(graph, node_idx, &replacement)?;
            if match_ops.is_empty() {
                return Err(RewriteError::InvalidRequest(format!(
                    "rewrite match '{}' has no downstream references to replace",
                    matched.match_id
                )));
            }
            if match_ops.len() > self.limits.max_patch_ops_per_match {
                return Err(RewriteError::CostBoundExceeded(format!(
                    "match '{}' would produce {} patch ops, limit is {}",
                    matched.match_id,
                    match_ops.len(),
                    self.limits.max_patch_ops_per_match
                )));
            }
            ops.append(&mut match_ops);
        }

        Ok(GraphPatch { ops })
    }

    fn match_rule(
        &self,
        module: &str,
        graph: &SemanticGraph,
        rule: &RuleDefinition,
        node_idx: NodeIndex,
        ordinal: usize,
    ) -> Option<RewriteMatch> {
        let node = graph.graph.node_weight(node_idx)?;
        let operands = binary_operands(graph, node_idx)?;
        let effect = match rule.kind {
            BuiltInRuleKind::I64AddZeroRight
                if is_i64_add(node) && is_i64(operands.left) && is_i64_const(operands.right, 0) =>
            {
                format!(
                    "Replace {} with left operand {}.",
                    node.id, operands.left.id
                )
            }
            BuiltInRuleKind::I64AddZeroLeft
                if is_i64_add(node) && is_i64_const(operands.left, 0) && is_i64(operands.right) =>
            {
                format!(
                    "Replace {} with right operand {}.",
                    node.id, operands.right.id
                )
            }
            BuiltInRuleKind::I64MulOneRight
                if is_i64_mul(node) && is_i64(operands.left) && is_i64_const(operands.right, 1) =>
            {
                format!(
                    "Replace {} with left operand {}.",
                    node.id, operands.left.id
                )
            }
            BuiltInRuleKind::ExperimentalFoldI64ConstAdd
                if is_i64_add(node)
                    && is_i64_const_node(operands.left)
                    && is_i64_const_node(operands.right) =>
            {
                let left_value = const_i64_value(operands.left)?;
                let right_value = const_i64_value(operands.right)?;
                format!(
                    "Would replace {} with Const({}).",
                    node.id,
                    left_value + right_value
                )
            }
            _ => return None,
        };

        let match_cost = CostEvidence {
            matches_considered: 1,
            matches_returned: 1,
            matches_truncated: 0,
            touched_node_count: 3,
            patch_op_count: usize::from(rule.summary.apply_capable),
            estimated_cost_units: rule.summary.default_cost,
            limits: self.limits,
        };

        Some(RewriteMatch {
            match_id: format!("{}:{}:{}:{}", rule.summary.id, module, node.id, ordinal),
            rule_id: rule.summary.id.clone(),
            module: module.to_string(),
            primary_node_id: node.id.to_string(),
            touched_node_ids: vec![
                operands.left.id.to_string(),
                operands.right.id.to_string(),
                node.id.to_string(),
            ],
            operation_summary: effect,
            explanation: rule.summary.explanation_template.clone(),
            cost: match_cost,
            validation: ValidationEvidence::not_run(),
            warnings: Vec::new(),
        })
    }
}

impl Default for RewriteEngine {
    fn default() -> Self {
        Self::new(RewriteCatalog::built_in(), RewriteLimits::default())
    }
}

struct BinaryOperands<'a> {
    left: &'a GraphNode,
    right: &'a GraphNode,
}

fn ordered_nodes(graph: &SemanticGraph) -> Vec<NodeIndex> {
    graph
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.nodes.iter().copied())
        .collect()
}

fn binary_operands(graph: &SemanticGraph, node_idx: NodeIndex) -> Option<BinaryOperands<'_>> {
    let mut left = None;
    let mut right = None;

    for edge in graph.graph.edges_directed(node_idx, petgraph::Incoming) {
        match edge.weight() {
            GraphEdge::Left => left = graph.graph.node_weight(edge.source()),
            GraphEdge::Right => right = graph.graph.node_weight(edge.source()),
            _ => {}
        }
    }

    Some(BinaryOperands {
        left: left?,
        right: right?,
    })
}

fn replacement_node_id(
    graph: &SemanticGraph,
    rule: &RuleDefinition,
    node_idx: NodeIndex,
) -> Option<String> {
    let operands = binary_operands(graph, node_idx)?;
    match rule.kind {
        BuiltInRuleKind::I64AddZeroRight | BuiltInRuleKind::I64MulOneRight => {
            Some(operands.left.id.to_string())
        }
        BuiltInRuleKind::I64AddZeroLeft => Some(operands.right.id.to_string()),
        BuiltInRuleKind::ExperimentalFoldI64ConstAdd => None,
    }
}

fn redirect_consumers(
    graph: &SemanticGraph,
    node_idx: NodeIndex,
    replacement_id: &str,
) -> Result<Vec<PatchOp>, RewriteError> {
    let mut refs = Vec::new();
    for edge in graph.graph.edges_directed(node_idx, petgraph::Outgoing) {
        let Some(field) = edge_field(edge.weight()) else {
            let Some(consumer) = graph.graph.node_weight(edge.target()) else {
                return Err(RewriteError::StaleMatch(
                    "missing consumer node".to_string(),
                ));
            };
            return Err(RewriteError::InvalidRequest(format!(
                "cannot rewrite unsupported consumer edge {:?} on {}",
                edge.weight(),
                consumer.id
            )));
        };
        let Some(consumer) = graph.graph.node_weight(edge.target()) else {
            return Err(RewriteError::StaleMatch(
                "missing consumer node".to_string(),
            ));
        };
        refs.push((consumer.id.to_string(), field.to_string()));
    }

    refs.sort();
    Ok(refs
        .into_iter()
        .map(|(node_id, field)| PatchOp::SetEdge {
            node_id,
            field,
            target_id: replacement_id.to_string(),
        })
        .collect())
}

fn edge_field(edge: &GraphEdge) -> Option<&'static str> {
    match edge {
        GraphEdge::Left => Some("duumbi:left"),
        GraphEdge::Right => Some("duumbi:right"),
        GraphEdge::Operand => Some("duumbi:operand"),
        GraphEdge::Condition => Some("duumbi:condition"),
        _ => None,
    }
}

fn is_i64_add(node: &GraphNode) -> bool {
    matches!(node.op, Op::Add) && is_i64(node)
}

fn is_i64_mul(node: &GraphNode) -> bool {
    matches!(node.op, Op::Mul) && is_i64(node)
}

fn is_i64(node: &GraphNode) -> bool {
    node.result_type == Some(DuumbiType::I64)
}

fn is_i64_const(node: &GraphNode, value: i64) -> bool {
    matches!(node.op, Op::Const(actual) if actual == value) && is_i64(node)
}

fn is_i64_const_node(node: &GraphNode) -> bool {
    matches!(node.op, Op::Const(_)) && is_i64(node)
}

fn const_i64_value(node: &GraphNode) -> Option<i64> {
    match node.op {
        Op::Const(value) if is_i64(node) => Some(value),
        _ => None,
    }
}

fn aggregate_cost(
    matches: &[RewriteMatch],
    matches_considered: usize,
    limits: RewriteLimits,
) -> CostEvidence {
    CostEvidence {
        matches_considered,
        matches_returned: matches.len(),
        matches_truncated: matches_considered.saturating_sub(matches.len()),
        touched_node_count: matches
            .iter()
            .map(|matched| matched.cost.touched_node_count)
            .sum(),
        patch_op_count: matches
            .iter()
            .map(|matched| matched.cost.patch_op_count)
            .sum(),
        estimated_cost_units: matches
            .iter()
            .map(|matched| matched.cost.estimated_cost_units)
            .sum(),
        limits,
    }
}

fn current_graph_from_source(source: &serde_json::Value) -> Result<SemanticGraph, RewriteError> {
    let graph = graph_from_source(source).map_err(RewriteError::InvalidCurrentGraph)?;
    let diagnostics = validate(&graph);
    if !diagnostics.is_empty() {
        let messages = diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RewriteError::InvalidCurrentGraph(messages));
    }
    Ok(graph)
}

fn validate_candidate_source(source: &serde_json::Value) -> Result<(), RewriteError> {
    let graph = graph_from_source(source).map_err(RewriteError::ValidationFailed)?;
    let diagnostics = validate(&graph);
    if !diagnostics.is_empty() {
        let messages = diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RewriteError::ValidationFailed(messages));
    }
    Ok(())
}

fn graph_from_source(source: &serde_json::Value) -> Result<SemanticGraph, String> {
    let json = serde_json::to_string(source).map_err(|err| err.to_string())?;
    let ast = parse_jsonld(&json).map_err(|err| err.to_string())?;
    build_graph(&ast).map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn select_matches(
    matches: &[RewriteMatch],
    options: &ApplyOptions,
    limits: RewriteLimits,
) -> Result<Vec<RewriteMatch>, RewriteError> {
    match options.mode {
        ApplyMode::Match => {
            let match_id = options.match_id.as_ref().ok_or_else(|| {
                RewriteError::InvalidRequest("--match requires match_id".to_string())
            })?;
            matches
                .iter()
                .find(|matched| matched.match_id == *match_id)
                .cloned()
                .map(|matched| vec![matched])
                .ok_or_else(|| RewriteError::StaleMatch(match_id.clone()))
        }
        ApplyMode::All => {
            let requested = options.max_matches.unwrap_or(limits.max_matches_per_apply);
            if requested > limits.max_apply_all_matches {
                return Err(RewriteError::CostBoundExceeded(format!(
                    "apply-all requested {requested} matches, limit is {}",
                    limits.max_apply_all_matches
                )));
            }
            if matches.len() > requested {
                return Err(RewriteError::CostBoundExceeded(format!(
                    "{} matches found, requested apply bound is {requested}",
                    matches.len()
                )));
            }
            if matches.is_empty() {
                return Err(RewriteError::StaleMatch(
                    "no matches available for apply-all".to_string(),
                ));
            }
            Ok(matches.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use petgraph::stable_graph::StableGraph;
    use serde_json::{Value, json};

    use super::*;
    use crate::graph::{BlockInfo, FunctionInfo, GraphNode, SemanticGraph};
    use crate::types::{BlockLabel, FunctionName, ModuleName, NodeId};

    fn source_fixture() -> Value {
        json!({
            "@context": {"duumbi": "https://duumbi.dev/ontology#"},
            "@type": "duumbi:Module",
            "@id": "duumbi:rewrite",
            "duumbi:name": "rewrite",
            "duumbi:functions": [{
                "@type": "duumbi:Function",
                "@id": "duumbi:rewrite/main",
                "duumbi:name": "main",
                "duumbi:returnType": "i64",
                "duumbi:blocks": [{
                    "@type": "duumbi:Block",
                    "@id": "duumbi:rewrite/main/entry",
                    "duumbi:label": "entry",
                    "duumbi:ops": [
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:rewrite/main/entry/left",
                            "duumbi:value": 42,
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:rewrite/main/entry/zero",
                            "duumbi:value": 0,
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Add",
                            "@id": "duumbi:rewrite/main/entry/add",
                            "duumbi:left": {"@id": "duumbi:rewrite/main/entry/left"},
                            "duumbi:right": {"@id": "duumbi:rewrite/main/entry/zero"},
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:rewrite/main/entry/return",
                            "duumbi:operand": {"@id": "duumbi:rewrite/main/entry/add"}
                        }
                    ]
                }]
            }]
        })
    }

    fn test_node(id: &str, op: Op, result_type: Option<DuumbiType>) -> GraphNode {
        GraphNode {
            id: NodeId(id.to_string()),
            op,
            result_type,
            function: FunctionName("main".to_string()),
            block: BlockLabel("entry".to_string()),
            owner: None,
            lifetime: None,
            lifetime_param: None,
        }
    }

    fn graph_for_rule(op: Op, left: Op, right: Op) -> SemanticGraph {
        let mut graph = StableGraph::new();
        let left_idx = graph.add_node(test_node("left", left, Some(DuumbiType::I64)));
        let right_idx = graph.add_node(test_node("right", right, Some(DuumbiType::I64)));
        let op_idx = graph.add_node(test_node("op", op, Some(DuumbiType::I64)));
        graph.add_edge(left_idx, op_idx, GraphEdge::Left);
        graph.add_edge(right_idx, op_idx, GraphEdge::Right);
        let mut node_map = HashMap::new();
        let node_indices = vec![left_idx, right_idx, op_idx];
        for idx in &node_indices {
            let node = graph
                .node_weight(*idx)
                .expect("invariant: test node exists");
            node_map.insert(node.id.clone(), *idx);
        }

        SemanticGraph {
            graph,
            node_map,
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: node_indices,
                }],
                lifetime_params: Vec::new(),
                contracts: Default::default(),
            }],
            branch_targets: HashMap::new(),
            module_name: ModuleName("test".to_string()),
        }
    }

    #[test]
    fn list_rules_returns_catalog_order() {
        let ids: Vec<_> = RewriteEngine::default()
            .list_rules()
            .into_iter()
            .map(|summary| summary.id)
            .collect();

        assert_eq!(
            ids,
            vec![
                "i64-add-zero-right",
                "i64-add-zero-left",
                "i64-mul-one-right",
                "experimental-fold-i64-const-add",
            ]
        );
    }

    #[test]
    fn preview_matches_i64_add_zero_right() {
        let graph = graph_for_rule(Op::Add, Op::Const(42), Op::Const(0));
        let preview = RewriteEngine::default()
            .preview_graph("test", &graph, "i64-add-zero-right", None)
            .expect("invariant: rule preview succeeds");

        assert_eq!(preview.matches.len(), 1);
        assert_eq!(preview.matches[0].match_id, "i64-add-zero-right:test:op:0");
        assert_eq!(
            preview.matches[0].touched_node_ids,
            vec!["left", "right", "op"]
        );
        assert_eq!(preview.cost.matches_considered, 1);
        assert_eq!(preview.cost.patch_op_count, 1);
    }

    #[test]
    fn preview_matches_i64_add_zero_left() {
        let graph = graph_for_rule(Op::Add, Op::Const(0), Op::Const(42));
        let preview = RewriteEngine::default()
            .preview_graph("test", &graph, "i64-add-zero-left", None)
            .expect("invariant: rule preview succeeds");

        assert_eq!(preview.matches.len(), 1);
        assert_eq!(
            preview.matches[0].operation_summary,
            "Replace op with right operand right."
        );
    }

    #[test]
    fn preview_matches_i64_mul_one_right() {
        let graph = graph_for_rule(Op::Mul, Op::Const(42), Op::Const(1));
        let preview = RewriteEngine::default()
            .preview_graph("test", &graph, "i64-mul-one-right", None)
            .expect("invariant: rule preview succeeds");

        assert_eq!(preview.matches.len(), 1);
        assert_eq!(
            preview.matches[0].operation_summary,
            "Replace op with left operand left."
        );
    }

    #[test]
    fn experimental_const_fold_is_preview_only_and_has_no_patch_ops() {
        let graph = graph_for_rule(Op::Add, Op::Const(40), Op::Const(2));
        let preview = RewriteEngine::default()
            .preview_graph("test", &graph, "experimental-fold-i64-const-add", None)
            .expect("invariant: rule preview succeeds");

        assert_eq!(preview.matches.len(), 1);
        assert!(!preview.rule.apply_capable);
        assert_eq!(
            preview.matches[0].operation_summary,
            "Would replace op with Const(42)."
        );
        assert_eq!(preview.cost.patch_op_count, 0);
    }

    #[test]
    fn preview_applies_max_match_limit_deterministically() {
        let mut graph = StableGraph::new();
        let a = graph.add_node(test_node("a", Op::Const(1), Some(DuumbiType::I64)));
        let zero = graph.add_node(test_node("zero", Op::Const(0), Some(DuumbiType::I64)));
        let first = graph.add_node(test_node("first", Op::Add, Some(DuumbiType::I64)));
        let second = graph.add_node(test_node("second", Op::Add, Some(DuumbiType::I64)));
        graph.add_edge(a, first, GraphEdge::Left);
        graph.add_edge(zero, first, GraphEdge::Right);
        graph.add_edge(a, second, GraphEdge::Left);
        graph.add_edge(zero, second, GraphEdge::Right);
        let mut node_map = HashMap::new();
        let node_indices = vec![a, zero, first, second];
        for idx in &node_indices {
            let node = graph
                .node_weight(*idx)
                .expect("invariant: test node exists");
            node_map.insert(node.id.clone(), *idx);
        }
        let semantic_graph = SemanticGraph {
            graph,
            node_map,
            functions: vec![FunctionInfo {
                name: FunctionName("main".to_string()),
                return_type: DuumbiType::I64,
                params: Vec::new(),
                blocks: vec![BlockInfo {
                    label: BlockLabel("entry".to_string()),
                    nodes: node_indices,
                }],
                lifetime_params: Vec::new(),
                contracts: Default::default(),
            }],
            branch_targets: HashMap::new(),
            module_name: ModuleName("test".to_string()),
        };

        let preview = RewriteEngine::default()
            .preview_graph("test", &semantic_graph, "i64-add-zero-right", Some(1))
            .expect("invariant: rule preview succeeds");

        assert_eq!(preview.matches.len(), 1);
        assert_eq!(
            preview.matches[0].match_id,
            "i64-add-zero-right:test:first:0"
        );
        assert_eq!(preview.cost.matches_considered, 2);
        assert_eq!(preview.cost.matches_truncated, 1);
    }

    #[test]
    fn preview_rejects_unknown_rule() {
        let graph = graph_for_rule(Op::Add, Op::Const(1), Op::Const(0));
        let err = RewriteEngine::default()
            .preview_graph("test", &graph, "missing-rule", None)
            .expect_err("invariant: missing rule is rejected");

        assert_eq!(err, RewriteError::UnknownRule("missing-rule".to_string()));
    }

    #[test]
    fn preview_source_parses_builds_validates_and_matches() {
        let preview = RewriteEngine::default()
            .preview_source(&source_fixture(), "i64-add-zero-right", None)
            .expect("invariant: preview source succeeds");

        assert_eq!(preview.matches.len(), 1);
        assert_eq!(
            preview.matches[0].match_id,
            "i64-add-zero-right:rewrite:duumbi:rewrite/main/entry/add:0"
        );
    }

    #[test]
    fn apply_to_source_reruns_match_and_validates_candidate() {
        let source = source_fixture();
        let preview = RewriteEngine::default()
            .preview_source(&source, "i64-add-zero-right", None)
            .expect("invariant: preview source succeeds");
        let options = ApplyOptions {
            rule_id: "i64-add-zero-right".to_string(),
            module: None,
            mode: ApplyMode::Match,
            match_id: Some(preview.matches[0].match_id.clone()),
            max_matches: None,
        };

        let outcome = RewriteEngine::default()
            .apply_to_source(&source, &options)
            .expect("invariant: apply source succeeds");

        assert_eq!(outcome.plan.status, "success");
        assert_eq!(
            outcome.plan.match_ids,
            vec![preview.matches[0].match_id.clone()]
        );
        assert_eq!(outcome.plan.validation, ValidationEvidence::valid());
        assert_eq!(outcome.plan.cost.patch_op_count, 1);
        assert_eq!(
            outcome.candidate_source["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][3]["duumbi:operand"]
                ["@id"],
            "duumbi:rewrite/main/entry/left"
        );
        assert_eq!(
            source["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][3]["duumbi:operand"]["@id"],
            "duumbi:rewrite/main/entry/add"
        );
    }

    #[test]
    fn apply_rejects_stale_match_before_patch() {
        let source = source_fixture();
        let options = ApplyOptions {
            rule_id: "i64-add-zero-right".to_string(),
            module: None,
            mode: ApplyMode::Match,
            match_id: Some("i64-add-zero-right:rewrite:missing:0".to_string()),
            max_matches: None,
        };

        let err = RewriteEngine::default()
            .apply_to_source(&source, &options)
            .expect_err("invariant: stale match is rejected");

        assert_eq!(
            err,
            RewriteError::StaleMatch("i64-add-zero-right:rewrite:missing:0".to_string())
        );
    }

    #[test]
    fn apply_rejects_experimental_rule() {
        let source = source_fixture();
        let options = ApplyOptions {
            rule_id: "experimental-fold-i64-const-add".to_string(),
            module: None,
            mode: ApplyMode::All,
            match_id: None,
            max_matches: Some(1),
        };

        let err = RewriteEngine::default()
            .apply_to_source(&source, &options)
            .expect_err("invariant: experimental apply is rejected");

        assert_eq!(
            err,
            RewriteError::UnsupportedSafetyClass("experimental-fold-i64-const-add".to_string())
        );
    }
}
