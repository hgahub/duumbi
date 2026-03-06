//! Verifier Agent — runs test cases from an [`IntentSpec`] against a compiled binary.
//!
//! # Strategy (M5 — Wrapper Main approach)
//!
//! For each test case, a temporary `main.jsonld` is generated that:
//! 1. Calls the target function with the specified arguments
//! 2. Prints the return value
//! 3. Returns the value as the exit code
//!
//! The temp workspace is compiled and run; stdout is parsed and compared to
//! `expected_return`.

use std::path::Path;
use std::process::Command;

use serde_json::json;

use crate::compiler::{linker, lowering};
use crate::graph::program::Program;
use crate::intent::spec::{IntentSpec, TestCase};

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// The outcome of a single test case execution.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test case name.
    pub name: String,
    /// Function that was called.
    pub function: String,
    /// Arguments passed to the function.
    pub args: Vec<i64>,
    /// Expected return value.
    pub expected: i64,
    /// Actual return value captured from the binary (or `None` on error).
    pub actual: Option<i64>,
    /// Whether the test passed.
    pub passed: bool,
    /// Error message if the test could not be run.
    pub error: Option<String>,
}

/// Aggregated report from running all test cases.
#[derive(Debug, Clone)]
pub struct TestReport {
    /// Number of tests that passed.
    pub passed: usize,
    /// Number of tests that failed or errored.
    pub failed: usize,
    /// Per-test results.
    pub results: Vec<TestResult>,
}

impl TestReport {
    /// Returns `true` if all test cases passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Renders a human-readable summary to a String.
    pub fn display(&self) -> String {
        let mut out = String::from("Test Results:\n");
        for r in &self.results {
            let args_str = r
                .args
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            if r.passed {
                out.push_str(&format!(
                    "  ✓ {}: {}({}) = {}\n",
                    r.name, r.function, args_str, r.expected
                ));
            } else if let Some(err) = &r.error {
                out.push_str(&format!("  ✗ {}: error — {}\n", r.name, err));
            } else {
                out.push_str(&format!(
                    "  ✗ {}: {}({}) = {:?} (expected {})\n",
                    r.name, r.function, args_str, r.actual, r.expected
                ));
            }
        }
        out.push_str(&format!(
            "\n{}/{} passed",
            self.passed,
            self.passed + self.failed
        ));
        out
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Runs all test cases in the [`IntentSpec`] against modules in `workspace`.
///
/// For each test case, generates a temporary main that calls the function and
/// compiles + runs it. Returns a [`TestReport`] with per-test outcomes.
pub fn run_tests(spec: &IntentSpec, workspace: &Path) -> TestReport {
    let mut results = Vec::new();

    for tc in &spec.test_cases {
        let result = run_one_test(tc, workspace);
        results.push(result);
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;

    TestReport {
        passed,
        failed,
        results,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Runs a single test case: generates wrapper → compile → run → check.
fn run_one_test(tc: &TestCase, workspace: &Path) -> TestResult {
    match build_and_run(tc, workspace) {
        Ok(actual) => {
            let passed = actual == tc.expected_return;
            TestResult {
                name: tc.name.clone(),
                function: tc.function.clone(),
                args: tc.args.clone(),
                expected: tc.expected_return,
                actual: Some(actual),
                passed,
                error: None,
            }
        }
        Err(e) => TestResult {
            name: tc.name.clone(),
            function: tc.function.clone(),
            args: tc.args.clone(),
            expected: tc.expected_return,
            actual: None,
            passed: false,
            error: Some(e),
        },
    }
}

/// Generates a wrapper main, compiles it alongside workspace modules, and runs it.
///
/// Returns the exit code (which equals the function's return value).
fn build_and_run(tc: &TestCase, workspace: &Path) -> Result<i64, String> {
    // Generate wrapper main JSON-LD
    let wrapper = generate_wrapper_main(tc);

    // Create temp workspace with wrapper + all workspace graph files
    let tmp = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let tmp_graph = tmp.path().join(".duumbi").join("graph");
    std::fs::create_dir_all(&tmp_graph).map_err(|e| format!("create tmpdir: {e}"))?;

    // Write the wrapper main
    let wrapper_str =
        serde_json::to_string_pretty(&wrapper).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(tmp_graph.join("main.jsonld"), &wrapper_str)
        .map_err(|e| format!("write wrapper: {e}"))?;

    // Copy all other modules from workspace (excluding existing main.jsonld)
    let ws_graph = workspace.join(".duumbi").join("graph");
    if ws_graph.exists() {
        for entry in std::fs::read_dir(&ws_graph).map_err(|e| format!("read ws graph: {e}"))? {
            let entry = entry.map_err(|e| format!("read dir entry: {e}"))?;
            let src = entry.path();
            if src.extension().and_then(|e| e.to_str()) == Some("jsonld") {
                let fname = src.file_name().expect("invariant: file has name");
                if fname != "main.jsonld" {
                    let dst = tmp_graph.join(fname);
                    std::fs::copy(&src, &dst).map_err(|e| format!("copy module: {e}"))?;
                }
            }
        }
    }

    // Load and compile
    let program = Program::load(tmp.path()).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let objects = lowering::compile_program(&program).map_err(|e| format!("compile: {e}"))?;

    // Find runtime C source and compile it
    let runtime_c_src = include_str!("../../runtime/duumbi_runtime.c");
    let runtime_c = tmp.path().join("duumbi_runtime.c");
    std::fs::write(&runtime_c, runtime_c_src).map_err(|e| format!("write runtime: {e}"))?;
    let runtime_o = tmp.path().join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).map_err(|e| format!("compile runtime: {e}"))?;

    // Write object files and link
    let mut obj_paths = Vec::new();
    let mut sorted_names: Vec<&String> = objects.keys().collect();
    sorted_names.sort_by_key(|n| if n.as_str() == "main" { 0u8 } else { 1u8 });
    for name in &sorted_names {
        let bytes = &objects[*name];
        let path = tmp.path().join(format!("{name}.o"));
        std::fs::write(&path, bytes).map_err(|e| format!("write obj: {e}"))?;
        obj_paths.push(path);
    }

    let binary = tmp.path().join("test_output");
    let obj_refs: Vec<&Path> = obj_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&obj_refs, &runtime_o, &binary).map_err(|e| format!("link: {e}"))?;

    // Run binary, capture exit code
    let output = Command::new(&binary)
        .output()
        .map_err(|e| format!("run: {e}"))?;

    let exit_code = output.status.code().unwrap_or(-1) as i64;
    Ok(exit_code)
}

/// Generates a wrapper `main.jsonld` that calls `tc.function(tc.args...)` and
/// returns/prints the result.
fn generate_wrapper_main(tc: &TestCase) -> serde_json::Value {
    let mut ops: Vec<serde_json::Value> = Vec::new();
    let mut arg_ids: Vec<serde_json::Value> = Vec::new();

    // Const ops for each argument
    for (i, &arg) in tc.args.iter().enumerate() {
        let id = format!("duumbi:main/main/entry/{i}");
        ops.push(json!({
            "@type": "duumbi:Const",
            "@id": id,
            "duumbi:value": arg,
            "duumbi:resultType": "i64"
        }));
        arg_ids.push(json!({ "@id": id }));
    }

    let call_idx = tc.args.len();
    let call_id = format!("duumbi:main/main/entry/{call_idx}");
    ops.push(json!({
        "@type": "duumbi:Call",
        "@id": call_id,
        "duumbi:function": tc.function,
        "duumbi:args": arg_ids,
        "duumbi:resultType": "i64"
    }));

    let print_idx = call_idx + 1;
    let print_id = format!("duumbi:main/main/entry/{print_idx}");
    ops.push(json!({
        "@type": "duumbi:Print",
        "@id": print_id,
        "duumbi:operand": { "@id": call_id }
    }));

    let ret_idx = print_idx + 1;
    let ret_id = format!("duumbi:main/main/entry/{ret_idx}");
    ops.push(json!({
        "@type": "duumbi:Return",
        "@id": ret_id,
        "duumbi:operand": { "@id": call_id }
    }));

    json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": ops
            }]
        }]
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_wrapper_main_correct_structure() {
        let tc = TestCase {
            name: "test".to_string(),
            function: "add".to_string(),
            args: vec![3, 5],
            expected_return: 8,
        };
        let wrapper = generate_wrapper_main(&tc);
        let ops = &wrapper["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"];
        assert!(ops.is_array());
        // 2 Const + 1 Call + 1 Print + 1 Return = 5
        assert_eq!(ops.as_array().unwrap().len(), 5);
        assert_eq!(ops[2]["@type"], "duumbi:Call");
        assert_eq!(ops[2]["duumbi:function"], "add");
    }

    #[test]
    fn test_report_all_passed() {
        let report = TestReport {
            passed: 3,
            failed: 0,
            results: vec![],
        };
        assert!(report.all_passed());
    }

    #[test]
    fn test_report_display_pass() {
        let report = TestReport {
            passed: 1,
            failed: 0,
            results: vec![TestResult {
                name: "addition".to_string(),
                function: "add".to_string(),
                args: vec![3, 5],
                expected: 8,
                actual: Some(8),
                passed: true,
                error: None,
            }],
        };
        let display = report.display();
        assert!(display.contains("✓ addition"));
        assert!(display.contains("1/1 passed"));
    }

    #[test]
    fn test_report_display_fail() {
        let report = TestReport {
            passed: 0,
            failed: 1,
            results: vec![TestResult {
                name: "sub".to_string(),
                function: "sub".to_string(),
                args: vec![10, 3],
                expected: 7,
                actual: Some(6),
                passed: false,
                error: None,
            }],
        };
        let display = report.display();
        assert!(display.contains("✗ sub"));
        assert!(display.contains("expected 7"));
    }
}
