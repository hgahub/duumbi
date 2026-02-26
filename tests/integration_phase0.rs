//! Phase 0 end-to-end integration test.
//!
//! Kill criterion: `add(3, 5)` JSON-LD → native binary → stdout "8\n", exit code 8.

use std::process::Command;

#[test]
fn phase0_add_3_5_prints_8_exits_8() {
    // 1. Build the duumbi binary
    let build_status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("invariant: cargo build must be runnable");
    assert!(build_status.success(), "cargo build failed");

    // 2. Run duumbi build on the fixture
    let tmp_dir = std::env::temp_dir().join("duumbi_integration_test");
    std::fs::create_dir_all(&tmp_dir).expect("invariant: temp dir must be creatable");
    let output_binary = tmp_dir.join("add_test");

    let duumbi_output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "build",
            "tests/fixtures/add.jsonld",
            "-o",
            &output_binary.to_string_lossy(),
        ])
        .output()
        .expect("invariant: cargo run must be runnable");

    assert!(
        duumbi_output.status.success(),
        "duumbi build failed: {}",
        String::from_utf8_lossy(&duumbi_output.stderr)
    );

    // 3. Run the compiled binary
    let binary_output = Command::new(&output_binary)
        .output()
        .expect("invariant: compiled binary must be runnable");

    let stdout = String::from_utf8_lossy(&binary_output.stdout);
    assert_eq!(
        stdout.trim(),
        "8",
        "Expected stdout '8', got '{}'",
        stdout.trim()
    );

    // 4. Check exit code (low 8 bits of return value)
    let exit_code = binary_output
        .status
        .code()
        .expect("invariant: binary must have an exit code");
    assert_eq!(exit_code, 8, "Expected exit code 8, got {exit_code}");

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
