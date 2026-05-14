//! Deterministic preflight report model for intent specifications.
//!
//! This module defines the provider-free report foundation used by create,
//! review, execute, workflow, REPL/TUI, and Studio surfaces to describe
//! `IntentSpec` readiness before graph mutation begins.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::spec::IntentSpec;
use serde_json::Value;

/// Minimum score for a passing preflight report when no errors are present.
pub const PASS_SCORE_THRESHOLD: u8 = 85;

/// Minimum score for a warning preflight report when no errors are present.
pub const WARN_SCORE_THRESHOLD: u8 = 60;

const ERROR_SCORE_PENALTY: u8 = 45;
const WARNING_SCORE_PENALTY: u8 = 20;
const DEFAULT_RENDER_LIMIT: usize = 5;
const SUPPORTED_VERSION: u32 = 1;

/// Readiness classification for an intent preflight report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentPreflightReadiness {
    /// The spec has no errors and scores high enough to proceed normally.
    Pass,
    /// The spec has no errors but has warnings or a moderate readiness score.
    Warn,
    /// The spec has an error or scores below the blocking threshold.
    Block,
}

impl IntentPreflightReadiness {
    /// Returns the uppercase label used in user-facing preflight summaries.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Block => "BLOCK",
        }
    }
}

/// Severity of a single preflight finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntentPreflightSeverity {
    /// Execution would be unsafe, ambiguous, or unverifiable.
    Error,
    /// Execution may work but has quality, coverage, or clarity risk.
    Warning,
    /// Advisory context that does not reduce readiness score.
    Info,
}

impl IntentPreflightSeverity {
    /// Returns the lowercase label used in report issue lines.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }

    fn score_penalty(self) -> u8 {
        match self {
            Self::Error => ERROR_SCORE_PENALTY,
            Self::Warning => WARNING_SCORE_PENALTY,
            Self::Info => 0,
        }
    }
}

/// A stable, actionable preflight issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentPreflightIssue {
    /// Stable issue code used by tests, reviewers, and future integrations.
    pub code: String,
    /// Severity of this finding.
    pub severity: IntentPreflightSeverity,
    /// Human-readable field path such as `intent` or `test_cases[0].function`.
    pub field_path: String,
    /// Human-readable issue description.
    pub message: String,
    /// Suggested user or agent action.
    pub suggested_fix: String,
}

impl IntentPreflightIssue {
    /// Creates a new preflight issue.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        severity: IntentPreflightSeverity,
        field_path: impl Into<String>,
        message: impl Into<String>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            field_path: field_path.into(),
            message: message.into(),
            suggested_fix: suggested_fix.into(),
        }
    }
}

/// A workspace graph module that may be reusable for an intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentReuseCandidate {
    /// Module name from `duumbi:name`.
    pub module_name: String,
    /// Source path relative to `.duumbi/graph` when known.
    pub relative_path: String,
    /// Exported or defined function names.
    pub functions: Vec<String>,
    /// Why this module is relevant to the intent.
    pub reason: String,
    /// Deterministic confidence label for user-facing display.
    pub confidence: String,
}

impl IntentReuseCandidate {
    /// Creates a new reuse candidate.
    #[must_use]
    pub fn new(
        module_name: impl Into<String>,
        relative_path: impl Into<String>,
        functions: Vec<String>,
        reason: impl Into<String>,
        confidence: impl Into<String>,
    ) -> Self {
        let mut functions = functions;
        functions.sort();
        functions.dedup();
        Self {
            module_name: module_name.into(),
            relative_path: relative_path.into(),
            functions,
            reason: reason.into(),
            confidence: confidence.into(),
        }
    }
}

/// Complete deterministic readiness report for an intent spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentPreflightReport {
    /// Final readiness state.
    pub readiness: IntentPreflightReadiness,
    /// Deterministic score from 0 to 100.
    pub score: u8,
    /// Highest issue severity, or `None` when there are no issues.
    pub highest_severity: Option<IntentPreflightSeverity>,
    /// Stable issue list.
    pub issues: Vec<IntentPreflightIssue>,
    /// Reusable workspace candidates.
    pub reuse_candidates: Vec<IntentReuseCandidate>,
    /// Advisory decomposition hints.
    pub decomposition_hints: Vec<String>,
}

/// Runs deterministic spec-only checks for an intent spec.
///
/// This function intentionally does not inspect workspace graph files.
#[must_use]
pub fn run_spec_checks(spec: &IntentSpec) -> IntentPreflightReport {
    let issues = collect_spec_issues(spec);
    let decomposition_hints = collect_decomposition_hints(spec);

    IntentPreflightReport::from_parts(issues, Vec::new(), decomposition_hints)
}

/// Runs deterministic preflight checks plus workspace-only reuse scanning.
///
/// Reuse scanning is limited to `.duumbi/graph/**/*.jsonld` under `workspace`.
/// Missing graph directories produce no candidates. Filesystem or JSON parse
/// failures become warning issues so spec-only checks remain available.
#[must_use]
pub fn run_preflight(spec: &IntentSpec, workspace: &Path) -> IntentPreflightReport {
    let mut issues = collect_spec_issues(spec);
    let reuse_candidates = collect_workspace_reuse_candidates(spec, workspace, &mut issues);
    let decomposition_hints = collect_decomposition_hints(spec);

    IntentPreflightReport::from_parts(issues, reuse_candidates, decomposition_hints)
}

fn collect_spec_issues(spec: &IntentSpec) -> Vec<IntentPreflightIssue> {
    let mut issues = Vec::new();

    check_intent_text(spec, &mut issues);
    check_version(spec, &mut issues);
    check_modules(spec, &mut issues);
    check_entrypoint(spec, &mut issues);
    check_acceptance_criteria(spec, &mut issues);
    check_test_cases(spec, &mut issues);
    check_acceptance_test_alignment(spec, &mut issues);
    check_type_capability_and_edge_cases(spec, &mut issues);

    issues
}

impl IntentPreflightReport {
    /// Builds a report from precomputed issues, reuse candidates, and hints.
    #[must_use]
    pub fn from_parts(
        mut issues: Vec<IntentPreflightIssue>,
        mut reuse_candidates: Vec<IntentReuseCandidate>,
        mut decomposition_hints: Vec<String>,
    ) -> Self {
        sort_issues(&mut issues);
        sort_reuse_candidates(&mut reuse_candidates);
        dedup_preserving_order(&mut decomposition_hints);

        let score = calculate_score(&issues);
        let highest_severity = highest_severity(&issues);
        let readiness = calculate_readiness(score, highest_severity);

        Self {
            readiness,
            score,
            highest_severity,
            issues,
            reuse_candidates,
            decomposition_hints,
        }
    }

    /// Returns `true` when this report blocks execution.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.readiness == IntentPreflightReadiness::Block
    }
}

fn check_intent_text(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    if spec.intent.trim().is_empty() {
        issues.push(IntentPreflightIssue::new(
            "E_EMPTY_INTENT",
            IntentPreflightSeverity::Error,
            "intent",
            "intent text is empty",
            "add a concise natural-language intent description",
        ));
    }
}

fn check_version(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    if spec.version != SUPPORTED_VERSION {
        issues.push(IntentPreflightIssue::new(
            "E_UNSUPPORTED_VERSION",
            IntentPreflightSeverity::Error,
            "version",
            format!("intent schema version {} is not supported", spec.version),
            format!("use version {SUPPORTED_VERSION}"),
        ));
    }
}

fn check_modules(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    let has_create = !spec.modules.create.is_empty();
    let has_modify = !spec.modules.modify.is_empty();
    if !has_create && !has_modify {
        issues.push(IntentPreflightIssue::new(
            "E_NO_MODULE_TARGETS",
            IntentPreflightSeverity::Error,
            "modules",
            "no create or modify module targets are defined",
            "add at least one module to modules.create or modules.modify",
        ));
    }

    check_empty_module_names("modules.create", &spec.modules.create, issues);
    check_empty_module_names("modules.modify", &spec.modules.modify, issues);
    check_duplicate_module_names("modules.create", &spec.modules.create, issues);
    check_duplicate_module_names("modules.modify", &spec.modules.modify, issues);
    check_create_modify_overlap(spec, issues);
}

fn check_empty_module_names(
    field_path: &str,
    modules: &[String],
    issues: &mut Vec<IntentPreflightIssue>,
) {
    for (idx, module) in modules.iter().enumerate() {
        if module.trim().is_empty() {
            issues.push(IntentPreflightIssue::new(
                "E_EMPTY_MODULE_NAME",
                IntentPreflightSeverity::Error,
                format!("{field_path}[{idx}]"),
                "module name is empty",
                "replace the empty module name with an explicit module path",
            ));
        }
    }
}

fn check_duplicate_module_names(
    field_path: &str,
    modules: &[String],
    issues: &mut Vec<IntentPreflightIssue>,
) {
    for module in duplicate_values(modules) {
        issues.push(IntentPreflightIssue::new(
            "W_DUPLICATE_MODULE_NAME",
            IntentPreflightSeverity::Warning,
            field_path,
            format!("module '{module}' appears more than once"),
            "remove duplicate module targets",
        ));
    }
}

fn check_create_modify_overlap(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    let create = normalized_non_empty_set(&spec.modules.create);
    let modify = normalized_non_empty_set(&spec.modules.modify);
    let mut overlap = create.intersection(&modify).cloned().collect::<Vec<_>>();
    overlap.sort();

    for module in overlap {
        issues.push(IntentPreflightIssue::new(
            "E_DUPLICATE_CREATE_MODIFY_TARGET",
            IntentPreflightSeverity::Error,
            "modules",
            format!("module '{module}' appears in both create and modify targets"),
            "choose either create or modify for this module",
        ));
    }
}

fn check_entrypoint(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    if !is_executable_graph_changing_intent(spec) {
        return;
    }

    let has_main_module = spec
        .modules
        .modify
        .iter()
        .chain(spec.modules.create.iter())
        .any(|module| matches!(module.trim(), "app/main" | "main"));
    let has_context_entrypoint = spec
        .context
        .as_ref()
        .and_then(|context| context.entrypoint.as_deref())
        .is_some_and(|entrypoint| !entrypoint.trim().is_empty());

    if !has_main_module && !has_context_entrypoint {
        issues.push(IntentPreflightIssue::new(
            "E_MISSING_ENTRYPOINT",
            IntentPreflightSeverity::Error,
            "modules.modify",
            "executable graph-changing intent has no app/main, main, or explicit context entrypoint",
            "add app/main to modules.modify or set context.entrypoint",
        ));
    }
}

fn check_acceptance_criteria(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    if spec
        .acceptance_criteria
        .iter()
        .all(|criterion| criterion.trim().is_empty())
    {
        issues.push(IntentPreflightIssue::new(
            "E_EMPTY_ACCEPTANCE_CRITERIA",
            IntentPreflightSeverity::Error,
            "acceptance_criteria",
            "acceptance criteria are empty",
            "add concrete acceptance criteria before execution",
        ));
    }
}

fn check_test_cases(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    if is_executable_graph_changing_intent(spec) && spec.test_cases.is_empty() {
        issues.push(IntentPreflightIssue::new(
            "E_NO_TEST_CASES",
            IntentPreflightSeverity::Error,
            "test_cases",
            "executable graph-changing intent has no test cases",
            "add callable function test cases with i64 arguments and expected returns",
        ));
    }

    for (idx, test_case) in spec.test_cases.iter().enumerate() {
        if test_case.function.trim() == "main" {
            issues.push(IntentPreflightIssue::new(
                "E_MAIN_TEST_CASE",
                IntentPreflightSeverity::Error,
                format!("test_cases[{idx}].function"),
                "test cases must target callable functions, not main",
                "replace the main test case with tests for exported functions",
            ));
        }
    }
}

fn check_acceptance_test_alignment(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    for name in duplicate_test_names(spec) {
        issues.push(IntentPreflightIssue::new(
            "W_DUPLICATE_TEST_NAME",
            IntentPreflightSeverity::Warning,
            "test_cases.name",
            format!("test name '{name}' appears more than once"),
            "give each test case a stable unique name",
        ));
    }

    let acceptance_functions = acceptance_function_names(&spec.acceptance_criteria);
    let test_functions = test_function_names(spec);
    if !acceptance_functions.is_empty() {
        for function in test_functions.difference(&acceptance_functions) {
            issues.push(IntentPreflightIssue::new(
                "W_TEST_FUNCTION_NOT_IN_ACCEPTANCE",
                IntentPreflightSeverity::Warning,
                "test_cases.function",
                format!(
                    "test function '{function}' is not inferably mentioned by acceptance criteria"
                ),
                "mention the tested callable in acceptance criteria or rename the test target",
            ));
        }

        for function in acceptance_functions.difference(&test_functions) {
            issues.push(IntentPreflightIssue::new(
                "W_ACCEPTANCE_FUNCTION_UNTESTED",
                IntentPreflightSeverity::Warning,
                "acceptance_criteria",
                format!("acceptance criteria mention function '{function}' without test coverage"),
                "add at least one test case for the acceptance criteria function",
            ));
        }
    }

    for (function, count) in test_function_counts(spec) {
        if count == 1 {
            issues.push(IntentPreflightIssue::new(
                "W_SINGLE_TEST_CASE_FOR_FUNCTION",
                IntentPreflightSeverity::Warning,
                "test_cases",
                format!("function '{function}' has only one test case"),
                "add at least one more test case for this callable function",
            ));
        }
    }
}

fn check_type_capability_and_edge_cases(spec: &IntentSpec, issues: &mut Vec<IntentPreflightIssue>) {
    let text = combined_spec_text(spec);

    if is_boolean_like(&text) && !spec.test_cases.is_empty() && !covers_boolean_returns(spec) {
        issues.push(IntentPreflightIssue::new(
            "W_BOOLEAN_RETURN_COVERAGE",
            IntentPreflightSeverity::Warning,
            "test_cases.expected_return",
            "boolean-like intent does not cover both 1 and 0 expected returns",
            "add tests for true as 1 and false as 0",
        ));
    }

    if mentions_division_or_modulo(&text) && !has_zero_denominator_edge_case(spec) {
        issues.push(IntentPreflightIssue::new(
            "W_DIVISION_MODULO_ZERO_EDGE",
            IntentPreflightSeverity::Warning,
            "test_cases",
            "division or modulo intent has no zero-denominator edge test",
            "add a denominator-zero test case or explicitly document the behavior",
        ));
    }

    if mentions_branching_or_comparison(&text) && !has_boundary_or_equal_case(spec) {
        issues.push(IntentPreflightIssue::new(
            "W_BRANCH_BOUNDARY_COVERAGE",
            IntentPreflightSeverity::Warning,
            "test_cases",
            "branching or comparison intent has no obvious equal or boundary test",
            "add equal, zero, or boundary-value test coverage where relevant",
        ));
    }

    let capabilities = not_directly_verifiable_capabilities(&text);
    if !capabilities.is_empty() {
        issues.push(IntentPreflightIssue::new(
            "W_NOT_DIRECTLY_VERIFIABLE_CAPABILITY",
            IntentPreflightSeverity::Warning,
            "acceptance_criteria",
            format!(
                "current i64 verifier tests cannot directly prove: {}",
                capabilities.join(", ")
            ),
            "keep i64 callable test cases and verify broader behavior through later manual or integration evidence",
        ));
    }
}

fn combined_spec_text(spec: &IntentSpec) -> String {
    let mut parts = Vec::with_capacity(2 + spec.acceptance_criteria.len());
    parts.push(spec.intent.as_str());
    parts.extend(
        spec.acceptance_criteria
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    if let Some(context) = &spec.context {
        if let Some(scope) = &context.scope {
            parts.push(scope);
        }
        if let Some(entrypoint) = &context.entrypoint {
            parts.push(entrypoint);
        }
        if let Some(surface) = &context.runtime_surface {
            parts.push(surface);
        }
        parts.extend(context.integration_points.iter().map(String::as_str));
        parts.extend(context.constraints.iter().map(String::as_str));
    }
    parts.join(" ").to_lowercase()
}

fn is_boolean_like(text: &str) -> bool {
    contains_any(
        text,
        &[
            "boolean",
            "true",
            "false",
            "is_",
            "is even",
            "is odd",
            "is prime",
            "is palindrome",
            "check if",
            "whether",
        ],
    )
}

fn covers_boolean_returns(spec: &IntentSpec) -> bool {
    let returns = spec
        .test_cases
        .iter()
        .map(|test_case| test_case.expected_return)
        .collect::<HashSet<_>>();
    returns.contains(&0) && returns.contains(&1)
}

fn mentions_division_or_modulo(text: &str) -> bool {
    contains_any(
        text,
        &[
            "division",
            "divide",
            "divides",
            "div(",
            " modulo",
            "modulo",
            "remainder",
            "mod(",
            "mod ",
        ],
    )
}

fn has_zero_denominator_edge_case(spec: &IntentSpec) -> bool {
    spec.test_cases
        .iter()
        .any(|test_case| test_case.args.iter().skip(1).any(|arg| *arg == 0))
}

fn mentions_branching_or_comparison(text: &str) -> bool {
    contains_any(
        text,
        &[
            "branch",
            "compare",
            "comparison",
            "less than",
            "greater than",
            "equal",
            "minimum",
            "maximum",
            "min(",
            "max(",
            "clamp",
            "if ",
        ],
    )
}

fn has_boundary_or_equal_case(spec: &IntentSpec) -> bool {
    spec.test_cases.iter().any(|test_case| {
        test_case.args.contains(&0)
            || test_case
                .args
                .windows(2)
                .any(|window| window[0] == window[1])
            || test_case.expected_return == 0
    })
}

fn not_directly_verifiable_capabilities(text: &str) -> Vec<&'static str> {
    let mut capabilities = Vec::new();
    if contains_any(text, &["string", "text", "substring", "palindrome"]) {
        capabilities.push("string behavior");
    }
    if contains_any(text, &["float", "f64", "decimal"]) {
        capabilities.push("floating-point behavior");
    }
    if contains_any(text, &["array", "list", "collection", "empty input"]) {
        capabilities.push("collection behavior");
    }
    if contains_any(text, &["option", "some", "none", "result", "ok", "err"]) {
        capabilities.push("option/result behavior");
    }
    if contains_any(text, &["ownership", "borrow", "lifetime", "move"]) {
        capabilities.push("ownership behavior");
    }
    if contains_any(text, &["print", "stdout", "output", "runtime output"]) {
        capabilities.push("runtime-output behavior");
    }
    capabilities
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[derive(Debug, Clone)]
struct WorkspaceGraphModule {
    module_name: String,
    relative_path: String,
    functions: Vec<String>,
    exports: Vec<String>,
}

fn collect_workspace_reuse_candidates(
    spec: &IntentSpec,
    workspace: &Path,
    issues: &mut Vec<IntentPreflightIssue>,
) -> Vec<IntentReuseCandidate> {
    let graph_dir = workspace.join(".duumbi").join("graph");
    if !graph_dir.exists() {
        return Vec::new();
    }
    if !graph_dir.is_dir() {
        push_reuse_scan_warning(
            issues,
            ".duumbi/graph",
            "workspace graph path exists but is not a directory",
        );
        return Vec::new();
    }

    let targets = ReuseTargets::from_spec(spec);
    let mut candidates = Vec::new();
    for path in jsonld_files_under(&graph_dir, issues) {
        match load_workspace_graph_module(&graph_dir, &path) {
            Ok(Some(module)) => {
                if let Some(candidate) = module.to_reuse_candidate(&targets) {
                    candidates.push(candidate);
                }
            }
            Ok(None) => {}
            Err(message) => {
                push_reuse_scan_warning(issues, relative_path(&graph_dir, &path), message)
            }
        }
    }

    candidates
}

#[derive(Debug, Default)]
struct ReuseTargets {
    module_names: BTreeSet<String>,
    test_functions: BTreeSet<String>,
    acceptance_functions: BTreeSet<String>,
}

impl ReuseTargets {
    fn from_spec(spec: &IntentSpec) -> Self {
        Self {
            module_names: spec
                .modules
                .create
                .iter()
                .chain(spec.modules.modify.iter())
                .map(|module| module.trim())
                .filter(|module| !module.is_empty())
                .map(ToString::to_string)
                .collect(),
            test_functions: spec
                .test_cases
                .iter()
                .map(|test_case| test_case.function.trim())
                .filter(|function| !function.is_empty())
                .map(ToString::to_string)
                .collect(),
            acceptance_functions: acceptance_function_names(&spec.acceptance_criteria),
        }
    }
}

impl WorkspaceGraphModule {
    fn to_reuse_candidate(&self, targets: &ReuseTargets) -> Option<IntentReuseCandidate> {
        let function_names = self.function_names();
        let mut reasons = BTreeSet::new();
        let mut confidence = "medium";

        if targets.module_names.contains(&self.module_name) {
            reasons.insert("planned module target already exists".to_string());
            confidence = "high";
        }

        let matching_test_functions = sorted_intersection(&function_names, &targets.test_functions);
        if !matching_test_functions.is_empty() {
            reasons.insert(format!(
                "test case function already exists: {}",
                matching_test_functions.join(", ")
            ));
        }

        let matching_acceptance_functions =
            sorted_intersection(&function_names, &targets.acceptance_functions);
        if !matching_acceptance_functions.is_empty() {
            reasons.insert(format!(
                "acceptance criteria function already exists: {}",
                matching_acceptance_functions.join(", ")
            ));
        }

        if reasons.is_empty() {
            return None;
        }

        Some(IntentReuseCandidate::new(
            &self.module_name,
            &self.relative_path,
            function_names.into_iter().collect(),
            reasons.into_iter().collect::<Vec<_>>().join("; "),
            confidence,
        ))
    }

    fn function_names(&self) -> BTreeSet<String> {
        self.functions
            .iter()
            .chain(self.exports.iter())
            .map(|function| function.trim())
            .filter(|function| !function.is_empty())
            .map(ToString::to_string)
            .collect()
    }
}

fn jsonld_files_under(dir: &Path, issues: &mut Vec<IntentPreflightIssue>) -> Vec<PathBuf> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            push_reuse_scan_warning(
                issues,
                relative_path(&dir.join(".."), dir),
                format!("could not read workspace graph directory: {err}"),
            );
            return Vec::new();
        }
    };

    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(err) => push_reuse_scan_warning(
                issues,
                relative_path(&dir.join(".."), dir),
                format!("could not read workspace graph entry: {err}"),
            ),
        }
    }
    paths.sort();

    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            files.extend(jsonld_files_under(&path, issues));
        } else if path
            .extension()
            .is_some_and(|extension| extension == "jsonld")
        {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn load_workspace_graph_module(
    graph_dir: &Path,
    path: &Path,
) -> Result<Option<WorkspaceGraphModule>, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("could not read JSON-LD: {err}"))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|err| format!("could not parse JSON-LD: {err}"))?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let Some(module_name) = object
        .get("duumbi:name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|module_name| !module_name.is_empty())
    else {
        return Ok(None);
    };

    Ok(Some(WorkspaceGraphModule {
        module_name: module_name.to_string(),
        relative_path: relative_path(graph_dir, path),
        functions: extract_function_names(object.get("duumbi:functions")),
        exports: extract_string_array(object.get("duumbi:exports")),
    }))
}

fn extract_function_names(value: Option<&Value>) -> Vec<String> {
    let Some(functions) = value.and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut names = functions
        .iter()
        .filter_map(|function| function.get("duumbi:name"))
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn extract_string_array(value: Option<&Value>) -> Vec<String> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut strings = values
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    strings.sort();
    strings.dedup();
    strings
}

fn acceptance_function_names(criteria: &[String]) -> BTreeSet<String> {
    criteria
        .iter()
        .flat_map(|criterion| function_names_before_parentheses(criterion))
        .filter(|name| !matches!(name.as_str(), "if" | "for" | "while" | "when"))
        .collect()
}

fn test_function_names(spec: &IntentSpec) -> BTreeSet<String> {
    spec.test_cases
        .iter()
        .map(|test_case| test_case.function.trim())
        .filter(|function| !function.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn test_function_counts(spec: &IntentSpec) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for function in spec
        .test_cases
        .iter()
        .map(|test_case| test_case.function.trim())
        .filter(|function| !function.is_empty())
    {
        *counts.entry(function.to_string()).or_default() += 1;
    }
    counts
}

fn duplicate_test_names(spec: &IntentSpec) -> Vec<String> {
    let names = spec
        .test_cases
        .iter()
        .map(|test_case| test_case.name.clone())
        .collect::<Vec<_>>();
    duplicate_values(&names)
}

fn function_names_before_parentheses(text: &str) -> Vec<String> {
    text.char_indices()
        .filter_map(|(idx, ch)| (ch == '(').then_some(&text[..idx]))
        .filter_map(|prefix| {
            prefix
                .split(|ch: char| !is_function_name_char(ch))
                .next_back()
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
        })
        .collect()
}

fn is_function_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '/' | ':')
}

fn sorted_intersection(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.intersection(right).cloned().collect()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn push_reuse_scan_warning(
    issues: &mut Vec<IntentPreflightIssue>,
    field_path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(IntentPreflightIssue::new(
        "W_REUSE_SCAN_UNAVAILABLE",
        IntentPreflightSeverity::Warning,
        field_path,
        message,
        "continue with spec-only preflight evidence and inspect workspace graph files manually",
    ));
}

fn collect_decomposition_hints(spec: &IntentSpec) -> Vec<String> {
    let mut hints = Vec::new();

    for module in non_empty_modules(&spec.modules.create) {
        hints.push(format!(
            "Create module '{module}' before dependent module changes."
        ));
    }

    for module in non_main_modules(&spec.modules.modify) {
        let functions = module_candidate_functions(spec);
        if functions.is_empty() {
            hints.push(format!(
                "Modify existing module '{module}' to satisfy acceptance criteria."
            ));
        } else {
            hints.push(format!(
                "Modify existing module '{module}' to host or update callable functions: {}.",
                functions.join(", ")
            ));
        }
    }

    hints.extend(missing_host_hints(spec));

    let main_targets = main_modules(spec);
    if main_targets.is_empty() && has_context_entrypoint(spec) {
        hints.push(
            "Apply explicit context entrypoint wiring after callable module work.".to_string(),
        );
    } else {
        for module in main_targets {
            hints.push(format!(
                "Update '{module}' last to demonstrate callable functions and return 0."
            ));
        }
    }

    hints
}

fn non_empty_modules(modules: &[String]) -> Vec<String> {
    modules
        .iter()
        .map(|module| module.trim())
        .filter(|module| !module.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn non_main_modules(modules: &[String]) -> Vec<String> {
    non_empty_modules(modules)
        .into_iter()
        .filter(|module| !is_main_module(module))
        .collect()
}

fn main_modules(spec: &IntentSpec) -> Vec<String> {
    spec.modules
        .create
        .iter()
        .chain(spec.modules.modify.iter())
        .map(|module| module.trim())
        .filter(|module| is_main_module(module))
        .map(ToString::to_string)
        .collect()
}

fn is_main_module(module: &str) -> bool {
    matches!(module, "app/main" | "main")
}

fn has_context_entrypoint(spec: &IntentSpec) -> bool {
    spec.context
        .as_ref()
        .and_then(|context| context.entrypoint.as_deref())
        .is_some_and(|entrypoint| !entrypoint.trim().is_empty())
}

fn module_candidate_functions(spec: &IntentSpec) -> Vec<String> {
    let mut functions = spec
        .test_cases
        .iter()
        .map(|test_case| test_case.function.trim())
        .filter(|function| !function.is_empty())
        .filter(|function| *function != "main")
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    functions.extend(acceptance_function_names(&spec.acceptance_criteria));
    functions.into_iter().collect()
}

fn missing_host_hints(spec: &IntentSpec) -> Vec<String> {
    let host_modules = spec
        .modules
        .create
        .iter()
        .chain(spec.modules.modify.iter())
        .map(|module| module.trim())
        .filter(|module| !module.is_empty())
        .filter(|module| !is_main_module(module))
        .collect::<Vec<_>>();
    if !host_modules.is_empty() {
        return Vec::new();
    }

    spec.test_cases
        .iter()
        .map(|test_case| test_case.function.trim())
        .filter(|function| !function.is_empty())
        .filter(|function| *function != "main")
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|function| {
            format!("Callable test function '{function}' has no obvious non-main module host.")
        })
        .collect()
}

fn duplicate_values(values: &[String]) -> Vec<String> {
    let mut counts = HashMap::<String, usize>::new();
    for value in values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        *counts.entry(value.to_string()).or_default() += 1;
    }

    let mut duplicates = counts
        .into_iter()
        .filter_map(|(value, count)| (count > 1).then_some(value))
        .collect::<Vec<_>>();
    duplicates.sort();
    duplicates
}

fn normalized_non_empty_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn is_executable_graph_changing_intent(spec: &IntentSpec) -> bool {
    !spec.modules.create.is_empty() || !spec.modules.modify.is_empty()
}

/// Calculates readiness from score and highest severity.
#[must_use]
pub fn calculate_readiness(
    score: u8,
    highest_severity: Option<IntentPreflightSeverity>,
) -> IntentPreflightReadiness {
    if highest_severity == Some(IntentPreflightSeverity::Error) || score < WARN_SCORE_THRESHOLD {
        IntentPreflightReadiness::Block
    } else if score >= PASS_SCORE_THRESHOLD {
        IntentPreflightReadiness::Pass
    } else {
        IntentPreflightReadiness::Warn
    }
}

/// Calculates a deterministic score from issue severities.
#[must_use]
pub fn calculate_score(issues: &[IntentPreflightIssue]) -> u8 {
    let penalty = issues
        .iter()
        .map(|issue| u16::from(issue.severity.score_penalty()))
        .sum::<u16>();
    100u8.saturating_sub(u8::try_from(penalty).unwrap_or(u8::MAX))
}

/// Returns the highest severity in a list of issues.
#[must_use]
pub fn highest_severity(issues: &[IntentPreflightIssue]) -> Option<IntentPreflightSeverity> {
    issues.iter().map(|issue| issue.severity).min()
}

/// Renders a bounded, human-readable report.
#[must_use]
pub fn render_preflight_report(report: &IntentPreflightReport) -> Vec<String> {
    render_preflight_report_with_limit(report, DEFAULT_RENDER_LIMIT)
}

/// Renders a bounded, human-readable report with an explicit group limit.
#[must_use]
pub fn render_preflight_report_with_limit(
    report: &IntentPreflightReport,
    limit: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let error_count = count_severity(report, IntentPreflightSeverity::Error);
    let warning_count = count_severity(report, IntentPreflightSeverity::Warning);
    let info_count = count_severity(report, IntentPreflightSeverity::Info);

    lines.push(format!(
        "Preflight: {} (score {}, {} error{}, {} warning{}, {} info{})",
        report.readiness.label(),
        report.score,
        error_count,
        plural(error_count),
        warning_count,
        plural(warning_count),
        info_count,
        plural(info_count)
    ));

    append_issue_group(
        &mut lines,
        "Errors",
        report,
        IntentPreflightSeverity::Error,
        limit,
    );
    append_issue_group(
        &mut lines,
        "Warnings",
        report,
        IntentPreflightSeverity::Warning,
        limit,
    );
    append_issue_group(
        &mut lines,
        "Info",
        report,
        IntentPreflightSeverity::Info,
        limit,
    );
    append_reuse_candidates(&mut lines, &report.reuse_candidates, limit);
    append_hints(&mut lines, &report.decomposition_hints, limit);

    lines
}

fn count_severity(report: &IntentPreflightReport, severity: IntentPreflightSeverity) -> usize {
    report
        .issues
        .iter()
        .filter(|issue| issue.severity == severity)
        .count()
}

fn append_issue_group(
    lines: &mut Vec<String>,
    title: &str,
    report: &IntentPreflightReport,
    severity: IntentPreflightSeverity,
    limit: usize,
) {
    let issues: Vec<&IntentPreflightIssue> = report
        .issues
        .iter()
        .filter(|issue| issue.severity == severity)
        .collect();
    if issues.is_empty() {
        return;
    }

    lines.push(format!("{title}:"));
    for issue in issues.iter().take(limit) {
        lines.push(format!(
            "  {} {} - {} ({})",
            issue.code, issue.field_path, issue.message, issue.suggested_fix
        ));
    }
    append_overflow(lines, issues.len(), limit, title);
}

fn append_reuse_candidates(
    lines: &mut Vec<String>,
    candidates: &[IntentReuseCandidate],
    limit: usize,
) {
    if candidates.is_empty() {
        return;
    }

    lines.push("Reuse candidates:".to_string());
    for candidate in candidates.iter().take(limit) {
        let functions = if candidate.functions.is_empty() {
            "no functions listed".to_string()
        } else {
            format!("exports {}", candidate.functions.join(", "))
        };
        lines.push(format!(
            "  {} - {} ({}, confidence: {})",
            candidate.module_name, functions, candidate.reason, candidate.confidence
        ));
    }
    append_overflow(lines, candidates.len(), limit, "reuse candidates");
}

fn append_hints(lines: &mut Vec<String>, hints: &[String], limit: usize) {
    if hints.is_empty() {
        return;
    }

    lines.push("Decomposition hints:".to_string());
    for hint in hints.iter().take(limit) {
        lines.push(format!("  {hint}"));
    }
    append_overflow(lines, hints.len(), limit, "decomposition hints");
}

fn append_overflow(lines: &mut Vec<String>, total: usize, limit: usize, label: &str) {
    if total > limit {
        lines.push(format!("  ... {} more {label}", total - limit));
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn sort_issues(issues: &mut [IntentPreflightIssue]) {
    issues.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.field_path.cmp(&b.field_path))
    });
}

fn sort_reuse_candidates(candidates: &mut [IntentReuseCandidate]) {
    candidates.sort_by(|a, b| {
        a.module_name
            .cmp(&b.module_name)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
}

fn dedup_preserving_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentContext, IntentModules, IntentSpec, IntentStatus, TestCase};
    use std::fs;
    use tempfile::TempDir;

    fn issue(
        code: &str,
        severity: IntentPreflightSeverity,
        field_path: &str,
    ) -> IntentPreflightIssue {
        IntentPreflightIssue::new(code, severity, field_path, "message", "suggested fix")
    }

    fn strong_spec() -> IntentSpec {
        IntentSpec {
            intent: "Build calculator operations".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["add(a, b) returns a + b".to_string()],
            modules: IntentModules {
                create: vec!["calculator/ops".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: vec![
                TestCase {
                    name: "addition".to_string(),
                    function: "add".to_string(),
                    args: vec![3, 5],
                    expected_return: 8,
                },
                TestCase {
                    name: "addition_zero".to_string(),
                    function: "add".to_string(),
                    args: vec![0, 5],
                    expected_return: 5,
                },
            ],
            dependencies: Vec::new(),
            context: None,
            created_at: None,
            execution: None,
        }
    }

    fn issue_codes(report: &IntentPreflightReport) -> Vec<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }

    fn write_graph_module(workspace: &TempDir, relative_path: &str, contents: &str) {
        let path = workspace.path().join(relative_path);
        fs::create_dir_all(path.parent().expect("invariant: graph path has parent"))
            .expect("invariant: graph parent directory is creatable");
        fs::write(path, contents).expect("invariant: graph fixture is writable");
    }

    fn graph_module_json(module_name: &str, functions: &[&str], exports: &[&str]) -> String {
        let functions = functions
            .iter()
            .map(|function| {
                format!(
                    r#"{{"@type":"duumbi:Function","duumbi:name":"{function}","duumbi:blocks":[]}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let exports = exports
            .iter()
            .map(|export| format!(r#""{export}""#))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{
                "@context": "https://duumbi.dev/context/v1",
                "@type": "duumbi:Module",
                "duumbi:name": "{module_name}",
                "duumbi:exports": [{exports}],
                "duumbi:functions": [{functions}]
            }}"#
        )
    }

    #[test]
    fn preflight_empty_report_passes_with_full_score() {
        let report = IntentPreflightReport::from_parts(Vec::new(), Vec::new(), Vec::new());

        assert_eq!(report.readiness, IntentPreflightReadiness::Pass);
        assert_eq!(report.score, 100);
        assert_eq!(report.highest_severity, None);
        assert!(!report.is_blocking());
    }

    #[test]
    fn preflight_warning_report_warns_below_pass_threshold() {
        let report = IntentPreflightReport::from_parts(
            vec![issue(
                "W_EDGE_COVERAGE",
                IntentPreflightSeverity::Warning,
                "test_cases",
            )],
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(report.readiness, IntentPreflightReadiness::Warn);
        assert_eq!(report.score, 80);
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn preflight_error_report_blocks_even_with_moderate_score() {
        let report = IntentPreflightReport::from_parts(
            vec![issue(
                "E_EMPTY_INTENT",
                IntentPreflightSeverity::Error,
                "intent",
            )],
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(report.readiness, IntentPreflightReadiness::Block);
        assert_eq!(report.score, 55);
        assert!(report.is_blocking());
    }

    #[test]
    fn preflight_score_clamps_to_zero() {
        let issues = (0..10)
            .map(|idx| {
                issue(
                    &format!("E_{idx:02}"),
                    IntentPreflightSeverity::Error,
                    "intent",
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(calculate_score(&issues), 0);
    }

    #[test]
    fn preflight_highest_severity_prefers_error() {
        let issues = vec![
            issue("I_REUSE", IntentPreflightSeverity::Info, "modules"),
            issue("W_EDGE", IntentPreflightSeverity::Warning, "test_cases"),
            issue("E_MODULE", IntentPreflightSeverity::Error, "modules.create"),
        ];

        assert_eq!(
            highest_severity(&issues),
            Some(IntentPreflightSeverity::Error)
        );
    }

    #[test]
    fn preflight_render_is_deterministic_and_bounded() {
        let report = IntentPreflightReport::from_parts(
            vec![
                issue("W_B", IntentPreflightSeverity::Warning, "z"),
                issue("E_A", IntentPreflightSeverity::Error, "b"),
                issue("E_A", IntentPreflightSeverity::Error, "a"),
                issue("W_A", IntentPreflightSeverity::Warning, "a"),
            ],
            vec![IntentReuseCandidate::new(
                "math/ops",
                "math/ops.jsonld",
                vec!["sub".to_string(), "add".to_string()],
                "matching function",
                "medium",
            )],
            vec![
                "Create calculator/ops before modifying app/main".to_string(),
                "Create calculator/ops before modifying app/main".to_string(),
            ],
        );

        let rendered = render_preflight_report_with_limit(&report, 1);

        assert_eq!(
            rendered,
            vec![
                "Preflight: BLOCK (score 0, 2 errors, 2 warnings, 0 infos)",
                "Errors:",
                "  E_A a - message (suggested fix)",
                "  ... 1 more Errors",
                "Warnings:",
                "  W_A a - message (suggested fix)",
                "  ... 1 more Warnings",
                "Reuse candidates:",
                "  math/ops - exports add, sub (matching function, confidence: medium)",
                "Decomposition hints:",
                "  Create calculator/ops before modifying app/main",
            ]
        );
    }

    #[test]
    fn spec_checks_strong_spec_passes() {
        let report = run_spec_checks(&strong_spec());

        assert_eq!(report.readiness, IntentPreflightReadiness::Pass);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn spec_checks_empty_intent_blocks() {
        let mut spec = strong_spec();
        spec.intent = "  ".to_string();

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_EMPTY_INTENT"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_unsupported_version_blocks() {
        let mut spec = strong_spec();
        spec.version = 2;

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_UNSUPPORTED_VERSION"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_no_module_targets_blocks() {
        let mut spec = strong_spec();
        spec.modules = IntentModules::default();

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_NO_MODULE_TARGETS"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_empty_module_names_block() {
        let mut spec = strong_spec();
        spec.modules.create.push(" ".to_string());
        spec.modules.modify.push(String::new());

        let report = run_spec_checks(&spec);

        assert_eq!(
            issue_codes(&report)
                .into_iter()
                .filter(|code| *code == "E_EMPTY_MODULE_NAME")
                .count(),
            2
        );
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_duplicate_module_names_warn() {
        let mut spec = strong_spec();
        spec.modules.create.push("calculator/ops".to_string());

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_DUPLICATE_MODULE_NAME"));
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn spec_checks_duplicate_create_modify_target_blocks() {
        let mut spec = strong_spec();
        spec.modules.modify.push("calculator/ops".to_string());

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_DUPLICATE_CREATE_MODIFY_TARGET"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_missing_main_or_context_entrypoint_blocks() {
        let mut spec = strong_spec();
        spec.modules.modify.clear();

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_MISSING_ENTRYPOINT"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_context_entrypoint_satisfies_entrypoint_requirement() {
        let mut spec = strong_spec();
        spec.modules.modify.clear();
        spec.context = Some(IntentContext {
            entrypoint: Some("cli demo command".to_string()),
            ..IntentContext::default()
        });

        let report = run_spec_checks(&spec);

        assert!(!issue_codes(&report).contains(&"E_MISSING_ENTRYPOINT"));
    }

    #[test]
    fn spec_checks_empty_acceptance_criteria_blocks() {
        let mut spec = strong_spec();
        spec.acceptance_criteria = vec![" ".to_string()];

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_EMPTY_ACCEPTANCE_CRITERIA"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_no_test_cases_for_executable_graph_change_blocks() {
        let mut spec = strong_spec();
        spec.test_cases.clear();

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_NO_TEST_CASES"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_main_test_case_blocks() {
        let mut spec = strong_spec();
        spec.test_cases.push(TestCase {
            name: "main_result".to_string(),
            function: "main".to_string(),
            args: Vec::new(),
            expected_return: 0,
        });

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"E_MAIN_TEST_CASE"));
        assert!(report.is_blocking());
    }

    #[test]
    fn spec_checks_duplicate_test_names_warn() {
        let mut spec = strong_spec();
        spec.test_cases[1].name = "addition".to_string();

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_DUPLICATE_TEST_NAME"));
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn spec_checks_test_function_not_in_acceptance_warns_when_inferable() {
        let mut spec = strong_spec();
        spec.test_cases.push(TestCase {
            name: "subtraction".to_string(),
            function: "sub".to_string(),
            args: vec![8, 3],
            expected_return: 5,
        });

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_TEST_FUNCTION_NOT_IN_ACCEPTANCE"));
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn spec_checks_acceptance_function_without_test_coverage_warns() {
        let mut spec = strong_spec();
        spec.acceptance_criteria = vec![
            "add(a, b) returns a + b".to_string(),
            "sub(a, b) returns a - b".to_string(),
        ];

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_ACCEPTANCE_FUNCTION_UNTESTED"));
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn spec_checks_one_test_case_per_function_warns() {
        let mut spec = strong_spec();
        spec.test_cases.truncate(1);

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_SINGLE_TEST_CASE_FOR_FUNCTION"));
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn spec_checks_boolean_like_spec_warns_without_false_return_coverage() {
        let mut spec = strong_spec();
        spec.intent = "Build is_even boolean checker".to_string();
        spec.acceptance_criteria = vec!["is_even(n) returns 1 when n is even".to_string()];
        spec.test_cases = vec![TestCase {
            name: "even".to_string(),
            function: "is_even".to_string(),
            args: vec![4],
            expected_return: 1,
        }];

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_BOOLEAN_RETURN_COVERAGE"));
        assert_eq!(
            report.highest_severity,
            Some(IntentPreflightSeverity::Warning)
        );
    }

    #[test]
    fn spec_checks_boolean_like_spec_accepts_true_and_false_return_coverage() {
        let mut spec = strong_spec();
        spec.intent = "Build is_even boolean checker".to_string();
        spec.acceptance_criteria = vec!["is_even(n) returns 1 for even and 0 for odd".to_string()];
        spec.test_cases = vec![
            TestCase {
                name: "even".to_string(),
                function: "is_even".to_string(),
                args: vec![4],
                expected_return: 1,
            },
            TestCase {
                name: "odd".to_string(),
                function: "is_even".to_string(),
                args: vec![5],
                expected_return: 0,
            },
        ];

        let report = run_spec_checks(&spec);

        assert!(!issue_codes(&report).contains(&"W_BOOLEAN_RETURN_COVERAGE"));
    }

    #[test]
    fn spec_checks_division_or_modulo_warns_without_zero_denominator_edge() {
        let mut spec = strong_spec();
        spec.intent = "Build integer division".to_string();
        spec.acceptance_criteria = vec!["divide(a, b) returns a divided by b".to_string()];
        spec.test_cases = vec![TestCase {
            name: "positive_division".to_string(),
            function: "divide".to_string(),
            args: vec![10, 2],
            expected_return: 5,
        }];

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_DIVISION_MODULO_ZERO_EDGE"));
    }

    #[test]
    fn spec_checks_division_or_modulo_accepts_zero_denominator_edge() {
        let mut spec = strong_spec();
        spec.intent = "Build integer division".to_string();
        spec.acceptance_criteria = vec!["divide(a, b) handles division by denominator".to_string()];
        spec.test_cases = vec![TestCase {
            name: "zero_denominator".to_string(),
            function: "divide".to_string(),
            args: vec![10, 0],
            expected_return: 0,
        }];

        let report = run_spec_checks(&spec);

        assert!(!issue_codes(&report).contains(&"W_DIVISION_MODULO_ZERO_EDGE"));
    }

    #[test]
    fn spec_checks_branching_or_comparison_warns_without_equal_or_boundary_case() {
        let mut spec = strong_spec();
        spec.intent = "Build maximum comparison".to_string();
        spec.acceptance_criteria = vec!["max(a, b) returns the greater than value".to_string()];
        spec.test_cases = vec![TestCase {
            name: "right_is_larger".to_string(),
            function: "max".to_string(),
            args: vec![3, 5],
            expected_return: 5,
        }];

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_BRANCH_BOUNDARY_COVERAGE"));
    }

    #[test]
    fn spec_checks_branching_or_comparison_accepts_equal_or_boundary_case() {
        let mut spec = strong_spec();
        spec.intent = "Build maximum comparison".to_string();
        spec.acceptance_criteria = vec!["max(a, b) returns the greater value".to_string()];
        spec.test_cases = vec![TestCase {
            name: "equal_inputs".to_string(),
            function: "max".to_string(),
            args: vec![5, 5],
            expected_return: 5,
        }];

        let report = run_spec_checks(&spec);

        assert!(!issue_codes(&report).contains(&"W_BRANCH_BOUNDARY_COVERAGE"));
    }

    #[test]
    fn spec_checks_not_directly_verifiable_capabilities_warn() {
        let mut spec = strong_spec();
        spec.intent = "Build string runtime output behavior".to_string();
        spec.acceptance_criteria =
            vec!["reverse string input and print runtime output for empty input".to_string()];

        let report = run_spec_checks(&spec);

        assert!(issue_codes(&report).contains(&"W_NOT_DIRECTLY_VERIFIABLE_CAPABILITY"));
    }

    #[test]
    fn preflight_reuse_scans_workspace_graph_candidates() {
        let workspace = TempDir::new().expect("invariant: tempdir");
        write_graph_module(
            &workspace,
            ".duumbi/graph/math/ops.jsonld",
            &graph_module_json("calculator/ops", &["sub", "add"], &["add"]),
        );

        let report = run_preflight(&strong_spec(), workspace.path());

        assert_eq!(report.reuse_candidates.len(), 1);
        let candidate = &report.reuse_candidates[0];
        assert_eq!(candidate.module_name, "calculator/ops");
        assert_eq!(candidate.relative_path, "math/ops.jsonld");
        assert_eq!(candidate.functions, vec!["add", "sub"]);
        assert!(
            candidate
                .reason
                .contains("planned module target already exists")
        );
        assert!(
            candidate
                .reason
                .contains("test case function already exists: add")
        );
        assert!(
            candidate
                .reason
                .contains("acceptance criteria function already exists: add")
        );
        assert_eq!(candidate.confidence, "high");
    }

    #[test]
    fn preflight_reuse_excludes_vendor_and_cache_modules() {
        let workspace = TempDir::new().expect("invariant: tempdir");
        write_graph_module(
            &workspace,
            ".duumbi/vendor/calculator/ops.jsonld",
            &graph_module_json("calculator/ops", &["add"], &["add"]),
        );
        write_graph_module(
            &workspace,
            ".duumbi/cache/calculator/ops.jsonld",
            &graph_module_json("calculator/ops", &["add"], &["add"]),
        );

        let report = run_preflight(&strong_spec(), workspace.path());

        assert!(report.reuse_candidates.is_empty());
        assert!(!issue_codes(&report).contains(&"W_REUSE_SCAN_UNAVAILABLE"));
    }

    #[test]
    fn preflight_reuse_candidate_ordering_is_deterministic() {
        let workspace = TempDir::new().expect("invariant: tempdir");
        write_graph_module(
            &workspace,
            ".duumbi/graph/zeta.jsonld",
            &graph_module_json("zeta", &["fib"], &[]),
        );
        write_graph_module(
            &workspace,
            ".duumbi/graph/alpha.jsonld",
            &graph_module_json("alpha", &["add"], &[]),
        );

        let mut spec = strong_spec();
        spec.acceptance_criteria = vec!["add(a, b) and fib(n) are callable".to_string()];

        let report = run_preflight(&spec, workspace.path());

        let candidate_names = report
            .reuse_candidates
            .iter()
            .map(|candidate| candidate.module_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(candidate_names, vec!["alpha", "zeta"]);
    }

    #[test]
    fn preflight_reuse_scan_failure_warns_without_blocking_spec_checks() {
        let workspace = TempDir::new().expect("invariant: tempdir");
        write_graph_module(&workspace, ".duumbi/graph/bad.jsonld", "{not valid JSON-LD");

        let report = run_preflight(&strong_spec(), workspace.path());

        assert!(issue_codes(&report).contains(&"W_REUSE_SCAN_UNAVAILABLE"));
        assert!(!report.is_blocking());
        assert!(report.reuse_candidates.is_empty());
    }

    #[test]
    fn preflight_decomposition_hints_order_create_modify_then_main() {
        let mut spec = strong_spec();
        spec.modules = IntentModules {
            create: vec!["calculator/new_ops".to_string()],
            modify: vec![
                "calculator/existing_ops".to_string(),
                "app/main".to_string(),
            ],
        };

        let report = run_spec_checks(&spec);

        assert_eq!(
            report.decomposition_hints,
            vec![
                "Create module 'calculator/new_ops' before dependent module changes.",
                "Modify existing module 'calculator/existing_ops' to host or update callable functions: add.",
                "Update 'app/main' last to demonstrate callable functions and return 0.",
            ]
        );
    }

    #[test]
    fn preflight_decomposition_hints_keep_non_main_modify_before_main() {
        let mut spec = strong_spec();
        spec.modules = IntentModules {
            create: Vec::new(),
            modify: vec!["math/ops".to_string(), "main".to_string()],
        };
        spec.test_cases = vec![TestCase {
            name: "subtraction".to_string(),
            function: "sub".to_string(),
            args: vec![8, 3],
            expected_return: 5,
        }];
        spec.acceptance_criteria = vec!["sub(a, b) returns a - b".to_string()];

        let report = run_spec_checks(&spec);

        assert_eq!(
            report.decomposition_hints,
            vec![
                "Modify existing module 'math/ops' to host or update callable functions: sub.",
                "Update 'main' last to demonstrate callable functions and return 0.",
            ]
        );
    }

    #[test]
    fn preflight_decomposition_hints_put_main_last_for_create_only_spec() {
        let report = run_spec_checks(&strong_spec());

        assert_eq!(
            report.decomposition_hints.last().map(String::as_str),
            Some("Update 'app/main' last to demonstrate callable functions and return 0.")
        );
    }

    #[test]
    fn preflight_decomposition_hints_missing_callable_host() {
        let mut spec = strong_spec();
        spec.modules = IntentModules {
            create: Vec::new(),
            modify: vec!["app/main".to_string()],
        };

        let report = run_spec_checks(&spec);

        assert!(report.decomposition_hints.contains(
            &"Callable test function 'add' has no obvious non-main module host.".to_string()
        ));
        assert_eq!(
            report.decomposition_hints.last().map(String::as_str),
            Some("Update 'app/main' last to demonstrate callable functions and return 0.")
        );
    }
}
