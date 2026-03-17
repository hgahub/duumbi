//! Phase 9A stdlib integration tests — string utilities, type casts, edge cases.
//!
//! Covers issues #295 (StringTrim/ToUpper/ToLower/Replace),
//! #293 (CastI64ToF64/CastF64ToI64), #300 (edge cases), #301 (regression matrix).

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

/// Compile and run a single-module JSON-LD program. Returns (stdout, exit_code).
fn compile_and_run_json(json: &str) -> (String, i32) {
    let module = parse_jsonld(json).expect("parse");
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

// ===== StringTrim (#295) =====

#[test]
fn string_trim_removes_whitespace() {
    // trim("  hello  ") → "hello", then return length (5)
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": "  hello  ", "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringTrim", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "string"},
                    {"@type": "duumbi:PrintString", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}},
                    {"@type": "duumbi:StringLength", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}}
                ]}]}]
    }"#;
    let (stdout, exit_code) = compile_and_run_json(json);
    assert_eq!(stdout, "hello", "trim must remove surrounding whitespace");
    assert_eq!(exit_code, 5, "trimmed length must be 5");
}

#[test]
fn string_trim_empty_string_unchanged() {
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": "", "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringTrim", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringLength", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}}
                ]}]}]
    }"#;
    let (_, exit_code) = compile_and_run_json(json);
    assert_eq!(exit_code, 0, "trim of empty string has length 0");
}

// ===== StringToUpper (#295) =====

#[test]
fn string_to_upper_converts_ascii() {
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": "hello world", "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringToUpper", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "string"},
                    {"@type": "duumbi:PrintString", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/3",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}}
                ]}]}]
    }"#;
    let (stdout, exit_code) = compile_and_run_json(json);
    assert_eq!(
        stdout, "HELLO WORLD",
        "to_upper must convert all ASCII lowercase"
    );
    assert_eq!(exit_code, 0);
}

// ===== StringToLower (#295) =====

#[test]
fn string_to_lower_converts_ascii() {
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": "DUUMBI", "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringToLower", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "string"},
                    {"@type": "duumbi:PrintString", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/3",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}}
                ]}]}]
    }"#;
    let (stdout, exit_code) = compile_and_run_json(json);
    assert_eq!(
        stdout, "duumbi",
        "to_lower must convert all ASCII uppercase"
    );
    assert_eq!(exit_code, 0);
}

// ===== StringReplace (#295) =====

#[test]
fn string_replace_first_occurrence() {
    // replace("aabbaa", "aa", "XX") → "XXbbaa" (only first occurrence)
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": "aabbaa", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                     "duumbi:value": "aa", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/2",
                     "duumbi:value": "XX", "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringReplace", "@id": "duumbi:test/main/entry/3",
                     "duumbi:haystack": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:needle": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:replacement": {"@id": "duumbi:test/main/entry/2"},
                     "duumbi:resultType": "string"},
                    {"@type": "duumbi:PrintString", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/5",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/6",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/5"}}
                ]}]}]
    }"#;
    let (stdout, exit_code) = compile_and_run_json(json);
    assert_eq!(stdout, "XXbbaa", "replace replaces first occurrence only");
    assert_eq!(exit_code, 0);
}

#[test]
fn string_replace_needle_not_found_unchanged() {
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": "hello", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                     "duumbi:value": "xyz", "duumbi:resultType": "string"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/2",
                     "duumbi:value": "ABC", "duumbi:resultType": "string"},
                    {"@type": "duumbi:StringReplace", "@id": "duumbi:test/main/entry/3",
                     "duumbi:haystack": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:needle": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:replacement": {"@id": "duumbi:test/main/entry/2"},
                     "duumbi:resultType": "string"},
                    {"@type": "duumbi:PrintString", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/5",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/6",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/5"}}
                ]}]}]
    }"#;
    let (stdout, _) = compile_and_run_json(json);
    assert_eq!(
        stdout, "hello",
        "replace with missing needle returns original"
    );
}

// ===== CastI64ToF64 / CastF64ToI64 (#293) =====

#[test]
fn cast_i64_to_f64_compiles_and_runs() {
    // CastI64ToF64(7) = 7.0, print as f64 → "7"
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 7, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:CastI64ToF64", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "f64"},
                    {"@type": "duumbi:Print", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/3",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}}
                ]}]}]
    }"#;
    let (stdout, exit_code) = compile_and_run_json(json);
    assert_eq!(stdout, "7", "cast i64(7) to f64 and print");
    assert_eq!(exit_code, 0);
}

#[test]
fn cast_f64_to_i64_truncates() {
    // CastF64ToI64(3.7) = 3 (truncation / saturation)
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 3.7, "duumbi:resultType": "f64"},
                    {"@type": "duumbi:CastF64ToI64", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"}}
                ]}]}]
    }"#;
    let (_, exit_code) = compile_and_run_json(json);
    assert_eq!(exit_code, 3, "cast f64(3.7) to i64 truncates to 3");
}

#[test]
fn cast_roundtrip_i64_f64_i64() {
    // 42 → f64 → i64 = 42
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 42, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:CastI64ToF64", "@id": "duumbi:test/main/entry/1",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:resultType": "f64"},
                    {"@type": "duumbi:CastF64ToI64", "@id": "duumbi:test/main/entry/2",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}}
                ]}]}]
    }"#;
    let (_, exit_code) = compile_and_run_json(json);
    assert_eq!(exit_code, 42, "i64→f64→i64 roundtrip must preserve value");
}

// ===== Edge cases (#300) =====

#[test]
fn edge_case_div_by_zero_i64_compiles() {
    // Cranelift sdiv by zero is UB in C, but the compiler pipeline must not fail.
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 10, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Div", "@id": "duumbi:test/main/entry/2",
                     "duumbi:left": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:right": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}}
                ]}]}]
    }"#;
    let module = parse_jsonld(json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let _obj_bytes = lowering::compile_to_object(&sg).expect("compile must succeed");
}

#[test]
fn edge_case_f64_inf_print() {
    // 1.0 / 0.0 = +Inf per IEEE 754 (f64 fdiv does not trap).
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 1.0, "duumbi:resultType": "f64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                     "duumbi:value": 0.0, "duumbi:resultType": "f64"},
                    {"@type": "duumbi:Div", "@id": "duumbi:test/main/entry/2",
                     "duumbi:left": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:right": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "f64"},
                    {"@type": "duumbi:Print", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/4",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/5",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/4"}}
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
    // IEEE 754: 1.0/0.0 = +Inf. Print format may vary (inf/Inf).
    assert!(
        stdout.to_lowercase().contains("inf"),
        "1.0/0.0 must print infinity: got {stdout}"
    );
}

#[test]
fn edge_case_modulo_zero_compiles() {
    // Compiler must accept modulo-by-zero programs; runtime behavior is UB.
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 5, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                     "duumbi:value": 0, "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Modulo", "@id": "duumbi:test/main/entry/2",
                     "duumbi:left": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:right": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/2"}}
                ]}]}]
    }"#;
    let module = parse_jsonld(json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let _obj_bytes =
        lowering::compile_to_object(&sg).expect("compiler must not reject modulo-by-zero");
}

#[test]
fn edge_case_cast_f64_nan_to_i64_saturates() {
    // fcvt_to_sint_sat: NaN → 0 per Cranelift saturation semantics
    let json = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module", "@id": "duumbi:test", "duumbi:name": "test",
        "duumbi:functions": [{"@type": "duumbi:Function", "@id": "duumbi:test/main",
            "duumbi:name": "main", "duumbi:returnType": "i64",
            "duumbi:blocks": [{"@type": "duumbi:Block", "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry", "duumbi:ops": [
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/0",
                     "duumbi:value": 0.0, "duumbi:resultType": "f64"},
                    {"@type": "duumbi:Const", "@id": "duumbi:test/main/entry/1",
                     "duumbi:value": 0.0, "duumbi:resultType": "f64"},
                    {"@type": "duumbi:Div", "@id": "duumbi:test/main/entry/2",
                     "duumbi:left": {"@id": "duumbi:test/main/entry/0"},
                     "duumbi:right": {"@id": "duumbi:test/main/entry/1"},
                     "duumbi:resultType": "f64"},
                    {"@type": "duumbi:CastF64ToI64", "@id": "duumbi:test/main/entry/3",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/2"},
                     "duumbi:resultType": "i64"},
                    {"@type": "duumbi:Return", "@id": "duumbi:test/main/entry/4",
                     "duumbi:operand": {"@id": "duumbi:test/main/entry/3"}}
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
    // fcvt_to_sint_sat(NaN) = 0 per Wasm/Cranelift spec
    assert_eq!(
        output.status.code(),
        Some(0),
        "CastF64ToI64(NaN) must saturate to 0"
    );
}

// ===== Regression matrix: stdlib module parse+build (#301) =====
//
// Library modules have no `main` function — we use parse_jsonld only to
// validate syntax and structure. Full end-to-end compilation of stdlib
// modules requires a caller program (via Program::load); math.jsonld has
// a compile test in src/compiler/lowering.rs, others are parse-only here.

#[test]
fn stdlib_math_jsonld_parses() {
    let json = include_str!("../stdlib/math.jsonld");
    let module = parse_jsonld(json).expect("stdlib math must parse");
    assert_eq!(module.name.0, "math");
    assert!(
        module.functions.len() >= 8,
        "math stdlib must have >=8 functions"
    );
    let export_names: Vec<_> = module.functions.iter().map(|f| f.name.0.as_str()).collect();
    for name in &["abs", "max", "min", "sqrt", "pow", "mod", "clamp", "sign"] {
        assert!(export_names.contains(name), "math must export {name}");
    }
}

#[test]
fn stdlib_io_jsonld_parses() {
    let json = include_str!("../stdlib/io.jsonld");
    let module = parse_jsonld(json).expect("stdlib io must parse");
    assert_eq!(module.name.0, "io");
    assert!(
        module.functions.len() >= 4,
        "io stdlib must have >=4 functions"
    );
    let export_names: Vec<_> = module.functions.iter().map(|f| f.name.0.as_str()).collect();
    for name in &["print_i64", "print_f64", "print_bool", "print_string"] {
        assert!(export_names.contains(name), "io must export {name}");
    }
}

#[test]
fn stdlib_lang_jsonld_parses() {
    let json = include_str!("../stdlib/lang.jsonld");
    let module = parse_jsonld(json).expect("stdlib lang must parse");
    assert_eq!(module.name.0, "lang");
    assert!(
        module.functions.len() >= 3,
        "lang stdlib must have >=3 functions"
    );
    let export_names: Vec<_> = module.functions.iter().map(|f| f.name.0.as_str()).collect();
    for name in &["assert_true", "i64_to_f64", "f64_to_i64"] {
        assert!(export_names.contains(name), "lang must export {name}");
    }
}

#[test]
fn stdlib_string_jsonld_parses() {
    let json = include_str!("../stdlib/string.jsonld");
    let module = parse_jsonld(json).expect("stdlib string must parse");
    assert_eq!(module.name.0, "string");
    assert!(
        module.functions.len() >= 7,
        "string stdlib must have >=7 functions"
    );
    let export_names: Vec<_> = module.functions.iter().map(|f| f.name.0.as_str()).collect();
    for name in &[
        "length", "contains", "find", "trim", "to_upper", "to_lower", "replace",
    ] {
        assert!(export_names.contains(name), "string must export {name}");
    }
}
