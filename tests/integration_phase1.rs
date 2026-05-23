//! Phase 1 end-to-end integration tests.
//!
//! Tests multi-function, branching, recursive calls, and CLI commands.

use std::process::Command;

fn duumbi_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_duumbi"))
}

fn native_output_path(path: &std::path::Path) -> std::path::PathBuf {
    if path.exists() || std::env::consts::EXE_SUFFIX.is_empty() {
        return path.to_path_buf();
    }

    std::path::PathBuf::from(format!(
        "{}{}",
        path.display(),
        std::env::consts::EXE_SUFFIX
    ))
}

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
fn phase1_build_help_lists_trace_flag() {
    let output = Command::new(duumbi_bin())
        .args(["build", "--help"])
        .output()
        .expect("invariant: duumbi help must be runnable");

    assert!(
        output.status.success(),
        "duumbi build --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--trace"));
    assert!(stdout.contains("local traced build"));
}

#[test]
fn phase1_trace_single_file_build_accepts_defaults() {
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir");
    let output_binary = tmp.path().join("hello-traced");

    let output = Command::new(duumbi_bin())
        .args([
            "build",
            "--trace",
            "tests/fixtures/hello.jsonld",
            "-o",
            &output_binary.to_string_lossy(),
        ])
        .output()
        .expect("invariant: duumbi build must be runnable");

    assert!(
        output.status.success(),
        "duumbi build --trace failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(native_output_path(&output_binary).exists());
    assert!(!tmp.path().join(".duumbi/telemetry/traces.jsonl").exists());
    assert!(
        !tmp.path()
            .join(".duumbi/telemetry/crash_dump.jsonl")
            .exists()
    );
}

#[test]
fn phase1_config_semantic_errors_only_block_trace_builds() {
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir");
    let duumbi_dir = tmp.path().join(".duumbi");
    std::fs::create_dir_all(&duumbi_dir).expect("invariant: .duumbi must be creatable");
    std::fs::write(
        duumbi_dir.join("config.toml"),
        r#"
[telemetry]
sample-rate = 2.0
"#,
    )
    .expect("invariant: config must be writable");

    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello.jsonld");
    let default_binary = tmp.path().join("hello-default");
    let default_output = Command::new(duumbi_bin())
        .args([
            "build",
            &fixture.to_string_lossy(),
            "-o",
            &default_binary.to_string_lossy(),
        ])
        .current_dir(tmp.path())
        .output()
        .expect("invariant: duumbi build must be runnable");

    assert!(
        default_output.status.success(),
        "default build should ignore telemetry semantic errors: {}",
        String::from_utf8_lossy(&default_output.stderr)
    );
    assert!(native_output_path(&default_binary).exists());

    let traced_binary = tmp.path().join("hello-traced");
    let traced_output = Command::new(duumbi_bin())
        .args([
            "build",
            "--trace",
            &fixture.to_string_lossy(),
            "-o",
            &traced_binary.to_string_lossy(),
        ])
        .current_dir(tmp.path())
        .output()
        .expect("invariant: duumbi build must be runnable");

    assert!(
        !traced_output.status.success(),
        "traced build should reject invalid telemetry config"
    );
    let stderr = String::from_utf8_lossy(&traced_output.stderr);
    assert!(stderr.contains("sample-rate"), "{stderr}");
    assert!(stderr.contains("0.0 and 1.0"), "{stderr}");
}

#[test]
fn phase1_trace_rejects_invalid_sample_rate_before_compilation() {
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir");
    let duumbi_dir = tmp.path().join(".duumbi");
    std::fs::create_dir_all(&duumbi_dir).expect("invariant: .duumbi must be creatable");
    std::fs::write(
        duumbi_dir.join("config.toml"),
        r#"
[telemetry]
sample-rate = 2.0
"#,
    )
    .expect("invariant: config must be writable");

    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello.jsonld");
    let traced_binary = tmp.path().join("hello-traced");
    let traced_output = Command::new(duumbi_bin())
        .args([
            "build",
            "--trace",
            &fixture.to_string_lossy(),
            "-o",
            &traced_binary.to_string_lossy(),
        ])
        .current_dir(tmp.path())
        .output()
        .expect("invariant: duumbi build must be runnable");

    assert!(
        !traced_output.status.success(),
        "traced build should reject invalid telemetry config"
    );
    let stderr = String::from_utf8_lossy(&traced_output.stderr);
    assert!(stderr.contains("sample-rate"), "{stderr}");
    assert!(stderr.contains("0.0 and 1.0"), "{stderr}");
    assert!(
        !stderr.contains("Failed to link binary"),
        "validation should fail before linking: {stderr}"
    );
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

    let raw = String::from_utf8_lossy(&output.stdout);
    // Strip ANSI escape codes for content comparison.
    let stdout: String = {
        let mut out = String::with_capacity(raw.len());
        let mut esc = false;
        for c in raw.chars() {
            if c == '\x1b' {
                esc = true;
            } else if esc {
                if c == 'm' {
                    esc = false;
                }
            } else {
                out.push(c);
            }
        }
        out
    };
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

    let duumbi_bin = std::path::PathBuf::from(format!(
        "target/debug/duumbi{}",
        std::env::consts::EXE_SUFFIX
    ))
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
    let init_stdout = String::from_utf8_lossy(&init_output.stdout);
    let init_stderr = String::from_utf8_lossy(&init_output.stderr);
    assert!(!init_stdout.contains("Next steps"));
    assert!(!init_stderr.contains("Next steps"));
    assert!(!init_stdout.contains("LLM provider not available"));
    assert!(!init_stderr.contains("LLM provider not available"));

    assert!(tmp_dir.join(".duumbi/config.toml").exists());
    assert!(tmp_dir.join(".duumbi/graph/main.jsonld").exists());
    let config = std::fs::read_to_string(tmp_dir.join(".duumbi/config.toml"))
        .expect("invariant: config must be readable");
    assert!(config.contains("name = \"duumbi_workspace_test\""));
    assert!(config.contains("namespace = \"duumbi-workspace-test\""));

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

    let workspace_output = duumbi::workspace::workspace_output_path(&tmp_dir);
    assert!(workspace_output.exists());

    // Run the compiled binary
    let binary_output = Command::new(&workspace_output)
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

#[test]
fn phase1_workspace_trace_build_supports_offline() {
    let duumbi_bin = duumbi_bin();
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir");
    let workspace = tmp.path().join("ws");

    let init_output = Command::new(&duumbi_bin)
        .args(["init", &workspace.to_string_lossy()])
        .output()
        .expect("invariant: duumbi init must be runnable");
    assert!(
        init_output.status.success(),
        "duumbi init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );
    std::fs::write(
        workspace.join(".duumbi/config.toml"),
        r#"
[workspace]
name = "ws"
namespace = "ws"
"#,
    )
    .expect("invariant: dependency-free config must be writable");

    let build_output = Command::new(&duumbi_bin)
        .args(["build", "--trace", "--offline"])
        .current_dir(&workspace)
        .output()
        .expect("invariant: duumbi build must be runnable");
    assert!(
        build_output.status.success(),
        "workspace duumbi build --trace --offline failed: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    let workspace_output = duumbi::workspace::workspace_output_path(&workspace);
    assert!(workspace_output.exists());
    assert!(!workspace.join(".duumbi/telemetry/traces.jsonl").exists());
    assert!(
        !workspace
            .join(".duumbi/telemetry/crash_dump.jsonl")
            .exists()
    );
}
