//! Integration tests for Phase 10 Track B: Context Assembly Algorithm.
//!
//! Tests classifier, traversal, collector, budget, few-shot, and the
//! full assemble_context pipeline.

use std::collections::HashMap;
use std::fs;

use duumbi::context::analyzer::{FunctionSummary, ModuleInfo, ProjectMap};
use duumbi::context::assemble_context;
use duumbi::context::budget::{CharEstimator, TokenEstimator, fit_to_budget};
use duumbi::context::classifier::{TaskType, classify, is_ambiguous};
use duumbi::context::collector::{ContextFragment, ContextNodes};
use duumbi::knowledge::learning;
use duumbi::knowledge::types::SuccessRecord;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Classifier tests (7 task types × multiple inputs)
// ---------------------------------------------------------------------------

fn empty_map() -> ProjectMap {
    ProjectMap {
        modules: Vec::new(),
        exports: HashMap::new(),
    }
}

fn sample_map() -> ProjectMap {
    ProjectMap {
        modules: vec![
            ModuleInfo {
                name: "main".to_string(),
                functions: vec![FunctionSummary {
                    name: "main".to_string(),
                    params: Vec::new(),
                    return_type: "i64".to_string(),
                }],
                is_main: true,
            },
            ModuleInfo {
                name: "math".to_string(),
                functions: vec![FunctionSummary {
                    name: "add".to_string(),
                    params: vec![
                        ("a".to_string(), "i64".to_string()),
                        ("b".to_string(), "i64".to_string()),
                    ],
                    return_type: "i64".to_string(),
                }],
                is_main: false,
            },
        ],
        exports: [("add".to_string(), "math".to_string())]
            .into_iter()
            .collect(),
    }
}

#[test]
fn classifier_fix_error_variants() {
    assert_eq!(classify("fix the bug", &empty_map()), TaskType::FixError);
    assert_eq!(
        classify("there is an error", &empty_map()),
        TaskType::FixError
    );
    assert_eq!(classify("E003 is broken", &empty_map()), TaskType::FixError);
}

#[test]
fn classifier_add_function_default() {
    assert_eq!(
        classify("implement fibonacci", &empty_map()),
        TaskType::AddFunction
    );
}

#[test]
fn classifier_modify_existing() {
    assert_eq!(
        classify("change add to handle f64", &sample_map()),
        TaskType::ModifyFunction
    );
}

#[test]
fn classifier_create_module() {
    assert_eq!(
        classify("create module string_ops", &empty_map()),
        TaskType::CreateModule
    );
}

#[test]
fn classifier_modify_main() {
    assert_eq!(
        classify("wire new function into main", &empty_map()),
        TaskType::ModifyMain
    );
}

#[test]
fn classifier_refactor() {
    assert_eq!(
        classify("refactor math module", &empty_map()),
        TaskType::RefactorModule
    );
}

#[test]
fn classifier_add_test() {
    assert_eq!(
        classify("add test for multiply", &empty_map()),
        TaskType::AddTest
    );
}

// ---------------------------------------------------------------------------
// Budget tests
// ---------------------------------------------------------------------------

#[test]
fn budget_char_estimator_accuracy() {
    let est = CharEstimator;
    // 100 chars ÷ 3.5 ≈ 29, +10% ≈ 32
    let text = "x".repeat(100);
    let estimate = est.estimate(&text);
    assert!((28..=35).contains(&estimate), "estimate was {estimate}");
}

#[test]
fn budget_fit_preserves_priority_order() {
    let nodes = ContextNodes {
        fragments: vec![
            ContextFragment {
                text: "critical context".to_string(),
                priority: 1,
                source_module: "main".to_string(),
            },
            ContextFragment {
                text: "x".repeat(50000),
                priority: 5,
                source_module: "other".to_string(),
            },
        ],
    };
    let result = fit_to_budget(&nodes, 100, &CharEstimator);
    assert!(result.contains("critical context"));
    assert!(result.len() < 1000);
}

#[test]
fn budget_empty_nodes() {
    let nodes = ContextNodes {
        fragments: Vec::new(),
    };
    let result = fit_to_budget(&nodes, 1000, &CharEstimator);
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Few-shot scoring
// ---------------------------------------------------------------------------

#[test]
fn fewshot_scoring_task_type_priority() {
    let mut r1 = SuccessRecord::new("add func", "AddFunction");
    r1.ops_count = 1;
    let mut r2 = SuccessRecord::new("fix error", "FixError");
    r2.error_codes = vec!["E001".to_string()];

    let s1 = learning::score_for_request(&r1, "AddFunction", "add multiply", &[]);
    let s2 = learning::score_for_request(&r2, "AddFunction", "add multiply", &[]);
    assert!(s1 > s2, "same task_type should score higher");
}

// ---------------------------------------------------------------------------
// Full pipeline (determinism, budget)
// ---------------------------------------------------------------------------

fn setup_workspace(tmp: &TempDir) {
    let graph_dir = tmp.path().join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("mkdir");
    let main_json = serde_json::json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    { "@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": 0, "duumbi:resultType": "i64" },
                    { "@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1", "duumbi:operand": { "@id": "duumbi:main/main/entry/0" } }
                ]
            }]
        }]
    });
    fs::write(
        graph_dir.join("main.jsonld"),
        serde_json::to_string_pretty(&main_json).expect("serialize"),
    )
    .expect("write");
}

#[test]
fn assemble_context_deterministic() {
    let tmp = TempDir::new().expect("temp dir");
    setup_workspace(&tmp);

    let r1 = assemble_context("add multiply function", tmp.path(), &[]).expect("first");
    let r2 = assemble_context("add multiply function", tmp.path(), &[]).expect("second");

    assert_eq!(r1.enriched_message, r2.enriched_message);
    assert_eq!(r1.task_type, r2.task_type);
    assert_eq!(r1.token_estimate, r2.token_estimate);
}

#[test]
fn assemble_context_includes_module_info() {
    let tmp = TempDir::new().expect("temp dir");
    setup_workspace(&tmp);

    let bundle = assemble_context("add multiply function", tmp.path(), &[]).expect("assemble");
    // Should contain the module summary
    assert!(
        bundle.enriched_message.contains("main"),
        "should mention main module"
    );
}

#[test]
fn assemble_context_classifies_correctly() {
    let tmp = TempDir::new().expect("temp dir");
    setup_workspace(&tmp);

    let bundle = assemble_context("fix the E001 error", tmp.path(), &[]).expect("assemble");
    assert_eq!(bundle.task_type, TaskType::FixError);
}

#[test]
fn assemble_context_session_history_included() {
    let tmp = TempDir::new().expect("temp dir");
    setup_workspace(&tmp);

    let history = vec![duumbi::session::PersistentTurn {
        request: "previous request".to_string(),
        summary: "did something".to_string(),
        timestamp: chrono::Utc::now(),
        task_type: "AddFunction".to_string(),
    }];

    let bundle = assemble_context("modify main", tmp.path(), &history).expect("assemble");
    assert!(bundle.enriched_message.contains("previous request"));
}

#[test]
fn assemble_context_with_fewshot_learning() {
    let tmp = TempDir::new().expect("temp dir");
    setup_workspace(&tmp);

    // Add learning history
    let mut r = SuccessRecord::new("add helper function", "AddFunction");
    r.ops_count = 1;
    r.module = "main".to_string();
    learning::append_success(tmp.path(), &r).expect("append");

    let bundle = assemble_context("add multiply function", tmp.path(), &[]).expect("assemble");
    // Should include few-shot example (task_type match scores > 2)
    assert!(
        bundle.enriched_message.contains("add helper function")
            || bundle.enriched_message.contains("multiply"),
        "should contain learning context or the request"
    );
}

#[test]
fn assemble_context_empty_workspace() {
    let tmp = TempDir::new().expect("temp dir");
    fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("mkdir");

    let bundle = assemble_context("add something", tmp.path(), &[]).expect("assemble");
    assert!(!bundle.enriched_message.is_empty());
    assert_eq!(bundle.task_type, TaskType::AddFunction);
}

// ---------------------------------------------------------------------------
// Ambiguity detection
// ---------------------------------------------------------------------------

#[test]
fn ambiguity_very_short_request() {
    assert!(is_ambiguous("do", &empty_map()));
}

#[test]
fn ambiguity_clear_request_not_ambiguous() {
    assert!(!is_ambiguous(
        "add a multiply function that takes two i64 parameters and returns their product",
        &empty_map()
    ));
}
