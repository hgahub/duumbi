//! Schema-versioned replay evidence records.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bench::report::{BenchmarkEvidence, ErrorCategory, ProviderUsageSummary};

use super::metrics::{AgreementRate, largest_group_agreement};

/// Replay report schema version.
pub const REPLAY_REPORT_SCHEMA_VERSION: &str = "duumbi.determinism.replay_report.v1";

/// Append-only replay ledger event schema version.
pub const REPLAY_LEDGER_SCHEMA_VERSION: &str = "duumbi.determinism.ledger_event.v1";

/// Complete determinism replay report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayReport {
    /// Replay report schema version.
    pub schema_version: String,
    /// Stable run identifier.
    pub run_id: String,
    /// UTC start timestamp as RFC3339 text.
    pub started_at: String,
    /// UTC finish timestamp as RFC3339 text.
    pub finished_at: String,
    /// DUUMBI package version.
    pub duumbi_version: String,
    /// Source commit used for the replay run.
    pub source_commit: String,
    /// Locked replay inputs.
    pub inputs: ReplayInputs,
    /// Bounded local environment state.
    pub environment: ReplayEnvironment,
    /// Selected replay tasks.
    pub tasks: Vec<ReplayTask>,
    /// Per-attempt evidence.
    pub attempts: Vec<ReplayAttempt>,
    /// Aggregate equivalence metrics.
    pub metrics: ReplayMetrics,
    /// Rewrite comparison status.
    pub rewrite_comparison: RewriteComparison,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
}

impl ReplayReport {
    /// Creates an empty replay report with schema defaults.
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        started_at: impl Into<String>,
        finished_at: impl Into<String>,
        duumbi_version: impl Into<String>,
        source_commit: impl Into<String>,
        inputs: ReplayInputs,
        environment: ReplayEnvironment,
    ) -> Self {
        Self {
            schema_version: REPLAY_REPORT_SCHEMA_VERSION.to_string(),
            run_id: run_id.into(),
            started_at: started_at.into(),
            finished_at: finished_at.into(),
            duumbi_version: duumbi_version.into(),
            source_commit: source_commit.into(),
            inputs,
            environment,
            tasks: Vec::new(),
            attempts: Vec::new(),
            metrics: ReplayMetrics::default(),
            rewrite_comparison: RewriteComparison::not_yet_comparable(
                "no constrained LLM rewrite mutation strategy for this task",
            ),
            warnings: Vec::new(),
        }
    }

    /// Renders a compact deterministic Markdown summary for review evidence.
    #[must_use]
    pub fn to_markdown_summary(&self) -> String {
        let mut output = String::new();
        output.push_str("# DUUMBI Determinism Replay Report\n\n");
        output.push_str(&format!("- Run: {}\n", self.run_id));
        output.push_str(&format!("- Started: {}\n", self.started_at));
        output.push_str(&format!("- Finished: {}\n", self.finished_at));
        output.push_str(&format!("- DUUMBI version: {}\n", self.duumbi_version));
        output.push_str(&format!("- Source commit: {}\n", self.source_commit));
        output.push_str(&format!("- Suite: {}\n", self.inputs.suite));
        output.push_str(&format!("- Smoke: {}\n", self.inputs.smoke));
        output.push_str(&format!(
            "- Showcases: {}\n",
            self.inputs.showcases.join(", ")
        ));
        output.push_str(&format!(
            "- Providers: {}\n",
            self.inputs.providers.join(", ")
        ));
        output.push_str(&format!("- Attempts: {}\n\n", self.inputs.attempts));

        output.push_str("## Metrics\n\n");
        output.push_str("| Metric | Status | Detail |\n");
        output.push_str("| --- | --- | --- |\n");
        output.push_str(&metric_row(
            "Exact graph",
            &self.metrics.exact_graph_agreement_rate,
        ));
        output.push_str(&metric_row(
            "Semantic graph",
            &self.metrics.semantic_graph_agreement_rate,
        ));
        output.push_str(&metric_row(
            "Behavioral",
            &self.metrics.behavioral_agreement_rate,
        ));
        output.push_str(&metric_row(
            "Failure category",
            &self.metrics.failure_category_agreement_rate,
        ));

        output.push_str("\n## Attempts\n\n");
        output.push_str("| Task | Provider | Model | Attempt | Success | Tests | Error |\n");
        output.push_str("| --- | --- | --- | ---: | --- | ---: | --- |\n");
        for attempt in &self.attempts {
            let model = match &attempt.model_identity {
                ModelIdentity::Available { label } => label.as_str(),
                ModelIdentity::Unavailable { reason } => reason.as_str(),
            };
            let error = if let Some(code) = &attempt.dominant_error_code {
                code.clone()
            } else if let Some(category) = attempt.error_category {
                category.to_string()
            } else {
                "none".to_string()
            };
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} | {}/{} | {} |\n",
                attempt.task_id,
                attempt.provider,
                model,
                attempt.attempt,
                attempt.success,
                attempt.tests_passed,
                attempt.tests_total,
                error
            ));
        }

        output.push_str("\n## Rewrite Comparison\n\n");
        output.push_str(&format!(
            "- Status: {:?}\n- Reason: {}\n",
            self.rewrite_comparison.status, self.rewrite_comparison.reason
        ));
        if !self.warnings.is_empty() {
            output.push_str("\n## Warnings\n\n");
            for warning in &self.warnings {
                output.push_str(&format!("- {warning}\n"));
            }
        }
        output
    }
}

/// Replay input selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayInputs {
    /// Selected benchmark suite.
    pub suite: String,
    /// Whether smoke filtering was enabled.
    pub smoke: bool,
    /// Selected showcase names.
    pub showcases: Vec<String>,
    /// Requested provider routes.
    pub providers: Vec<String>,
    /// Attempts per selected task/provider pair.
    pub attempts: u32,
}

/// Replay environment hash evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayEnvironment {
    /// Source of provider configuration, such as `user`.
    pub provider_source: String,
    /// Combined local registry/dependency state hash.
    pub registry_state_hash: String,
    /// `.duumbi/deps.lock` hash or `absent`.
    pub lockfile_hash: String,
    /// `.duumbi/config.toml` hash or `absent`.
    pub workspace_dependency_config_hash: String,
}

/// Selected replay task metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayTask {
    /// Stable task identifier.
    pub task_id: String,
    /// Benchmark suite.
    pub suite: String,
    /// Task tags.
    pub tags: Vec<String>,
}

/// Evidence for the resolved model used by one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModelIdentity {
    /// Model identity was resolved.
    Available {
        /// Resolved model or catalog label.
        label: String,
    },
    /// Model identity was unavailable.
    Unavailable {
        /// Stable unavailable reason.
        reason: String,
    },
}

impl ModelIdentity {
    /// Creates unavailable model identity evidence.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }
}

/// Prompt hash evidence for one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PromptHashes {
    /// Full prompt hashes are available.
    Full {
        /// Final mutation prompt hash per task or step.
        hashes: BTreeMap<String, String>,
    },
    /// Only partial context-pack hashing is available.
    Partial {
        /// Stable partial-hashing reason.
        reason: String,
        /// Hashes that were available.
        hashes: BTreeMap<String, String>,
    },
    /// No prompt/context hash evidence is available.
    Unavailable {
        /// Stable unavailable reason.
        reason: String,
    },
}

/// One replay attempt evidence row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayAttempt {
    /// Stable task identifier.
    pub task_id: String,
    /// Benchmark suite.
    pub suite: String,
    /// Task tags.
    pub tags: Vec<String>,
    /// Requested provider route.
    pub provider: String,
    /// Resolved model identity evidence.
    pub model_identity: ModelIdentity,
    /// One-based attempt number.
    pub attempt: u32,
    /// Workspace isolation strategy.
    pub workspace_strategy: String,
    /// Initial exact graph digest.
    pub initial_graph_exact_hash: Option<String>,
    /// Initial semantic graph hash.
    pub initial_graph_semantic_hash: Option<String>,
    /// Final exact graph digest.
    pub final_graph_exact_hash: Option<String>,
    /// Final semantic graph hash.
    pub final_graph_semantic_hash: Option<String>,
    /// Intent specification hash.
    pub intent_spec_hash: Option<String>,
    /// BDD context hash.
    pub bdd_context_hash: Option<String>,
    /// Context pack hash.
    pub context_pack_hash: Option<String>,
    /// Prompt hash evidence.
    pub prompt_hashes: PromptHashes,
    /// Whether the attempt succeeded.
    pub success: bool,
    /// Number of tests passed.
    pub tests_passed: usize,
    /// Number of tests total.
    pub tests_total: usize,
    /// BDD readiness label.
    pub bdd_readiness: Option<String>,
    /// BDD coverage labels.
    pub bdd_coverage: Vec<String>,
    /// Behavior signature hash or key.
    pub behavior_signature: Option<String>,
    /// Failure category.
    pub error_category: Option<ErrorCategory>,
    /// Dominant error code or signal.
    pub dominant_error_code: Option<String>,
    /// Provider usage evidence.
    pub provider_usage: ProviderUsageSummary,
    /// Additional benchmark evidence.
    pub benchmark_evidence: Option<BenchmarkEvidence>,
    /// Relative artifact paths retained for this attempt.
    pub artifact_paths: Vec<String>,
    /// Wall-clock duration in seconds.
    pub duration_secs: f64,
}

/// Aggregate replay metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayMetrics {
    /// Total selected attempts.
    pub attempts_total: u32,
    /// Attempts that reached terminal attempt evidence.
    pub attempts_completed: u32,
    /// Exact graph agreement.
    pub exact_graph_agreement_rate: AgreementRate,
    /// Semantic graph agreement.
    pub semantic_graph_agreement_rate: AgreementRate,
    /// Behavior agreement.
    pub behavioral_agreement_rate: AgreementRate,
    /// Failure category agreement.
    pub failure_category_agreement_rate: AgreementRate,
    /// Divergence examples keyed by metric or task.
    pub divergence_examples: Vec<DivergenceExample>,
}

impl Default for ReplayMetrics {
    fn default() -> Self {
        Self {
            attempts_total: 0,
            attempts_completed: 0,
            exact_graph_agreement_rate: AgreementRate::Unavailable {
                reason: "no comparable attempts produced final graph evidence".to_string(),
                comparable_attempt_count: 0,
            },
            semantic_graph_agreement_rate: AgreementRate::Unavailable {
                reason: "no comparable attempts produced semantic graph evidence".to_string(),
                comparable_attempt_count: 0,
            },
            behavioral_agreement_rate: AgreementRate::Unavailable {
                reason: "no comparable attempts produced behavior evidence".to_string(),
                comparable_attempt_count: 0,
            },
            failure_category_agreement_rate: AgreementRate::Unavailable {
                reason: "no comparable attempts produced failure category evidence".to_string(),
                comparable_attempt_count: 0,
            },
            divergence_examples: Vec::new(),
        }
    }
}

impl ReplayMetrics {
    /// Derives aggregate metrics from attempt evidence.
    #[must_use]
    pub fn from_attempts(attempts: &[ReplayAttempt]) -> Self {
        Self {
            attempts_total: attempts.len() as u32,
            attempts_completed: attempts.len() as u32,
            exact_graph_agreement_rate: largest_group_agreement(
                attempts
                    .iter()
                    .map(|attempt| attempt.final_graph_exact_hash.as_deref()),
                "no comparable attempts produced final graph evidence",
            ),
            semantic_graph_agreement_rate: largest_group_agreement(
                attempts
                    .iter()
                    .map(|attempt| attempt.final_graph_semantic_hash.as_deref()),
                "no comparable attempts produced semantic graph evidence",
            ),
            behavioral_agreement_rate: largest_group_agreement(
                attempts
                    .iter()
                    .map(|attempt| attempt.behavior_signature.as_deref()),
                "no comparable attempts produced behavior evidence",
            ),
            failure_category_agreement_rate: largest_group_agreement(
                attempts
                    .iter()
                    .map(|attempt| attempt.error_category.map(|category| category.to_string())),
                "no comparable attempts produced failure category evidence",
            ),
            divergence_examples: Vec::new(),
        }
    }
}

/// One divergence example for human inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceExample {
    /// Task identifier.
    pub task_id: String,
    /// Provider route.
    pub provider: String,
    /// Attempt numbers involved in the divergence.
    pub attempts: Vec<u32>,
    /// Divergence kind.
    pub kind: String,
    /// Human-readable detail.
    pub detail: String,
}

/// Rewrite comparison status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewriteComparisonStatus {
    /// Rewrite comparison is not relevant for the selected task.
    NotApplicable,
    /// No comparable constrained rewrite mutation strategy exists yet.
    NotYetComparable,
    /// Only metadata can be reported.
    MetadataOnly,
    /// Same-task comparison evidence exists.
    Comparable,
}

/// Rewrite comparison evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewriteComparison {
    /// Comparison status.
    pub status: RewriteComparisonStatus,
    /// Stable reason or summary.
    pub reason: String,
}

impl RewriteComparison {
    /// Creates a conservative not-yet-comparable rewrite comparison.
    #[must_use]
    pub fn not_yet_comparable(reason: impl Into<String>) -> Self {
        Self {
            status: RewriteComparisonStatus::NotYetComparable,
            reason: reason.into(),
        }
    }
}

/// Append-only replay ledger event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEvent {
    /// Ledger event schema version.
    pub schema_version: String,
    /// Stable run identifier.
    pub run_id: String,
    /// Event kind.
    pub event: LedgerEventKind,
    /// Monotonic event sequence.
    pub sequence: u64,
    /// UTC event timestamp as RFC3339 text.
    pub timestamp: String,
    /// Task identifier, when applicable.
    pub task_id: Option<String>,
    /// Provider route, when applicable.
    pub provider: Option<String>,
    /// Attempt number, when applicable.
    pub attempt: Option<u32>,
    /// Event payload.
    pub payload: serde_json::Value,
}

impl LedgerEvent {
    /// Creates a ledger event with the v1 schema version.
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        event: LedgerEventKind,
        sequence: u64,
        timestamp: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            schema_version: REPLAY_LEDGER_SCHEMA_VERSION.to_string(),
            run_id: run_id.into(),
            event,
            sequence,
            timestamp: timestamp.into(),
            task_id: None,
            provider: None,
            attempt: None,
            payload,
        }
    }
}

/// Replay ledger event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEventKind {
    /// Run started.
    RunStarted,
    /// Task was selected.
    TaskSelected,
    /// Attempt started.
    AttemptStarted,
    /// Reserved for future context lock emission once replay context capture
    /// can write the ledger at the exact lock point.
    ContextLocked,
    /// Attempt completed.
    AttemptCompleted,
    /// Attempt failed.
    AttemptFailed,
    /// Run completed.
    RunCompleted,
    /// Reserved for future interrupted-run recovery. The current runner
    /// returns errors before producing a partial run ledger.
    RunInterrupted,
}

fn metric_row(name: &str, metric: &AgreementRate) -> String {
    match metric {
        AgreementRate::Available {
            rate,
            comparable_attempt_count,
            largest_equivalence_group_count,
            dominant_key,
        } => format!(
            "| {name} | available | rate {:.3}; {largest_equivalence_group_count}/{comparable_attempt_count}; dominant `{dominant_key}` |\n",
            rate
        ),
        AgreementRate::Unavailable {
            reason,
            comparable_attempt_count,
        } => format!(
            "| {name} | unavailable | {reason}; comparable attempts {comparable_attempt_count} |\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_serializes_schema_and_rewrite_status() {
        let report = ReplayReport::new(
            "run-1",
            "2026-06-17T00:00:00Z",
            "2026-06-17T00:00:01Z",
            "0.4.0-preview",
            "abc123",
            ReplayInputs {
                suite: "core".to_string(),
                smoke: false,
                showcases: vec!["calculator".to_string()],
                providers: vec!["minimax:auto:primary:MINIMAX_API_KEY".to_string()],
                attempts: 2,
            },
            ReplayEnvironment {
                provider_source: "user".to_string(),
                registry_state_hash: "state".to_string(),
                lockfile_hash: "absent".to_string(),
                workspace_dependency_config_hash: "absent".to_string(),
            },
        );

        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(json.contains("\"schema_version\":\"duumbi.determinism.replay_report.v1\""));
        assert!(json.contains("\"status\":\"not_yet_comparable\""));
        assert!(!json.contains("api_key"));
    }

    #[test]
    fn model_identity_unavailable_is_explicit() {
        let model = ModelIdentity::unavailable("provider did not expose resolved model");

        let json = serde_json::to_string(&model).expect("model identity serializes");
        assert_eq!(
            json,
            r#"{"status":"unavailable","reason":"provider did not expose resolved model"}"#
        );
    }

    #[test]
    fn ledger_event_uses_schema_version() {
        let event = LedgerEvent::new(
            "run-1",
            LedgerEventKind::RunStarted,
            1,
            "2026-06-17T00:00:00Z",
            serde_json::json!({"ok": true}),
        );

        assert_eq!(event.schema_version, REPLAY_LEDGER_SCHEMA_VERSION);
        let json = serde_json::to_string(&event).expect("ledger event serializes");
        assert!(json.contains("\"event\":\"run_started\""));
    }

    #[test]
    fn metrics_derive_from_attempt_rows() {
        let attempts = vec![
            ReplayAttempt {
                task_id: "calculator".to_string(),
                suite: "core".to_string(),
                tags: Vec::new(),
                provider: "mock".to_string(),
                model_identity: ModelIdentity::unavailable("not exposed"),
                attempt: 1,
                workspace_strategy: "tempdir".to_string(),
                initial_graph_exact_hash: None,
                initial_graph_semantic_hash: None,
                final_graph_exact_hash: Some("exact-a".to_string()),
                final_graph_semantic_hash: Some("semantic-a".to_string()),
                intent_spec_hash: None,
                bdd_context_hash: None,
                context_pack_hash: None,
                prompt_hashes: PromptHashes::Unavailable {
                    reason: "not captured".to_string(),
                },
                success: true,
                tests_passed: 1,
                tests_total: 1,
                bdd_readiness: None,
                bdd_coverage: Vec::new(),
                behavior_signature: Some("behavior-a".to_string()),
                error_category: None,
                dominant_error_code: None,
                provider_usage: ProviderUsageSummary::unavailable("not exposed"),
                benchmark_evidence: None,
                artifact_paths: Vec::new(),
                duration_secs: 0.1,
            },
            ReplayAttempt {
                task_id: "calculator".to_string(),
                suite: "core".to_string(),
                tags: Vec::new(),
                provider: "mock".to_string(),
                model_identity: ModelIdentity::unavailable("not exposed"),
                attempt: 2,
                workspace_strategy: "tempdir".to_string(),
                initial_graph_exact_hash: None,
                initial_graph_semantic_hash: None,
                final_graph_exact_hash: Some("exact-b".to_string()),
                final_graph_semantic_hash: Some("semantic-a".to_string()),
                intent_spec_hash: None,
                bdd_context_hash: None,
                context_pack_hash: None,
                prompt_hashes: PromptHashes::Unavailable {
                    reason: "not captured".to_string(),
                },
                success: true,
                tests_passed: 1,
                tests_total: 1,
                bdd_readiness: None,
                bdd_coverage: Vec::new(),
                behavior_signature: Some("behavior-a".to_string()),
                error_category: None,
                dominant_error_code: None,
                provider_usage: ProviderUsageSummary::unavailable("not exposed"),
                benchmark_evidence: None,
                artifact_paths: Vec::new(),
                duration_secs: 0.1,
            },
        ];

        let metrics = ReplayMetrics::from_attempts(&attempts);

        assert_eq!(metrics.attempts_total, 2);
        assert!(matches!(
            metrics.exact_graph_agreement_rate,
            AgreementRate::Available { rate, .. } if rate == 0.5
        ));
        assert!(matches!(
            metrics.semantic_graph_agreement_rate,
            AgreementRate::Available { rate, .. } if rate == 1.0
        ));
    }

    #[test]
    fn markdown_summary_contains_metric_table() {
        let mut report = ReplayReport::new(
            "run-1",
            "2026-06-17T00:00:00Z",
            "2026-06-17T00:00:01Z",
            "0.4.0-preview",
            "abc123",
            ReplayInputs {
                suite: "core".to_string(),
                smoke: false,
                showcases: vec!["calculator".to_string()],
                providers: vec!["mock".to_string()],
                attempts: 2,
            },
            ReplayEnvironment {
                provider_source: "test".to_string(),
                registry_state_hash: "state".to_string(),
                lockfile_hash: "absent".to_string(),
                workspace_dependency_config_hash: "absent".to_string(),
            },
        );
        report.metrics = ReplayMetrics::from_attempts(&[]);

        let markdown = report.to_markdown_summary();

        assert!(markdown.contains("# DUUMBI Determinism Replay Report"));
        assert!(markdown.contains("| Exact graph | unavailable |"));
    }
}
