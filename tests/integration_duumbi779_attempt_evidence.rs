//! DUUMBI-779 attempt evidence integration checks (no live LLM).

use std::process::Command;

use duumbi::bench::report::{BenchmarkResult, ErrorCategory};
use duumbi::intent::attempt::EvidencePersistence;
use duumbi::intent::taxonomy::{RootCauseAttribution, RootCauseClass};

#[test]
fn unknown_failure_json_keeps_error_category_null_key() {
    let json = r#"{
      "showcase": "scaled_math_pipeline",
      "provider": "mock",
      "attempt": 1,
      "success": false,
      "tests_passed": 0,
      "tests_total": 1,
      "duration_secs": 0.4,
      "repair_attempted": false,
      "root_cause": "unknown",
      "root_cause_attribution": "no_matching_rule",
      "error_category": null,
      "evidence_persistence": "complete"
    }"#;
    let result: BenchmarkResult = serde_json::from_str(json).expect("parse unknown failure");
    assert_eq!(result.root_cause, Some(RootCauseClass::Unknown));
    assert_eq!(
        result.root_cause_attribution,
        Some(RootCauseAttribution::NoMatchingRule)
    );
    assert_eq!(result.error_category, None);
    assert_eq!(
        result.evidence_persistence,
        Some(EvidencePersistence::Complete)
    );
    let encoded = serde_json::to_value(&result).expect("encode");
    assert!(encoded.get("error_category").unwrap().is_null());
}

#[test]
fn classified_failure_maps_schema_root_cause_to_schema_error() {
    let json = r#"{
      "showcase": "scaled_math_pipeline",
      "provider": "mock",
      "attempt": 1,
      "success": false,
      "tests_passed": 0,
      "tests_total": 4,
      "duration_secs": 1.0,
      "repair_attempted": true,
      "root_cause": "schema_graph_validation",
      "root_cause_attribution": "matched_rule",
      "error_category": "schema_error",
      "evidence_persistence": "complete"
    }"#;
    let result: BenchmarkResult = serde_json::from_str(json).expect("parse classified failure");
    assert_eq!(
        result.root_cause,
        Some(RootCauseClass::SchemaGraphValidation)
    );
    assert_eq!(result.error_category, Some(ErrorCategory::SchemaError));
}

#[test]
fn benchmark_cli_help_exposes_evidence_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .args(["benchmark", "--help"])
        .output()
        .expect("duumbi benchmark --help should run");
    assert!(
        output.status.success(),
        "help command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--artifact-dir"));
    assert!(stdout.contains("--keep-workspaces"));
    assert!(stdout.contains("--capture-model-io"));
}
