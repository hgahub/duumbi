//! Phase 5 (M5) integration tests — Intent-Driven Development.
//!
//! Tests cover:
//! 1. Intent spec lifecycle: create → save → load → list → archive
//! 2. Coordinator task decomposition ordering
//! 3. Verifier: real compile+run of a known function via wrapper main
//! 4. M5 kill criterion: verifier passes `double(21) = 42` using the
//!    multi_module fixture (workspace with a real compiled module)

use std::fs;
use std::path::Path;

use duumbi::intent::coordinator;
use duumbi::intent::spec::{
    ExecutionMeta, IntentModules, IntentSpec, IntentStatus, TaskKind, TestCase,
};
use duumbi::intent::verifier;
use duumbi::intent::{
    IntentError, history_dir, intent_path, list_intents, load_intent, save_intent,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn minimal_spec(intent: &str) -> IntentSpec {
    IntentSpec {
        intent: intent.to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules::default(),
        test_cases: vec![],
        dependencies: vec![],
        context: None,
        created_at: Some("2026-01-01T00:00:00Z".to_string()),
        execution: None,
    }
}

/// Copies the multi_module fixture into a temporary workspace with the
/// expected `.duumbi/graph/` layout.
fn setup_multi_module_workspace() -> tempfile::TempDir {
    let ws = tempfile::TempDir::new().expect("invariant: tempdir");
    let graph_dir = ws.path().join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).expect("invariant: create graph dir");

    let fixture_dir = Path::new("tests/fixtures/multi_module");
    for name in &["main.jsonld", "math.jsonld"] {
        fs::copy(fixture_dir.join(name), graph_dir.join(name)).expect("invariant: copy fixture");
    }
    ws
}

// ---------------------------------------------------------------------------
// Intent lifecycle tests (#87)
// ---------------------------------------------------------------------------

#[test]
fn phase5_intent_save_load_roundtrip() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    let spec = minimal_spec("Add calculator module");

    save_intent(ws.path(), "calc", &spec).expect("save");
    let loaded = load_intent(ws.path(), "calc").expect("load");
    assert_eq!(loaded.intent, "Add calculator module");
    assert_eq!(loaded.status, IntentStatus::Pending);
}

#[test]
fn phase5_intent_list_returns_sorted_slugs() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    save_intent(ws.path(), "bravo", &minimal_spec("B")).expect("save");
    save_intent(ws.path(), "alpha", &minimal_spec("A")).expect("save");

    let slugs = list_intents(ws.path()).expect("list");
    assert_eq!(slugs, vec!["alpha", "bravo"]);
}

#[test]
fn phase5_intent_not_found_returns_error() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    let err = load_intent(ws.path(), "missing").expect_err("must error");
    assert!(matches!(err, IntentError::NotFound { .. }));
}

#[test]
fn phase5_intent_archive_moves_to_history() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    save_intent(ws.path(), "my-intent", &minimal_spec("Do something")).expect("save");

    let meta = ExecutionMeta {
        completed_at: "2026-01-02T00:00:00Z".to_string(),
        tasks_completed: 1,
        tests_passed: 1,
        tests_total: 1,
    };
    duumbi::intent::status::archive_intent(ws.path(), "my-intent", meta).expect("archive");

    // Active file removed
    assert!(!intent_path(ws.path(), "my-intent").exists());
    // History file present
    assert!(history_dir(ws.path()).join("my-intent.yaml").exists());
}

#[test]
fn phase5_intent_archive_sets_completed_status() {
    let ws = tempfile::TempDir::new().expect("tempdir");
    save_intent(ws.path(), "intent-a", &minimal_spec("Test")).expect("save");

    duumbi::intent::status::archive_intent(
        ws.path(),
        "intent-a",
        ExecutionMeta {
            completed_at: "2026-01-01T00:00:00Z".to_string(),
            tasks_completed: 2,
            tests_passed: 3,
            tests_total: 3,
        },
    )
    .expect("archive");

    let hist_path = history_dir(ws.path()).join("intent-a.yaml");
    let contents = fs::read_to_string(&hist_path).expect("read history");
    let archived: IntentSpec = serde_yaml::from_str(&contents).expect("parse");
    assert_eq!(archived.status, IntentStatus::Completed);
    assert!(archived.execution.is_some());
    assert_eq!(archived.execution.unwrap().tasks_completed, 2);
}

// ---------------------------------------------------------------------------
// Coordinator tests (#87)
// ---------------------------------------------------------------------------

#[test]
fn phase5_coordinator_creates_before_modifies() {
    let spec = IntentSpec {
        intent: "Build calculator".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules {
            create: vec!["calculator/ops".to_string()],
            modify: vec!["calculator/ops".to_string(), "app/main".to_string()],
        },
        test_cases: vec![],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let tasks = coordinator::decompose(&spec);
    assert!(!tasks.is_empty());

    // CreateModule tasks must come before AddFunction/ModifyMain tasks
    let first_non_create = tasks
        .iter()
        .position(|t| !matches!(t.kind, TaskKind::CreateModule { .. }));
    let last_create = tasks
        .iter()
        .rposition(|t| matches!(t.kind, TaskKind::CreateModule { .. }));

    if let (Some(first_non), Some(last_c)) = (first_non_create, last_create) {
        assert!(
            last_c < first_non,
            "All CreateModule tasks must come before AddFunction/ModifyMain"
        );
    }
}

#[test]
fn phase5_coordinator_modifymain_is_last() {
    let spec = IntentSpec {
        intent: "Build calculator".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules {
            create: vec!["calculator/ops".to_string()],
            modify: vec!["app/main".to_string()],
        },
        test_cases: vec![],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let tasks = coordinator::decompose(&spec);
    assert!(!tasks.is_empty());

    let last = tasks.last().expect("invariant: at least one task");
    assert!(
        matches!(last.kind, TaskKind::ModifyMain { .. }),
        "ModifyMain should be the last task, got: {:?}",
        last.kind
    );
}

#[test]
fn phase5_coordinator_task_ids_are_sequential() {
    let spec = IntentSpec {
        intent: "Test".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules {
            create: vec!["mod/a".to_string(), "mod/b".to_string()],
            modify: vec!["app/main".to_string()],
        },
        test_cases: vec![],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let tasks = coordinator::decompose(&spec);
    for (i, task) in tasks.iter().enumerate() {
        assert_eq!(task.id, i + 1, "Task IDs must be 1-indexed sequential");
    }
}

// ---------------------------------------------------------------------------
// Verifier tests (#87) — actual compile+run using the multi_module fixture
// ---------------------------------------------------------------------------

#[test]
fn phase5_verifier_double_21_returns_42() {
    let ws = setup_multi_module_workspace();

    let spec = IntentSpec {
        intent: "Verify double function".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec!["double(x) returns x + x".to_string()],
        modules: IntentModules::default(),
        test_cases: vec![TestCase {
            name: "double_21".to_string(),
            function: "double".to_string(),
            args: vec![21],
            expected_return: 42,
        }],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let report = verifier::run_tests(&spec, ws.path());
    assert!(
        report.all_passed(),
        "double(21) should return 42; report: {}",
        report.display()
    );
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 0);
}

#[test]
fn phase5_verifier_double_0_returns_0() {
    let ws = setup_multi_module_workspace();

    let spec = IntentSpec {
        intent: "Verify double(0)".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules::default(),
        test_cases: vec![TestCase {
            name: "double_zero".to_string(),
            function: "double".to_string(),
            args: vec![0],
            expected_return: 0,
        }],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let report = verifier::run_tests(&spec, ws.path());
    assert!(
        report.all_passed(),
        "double(0) should return 0: {}",
        report.display()
    );
}

#[test]
fn phase5_verifier_wrong_expected_fails() {
    let ws = setup_multi_module_workspace();

    let spec = IntentSpec {
        intent: "Wrong expectation".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules::default(),
        test_cases: vec![TestCase {
            name: "double_wrong".to_string(),
            function: "double".to_string(),
            args: vec![5],
            expected_return: 99, // wrong: double(5) = 10
        }],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let report = verifier::run_tests(&spec, ws.path());
    assert!(!report.all_passed(), "Should fail: double(5) != 99");
    assert_eq!(report.failed, 1);
}

#[test]
fn phase5_verifier_empty_test_cases_passes() {
    let ws = setup_multi_module_workspace();

    let spec = IntentSpec {
        intent: "No tests".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec![],
        modules: IntentModules::default(),
        test_cases: vec![],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let report = verifier::run_tests(&spec, ws.path());
    assert!(report.all_passed());
    assert_eq!(report.passed, 0);
    assert_eq!(report.failed, 0);
}

// ---------------------------------------------------------------------------
// M5 kill criterion (#87)
// ---------------------------------------------------------------------------

/// M5 kill criterion: the verifier correctly validates a real compiled function.
///
/// Given a workspace with the `double` module:
///   double(21) == 42  ✓
///   double(0)  == 0   ✓
/// The verifier must report 2/2 passed.
#[test]
fn phase5_m5_kill_criterion_verifier_validates_real_function() {
    let ws = setup_multi_module_workspace();

    let spec = IntentSpec {
        intent: "Verify double function correctness".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec!["double(x) returns x + x for any i64".to_string()],
        modules: IntentModules::default(),
        test_cases: vec![
            TestCase {
                name: "double_21_equals_42".to_string(),
                function: "double".to_string(),
                args: vec![21],
                expected_return: 42,
            },
            TestCase {
                name: "double_0_equals_0".to_string(),
                function: "double".to_string(),
                args: vec![0],
                expected_return: 0,
            },
        ],
        dependencies: vec![],
        context: None,
        created_at: None,
        execution: None,
    };

    let report = verifier::run_tests(&spec, ws.path());

    eprintln!("{}", report.display());
    assert!(
        report.all_passed(),
        "M5 kill criterion: verifier must pass all test cases"
    );
    assert_eq!(report.passed, 2, "Both test cases must pass");
    assert_eq!(report.failed, 0);
}
