//! DUUMBI-714 backend hardening integration tests.

use std::fs;
use std::path::Path;
use std::process::Command;

use duumbi::compiler::{linker, lowering};
use duumbi::errors::DiagnosticLevel;
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;

const FIXTURE_DIR: &str = "tests/fixtures/backend_hardening";
const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

struct RunResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

fn compile_runtime_to(tmp_dir: &Path) -> std::path::PathBuf {
    let runtime_c = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_c, RUNTIME_C_SOURCE).expect("invariant: must write runtime C");
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).expect("invariant: runtime must compile");
    runtime_o
}

fn compile_and_run(fixture_name: &str) -> RunResult {
    let fixture = format!("{FIXTURE_DIR}/{fixture_name}");
    let json = fs::read_to_string(&fixture)
        .unwrap_or_else(|error| panic!("failed to read fixture {fixture}: {error}"));
    let module = parse_jsonld(&json).unwrap_or_else(|error| panic!("parse failed: {error}"));
    let graph = build_graph(&module).unwrap_or_else(|error| panic!("build failed: {error:?}"));
    let diagnostics = validate(&graph);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "validation errors: {errors:?}");

    let object = lowering::compile_to_object(&graph).expect("compile");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let obj_path = tmp.path().join("fixture.o");
    let binary = tmp.path().join("fixture_bin");
    fs::write(&obj_path, object).expect("invariant: object must be writable");
    let runtime_o = compile_runtime_to(tmp.path());
    linker::link(&obj_path, &runtime_o, &binary).expect("link");

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled fixture must run");

    RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

#[test]
fn div_zero_node_attribution() {
    let result = compile_and_run("div_zero_node_attribution.jsonld");
    assert_ne!(result.exit_code, 0, "division by zero must fail");
    assert!(
        result.stderr.contains("duumbi panic: division by zero"),
        "stderr missing deterministic panic kind: {}",
        result.stderr
    );
    assert!(
        result
            .stderr
            .contains("duumbi:backend_hardening/main/entry/div_zero"),
        "stderr missing div node id: {}",
        result.stderr
    );
}

#[test]
fn array_get_oob_node_attribution() {
    let result = compile_and_run("array_get_oob_node_attribution.jsonld");
    assert_ne!(result.exit_code, 0, "ArrayGet OOB must fail");
    assert!(
        result
            .stderr
            .contains("duumbi panic: array index out of bounds"),
        "stderr missing deterministic panic kind: {}",
        result.stderr
    );
    assert!(
        result
            .stderr
            .contains("duumbi:backend_hardening/main/entry/array_get_oob"),
        "stderr missing ArrayGet node id: {}",
        result.stderr
    );
}

#[test]
fn array_set_negative_node_attribution() {
    let result = compile_and_run("array_set_negative_node_attribution.jsonld");
    assert_ne!(result.exit_code, 0, "ArraySet negative index must fail");
    assert!(
        result
            .stderr
            .contains("duumbi panic: array index out of bounds"),
        "stderr missing deterministic panic kind: {}",
        result.stderr
    );
    assert!(
        result
            .stderr
            .contains("duumbi:backend_hardening/main/entry/array_set_negative"),
        "stderr missing ArraySet node id: {}",
        result.stderr
    );
}

#[test]
fn array_try_get_some_match() {
    let result = compile_and_run("array_try_get_some_match.jsonld");
    assert_eq!(result.exit_code, 11, "Some branch must return 11");
    assert_eq!(result.stdout, "", "Some branch fixture should not print");
    assert_eq!(result.stderr, "", "ArrayTryGet Some path should not panic");
}

#[test]
fn array_try_get_none_match() {
    let result = compile_and_run("array_try_get_none_match.jsonld");
    assert_eq!(result.exit_code, 22, "None branch must return 22");
    assert_eq!(result.stdout, "", "None branch fixture should not print");
    assert_eq!(result.stderr, "", "ArrayTryGet None path should not panic");
}

#[test]
fn checked_mul_overflow_returns_err() {
    let result = compile_and_run("checked_mul_overflow_returns_err.jsonld");
    assert_eq!(result.exit_code, 33, "Err branch must return 33");
    assert_eq!(result.stdout, "", "checked overflow should not print");
    assert_eq!(result.stderr, "", "checked overflow should not panic");
}

#[test]
fn large_mixed_struct_layout() {
    let result = compile_and_run("large_mixed_struct_layout.jsonld");
    assert_eq!(
        result.exit_code, 29,
        "large struct should preserve early/middle/late fields and bool branch"
    );
    assert_eq!(result.stdout, "", "large struct fixture should not print");
    assert_eq!(result.stderr, "", "large struct fixture should not panic");
}

#[test]
fn unchecked_mul_wrap_policy() {
    let result = compile_and_run("unchecked_mul_wrap_policy.jsonld");
    assert_eq!(result.exit_code, 0, "unchecked wrap fixture must succeed");
    assert_eq!(result.stdout, "-2", "i64::MAX * 2 should wrap to -2");
    assert_eq!(result.stderr, "", "unchecked wrapping should not panic");
}

#[test]
fn differential_interpreter_native_subset() {
    let fixtures = [
        (
            "array_try_get_some_match.jsonld",
            RunResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 11,
            },
        ),
        (
            "array_try_get_none_match.jsonld",
            RunResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 22,
            },
        ),
        (
            "checked_mul_overflow_returns_err.jsonld",
            RunResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 33,
            },
        ),
        (
            "unchecked_mul_wrap_policy.jsonld",
            RunResult {
                stdout: "-2".to_string(),
                stderr: String::new(),
                exit_code: 0,
            },
        ),
        (
            "large_mixed_struct_layout.jsonld",
            RunResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 29,
            },
        ),
    ];

    for (fixture, oracle) in fixtures {
        let native = compile_and_run(fixture);
        assert_eq!(native.stdout, oracle.stdout, "{fixture} stdout mismatch");
        assert_eq!(native.stderr, oracle.stderr, "{fixture} stderr mismatch");
        assert_eq!(
            native.exit_code, oracle.exit_code,
            "{fixture} exit status mismatch"
        );
    }
}

#[test]
fn traced_div_zero_records_node_attribution() {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let telemetry_dir = tmp.path().join("telemetry");
    let fixture = format!("{FIXTURE_DIR}/div_zero_node_attribution.jsonld");
    let binary = tmp
        .path()
        .join(format!("traced_div_zero{}", std::env::consts::EXE_SUFFIX));
    let expected_node = "duumbi:backend_hardening/main/entry/div_zero";

    let build = Command::new(duumbi)
        .args(["build", "--trace", &fixture, "-o"])
        .arg(&binary)
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: traced build must run");
    assert!(
        build.status.success(),
        "traced build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: traced binary must run");
    assert!(!run.status.success(), "div zero fixture must fail");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains(expected_node),
        "stderr missing exact node id: {stderr}"
    );

    let traces = fs::read_to_string(telemetry_dir.join("traces.jsonl"))
        .expect("invariant: traced run must write trace events");
    let crash = fs::read_to_string(telemetry_dir.join("crash_dump.jsonl"))
        .expect("invariant: traced run must write crash evidence");
    let expected_json = format!("\"node_id\":\"{expected_node}\"");
    assert!(
        traces.contains(&expected_json),
        "trace event evidence missing node_id: {traces}"
    );
    assert!(
        crash.contains(&expected_json),
        "crash evidence missing node_id: {crash}"
    );
}
