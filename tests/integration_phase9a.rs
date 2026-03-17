//! Phase 9A integration tests: stdlib & instruction set.
//!
//! Tests math, bitwise, and stdlib operations compile and run correctly.

use std::fs;
use std::path::Path;
use std::process::Command;

use duumbi::compiler::{linker, lowering};
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;

const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

fn compile_runtime_to(tmp_dir: &Path) -> std::path::PathBuf {
    let runtime_c = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_c, RUNTIME_C_SOURCE).expect("invariant: must write runtime C");
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).expect("invariant: runtime must compile");
    runtime_o
}

fn compile_and_run(fixture_path: &str) -> (String, i32) {
    let json = std::fs::read_to_string(fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read {fixture_path}: {e}"));
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

// === Modulo ===

#[test]
fn modulo_i64_compiles_and_runs() {
    let (stdout, exit_code) = compile_and_run("tests/fixtures/modulo.jsonld");
    assert_eq!(stdout, "2", "17 % 5 = 2");
    assert_eq!(exit_code, 2);
}

// === Negate ===

#[test]
fn negate_i64_compiles_and_runs() {
    // Inline fixture: negate(42) = -42
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0", "duumbi:value": 42, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Negate", "@id": "duumbi:test/main/entry/1", "duumbi:operand": {"@id": "duumbi:test/main/entry/0"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Print", "@id": "duumbi:test/main/entry/2", "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/3", "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4", "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}}
                ]}]}]
    }"#;
    let module = parse_jsonld(json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let obj_bytes = lowering::compile_to_object(&sg).expect("compile");
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let obj_path = tmp.path().join("test.o");
    let binary = tmp.path().join("test_bin");
    fs::write(&obj_path, &obj_bytes).expect("write obj");
    let runtime_o = compile_runtime_to(tmp.path());
    linker::link(&obj_path, &runtime_o, &binary).expect("link");
    let output = Command::new(&binary).output().expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "-42", "negate(42) = -42");
}

// === Bitwise ===

#[test]
fn bitwise_and_compiles_and_runs() {
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0", "duumbi:value": 15, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1", "duumbi:value": 6, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:BitwiseAnd", "@id": "duumbi:test/main/entry/2", "duumbi:left": {"@id": "duumbi:test/main/entry/0"}, "duumbi:right": {"@id": "duumbi:test/main/entry/1"}, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Print", "@id": "duumbi:test/main/entry/3", "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4", "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}}
                ]}]}]
    }"#;
    let module = parse_jsonld(json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let obj_bytes = lowering::compile_to_object(&sg).expect("compile");
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let obj_path = tmp.path().join("test.o");
    let binary = tmp.path().join("test_bin");
    fs::write(&obj_path, &obj_bytes).expect("write obj");
    let runtime_o = compile_runtime_to(tmp.path());
    linker::link(&obj_path, &runtime_o, &binary).expect("link");
    let output = Command::new(&binary).output().expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // 15 & 6 = 0b1111 & 0b0110 = 0b0110 = 6
    assert_eq!(stdout, "6", "15 & 6 = 6");
    assert_eq!(output.status.code(), Some(6));
}
