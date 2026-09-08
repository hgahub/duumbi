//! Deterministic root-cause classification for scaled write-path attempts.
//!
//! Classification uses **terminal** phase evidence only. Recovered events are
//! stored on the attempt but never select `root_cause`.

use serde::{Deserialize, Serialize};

use crate::bench::report::ErrorCategory;
use crate::intent::execute::{CapturedDiagnostic, ExecutionPhaseEvent, PhaseEventStatus};

/// Fine-grained root-cause class for a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootCauseClass {
    /// Schema or graph validation failure (`E009` and schema messages).
    SchemaGraphValidation,
    /// Missing function or failed task decomposition before a graph exists.
    TaskDecompositionOrMissingFunction,
    /// Unresolved export/import/call (`E010`).
    CrossModuleResolution,
    /// SSA dominance, forward reference, or backward-branch construction.
    ControlFlowOrSsa,
    /// Compiler, linker, or runtime crash (`E008`, cranelift, signal).
    CompilerOrRuntime,
    /// Verifier ran applicable checks and tests failed with no diagnostic code.
    ProductLogicMismatch,
    /// Verifier or process checker cannot judge the claimed behavior.
    VerifierMismatchOrUnsupportedEvidence,
    /// Provider, auth, rate-limit, timeout, or missing credentials.
    ProviderOrInfrastructure,
    /// No rule matched.
    Unknown,
}

/// Whether a rule matched or classification fell through to `unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootCauseAttribution {
    /// A deterministic rule selected the class.
    MatchedRule,
    /// No rule matched; class is `unknown`.
    NoMatchingRule,
}

/// Result of [`classify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    /// Failure class, omitted on success.
    pub root_cause: Option<RootCauseClass>,
    /// Attribution, omitted on success.
    pub attribution: Option<RootCauseAttribution>,
    /// Coarse compatibility category. `None` serializes as JSON `null` on failures.
    pub error_category: Option<ErrorCategory>,
}

/// Inputs for deterministic classification.
#[derive(Debug, Clone)]
pub struct ClassifyInput<'a> {
    /// Whether the attempt succeeded.
    pub success: bool,
    /// Ordered phase events from execute.
    pub events: &'a [ExecutionPhaseEvent],
    /// Captured diagnostics.
    pub diagnostics: &'a [CapturedDiagnostic],
    /// Tests that passed.
    pub tests_passed: usize,
    /// Tests selected.
    pub tests_total: usize,
    /// Terminal execute status label.
    pub terminal_status: &'a str,
    /// Sanitized log/haystack used only as a diagnostic supplement.
    pub log_text: &'a str,
    /// Process-evidence status when present (`passed`/`failed`/`unsupported`).
    pub process_evidence_status: Option<&'a str>,
    /// Whether a graph compiled or otherwise produced a runnable artifact.
    pub graph_compiled: bool,
    /// Whether credentials were missing for the provider route.
    pub credentials_missing: bool,
}

impl Classification {
    fn success() -> Self {
        Self {
            root_cause: None,
            attribution: None,
            error_category: None,
        }
    }

    fn matched(class: RootCauseClass) -> Self {
        Self {
            root_cause: Some(class),
            attribution: Some(RootCauseAttribution::MatchedRule),
            error_category: error_category_for(class),
        }
    }

    fn unknown() -> Self {
        Self {
            root_cause: Some(RootCauseClass::Unknown),
            attribution: Some(RootCauseAttribution::NoMatchingRule),
            error_category: None,
        }
    }
}

/// Maps a root-cause class onto the locked coarse [`ErrorCategory`].
#[must_use]
pub fn error_category_for(class: RootCauseClass) -> Option<ErrorCategory> {
    match class {
        RootCauseClass::SchemaGraphValidation | RootCauseClass::ControlFlowOrSsa => {
            Some(ErrorCategory::SchemaError)
        }
        RootCauseClass::TaskDecompositionOrMissingFunction
        | RootCauseClass::CrossModuleResolution => Some(ErrorCategory::MutationFailed),
        RootCauseClass::CompilerOrRuntime => Some(ErrorCategory::Crash),
        RootCauseClass::ProductLogicMismatch => Some(ErrorCategory::LogicError),
        RootCauseClass::VerifierMismatchOrUnsupportedEvidence => {
            Some(ErrorCategory::EvidenceRequired)
        }
        RootCauseClass::ProviderOrInfrastructure => Some(ErrorCategory::ProviderError),
        RootCauseClass::Unknown => None,
    }
}

/// Returns `true` when the row should increment graph-failure totals.
#[must_use]
pub fn counts_as_graph_failure(classification: &Classification, credentials_missing: bool) -> bool {
    if credentials_missing {
        return false;
    }
    match classification.root_cause {
        None | Some(RootCauseClass::ProviderOrInfrastructure) => false,
        Some(_) => true,
    }
}

/// Classifies an attempt from terminal phase evidence.
#[must_use]
pub fn classify(input: &ClassifyInput<'_>) -> Classification {
    if input.success {
        return Classification::success();
    }

    let haystack = terminal_haystack(input);

    // 1. Provider / auth / rate-limit / timeout / missing credentials
    if input.credentials_missing || matches_provider(&haystack, input.events, input.terminal_status)
    {
        return Classification::matched(RootCauseClass::ProviderOrInfrastructure);
    }

    // 2. Process-evidence gap without execution
    if process_evidence_gap_without_execution(input) {
        return Classification::matched(RootCauseClass::VerifierMismatchOrUnsupportedEvidence);
    }

    // 3. Schema / graph validation (E009), excluding SSA rules below
    if has_schema_validation(&haystack) && !has_control_flow(&haystack) {
        return Classification::matched(RootCauseClass::SchemaGraphValidation);
    }

    // 4. Cross-module export/import/call
    if has_cross_module(&haystack) {
        return Classification::matched(RootCauseClass::CrossModuleResolution);
    }

    // 5. SSA / control-flow
    if has_control_flow(&haystack) {
        return Classification::matched(RootCauseClass::ControlFlowOrSsa);
    }

    // 6. Compiler / linker / runtime
    if has_compiler_runtime(&haystack) {
        return Classification::matched(RootCauseClass::CompilerOrRuntime);
    }

    // 7. Missing function / failed decomposition before a graph exists
    if has_decomposition(&haystack, input) {
        return Classification::matched(RootCauseClass::TaskDecompositionOrMissingFunction);
    }

    // 8. Verifier cannot judge claimed behavior (before product logic)
    if verifier_cannot_judge(input, &haystack) {
        return Classification::matched(RootCauseClass::VerifierMismatchOrUnsupportedEvidence);
    }

    // 9. Product logic: applicable verifier checks, compiled graph, tests failed
    if product_logic_applies(input, &haystack) {
        return Classification::matched(RootCauseClass::ProductLogicMismatch);
    }

    Classification::unknown()
}

fn terminal_haystack(input: &ClassifyInput<'_>) -> String {
    let mut parts = Vec::new();
    for event in input.events.iter().filter(|event| {
        matches!(
            event.status,
            PhaseEventStatus::Terminal | PhaseEventStatus::Informational
        )
    }) {
        if event.status == PhaseEventStatus::Informational && event.phase != "complete" {
            continue;
        }
        if event.status != PhaseEventStatus::Terminal && event.phase != "complete" {
            continue;
        }
        parts.push(event.phase.clone());
        if let Some(code) = &event.error_code {
            parts.push(code.clone());
        }
    }
    for event in input
        .events
        .iter()
        .filter(|event| event.status == PhaseEventStatus::Terminal)
    {
        parts.push(event.phase.clone());
        if let Some(code) = &event.error_code {
            parts.push(code.clone());
        }
    }
    for diagnostic in input.diagnostics {
        if let Some(code) = &diagnostic.code {
            parts.push(code.clone());
        }
        parts.push(diagnostic.message.clone());
    }
    parts.push(input.terminal_status.to_string());
    parts.push(input.log_text.to_string());
    parts.join("\n").to_ascii_lowercase()
}

fn terminal_codes(input: &ClassifyInput<'_>) -> Vec<String> {
    let mut codes = Vec::new();
    for event in input
        .events
        .iter()
        .filter(|event| event.status == PhaseEventStatus::Terminal)
    {
        if let Some(code) = &event.error_code {
            codes.push(code.to_ascii_lowercase());
        }
    }
    for diagnostic in input.diagnostics {
        if let Some(code) = &diagnostic.code {
            codes.push(code.to_ascii_lowercase());
        }
        codes.push(diagnostic.message.to_ascii_lowercase());
    }
    codes
}

fn recovered_only_provider(input: &ClassifyInput<'_>) -> bool {
    let recovered_provider = input.events.iter().any(|event| {
        event.status == PhaseEventStatus::Recovered
            && (event.phase == "provider"
                || event
                    .error_code
                    .as_deref()
                    .is_some_and(|code| is_provider_token(&code.to_ascii_lowercase())))
    });
    let terminal_provider = input.events.iter().any(|event| {
        event.status == PhaseEventStatus::Terminal
            && (event.phase == "provider"
                || event
                    .error_code
                    .as_deref()
                    .is_some_and(|code| is_provider_token(&code.to_ascii_lowercase())))
    });
    recovered_provider && !terminal_provider
}

fn matches_provider(haystack: &str, events: &[ExecutionPhaseEvent], terminal_status: &str) -> bool {
    if recovered_only_provider(&ClassifyInput {
        success: false,
        events,
        diagnostics: &[],
        tests_passed: 0,
        tests_total: 0,
        terminal_status,
        log_text: "",
        process_evidence_status: None,
        graph_compiled: false,
        credentials_missing: false,
    }) {
        return false;
    }
    let terminal = events
        .iter()
        .filter(|event| event.status == PhaseEventStatus::Terminal)
        .any(|event| {
            event.phase == "provider"
                || event.phase == "preflight"
                    && event.error_code.as_deref() == Some("preflight_blocked")
                || event
                    .error_code
                    .as_deref()
                    .is_some_and(|code| is_provider_token(&code.to_ascii_lowercase()))
        });
    terminal
        || is_provider_token(haystack)
        || terminal_status.contains("provider")
        || haystack.contains("missing credential")
        || haystack.contains("api key")
}

fn is_provider_token(text: &str) -> bool {
    text.contains("provider_timeout")
        || text.contains("provider_auth")
        || text.contains("provider_rate_limit")
        || text.contains("provider_server_error")
        || text.contains("rate limit")
        || text.contains("rate_limit")
        || text.contains("status 401")
        || text.contains("status 403")
        || text.contains("status 429")
        || text.contains("timed out")
        || text.contains("timeout")
        || text.contains("missing credential")
        || text.contains("credentials")
}

fn process_evidence_gap_without_execution(input: &ClassifyInput<'_>) -> bool {
    let status = input.process_evidence_status.unwrap_or("");
    (status == "broader_evidence_required" || status == "unsupported")
        && !input.graph_compiled
        && input
            .events
            .iter()
            .all(|event| event.phase != "mutation" || event.status != PhaseEventStatus::Terminal)
        && (input.terminal_status == "process_evidence_required"
            || status == "broader_evidence_required"
            || input.log_text.contains("broader_evidence_required"))
}

fn has_schema_validation(haystack: &str) -> bool {
    haystack.contains("e009")
        || (haystack.contains("schema") && !haystack.contains("schema_error_ignored"))
}

fn has_cross_module(haystack: &str) -> bool {
    haystack.contains("e010")
        || haystack.contains("unresolved")
        || haystack.contains("missing export")
        || haystack.contains("unresolved call")
}

fn has_control_flow(haystack: &str) -> bool {
    haystack.contains("dominance")
        || haystack.contains("forward reference")
        || haystack.contains("forward-reference")
        || haystack.contains("backward-branch")
        || haystack.contains("backward branch")
        || haystack.contains("ssa")
}

fn has_compiler_runtime(haystack: &str) -> bool {
    haystack.contains("e008")
        || haystack.contains("cranelift")
        || haystack.contains("link failed")
        || haystack.contains("segfault")
        || haystack.contains("signal")
        || haystack.contains("write obj")
        || haystack.contains("compile")
}

fn has_decomposition(haystack: &str, input: &ClassifyInput<'_>) -> bool {
    let mutation_terminal = input
        .events
        .iter()
        .any(|event| event.phase == "mutation" && event.status == PhaseEventStatus::Terminal);
    mutation_terminal
        && (haystack.contains("missing function")
            || haystack.contains("clarification_needed")
            || haystack.contains("no_tool_calls")
            || haystack.contains("mutation_failed")
            || haystack.contains("task_decomposition")
            || input.terminal_status == "mutation_failed"
            || input.terminal_status == "clarification_needed")
}

fn verifier_cannot_judge(input: &ClassifyInput<'_>, haystack: &str) -> bool {
    let unsupported = haystack.contains("unsupported")
        || haystack.contains("broader_evidence_required")
        || haystack.contains("verification-gap")
        || haystack.contains("verification gap")
        || input.process_evidence_status == Some("unsupported")
        || input.process_evidence_status == Some("broader_evidence_required");
    let verifier_did_not_run = input.tests_total == 0
        && input.process_evidence_status.is_none()
        && haystack.contains("verifier did not run");
    unsupported || verifier_did_not_run
}

fn product_logic_applies(input: &ClassifyInput<'_>, haystack: &str) -> bool {
    let mutation_terminal = input
        .events
        .iter()
        .any(|event| event.phase == "mutation" && event.status == PhaseEventStatus::Terminal);
    if mutation_terminal {
        return false;
    }
    let process_ok = matches!(
        input.process_evidence_status,
        Some("passed") | Some("failed") | None
    );
    let applicable = input.graph_compiled
        && (input.tests_total > 0
            || matches!(
                input.process_evidence_status,
                Some("passed") | Some("failed")
            ));
    let no_unsupported = input.process_evidence_status != Some("unsupported")
        && !haystack.contains("unsupported")
        && !haystack.contains("broader_evidence_required");
    let no_diag_code = !terminal_codes(input).iter().any(|code| {
        code.contains("e009")
            || code.contains("e010")
            || code.contains("e008")
            || code.contains("e001")
    });
    applicable
        && process_ok
        && no_unsupported
        && no_diag_code
        && input.tests_passed < input.tests_total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::execute::{CapturedDiagnostic, ExecutionPhaseEvent, PhaseEventStatus};

    fn event(phase: &str, status: PhaseEventStatus, code: Option<&str>) -> ExecutionPhaseEvent {
        ExecutionPhaseEvent {
            phase: phase.to_string(),
            status,
            error_code: code.map(str::to_string),
        }
    }

    fn classify_events(
        events: &[ExecutionPhaseEvent],
        diagnostics: &[CapturedDiagnostic],
        extra: impl Fn(&mut ClassifyInput<'_>),
    ) -> Classification {
        let mut input = ClassifyInput {
            success: false,
            events,
            diagnostics,
            tests_passed: 0,
            tests_total: 4,
            terminal_status: "verifier_failed",
            log_text: "",
            process_evidence_status: None,
            graph_compiled: true,
            credentials_missing: false,
        };
        extra(&mut input);
        classify(&input)
    }

    #[test]
    fn success_omits_root_cause() {
        let input = ClassifyInput {
            success: true,
            events: &[],
            diagnostics: &[],
            tests_passed: 4,
            tests_total: 4,
            terminal_status: "completed",
            log_text: "",
            process_evidence_status: None,
            graph_compiled: true,
            credentials_missing: false,
        };
        let class = classify(&input);
        assert_eq!(class.root_cause, None);
        assert_eq!(class.attribution, None);
        assert_eq!(class.error_category, None);
    }

    #[test]
    fn recovered_provider_timeout_does_not_win_over_terminal_e009() {
        let events = vec![
            event("provider", PhaseEventStatus::Recovered, Some("timeout")),
            event("validate", PhaseEventStatus::Terminal, Some("E009")),
        ];
        let class = classify_events(&events, &[], |_| {});
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::SchemaGraphValidation)
        );
        assert_eq!(class.error_category, Some(ErrorCategory::SchemaError));
        assert_eq!(class.attribution, Some(RootCauseAttribution::MatchedRule));
    }

    #[test]
    fn overlapping_e009_and_later_test_failure_keeps_schema() {
        let events = vec![
            event("validate", PhaseEventStatus::Terminal, Some("E009")),
            event("verify", PhaseEventStatus::Informational, None),
        ];
        let diagnostics = vec![CapturedDiagnostic {
            code: None,
            message: "expected 8 but got 7".to_string(),
        }];
        let class = classify_events(&events, &diagnostics, |_| {});
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::SchemaGraphValidation)
        );
    }

    #[test]
    fn control_flow_maps_to_schema_error() {
        let events = vec![event(
            "validate",
            PhaseEventStatus::Terminal,
            Some("ssa dominance"),
        )];
        let class = classify_events(&events, &[], |_| {});
        assert_eq!(class.root_cause, Some(RootCauseClass::ControlFlowOrSsa));
        assert_eq!(class.error_category, Some(ErrorCategory::SchemaError));
    }

    #[test]
    fn cross_module_maps_to_mutation_failed() {
        let events = vec![event("validate", PhaseEventStatus::Terminal, Some("E010"))];
        let class = classify_events(&events, &[], |_| {});
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::CrossModuleResolution)
        );
        assert_eq!(class.error_category, Some(ErrorCategory::MutationFailed));
    }

    #[test]
    fn compiler_runtime_maps_to_crash() {
        let events = vec![event("compile", PhaseEventStatus::Terminal, Some("E008"))];
        let class = classify_events(&events, &[], |_| {});
        assert_eq!(class.root_cause, Some(RootCauseClass::CompilerOrRuntime));
        assert_eq!(class.error_category, Some(ErrorCategory::Crash));
    }

    #[test]
    fn verifier_unsupported_beats_product_logic() {
        let events = vec![event("verify", PhaseEventStatus::Terminal, None)];
        let class = classify_events(&events, &[], |input| {
            input.process_evidence_status = Some("unsupported");
            input.graph_compiled = true;
            input.tests_total = 0;
        });
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::VerifierMismatchOrUnsupportedEvidence)
        );
        assert_eq!(class.error_category, Some(ErrorCategory::EvidenceRequired));
    }

    #[test]
    fn product_logic_requires_applicability() {
        let events = vec![event("verify", PhaseEventStatus::Terminal, None)];
        let class = classify_events(&events, &[], |input| {
            input.graph_compiled = true;
            input.tests_passed = 0;
            input.tests_total = 4;
            input.process_evidence_status = Some("failed");
        });
        assert_eq!(class.root_cause, Some(RootCauseClass::ProductLogicMismatch));
        assert_eq!(class.error_category, Some(ErrorCategory::LogicError));
    }

    #[test]
    fn unmatched_failure_is_unknown_with_null_category() {
        let events = vec![event("complete", PhaseEventStatus::Terminal, None)];
        let class = classify_events(&events, &[], |input| {
            input.graph_compiled = false;
            input.tests_total = 0;
            input.terminal_status = "complete";
            input.log_text = "mysterious";
        });
        assert_eq!(class.root_cause, Some(RootCauseClass::Unknown));
        assert_eq!(
            class.attribution,
            Some(RootCauseAttribution::NoMatchingRule)
        );
        assert_eq!(class.error_category, None);
    }

    #[test]
    fn missing_credentials_excluded_from_graph_failure() {
        let events = vec![event(
            "provider",
            PhaseEventStatus::Terminal,
            Some("provider_auth"),
        )];
        let class = classify_events(&events, &[], |input| {
            input.credentials_missing = true;
        });
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::ProviderOrInfrastructure)
        );
        assert!(!counts_as_graph_failure(&class, true));
        assert!(!counts_as_graph_failure(&class, false));
    }

    #[test]
    fn provider_timeout_rule() {
        let events = vec![event(
            "mutation",
            PhaseEventStatus::Terminal,
            Some("provider_timeout"),
        )];
        let class = classify_events(&events, &[], |_| {});
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::ProviderOrInfrastructure)
        );
        assert_eq!(class.error_category, Some(ErrorCategory::ProviderError));
    }

    #[test]
    fn decomposition_before_graph() {
        let events = vec![event(
            "mutation",
            PhaseEventStatus::Terminal,
            Some("no_tool_calls"),
        )];
        let class = classify_events(&events, &[], |input| {
            input.graph_compiled = false;
            input.terminal_status = "mutation_failed";
        });
        assert_eq!(
            class.root_cause,
            Some(RootCauseClass::TaskDecompositionOrMissingFunction)
        );
        assert_eq!(class.error_category, Some(ErrorCategory::MutationFailed));
    }
}
