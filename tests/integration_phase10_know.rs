//! Integration tests for Phase 10 Track A: Knowledge Graph Foundation.
//!
//! Tests knowledge store CRUD, learning JSONL logger, scoring, and JSON-LD roundtrips.

use duumbi::knowledge::learning;
use duumbi::knowledge::store::KnowledgeStore;
use duumbi::knowledge::types::{
    DecisionRecord, FailureRecord, KnowledgeNode, PatternRecord, SuccessRecord, TYPE_DECISION,
    TYPE_FAILURE, TYPE_PATTERN, TYPE_SUCCESS,
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Store CRUD
// ---------------------------------------------------------------------------

#[test]
fn store_save_load_all_types() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");

    let s = SuccessRecord::new("add multiply", "AddFunction");
    let f = FailureRecord::new("add multiply", "AddFunction", "timeout");
    let d = DecisionRecord::new("use separate modules");
    let p = PatternRecord::new("add-and-wire", "Add function then wire");

    store
        .save_node(&KnowledgeNode::Success(s))
        .expect("save success");
    store
        .save_node(&KnowledgeNode::Failure(f))
        .expect("save failure");
    store
        .save_node(&KnowledgeNode::Decision(d))
        .expect("save decision");
    store
        .save_node(&KnowledgeNode::Pattern(p))
        .expect("save pattern");

    let all = store.load_all();
    assert_eq!(all.len(), 4);

    let stats = store.stats();
    assert_eq!(stats.successes, 1);
    assert_eq!(stats.failures, 1);
    assert_eq!(stats.decisions, 1);
    assert_eq!(stats.patterns, 1);
    assert_eq!(stats.total(), 4);
}

#[test]
fn store_query_by_type_filters_correctly() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");

    for i in 0..3 {
        store
            .save_node(&KnowledgeNode::Success(SuccessRecord::new(
                format!("req {i}"),
                "AddFunction",
            )))
            .expect("save");
    }
    store
        .save_node(&KnowledgeNode::Decision(DecisionRecord::new("d")))
        .expect("save");

    let successes = store.query_by_type(TYPE_SUCCESS);
    assert_eq!(successes.len(), 3);
    let failures = store.query_by_type(TYPE_FAILURE);
    assert!(failures.is_empty());
    let decisions = store.query_by_type(TYPE_DECISION);
    assert_eq!(decisions.len(), 1);
    let patterns = store.query_by_type(TYPE_PATTERN);
    assert!(patterns.is_empty());
}

#[test]
fn store_query_by_tag() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");

    let mut d1 = DecisionRecord::new("decision 1");
    d1.tags = vec!["arch".to_string(), "modules".to_string()];
    let mut d2 = DecisionRecord::new("decision 2");
    d2.tags = vec!["performance".to_string()];

    store.save_node(&KnowledgeNode::Decision(d1)).expect("save");
    store.save_node(&KnowledgeNode::Decision(d2)).expect("save");

    let arch = store.query_by_tag("arch");
    assert_eq!(arch.len(), 1);
    let perf = store.query_by_tag("performance");
    assert_eq!(perf.len(), 1);
    let none = store.query_by_tag("nonexistent");
    assert!(none.is_empty());
}

#[test]
fn store_remove_and_verify() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");

    let node = KnowledgeNode::Success(SuccessRecord::new("r", "t"));
    let id = node.id().to_string();
    store.save_node(&node).expect("save");

    assert_eq!(store.stats().total(), 1);
    assert!(store.remove_node(&id).expect("remove"));
    assert_eq!(store.stats().total(), 0);
    assert!(!store.remove_node(&id).expect("remove again"));
}

#[test]
fn store_remove_failure_and_verify() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");

    let node = KnowledgeNode::Failure(FailureRecord::new("r", "t", "timeout"));
    let id = node.id().to_string();
    store.save_node(&node).expect("save");

    assert_eq!(store.stats().failures, 1);
    assert_eq!(store.stats().total(), 1);
    assert!(store.remove_node(&id).expect("remove"));
    assert_eq!(store.stats().failures, 0);
    assert_eq!(store.stats().total(), 0);
}

#[test]
fn store_overwrite_same_id() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");

    let mut r = SuccessRecord::new("original", "AddFunction");
    store
        .save_node(&KnowledgeNode::Success(r.clone()))
        .expect("save");

    r.request = "updated".to_string();
    store
        .save_node(&KnowledgeNode::Success(r))
        .expect("overwrite");

    // Same ID → same file → still 1 node
    assert_eq!(store.stats().successes, 1);
    let all = store.load_all();
    if let KnowledgeNode::Success(s) = &all[0] {
        assert_eq!(s.request, "updated");
    }
}

// ---------------------------------------------------------------------------
// Learning JSONL logger
// ---------------------------------------------------------------------------

#[test]
fn learning_append_and_query_roundtrip() {
    let tmp = TempDir::new().expect("temp dir");

    let mut r1 = SuccessRecord::new("add multiply", "AddFunction");
    r1.ops_count = 1;
    r1.module = "main".to_string();
    r1.functions = vec!["multiply".to_string()];
    learning::append_success(tmp.path(), &r1).expect("append");

    let mut r2 = SuccessRecord::new("fix E003 error", "FixError");
    r2.error_codes = vec!["E003".to_string()];
    r2.retry_count = 1;
    learning::append_success(tmp.path(), &r2).expect("append");

    let records = learning::query_successes(tmp.path(), 10);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].request, "add multiply");
    assert_eq!(records[1].error_codes, vec!["E003"]);
}

#[test]
fn learning_query_respects_limit() {
    let tmp = TempDir::new().expect("temp dir");
    for i in 0..10 {
        let r = SuccessRecord::new(format!("request {i}"), "AddFunction");
        learning::append_success(tmp.path(), &r).expect("append");
    }
    let records = learning::query_successes(tmp.path(), 3);
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].request, "request 7");
}

#[test]
fn learning_count() {
    let tmp = TempDir::new().expect("temp dir");
    assert_eq!(learning::success_count(tmp.path()), 0);

    for _ in 0..5 {
        learning::append_success(tmp.path(), &SuccessRecord::new("r", "t")).expect("append");
    }
    assert_eq!(learning::success_count(tmp.path()), 5);
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

#[test]
fn scoring_task_type_match() {
    let r = SuccessRecord::new("add func", "AddFunction");
    let score = learning::score_for_request(&r, "AddFunction", "add something", &[]);
    assert!(score >= 3, "task type match should score >= 3, got {score}");
}

#[test]
fn scoring_error_code_overlap() {
    let mut r = SuccessRecord::new("fix", "FixError");
    r.error_codes = vec!["E001".to_string(), "E003".to_string()];

    let score = learning::score_for_request(
        &r,
        "FixError",
        "fix error",
        &["E001".to_string(), "E005".to_string()],
    );
    // task_type +3, E001 match +5 = 8
    assert_eq!(score, 8);
}

#[test]
fn scoring_no_match_returns_zero() {
    let r = SuccessRecord::new("unrelated", "RefactorModule");
    let score = learning::score_for_request(&r, "AddFunction", "add foo", &[]);
    assert_eq!(score, 0);
}

#[test]
fn scoring_module_match() {
    let mut r = SuccessRecord::new("add to math", "AddFunction");
    r.module = "math".to_string();

    let score = learning::score_for_request(&r, "CreateModule", "create math module", &[]);
    assert!(score >= 2, "module match should score >= 2, got {score}");
}

// ---------------------------------------------------------------------------
// JSON-LD roundtrip via KnowledgeNode enum
// ---------------------------------------------------------------------------

#[test]
fn knowledge_node_json_roundtrip_all_types() {
    let nodes = vec![
        KnowledgeNode::Success(SuccessRecord::new("test", "AddFunction")),
        KnowledgeNode::Decision(DecisionRecord::new("use modules")),
        KnowledgeNode::Pattern(PatternRecord::new("pattern", "desc")),
    ];

    for node in &nodes {
        let json = serde_json::to_string(node).expect("serialize");
        let roundtripped: KnowledgeNode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(node.id(), roundtripped.id());
        assert_eq!(node.node_type(), roundtripped.node_type());
    }
}

#[test]
fn empty_store_graceful() {
    let tmp = TempDir::new().expect("temp dir");
    let store = KnowledgeStore::new(tmp.path()).expect("store");
    assert!(store.load_all().is_empty());
    assert_eq!(store.stats().total(), 0);
    assert!(store.query_by_type(TYPE_SUCCESS).is_empty());
    assert!(store.query_by_tag("anything").is_empty());
}
