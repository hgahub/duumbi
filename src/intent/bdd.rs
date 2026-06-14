//! BDD/Gherkin companion artifact support for runtime intents.
//!
//! The parser in this module is intentionally lightweight. It validates the
//! DUUMBI-owned subset needed for review, preflight, prompt context, and
//! evidence mapping without introducing a Cucumber runtime.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::intents_dir;
use super::preflight::{IntentPreflightIssue, IntentPreflightSeverity};
use super::spec::{IntentSpec, TestCase};

/// Default prompt/rendering limit for BDD scenario context.
pub const DEFAULT_BDD_CONTEXT_LIMIT: usize = 5;

/// Readiness state for linked BDD artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BddReadiness {
    /// Linked artifacts are present and structurally usable.
    Ready,
    /// No blocking problem exists, but BDD guidance is missing or partial.
    Warning,
    /// One or more explicit linked artifacts are unusable.
    Blocked,
}

impl BddReadiness {
    /// Returns the uppercase display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Warning => "WARNING",
            Self::Blocked => "BLOCKED",
        }
    }
}

/// A stable BDD readiness finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddIssue {
    /// Stable issue code.
    pub code: String,
    /// Whether this issue blocks execution.
    pub severity: IntentPreflightSeverity,
    /// Path or logical field where the issue occurred.
    pub field_path: String,
    /// Human-readable issue description.
    pub message: String,
    /// Suggested user or agent action.
    pub suggested_fix: String,
}

impl BddIssue {
    /// Creates a new BDD issue.
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

    /// Converts the BDD issue into a preflight issue.
    #[must_use]
    pub fn to_preflight_issue(&self) -> IntentPreflightIssue {
        IntentPreflightIssue::new(
            self.code.clone(),
            self.severity,
            self.field_path.clone(),
            self.message.clone(),
            self.suggested_fix.clone(),
        )
    }
}

/// One parsed `.feature` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddFeatureFile {
    /// User-provided feature reference from `bdd.feature_files`.
    pub reference: String,
    /// Resolved filesystem path.
    pub path: PathBuf,
    /// Feature title, when a `Feature:` line exists.
    pub feature: Option<String>,
    /// Parsed scenarios in source order.
    pub scenarios: Vec<BddScenario>,
}

/// One parsed Gherkin scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddScenario {
    /// Optional enclosing rule title.
    pub rule: Option<String>,
    /// Scenario title.
    pub name: String,
    /// Parsed Given/When/Then style steps.
    pub steps: Vec<BddStep>,
}

impl BddScenario {
    fn searchable_text(&self) -> String {
        let mut parts = vec![self.name.clone()];
        if let Some(rule) = &self.rule {
            parts.push(rule.clone());
        }
        parts.extend(self.steps.iter().map(|step| step.text.clone()));
        parts.join(" ").to_lowercase()
    }
}

/// A parsed Gherkin step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddStep {
    /// Semantic step kind.
    pub kind: BddStepKind,
    /// Step text without the keyword.
    pub text: String,
}

/// Supported Gherkin step kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BddStepKind {
    /// A `Given` precondition.
    Given,
    /// A `When` action.
    When,
    /// A `Then` expected outcome.
    Then,
}

impl BddStepKind {
    fn label(self) -> &'static str {
        match self {
            Self::Given => "Given",
            Self::When => "When",
            Self::Then => "Then",
        }
    }
}

/// Conservative coverage classification for one BDD scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BddCoverageClassification {
    /// The scenario appears fully covered by existing i64 verifier tests.
    VerifierCovered,
    /// The scenario partially maps to verifier tests but needs more evidence.
    PartiallyCovered,
    /// The scenario needs broader graph, run, UI, or manual evidence.
    BroaderEvidenceRequired,
    /// The scenario cannot be classified because the BDD artifact is blocked.
    Blocked,
}

impl BddCoverageClassification {
    /// Returns a stable display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::VerifierCovered => "verifier-covered",
            Self::PartiallyCovered => "partially-covered",
            Self::BroaderEvidenceRequired => "broader-evidence-required",
            Self::Blocked => "blocked",
        }
    }
}

/// Coverage evidence for one BDD scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddScenarioCoverage {
    /// Scenario name.
    pub scenario: String,
    /// Conservative classification.
    pub classification: BddCoverageClassification,
    /// Matching verifier test names.
    pub verifier_tests: Vec<String>,
}

/// Complete BDD readiness and coverage report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BddReadinessReport {
    /// Final BDD readiness.
    pub readiness: BddReadiness,
    /// Parsed feature files.
    pub feature_files: Vec<BddFeatureFile>,
    /// Stable BDD issue list.
    pub issues: Vec<BddIssue>,
    /// Scenario coverage classifications.
    pub coverage: Vec<BddScenarioCoverage>,
}

impl BddReadinessReport {
    /// Returns `true` when BDD readiness blocks execution.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.readiness == BddReadiness::Blocked
    }

    /// Returns the total parsed scenario count.
    #[must_use]
    pub fn scenario_count(&self) -> usize {
        self.feature_files
            .iter()
            .map(|feature| feature.scenarios.len())
            .sum()
    }
}

/// Returns the default intent-relative feature path for a slug.
#[must_use]
pub fn default_feature_path(slug: &str) -> PathBuf {
    PathBuf::from("features").join(format!("{slug}.feature"))
}

/// Returns the default intent-relative feature reference for a slug.
#[must_use]
pub fn default_feature_reference(slug: &str) -> String {
    format!("features/{slug}.feature")
}

/// Renders a deterministic fallback feature file for an intent.
#[must_use]
pub fn render_default_feature(spec: &IntentSpec, slug: &str) -> String {
    let title = sentence_case_title(&spec.intent, slug);
    let mut lines = vec![format!("Feature: {title}"), String::new()];

    if spec.test_cases.is_empty() {
        lines.extend([
            "  Scenario: Satisfy accepted intent behavior".to_string(),
            format!("    Given the intent {}", spec.intent.trim()),
            "    When DUUMBI executes the intent".to_string(),
            "    Then the accepted behavior is implemented and verified".to_string(),
        ]);
        return lines.join("\n") + "\n";
    }

    for test_case in &spec.test_cases {
        lines.push(format!(
            "  Scenario: {}",
            humanize_identifier(&test_case.name)
        ));
        lines.push(format!(
            "    Given the {} behavior is required",
            test_case.function
        ));
        lines.push(format!(
            "    When {} is called with {:?}",
            test_case.function, test_case.args
        ));
        lines.push(format!("    Then it returns {}", test_case.expected_return));
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Loads and validates BDD artifacts linked from an intent.
#[must_use]
pub fn load_bdd_report(spec: &IntentSpec, workspace: &Path, slug: &str) -> BddReadinessReport {
    if spec.bdd.feature_files.is_empty() {
        let issues = vec![BddIssue::new(
            "W_BDD_MISSING",
            IntentPreflightSeverity::Info,
            "bdd.feature_files",
            "intent has no linked BDD feature files",
            "add a .feature file under .duumbi/intents/<slug>/features/ for scenario guidance",
        )];
        return BddReadinessReport {
            readiness: BddReadiness::Warning,
            feature_files: Vec::new(),
            issues,
            coverage: Vec::new(),
        };
    }

    let mut issues = Vec::new();
    let mut feature_files = Vec::new();

    for (index, reference) in spec.bdd.feature_files.iter().enumerate() {
        let field_path = format!("bdd.feature_files[{index}]");
        match resolve_feature_path(workspace, slug, reference, &field_path) {
            Ok(path) => load_feature_file(
                workspace,
                reference,
                path,
                &field_path,
                &mut issues,
                &mut feature_files,
            ),
            Err(issue) => issues.push(issue),
        }
    }

    let readiness = readiness_from_issues(&issues);
    let coverage = classify_bdd_coverage(&feature_files, &spec.test_cases, readiness);

    BddReadinessReport {
        readiness,
        feature_files,
        issues,
        coverage,
    }
}

/// Validates generated BDD feature text before it is persisted.
#[must_use]
pub fn validate_bdd_feature_text(
    reference: &str,
    contents: &str,
    test_cases: &[TestCase],
) -> BddReadinessReport {
    let mut issues = Vec::new();
    let mut feature_files = Vec::new();

    if contents.trim().is_empty() {
        issues.push(BddIssue::new(
            "E_BDD_FILE_EMPTY",
            IntentPreflightSeverity::Error,
            "bdd_feature",
            "generated BDD feature text is empty",
            "use the deterministic default BDD feature instead",
        ));
    } else {
        let parsed = parse_feature(
            reference,
            PathBuf::from(reference),
            contents,
            "bdd_feature",
            &mut issues,
        );
        feature_files.push(parsed);
    }

    let readiness = readiness_from_issues(&issues);
    let coverage = classify_bdd_coverage(&feature_files, test_cases, readiness);

    BddReadinessReport {
        readiness,
        feature_files,
        issues,
        coverage,
    }
}

/// Converts BDD readiness findings into preflight issues.
#[must_use]
pub fn preflight_issues_for_bdd(
    spec: &IntentSpec,
    workspace: &Path,
    slug: &str,
) -> Vec<IntentPreflightIssue> {
    load_bdd_report(spec, workspace, slug)
        .issues
        .iter()
        .map(BddIssue::to_preflight_issue)
        .collect()
}

/// Renders a bounded human-readable BDD report.
#[must_use]
pub fn render_bdd_report(report: &BddReadinessReport) -> Vec<String> {
    let mut lines = vec![format!(
        "BDD readiness: {} ({} feature file(s), {} scenario(s))",
        report.readiness.label(),
        report.feature_files.len(),
        report.scenario_count()
    )];

    for feature_file in &report.feature_files {
        lines.push(format!(
            "- {}: {} scenario(s)",
            feature_file.reference,
            feature_file.scenarios.len()
        ));
        if let Some(feature) = &feature_file.feature {
            lines.push(format!("  Feature: {feature}"));
        }
        for scenario in feature_file
            .scenarios
            .iter()
            .take(DEFAULT_BDD_CONTEXT_LIMIT)
        {
            lines.push(format!("  Scenario: {}", scenario.name));
        }
        if feature_file.scenarios.len() > DEFAULT_BDD_CONTEXT_LIMIT {
            lines.push(format!(
                "  ... {} more scenario(s)",
                feature_file.scenarios.len() - DEFAULT_BDD_CONTEXT_LIMIT
            ));
        }
    }

    if !report.coverage.is_empty() {
        lines.push("Coverage:".to_string());
        for coverage in report.coverage.iter().take(DEFAULT_BDD_CONTEXT_LIMIT) {
            let tests = if coverage.verifier_tests.is_empty() {
                "no verifier tests".to_string()
            } else {
                format!("verifier tests: {}", coverage.verifier_tests.join(", "))
            };
            lines.push(format!(
                "  {}: {} ({tests})",
                coverage.scenario,
                coverage.classification.label()
            ));
        }
        if report.coverage.len() > DEFAULT_BDD_CONTEXT_LIMIT {
            lines.push(format!(
                "  ... {} more coverage classification(s)",
                report.coverage.len() - DEFAULT_BDD_CONTEXT_LIMIT
            ));
        }
    }

    for issue in &report.issues {
        lines.push(format!(
            "- {} [{}] {}: {}",
            issue.severity.label(),
            issue.code,
            issue.field_path,
            issue.message
        ));
    }

    lines
}

/// Renders bounded BDD scenario context for decomposition and LLM prompts.
#[must_use]
pub fn render_bdd_prompt_context(report: &BddReadinessReport, limit: usize) -> Vec<String> {
    if report.feature_files.is_empty() {
        return vec!["BDD scenario contract: unavailable".to_string()];
    }

    let limit = limit.max(1);
    let mut lines = vec!["BDD scenario contract:".to_string()];
    let mut rendered = 0;

    for feature_file in &report.feature_files {
        if let Some(feature) = &feature_file.feature {
            lines.push(format!("- Feature: {feature}"));
        }
        for scenario in &feature_file.scenarios {
            if rendered >= limit {
                let remaining = report.scenario_count().saturating_sub(rendered);
                if remaining > 0 {
                    lines.push(format!("- ... {remaining} more scenario(s) summarized"));
                }
                return lines;
            }
            lines.push(format!("- Scenario: {}", scenario.name));
            for step in scenario.steps.iter().take(3) {
                lines.push(format!("  {} {}", step.kind.label(), step.text));
            }
            rendered += 1;
        }
    }

    lines
}

fn resolve_feature_path(
    workspace: &Path,
    slug: &str,
    reference: &str,
    field_path: &str,
) -> Result<PathBuf, BddIssue> {
    let raw = Path::new(reference);
    if raw.is_absolute() || has_parent_component(raw) || reference.trim().is_empty() {
        return Err(BddIssue::new(
            "E_BDD_PATH_UNSAFE",
            IntentPreflightSeverity::Error,
            field_path,
            format!("BDD feature path '{reference}' is not a safe relative path"),
            "use an intent-relative features/*.feature path",
        ));
    }
    if raw.extension().and_then(|extension| extension.to_str()) != Some("feature") {
        return Err(BddIssue::new(
            "E_BDD_PATH_UNSAFE",
            IntentPreflightSeverity::Error,
            field_path,
            format!("BDD feature path '{reference}' does not use .feature extension"),
            "use a .feature file under the intent feature directory",
        ));
    }

    let path = if starts_with_duumbi_intents(raw) {
        workspace.join(raw)
    } else {
        intents_dir(workspace).join(slug).join(raw)
    };

    if !path_stays_under_intents(workspace, &path) {
        return Err(BddIssue::new(
            "E_BDD_PATH_UNSAFE",
            IntentPreflightSeverity::Error,
            field_path,
            format!("BDD feature path '{reference}' resolves outside .duumbi/intents/"),
            "keep BDD files under .duumbi/intents/",
        ));
    }

    Ok(path)
}

fn load_feature_file(
    workspace: &Path,
    reference: &str,
    path: PathBuf,
    field_path: &str,
    issues: &mut Vec<BddIssue>,
    feature_files: &mut Vec<BddFeatureFile>,
) {
    if !path.exists() {
        issues.push(BddIssue::new(
            "E_BDD_FILE_MISSING",
            IntentPreflightSeverity::Error,
            field_path,
            format!(
                "linked BDD feature file '{}' does not exist",
                path.display()
            ),
            "create the linked .feature file or remove the broken reference",
        ));
        return;
    }

    match resolved_path_stays_under_intents(workspace, &path) {
        Ok(true) => {}
        Ok(false) => {
            issues.push(BddIssue::new(
                "E_BDD_PATH_UNSAFE",
                IntentPreflightSeverity::Error,
                field_path,
                format!(
                    "BDD feature path '{}' resolves outside .duumbi/intents/",
                    path.display()
                ),
                "keep BDD files under .duumbi/intents/",
            ));
            return;
        }
        Err(_) => {
            issues.push(BddIssue::new(
                "E_BDD_FILE_UNREADABLE",
                IntentPreflightSeverity::Error,
                field_path,
                format!(
                    "linked BDD feature file '{}' could not be resolved",
                    path.display()
                ),
                "make the .feature file readable under .duumbi/intents/",
            ));
            return;
        }
    }

    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(_) => {
            issues.push(BddIssue::new(
                "E_BDD_FILE_UNREADABLE",
                IntentPreflightSeverity::Error,
                field_path,
                format!(
                    "linked BDD feature file '{}' is unreadable or not UTF-8",
                    path.display()
                ),
                "make the .feature file readable UTF-8 text",
            ));
            return;
        }
    };

    if contents.trim().is_empty() {
        issues.push(BddIssue::new(
            "E_BDD_FILE_EMPTY",
            IntentPreflightSeverity::Error,
            field_path,
            format!("linked BDD feature file '{}' is empty", path.display()),
            "add Feature and Scenario content to the .feature file",
        ));
        return;
    }

    let mut parsed_issues = Vec::new();
    let parsed = parse_feature(reference, path, &contents, field_path, &mut parsed_issues);
    issues.extend(parsed_issues);
    feature_files.push(parsed);
}

fn parse_feature(
    reference: &str,
    path: PathBuf,
    contents: &str,
    field_path: &str,
    issues: &mut Vec<BddIssue>,
) -> BddFeatureFile {
    let mut feature = None;
    let mut current_rule = None;
    let mut current_scenario: Option<BddScenario> = None;
    let mut scenarios = Vec::new();
    let mut last_step_kind = None;

    for (line_index, line) in contents.lines().enumerate() {
        let line_no = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            continue;
        }

        if let Some(title) = strip_keyword(trimmed, "Feature:") {
            feature = Some(title.to_string());
            continue;
        }
        if let Some(rule) = strip_keyword(trimmed, "Rule:") {
            current_rule = Some(rule.to_string());
            continue;
        }
        if let Some(name) = strip_keyword(trimmed, "Scenario:") {
            finish_scenario(current_scenario.take(), field_path, issues, &mut scenarios);
            current_scenario = Some(BddScenario {
                rule: current_rule.clone(),
                name: name.to_string(),
                steps: Vec::new(),
            });
            last_step_kind = None;
            continue;
        }
        if let Some(name) = strip_keyword(trimmed, "Scenario Outline:") {
            issues.push(BddIssue::new(
                "W_BDD_SCENARIO_OUTLINE_UNMAPPED",
                IntentPreflightSeverity::Warning,
                format!("{field_path}:{line_no}"),
                "scenario outlines are preserved but not mapped to verifier evidence in v1",
                "use concrete Scenario blocks when verifier mapping is required",
            ));
            finish_scenario(current_scenario.take(), field_path, issues, &mut scenarios);
            current_scenario = Some(BddScenario {
                rule: current_rule.clone(),
                name: name.to_string(),
                steps: Vec::new(),
            });
            last_step_kind = None;
            continue;
        }
        if trimmed == "Examples:" || trimmed.starts_with("Examples:") {
            issues.push(BddIssue::new(
                "W_BDD_SCENARIO_OUTLINE_UNMAPPED",
                IntentPreflightSeverity::Warning,
                format!("{field_path}:{line_no}"),
                "Examples tables are not mapped to verifier evidence in v1",
                "convert important examples into concrete Scenario blocks",
            ));
            continue;
        }

        if let Some((kind, text)) = parse_step(trimmed, last_step_kind) {
            if let Some(scenario) = &mut current_scenario {
                scenario.steps.push(BddStep { kind, text });
                last_step_kind = Some(kind);
            } else {
                issues.push(BddIssue::new(
                    "W_BDD_UNKNOWN_LINE",
                    IntentPreflightSeverity::Warning,
                    format!("{field_path}:{line_no}"),
                    "BDD step appears before any Scenario",
                    "move Given/When/Then steps inside a Scenario",
                ));
            }
            continue;
        }

        issues.push(BddIssue::new(
            "W_BDD_UNKNOWN_LINE",
            IntentPreflightSeverity::Warning,
            format!("{field_path}:{line_no}"),
            format!("unrecognized BDD line: {trimmed}"),
            "use Feature, Rule, Scenario, Given, When, Then, And, or But syntax",
        ));
    }

    finish_scenario(current_scenario, field_path, issues, &mut scenarios);

    if feature.is_none() {
        issues.push(BddIssue::new(
            "E_BDD_NO_FEATURE",
            IntentPreflightSeverity::Error,
            field_path,
            "BDD feature file has no Feature line",
            "add a Feature: line",
        ));
    }
    if scenarios.is_empty() {
        issues.push(BddIssue::new(
            "E_BDD_NO_SCENARIOS",
            IntentPreflightSeverity::Error,
            field_path,
            "BDD feature file has no Scenario lines",
            "add at least one Scenario with Given/When/Then steps",
        ));
    }

    BddFeatureFile {
        reference: reference.to_string(),
        path,
        feature,
        scenarios,
    }
}

fn finish_scenario(
    scenario: Option<BddScenario>,
    field_path: &str,
    issues: &mut Vec<BddIssue>,
    scenarios: &mut Vec<BddScenario>,
) {
    let Some(scenario) = scenario else {
        return;
    };
    let kinds: BTreeSet<BddStepKind> = scenario.steps.iter().map(|step| step.kind).collect();
    if !kinds.contains(&BddStepKind::Given)
        || !kinds.contains(&BddStepKind::When)
        || !kinds.contains(&BddStepKind::Then)
    {
        issues.push(BddIssue::new(
            "E_BDD_SCENARIO_INCOMPLETE",
            IntentPreflightSeverity::Error,
            field_path,
            format!(
                "scenario '{}' does not contain Given, When, and Then steps",
                scenario.name
            ),
            "add Given, When, and Then steps to the scenario",
        ));
        return;
    }
    scenarios.push(scenario);
}

fn classify_bdd_coverage(
    feature_files: &[BddFeatureFile],
    test_cases: &[TestCase],
    readiness: BddReadiness,
) -> Vec<BddScenarioCoverage> {
    feature_files
        .iter()
        .flat_map(|feature| feature.scenarios.iter())
        .map(|scenario| classify_scenario(scenario, test_cases, readiness))
        .collect()
}

fn classify_scenario(
    scenario: &BddScenario,
    test_cases: &[TestCase],
    readiness: BddReadiness,
) -> BddScenarioCoverage {
    if readiness == BddReadiness::Blocked {
        return BddScenarioCoverage {
            scenario: scenario.name.clone(),
            classification: BddCoverageClassification::Blocked,
            verifier_tests: Vec::new(),
        };
    }

    let text = scenario.searchable_text();
    let verifier_tests: Vec<String> = test_cases
        .iter()
        .filter(|test| {
            let name = test.name.to_lowercase();
            let function = test.function.to_lowercase();
            text.contains(&name) || text.contains(&function)
        })
        .map(|test| test.name.clone())
        .collect();

    let classification = if verifier_tests.is_empty() {
        BddCoverageClassification::BroaderEvidenceRequired
    } else if verifier_tests.len() == test_cases.len().max(1) {
        BddCoverageClassification::VerifierCovered
    } else {
        BddCoverageClassification::PartiallyCovered
    };

    BddScenarioCoverage {
        scenario: scenario.name.clone(),
        classification,
        verifier_tests,
    }
}

fn readiness_from_issues(issues: &[BddIssue]) -> BddReadiness {
    if issues
        .iter()
        .any(|issue| issue.severity == IntentPreflightSeverity::Error)
    {
        BddReadiness::Blocked
    } else if issues.is_empty() {
        BddReadiness::Ready
    } else {
        BddReadiness::Warning
    }
}

fn parse_step(trimmed: &str, last_step_kind: Option<BddStepKind>) -> Option<(BddStepKind, String)> {
    if let Some(text) = strip_keyword(trimmed, "Given ") {
        return Some((BddStepKind::Given, text.to_string()));
    }
    if let Some(text) = strip_keyword(trimmed, "When ") {
        return Some((BddStepKind::When, text.to_string()));
    }
    if let Some(text) = strip_keyword(trimmed, "Then ") {
        return Some((BddStepKind::Then, text.to_string()));
    }
    if let Some(text) = strip_keyword(trimmed, "And ") {
        return last_step_kind.map(|kind| (kind, text.to_string()));
    }
    if let Some(text) = strip_keyword(trimmed, "But ") {
        return last_step_kind.map(|kind| (kind, text.to_string()));
    }
    None
}

fn strip_keyword<'a>(value: &'a str, keyword: &str) -> Option<&'a str> {
    value
        .strip_prefix(keyword)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn starts_with_duumbi_intents(path: &Path) -> bool {
    let mut components = path.components();
    matches!(components.next(), Some(Component::Normal(value)) if value == ".duumbi")
        && matches!(components.next(), Some(Component::Normal(value)) if value == "intents")
}

fn path_stays_under_intents(workspace: &Path, path: &Path) -> bool {
    let intents = intents_dir(workspace);
    path.starts_with(intents)
}

fn resolved_path_stays_under_intents(workspace: &Path, path: &Path) -> std::io::Result<bool> {
    let intents = fs::canonicalize(intents_dir(workspace))?;
    let resolved = fs::canonicalize(path)?;
    Ok(resolved.starts_with(intents))
}

fn sentence_case_title(intent: &str, slug: &str) -> String {
    let trimmed = intent.trim();
    if trimmed.is_empty() {
        return humanize_identifier(slug);
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return humanize_identifier(slug);
    };
    first.to_uppercase().collect::<String>() + chars.as_str()
}

fn humanize_identifier(value: &str) -> String {
    value
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentBdd, IntentModules, IntentStatus};

    fn spec_with_bdd(reference: &str) -> IntentSpec {
        IntentSpec {
            intent: "Build calculator add behavior".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["add returns sums".to_string()],
            modules: IntentModules {
                create: vec!["calculator/ops".to_string()],
                modify: Vec::new(),
            },
            test_cases: vec![TestCase {
                name: "addition".to_string(),
                function: "add".to_string(),
                args: vec![3, 5],
                expected_return: 8,
            }],
            dependencies: Vec::new(),
            bdd: IntentBdd {
                feature_files: vec![reference.to_string()],
            },
            context: None,
            created_at: None,
            execution: None,
        }
    }

    fn write_feature(workspace: &Path, slug: &str, relative: &str, contents: &str) {
        let path = intents_dir(workspace).join(slug).join(relative);
        fs::create_dir_all(path.parent().expect("invariant: feature has parent"))
            .expect("invariant: feature dir is creatable");
        fs::write(path, contents).expect("invariant: feature is writable");
    }

    #[test]
    fn missing_bdd_is_warning_only() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let mut spec = spec_with_bdd("features/calculator.feature");
        spec.bdd.feature_files.clear();

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Warning);
        assert_eq!(report.issues[0].code, "W_BDD_MISSING");
        assert!(!report.is_blocking());
    }

    #[test]
    fn parses_feature_and_classifies_coverage() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_feature(
            tmp.path(),
            "calculator",
            "features/calculator.feature",
            "Feature: Calculator\n\n  Scenario: addition\n    Given the add behavior is required\n    When add is called with [3, 5]\n    Then it returns 8\n",
        );
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Ready);
        assert_eq!(report.scenario_count(), 1);
        assert_eq!(
            report.feature_files[0].feature.as_deref(),
            Some("Calculator")
        );
        assert_eq!(
            report.coverage[0].classification,
            BddCoverageClassification::VerifierCovered
        );
    }

    #[test]
    fn rendered_report_includes_coverage_classification() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_feature(
            tmp.path(),
            "calculator",
            "features/calculator.feature",
            "Feature: Calculator\n\n  Scenario: visible output\n    Given output behavior is required\n    When the program is run\n    Then it prints a result\n",
        );
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");
        let lines = render_bdd_report(&report);

        assert!(lines.iter().any(|line| line == "Coverage:"));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("broader-evidence-required"))
        );
    }

    #[test]
    fn unsafe_path_blocks() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let spec = spec_with_bdd("../outside.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.issues[0].code, "E_BDD_PATH_UNSAFE");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_feature_resolving_outside_intents_blocks() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let outside = tmp.path().join("outside.feature");
        fs::write(
            &outside,
            "Feature: Calculator\n\n  Scenario: addition\n    Given the add behavior is required\n    When add is called with [3, 5]\n    Then it returns 8\n",
        )
        .expect("outside feature");
        let link = intents_dir(tmp.path())
            .join("calculator")
            .join("features")
            .join("calculator.feature");
        fs::create_dir_all(link.parent().expect("invariant: feature has parent"))
            .expect("feature dir");
        std::os::unix::fs::symlink(&outside, &link).expect("feature symlink");
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.issues[0].code, "E_BDD_PATH_UNSAFE");
    }

    #[test]
    fn missing_feature_file_blocks() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let spec = spec_with_bdd("features/missing.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.issues[0].code, "E_BDD_FILE_MISSING");
    }

    #[test]
    fn unreadable_or_non_utf8_feature_file_blocks() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let path = intents_dir(tmp.path())
            .join("calculator")
            .join("features")
            .join("calculator.feature");
        fs::create_dir_all(path.parent().expect("invariant: feature has parent"))
            .expect("invariant: feature dir is creatable");
        fs::write(&path, [0xFF, 0xFE, 0xFD]).expect("invariant: invalid utf8 fixture writable");
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.issues[0].code, "E_BDD_FILE_UNREADABLE");
    }

    #[test]
    fn empty_feature_file_blocks() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_feature(
            tmp.path(),
            "calculator",
            "features/calculator.feature",
            "  \n",
        );
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.issues[0].code, "E_BDD_FILE_EMPTY");
    }

    #[test]
    fn incomplete_scenario_blocks() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_feature(
            tmp.path(),
            "calculator",
            "features/calculator.feature",
            "Feature: Calculator\n\n  Scenario: addition\n    Given add behavior\n",
        );
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.scenario_count(), 0);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "E_BDD_SCENARIO_INCOMPLETE")
        );
    }

    #[test]
    fn generated_feature_text_validation_blocks_structurally_invalid_feature() {
        let spec = spec_with_bdd("features/calculator.feature");
        let report = validate_bdd_feature_text(
            "features/calculator.feature",
            "Feature: Calculator\nScenario: incomplete\nGiven add behavior\n",
            &spec.test_cases,
        );

        assert_eq!(report.readiness, BddReadiness::Blocked);
        assert_eq!(report.scenario_count(), 0);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "E_BDD_SCENARIO_INCOMPLETE")
        );
    }

    #[test]
    fn and_but_inherit_previous_step_kind() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_feature(
            tmp.path(),
            "calculator",
            "features/calculator.feature",
            "Feature: Calculator\n\n  Scenario: addition\n    Given add behavior\n    And positive i64 args\n    When add is called\n    But not divide\n    Then it returns a sum\n",
        );
        let spec = spec_with_bdd("features/calculator.feature");

        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        let steps = &report.feature_files[0].scenarios[0].steps;
        assert_eq!(steps[1].kind, BddStepKind::Given);
        assert_eq!(steps[3].kind, BddStepKind::When);
    }

    #[test]
    fn prompt_context_is_bounded() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_feature(
            tmp.path(),
            "calculator",
            "features/calculator.feature",
            "Feature: Calculator\n\n  Scenario: addition\n    Given add behavior\n    When add is called\n    Then it returns a sum\n\n  Scenario: subtraction\n    Given sub behavior\n    When sub is called\n    Then it returns a difference\n",
        );
        let spec = spec_with_bdd("features/calculator.feature");
        let report = load_bdd_report(&spec, tmp.path(), "calculator");

        let lines = render_bdd_prompt_context(&report, 1);

        assert!(lines.iter().any(|line| line.contains("addition")));
        assert!(lines.iter().any(|line| line.contains("more scenario")));
        assert!(!lines.iter().any(|line| line.contains("subtraction")));
    }

    #[test]
    fn default_feature_uses_standard_gherkin() {
        let spec = spec_with_bdd("features/calculator.feature");

        let feature = render_default_feature(&spec, "calculator");

        assert!(feature.contains("Feature:"));
        assert!(feature.contains("Scenario:"));
        assert!(feature.contains("Given "));
        assert!(feature.contains("When "));
        assert!(feature.contains("Then "));
    }

    #[test]
    fn default_feature_reference_uses_portable_forward_slashes() {
        assert_eq!(
            default_feature_reference("calculator"),
            "features/calculator.feature"
        );
    }
}
