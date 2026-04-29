//! Persistent model-selection telemetry for adaptive routing.
//!
//! The raw log is append-only for auditability. Aggregates are updated with
//! simple rolling statistics so routing policy can later bias toward models
//! that succeed for Duumbi-specific workloads.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const MODEL_PERFORMANCE_DIR: &str = ".duumbi/knowledge/model-performance";
const EVENTS_FILE: &str = "events.jsonl";
const AGGREGATES_FILE: &str = "aggregates.json";
const EWMA_ALPHA: f64 = 0.20;

/// Outcome of a model call after Duumbi validation has run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCallOutcome {
    /// Call produced an accepted result.
    Success,
    /// Provider call failed before a usable response.
    ProviderFailure,
    /// Tool response could not be parsed into patch operations.
    ParseFailure,
    /// Patch failed schema/type validation.
    ValidationFailure,
    /// Build or test verification failed.
    VerificationFailure,
    /// User rejected or reverted the result.
    Reverted,
}

/// One append-only model call telemetry event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCallEvent {
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Provider used.
    pub provider: String,
    /// Concrete model selected.
    pub model: String,
    /// Agent role, if known.
    pub agent_role: Option<String>,
    /// Template version, if known.
    pub template_version: Option<String>,
    /// Task type, if known.
    pub task_type: Option<String>,
    /// Complexity, if known.
    pub complexity: Option<String>,
    /// Scope, if known.
    pub scope: Option<String>,
    /// Risk, if known.
    pub risk: Option<String>,
    /// Prompt tokens, measured or estimated.
    pub prompt_tokens: Option<usize>,
    /// Completion tokens, measured or estimated.
    pub completion_tokens: Option<usize>,
    /// Reasoning tokens, if the provider reports them.
    pub reasoning_tokens: Option<usize>,
    /// End-to-end latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// First token latency in milliseconds.
    pub first_token_latency_ms: Option<u64>,
    /// Estimated cost in USD.
    pub cost_usd: Option<f64>,
    /// Whether tool-call parsing succeeded.
    pub tool_parse_success: bool,
    /// Patch operation count.
    pub patch_count: usize,
    /// Validation error codes.
    pub validation_errors: Vec<String>,
    /// Retry count before final outcome.
    pub retries: u32,
    /// Final outcome.
    pub outcome: ModelCallOutcome,
}

/// Rolling aggregate keyed by provider/model/agent/task dimensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAggregate {
    /// Total observed calls.
    pub calls: u64,
    /// Successful outcomes.
    pub successes: u64,
    /// Failed outcomes.
    pub failures: u64,
    /// Exponentially weighted moving average latency.
    pub ewma_latency_ms: Option<f64>,
    /// Exponentially weighted moving average cost.
    pub ewma_cost_usd: Option<f64>,
    /// Parse failures.
    pub parse_failures: u64,
    /// Validation failures.
    pub validation_failures: u64,
    /// Total retries.
    pub retries: u64,
    /// Last update timestamp.
    pub last_updated: Option<DateTime<Utc>>,
}

/// Complete aggregate file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPerformanceDb {
    /// Aggregates keyed by provider/model/profile tuple.
    pub aggregates: HashMap<String, ModelAggregate>,
}

/// Append event and update aggregate storage.
pub struct ModelPerformanceStore;

impl ModelPerformanceStore {
    /// Records a model-call event in the given workspace.
    pub fn record_event(workspace: &Path, event: &ModelCallEvent) -> std::io::Result<()> {
        let dir = workspace.join(MODEL_PERFORMANCE_DIR);
        fs::create_dir_all(&dir)?;

        let mut events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(EVENTS_FILE))?;
        let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
        writeln!(events, "{line}")?;

        let mut db = Self::load_db(workspace);
        let key = aggregate_key(event);
        update_aggregate(db.aggregates.entry(key).or_default(), event);
        let json = serde_json::to_string_pretty(&db).map_err(std::io::Error::other)?;
        fs::write(dir.join(AGGREGATES_FILE), json)?;

        Ok(())
    }

    /// Loads aggregate telemetry, returning an empty DB if no file exists.
    #[must_use]
    pub fn load_db(workspace: &Path) -> ModelPerformanceDb {
        let path = workspace.join(MODEL_PERFORMANCE_DIR).join(AGGREGATES_FILE);
        let Ok(content) = fs::read_to_string(path) else {
            return ModelPerformanceDb::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }
}

fn aggregate_key(event: &ModelCallEvent) -> String {
    [
        event.provider.as_str(),
        event.model.as_str(),
        event.agent_role.as_deref().unwrap_or("*"),
        event.task_type.as_deref().unwrap_or("*"),
        event.complexity.as_deref().unwrap_or("*"),
        event.scope.as_deref().unwrap_or("*"),
        event.risk.as_deref().unwrap_or("*"),
    ]
    .join("|")
}

fn update_aggregate(aggregate: &mut ModelAggregate, event: &ModelCallEvent) {
    aggregate.calls += 1;
    if event.outcome == ModelCallOutcome::Success {
        aggregate.successes += 1;
    } else {
        aggregate.failures += 1;
    }
    if event.outcome == ModelCallOutcome::ParseFailure {
        aggregate.parse_failures += 1;
    }
    if event.outcome == ModelCallOutcome::ValidationFailure {
        aggregate.validation_failures += 1;
    }
    aggregate.retries += u64::from(event.retries);
    aggregate.ewma_latency_ms = update_ewma(
        aggregate.ewma_latency_ms,
        event.latency_ms.map(|v| v as f64),
    );
    aggregate.ewma_cost_usd = update_ewma(aggregate.ewma_cost_usd, event.cost_usd);
    aggregate.last_updated = Some(event.timestamp);
}

fn update_ewma(current: Option<f64>, next: Option<f64>) -> Option<f64> {
    let next = next?;
    Some(match current {
        Some(current) => (EWMA_ALPHA * next) + ((1.0 - EWMA_ALPHA) * current),
        None => next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn event(outcome: ModelCallOutcome, latency_ms: u64) -> ModelCallEvent {
        ModelCallEvent {
            timestamp: Utc::now(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            agent_role: Some("coder".to_string()),
            template_version: Some("1.0.0".to_string()),
            task_type: Some("create".to_string()),
            complexity: Some("simple".to_string()),
            scope: Some("single_module".to_string()),
            risk: Some("low".to_string()),
            prompt_tokens: Some(100),
            completion_tokens: Some(20),
            reasoning_tokens: None,
            latency_ms: Some(latency_ms),
            first_token_latency_ms: None,
            cost_usd: Some(0.01),
            tool_parse_success: outcome != ModelCallOutcome::ParseFailure,
            patch_count: usize::from(outcome == ModelCallOutcome::Success),
            validation_errors: vec![],
            retries: 0,
            outcome,
        }
    }

    #[test]
    fn record_event_appends_log_and_updates_aggregates() {
        let temp = TempDir::new().expect("invariant: temp dir");

        ModelPerformanceStore::record_event(temp.path(), &event(ModelCallOutcome::Success, 100))
            .expect("first event must write");
        ModelPerformanceStore::record_event(
            temp.path(),
            &event(ModelCallOutcome::ParseFailure, 200),
        )
        .expect("second event must write");

        let log = fs::read_to_string(temp.path().join(MODEL_PERFORMANCE_DIR).join(EVENTS_FILE))
            .expect("events log must exist");
        assert_eq!(log.lines().count(), 2);

        let db = ModelPerformanceStore::load_db(temp.path());
        let aggregate = db.aggregates.values().next().expect("aggregate must exist");
        assert_eq!(aggregate.calls, 2);
        assert_eq!(aggregate.successes, 1);
        assert_eq!(aggregate.parse_failures, 1);
        assert_eq!(aggregate.ewma_latency_ms, Some(120.0));
    }

    #[test]
    fn project_roots_keep_separate_performance_aggregates() {
        let project_a = TempDir::new().expect("invariant: temp dir");
        let project_b = TempDir::new().expect("invariant: temp dir");

        ModelPerformanceStore::record_event(
            project_a.path(),
            &event(ModelCallOutcome::Success, 100),
        )
        .expect("project A event must write");
        ModelPerformanceStore::record_event(
            project_b.path(),
            &event(ModelCallOutcome::ParseFailure, 200),
        )
        .expect("project B event must write");

        let db_a = ModelPerformanceStore::load_db(project_a.path());
        let db_b = ModelPerformanceStore::load_db(project_b.path());
        let aggregate_a = db_a.aggregates.values().next().expect("A aggregate");
        let aggregate_b = db_b.aggregates.values().next().expect("B aggregate");

        assert_eq!(aggregate_a.successes, 1);
        assert_eq!(aggregate_a.failures, 0);
        assert_eq!(aggregate_b.successes, 0);
        assert_eq!(aggregate_b.failures, 1);
    }
}
