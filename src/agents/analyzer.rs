//! Deterministic task analysis engine for intent specs.
//!
//! Scores an [`IntentSpec`] on four independent dimensions and produces a
//! [`TaskProfile`].  All logic is rule-based — no LLM calls, no I/O.

use serde::{Deserialize, Serialize};

use crate::intent::spec::IntentSpec;

// ---------------------------------------------------------------------------
// Profile types
// ---------------------------------------------------------------------------

/// Estimated complexity based on function and test-case counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Complexity {
    /// 0–1 functions / test cases.
    Simple,
    /// 2–5 functions / test cases.
    Moderate,
    /// 6 or more functions / test cases.
    Complex,
}

/// High-level task category derived from intent text and structural cues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// The intent creates new functions or modules.
    Create,
    /// The intent modifies existing code without restructuring.
    Modify,
    /// The intent adds or improves tests.
    Test,
    /// The intent reorganises code without changing behaviour.
    Refactor,
    /// The intent fixes a defect.
    Fix,
}

/// Module scope of the intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Touches at most one module.
    SingleModule,
    /// Touches two or more modules.
    MultiModule,
}

/// Risk level based on what the intent changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Risk {
    /// No elevated risk factors.
    Low,
    /// One elevated risk factor.
    Medium,
    /// Two or more elevated risk factors.
    High,
}

/// Complete task profile produced by [`analyze`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProfile {
    /// How complex the task is.
    pub complexity: Complexity,
    /// What kind of change the task represents.
    pub task_type: TaskType,
    /// How many modules are in scope.
    pub scope: Scope,
    /// How risky the change is.
    pub risk: Risk,
}

// ---------------------------------------------------------------------------
// Scoring functions
// ---------------------------------------------------------------------------

/// Score complexity from an intent spec.
///
/// The function count is approximated by the number of test cases (each test
/// typically exercises one function).  Thresholds: 0–1 → [`Complexity::Simple`],
/// 2–5 → [`Complexity::Moderate`], 6+ → [`Complexity::Complex`].
#[must_use]
pub fn score_complexity(spec: &IntentSpec) -> Complexity {
    let fn_count = spec.test_cases.len();
    tracing::debug!(fn_count, "scoring complexity");
    if fn_count >= 6 {
        Complexity::Complex
    } else if fn_count >= 2 {
        Complexity::Moderate
    } else {
        Complexity::Simple
    }
}

/// Score scope from module counts.
///
/// Counts `modules.create` + `modules.modify`.  0–1 → [`Scope::SingleModule`],
/// 2+ → [`Scope::MultiModule`].
#[must_use]
pub fn score_scope(spec: &IntentSpec) -> Scope {
    let total = spec.modules.create.len() + spec.modules.modify.len();
    tracing::debug!(total, "scoring scope");
    if total >= 2 {
        Scope::MultiModule
    } else {
        Scope::SingleModule
    }
}

/// Score task type from intent text and structural cues.
///
/// Keyword precedence (highest → lowest):
/// 1. `"fix"` / `"bug"` / `"error"` → [`TaskType::Fix`]
/// 2. `"refactor"` / `"rename"` / `"reorganize"` / `"reorganise"` → [`TaskType::Refactor`]
/// 3. `"test"` / `"verify"` → [`TaskType::Test`]
/// 4. `modules.create` is non-empty → [`TaskType::Create`]
/// 5. fallback → [`TaskType::Modify`]
#[must_use]
pub fn score_task_type(spec: &IntentSpec) -> TaskType {
    let lower = spec.intent.to_lowercase();
    tracing::debug!(intent = %spec.intent, "scoring task type");

    let fix_keywords = ["fix", "bug", "error"];
    let refactor_keywords = ["refactor", "rename", "reorganize", "reorganise"];
    let test_keywords = ["test", "verify"];

    if fix_keywords.iter().any(|kw| lower.contains(kw)) {
        return TaskType::Fix;
    }
    if refactor_keywords.iter().any(|kw| lower.contains(kw)) {
        return TaskType::Refactor;
    }
    if test_keywords.iter().any(|kw| lower.contains(kw)) {
        return TaskType::Test;
    }
    if !spec.modules.create.is_empty() {
        return TaskType::Create;
    }
    TaskType::Modify
}

/// Score risk based on workspace analysis.
///
/// Risk flags:
/// - The intent touches the `"main"` module (modifies core entry point).
/// - The intent modifies exports (any `modules.modify` entry).
/// - The intent spans multiple modules.
///
/// 0 flags → [`Risk::Low`], 1 flag → [`Risk::Medium`], 2+ flags → [`Risk::High`].
#[must_use]
pub fn score_risk(spec: &IntentSpec) -> Risk {
    let touches_main = spec
        .modules
        .create
        .iter()
        .chain(spec.modules.modify.iter())
        .any(|m| m == "main" || m.ends_with("/main"));

    let modifies_exports = !spec.modules.modify.is_empty();

    let multi_module = (spec.modules.create.len() + spec.modules.modify.len()) >= 2;

    let flags = [touches_main, modifies_exports, multi_module]
        .iter()
        .filter(|&&f| f)
        .count();

    tracing::debug!(
        touches_main,
        modifies_exports,
        multi_module,
        flags,
        "scoring risk"
    );

    match flags {
        0 => Risk::Low,
        1 => Risk::Medium,
        _ => Risk::High,
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Analyse an intent spec and produce a [`TaskProfile`].
///
/// This is fully deterministic — no LLM calls, no I/O, no panics.
/// Each of the four dimension scores is computed independently from the
/// spec fields; the result reflects the actual spec content and may
/// produce any valid combination of dimension values.
#[must_use]
pub fn analyze(spec: &IntentSpec) -> TaskProfile {
    tracing::debug!(intent = %spec.intent, "analyzing intent spec");
    TaskProfile {
        complexity: score_complexity(spec),
        task_type: score_task_type(spec),
        scope: score_scope(spec),
        risk: score_risk(spec),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};

    fn make_spec(
        intent: &str,
        test_count: usize,
        create: Vec<&str>,
        modify: Vec<&str>,
    ) -> IntentSpec {
        IntentSpec {
            intent: intent.to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec![],
            modules: IntentModules {
                create: create.into_iter().map(String::from).collect(),
                modify: modify.into_iter().map(String::from).collect(),
            },
            test_cases: (0..test_count)
                .map(|i| TestCase {
                    name: format!("t{i}"),
                    function: "f".to_string(),
                    args: vec![],
                    expected_return: 0,
                })
                .collect(),
            dependencies: vec![],
            context: None,
            created_at: None,
            execution: None,
        }
    }

    // -----------------------------------------------------------------------
    // Complexity scoring
    // -----------------------------------------------------------------------

    #[test]
    fn simple_one_test_case() {
        let spec = make_spec("add two numbers", 1, vec!["math/ops"], vec![]);
        assert_eq!(score_complexity(&spec), Complexity::Simple);
    }

    #[test]
    fn simple_zero_test_cases() {
        let spec = make_spec("do something", 0, vec![], vec![]);
        assert_eq!(score_complexity(&spec), Complexity::Simple);
    }

    #[test]
    fn moderate_three_test_cases() {
        let spec = make_spec("build calculator", 3, vec!["calc"], vec![]);
        assert_eq!(score_complexity(&spec), Complexity::Moderate);
    }

    #[test]
    fn moderate_five_test_cases() {
        let spec = make_spec("build calculator", 5, vec!["calc"], vec![]);
        assert_eq!(score_complexity(&spec), Complexity::Moderate);
    }

    #[test]
    fn complex_six_test_cases() {
        let spec = make_spec("big feature", 6, vec!["a"], vec![]);
        assert_eq!(score_complexity(&spec), Complexity::Complex);
    }

    // -----------------------------------------------------------------------
    // Scope scoring
    // -----------------------------------------------------------------------

    #[test]
    fn single_module_one_create() {
        let spec = make_spec("create math", 1, vec!["math"], vec![]);
        assert_eq!(score_scope(&spec), Scope::SingleModule);
    }

    #[test]
    fn single_module_one_modify() {
        let spec = make_spec("update main", 1, vec![], vec!["main"]);
        assert_eq!(score_scope(&spec), Scope::SingleModule);
    }

    #[test]
    fn single_module_zero_modules() {
        let spec = make_spec("something", 1, vec![], vec![]);
        assert_eq!(score_scope(&spec), Scope::SingleModule);
    }

    #[test]
    fn multi_module_two_creates() {
        let spec = make_spec("build system", 3, vec!["a", "b"], vec![]);
        assert_eq!(score_scope(&spec), Scope::MultiModule);
    }

    #[test]
    fn multi_module_one_create_one_modify() {
        let spec = make_spec("extend", 2, vec!["new_mod"], vec!["main"]);
        assert_eq!(score_scope(&spec), Scope::MultiModule);
    }

    // -----------------------------------------------------------------------
    // TaskType scoring
    // -----------------------------------------------------------------------

    #[test]
    fn task_type_fix_keyword_fix() {
        let spec = make_spec("fix the off-by-one error", 1, vec![], vec!["main"]);
        assert_eq!(score_task_type(&spec), TaskType::Fix);
    }

    #[test]
    fn task_type_fix_keyword_bug() {
        let spec = make_spec("resolve the memory bug", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Fix);
    }

    #[test]
    fn task_type_fix_keyword_error() {
        let spec = make_spec("handle the parse error gracefully", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Fix);
    }

    #[test]
    fn task_type_refactor_keyword_refactor() {
        let spec = make_spec("refactor the parser module", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Refactor);
    }

    #[test]
    fn task_type_refactor_keyword_rename() {
        let spec = make_spec("rename all public functions", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Refactor);
    }

    #[test]
    fn task_type_test_keyword_test() {
        let spec = make_spec("test the add function", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Test);
    }

    #[test]
    fn task_type_test_keyword_verify() {
        let spec = make_spec("verify that division rounds down", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Test);
    }

    #[test]
    fn task_type_create_via_modules() {
        let spec = make_spec("implement sorting", 1, vec!["sort"], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Create);
    }

    #[test]
    fn task_type_modify_fallback() {
        let spec = make_spec("update the output format", 1, vec![], vec!["main"]);
        assert_eq!(score_task_type(&spec), TaskType::Modify);
    }

    // fix takes priority over refactor/test keywords
    #[test]
    fn task_type_fix_takes_priority_over_test() {
        let spec = make_spec("fix and test the parser", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Fix);
    }

    // -----------------------------------------------------------------------
    // Risk scoring
    // -----------------------------------------------------------------------

    #[test]
    fn risk_low_create_only_no_main() {
        let spec = make_spec("create math ops", 1, vec!["math/ops"], vec![]);
        assert_eq!(score_risk(&spec), Risk::Low);
    }

    #[test]
    fn risk_medium_modifies_one_module() {
        let spec = make_spec("update utils", 1, vec![], vec!["utils"]);
        assert_eq!(score_risk(&spec), Risk::Medium);
    }

    #[test]
    fn risk_high_touches_main() {
        let spec = make_spec("update main", 1, vec![], vec!["main"]);
        // touches_main=true, modifies_exports=true → 2 flags
        assert_eq!(score_risk(&spec), Risk::High);
    }

    #[test]
    fn risk_high_multi_module_with_modify() {
        let spec = make_spec("big refactor", 3, vec!["a"], vec!["b"]);
        // modifies_exports=true, multi_module=true → 2 flags
        assert_eq!(score_risk(&spec), Risk::High);
    }

    // -----------------------------------------------------------------------
    // Full profile assertions (lookup table rows)
    // -----------------------------------------------------------------------

    #[test]
    fn profile_simple_create_single_low() {
        let spec = make_spec("implement add function", 1, vec!["math"], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Simple);
        assert_eq!(profile.task_type, TaskType::Create);
        assert_eq!(profile.scope, Scope::SingleModule);
        assert_eq!(profile.risk, Risk::Low);
    }

    #[test]
    fn profile_simple_modify_single_low() {
        let spec = make_spec("update the output format", 1, vec![], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Simple);
        assert_eq!(profile.task_type, TaskType::Modify);
        assert_eq!(profile.scope, Scope::SingleModule);
        assert_eq!(profile.risk, Risk::Low);
    }

    #[test]
    fn profile_simple_test_any() {
        let spec = make_spec("test the add function", 1, vec![], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Simple);
        assert_eq!(profile.task_type, TaskType::Test);
    }

    #[test]
    fn profile_moderate_create_single() {
        let spec = make_spec("build a calculator", 3, vec!["calc"], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Moderate);
        assert_eq!(profile.task_type, TaskType::Create);
        assert_eq!(profile.scope, Scope::SingleModule);
    }

    #[test]
    fn profile_moderate_create_multi() {
        let spec = make_spec("build a system", 3, vec!["mod_a", "mod_b"], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Moderate);
        assert_eq!(profile.task_type, TaskType::Create);
        assert_eq!(profile.scope, Scope::MultiModule);
    }

    #[test]
    fn profile_moderate_modify_medium_risk() {
        let spec = make_spec("extend the parser", 3, vec![], vec!["parser"]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Moderate);
        assert_eq!(profile.task_type, TaskType::Modify);
        assert!(
            profile.risk == Risk::Medium || profile.risk == Risk::High,
            "expected Medium or High risk, got {:?}",
            profile.risk
        );
    }

    #[test]
    fn profile_complex_multi() {
        let spec = make_spec("overhaul the engine", 6, vec!["a", "b"], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.complexity, Complexity::Complex);
        assert_eq!(profile.scope, Scope::MultiModule);
    }

    #[test]
    fn profile_refactor_keyword() {
        let spec = make_spec("refactor the graph module", 2, vec![], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.task_type, TaskType::Refactor);
    }

    #[test]
    fn profile_fix_keyword() {
        let spec = make_spec("fix the off-by-one bug", 1, vec![], vec![]);
        let profile = analyze(&spec);
        assert_eq!(profile.task_type, TaskType::Fix);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn empty_spec_returns_safe_default() {
        let spec = make_spec("", 0, vec![], vec![]);
        let profile = analyze(&spec);
        // With no test cases and no modules: simple+modify+single+low
        assert_eq!(profile.complexity, Complexity::Simple);
        assert_eq!(profile.scope, Scope::SingleModule);
        assert_eq!(profile.risk, Risk::Low);
        // Fallback task type when no keywords and no creates
        assert_eq!(profile.task_type, TaskType::Modify);
    }

    #[test]
    fn zero_modules_is_single_scope() {
        let spec = make_spec("do something", 0, vec![], vec![]);
        assert_eq!(score_scope(&spec), Scope::SingleModule);
    }

    #[test]
    fn error_context_in_intent_triggers_fix() {
        // "error" is a fix keyword
        let spec = make_spec("address the error in streaming", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Fix);
    }

    #[test]
    fn case_insensitive_keyword_matching() {
        let spec = make_spec("FIX the critical BUG", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Fix);
    }

    #[test]
    fn reorganise_british_spelling_is_refactor() {
        let spec = make_spec("reorganise the module layout", 1, vec![], vec![]);
        assert_eq!(score_task_type(&spec), TaskType::Refactor);
    }
}
