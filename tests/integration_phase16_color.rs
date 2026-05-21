//! Phase 16 terminal color fallback integration tests.

use std::path::Path;
use std::process::Command;

fn duumbi_check_output(no_color: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_duumbi"));
    command
        .args([
            "check",
            Path::new("tests")
                .join("fixtures")
                .join("fibonacci.jsonld")
                .to_str()
                .expect("invariant: fixture path must be valid UTF-8"),
        ])
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE");

    if no_color {
        command.env("NO_COLOR", "1");
    } else {
        command.env_remove("NO_COLOR");
    }

    command
        .output()
        .expect("invariant: duumbi binary must be runnable")
}

fn contains_ansi_escape(output: &str) -> bool {
    output.contains("\x1b[")
}

#[test]
fn no_color_check_output_has_no_ansi_escapes() {
    let output = duumbi_check_output(true);
    assert!(
        output.status.success(),
        "duumbi check should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Validation passed"));
    assert!(
        !contains_ansi_escape(&stderr),
        "NO_COLOR output must not contain ANSI escapes: {stderr:?}"
    );
}

#[test]
fn captured_check_output_has_no_raw_ansi_escapes() {
    let output = duumbi_check_output(false);
    assert!(
        output.status.success(),
        "duumbi check should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Validation passed"));
    assert!(
        !contains_ansi_escape(&stderr),
        "captured output must not contain ANSI escapes: {stderr:?}"
    );
}
