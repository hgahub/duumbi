//! Schema-versioned replay evidence records.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bench::report::{BenchmarkEvidence, ErrorCategory, ProviderUsageSummary};

use super::metrics::AgreementRate;

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
    /// Context was locked.
    ContextLocked,
    /// Attempt completed.
    AttemptCompleted,
    /// Attempt failed.
    AttemptFailed,
    /// Run completed.
    RunCompleted,
    /// Run interrupted.
    RunInterrupted,
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
}
