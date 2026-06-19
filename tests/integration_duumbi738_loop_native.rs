//! DUUMBI-738 native Loop provider integration evidence.

use std::fs;
use std::path::Path;
use std::process::Command;

use duumbi::intent::spec::{IntentBdd, IntentModules, IntentSpec, IntentStatus};
use duumbi::knowledge::store::KnowledgeStore;
use duumbi::knowledge::types::{DecisionRecord, KnowledgeNode, PatternRecord};
use duumbi::loop_native::{
    ChangeSet, ContextSourceKind, LOOP_ARTIFACT_SCHEMA_VERSION, LoopRunState,
    graph_patch_review_target, run_native_intake_spec,
};
use duumbi::patch::{GraphPatch, PatchOp};
use serde_json::{Value, json};
use tempfile::TempDir;

const INTENT_SLUG: &str = "native-loop";

fn seed_native_workspace() -> TempDir {
    let ws = TempDir::new().expect("workspace tempdir");
    let root = ws.path();

    write_file(
        &root.join(".duumbi/graph/main.jsonld"),
        r#"{
  "@context": "https://duumbi.dev/ctx",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": []
}
"#,
    );
    write_file(
        &root.join(".duumbi/deps.lock"),
        r#"schema = 1

[[package]]
name = "duumbi-core"
version = "0.4.1-preview"
"#,
    );
    write_file(
        &root.join(".duumbi/session/current.json"),
        r#"{"session_id":"stage10-native-loop","turns":[{"role":"user"}]}"#,
    );
    write_file(
        &root.join(".duumbi/intents/native-loop/features/native-loop.feature"),
        r#"Feature: Native DUUMBI Loop

  Scenario: Run intake and spec without Git
    Given a DUUMBI intent exists
    When provider-duumbi runs intake and spec
    Then native artifacts are written without Git credentials
"#,
    );

    let spec = IntentSpec {
        intent: "Implement the first DUUMBI-native Loop workflow slice".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![
            "provider-duumbi runs without GitHub or GitLab credentials".to_string(),
            "native intake and spec artifacts are written".to_string(),
            "graph-aware registry context is included".to_string(),
        ],
        modules: IntentModules {
            create: vec!["loop/native".to_string()],
            modify: vec!["cli".to_string()],
        },
        test_cases: Vec::new(),
        dependencies: Vec::new(),
        bdd: IntentBdd {
            feature_files: vec!["features/native-loop.feature".to_string()],
        },
        context: None,
        created_at: Some("2026-06-19T00:00:00Z".to_string()),
        execution: None,
    };
    duumbi::intent::save_intent(root, INTENT_SLUG, &spec).expect("intent should save");

    let store = KnowledgeStore::new(root).expect("knowledge store should initialize");
    let mut decision = DecisionRecord::new("Use provider-duumbi as the native MVP provider.");
    decision.tags.push("architecture".to_string());
    store
        .save_node(&KnowledgeNode::Decision(decision))
        .expect("decision should save");
    let mut pattern = PatternRecord::new(
        "provider-core-vocabulary",
        "Map provider-specific work into provider-neutral Loop objects.",
    );
    pattern.tags.push("candidate".to_string());
    store
        .save_node(&KnowledgeNode::Pattern(pattern))
        .expect("pattern should save");

    ws
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("path should have parent")).expect("create parent");
    fs::write(path, contents).expect("write fixture");
}

fn read_json(path: &Path) -> Value {
    let contents = fs::read_to_string(path).expect("json file should be readable");
    assert!(!contents.contains("api_key"));
    assert!(!contents.contains("github_token"));
    assert!(!contents.contains("gitlab_token"));
    serde_json::from_str(&contents).expect("json should parse")
}

#[test]
fn native_intake_spec_writes_provider_free_artifacts_from_local_context() {
    let ws = seed_native_workspace();

    let result = run_native_intake_spec(ws.path(), INTENT_SLUG).expect("native run should succeed");

    assert_eq!(result.state, LoopRunState::Completed);
    assert!(result.blocking_reasons.is_empty());
    assert_eq!(result.run_id, "duumbi-native-native-loop");
    assert_eq!(result.artifacts.len(), 4);
    assert!(
        result
            .context_summary
            .iter()
            .any(|item| item == "graph_sources=1")
    );
    assert!(
        result
            .context_summary
            .iter()
            .any(|item| item == "bdd_sources=1")
    );
    assert!(
        result
            .context_summary
            .iter()
            .any(|item| item == "knowledge_sources=2")
    );
    assert!(
        result
            .context_summary
            .iter()
            .any(|item| item == "registry_sources=1")
    );
    assert!(!ws.path().join(".git").exists());

    let artifacts_dir = ws
        .path()
        .join(".duumbi/loop/runs/duumbi-native-native-loop/artifacts");
    let intake = read_json(&artifacts_dir.join("intake.json"));
    assert_eq!(intake["schema_version"], LOOP_ARTIFACT_SCHEMA_VERSION);
    assert_eq!(intake["provider_kind"], "duumbi");
    assert_eq!(intake["body"]["estimated_credits"], 0);
    assert!(intake["body"]["resolved_provider"].is_null());
    assert!(intake["body"]["graph_semantic_hash"].is_string());
    assert!(
        intake["sources"]
            .as_array()
            .expect("sources should be an array")
            .iter()
            .any(|source| source["kind"] == "registry")
    );
    assert!(
        intake["sources"]
            .as_array()
            .expect("sources should be an array")
            .iter()
            .any(|source| source["kind"] == "knowledge")
    );

    let metadata = read_json(&artifacts_dir.join("metadata.json"));
    assert_eq!(metadata["body"]["resource_policy"]["external_llm_calls"], 0);
    assert_eq!(
        metadata["body"]["resource_policy"]["git_provider_required"],
        false
    );
    assert_eq!(
        metadata["body"]["resource_policy"]["spec_model_label"],
        "balanced"
    );
    assert_eq!(
        metadata["body"]["resource_policy"]["review_model_label"],
        "strict_review"
    );
    let product_spec = read_json(&artifacts_dir.join("product_spec.json"));
    assert_eq!(product_spec["schema_version"], LOOP_ARTIFACT_SCHEMA_VERSION);
    assert_eq!(product_spec["artifact_kind"], "product_spec");
    assert_eq!(product_spec["provider_kind"], "duumbi");
    assert!(
        product_spec["links"]
            .as_array()
            .expect("links should be an array")
            .iter()
            .any(|link| link["path"]
                == ".duumbi/loop/runs/duumbi-native-native-loop/artifacts/product_spec.md")
    );
    assert!(
        product_spec["body"]["bdd_scenarios"]
            .as_array()
            .expect("bdd scenarios should be an array")
            .iter()
            .any(|scenario| scenario == "Run intake and spec without Git")
    );
    let technical_spec = read_json(&artifacts_dir.join("technical_spec.json"));
    assert_eq!(
        technical_spec["schema_version"],
        LOOP_ARTIFACT_SCHEMA_VERSION
    );
    assert_eq!(technical_spec["artifact_kind"], "technical_spec");
    assert_eq!(
        technical_spec["body"]["resource_policy"]["external_llm_calls"],
        0
    );
    assert!(artifacts_dir.join("product_spec.md").exists());
    assert!(artifacts_dir.join("technical_spec.md").exists());
}

#[test]
fn native_loop_cli_emits_json_result_without_git_provider_setup() {
    let ws = seed_native_workspace();

    let output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .args(["loop", "intake-spec", INTENT_SLUG, "--json"])
        .current_dir(ws.path())
        .output()
        .expect("duumbi loop intake-spec should run");

    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(stdout["state"], "completed");
    assert_eq!(stdout["run_id"], "duumbi-native-native-loop");
    assert_eq!(stdout["artifacts"].as_array().expect("artifacts").len(), 4);
}

#[test]
fn graph_patch_review_target_maps_affected_nodes_without_applying_patch() {
    let ws = seed_native_workspace();
    let patch = GraphPatch {
        ops: vec![
            PatchOp::ModifyOp {
                node_id: "duumbi:main/b".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(1),
            },
            PatchOp::SetEdge {
                node_id: "duumbi:main/a".to_string(),
                field: "duumbi:next".to_string(),
                target_id: "duumbi:main/c".to_string(),
            },
        ],
    };

    let target =
        graph_patch_review_target(ws.path(), INTENT_SLUG, &patch).expect("target should build");

    assert_eq!(target.work_item_id, "duumbi:intent/native-loop");
    assert!(
        target
            .sources
            .iter()
            .any(|source| source.kind == ContextSourceKind::Graph)
    );
    match target.change_set {
        ChangeSet::GraphPatch {
            operation_count,
            affected_nodes,
        } => {
            assert_eq!(operation_count, 2);
            assert_eq!(
                affected_nodes,
                vec![
                    "duumbi:main/a".to_string(),
                    "duumbi:main/b".to_string(),
                    "duumbi:main/c".to_string(),
                ]
            );
        }
        other => panic!("unexpected change set: {other:?}"),
    }
}

#[test]
fn missing_explicit_bdd_reference_blocks_before_writing_artifacts() {
    let ws = seed_native_workspace();
    fs::remove_file(
        ws.path()
            .join(".duumbi/intents/native-loop/features/native-loop.feature"),
    )
    .expect("feature fixture should remove");

    let result = run_native_intake_spec(ws.path(), INTENT_SLUG).expect("blocked run should return");

    assert_eq!(result.state, LoopRunState::Blocked);
    assert!(result.artifacts.is_empty());
    assert!(
        result
            .blocking_reasons
            .iter()
            .any(|reason| reason.contains("E_BDD_FILE_MISSING"))
    );
    assert!(
        !ws.path()
            .join(".duumbi/loop/runs/duumbi-native-native-loop/artifacts")
            .exists()
    );
}

#[test]
fn graph_source_walk_is_depth_bounded() {
    let ws = seed_native_workspace();
    let mut deep_dir = ws.path().join(".duumbi/graph");
    for level in 0..20 {
        deep_dir = deep_dir.join(format!("level-{level}"));
        fs::create_dir_all(&deep_dir).expect("deep graph dir should create");
    }
    write_file(
        &deep_dir.join("deep.jsonld"),
        r#"{
  "@context": "https://duumbi.dev/ctx",
  "@id": "duumbi:deep",
  "duumbi:name": "deep",
  "duumbi:functions": []
}
"#,
    );

    let result =
        run_native_intake_spec(ws.path(), INTENT_SLUG).expect("native run should complete");

    assert_eq!(result.state, LoopRunState::Completed);
    assert!(
        result
            .context_summary
            .iter()
            .any(|item| item == "graph_sources=1")
    );
}

#[cfg(unix)]
#[test]
fn graph_source_walk_skips_symlink_cycle() {
    let ws = seed_native_workspace();
    std::os::unix::fs::symlink(".", ws.path().join(".duumbi/graph/cycle"))
        .expect("graph symlink should create");

    let result =
        run_native_intake_spec(ws.path(), INTENT_SLUG).expect("native run should complete");

    assert_eq!(result.state, LoopRunState::Completed);
    assert!(
        result
            .context_summary
            .iter()
            .any(|item| item == "graph_sources=1")
    );
}

#[test]
fn loop_cli_help_keeps_git_providers_optional() {
    let output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .args(["loop", "--help"])
        .output()
        .expect("duumbi loop --help should run");

    assert!(
        output.status.success(),
        "help command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Run native DUUMBI Loop workflows"));
    assert!(stdout.contains("intake-spec"));
    assert!(stdout.contains("review-patch"));
    assert!(!stdout.contains("GitHub credential"));
    assert!(!stdout.contains("GitLab credential"));
    assert!(!stdout.contains("pull request"));
    assert!(!stdout.contains("merge request"));
}
