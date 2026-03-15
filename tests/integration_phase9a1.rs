//! Phase 9a-1 end-to-end integration tests.
//!
//! Kill criterion: String concat+print, Array push+get+length, Struct create+field
//! access → all compile, link, and run correctly. Phase 0-8 tests green.

use std::process::Command;

/// Helper: compile a fixture via `duumbi build` and return the binary path.
fn compile_fixture(fixture: &str, output_name: &str) -> std::path::PathBuf {
    let tmp_dir = std::env::temp_dir().join("duumbi_phase9a1_tests");
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
fn string_concat_prints_hello_world() {
    let binary = compile_fixture("tests/fixtures/string_concat.jsonld", "string_concat_test");

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "hello world",
        "Expected 'hello world', got '{}'",
        stdout.trim()
    );

    let exit_code = output
        .status
        .code()
        .expect("invariant: binary must have an exit code");
    assert_eq!(exit_code, 0, "Expected exit code 0, got {exit_code}");

    let _ = std::fs::remove_file(&binary);
}

#[test]
fn string_length_prints_6() {
    let binary = compile_fixture("tests/fixtures/string_length.jsonld", "string_length_test");

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "6",
        "Expected '6' (length of 'duumbi'), got '{}'",
        stdout.trim()
    );

    let _ = std::fs::remove_file(&binary);
}

#[test]
fn array_push_get_length() {
    let binary = compile_fixture(
        "tests/fixtures/array_push_get.jsonld",
        "array_push_get_test",
    );

    let output = Command::new(&binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected 2 print lines, got {}",
        lines.len()
    );
    assert_eq!(lines[0], "3", "Expected array length 3");
    assert_eq!(lines[1], "20", "Expected arr[1] = 20");

    let exit_code = output
        .status
        .code()
        .expect("invariant: binary must have an exit code");
    assert_eq!(exit_code, 0, "Expected exit code 0, got {exit_code}");

    let _ = std::fs::remove_file(&binary);
}
