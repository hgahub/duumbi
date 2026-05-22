//! Telemetry integration tests for traced local build/run evidence.

use std::process::Command;

#[test]
fn traced_option_none_unwrap_writes_crash_evidence_and_inspects() {
    let duumbi = env!("CARGO_BIN_EXE_duumbi");
    let tmp = tempfile::TempDir::new().expect("invariant: temp dir must be created");
    let telemetry_dir = tmp.path().join("telemetry");
    let binary = tmp
        .path()
        .join(format!("panic{}", std::env::consts::EXE_SUFFIX));

    let build = Command::new(duumbi)
        .args([
            "build",
            "--trace",
            "tests/fixtures/telemetry/option_none_unwrap.jsonld",
            "-o",
            binary.to_str().expect("invariant: temp path must be UTF-8"),
        ])
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: duumbi build must run");
    assert!(
        build.status.success(),
        "traced build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DUUMBI_TELEMETRY_DIR", &telemetry_dir)
        .output()
        .expect("invariant: traced binary must run");
    assert!(
        !run.status.success(),
        "controlled panic fixture must exit nonzero"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("duumbi panic: called Option::unwrap() on a None value"),
        "panic stderr must preserve original message, got: {stderr}"
    );

    assert!(telemetry_dir.join("trace_map.json").exists());
    assert!(telemetry_dir.join("traces.jsonl").exists());
    assert!(telemetry_dir.join("crash_dump.jsonl").exists());

    let inspect = Command::new(duumbi)
        .args([
            "telemetry",
            "inspect",
            "--telemetry-dir",
            telemetry_dir
                .to_str()
                .expect("invariant: telemetry path must be UTF-8"),
        ])
        .output()
        .expect("invariant: telemetry inspect must run");
    assert!(
        inspect.status.success(),
        "telemetry inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(stdout.contains("Function: duumbi:telemetry/main"));
    assert!(stdout.contains("Block: duumbi:telemetry/main/entry"));
    assert!(stdout.contains("Exact node evidence: unavailable in v1"));
}
