//! DUUMBI-720 determinism replay integration evidence.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use duumbi::bench::report::ProviderUsageSummary;
use duumbi::determinism::digest::safe_artifact_key;
use duumbi::determinism::evidence::{
    LedgerEvent, LedgerEventKind, ModelIdentity, PromptHashes, REPLAY_LEDGER_SCHEMA_VERSION,
    REPLAY_REPORT_SCHEMA_VERSION, ReplayAttempt, ReplayEnvironment, ReplayInputs, ReplayMetrics,
    ReplayReport,
};
use duumbi::determinism::ledger::LedgerWriter;
use duumbi::determinism::metrics::AgreementRate;
use serde_json::Value;
use tempfile::TempDir;

fn sample_attempt(attempt: u32, exact: &str, semantic: &str) -> ReplayAttempt {
    ReplayAttempt {
        task_id: "calculator".to_string(),
        suite: "core".to_string(),
        tags: vec!["smoke".to_string()],
        provider: "mock".to_string(),
        model_identity: ModelIdentity::Available {
            label: "mock-model".to_string(),
        },
        attempt,
        workspace_strategy: "tempdir-per-attempt".to_string(),
        initial_graph_exact_hash: Some("initial-exact".to_string()),
        initial_graph_semantic_hash: Some("initial-semantic".to_string()),
        final_graph_exact_hash: Some(exact.to_string()),
        final_graph_semantic_hash: Some(semantic.to_string()),
        intent_spec_hash: Some("intent-spec".to_string()),
        bdd_context_hash: Some("bdd-context".to_string()),
        context_pack_hash: Some("context-pack".to_string()),
        prompt_hashes: PromptHashes::Partial {
            reason: "final provider prompt capture unavailable".to_string(),
            hashes: [("context_pack".to_string(), "context-pack".to_string())].into(),
        },
        success: true,
        tests_passed: 1,
        tests_total: 1,
        bdd_readiness: Some("ready".to_string()),
        bdd_coverage: vec!["exact-graph".to_string(), "behavior".to_string()],
        behavior_signature: Some("behavior-pass".to_string()),
        error_category: None,
        dominant_error_code: None,
        provider_usage: ProviderUsageSummary::unavailable("provider usage unavailable"),
        benchmark_evidence: None,
        artifact_paths: vec!["attempts/calculator/mock/1".to_string()],
        duration_secs: 0.1,
    }
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).expect("json evidence should be readable");
    assert!(!text.contains("api_key"));
    serde_json::from_str(&text).expect("json evidence should parse")
}

#[test]
fn determinism_replay_cli_exposes_reviewable_options_without_provider_calls() {
    let output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .args(["determinism", "replay", "--help"])
        .output()
        .expect("duumbi determinism replay --help should run");

    assert!(
        output.status.success(),
        "help command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Replay selected benchmark showcases and report agreement metrics"));
    assert!(stdout.contains("--suite"));
    assert!(stdout.contains("--provider"));
    assert!(stdout.contains("--attempts"));
    assert!(stdout.contains("--markdown-output"));
    assert!(stdout.contains("--min-exact-agreement"));
    assert!(stdout.contains("--keep-workspaces"));
}

#[test]
fn replay_report_json_and_markdown_keep_schema_and_agreement_evidence() {
    let mut report = ReplayReport::new(
        "run-720",
        "2026-06-19T00:00:00Z",
        "2026-06-19T00:00:01Z",
        "0.4.0-preview",
        "abc123",
        ReplayInputs {
            suite: "core".to_string(),
            smoke: true,
            showcases: vec!["calculator".to_string()],
            providers: vec!["mock".to_string()],
            attempts: 2,
        },
        ReplayEnvironment {
            provider_source: "test".to_string(),
            registry_state_hash: "registry".to_string(),
            lockfile_hash: "absent".to_string(),
            workspace_dependency_config_hash: "absent".to_string(),
        },
    );
    report.attempts = vec![
        sample_attempt(1, "exact-a", "semantic-a"),
        sample_attempt(2, "exact-b", "semantic-a"),
    ];
    report.metrics = ReplayMetrics::from_attempts(&report.attempts);

    let json = serde_json::to_value(&report).expect("report should serialize");
    assert_eq!(json["schema_version"], REPLAY_REPORT_SCHEMA_VERSION);
    assert_eq!(json["attempts"][0]["prompt_hashes"]["status"], "partial");
    assert_eq!(
        json["metrics"]["semantic_graph_agreement_rate"]["status"],
        "available"
    );
    assert_eq!(
        json["metrics"]["behavioral_agreement_rate"]["status"],
        "available"
    );
    assert!(!json.to_string().contains("api_key"));

    assert!(matches!(
        report.metrics.exact_graph_agreement_rate,
        AgreementRate::Available { rate, .. } if rate == 0.5
    ));
    assert!(matches!(
        report.metrics.semantic_graph_agreement_rate,
        AgreementRate::Available { rate, .. } if rate == 1.0
    ));

    let markdown = report.to_markdown_summary();
    assert!(markdown.contains("# DUUMBI Determinism Replay Report"));
    assert!(markdown.contains("| Exact graph | available |"));
    assert!(markdown.contains("| Semantic graph | available |"));
    assert!(markdown.contains("| calculator | mock | mock-model | 1 | true | 1/1 | none |"));
}

#[test]
fn ledger_writer_appends_schema_versioned_events_as_jsonl() {
    let temp_dir = TempDir::new().expect("temp dir");
    let ledger_path = temp_dir.path().join("ledger.jsonl");
    let mut writer = LedgerWriter::open(&ledger_path).expect("ledger should open");

    writer
        .append(&LedgerEvent::new(
            "run-720",
            LedgerEventKind::RunStarted,
            1,
            "2026-06-19T00:00:00Z",
            serde_json::json!({"suite": "core"}),
        ))
        .expect("run_started should write");
    writer
        .append(&LedgerEvent::new(
            "run-720",
            LedgerEventKind::AttemptCompleted,
            2,
            "2026-06-19T00:00:01Z",
            serde_json::json!({"task_id": "calculator", "success": true}),
        ))
        .expect("attempt_completed should write");

    let contents = fs::read_to_string(&ledger_path).expect("ledger should be readable");
    let lines: Vec<_> = contents.lines().collect();
    assert_eq!(lines.len(), 2);

    let first: Value = serde_json::from_str(lines[0]).expect("first event should parse");
    let second: Value = serde_json::from_str(lines[1]).expect("second event should parse");
    assert_eq!(first["schema_version"], REPLAY_LEDGER_SCHEMA_VERSION);
    assert_eq!(first["event"], "run_started");
    assert_eq!(second["event"], "attempt_completed");
    assert!(!contents.contains("api_key"));
}

#[test]
fn replay_artifact_keys_are_safe_path_components() {
    let key = safe_artifact_key(
        "minimax:http://localhost:8080/v1/../../secret?api_key=hidden",
        "provider",
    );
    let path = PathBuf::from(&key);

    assert_eq!(path.components().count(), 1);
    assert!(!key.contains('/'));
    assert!(!key.contains(".."));
    assert!(!key.contains("api_key"));
    assert!(key.starts_with("minimax-http-localhost-8080-v1-secret-"));
}

#[test]
fn persisted_report_stays_compact_parseable_and_secret_free() {
    let temp_dir = TempDir::new().expect("temp dir");
    let report_path = temp_dir.path().join("report.json");
    let report = ReplayReport::new(
        "run-720",
        "2026-06-19T00:00:00Z",
        "2026-06-19T00:00:01Z",
        "0.4.0-preview",
        "abc123",
        ReplayInputs {
            suite: "core".to_string(),
            smoke: true,
            showcases: vec!["calculator".to_string()],
            providers: vec!["mock".to_string()],
            attempts: 2,
        },
        ReplayEnvironment {
            provider_source: "test".to_string(),
            registry_state_hash: "registry".to_string(),
            lockfile_hash: "absent".to_string(),
            workspace_dependency_config_hash: "absent".to_string(),
        },
    );

    fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("report serializes"),
    )
    .expect("report writes");

    let metadata = fs::metadata(&report_path).expect("report metadata");
    assert!(metadata.len() < 64 * 1024);
    let json = read_json(&report_path);
    assert_eq!(json["schema_version"], REPLAY_REPORT_SCHEMA_VERSION);
    assert_eq!(json["run_id"], "run-720");
}
