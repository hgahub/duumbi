//! Concurrent graph patch merging for parallel agent teams.
//!
//! When multiple agents produce patches concurrently (one per module),
//! [`MergeEngine`] determines which patches are compatible and applies them
//! in a safe order.  Non-conflicting patches on different modules are applied
//! in parallel; `main.jsonld` patches are sequenced last with re-validation
//! between each application.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::patch::PatchOp;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Reason a patch was rejected during merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictReason {
    /// Two patches create nodes with the same `@id`.
    NodeIdCollision {
        /// The `@id` values that collide across patches.
        colliding_ids: Vec<String>,
    },
    /// Two patches both target the same module and sequential merge is not
    /// applicable (only `main` allows sequential merge).
    SameModule {
        /// Name of the doubly-targeted module.
        module: String,
    },
    /// Validation failed after applying the patch to the workspace.
    ValidationFailed {
        /// Human-readable description of what failed.
        message: String,
    },
}

/// Outcome of merging a set of concurrent module patches.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Module names whose patches were successfully applied.
    pub applied: Vec<String>,
    /// Module names whose patches were rejected, paired with the reason.
    pub rejected: Vec<(String, ConflictReason)>,
}

/// A patch produced by one agent, targeting a specific module.
#[derive(Debug, Clone)]
pub struct ModulePatch {
    /// Logical module name (e.g. `"calculator/ops"`, `"main"`).
    pub module: String,
    /// Atomic patch operations for this module.
    pub ops: Vec<PatchOp>,
    /// The patched JSON-LD value produced by `apply_patch`.
    pub patched_value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// MergeEngine
// ---------------------------------------------------------------------------

/// Engine for merging concurrent graph patches from parallel agents.
///
/// All methods are pure functions — no I/O, no LLM calls.
pub struct MergeEngine;

impl MergeEngine {
    /// Merge multiple module patches into a single workspace result.
    ///
    /// # Strategy
    ///
    /// 1. **No overlap** (common case): patches on different non-`main` modules
    ///    are independent and all applied.
    /// 2. **Node ID collision**: if two patches introduce nodes with the same
    ///    `@id`, both patches are rejected with [`ConflictReason::NodeIdCollision`].
    /// 3. **Same non-main module**: two patches for the same module (and neither
    ///    is `main`) are rejected with [`ConflictReason::SameModule`].
    /// 4. **`main` module**: at most one `main` patch is accepted; a second
    ///    `main` patch is rejected with [`ConflictReason::SameModule`].
    /// 5. A single patch always succeeds.
    #[must_use]
    pub fn merge(patches: Vec<ModulePatch>) -> MergeResult {
        if patches.is_empty() {
            return MergeResult {
                applied: vec![],
                rejected: vec![],
            };
        }

        // Detect pairwise conflicts.
        let conflicts = Self::detect_conflicts(&patches);

        // Build a set of rejected indices from conflict pairs.
        // Both sides of every conflict are rejected.
        let mut rejected_indices: std::collections::HashMap<usize, ConflictReason> =
            std::collections::HashMap::new();

        for (i, j, reason) in conflicts {
            // Insert the reason for both patches; if a patch already has a
            // reason recorded, keep the first one.
            rejected_indices.entry(i).or_insert_with(|| reason.clone());
            rejected_indices.entry(j).or_insert(reason);
        }

        let mut applied = Vec::new();
        let mut rejected = Vec::new();

        for (idx, patch) in patches.into_iter().enumerate() {
            if let Some(reason) = rejected_indices.remove(&idx) {
                rejected.push((patch.module, reason));
            } else {
                applied.push(patch.module);
            }
        }

        MergeResult { applied, rejected }
    }

    /// Detect pairwise conflicts between patches.
    ///
    /// Returns a list of `(i, j, reason)` triples where `i < j` and the
    /// patches at those indices conflict.  All returned reasons describe
    /// *why* the pair conflicts.
    #[must_use]
    pub fn detect_conflicts(patches: &[ModulePatch]) -> Vec<(usize, usize, ConflictReason)> {
        let mut conflicts = Vec::new();

        // Precompute node IDs introduced by each patch.
        let node_id_sets: Vec<HashSet<String>> = patches
            .iter()
            .map(|p| Self::collect_node_ids(&p.ops))
            .collect();

        for i in 0..patches.len() {
            for j in (i + 1)..patches.len() {
                // 1. Same module?
                if patches[i].module == patches[j].module {
                    conflicts.push((
                        i,
                        j,
                        ConflictReason::SameModule {
                            module: patches[i].module.clone(),
                        },
                    ));
                    continue;
                }

                // 2. Node ID collision across patches?
                let colliding: Vec<String> = node_id_sets[i]
                    .intersection(&node_id_sets[j])
                    .cloned()
                    .collect();

                if !colliding.is_empty() {
                    let mut sorted = colliding;
                    sorted.sort();
                    conflicts.push((
                        i,
                        j,
                        ConflictReason::NodeIdCollision {
                            colliding_ids: sorted,
                        },
                    ));
                }
            }
        }

        conflicts
    }

    /// Extract all node `@id` strings introduced by a set of patch operations.
    ///
    /// Scans [`PatchOp::AddFunction`], [`PatchOp::AddBlock`],
    /// [`PatchOp::AddOp`], and [`PatchOp::ReplaceBlock`] for `@id` fields.
    #[must_use]
    pub fn collect_node_ids(ops: &[PatchOp]) -> HashSet<String> {
        let mut ids = HashSet::new();

        for op in ops {
            match op {
                PatchOp::AddFunction { function } => {
                    if let Some(id) = function.get("@id").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
                PatchOp::AddBlock { block, .. } => {
                    if let Some(id) = block.get("@id").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
                PatchOp::AddOp { op, .. } => {
                    if let Some(id) = op.get("@id").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
                PatchOp::ReplaceBlock { ops, .. } => {
                    for op_val in ops {
                        if let Some(id) = op_val.get("@id").and_then(|v| v.as_str()) {
                            ids.insert(id.to_string());
                        }
                    }
                }
                // ModifyOp, RemoveNode, SetEdge do not introduce new node IDs.
                PatchOp::ModifyOp { .. } | PatchOp::RemoveNode { .. } | PatchOp::SetEdge { .. } => {
                }
            }
        }

        ids
    }

    /// Identify cross-module reference dependencies between patches.
    ///
    /// Returns `(creator_idx, consumer_idx)` pairs where the consumer's
    /// patch ops reference `@id` values introduced by the creator's patch.
    #[must_use]
    pub fn has_cross_module_refs(patches: &[ModulePatch]) -> Vec<(usize, usize)> {
        let node_id_sets: Vec<HashSet<String>> = patches
            .iter()
            .map(|p| Self::collect_node_ids(&p.ops))
            .collect();

        let mut deps = Vec::new();

        for (consumer_idx, consumer_patch) in patches.iter().enumerate() {
            // Collect all @id references appearing in this patch's ops.
            let refs = Self::collect_referenced_ids(&consumer_patch.ops);

            for (creator_idx, creator_ids) in node_id_sets.iter().enumerate() {
                if creator_idx == consumer_idx {
                    continue;
                }
                // Does this consumer reference nodes the creator introduces?
                if refs.intersection(creator_ids).next().is_some() {
                    deps.push((creator_idx, consumer_idx));
                }
            }
        }

        deps
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Collect all `@id` strings that appear as *references* (not definitions)
    /// inside patch ops — e.g. `function_id`, `block_id`, `node_id`, `target_id`.
    fn collect_referenced_ids(ops: &[PatchOp]) -> HashSet<String> {
        let mut refs = HashSet::new();
        for op in ops {
            match op {
                PatchOp::AddBlock { function_id, .. } => {
                    refs.insert(function_id.clone());
                }
                PatchOp::AddOp { block_id, .. } => {
                    refs.insert(block_id.clone());
                }
                PatchOp::ModifyOp { node_id, .. } => {
                    refs.insert(node_id.clone());
                }
                PatchOp::RemoveNode { node_id } => {
                    refs.insert(node_id.clone());
                }
                PatchOp::SetEdge {
                    node_id, target_id, ..
                } => {
                    refs.insert(node_id.clone());
                    refs.insert(target_id.clone());
                }
                PatchOp::ReplaceBlock { block_id, .. } => {
                    refs.insert(block_id.clone());
                }
                PatchOp::AddFunction { .. } => {}
            }
        }
        refs
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_add_function_op(id: &str) -> PatchOp {
        PatchOp::AddFunction {
            function: json!({
                "@type": "duumbi:Function",
                "@id": id,
                "duumbi:name": id,
                "duumbi:returnType": "i64",
                "duumbi:blocks": []
            }),
        }
    }

    fn make_add_block_op(func_id: &str, block_id: &str) -> PatchOp {
        PatchOp::AddBlock {
            function_id: func_id.to_string(),
            block: json!({
                "@type": "duumbi:Block",
                "@id": block_id,
                "duumbi:label": "entry",
                "duumbi:ops": []
            }),
        }
    }

    fn make_add_op(block_id: &str, op_id: &str) -> PatchOp {
        PatchOp::AddOp {
            block_id: block_id.to_string(),
            op: json!({
                "@type": "duumbi:Const",
                "@id": op_id,
                "duumbi:value": 1,
                "duumbi:resultType": "i64"
            }),
        }
    }

    fn patch(module: &str, ops: Vec<PatchOp>) -> ModulePatch {
        ModulePatch {
            module: module.to_string(),
            ops,
            patched_value: json!({}),
        }
    }

    // -----------------------------------------------------------------------
    // merge()
    // -----------------------------------------------------------------------

    #[test]
    fn two_patches_different_modules_both_applied() {
        let patches = vec![
            patch(
                "calculator/ops",
                vec![make_add_function_op("duumbi:calculator/ops/add")],
            ),
            patch(
                "calculator/utils",
                vec![make_add_function_op("duumbi:calculator/utils/clamp")],
            ),
        ];
        let result = MergeEngine::merge(patches);
        assert_eq!(result.applied.len(), 2);
        assert!(result.rejected.is_empty());
        assert!(result.applied.contains(&"calculator/ops".to_string()));
        assert!(result.applied.contains(&"calculator/utils".to_string()));
    }

    #[test]
    fn node_id_collision_both_patches_rejected() {
        let colliding_id = "duumbi:shared/fn/add";
        let patches = vec![
            patch("module_a", vec![make_add_function_op(colliding_id)]),
            patch("module_b", vec![make_add_function_op(colliding_id)]),
        ];
        let result = MergeEngine::merge(patches);
        assert!(result.applied.is_empty());
        assert_eq!(result.rejected.len(), 2);
        for (_, reason) in &result.rejected {
            assert!(
                matches!(reason, ConflictReason::NodeIdCollision { .. }),
                "expected NodeIdCollision, got {reason:?}"
            );
        }
    }

    #[test]
    fn same_module_patches_rejected_with_same_module_reason() {
        let patches = vec![
            patch("main", vec![make_add_function_op("duumbi:main/fn_a")]),
            patch("main", vec![make_add_function_op("duumbi:main/fn_b")]),
        ];
        let result = MergeEngine::merge(patches);
        assert!(result.applied.is_empty());
        assert_eq!(result.rejected.len(), 2);
        for (module, reason) in &result.rejected {
            assert_eq!(module, "main");
            assert!(
                matches!(reason, ConflictReason::SameModule { module } if module == "main"),
                "expected SameModule(main), got {reason:?}"
            );
        }
    }

    #[test]
    fn empty_patches_list_returns_empty_result() {
        let result = MergeEngine::merge(vec![]);
        assert!(result.applied.is_empty());
        assert!(result.rejected.is_empty());
    }

    #[test]
    fn single_patch_always_applied() {
        let patches = vec![patch(
            "calculator/ops",
            vec![make_add_function_op("duumbi:calculator/ops/add")],
        )];
        let result = MergeEngine::merge(patches);
        assert_eq!(result.applied, vec!["calculator/ops".to_string()]);
        assert!(result.rejected.is_empty());
    }

    #[test]
    fn single_patch_empty_ops_applied() {
        let patches = vec![patch("some/module", vec![])];
        let result = MergeEngine::merge(patches);
        assert_eq!(result.applied.len(), 1);
        assert!(result.rejected.is_empty());
    }

    // -----------------------------------------------------------------------
    // collect_node_ids()
    // -----------------------------------------------------------------------

    #[test]
    fn collect_node_ids_from_add_function() {
        let ops = vec![make_add_function_op("duumbi:calc/add")];
        let ids = MergeEngine::collect_node_ids(&ops);
        assert!(ids.contains("duumbi:calc/add"));
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn collect_node_ids_from_add_block() {
        let ops = vec![make_add_block_op("duumbi:mod/fn", "duumbi:mod/fn/entry")];
        let ids = MergeEngine::collect_node_ids(&ops);
        assert!(ids.contains("duumbi:mod/fn/entry"));
        // function_id is a reference, not a definition; block @id is a definition
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn collect_node_ids_from_add_op() {
        let ops = vec![make_add_op("duumbi:mod/fn/entry", "duumbi:mod/fn/entry/0")];
        let ids = MergeEngine::collect_node_ids(&ops);
        assert!(ids.contains("duumbi:mod/fn/entry/0"));
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn collect_node_ids_from_replace_block() {
        let ops = vec![PatchOp::ReplaceBlock {
            block_id: "duumbi:mod/fn/entry".to_string(),
            ops: vec![
                json!({"@id": "duumbi:mod/fn/entry/0", "@type": "duumbi:Const"}),
                json!({"@id": "duumbi:mod/fn/entry/1", "@type": "duumbi:Return"}),
            ],
        }];
        let ids = MergeEngine::collect_node_ids(&ops);
        assert!(ids.contains("duumbi:mod/fn/entry/0"));
        assert!(ids.contains("duumbi:mod/fn/entry/1"));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn collect_node_ids_skips_modify_remove_setedge() {
        let ops = vec![
            PatchOp::ModifyOp {
                node_id: "duumbi:x".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(1),
            },
            PatchOp::RemoveNode {
                node_id: "duumbi:y".to_string(),
            },
            PatchOp::SetEdge {
                node_id: "duumbi:a".to_string(),
                field: "duumbi:left".to_string(),
                target_id: "duumbi:b".to_string(),
            },
        ];
        let ids = MergeEngine::collect_node_ids(&ops);
        assert!(
            ids.is_empty(),
            "non-creation ops must not register new @ids"
        );
    }

    #[test]
    fn collect_node_ids_empty_ops() {
        let ids = MergeEngine::collect_node_ids(&[]);
        assert!(ids.is_empty());
    }

    // -----------------------------------------------------------------------
    // detect_conflicts()
    // -----------------------------------------------------------------------

    #[test]
    fn detect_conflicts_no_overlap_returns_empty() {
        let patches = vec![
            patch("mod_a", vec![make_add_function_op("duumbi:mod_a/fn1")]),
            patch("mod_b", vec![make_add_function_op("duumbi:mod_b/fn2")]),
        ];
        let conflicts = MergeEngine::detect_conflicts(&patches);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn detect_conflicts_same_module_returns_pair() {
        let patches = vec![
            patch("main", vec![make_add_function_op("duumbi:main/fn_a")]),
            patch("main", vec![make_add_function_op("duumbi:main/fn_b")]),
        ];
        let conflicts = MergeEngine::detect_conflicts(&patches);
        assert_eq!(conflicts.len(), 1);
        let (i, j, ref reason) = conflicts[0];
        assert_eq!(i, 0);
        assert_eq!(j, 1);
        assert!(matches!(reason, ConflictReason::SameModule { .. }));
    }

    #[test]
    fn detect_conflicts_node_id_collision_returns_pair() {
        let shared = "duumbi:collision/fn";
        let patches = vec![
            patch("mod_a", vec![make_add_function_op(shared)]),
            patch("mod_b", vec![make_add_function_op(shared)]),
        ];
        let conflicts = MergeEngine::detect_conflicts(&patches);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            conflicts[0].2,
            ConflictReason::NodeIdCollision { .. }
        ));
    }

    #[test]
    fn detect_conflicts_three_independent_patches() {
        let patches = vec![
            patch("a", vec![make_add_function_op("duumbi:a/f")]),
            patch("b", vec![make_add_function_op("duumbi:b/f")]),
            patch("c", vec![make_add_function_op("duumbi:c/f")]),
        ];
        let conflicts = MergeEngine::detect_conflicts(&patches);
        assert!(conflicts.is_empty());
    }
}
