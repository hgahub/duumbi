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
        inspect_stdout: String::from_utf8(inspect.stdout)
            .expect("invariant: inspect stdout must be UTF-8"),
        trace_map,
        trace_events,
    }
}

#[derive(Debug)]
struct TraceFixtureEvidence {
    inspect_stdout: String,
    trace_map: TraceMap,
    trace_events: Vec<TraceEvent>,
}

#[derive(Debug, Deserialize)]
struct TraceEvent {
    event: String,
    trace_id: Option<u64>,
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

fn assert_trace_events_join_trace_map(evidence: &TraceFixtureEvidence, required_events: &[&str]) {
    for event in required_events {
        let kind = match *event {
            "function_enter" | "function_exit" => TraceMapKind::Function,
            "block_enter" | "block_exit" => TraceMapKind::Block,
            _ => panic!("unsupported trace event kind {event}"),
        };
        let trace_id = evidence
            .trace_events
            .iter()
            .find(|trace| trace.event == *event)
            .unwrap_or_else(|| panic!("missing trace event kind {event}"))
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
