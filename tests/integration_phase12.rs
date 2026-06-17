//! Phase 12 integration tests — kill criterion verification.
//!
//! These tests exercise the five Phase 12 kill criteria without requiring
//! LLM API keys or network access. All operations are purely in-process.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

fn read_optional(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

fn snapshot(paths: &[PathBuf]) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    paths
        .iter()
        .map(|path| (path.clone(), read_optional(path)))
        .collect()
}

fn assert_snapshot_unchanged(before: &[(PathBuf, Option<Vec<u8>>)]) {
    for (path, bytes) in before {
        assert_eq!(
            read_optional(path),
            *bytes,
            "file must be unchanged: {}",
            path.display()
        );
    }
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

/// DUUMBI-682: model telemetry MCP analytics are discoverable, callable, and read-only.
#[test]
fn duumbi682_model_telemetry_tools_do_not_mutate_local_state() {
    use duumbi::mcp::server::{JsonRpcRequest, McpServer};

    let _guard = ENV_LOCK.lock().expect("invariant: env lock");
    let original_home = std::env::var_os("HOME");
    let workspace = setup_workspace();
    let home = TempDir::new().expect("invariant: temp home");
    unsafe {
        std::env::set_var("HOME", home.path());
    }

    let access_dir = home.path().join(".duumbi/knowledge/model-access");
    std::fs::create_dir_all(&access_dir).expect("create access dir");
    let access_current = access_dir.join("current.json");
    let access_events = access_dir.join("events.jsonl");
    std::fs::write(
        &access_current,
        serde_json::json!({
            "records": {
                "sha256:secret|minimax|MiniMax-M2.7": {
                    "credentialFingerprint": "sha256:secret",
                    "provider": "minimax",
                    "model": "MiniMax-M2.7",
                    "status": "accessible",
                    "reasonCode": null,
                    "message": "provider message",
                    "probeVersion": "test",
                    "lastChecked": "2026-06-13T10:00:00Z",
                    "lastSuccess": "2026-06-13T10:00:00Z"
                }
            }
        })
        .to_string(),
    )
    .expect("write access current");
    std::fs::write(
        &access_events,
        r#"{"credentialFingerprint":"sha256:secret","provider":"minimax","model":"MiniMax-M2.7","status":"accessible","reasonCode":null,"message":"provider message","checkedAt":"2026-06-13T10:00:00Z"}"#,
    )
    .expect("write access events");

    let performance_dir = workspace.path().join(".duumbi/knowledge/model-performance");
    std::fs::create_dir_all(&performance_dir).expect("create performance dir");
    let performance_aggregates = performance_dir.join("aggregates.json");
    let performance_events = performance_dir.join("events.jsonl");
    std::fs::write(
        &performance_aggregates,
        serde_json::json!({
            "aggregates": {
                "anthropic|claude-sonnet|coder|create|simple|single|high": {
                    "calls": 2,
                    "successes": 1,
                    "failures": 1,
                    "ewmaLatencyMs": 100.0,
                    "ewmaCostUsd": 0.01,
                    "parseFailures": 0,
                    "validationFailures": 1,
                    "retries": 1,
                    "lastUpdated": "2026-06-13T10:00:00Z"
                }
            }
        })
        .to_string(),
    )
    .expect("write aggregates");
    std::fs::write(
        &performance_events,
        r#"{"timestamp":"2026-06-13T10:00:00Z","provider":"anthropic","model":"claude-sonnet","agentRole":"coder","templateVersion":"v1","taskType":"create","complexity":"simple","scope":"single","risk":"high","promptTokens":10,"completionTokens":5,"reasoningTokens":null,"latencyMs":100,"firstTokenLatencyMs":25,"costUsd":0.01,"toolParseSuccess":true,"patchCount":1,"validationErrors":[],"retries":0,"outcome":"success"}"#,
    )
    .expect("write performance events");

    let intent_dir = workspace.path().join(".duumbi/intents");
    std::fs::create_dir_all(&intent_dir).expect("create intents dir");
    let intent_file = intent_dir.join("example.yaml");
    std::fs::write(&intent_file, "intent: example\n").expect("write intent");
    let catalog_file = workspace.path().join(".duumbi/model-catalog.json");
    std::fs::write(&catalog_file, "{}").expect("write catalog");
    let credentials = home.path().join(".duumbi/credentials.toml");
    std::fs::write(&credentials, "[registries]\n").expect("write credentials");

    let watched = vec![
        access_current,
        access_events,
        performance_aggregates,
        performance_events,
        workspace.path().join(".duumbi/config.toml"),
        workspace.path().join(".duumbi/graph/main.jsonld"),
        intent_file,
        catalog_file,
        credentials,
    ];
    let before = snapshot(&watched);
    let server = McpServer::new(workspace.path().to_path_buf());

    let tools_response = server
        .handle_request(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".to_string(),
            params: None,
        })
        .expect("tools/list response");
    let tools = tools_response.result.expect("tools result");
    assert!(
        tools["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .any(|tool| tool["name"] == "model_access_summary")
    );

    for (id, name, arguments) in [
        (
            2,
            "model_access_summary",
            serde_json::json!({"include_raw_events": true, "limit": 1}),
        ),
        (
            3,
            "model_performance_summary",
            serde_json::json!({"task_type": "create", "risk": "high", "limit": 10}),
        ),
        (4, "model_telemetry_health", serde_json::json!({})),
    ] {
        let response = server
            .handle_request(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(serde_json::json!(id)),
                method: "tools/call".to_string(),
                params: Some(serde_json::json!({
                    "name": name,
                    "arguments": arguments,
                })),
            })
            .expect("tools/call response");
        assert!(response.error.is_none(), "{name} should succeed");
        let text = response.result.expect("result")["content"][0]["text"]
            .as_str()
            .expect("text content")
            .to_string();
        assert!(!text.contains("sha256:secret"));
        assert!(!text.contains("provider message"));
    }

    assert_snapshot_unchanged(&before);

    unsafe {
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }
}

/// DUUMBI-719 Cycle 1: MCP exposes agent-facing capability metadata and status.
#[test]
fn duumbi719_mcp_capability_status_is_discoverable_and_read_only() {
    use duumbi::mcp::server::{JsonRpcRequest, McpServer};

    let workspace = setup_workspace();
    let watched = vec![
        workspace.path().join(".duumbi/config.toml"),
        workspace.path().join(".duumbi/graph/main.jsonld"),
    ];
    let before = snapshot(&watched);
    let server = McpServer::new(workspace.path().to_path_buf());

    let init = server
        .handle_request(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        })
        .expect("initialize response");
    assert!(init.error.is_none(), "initialize must succeed");
    assert_eq!(
        init.result.expect("initialize result")["serverInfo"]["name"],
        "duumbi-mcp"
    );

    let tools_response = server
        .handle_request(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".to_string(),
            params: None,
        })
        .expect("tools/list response");
    assert!(tools_response.error.is_none(), "tools/list must succeed");
    let tools = tools_response.result.expect("tools result");
    let tool_values = tools["tools"].as_array().expect("tools array");
    for expected in [
        "mcp_capability_status",
        "query_ask",
        "graph_patch_preview",
        "graph_patch_request_approval",
        "approval_status",
        "approval_decide",
        "graph_patch_apply_approval",
        "graph_query",
        "graph_mutate",
        "graph_validate",
        "build_compile",
        "build_run",
        "intent_create",
        "intent_execute",
        "rewrite_preview",
        "rewrite_apply",
    ] {
        assert!(
            tool_values.iter().any(|tool| tool["name"] == expected),
            "{expected} must be discoverable"
        );
    }
    assert!(
        tool_values
            .iter()
            .any(|tool| tool["name"] == "graph_query" && tool["duumbi"]["safety"] == "read_only"),
        "tools/list must include read-only DUUMBI metadata"
    );
    assert!(
        tool_values.iter().any(|tool| tool["name"] == "graph_mutate"
            && tool["duumbi"]["safety"] == "trusted_immediate_write"),
        "legacy immediate write tools must be clearly labeled"
    );

    let status_response = server
        .handle_request(&JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "mcp_capability_status",
                "arguments": {},
            })),
        })
        .expect("status response");
    assert!(status_response.error.is_none(), "status tool must succeed");
    let status_text = status_response.result.expect("status result")["content"][0]["text"]
        .as_str()
        .expect("status text")
        .to_string();
    let status: serde_json::Value = serde_json::from_str(&status_text).expect("status JSON");
    assert_eq!(status["workspace"]["duumbiInitialized"], true);
    assert_eq!(status["workspace"]["mainGraphPresent"], true);
    assert!(
        status["capabilities"]["unavailableTools"]
            .as_array()
            .expect("unavailable tools")
            .iter()
            .any(|tool| tool["name"] == "build_compile"),
        "stubbed workflow tools must expose structured unavailable state"
    );

    assert_snapshot_unchanged(&before);
}

/// DUUMBI-719 Cycle 3: approval-gated graph patch flow works through MCP dispatch.
#[test]
fn duumbi719_mcp_graph_patch_approval_applies_exact_candidate() {
    use duumbi::mcp::server::{JsonRpcRequest, McpServer};

    fn call_tool(
        server: &McpServer,
        id: u64,
        name: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        let response = server
            .handle_request(&JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(serde_json::json!(id)),
                method: "tools/call".to_string(),
                params: Some(serde_json::json!({
                    "name": name,
                    "arguments": arguments,
                })),
            })
            .expect("tools/call response");
        assert!(
            response.error.is_none(),
            "{name} should succeed: {:?}",
            response.error
        );
        let text = response.result.expect("result")["content"][0]["text"]
            .as_str()
            .expect("text content")
            .to_string();
        serde_json::from_str(&text).expect("tool JSON")
    }

    let workspace = setup_workspace();
    let graph_path = workspace.path().join(".duumbi/graph/main.jsonld");
    let before = std::fs::read_to_string(&graph_path).expect("read graph before");
    let server = McpServer::new(workspace.path().to_path_buf());
    let ops = serde_json::json!([{
        "kind": "modify_op",
        "node_id": "duumbi:main/main/entry/0",
        "field": "duumbi:value",
        "value": 9
    }]);

    let preview = call_tool(
        &server,
        1,
        "graph_patch_preview",
        serde_json::json!({ "ops": ops.clone() }),
    );
    assert_eq!(preview["read_only"], true);
    assert_eq!(
        std::fs::read_to_string(&graph_path).expect("read graph after preview"),
        before,
        "preview must not write graph"
    );

    let requested = call_tool(
        &server,
        2,
        "graph_patch_request_approval",
        serde_json::json!({
            "ops": ops,
            "summary": "Change main return value to nine"
        }),
    );
    let approval_id = requested["approval"]["id"]
        .as_str()
        .expect("approval id")
        .to_string();
    assert_eq!(requested["approval"]["status"], "pending");
    assert_eq!(
        std::fs::read_to_string(&graph_path).expect("read graph after request"),
        before,
        "approval request must not write graph"
    );

    let approved = call_tool(
        &server,
        3,
        "approval_decide",
        serde_json::json!({
            "id": approval_id,
            "decision": "approve",
            "decision_source": "integration_test"
        }),
    );
    assert_eq!(approved["approval"]["status"], "approved");

    let applied = call_tool(
        &server,
        4,
        "graph_patch_apply_approval",
        serde_json::json!({ "id": approval_id }),
    );
    assert_eq!(applied["approval"]["status"], "applied");
    let after = std::fs::read_to_string(&graph_path).expect("read graph after apply");
    assert!(after.contains("\"duumbi:value\": 9"));
}

/// DUUMBI-719 Cycle 4: agent-facing docs match the implemented MCP surface and workflow rules.
#[test]
fn duumbi719_agent_docs_cover_current_mcp_surface() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        repo.join("docs/agents/mcp-workflow-audit.md"),
        repo.join("docs/agents/mcp-error-contract.md"),
        repo.join("docs/agents/mcp-agent-guide.md"),
        repo.join("docs/e2e/duumbi-719-mcp-agent-benchmark.md"),
        repo.join("docs/agents/claude-code-duumbi-skill/SKILL.md"),
    ];

    let mut combined = String::new();
    for doc in docs {
        assert!(
            doc.exists(),
            "required DUUMBI-719 doc missing: {}",
            doc.display()
        );
        let contents = std::fs::read_to_string(&doc)
            .unwrap_or_else(|err| panic!("read {}: {err}", doc.display()));
        assert!(
            contents.contains("Related to #719") || contents.contains("duumbi-mcp-agent"),
            "doc must use non-closing issue context or skill metadata: {}",
            doc.display()
        );
        combined.push_str(&contents);
        combined.push('\n');
    }

    for required in [
        "target/debug/duumbi mcp",
        "mcp_capability_status",
        "query_ask",
        "graph_patch_preview",
        "graph_patch_request_approval",
        "approval_status",
        "approval_decide",
        "graph_patch_apply_approval",
        "error.data",
        "provider_unavailable",
        "approval_stale",
        "examples/flagship-http-sqlite-json",
    ] {
        assert!(
            combined.contains(required),
            "DUUMBI-719 docs must mention {required}"
        );
    }

    let lower = combined.to_lowercase();
    for forbidden in [
        "closes #719",
        "fixes #719",
        "resolves #719",
        "api_key =",
        "authorization: bearer",
        "password =",
        "greptile",
    ] {
        assert!(
            !lower.contains(forbidden),
            "DUUMBI-719 docs must not contain forbidden text: {forbidden}"
        );
    }
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
        bdd: Default::default(),
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
