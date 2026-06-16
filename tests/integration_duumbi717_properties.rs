//! DUUMBI-717 property-check CLI integration evidence.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new("tests")
        .join("fixtures")
        .join("properties")
        .join(name)
}

fn evidence_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "duumbi-717-{label}-{}-{nanos}.json",
        std::process::id()
    ))
}

fn run_properties(name: &str, label: &str, cases: &str) -> (Output, PathBuf) {
    let output = evidence_path(label);
    let command_output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .arg("check")
        .arg(fixture(name))
        .arg("--properties")
        .arg("--seed")
        .arg("717")
        .arg("--cases")
        .arg(cases)
        .arg("--property-output")
        .arg(&output)
        .output()
        .expect("duumbi check --properties must run");

    (command_output, output)
}

fn evidence(path: &Path) -> Value {
    let text = fs::read_to_string(path).expect("property evidence must be written");
    assert!(
        text.len() < 64 * 1024,
        "property evidence must stay compact"
    );
    assert!(!text.contains("api_key"));
    assert!(!text.contains("provider"));
    serde_json::from_str(&text).expect("property evidence must be valid JSON")
}

#[test]
fn passing_property_fixture_writes_deterministic_pass_evidence() {
    let (output, path) = run_properties("passing_identity.jsonld", "passing", "3");
    assert!(
        output.status.success(),
        "passing property check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Property evidence written"));
    let json = evidence(&path);

    assert_eq!(json["schema_version"], "duumbi.property_evidence.v1");
    assert_eq!(json["settings"]["seed"], 717);
    assert_eq!(json["settings"]["cases"], 3);
    assert_eq!(json["summary"]["functions_discovered"], 1);
    assert_eq!(json["summary"]["functions_checked"], 1);
    assert_eq!(json["summary"]["functions_unsupported"], 0);
    assert_eq!(json["summary"]["properties_failed"], 0);
    assert_eq!(json["functions"][0]["status"], "passed");
    assert_eq!(json["functions"][0]["cases_executed"], 3);
    assert_eq!(json["functions"][0]["postconditions_checked"], 3);
}

#[test]
fn precondition_fixture_counts_rejections_without_hiding_executed_cases() {
    let (output, path) =
        run_properties("precondition_positive_identity.jsonld", "precondition", "3");
    assert!(
        output.status.success(),
        "precondition property check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json = evidence(&path);
    assert_eq!(json["summary"]["functions_checked"], 1);
    assert_eq!(json["functions"][0]["status"], "passed");
    assert_eq!(json["functions"][0]["cases_executed"], 1);
    assert_eq!(json["functions"][0]["cases_rejected"], 2);
    assert_eq!(json["functions"][0]["postconditions_checked"], 1);
}

#[test]
fn failing_property_fixture_exits_nonzero_and_records_counterexample() {
    let (output, path) = run_properties("failing_identity_nonnegative.jsonld", "failing", "3");
    assert!(
        !output.status.success(),
        "failing property check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Property check failed"),
        "stderr should summarize property failure"
    );

    let json = evidence(&path);
    assert_eq!(json["summary"]["functions_discovered"], 1);
    assert_eq!(json["summary"]["functions_checked"], 1);
    assert_eq!(json["summary"]["properties_failed"], 1);
    assert_eq!(json["functions"][0]["status"], "failed");
    assert_eq!(
        json["functions"][0]["failure"]["contract_id"],
        "result-nonnegative"
    );
    assert_eq!(json["functions"][0]["failure"]["actual"], "result=-1");
    assert_eq!(json["functions"][0]["failure"]["shrink_status"], "minimal");
    assert_eq!(
        json["functions"][0]["failure"]["counterexample"][0]["kind"],
        "i64"
    );
    assert_eq!(
        json["functions"][0]["failure"]["counterexample"][0]["value"],
        -1
    );
}

#[test]
fn unsupported_resource_fixture_records_reason_without_generating_handles() {
    let (output, path) = run_properties("unsupported_resource.jsonld", "unsupported-resource", "3");
    assert!(
        output.status.success(),
        "unsupported resource policy should write evidence without failing check\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json = evidence(&path);
    assert_eq!(json["summary"]["functions_discovered"], 1);
    assert_eq!(json["summary"]["functions_checked"], 0);
    assert_eq!(json["summary"]["functions_unsupported"], 1);
    assert_eq!(json["functions"][0]["status"], "unsupported");
    assert_eq!(
        json["functions"][0]["unsupported"]["reason"],
        "unsupported_resource_db_rows"
    );
    assert_eq!(json["functions"][0]["cases_generated"], 0);
    assert_eq!(json["functions"][0]["cases_executed"], 0);
}

#[test]
fn malformed_contract_fails_before_property_evidence_is_written() {
    let output = evidence_path("malformed");
    let command_output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .arg("check")
        .arg(fixture("malformed_contract.jsonld"))
        .arg("--properties")
        .arg("--seed")
        .arg("717")
        .arg("--cases")
        .arg("3")
        .arg("--property-output")
        .arg(&output)
        .output()
        .expect("duumbi check --properties must run");

    assert!(
        !command_output.status.success(),
        "malformed contract unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&command_output.stdout),
        String::from_utf8_lossy(&command_output.stderr)
    );
    assert!(String::from_utf8_lossy(&command_output.stderr).contains("unknown contract operator"));
    assert!(
        !output.exists(),
        "property evidence must not be written when validation fails"
    );
}
