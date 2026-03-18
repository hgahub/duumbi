//! Phase 9C integration tests — benchmark showcases, report, and runner.
//!
//! These tests validate the benchmark infrastructure without requiring
//! LLM API keys. They cover showcase parsing, report aggregation,
//! kill criterion logic, error categorization, JSON roundtrip, and
//! regression detection.

use duumbi::bench::report::{
    BenchmarkReport, BenchmarkResult, ErrorCategory, ProviderStats, ShowcaseSummary,
    categorize_error, check_kill_criterion, detect_regressions,
};
use duumbi::bench::showcases::{self, ALL_SHOWCASES, parse_showcase};

// ---------------------------------------------------------------------------
// Showcase YAML parsing
// ---------------------------------------------------------------------------

#[test]
fn all_six_showcases_parse_into_valid_intent_specs() {
    assert_eq!(ALL_SHOWCASES.len(), 6, "expected exactly 6 showcases");

    for showcase in ALL_SHOWCASES {
        let spec = parse_showcase(showcase)
            .unwrap_or_else(|e| panic!("showcase '{}' failed to parse: {e}", showcase.name));

        assert!(
            !spec.intent.is_empty(),
            "showcase '{}' has empty intent",
            showcase.name
        );
        assert!(
            !spec.test_cases.is_empty(),
            "showcase '{}' has no test cases",
            showcase.name
        );
        assert!(
            !spec.modules.create.is_empty() || !spec.modules.modify.is_empty(),
            "showcase '{}' has no module references",
            showcase.name
        );
    }
}

#[test]
fn calculator_showcase_has_four_test_cases() {
    let calc = ALL_SHOWCASES
        .iter()
        .find(|s| s.name == "calculator")
        .expect("calculator not found");
    let spec = parse_showcase(calc).expect("parse failed");
    assert_eq!(spec.test_cases.len(), 4);
    assert_eq!(spec.test_cases[0].function, "add");
    assert_eq!(spec.test_cases[0].expected_return, 8);
}

#[test]
fn fibonacci_showcase_has_three_test_cases() {
    let fib = ALL_SHOWCASES
        .iter()
        .find(|s| s.name == "fibonacci")
        .expect("fibonacci not found");
    let spec = parse_showcase(fib).expect("parse failed");
    assert_eq!(spec.test_cases.len(), 3);
    assert_eq!(spec.test_cases[2].expected_return, 55); // fib(10)
}

#[test]
fn sorting_showcase_parses_correctly() {
    let sort = ALL_SHOWCASES
        .iter()
        .find(|s| s.name == "sorting")
        .expect("sorting not found");
    let spec = parse_showcase(sort).expect("parse failed");
    assert_eq!(spec.test_cases.len(), 2);
    assert_eq!(spec.test_cases[0].function, "sort_and_get");
}

#[test]
fn state_machine_showcase_has_four_transitions() {
    let sm = ALL_SHOWCASES
        .iter()
        .find(|s| s.name == "state_machine")
        .expect("state_machine not found");
    let spec = parse_showcase(sm).expect("parse failed");
    assert_eq!(spec.test_cases.len(), 4);
}

#[test]
fn multi_module_showcase_creates_ops_module() {
    let mm = ALL_SHOWCASES
        .iter()
        .find(|s| s.name == "multi_module")
        .expect("multi_module not found");
    let spec = parse_showcase(mm).expect("parse failed");
    assert!(spec.modules.create.contains(&"math/ops".to_string()));
}

#[test]
fn string_ops_showcase_has_two_test_cases() {
    let so = ALL_SHOWCASES
        .iter()
        .find(|s| s.name == "string_ops")
        .expect("string_ops not found");
    let spec = parse_showcase(so).expect("parse failed");
    assert_eq!(spec.test_cases.len(), 2);
    assert_eq!(spec.test_cases[0].expected_return, 5);
    assert_eq!(spec.test_cases[1].expected_return, 1);
}

// ---------------------------------------------------------------------------
// Showcase filtering
// ---------------------------------------------------------------------------

#[test]
fn filter_showcases_returns_all_when_none() {
    let all = showcases::filter_showcases(None);
    assert_eq!(all.len(), 6);
}

#[test]
fn filter_showcases_returns_matching_subset() {
    let names = vec!["fibonacci".to_string(), "sorting".to_string()];
    let filtered = showcases::filter_showcases(Some(&names));
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].name, "fibonacci");
    assert_eq!(filtered[1].name, "sorting");
}

#[test]
fn filter_showcases_returns_empty_for_unknown_name() {
    let names = vec!["nonexistent".to_string()];
    let filtered = showcases::filter_showcases(Some(&names));
    assert!(filtered.is_empty());
}

// ---------------------------------------------------------------------------
// Error categorization
// ---------------------------------------------------------------------------

#[test]
fn categorize_error_schema() {
    assert_eq!(
        categorize_error("E009 schema invalid: missing field"),
        ErrorCategory::SchemaError
    );
    assert_eq!(
        categorize_error("Schema validation failed"),
        ErrorCategory::SchemaError
    );
}

#[test]
fn categorize_error_type() {
    assert_eq!(
        categorize_error("E001 Type mismatch: expected i64 but got f64"),
        ErrorCategory::TypeError
    );
    assert_eq!(
        categorize_error("E002 unknown Op: Foo"),
        ErrorCategory::TypeError
    );
    assert_eq!(
        categorize_error("E003 missing field duumbi:left"),
        ErrorCategory::TypeError
    );
}

#[test]
fn categorize_error_provider() {
    assert_eq!(
        categorize_error("rate limit exceeded (429)"),
        ErrorCategory::ProviderError
    );
    assert_eq!(
        categorize_error("401 Unauthorized"),
        ErrorCategory::ProviderError
    );
    assert_eq!(
        categorize_error("403 forbidden"),
        ErrorCategory::ProviderError
    );
    assert_eq!(
        categorize_error("invalid API key"),
        ErrorCategory::ProviderError
    );
    assert_eq!(
        categorize_error("request timeout after 30s"),
        ErrorCategory::ProviderError
    );
}

#[test]
fn categorize_error_mutation() {
    assert_eq!(
        categorize_error("mutation failed after 3 retries"),
        ErrorCategory::MutationFailed
    );
    assert_eq!(
        categorize_error("patch rejected: invalid node"),
        ErrorCategory::MutationFailed
    );
    assert_eq!(
        categorize_error("no tool_use response from LLM"),
        ErrorCategory::MutationFailed
    );
}

#[test]
fn categorize_error_crash() {
    assert_eq!(
        categorize_error("link failed: undefined symbol _duumbi_print"),
        ErrorCategory::Crash
    );
    assert_eq!(
        categorize_error("cranelift codegen error"),
        ErrorCategory::Crash
    );
    assert_eq!(
        categorize_error("compile error: invalid function signature"),
        ErrorCategory::Crash
    );
}

#[test]
fn categorize_error_logic_default() {
    assert_eq!(
        categorize_error("expected 8 but got 7"),
        ErrorCategory::LogicError
    );
    assert_eq!(
        categorize_error("some unknown error message"),
        ErrorCategory::LogicError
    );
}

// ---------------------------------------------------------------------------
// Report aggregation
// ---------------------------------------------------------------------------

fn make_result(showcase: &str, provider: &str, attempt: u32, success: bool) -> BenchmarkResult {
    BenchmarkResult {
        showcase: showcase.to_string(),
        provider: provider.to_string(),
        attempt,
        success,
        error_category: if success {
            None
        } else {
            Some(ErrorCategory::LogicError)
        },
        error_message: if success {
            None
        } else {
            Some("test failed".to_string())
        },
        tests_passed: if success { 4 } else { 2 },
        tests_total: 4,
        duration_secs: 3.5,
    }
}

#[test]
fn report_from_results_aggregates_correctly() {
    let results = vec![
        make_result("calculator", "anthropic", 1, true),
        make_result("calculator", "anthropic", 2, true),
        make_result("calculator", "openai", 1, true),
        make_result("calculator", "openai", 2, false),
    ];

    let report = BenchmarkReport::from_results(
        results,
        2,
        "2026-03-18T00:00:00Z".to_string(),
        "2026-03-18T01:00:00Z".to_string(),
    );

    assert_eq!(report.showcases.len(), 1);
    let calc = &report.showcases[0];
    assert_eq!(calc.name, "calculator");
    assert_eq!(calc.total_attempts, 4);
    assert_eq!(calc.successes, 3);

    // Anthropic: 2/2 = 100%
    let anthropic = calc
        .providers
        .iter()
        .find(|p| p.name == "anthropic")
        .expect("anthropic");
    assert_eq!(anthropic.successes, 2);
    assert!((anthropic.success_rate - 1.0).abs() < f64::EPSILON);

    // OpenAI: 1/2 = 50%
    let openai = calc
        .providers
        .iter()
        .find(|p| p.name == "openai")
        .expect("openai");
    assert_eq!(openai.successes, 1);
    assert!((openai.success_rate - 0.5).abs() < f64::EPSILON);
}

#[test]
fn report_json_roundtrip() {
    let results = vec![
        make_result("fibonacci", "anthropic", 1, true),
        make_result("fibonacci", "openai", 1, false),
    ];

    let report = BenchmarkReport::from_results(
        results,
        1,
        "2026-03-18T00:00:00Z".to_string(),
        "2026-03-18T00:05:00Z".to_string(),
    );

    let json = report.to_json().expect("serialization failed");
    let parsed: BenchmarkReport = serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(parsed.showcases.len(), 1);
    assert_eq!(parsed.results.len(), 2);
    assert_eq!(parsed.attempts_per_run, 1);
    assert!(!parsed.duumbi_version.is_empty());
}

// ---------------------------------------------------------------------------
// Kill criterion
// ---------------------------------------------------------------------------

#[test]
fn kill_criterion_met_5_of_6_with_2_providers() {
    let passing_showcases = [
        "calculator",
        "fibonacci",
        "sorting",
        "state_machine",
        "multi_module",
    ];
    let mut summaries: Vec<ShowcaseSummary> = passing_showcases
        .iter()
        .map(|name| ShowcaseSummary {
            name: name.to_string(),
            total_attempts: 40,
            successes: 40,
            success_rate: 1.0,
            providers: vec![
                ProviderStats {
                    name: "anthropic".to_string(),
                    attempts: 20,
                    successes: 20,
                    success_rate: 1.0,
                    error_categories: Default::default(),
                },
                ProviderStats {
                    name: "openai".to_string(),
                    attempts: 20,
                    successes: 20,
                    success_rate: 1.0,
                    error_categories: Default::default(),
                },
            ],
        })
        .collect();

    // 6th showcase fails
    summaries.push(ShowcaseSummary {
        name: "string_ops".to_string(),
        total_attempts: 40,
        successes: 10,
        success_rate: 0.25,
        providers: vec![
            ProviderStats {
                name: "anthropic".to_string(),
                attempts: 20,
                successes: 5,
                success_rate: 0.25,
                error_categories: Default::default(),
            },
            ProviderStats {
                name: "openai".to_string(),
                attempts: 20,
                successes: 5,
                success_rate: 0.25,
                error_categories: Default::default(),
            },
        ],
    });

    assert!(check_kill_criterion(&summaries));
}

#[test]
fn kill_criterion_not_met_only_4_passing() {
    let passing_showcases = ["calculator", "fibonacci", "sorting", "state_machine"];
    let mut summaries: Vec<ShowcaseSummary> = passing_showcases
        .iter()
        .map(|name| ShowcaseSummary {
            name: name.to_string(),
            total_attempts: 40,
            successes: 40,
            success_rate: 1.0,
            providers: vec![
                ProviderStats {
                    name: "anthropic".to_string(),
                    attempts: 20,
                    successes: 20,
                    success_rate: 1.0,
                    error_categories: Default::default(),
                },
                ProviderStats {
                    name: "openai".to_string(),
                    attempts: 20,
                    successes: 20,
                    success_rate: 1.0,
                    error_categories: Default::default(),
                },
            ],
        })
        .collect();

    // 2 showcases fail
    for name in &["multi_module", "string_ops"] {
        summaries.push(ShowcaseSummary {
            name: name.to_string(),
            total_attempts: 40,
            successes: 0,
            success_rate: 0.0,
            providers: vec![
                ProviderStats {
                    name: "anthropic".to_string(),
                    attempts: 20,
                    successes: 0,
                    success_rate: 0.0,
                    error_categories: Default::default(),
                },
                ProviderStats {
                    name: "openai".to_string(),
                    attempts: 20,
                    successes: 0,
                    success_rate: 0.0,
                    error_categories: Default::default(),
                },
            ],
        });
    }

    assert!(!check_kill_criterion(&summaries));
}

#[test]
fn kill_criterion_not_met_only_one_provider() {
    // 6 showcases pass with only 1 provider — need 2+
    let summaries: Vec<ShowcaseSummary> = ALL_SHOWCASES
        .iter()
        .map(|s| ShowcaseSummary {
            name: s.name.to_string(),
            total_attempts: 20,
            successes: 20,
            success_rate: 1.0,
            providers: vec![ProviderStats {
                name: "anthropic".to_string(),
                attempts: 20,
                successes: 20,
                success_rate: 1.0,
                error_categories: Default::default(),
            }],
        })
        .collect();

    assert!(!check_kill_criterion(&summaries));
}

// ---------------------------------------------------------------------------
// Regression detection
// ---------------------------------------------------------------------------

#[test]
fn regression_detected_when_rate_drops() {
    let baseline = BenchmarkReport::from_results(
        vec![
            make_result("calculator", "anthropic", 1, true),
            make_result("calculator", "anthropic", 2, true),
        ],
        2,
        "2026-03-17T00:00:00Z".to_string(),
        "2026-03-17T01:00:00Z".to_string(),
    );

    let current = BenchmarkReport::from_results(
        vec![
            make_result("calculator", "anthropic", 1, true),
            make_result("calculator", "anthropic", 2, false),
        ],
        2,
        "2026-03-18T00:00:00Z".to_string(),
        "2026-03-18T01:00:00Z".to_string(),
    );

    let regressions = detect_regressions(&current, &baseline, 0.05);
    assert_eq!(regressions.len(), 1);
    assert_eq!(regressions[0].showcase, "calculator");
    assert_eq!(regressions[0].provider, "anthropic");
    assert!((regressions[0].baseline_rate - 1.0).abs() < f64::EPSILON);
    assert!((regressions[0].current_rate - 0.5).abs() < f64::EPSILON);
    assert!((regressions[0].drop - 0.5).abs() < f64::EPSILON);
}

#[test]
fn no_regression_when_rate_stable() {
    let results = vec![
        make_result("calculator", "anthropic", 1, true),
        make_result("calculator", "anthropic", 2, false),
    ];

    let baseline = BenchmarkReport::from_results(
        results.clone(),
        2,
        "2026-03-17T00:00:00Z".to_string(),
        "2026-03-17T01:00:00Z".to_string(),
    );
    let current = BenchmarkReport::from_results(
        results,
        2,
        "2026-03-18T00:00:00Z".to_string(),
        "2026-03-18T01:00:00Z".to_string(),
    );

    let regressions = detect_regressions(&current, &baseline, 0.05);
    assert!(regressions.is_empty());
}

#[test]
fn no_regression_when_rate_improves() {
    let baseline = BenchmarkReport::from_results(
        vec![
            make_result("calculator", "anthropic", 1, true),
            make_result("calculator", "anthropic", 2, false),
        ],
        2,
        "2026-03-17T00:00:00Z".to_string(),
        "2026-03-17T01:00:00Z".to_string(),
    );

    let current = BenchmarkReport::from_results(
        vec![
            make_result("calculator", "anthropic", 1, true),
            make_result("calculator", "anthropic", 2, true),
        ],
        2,
        "2026-03-18T00:00:00Z".to_string(),
        "2026-03-18T01:00:00Z".to_string(),
    );

    let regressions = detect_regressions(&current, &baseline, 0.05);
    assert!(regressions.is_empty());
}

// ---------------------------------------------------------------------------
// Error category serialization
// ---------------------------------------------------------------------------

#[test]
fn error_category_serializes_as_snake_case() {
    let json = serde_json::to_string(&ErrorCategory::SchemaError).expect("serialize");
    assert_eq!(json, "\"schema_error\"");

    let json = serde_json::to_string(&ErrorCategory::MutationFailed).expect("serialize");
    assert_eq!(json, "\"mutation_failed\"");
}

#[test]
fn error_category_roundtrip() {
    for cat in [
        ErrorCategory::SchemaError,
        ErrorCategory::TypeError,
        ErrorCategory::LogicError,
        ErrorCategory::Crash,
        ErrorCategory::ProviderError,
        ErrorCategory::MutationFailed,
    ] {
        let json = serde_json::to_string(&cat).expect("serialize");
        let parsed: ErrorCategory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, cat);
    }
}
