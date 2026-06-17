//! DUUMBI-714 backend hardening integration tests.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use duumbi::compiler::{linker, lowering};
use duumbi::errors::{Diagnostic, DiagnosticLevel, codes};
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;
use serde_json::Value;

const FIXTURE_DIR: &str = "tests/fixtures/backend_hardening";
const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

#[derive(Clone, Debug, PartialEq, Eq)]
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

fn diagnostics_for_jsonld(json: &str) -> Vec<Diagnostic> {
    let module = parse_jsonld(json).unwrap_or_else(|error| panic!("parse failed: {error}"));
    let graph = build_graph(&module).unwrap_or_else(|error| panic!("build failed: {error:?}"));
    validate(&graph)
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum OracleValue {
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
    Array(Vec<OracleValue>),
    Option(Option<Box<OracleValue>>),
    ResultOk(Box<OracleValue>),
    ResultErr(Box<OracleValue>),
    Struct(HashMap<String, OracleValue>),
}

fn interpret_fixture(fixture_name: &str) -> RunResult {
    let fixture = format!("{FIXTURE_DIR}/{fixture_name}");
    let json = fs::read_to_string(&fixture)
        .unwrap_or_else(|error| panic!("failed to read fixture {fixture}: {error}"));
    let module: Value = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("failed to decode fixture {fixture}: {error}"));
    interpret_module(&module)
}

fn interpret_module(module: &Value) -> RunResult {
    let function = module["duumbi:functions"]
        .as_array()
        .and_then(|functions| functions.first())
        .expect("invariant: fixture has a main function");
    let mut blocks = HashMap::new();
    for block in function["duumbi:blocks"]
        .as_array()
        .expect("invariant: fixture has blocks")
    {
        let label = block["duumbi:label"]
            .as_str()
            .expect("invariant: block has label");
        blocks.insert(label.to_string(), block);
    }

    let mut env: HashMap<String, OracleValue> = HashMap::new();
    let mut stdout = String::new();
    let mut current_label = "entry".to_string();

    for _ in 0..128 {
        let block = blocks
            .get(&current_label)
            .unwrap_or_else(|| panic!("missing block label {current_label}"));
        let ops = block["duumbi:ops"]
            .as_array()
            .expect("invariant: block has ops");
        let mut jumped = false;

        for op in ops {
            let op_id = node_id(op);
            match op_kind(op) {
                "Const" => {
                    env.insert(op_id.to_string(), const_value(op));
                }
                "Add" => {
                    let value = i64_value(&env, ref_field(op, "duumbi:left"))
                        .wrapping_add(i64_value(&env, ref_field(op, "duumbi:right")));
                    env.insert(op_id.to_string(), OracleValue::I64(value));
                }
                "Mul" => {
                    let value = i64_value(&env, ref_field(op, "duumbi:left"))
                        .wrapping_mul(i64_value(&env, ref_field(op, "duumbi:right")));
                    env.insert(op_id.to_string(), OracleValue::I64(value));
                }
                "Div" => {
                    let left = i64_value(&env, ref_field(op, "duumbi:left"));
                    let right = i64_value(&env, ref_field(op, "duumbi:right"));
                    if right == 0 {
                        return panic_result(&stdout, "division by zero", op_id);
                    }
                    if left == i64::MIN && right == -1 {
                        return panic_result(&stdout, "division overflow", op_id);
                    }
                    env.insert(op_id.to_string(), OracleValue::I64(left / right));
                }
                "MulChecked" => {
                    let left = i64_value(&env, ref_field(op, "duumbi:left"));
                    let right = i64_value(&env, ref_field(op, "duumbi:right"));
                    let value = match left.checked_mul(right) {
                        Some(value) => OracleValue::ResultOk(Box::new(OracleValue::I64(value))),
                        None => OracleValue::ResultErr(Box::new(OracleValue::String(
                            "integer overflow".to_string(),
                        ))),
                    };
                    env.insert(op_id.to_string(), value);
                }
                "ArrayNew" => {
                    env.insert(op_id.to_string(), OracleValue::Array(Vec::new()));
                }
                "ArrayPush" => {
                    let array_id = ref_field(op, "duumbi:array");
                    let element = value(&env, ref_field(op, "duumbi:element"));
                    let mut array = array_value(&env, array_id);
                    array.push(element);
                    env.insert(array_id.to_string(), OracleValue::Array(array.clone()));
                    env.insert(op_id.to_string(), OracleValue::Array(array));
                }
                "ArrayGet" => {
                    let array = array_value(&env, ref_field(op, "duumbi:array"));
                    let index = i64_value(&env, ref_field(op, "duumbi:index"));
                    let Some(value) = array.get(index as usize).filter(|_| index >= 0) else {
                        return panic_result(&stdout, "array index out of bounds", op_id);
                    };
                    env.insert(op_id.to_string(), value.clone());
                }
                "ArrayTryGet" => {
                    let array = array_value(&env, ref_field(op, "duumbi:array"));
                    let index = i64_value(&env, ref_field(op, "duumbi:index"));
                    let value = array
                        .get(index as usize)
                        .filter(|_| index >= 0)
                        .cloned()
                        .map(Box::new);
                    env.insert(op_id.to_string(), OracleValue::Option(value));
                }
                "ArraySet" => {
                    let array_id = ref_field(op, "duumbi:array");
                    let mut array = array_value(&env, array_id);
                    let index = i64_value(&env, ref_field(op, "duumbi:index"));
                    let Some(slot) = array.get_mut(index as usize).filter(|_| index >= 0) else {
                        return panic_result(&stdout, "array index out of bounds", op_id);
                    };
                    *slot = value(&env, ref_field(op, "duumbi:value"));
                    env.insert(array_id.to_string(), OracleValue::Array(array));
                }
                "StructNew" => {
                    env.insert(op_id.to_string(), OracleValue::Struct(HashMap::new()));
                }
                "FieldSet" => {
                    let struct_id = ref_field(op, "duumbi:operand");
                    let mut fields = struct_value(&env, struct_id);
                    fields.insert(
                        string_field(op, "duumbi:fieldName").to_string(),
                        value(&env, ref_field(op, "duumbi:value")),
                    );
                    env.insert(struct_id.to_string(), OracleValue::Struct(fields));
                }
                "FieldGet" => {
                    let fields = struct_value(&env, ref_field(op, "duumbi:operand"));
                    let field_name = string_field(op, "duumbi:fieldName");
                    let field_value = fields
                        .get(field_name)
                        .unwrap_or_else(|| panic!("missing struct field {field_name}"))
                        .clone();
                    env.insert(op_id.to_string(), field_value);
                }
                "CastF64ToI64" => {
                    let value = match value(&env, ref_field(op, "duumbi:operand")) {
                        OracleValue::F64(value) => value as i64,
                        other => panic!("CastF64ToI64 expected f64, got {other:?}"),
                    };
                    env.insert(op_id.to_string(), OracleValue::I64(value));
                }
                "Print" => {
                    let printed = value(&env, ref_field(op, "duumbi:operand"));
                    match printed {
                        OracleValue::I64(value) => stdout.push_str(&format!("{value}\n")),
                        other => panic!("Print only supports i64 in these fixtures, got {other:?}"),
                    }
                }
                "Branch" => {
                    let condition = bool_value(&env, ref_field(op, "duumbi:condition"));
                    current_label = if condition {
                        string_field(op, "duumbi:trueBlock").to_string()
                    } else {
                        string_field(op, "duumbi:falseBlock").to_string()
                    };
                    jumped = true;
                    break;
                }
                "Match" => {
                    let operand = value(&env, ref_field(op, "duumbi:operand"));
                    current_label = match operand {
                        OracleValue::Option(Some(inner)) => {
                            env.insert(format!("{op_id}:payload"), *inner);
                            string_field(op, "duumbi:okBlock").to_string()
                        }
                        OracleValue::Option(None) => {
                            string_field(op, "duumbi:errBlock").to_string()
                        }
                        OracleValue::ResultOk(inner) => {
                            env.insert(format!("{op_id}:payload"), *inner);
                            string_field(op, "duumbi:okBlock").to_string()
                        }
                        OracleValue::ResultErr(inner) => {
                            env.insert(format!("{op_id}:payload"), *inner);
                            string_field(op, "duumbi:errBlock").to_string()
                        }
                        other => panic!("Match expected option/result, got {other:?}"),
                    };
                    jumped = true;
                    break;
                }
                "Return" => {
                    return RunResult {
                        stdout: stdout.trim().to_string(),
                        stderr: String::new(),
                        exit_code: i64_value(&env, ref_field(op, "duumbi:operand")) as i32,
                    };
                }
                other => panic!("unsupported oracle op {other} in node {op_id}"),
            }
        }

        assert!(jumped, "block {current_label} did not terminate");
    }

    panic!("oracle exceeded maximum block steps");
}

fn panic_result(stdout: &str, message: &str, node_id: &str) -> RunResult {
    RunResult {
        stdout: stdout.trim().to_string(),
        stderr: format!("duumbi panic: {message} at node {node_id}\n"),
        exit_code: 1,
    }
}

fn op_kind(op: &Value) -> &str {
    op["@type"]
        .as_str()
        .and_then(|kind| kind.strip_prefix("duumbi:"))
        .expect("invariant: op has duumbi type")
}

fn node_id(op: &Value) -> &str {
    op["@id"].as_str().expect("invariant: op has id")
}

fn string_field<'a>(op: &'a Value, field: &str) -> &'a str {
    op[field]
        .as_str()
        .unwrap_or_else(|| panic!("missing string field {field} on {}", node_id(op)))
}

fn ref_field<'a>(op: &'a Value, field: &str) -> &'a str {
    op[field]["@id"]
        .as_str()
        .unwrap_or_else(|| panic!("missing ref field {field} on {}", node_id(op)))
}

fn const_value(op: &Value) -> OracleValue {
    let value = &op["duumbi:value"];
    match string_field(op, "duumbi:resultType") {
        "i64" => OracleValue::I64(value.as_i64().expect("i64 const")),
        "f64" => OracleValue::F64(value.as_f64().expect("f64 const")),
        "bool" => OracleValue::Bool(value.as_bool().expect("bool const")),
        "string" => OracleValue::String(value.as_str().expect("string const").to_string()),
        ty => panic!("unsupported const type {ty}"),
    }
}

fn value(env: &HashMap<String, OracleValue>, id: &str) -> OracleValue {
    env.get(id)
        .unwrap_or_else(|| panic!("missing oracle value {id}"))
        .clone()
}

fn i64_value(env: &HashMap<String, OracleValue>, id: &str) -> i64 {
    match value(env, id) {
        OracleValue::I64(value) => value,
        other => panic!("expected i64 for {id}, got {other:?}"),
    }
}

fn bool_value(env: &HashMap<String, OracleValue>, id: &str) -> bool {
    match value(env, id) {
        OracleValue::Bool(value) => value,
        other => panic!("expected bool for {id}, got {other:?}"),
    }
}

fn array_value(env: &HashMap<String, OracleValue>, id: &str) -> Vec<OracleValue> {
    match value(env, id) {
        OracleValue::Array(value) => value,
        other => panic!("expected array for {id}, got {other:?}"),
    }
}

fn struct_value(env: &HashMap<String, OracleValue>, id: &str) -> HashMap<String, OracleValue> {
    match value(env, id) {
        OracleValue::Struct(value) => value,
        other => panic!("expected struct for {id}, got {other:?}"),
    }
}

#[test]
fn ignored_array_try_get_reports_unhandled_option() {
    let diagnostics = diagnostics_for_jsonld(
        r#"{
          "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
          "@type": "duumbi:Module",
          "@id": "duumbi:review",
          "duumbi:name": "review",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:review/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:review/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type":"duumbi:ArrayNew","@id":"duumbi:review/main/entry/array","duumbi:resultType":"array<i64>"},
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/index","duumbi:value":0,"duumbi:resultType":"i64"},
                {"@type":"duumbi:ArrayTryGet","@id":"duumbi:review/main/entry/try_get","duumbi:array":{"@id":"duumbi:review/main/entry/array"},"duumbi:index":{"@id":"duumbi:review/main/entry/index"},"duumbi:resultType":"option<i64>"},
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/zero","duumbi:value":0,"duumbi:resultType":"i64"},
                {"@type":"duumbi:Return","@id":"duumbi:review/main/entry/return","duumbi:operand":{"@id":"duumbi:review/main/entry/zero"}}
              ]
            }]
          }]
        }"#,
    );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == codes::E031_UNHANDLED_OPTION),
        "expected E031 for ignored ArrayTryGet, got {diagnostics:?}"
    );
}

#[test]
fn ignored_checked_arithmetic_reports_unhandled_result() {
    let diagnostics = diagnostics_for_jsonld(
        r#"{
          "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
          "@type": "duumbi:Module",
          "@id": "duumbi:review",
          "duumbi:name": "review",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:review/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:review/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/one","duumbi:value":1,"duumbi:resultType":"i64"},
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/two","duumbi:value":2,"duumbi:resultType":"i64"},
                {"@type":"duumbi:AddChecked","@id":"duumbi:review/main/entry/add_checked","duumbi:left":{"@id":"duumbi:review/main/entry/one"},"duumbi:right":{"@id":"duumbi:review/main/entry/two"},"duumbi:resultType":"result<i64,string>"},
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/zero","duumbi:value":0,"duumbi:resultType":"i64"},
                {"@type":"duumbi:Return","@id":"duumbi:review/main/entry/return","duumbi:operand":{"@id":"duumbi:review/main/entry/zero"}}
              ]
            }]
          }]
        }"#,
    );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == codes::E030_UNHANDLED_RESULT),
        "expected E030 for ignored checked arithmetic, got {diagnostics:?}"
    );
}

#[test]
fn checked_arithmetic_requires_result_i64_string_type() {
    let diagnostics = diagnostics_for_jsonld(
        r#"{
          "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
          "@type": "duumbi:Module",
          "@id": "duumbi:review",
          "duumbi:name": "review",
          "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:review/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
              "@type": "duumbi:Block",
              "@id": "duumbi:review/main/entry",
              "duumbi:label": "entry",
              "duumbi:ops": [
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/one","duumbi:value":1,"duumbi:resultType":"i64"},
                {"@type":"duumbi:Const","@id":"duumbi:review/main/entry/two","duumbi:value":2,"duumbi:resultType":"i64"},
                {"@type":"duumbi:AddChecked","@id":"duumbi:review/main/entry/add_checked","duumbi:left":{"@id":"duumbi:review/main/entry/one"},"duumbi:right":{"@id":"duumbi:review/main/entry/two"},"duumbi:resultType":"i64"},
                {"@type":"duumbi:Return","@id":"duumbi:review/main/entry/return","duumbi:operand":{"@id":"duumbi:review/main/entry/add_checked"}}
              ]
            }]
          }]
        }"#,
    );

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.level == DiagnosticLevel::Error
                && diagnostic.message == "checked arithmetic ops must return result<i64,string>"
        }),
        "expected checked arithmetic result type error, got {diagnostics:?}"
    );
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
fn div_min_overflow_node_attribution() {
    let result = compile_and_run("div_min_overflow_node_attribution.jsonld");
    assert_ne!(result.exit_code, 0, "i64::MIN / -1 must fail");
    assert!(
        result.stderr.contains("duumbi panic: division overflow"),
        "stderr missing deterministic panic kind: {}",
        result.stderr
    );
    assert!(
        result
            .stderr
            .contains("duumbi:backend_hardening/main/entry/div_overflow"),
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
        "div_zero_node_attribution.jsonld",
        "div_min_overflow_node_attribution.jsonld",
        "array_get_oob_node_attribution.jsonld",
        "array_set_negative_node_attribution.jsonld",
        "array_try_get_some_match.jsonld",
        "array_try_get_none_match.jsonld",
        "checked_mul_overflow_returns_err.jsonld",
        "unchecked_mul_wrap_policy.jsonld",
        "large_mixed_struct_layout.jsonld",
    ];

    for fixture in fixtures {
        let oracle = interpret_fixture(fixture);
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
