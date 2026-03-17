//! Phase 9a-2 integration tests: ownership & lifetimes.
//!
//! Tests the kill criterion: the validator rejects use-after-move (E021),
//! double-borrow-mut (E022), and dangling reference (E026) at graph
//! validation time, before compilation.

use duumbi::errors::codes;
use duumbi::graph::builder::build_graph;
use duumbi::graph::validator::validate;
use duumbi::parser::parse_jsonld;

fn load_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/ownership/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

fn parse_and_validate(fixture_name: &str) -> Vec<duumbi::errors::Diagnostic> {
    let json = load_fixture(fixture_name);
    let module =
        parse_jsonld(&json).unwrap_or_else(|e| panic!("Parse failed for {fixture_name}: {e}"));
    let sg =
        build_graph(&module).unwrap_or_else(|e| panic!("Build failed for {fixture_name}: {e:?}"));
    validate(&sg)
}

// === Kill Criterion Tests ===

#[test]
fn kill_criterion_use_after_move_rejected() {
    let diags = parse_and_validate("use_after_move.jsonld");
    assert!(
        diags.iter().any(|d| d.code == codes::E021_USE_AFTER_MOVE),
        "Kill criterion: use-after-move must produce E021, got: {diags:?}"
    );
}

#[test]
fn kill_criterion_double_borrow_mut_rejected() {
    let diags = parse_and_validate("double_borrow_mut.jsonld");
    assert!(
        diags
            .iter()
            .any(|d| d.code == codes::E022_BORROW_EXCLUSIVITY),
        "Kill criterion: double-borrow-mut must produce E022, got: {diags:?}"
    );
}

#[test]
fn kill_criterion_dangling_ref_rejected() {
    let diags = parse_and_validate("dangling_ref.jsonld");
    assert!(
        diags
            .iter()
            .any(|d| d.code == codes::E026_DANGLING_REFERENCE),
        "Kill criterion: dangling reference must produce E026, got: {diags:?}"
    );
}

// === Additional Error Code Tests ===

#[test]
fn double_free_produces_e025() {
    let diags = parse_and_validate("double_free.jsonld");
    assert!(
        diags.iter().any(|d| d.code == codes::E025_DOUBLE_FREE),
        "Expected E025 double free, got: {diags:?}"
    );
}

#[test]
fn move_while_borrowed_produces_e027() {
    let diags = parse_and_validate("move_while_borrowed.jsonld");
    assert!(
        diags
            .iter()
            .any(|d| d.code == codes::E027_MOVE_WHILE_BORROWED),
        "Expected E027 move while borrowed, got: {diags:?}"
    );
}

// === Positive Tests ===

#[test]
fn valid_ownership_passes_validation() {
    let diags = parse_and_validate("valid_ownership.jsonld");
    assert!(
        diags.is_empty(),
        "Valid ownership graph should produce no errors, got: {diags:?}"
    );
}

// === Regression Tests ===

#[test]
fn phase0_add_fixture_still_passes() {
    let json = std::fs::read_to_string("tests/fixtures/add.jsonld").expect("add.jsonld must exist");
    let module = parse_jsonld(&json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let diags = validate(&sg);
    assert!(
        diags.is_empty(),
        "Phase 0 add(3,5) must still pass: {diags:?}"
    );
}

#[test]
fn phase1_fibonacci_fixture_still_passes() {
    let json = std::fs::read_to_string("tests/fixtures/fibonacci.jsonld")
        .expect("fibonacci.jsonld must exist");
    let module = parse_jsonld(&json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let diags = validate(&sg);
    assert!(
        diags.is_empty(),
        "Phase 1 fibonacci must still pass: {diags:?}"
    );
}

#[test]
fn phase9a1_string_concat_fixture_still_passes() {
    let json = std::fs::read_to_string("tests/fixtures/string_concat.jsonld")
        .expect("string_concat.jsonld must exist");
    let module = parse_jsonld(&json).expect("parse");
    let sg = build_graph(&module).expect("build");
    let diags = validate(&sg);
    assert!(
        diags.is_empty(),
        "Phase 9a-1 string_concat must still pass: {diags:?}"
    );
}
