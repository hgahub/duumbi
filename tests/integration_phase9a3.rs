//! Phase 9a-3 integration tests: error handling (Result/Option).
//!
//! Tests the kill criterion: `Result<i64, i64>` return + Ok/Err handling
//! + E030 validator rejection for unhandled Result.

use std::fs;
use std::path::Path;
use std::process::Command;

use duumbi::compiler::{linker, lowering};
use duumbi::errors::codes;
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;

const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

fn load_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/error_handling/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

fn parse_and_validate(fixture_name: &str) -> Vec<duumbi::errors::Diagnostic> {
    let json = load_fixture(fixture_name);
    let module =
        parse_jsonld(&json).unwrap_or_else(|e| panic!("Parse failed for {fixture_name}: {e}"));
    let sg =
        build_graph(&module).unwrap_or_else(|e| panic!("Build failed for {fixture_name}: {e:?}"));
    validate(&sg)
}

fn compile_runtime_to(tmp_dir: &Path) -> std::path::PathBuf {
    let runtime_c = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_c, RUNTIME_C_SOURCE).expect("invariant: must write runtime C");
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).expect("invariant: runtime must compile");
    runtime_o
}

fn compile_and_run(fixture_name: &str) -> (String, i32) {
    let json = load_fixture(fixture_name);
    let module = parse_jsonld(&json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let diags = validate(&sg);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.level == duumbi::errors::DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "Validation errors: {errors:?}");

    let obj_bytes = lowering::compile_to_object(&sg).expect("compile");

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let obj_path = tmp.path().join("test.o");
    let binary = tmp.path().join("test_bin");
    fs::write(&obj_path, &obj_bytes).expect("write obj");

    let runtime_o = compile_runtime_to(tmp.path());
    linker::link(&obj_path, &runtime_o, &binary).expect("link");

    let output = Command::new(&binary).output().expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    (stdout, exit_code)
}

// === Kill Criterion: Result parse + validate ===

#[test]
fn result_ok_unwrap_parses_and_validates() {
    let diags = parse_and_validate("result_ok_unwrap.jsonld");
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.level == duumbi::errors::DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "result_ok_unwrap.jsonld should validate without errors, got: {errors:?}"
    );
}

#[test]
fn option_some_unwrap_parses_and_validates() {
    let diags = parse_and_validate("option_some_unwrap.jsonld");
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.level == duumbi::errors::DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "option_some_unwrap.jsonld should validate without errors, got: {errors:?}"
    );
}

// === E030: Unhandled Result ===

#[test]
fn unhandled_result_produces_e030() {
    let diags = parse_and_validate("unhandled_result.jsonld");
    assert!(
        diags.iter().any(|d| d.code == codes::E030_UNHANDLED_RESULT),
        "Expected E030 for unhandled Result, got: {diags:?}"
    );
}

// === Compile + Run (end-to-end kill criterion) ===

#[test]
fn kill_criterion_result_ok_compiles_and_runs() {
    let (stdout, exit_code) = compile_and_run("result_ok_unwrap.jsonld");
    assert!(
        stdout.contains("42"),
        "Expected output to contain '42', got: {stdout}"
    );
    // Exit code is (42 % 256) on macOS/Linux
    assert_eq!(exit_code, 42, "Expected exit code 42, got: {exit_code}");
}

#[test]
fn kill_criterion_option_some_compiles_and_runs() {
    let (stdout, exit_code) = compile_and_run("option_some_unwrap.jsonld");
    assert!(
        stdout.contains("99"),
        "Expected output to contain '99', got: {stdout}"
    );
    assert_eq!(exit_code, 99, "Expected exit code 99, got: {exit_code}");
}
