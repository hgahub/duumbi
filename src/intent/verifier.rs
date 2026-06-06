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

use std::path::{Path, PathBuf};
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

/// Compiles and runs a test case, returning the function's return value.
///
/// For regular functions, generates a wrapper main that calls the function,
/// prints the result, and returns it. For `function: "main"` test cases,
/// uses the workspace's own main directly (no wrapper) to avoid self-recursion.
fn build_and_run(tc: &TestCase, workspace: &Path) -> Result<i64, String> {
    let tmp = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let tmp_graph = tmp.path().join(".duumbi").join("graph");
    std::fs::create_dir_all(&tmp_graph).map_err(|e| format!("create tmpdir: {e}"))?;

    let ws_graph = workspace.join(".duumbi").join("graph");

    if tc.function == "main" {
        // Copy ALL modules including main.jsonld — run the workspace as-is
        copy_all_modules(&ws_graph, &tmp_graph)?;
    } else {
        // Copy ALL non-main modules first
        copy_non_main_modules(&ws_graph, &tmp_graph)?;

        // Check if the target function lives in main.jsonld
        let main_path = ws_graph.join("main.jsonld");
        let target_in_main = main_path.exists() && {
            std::fs::read_to_string(&main_path)
                .ok()
                .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
                .map(|v| has_function(&v, &tc.function))
                .unwrap_or(false)
        };

        if target_in_main {
            // The target function is defined in main.jsonld — inject the wrapper
            // main() alongside existing functions instead of replacing the file
            let main_content = std::fs::read_to_string(&main_path)
                .map_err(|e| format!("read main.jsonld: {e}"))?;
            let main_value: serde_json::Value = serde_json::from_str(&main_content)
                .map_err(|e| format!("parse main.jsonld: {e}"))?;
            let merged = inject_wrapper_main(main_value, tc);
            let merged_str =
                serde_json::to_string_pretty(&merged).map_err(|e| format!("serialize: {e}"))?;
            std::fs::write(tmp_graph.join("main.jsonld"), &merged_str)
                .map_err(|e| format!("write merged main: {e}"))?;
        } else {
            // Target function is in a separate module — generate a clean wrapper main
            let wrapper = generate_wrapper_main(tc);
            let wrapper_str =
                serde_json::to_string_pretty(&wrapper).map_err(|e| format!("serialize: {e}"))?;
            std::fs::write(tmp_graph.join("main.jsonld"), &wrapper_str)
                .map_err(|e| format!("write wrapper: {e}"))?;
        }
    }

    compile_and_run(tmp.path(), &tc.function)
}

/// Copies all `.jsonld` files from `src_dir` to `dst_dir`.
fn copy_all_modules(src_dir: &Path, dst_dir: &Path) -> Result<(), String> {
    copy_modules(src_dir, dst_dir, true)
}

/// Copies all `.jsonld` files from `src_dir` to `dst_dir`, excluding `main.jsonld`.
fn copy_non_main_modules(src_dir: &Path, dst_dir: &Path) -> Result<(), String> {
    copy_modules(src_dir, dst_dir, false)
}

fn copy_modules(src_dir: &Path, dst_dir: &Path, include_main: bool) -> Result<(), String> {
    if !src_dir.exists() {
        return Ok(());
    }

    for src in collect_jsonld_paths(src_dir)? {
        let fname = src.file_name().expect("invariant: file has name");
        if !include_main && fname == "main.jsonld" {
            continue;
        }
        let rel = src
            .strip_prefix(src_dir)
            .map_err(|e| format!("strip module prefix: {e}"))?;
        let dst = dst_dir.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create module dir: {e}"))?;
        }
        std::fs::copy(&src, &dst).map_err(|e| format!("copy module: {e}"))?;
    }
    Ok(())
}

fn collect_jsonld_paths(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    collect_jsonld_paths_into(dir, &mut paths)?;
    Ok(paths)
}

fn collect_jsonld_paths_into(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read ws graph: {e}"))? {
        let entry = entry.map_err(|e| format!("read dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonld_paths_into(&path, paths)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonld") {
            paths.push(path);
        }
    }
    Ok(())
}

/// Compiles a workspace and runs the binary, returning the last printed i64 value.
fn compile_and_run(tmp_workspace: &Path, function: &str) -> Result<i64, String> {
    let program = Program::load(tmp_workspace).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let objects = lowering::compile_program(&program).map_err(|e| format!("compile: {e}"))?;

    // Compile runtime
    let runtime_c_src = include_str!("../../runtime/duumbi_runtime.c");
    let runtime_c = tmp_workspace.join("duumbi_runtime.c");
    std::fs::write(&runtime_c, runtime_c_src).map_err(|e| format!("write runtime: {e}"))?;
    let runtime_o = tmp_workspace.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).map_err(|e| format!("compile runtime: {e}"))?;

    // Write object files and link
    let mut obj_paths = Vec::new();
    let mut sorted_names: Vec<&String> = objects.keys().collect();
    sorted_names.sort_by_key(|n| if n.as_str() == "main" { 0u8 } else { 1u8 });
    for name in &sorted_names {
        let bytes = &objects[*name];
        let path = tmp_workspace.join(format!("{name}.o"));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create obj dir: {e}"))?;
        }
        std::fs::write(&path, bytes).map_err(|e| format!("write obj '{name}': {e}"))?;
        obj_paths.push(path);
    }

    let binary = tmp_workspace.join("test_output");
    let obj_refs: Vec<&Path> = obj_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&obj_refs, &runtime_o, &binary).map_err(|e| format!("link: {e}"))?;

    // Run binary, capture stdout
    let output = Command::new(&binary)
        .output()
        .map_err(|e| format!("run: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if function == "main" {
        // For main tests, use exit code since main's return value may not be printed
        // (the workspace main may or may not print it). Fall back to exit code
        // which is correct for 0–255 values.
        // Try stdout first, fall back to exit code.
        let last_line = stdout.lines().last().unwrap_or("").trim();
        if let Ok(val) = last_line.parse::<i64>() {
            return Ok(val);
        }
        let exit_code = output.status.code().unwrap_or(-1) as i64;
        Ok(exit_code)
    } else {
        // For wrapper tests, the last printed line is the return value
        let last_line = stdout.lines().last().unwrap_or("").trim();
        last_line
            .parse::<i64>()
            .map_err(|e| format!("failed to parse stdout '{last_line}' as i64: {e}"))
    }
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

/// Checks if a JSON-LD module value contains a function with the given name.
fn has_function(module: &serde_json::Value, function_name: &str) -> bool {
    module["duumbi:functions"]
        .as_array()
        .map(|funcs| {
            funcs
                .iter()
                .any(|f| f["duumbi:name"].as_str() == Some(function_name))
        })
        .unwrap_or(false)
}

/// Injects a wrapper main() function into an existing module, preserving all
/// other functions. This is used when the target function is defined in
/// main.jsonld — we need to keep it while replacing the main() body.
fn inject_wrapper_main(mut module: serde_json::Value, tc: &TestCase) -> serde_json::Value {
    // Build the wrapper main function (same logic as generate_wrapper_main)
    let wrapper = generate_wrapper_main(tc);
    let wrapper_main_fn = &wrapper["duumbi:functions"][0];

    // Replace or append the main function in the module
    if let Some(funcs) = module["duumbi:functions"].as_array_mut() {
        // Find and replace existing main function
        let main_idx = funcs
            .iter()
            .position(|f| f["duumbi:name"].as_str() == Some("main"));

        if let Some(idx) = main_idx {
            funcs[idx] = wrapper_main_fn.clone();
        } else {
            // No main function exists — append the wrapper
            funcs.push(wrapper_main_fn.clone());
        }
    }

    module
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

    #[test]
    fn has_function_finds_existing() {
        let module = json!({
            "duumbi:functions": [
                { "duumbi:name": "main" },
                { "duumbi:name": "double" }
            ]
        });
        assert!(has_function(&module, "double"));
        assert!(has_function(&module, "main"));
        assert!(!has_function(&module, "nonexistent"));
    }

    #[test]
    fn has_function_empty_module() {
        let module = json!({ "duumbi:functions": [] });
        assert!(!has_function(&module, "anything"));
    }

    #[test]
    fn inject_wrapper_replaces_main_preserves_others() {
        let module = json!({
            "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
            "@type": "duumbi:Module",
            "@id": "duumbi:main",
            "duumbi:name": "main",
            "duumbi:functions": [
                {
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/double",
                    "duumbi:name": "double",
                    "duumbi:returnType": "i64",
                    "duumbi:params": [{ "duumbi:name": "n", "duumbi:paramType": "i64" }],
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/double/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": []
                    }]
                },
                {
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/main",
                    "duumbi:name": "main",
                    "duumbi:returnType": "i64",
                    "duumbi:blocks": [{
                        "@type": "duumbi:Block",
                        "@id": "duumbi:main/main/entry",
                        "duumbi:label": "entry",
                        "duumbi:ops": []
                    }]
                }
            ]
        });

        let tc = TestCase {
            name: "double_21".to_string(),
            function: "double".to_string(),
            args: vec![21],
            expected_return: 42,
        };

        let merged = inject_wrapper_main(module, &tc);
        let funcs = merged["duumbi:functions"]
            .as_array()
            .expect("functions array");

        // Should have exactly 2 functions: double (preserved) + main (replaced)
        assert_eq!(funcs.len(), 2, "must preserve double + replace main");

        // double should still be there
        assert!(
            funcs
                .iter()
                .any(|f| f["duumbi:name"].as_str() == Some("double")),
            "double function must be preserved"
        );

        // main should now contain the wrapper (Call to double)
        let main_fn = funcs
            .iter()
            .find(|f| f["duumbi:name"].as_str() == Some("main"))
            .expect("main function");
        let ops = &main_fn["duumbi:blocks"][0]["duumbi:ops"];
        // 1 Const(21) + 1 Call(double) + 1 Print + 1 Return = 4
        assert_eq!(
            ops.as_array().map(|a| a.len()).unwrap_or(0),
            4,
            "wrapper main should have 4 ops"
        );
        assert_eq!(ops[1]["duumbi:function"], "double");
    }

    #[test]
    fn inject_wrapper_appends_main_if_missing() {
        let module = json!({
            "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
            "@type": "duumbi:Module",
            "@id": "duumbi:main",
            "duumbi:name": "main",
            "duumbi:functions": [
                {
                    "@type": "duumbi:Function",
                    "@id": "duumbi:main/helper",
                    "duumbi:name": "helper",
                    "duumbi:returnType": "i64",
                    "duumbi:blocks": []
                }
            ]
        });

        let tc = TestCase {
            name: "test".to_string(),
            function: "helper".to_string(),
            args: vec![5],
            expected_return: 5,
        };

        let merged = inject_wrapper_main(module, &tc);
        let funcs = merged["duumbi:functions"]
            .as_array()
            .expect("functions array");

        // Should have 2 functions: helper (original) + main (appended)
        assert_eq!(funcs.len(), 2);
        assert!(
            funcs
                .iter()
                .any(|f| f["duumbi:name"].as_str() == Some("main"))
        );
        assert!(
            funcs
                .iter()
                .any(|f| f["duumbi:name"].as_str() == Some("helper"))
        );
    }
}
