//! Phase 9a-1 end-to-end integration tests.
//!
//! Tests string concat+print, string length, and string literal operations.
//! Kill criterion: String concat+print compiles, links, and runs correctly.

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
