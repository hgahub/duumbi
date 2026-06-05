//! Telemetry integration tests for traced local build/run evidence.

use std::fs;
use std::process::Command;

use duumbi::telemetry::{TraceMap, TraceMapKind};
use serde::Deserialize;

#[test]
fn traced_option_none_unwrap_writes_crash_evidence_and_inspects() {
    let evidence = traced_fixture_evidence(
        "tests/fixtures/telemetry/option_none_unwrap.jsonld",
        "panic",
    );

    let stdout = &evidence.inspect_stdout;
    assert!(stdout.contains("Function: duumbi:telemetry/main"));
    assert!(stdout.contains("Block: duumbi:telemetry/main/entry"));
    assert!(stdout.contains("Exact node evidence: unavailable in v1"));

    assert_trace_events_join_trace_map(&evidence, &["function_enter", "block_enter"]);
    assert_latest_crash_joins_trace_map(
        &evidence,
        "duumbi:telemetry/main",
        "duumbi:telemetry/main/entry",
    );
}

#[test]
fn traced_option_none_unwrap_emits_repair_context() {
    let evidence = traced_fixture_evidence(
        "tests/fixtures/telemetry/option_none_unwrap.jsonld",
        "panic_repair_context",
    );
    let repair_context_stdout = repair_context_stdout(
        &evidence,
        "tests/fixtures/telemetry/option_none_unwrap.jsonld",
    );

    let context: serde_json::Value =
        serde_json::from_str(&repair_context_stdout).expect("repair context stdout must be JSON");

    assert_eq!(
        context["schema_version"],
        serde_json::json!("duumbi.telemetry.repair_context.v1")
    );
    assert_eq!(
        context["crash_message"],
        serde_json::json!("called Option::unwrap() on a None value")
    );
    assert_eq!(
        context["function_id"],
        serde_json::json!("duumbi:telemetry/main")
    );
    assert_eq!(
        context["block_id"],
        serde_json::json!("duumbi:telemetry/main/entry")
    );
    assert_eq!(context["exact_node_id"], serde_json::Value::Null);
    assert_eq!(
        context["graph_context"]["context_limit"],
        serde_json::json!("containing_function_and_selected_block")
    );
    assert_eq!(
        context["evidence"]["source"],
        serde_json::json!("local_telemetry_artifacts")
    );
    assert_eq!(
        context["evidence"]["selected_crash_line"],
        serde_json::json!(1)
    );
    assert_eq!(
        context["evidence"]["selection"],
        serde_json::json!("latest")
    );
    assert_eq!(context["human_review_required"], serde_json::json!(true));
    assert!(
        context["validation_expectations"]
            .as_array()
            .expect("validation expectations must be an array")
            .iter()
            .any(|expectation| expectation == "proposed patch parses as GraphPatch")
    );
    assert!(
        context["test_expectations"]
            .as_array()
            .expect("test expectations must be an array")
            .iter()
            .any(|expectation| expectation == "default untraced build behavior remains unchanged")
    );

    let serialized = repair_context_stdout.as_str();
    assert!(!serialized.contains("heap"));
    assert!(!serialized.contains("stack"));
    assert!(!serialized.contains("runtime_value"));
    assert!(!serialized.contains("value_snapshot"));
}

#[test]
fn traced_option_none_unwrap_repair_validate_emits_reviewable_evidence() {
    let fixture = "tests/fixtures/telemetry/option_none_unwrap.jsonld";
    let evidence = traced_fixture_evidence(fixture, "panic_repair_validate");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let context_path = tmp.path().join("repair-context.json");
    let patch_path = tmp.path().join("repair-patch.json");
    let output_path = tmp.path().join("repair-validation.json");
    fs::write(&context_path, repair_context_stdout(&evidence, fixture))
        .expect("invariant: repair context must be written");
    fs::write(&patch_path, repair_patch_json()).expect("invariant: repair patch must be written");
    let original_bytes = fs::read(fixture).expect("invariant: fixture must be readable");
    let duumbi = env!("CARGO_BIN_EXE_duumbi");

    let repair_validate = Command::new(duumbi)
        .args(["telemetry", "repair-validate", "--context"])
        .arg(&context_path)
        .arg("--patch")
        .arg(&patch_path)
        .arg("--graph")
        .arg(fixture)
        .arg("--test")
        .arg("{candidate_binary}")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("invariant: telemetry repair-validate must run");

    assert!(
        repair_validate.status.success(),
        "telemetry repair-validate failed: {}",
        String::from_utf8_lossy(&repair_validate.stderr)
    );
    assert_eq!(
        fs::read(fixture).expect("invariant: fixture must remain readable"),
        original_bytes,
        "repair validation must not modify the original graph source"
    );

    let stdout = String::from_utf8(repair_validate.stdout)
        .expect("invariant: repair-validate stdout must be UTF-8");
    let written = fs::read_to_string(&output_path)
        .expect("invariant: repair validation output must be written");
    let stdout_evidence: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout evidence must be JSON");
    let file_evidence: serde_json::Value =
        serde_json::from_str(&written).expect("file evidence must be JSON");

    assert_eq!(stdout_evidence, file_evidence);
    assert_eq!(
        stdout_evidence["schema_version"],
        serde_json::json!("duumbi.telemetry.repair_validation.v1")
    );
    assert_eq!(
        stdout_evidence["source_artifact"]["paths"][0],
        serde_json::json!(fixture)
    );
    assert_eq!(
        stdout_evidence["patch_summary"]["operation_count"],
        serde_json::json!(1)
    );
    assert_eq!(
        stdout_evidence["local_validation_passed"],
        serde_json::json!(true)
    );
    assert_eq!(
        stdout_evidence["requires_human_review"],
        serde_json::json!(true)
    );
    assert_eq!(
        stdout_evidence["accepted_for_application"],
        serde_json::json!(false)
    );
    assert_eq!(
        stdout_evidence["human_review_state"],
        serde_json::json!("pending")
    );
    assert_eq!(
        stdout_evidence["rebuild_summary"]["status"],
        serde_json::json!("passed")
    );
    assert_eq!(
        stdout_evidence["test_summary"]["status"],
        serde_json::json!("passed")
    );
    assert_required_gates_passed(&stdout_evidence);
}

#[test]
fn traced_call_then_panic_preserves_caller_context() {
    let evidence = traced_fixture_evidence(
        "tests/fixtures/telemetry/call_then_panic.jsonld",
        "call_then_panic",
    );

    let stdout = &evidence.inspect_stdout;
    assert!(stdout.contains("Function: duumbi:telemetry_call/main"));
    assert!(stdout.contains("Block: duumbi:telemetry_call/main/entry"));
    assert!(!stdout.contains("Function: duumbi:telemetry_call/helper"));
    assert!(!stdout.contains("Block: duumbi:telemetry_call/helper/entry"));

    assert_trace_events_join_trace_map(
        &evidence,
        &[
            "function_enter",
            "function_exit",
            "block_enter",
            "block_exit",
        ],
    );
    assert_latest_crash_joins_trace_map(
        &evidence,
        "duumbi:telemetry_call/main",
        "duumbi:telemetry_call/main/entry",
    );
}

#[test]
fn default_untraced_option_none_unwrap_is_not_back_mapping_proof() {
    let evidence = untraced_fixture_failure(
        "tests/fixtures/telemetry/option_none_unwrap.jsonld",
        "untraced_panic",
    );

    assert!(
        !evidence.telemetry_dir.join("trace_map.json").exists(),
        "default untraced build must not write trace map evidence"
    );
    assert!(
        !evidence.telemetry_dir.join("traces.jsonl").exists(),
        "default untraced run must not write trace event evidence"
    );
    assert!(
        !evidence.telemetry_dir.join("crash_dump.jsonl").exists(),
        "default untraced run must not write crash back-mapping evidence"
    );

    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let inspect = Command::new(duumbi)
        .args([
            "telemetry",
            "inspect",
            "--telemetry-dir",
            evidence
                .telemetry_dir
                .to_str()
                .expect("invariant: telemetry path must be UTF-8"),
        ])
        .output()
        .expect("invariant: telemetry inspect must run");

    assert!(
        !inspect.status.success(),
        "untraced failure must not inspect as mapped telemetry evidence"
    );
    let stderr = String::from_utf8_lossy(&inspect.stderr);
    assert!(
        stderr.contains("Failed to read crash artifact") && stderr.contains("crash_dump.jsonl"),
        "untraced failure should fail because crash evidence is missing, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        !stdout.contains("Function:") && !stdout.contains("Block:"),
        "untraced failure must not report mapped graph context, got: {stdout}"
    );
}

#[test]
fn workspace_run_uses_configured_telemetry_artifact_dir() {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let workspace = tmp.path().join("workspace");
    let graph_dir = workspace.join(".duumbi/graph");
    let build_dir = workspace.join(".duumbi/build");
    fs::create_dir_all(&graph_dir).expect("invariant: graph dir must be created");
    fs::create_dir_all(&build_dir).expect("invariant: build dir must be created");

    let fixture = "tests/fixtures/telemetry/option_none_unwrap.jsonld";
    let original_graph = fs::read(fixture).expect("invariant: fixture must be readable");
    let graph_path = graph_dir.join("main.jsonld");
    fs::write(&graph_path, &original_graph).expect("invariant: graph fixture must be copied");
    fs::write(
        workspace.join(".duumbi/config.toml"),
        "[workspace]\nname = \"telemetry-config-e2e\"\n\n[telemetry]\nartifact-dir = \"custom/telemetry\"\n",
    )
    .expect("invariant: workspace config must be written");

    let build = Command::new(duumbi)
        .args(["build", "--trace", "-o", ".duumbi/build/output"])
        .current_dir(&workspace)
        .env_remove("DUUMBI_TELEMETRY_DIR")
        .output()
        .expect("invariant: duumbi build must run");
    assert!(
        build.status.success(),
        "workspace traced build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(duumbi)
        .arg("run")
        .current_dir(&workspace)
        .env("DUUMBI_TELEMETRY_DIR", "")
        .output()
        .expect("invariant: duumbi run must run");
    assert!(
        !run.status.success(),
        "controlled panic fixture must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("duumbi panic: called Option::unwrap() on a None value"),
        "panic stderr must preserve original message, got: {stderr}"
    );

    let configured_dir = workspace.join("custom/telemetry");
    assert!(configured_dir.join("trace_map.json").exists());
    assert!(configured_dir.join("traces.jsonl").exists());
    assert!(configured_dir.join("crash_dump.jsonl").exists());
    assert!(
        !workspace.join(".duumbi/telemetry/traces.jsonl").exists(),
        "workspace run should not split trace events into the default telemetry dir"
    );
    assert_eq!(
        fs::read(&graph_path).expect("invariant: graph file must be readable"),
        original_graph,
        "workspace run must not mutate source graph files"
    );

    let inspect = Command::new(duumbi)
        .args(["telemetry", "inspect"])
        .current_dir(&workspace)
        .env_remove("DUUMBI_TELEMETRY_DIR")
        .output()
        .expect("invariant: telemetry inspect must run");
    assert!(
        inspect.status.success(),
        "telemetry inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let stdout = String::from_utf8(inspect.stdout).expect("invariant: inspect stdout is UTF-8");
    assert!(stdout.contains("Function: duumbi:telemetry/main"));
    assert!(stdout.contains("Block: duumbi:telemetry/main/entry"));
    assert!(stdout.contains("Exact node evidence: unavailable in v1"));
}

#[test]
fn telemetry_inspect_without_dir_reports_malformed_config() {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let workspace = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let duumbi_dir = workspace.path().join(".duumbi");
    fs::create_dir_all(&duumbi_dir).expect("invariant: .duumbi dir must be created");
    fs::write(
        duumbi_dir.join("config.toml"),
        "[telemetry\nartifact-dir = \"x\"\n",
    )
    .expect("invariant: config must be written");

    let inspect = Command::new(duumbi)
        .current_dir(workspace.path())
        .args(["telemetry", "inspect"])
        .output()
        .expect("invariant: telemetry inspect must run");

    assert!(!inspect.status.success(), "malformed config must fail");
    let stderr = String::from_utf8_lossy(&inspect.stderr);
    assert!(
        stderr.contains("Failed to load telemetry config"),
        "stderr should include config context, got: {stderr}"
    );
    assert!(
        !stderr.contains("missing telemetry evidence"),
        "malformed config should not be hidden as missing evidence, got: {stderr}"
    );
}

fn traced_fixture_evidence(fixture: &str, binary_name: &str) -> TraceFixtureEvidence {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let telemetry_dir = tmp.path().join("telemetry");
    let binary = tmp
        .path()
        .join(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX));

    let build = Command::new(duumbi)
        .args(["build", "--trace", fixture, "-o"])
        .arg(binary.to_str().expect("invariant: temp path must be UTF-8"))
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: duumbi build must run");
    assert!(
        build.status.success(),
        "traced build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: traced binary must run");
    assert!(
        !run.status.success(),
        "controlled panic fixture must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("duumbi panic: called Option::unwrap() on a None value"),
        "panic stderr must preserve original message, got: {stderr}"
    );

    assert!(telemetry_dir.join("trace_map.json").exists());
    assert!(telemetry_dir.join("traces.jsonl").exists());
    assert!(telemetry_dir.join("crash_dump.jsonl").exists());

    let trace_map = read_trace_map(&telemetry_dir);
    let trace_events = read_trace_events(&telemetry_dir);
    let crash_records = read_crash_records(&telemetry_dir);

    let inspect = Command::new(duumbi)
        .args([
            "telemetry",
            "inspect",
            "--telemetry-dir",
            telemetry_dir
                .to_str()
                .expect("invariant: telemetry path must be UTF-8"),
        ])
        .output()
        .expect("invariant: telemetry inspect must run");
    assert!(
        inspect.status.success(),
        "telemetry inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    TraceFixtureEvidence {
        _tmp: tmp,
        telemetry_dir,
        inspect_stdout: String::from_utf8(inspect.stdout)
            .expect("invariant: inspect stdout must be UTF-8"),
        trace_map,
        trace_events,
        crash_records,
    }
}

#[derive(Debug)]
struct TraceFixtureEvidence {
    _tmp: tempfile::TempDir,
    telemetry_dir: std::path::PathBuf,
    inspect_stdout: String,
    trace_map: TraceMap,
    trace_events: Vec<TraceEvent>,
    crash_records: Vec<CrashRecord>,
}

#[derive(Debug)]
struct UntracedFailureEvidence {
    _tmp: tempfile::TempDir,
    telemetry_dir: std::path::PathBuf,
}

#[derive(Debug, Deserialize)]
struct TraceEvent {
    event: String,
    trace_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CrashRecord {
    event: String,
    function_id: u64,
    block_id: u64,
    trace_active: bool,
}

fn untraced_fixture_failure(fixture: &str, binary_name: &str) -> UntracedFailureEvidence {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let telemetry_dir = tmp.path().join("telemetry");
    let binary = tmp
        .path()
        .join(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX));

    let build = Command::new(duumbi)
        .args(["build", fixture, "-o"])
        .arg(binary.to_str().expect("invariant: temp path must be UTF-8"))
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: duumbi build must run");
    assert!(
        build.status.success(),
        "untraced build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: untraced binary must run");
    assert!(
        !run.status.success(),
        "controlled panic fixture must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("duumbi panic: called Option::unwrap() on a None value"),
        "panic stderr must preserve original message, got: {stderr}"
    );

    UntracedFailureEvidence {
        _tmp: tmp,
        telemetry_dir: telemetry_dir.clone(),
    }
}

fn repair_context_stdout(evidence: &TraceFixtureEvidence, fixture: &str) -> String {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let repair_context = Command::new(duumbi)
        .args([
            "telemetry",
            "repair-context",
            "--telemetry-dir",
            evidence
                .telemetry_dir
                .to_str()
                .expect("invariant: telemetry path must be UTF-8"),
            "--graph",
            fixture,
        ])
        .output()
        .expect("invariant: telemetry repair-context must run");
    assert!(
        repair_context.status.success(),
        "telemetry repair-context failed: {}",
        String::from_utf8_lossy(&repair_context.stderr)
    );

    String::from_utf8(repair_context.stdout)
        .expect("invariant: repair-context stdout must be UTF-8")
}

fn repair_patch_json() -> &'static str {
    r#"{
        "ops": [{
            "kind": "replace_block",
            "block_id": "duumbi:telemetry/main/entry",
            "ops": [
                {
                    "@type": "duumbi:Const",
                    "@id": "duumbi:telemetry/main/entry/0",
                    "duumbi:value": 0,
                    "duumbi:resultType": "i64"
                },
                {
                    "@type": "duumbi:Return",
                    "@id": "duumbi:telemetry/main/entry/1",
                    "duumbi:operand": {"@id": "duumbi:telemetry/main/entry/0"}
                }
            ]
        }]
    }"#
}

fn assert_required_gates_passed(evidence: &serde_json::Value) {
    let gates = evidence["gates"]
        .as_array()
        .expect("repair validation gates must be an array");
    for required in [
        "graph_patch_parse",
        "atomic_patch_application",
        "graph_parse",
        "graph_build",
        "graph_validation",
        "native_rebuild",
        "relevant_tests",
    ] {
        assert!(
            gates
                .iter()
                .any(|gate| gate["gate"] == required && gate["passed"] == true),
            "required gate should pass: {required}"
        );
    }
}

fn read_trace_map(telemetry_dir: &std::path::Path) -> TraceMap {
    let content = fs::read_to_string(telemetry_dir.join("trace_map.json"))
        .expect("invariant: trace map must be readable");
    serde_json::from_str(&content).expect("invariant: trace map must parse")
}

fn read_trace_events(telemetry_dir: &std::path::Path) -> Vec<TraceEvent> {
    let content = fs::read_to_string(telemetry_dir.join("traces.jsonl"))
        .expect("invariant: trace events must be readable");
    content
        .lines()
        .map(|line| serde_json::from_str(line).expect("invariant: trace event must parse"))
        .collect()
}

fn read_crash_records(telemetry_dir: &std::path::Path) -> Vec<CrashRecord> {
    let content = fs::read_to_string(telemetry_dir.join("crash_dump.jsonl"))
        .expect("invariant: crash records must be readable");
    content
        .lines()
        .map(|line| serde_json::from_str(line).expect("invariant: crash record must parse"))
        .collect()
}

fn assert_trace_events_join_trace_map(evidence: &TraceFixtureEvidence, required_events: &[&str]) {
    for event in required_events {
        let kind = match *event {
            "function_enter" | "function_exit" => TraceMapKind::Function,
            "block_enter" | "block_exit" => TraceMapKind::Block,
            _ => panic!("unsupported trace event kind {event}"),
        };
        let matching_events: Vec<_> = evidence
            .trace_events
            .iter()
            .filter(|trace| trace.event == *event)
            .collect();
        assert!(
            !matching_events.is_empty(),
            "missing trace event kind {event}"
        );

        for trace in matching_events {
            let trace_id = trace
                .trace_id
                .unwrap_or_else(|| panic!("trace event kind {event} did not include trace_id"));

            assert!(
                evidence
                    .trace_map
                    .entries
                    .iter()
                    .any(|entry| entry.kind == kind && entry.trace_id == trace_id),
                "trace event {event} with id {trace_id} did not join to the trace map"
            );
        }
    }
}

fn assert_latest_crash_joins_trace_map(
    evidence: &TraceFixtureEvidence,
    expected_function: &str,
    expected_block: &str,
) {
    let crash = evidence
        .crash_records
        .last()
        .expect("crash evidence must contain a panic record");
    assert_eq!(crash.event, "panic");
    assert!(
        crash.trace_active,
        "crash record must come from a traced run"
    );

    let function = evidence
        .trace_map
        .entries
        .iter()
        .find(|entry| entry.kind == TraceMapKind::Function && entry.trace_id == crash.function_id)
        .unwrap_or_else(|| {
            panic!(
                "crash function_id {} did not join to a function trace map entry",
                crash.function_id
            )
        });
    assert_eq!(function.graph_id, expected_function);

    let block = evidence
        .trace_map
        .entries
        .iter()
        .find(|entry| entry.kind == TraceMapKind::Block && entry.trace_id == crash.block_id)
        .unwrap_or_else(|| {
            panic!(
                "crash block_id {} did not join to a block trace map entry",
                crash.block_id
            )
        });
    assert_eq!(block.graph_id, expected_block);
}
