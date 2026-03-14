//! Phase 1 end-to-end integration tests.
//!
//! Tests multi-function, branching, recursive calls, and CLI commands.

use std::process::Command;

/// Helper: compile a fixture and return the output binary path.
fn compile_fixture(fixture: &str, output_name: &str) -> std::path::PathBuf {
    let tmp_dir = std::env::temp_dir().join("duumbi_phase1_tests");
    std::fs::create_dir_all(&tmp_dir).expect("invariant: temp dir must be creatable");
    let output_binary = tmp_dir.join(output_name);

    let duumbi_output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "build",
            fixture,
            "-o",
            &output_binary.to_string_lossy(),
        ])
        .output()
        .expect("invariant: cargo run must be runnable");

    assert!(
        duumbi_output.status.success(),
        "duumbi build of {fixture} failed: {}",
        String::from_utf8_lossy(&duumbi_output.stderr)
    );

    output_binary
}

#[test]
fn phase1_fibonacci_prints_55() {
    let binary = compile_fixture("tests/fixtures/fibonacci.jsonld", "fib_test");

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "55",
        "Expected fibonacci(10) = 55, got '{}'",
        stdout.trim()
    );

    let exit_code = output
        .status
        .code()
        .expect("invariant: binary must have an exit code");
    assert_eq!(exit_code, 55, "Expected exit code 55, got {exit_code}");

    let _ = std::fs::remove_file(&binary);
}

#[test]
fn phase1_hello_prints_multiple_lines() {
    let binary = compile_fixture("tests/fixtures/hello.jsonld", "hello_test");

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "Expected 3 print lines, got {}",
        lines.len()
    );
    assert_eq!(lines[0], "72");
    assert_eq!(lines[1], "101");
    assert_eq!(lines[2], "108");

    let exit_code = output
        .status
        .code()
        .expect("invariant: binary must have an exit code");
    assert_eq!(exit_code, 0, "Expected exit code 0, got {exit_code}");

    let _ = std::fs::remove_file(&binary);
}

#[test]
fn phase1_check_valid_file() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "check",
            "tests/fixtures/fibonacci.jsonld",
        ])
        .output()
        .expect("invariant: cargo run must be runnable");

    assert!(
        output.status.success(),
        "duumbi check should succeed for valid fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn phase1_describe_produces_output() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "describe",
            "tests/fixtures/add.jsonld",
        ])
        .output()
        .expect("invariant: cargo run must be runnable");

    assert!(
        output.status.success(),
        "duumbi describe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function main"),
        "Describe output should contain 'function main'"
    );
    assert!(
        stdout.contains("Const(3)"),
        "Describe output should contain 'Const(3)'"
    );
}

#[test]
fn phase1_workspace_init_build_run() {
    // First ensure duumbi is built
    let build_status = Command::new("cargo")
        .args(["build", "--quiet"])
        .output()
        .expect("invariant: cargo build must be runnable");
    assert!(build_status.status.success(), "cargo build failed");

    let duumbi_bin = std::path::PathBuf::from("target/debug/duumbi")
        .canonicalize()
        .expect("invariant: duumbi binary must exist after build");

    let tmp_dir = std::env::temp_dir().join("duumbi_workspace_test");
    let _ = std::fs::remove_dir_all(&tmp_dir);

    // Init
    let init_output = Command::new(&duumbi_bin)
        .args(["init", &tmp_dir.to_string_lossy()])
        .output()
        .expect("invariant: duumbi init must be runnable");

    assert!(
        init_output.status.success(),
        "duumbi init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    assert!(tmp_dir.join(".duumbi/config.toml").exists());
    assert!(tmp_dir.join(".duumbi/graph/main.jsonld").exists());

    // Build (workspace-aware — run duumbi from within the workspace dir)
    let build_output = Command::new(&duumbi_bin)
        .args(["build"])
        .current_dir(&tmp_dir)
        .output()
        .expect("invariant: duumbi build must be runnable");

    assert!(
        build_output.status.success(),
        "duumbi build in workspace failed: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    assert!(tmp_dir.join(".duumbi/build/output").exists());

    // Run the compiled binary
    let binary_output = Command::new(tmp_dir.join(".duumbi/build/output"))
        .output()
        .expect("invariant: compiled binary must be runnable");

    // The skeleton program is a minimal Const(0) + Return — it exits cleanly
    // with no printed output (no Print op).
    assert!(
        binary_output.status.success(),
        "Compiled skeleton binary must exit with code 0"
    );

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
