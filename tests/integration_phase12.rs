//! Phase 12 integration tests — kill criterion verification.
//!
//! These tests exercise the five Phase 12 kill criteria without requiring
//! LLM API keys or network access. All operations are purely in-process.

use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Shared test workspace graph
// ---------------------------------------------------------------------------

/// A minimal but valid `main.jsonld` that passes the full parse → build → validate pipeline.
///
/// Derived from `tests/fixtures/add.jsonld` — the simplest known-good graph.
const VALID_GRAPH: &str = r#"{
    "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
    "@type": "duumbi:Module",
    "@id": "duumbi:main",
    "duumbi:name": "main",
    "duumbi:functions": [{
        "@type": "duumbi:Function",
        "@id": "duumbi:main/main",
        "duumbi:name": "main",
        "duumbi:params": [],
        "duumbi:returnType": "i64",
        "duumbi:blocks": [{
            "@type": "duumbi:Block",
            "@id": "duumbi:main/main/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/main/entry/0",
                    "duumbi:value": 0,
                    "duumbi:resultType": "i64"
                },
                {
                    "@type": "duumbi:Return",
                    "@id": "duumbi:main/main/entry/1",
                    "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}
                }
            ]
        }]
    }]
}"#;

/// Create a temporary workspace with a valid main.jsonld and minimal config.toml.
fn setup_workspace() -> TempDir {
    let dir = TempDir::new().expect("invariant: OS must support temp dirs");
    let graph_dir = dir.path().join(".duumbi").join("graph");
    std::fs::create_dir_all(&graph_dir).expect("invariant: create .duumbi/graph");
    std::fs::write(graph_dir.join("main.jsonld"), VALID_GRAPH)
        .expect("invariant: write main.jsonld");
    let config_dir = dir.path().join(".duumbi");
    std::fs::write(
        config_dir.join("config.toml"),
        "[workspace]\nname = \"test\"\n",
    )
    .expect("invariant: write config.toml");
    dir
}

// ---------------------------------------------------------------------------
// Kill Criterion 1 — MCP graph tools work
// ---------------------------------------------------------------------------

/// KC-1a: graph_query returns nodes matching a type filter.
#[test]
fn kill_criterion_1a_mcp_graph_query_by_type() {
    use duumbi::mcp::tools::graph;

    let dir = setup_workspace();
    let result = graph::graph_query(
        dir.path(),
        &serde_json::json!({"type_filter": "duumbi:Function"}),
    );
    assert!(result.is_ok(), "graph_query must succeed: {result:?}");

    let val = result.unwrap();
    let nodes = val["nodes"].as_array().expect("nodes must be an array");
    assert!(
        !nodes.is_empty(),
        "at least one Function node must be found"
    );
    assert_eq!(
        nodes[0]["@type"].as_str(),
        Some("duumbi:Function"),
        "returned node must have the filtered type"
    );
}

/// KC-1b: graph_query returns a specific node by @id.
#[test]
fn kill_criterion_1b_mcp_graph_query_by_node_id() {
    use duumbi::mcp::tools::graph;

    let dir = setup_workspace();
    let result = graph::graph_query(
        dir.path(),
        &serde_json::json!({"node_id": "duumbi:main/main"}),
    );
    assert!(result.is_ok(), "graph_query by id must succeed");

    let val = result.unwrap();
    let nodes = val["nodes"].as_array().expect("nodes must be an array");
    assert_eq!(nodes.len(), 1, "exactly one node must match the @id");
    assert_eq!(nodes[0]["@id"].as_str(), Some("duumbi:main/main"));
}

/// KC-1c: graph_validate returns a well-formed result object.
#[test]
fn kill_criterion_1c_mcp_graph_validate() {
    use duumbi::mcp::tools::graph;

    let dir = setup_workspace();
    let result = graph::graph_validate(dir.path(), &serde_json::json!({}));
    assert!(
        result.is_ok(),
        "graph_validate must not error on a valid workspace"
    );

    let val = result.unwrap();
    assert!(
        val.get("valid").is_some(),
        "result must contain 'valid' field"
    );
    assert!(
        val.get("diagnostics").is_some(),
        "result must contain 'diagnostics' field"
    );
}

/// KC-1d: graph_mutate with missing 'ops' field returns an error.
#[test]
fn kill_criterion_1d_mcp_graph_mutate_missing_ops() {
    use duumbi::mcp::tools::graph;

    let dir = setup_workspace();
    let result = graph::graph_mutate(dir.path(), &serde_json::json!({}));
    assert!(
        result.is_err(),
        "graph_mutate must fail when 'ops' field is absent"
    );
}

/// KC-1e: graph_mutate with an empty ops list succeeds and returns the workspace unchanged.
#[test]
fn kill_criterion_1e_mcp_graph_mutate_empty_ops() {
    use duumbi::mcp::tools::graph;

    let dir = setup_workspace();
    let result = graph::graph_mutate(dir.path(), &serde_json::json!({"ops": []}));
    assert!(
        result.is_ok(),
        "graph_mutate with empty ops must succeed: {result:?}"
    );

    let val = result.unwrap();
    assert_eq!(val["success"], true, "success must be true for empty patch");
    assert_eq!(val["ops_count"], 0, "ops_count must be 0 for empty patch");
}

// ---------------------------------------------------------------------------
// Kill Criterion 2 — Dynamic agent assembly
// ---------------------------------------------------------------------------

/// KC-2: A multi-module intent spec triggers Parallel strategy with Planner + Coder team.
#[test]
fn kill_criterion_2_dynamic_assembly_multi_module() {
    use duumbi::agents::analyzer;
    use duumbi::agents::assembler::{self, ExecutionStrategy};
    use duumbi::agents::template::{AgentRole, TemplateStore};
    use duumbi::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};

    // Intent: 2 modules to create + 1 to modify, and 3 test cases → Moderate complexity.
    let spec = IntentSpec {
        intent: "Build calculator with ops and display modules".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec!["add works".to_string(), "display works".to_string()],
        modules: IntentModules {
            create: vec![
                "calculator/ops".to_string(),
                "calculator/display".to_string(),
            ],
            modify: vec!["app/main".to_string()],
        },
        test_cases: vec![
            TestCase {
                name: "add_test".to_string(),
                function: "add".to_string(),
                args: vec![],
                expected_return: 0,
            },
            TestCase {
                name: "sub_test".to_string(),
                function: "sub".to_string(),
                args: vec![],
                expected_return: 0,
            },
            TestCase {
                name: "display_test".to_string(),
                function: "display".to_string(),
                args: vec![],
                expected_return: 0,
            },
        ],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let profile = analyzer::analyze(&spec);
    // 3 test cases → Moderate; 3 modules → MultiModule; has modules.create → Create task type.
    assert_eq!(
        profile.scope,
        analyzer::Scope::MultiModule,
        "3 modules must give MultiModule scope"
    );
    assert_eq!(
        profile.complexity,
        analyzer::Complexity::Moderate,
        "3 test cases must give Moderate complexity"
    );
    assert_eq!(
        profile.task_type,
        analyzer::TaskType::Create,
        "non-empty modules.create must yield Create task type"
    );

    let tmp = TempDir::new().expect("invariant: temp dir for TemplateStore");
    let store = TemplateStore::load(tmp.path());
    let team = assembler::assemble(&profile, &store);

    // Moderate + Create + Multi → Row 5: [Planner, Coder, Tester], Parallel, parallel_coders >= 2.
    assert!(
        team.agents.contains(&AgentRole::Planner),
        "team must include a Planner agent"
    );
    assert!(
        team.agents.contains(&AgentRole::Coder),
        "team must include a Coder agent"
    );
    assert_eq!(
        team.strategy,
        ExecutionStrategy::Parallel,
        "multi-module moderate intent must use Parallel strategy"
    );
    assert!(
        team.parallel_coders >= 2,
        "parallel_coders must be at least 2 for multi-module parallel team"
    );
}

// ---------------------------------------------------------------------------
// Kill Criterion 3 — Knowledge persistence
// ---------------------------------------------------------------------------

/// KC-3: A strategy saved to disk in one logical "run" is loadable in the next.
#[test]
fn kill_criterion_3_knowledge_persistence() {
    use duumbi::agents::agent_knowledge::{AgentKnowledgeStore, Strategy};

    let dir = TempDir::new().expect("invariant: temp dir for knowledge store");

    // Simulate run 1: record a successful strategy.
    let strategy = Strategy::new(
        "duumbi:template/coder",
        "Test strategy description",
        "create task",
        "use add_function patch op",
    );
    AgentKnowledgeStore::save_strategy(dir.path(), &strategy).expect("save_strategy must succeed");

    // Simulate run 2: load strategies from the same workspace.
    let loaded = AgentKnowledgeStore::load_strategies(dir.path());
    assert_eq!(loaded.len(), 1, "exactly one strategy must be loaded");
    assert_eq!(
        loaded[0].description, "Test strategy description",
        "loaded description must match saved value"
    );
    assert_eq!(
        loaded[0].template_id, "duumbi:template/coder",
        "loaded template_id must match"
    );
    assert_eq!(
        loaded[0].success_count, 0,
        "success_count must be zero for a freshly created strategy"
    );
    assert!(
        !loaded[0].deprecated,
        "a new strategy must not be deprecated"
    );
}

/// KC-3b: Multiple strategies survive a save-load cycle without corruption.
#[test]
fn kill_criterion_3b_multiple_strategies_persist() {
    use duumbi::agents::agent_knowledge::{AgentKnowledgeStore, Strategy};

    let dir = TempDir::new().expect("invariant: temp dir");

    let s1 = Strategy::new("duumbi:template/coder", "strategy one", "create", "add_op");
    let s2 = Strategy::new(
        "duumbi:template/planner",
        "strategy two",
        "plan",
        "decompose",
    );
    AgentKnowledgeStore::save_strategy(dir.path(), &s1).expect("save s1");
    AgentKnowledgeStore::save_strategy(dir.path(), &s2).expect("save s2");

    let loaded = AgentKnowledgeStore::load_strategies(dir.path());
    assert_eq!(loaded.len(), 2, "both strategies must be loadable");

    // Both IDs must be present (order may differ).
    let ids: Vec<&str> = loaded.iter().map(|s| s.id.as_str()).collect();
    assert!(
        ids.contains(&s1.id.as_str()),
        "s1 id must be present after load"
    );
    assert!(
        ids.contains(&s2.id.as_str()),
        "s2 id must be present after load"
    );
}

// ---------------------------------------------------------------------------
// Kill Criterion 4 — Parallel merge
// ---------------------------------------------------------------------------

/// KC-4a: Two patches on different modules are both applied without conflict.
#[test]
fn kill_criterion_4a_parallel_merge_independent_modules() {
    use duumbi::agents::merger::{MergeEngine, ModulePatch};

    let patch_a = ModulePatch {
        module: "calculator/ops".to_string(),
        ops: vec![],
        patched_value: serde_json::json!({
            "@type": "duumbi:Module",
            "duumbi:name": "ops"
        }),
    };
    let patch_b = ModulePatch {
        module: "calculator/display".to_string(),
        ops: vec![],
        patched_value: serde_json::json!({
            "@type": "duumbi:Module",
            "duumbi:name": "display"
        }),
    };

    let result = MergeEngine::merge(vec![patch_a, patch_b]);
    assert_eq!(
        result.applied.len(),
        2,
        "both patches on distinct modules must be applied"
    );
    assert!(
        result.rejected.is_empty(),
        "no patches must be rejected when modules are distinct"
    );
    assert!(
        result.applied.contains(&"calculator/ops".to_string()),
        "ops module must be in applied list"
    );
    assert!(
        result.applied.contains(&"calculator/display".to_string()),
        "display module must be in applied list"
    );
}

/// KC-4b: Two patches targeting the same module are both rejected.
#[test]
fn kill_criterion_4b_parallel_merge_same_module_conflict() {
    use duumbi::agents::merger::{ConflictReason, MergeEngine, ModulePatch};

    let patch_a = ModulePatch {
        module: "calculator/ops".to_string(),
        ops: vec![],
        patched_value: serde_json::json!({}),
    };
    let patch_b = ModulePatch {
        module: "calculator/ops".to_string(),
        ops: vec![],
        patched_value: serde_json::json!({}),
    };

    let result = MergeEngine::merge(vec![patch_a, patch_b]);
    assert!(
        result.applied.is_empty(),
        "same-module conflict must reject both"
    );
    assert_eq!(
        result.rejected.len(),
        2,
        "both patches must be in rejected list"
    );
    for (_, reason) in &result.rejected {
        assert!(
            matches!(reason, ConflictReason::SameModule { .. }),
            "rejection reason must be SameModule, got {reason:?}"
        );
    }
}

/// KC-4c: A single patch is always applied with no rejections.
#[test]
fn kill_criterion_4c_single_patch_always_applied() {
    use duumbi::agents::merger::{MergeEngine, ModulePatch};

    let patch = ModulePatch {
        module: "some/module".to_string(),
        ops: vec![],
        patched_value: serde_json::json!({}),
    };

    let result = MergeEngine::merge(vec![patch]);
    assert_eq!(
        result.applied.len(),
        1,
        "single patch must always be applied"
    );
    assert!(
        result.rejected.is_empty(),
        "single patch must not be rejected"
    );
}

// ---------------------------------------------------------------------------
// Kill Criterion 5 — Budget control
// ---------------------------------------------------------------------------

/// KC-5a: CostTracker allows usage within budget and blocks usage that exceeds it.
#[test]
fn kill_criterion_5a_budget_control_basic() {
    use duumbi::agents::cost::CostTracker;
    use duumbi::config::CostSection;

    let config = CostSection {
        budget_per_intent: 10_000,
        budget_per_session: 200_000,
        max_parallel_agents: 3,
        circuit_breaker_failures: 5,
        alert_threshold_pct: 80,
    };

    let tracker = CostTracker::new(config);

    // Simulate 5 tasks at 500 tokens each — total 2500, well under the 10 000 budget.
    for _ in 0..5 {
        assert!(
            tracker.check_budget().is_ok(),
            "budget check must pass before limit is reached"
        );
        tracker.record_usage(500);
    }
    assert_eq!(
        tracker.intent_usage(),
        2500,
        "intent usage must reflect accumulated tokens"
    );
    assert!(
        tracker.intent_usage() < 10_000,
        "usage must be under budget"
    );
}

/// KC-5b: CostTracker returns BudgetExceeded once the per-intent limit is reached.
#[test]
fn kill_criterion_5b_budget_control_exceeded() {
    use duumbi::agents::cost::CostTracker;
    use duumbi::config::CostSection;

    let config = CostSection {
        budget_per_intent: 10_000,
        budget_per_session: 200_000,
        max_parallel_agents: 3,
        circuit_breaker_failures: 5,
        alert_threshold_pct: 80,
    };

    let tracker = CostTracker::new(config);

    // Start with 2500 tokens (from 5 × 500 as in KC-5a).
    tracker.record_usage(2500);
    assert!(
        tracker.check_budget().is_ok(),
        "2500 tokens is under the 10 000 limit"
    );

    // Push over the budget.
    tracker.record_usage(8000); // total = 10 500
    assert!(
        tracker.check_budget().is_err(),
        "budget check must fail after 10 500 tokens consumed against a 10 000 limit"
    );
    assert_eq!(
        tracker.intent_usage(),
        10_500,
        "intent usage must reflect all recorded tokens"
    );
}

/// KC-5c: reset_intent resets the per-intent counter and allows budget checks to pass again.
#[test]
fn kill_criterion_5c_budget_reset_between_intents() {
    use duumbi::agents::cost::CostTracker;
    use duumbi::config::CostSection;

    let config = CostSection {
        budget_per_intent: 5_000,
        budget_per_session: 100_000,
        max_parallel_agents: 3,
        circuit_breaker_failures: 5,
        alert_threshold_pct: 80,
    };

    let tracker = CostTracker::new(config);
    tracker.record_usage(5_000);
    assert!(
        tracker.check_budget().is_err(),
        "budget must be exceeded after recording 5 000 tokens"
    );

    // Simulate completing the intent and starting a new one.
    tracker.reset_intent();
    assert_eq!(
        tracker.intent_usage(),
        0,
        "intent counter must be zero after reset"
    );
    assert!(
        tracker.check_budget().is_ok(),
        "budget check must pass again after reset"
    );

    // Session counter is NOT reset — it accumulates across intents.
    assert_eq!(
        tracker.session_usage(),
        5_000,
        "session usage must retain tokens from the previous intent"
    );
}
