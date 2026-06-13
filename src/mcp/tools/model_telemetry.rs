//! Read-only MCP model telemetry analytics tools.
//!
//! These tools summarize the existing local model-access and model-performance
//! stores. They do not mutate telemetry, provider configuration, credentials,
//! the model catalog, graph files, or intents.

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::agents::model_access::{
    ModelAccessDb, ModelAccessEvent, ModelAccessRecord, ModelAccessStatus,
    model_access_current_path_for_home, model_access_events_path_for_home,
};
use crate::agents::model_performance::{
    ModelCallEvent, ModelPerformanceDb, model_performance_aggregates_path_for_workspace,
    model_performance_events_path_for_workspace,
};

const DEFAULT_STALE_AFTER_HOURS: u64 = 168;
const MAX_STALE_AFTER_HOURS: u64 = 8_760;
const DEFAULT_LIMIT: usize = 25;
const MAX_LIMIT: usize = 100;
const MAX_RAW_LIMIT: usize = 50;

/// Summarizes user-level model-access telemetry.
pub fn model_access_summary(_workspace: &Path, params: &Value) -> Result<Value, String> {
    let query = Query::parse(params, QueryKind::Access)?;
    let home = duumbi_home();
    model_access_summary_from_home(&home, &query)
}

/// Summarizes workspace model-performance telemetry.
pub fn model_performance_summary(workspace: &Path, params: &Value) -> Result<Value, String> {
    let query = Query::parse(params, QueryKind::Performance)?;
    model_performance_summary_from_workspace(workspace, &query)
}

/// Reports source health for local model telemetry stores.
pub fn model_telemetry_health(workspace: &Path, params: &Value) -> Result<Value, String> {
    let query = Query::parse(params, QueryKind::Health)?;
    let home = duumbi_home();
    Ok(serde_json::json!({
        "status": "success",
        "data_status": combined_health_status(&home, workspace, query.stale_after_hours),
        "generated_at": Utc::now(),
        "scope": "model_telemetry",
        "filters": normalized_filters(&query),
        "summary": {
            "model_access": access_health(&home, query.stale_after_hours),
            "model_performance": performance_health(workspace, query.stale_after_hours),
        },
        "rows": [],
        "raw_events": Value::Null,
        "privacy": privacy_statement(),
        "warnings": [],
    }))
}

#[derive(Clone, Copy)]
enum QueryKind {
    Access,
    Performance,
    Health,
}

#[derive(Debug, Default)]
struct Query {
    provider: Option<String>,
    model: Option<String>,
    agent_role: Option<String>,
    task_type: Option<String>,
    complexity: Option<String>,
    scope: Option<String>,
    risk: Option<String>,
    stale_after_hours: u64,
    limit: usize,
    include_raw_events: bool,
    explicit_limit: bool,
}

impl Query {
    fn parse(params: &Value, kind: QueryKind) -> Result<Self, String> {
        let object = params
            .as_object()
            .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;
        let mut query = Self {
            stale_after_hours: DEFAULT_STALE_AFTER_HOURS,
            limit: DEFAULT_LIMIT,
            ..Self::default()
        };

        for (key, value) in object {
            match key.as_str() {
                "provider" => query.provider = Some(string_field(key, value)?),
                "model" => query.model = Some(string_field(key, value)?),
                "agent_role" => {
                    ensure_performance_filter(kind, key)?;
                    query.agent_role = Some(string_field(key, value)?);
                }
                "task_type" => {
                    ensure_performance_filter(kind, key)?;
                    query.task_type = Some(string_field(key, value)?);
                }
                "complexity" => {
                    ensure_performance_filter(kind, key)?;
                    query.complexity = Some(string_field(key, value)?);
                }
                "scope" => {
                    ensure_performance_filter(kind, key)?;
                    query.scope = Some(string_field(key, value)?);
                }
                "risk" => {
                    ensure_performance_filter(kind, key)?;
                    query.risk = Some(string_field(key, value)?);
                }
                "stale_after_hours" => {
                    query.stale_after_hours = bounded_u64(key, value, 1, MAX_STALE_AFTER_HOURS)?;
                }
                "limit" => {
                    query.explicit_limit = true;
                    query.limit = bounded_usize(key, value, 1, MAX_LIMIT)?;
                }
                "include_raw_events" => {
                    query.include_raw_events = value
                        .as_bool()
                        .ok_or_else(|| "'include_raw_events' must be a boolean".to_string())?;
                }
                other => return Err(format!("Unsupported filter '{other}'")),
            }
        }

        if query.include_raw_events && matches!(kind, QueryKind::Health) {
            return Err("Raw event mode is not supported for 'model_telemetry_health'".to_string());
        }

        if query.include_raw_events {
            if !query.explicit_limit {
                return Err("Raw event mode requires an explicit 'limit'".to_string());
            }
            if query.limit > MAX_RAW_LIMIT {
                return Err(format!(
                    "Raw event mode limit must be at most {MAX_RAW_LIMIT}"
                ));
            }
        }

        Ok(query)
    }
}

fn ensure_performance_filter(kind: QueryKind, key: &str) -> Result<(), String> {
    if matches!(kind, QueryKind::Performance) {
        Ok(())
    } else {
        Err(format!(
            "Filter '{key}' is only supported for performance analytics"
        ))
    }
}

fn string_field(key: &str, value: &Value) -> Result<String, String> {
    let value = value
        .as_str()
        .ok_or_else(|| format!("'{key}' must be a string"))?
        .trim();
    if value.is_empty() {
        return Err(format!("'{key}' must not be empty"));
    }
    Ok(value.to_string())
}

fn bounded_u64(key: &str, value: &Value, min: u64, max: u64) -> Result<u64, String> {
    let number = value
        .as_u64()
        .ok_or_else(|| format!("'{key}' must be a positive integer"))?;
    if !(min..=max).contains(&number) {
        return Err(format!("'{key}' must be between {min} and {max}"));
    }
    Ok(number)
}

fn bounded_usize(key: &str, value: &Value, min: usize, max: usize) -> Result<usize, String> {
    let number = bounded_u64(key, value, min as u64, max as u64)?;
    usize::try_from(number).map_err(|_| format!("'{key}' is too large"))
}

enum StoreRead<T> {
    Absent,
    Empty,
    Present(T),
    Malformed(String),
    Unreadable(String),
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> StoreRead<T> {
    if !path.exists() {
        return StoreRead::Absent;
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => return StoreRead::Unreadable(error.kind().to_string()),
    };
    if content.trim().is_empty() {
        return StoreRead::Empty;
    }
    match serde_json::from_str(&content) {
        Ok(value) => StoreRead::Present(value),
        Err(error) => StoreRead::Malformed(format!("invalid JSON: {:?}", error.classify())),
    }
}

fn model_access_summary_from_home(home: &Path, query: &Query) -> Result<Value, String> {
    let path = model_access_current_path_for_home(home);
    let mut warnings = Vec::new();
    let mut rows = Vec::new();
    let mut data_status = "absent";
    let mut status = "success";

    match read_json_file::<ModelAccessDb>(&path) {
        StoreRead::Absent => {}
        StoreRead::Empty => data_status = "empty",
        StoreRead::Malformed(diagnostic) => {
            status = "error";
            data_status = "malformed";
            warnings.push(format!(
                "model-access current store is malformed: {diagnostic}"
            ));
        }
        StoreRead::Unreadable(diagnostic) => {
            status = "error";
            data_status = "partial";
            warnings.push(format!(
                "model-access current store is unreadable: {diagnostic}"
            ));
        }
        StoreRead::Present(db) => {
            if db.records.is_empty() {
                data_status = "empty";
            } else {
                let grouped = group_access_records(db.records.values(), query);
                let stale_count = grouped.iter().filter(|row| row.is_stale).count();
                data_status = if grouped.is_empty() {
                    "empty"
                } else if stale_count == grouped.len() {
                    "stale"
                } else if stale_count > 0 {
                    "partial"
                } else {
                    "present"
                };
                if stale_count > 0 {
                    warnings.push(
                        "stale model-access evidence is not current access proof".to_string(),
                    );
                }
                rows = grouped
                    .into_iter()
                    .take(query.limit)
                    .map(AccessSummaryRow::into_json)
                    .collect();
            }
        }
    }

    let raw_events = if query.include_raw_events {
        access_raw_events(home, query, &mut warnings)
    } else {
        Value::Null
    };

    Ok(analytics_response(AnalyticsResponseParts {
        status,
        data_status,
        scope: "user_home_model_access",
        query,
        summary: access_summary(&rows),
        rows,
        raw_events,
        warnings,
    }))
}

fn model_performance_summary_from_workspace(
    workspace: &Path,
    query: &Query,
) -> Result<Value, String> {
    let path = model_performance_aggregates_path_for_workspace(workspace);
    let mut warnings = Vec::new();
    let mut rows = Vec::new();
    let mut data_status = "absent";
    let mut status = "success";

    match read_json_file::<ModelPerformanceDb>(&path) {
        StoreRead::Absent => {}
        StoreRead::Empty => data_status = "empty",
        StoreRead::Malformed(diagnostic) => {
            status = "error";
            data_status = "malformed";
            warnings.push(format!(
                "model-performance aggregate store is malformed: {diagnostic}"
            ));
        }
        StoreRead::Unreadable(diagnostic) => {
            status = "error";
            data_status = "partial";
            warnings.push(format!(
                "model-performance aggregate store is unreadable: {diagnostic}"
            ));
        }
        StoreRead::Present(db) => {
            if db.aggregates.is_empty() {
                data_status = "empty";
            } else {
                let grouped = parse_performance_rows(&db, query, &mut warnings);
                let stale_count = grouped.iter().filter(|row| row.is_stale).count();
                data_status = if !warnings.is_empty() {
                    status = "partial";
                    "partial"
                } else if grouped.is_empty() {
                    "empty"
                } else if stale_count == grouped.len() {
                    "stale"
                } else if stale_count > 0 {
                    "partial"
                } else {
                    "present"
                };
                if stale_count > 0 {
                    warnings.push(
                        "stale model-performance evidence is not current routing evidence"
                            .to_string(),
                    );
                }
                rows = grouped
                    .into_iter()
                    .take(query.limit)
                    .map(PerformanceSummaryRow::into_json)
                    .collect();
            }
        }
    }

    let raw_events = if query.include_raw_events {
        performance_raw_events(workspace, query, &mut warnings)
    } else {
        Value::Null
    };

    Ok(analytics_response(AnalyticsResponseParts {
        status,
        data_status,
        scope: "workspace_model_performance",
        query,
        summary: performance_summary(&rows),
        rows,
        raw_events,
        warnings,
    }))
}

struct AccessSummaryRow {
    provider: String,
    model: String,
    record_count: usize,
    accessible: usize,
    denied: usize,
    auth_failed: usize,
    unknown: usize,
    latest_status: ModelAccessStatus,
    last_checked: DateTime<Utc>,
    last_success: Option<DateTime<Utc>>,
    is_stale: bool,
    stale_after_hours: u64,
}

impl AccessSummaryRow {
    fn into_json(self) -> Value {
        serde_json::json!({
            "provider": self.provider,
            "model": self.model,
            "record_count": self.record_count,
            "status_counts": {
                "accessible": self.accessible,
                "denied": self.denied,
                "auth_failed": self.auth_failed,
                "unknown": self.unknown,
            },
            "latest_status": self.latest_status,
            "last_checked": self.last_checked,
            "last_success": self.last_success,
            "is_stale": self.is_stale,
            "stale_after_hours": self.stale_after_hours,
        })
    }
}

fn group_access_records<'a>(
    records: impl Iterator<Item = &'a ModelAccessRecord>,
    query: &Query,
) -> Vec<AccessSummaryRow> {
    let mut grouped: BTreeMap<(String, String), Vec<&ModelAccessRecord>> = BTreeMap::new();
    for record in records {
        if !matches_filter(&query.provider, &record.provider)
            || !matches_filter(&query.model, &record.model)
        {
            continue;
        }
        grouped
            .entry((record.provider.clone(), record.model.clone()))
            .or_default()
            .push(record);
    }
    grouped
        .into_iter()
        .filter_map(|((provider, model), records)| {
            let latest = records.iter().max_by_key(|record| record.last_checked)?;
            let last_success = records
                .iter()
                .filter_map(|record| record.last_success)
                .max();
            Some(AccessSummaryRow {
                provider,
                model,
                record_count: records.len(),
                accessible: count_status(&records, ModelAccessStatus::Accessible),
                denied: count_status(&records, ModelAccessStatus::Denied),
                auth_failed: count_status(&records, ModelAccessStatus::AuthFailed),
                unknown: count_status(&records, ModelAccessStatus::Unknown),
                latest_status: latest.status,
                last_checked: latest.last_checked,
                last_success,
                is_stale: is_stale(latest.last_checked, query.stale_after_hours),
                stale_after_hours: query.stale_after_hours,
            })
        })
        .collect()
}

fn count_status(records: &[&ModelAccessRecord], status: ModelAccessStatus) -> usize {
    records
        .iter()
        .filter(|record| record.status == status)
        .count()
}

struct PerformanceSummaryRow {
    provider: String,
    model: String,
    agent_role: Option<String>,
    task_type: Option<String>,
    complexity: Option<String>,
    scope: Option<String>,
    risk: Option<String>,
    calls: u64,
    successes: u64,
    failures: u64,
    success_rate: Option<f64>,
    failure_rate: Option<f64>,
    parse_failures: u64,
    validation_failures: u64,
    retries: u64,
    ewma_latency_ms: Option<f64>,
    ewma_cost_usd: Option<f64>,
    last_updated: Option<DateTime<Utc>>,
    is_stale: bool,
}

impl PerformanceSummaryRow {
    fn into_json(self) -> Value {
        serde_json::json!({
            "provider": self.provider,
            "model": self.model,
            "agent_role": self.agent_role,
            "task_type": self.task_type,
            "complexity": self.complexity,
            "scope": self.scope,
            "risk": self.risk,
            "calls": self.calls,
            "successes": self.successes,
            "failures": self.failures,
            "success_rate": self.success_rate,
            "failure_rate": self.failure_rate,
            "parse_failures": self.parse_failures,
            "validation_failures": self.validation_failures,
            "retries": self.retries,
            "ewma_latency_ms": self.ewma_latency_ms,
            "ewma_cost_usd": self.ewma_cost_usd,
            "last_updated": self.last_updated,
            "is_stale": self.is_stale,
        })
    }
}

fn parse_performance_rows(
    db: &ModelPerformanceDb,
    query: &Query,
    warnings: &mut Vec<String>,
) -> Vec<PerformanceSummaryRow> {
    let mut rows = Vec::new();
    for (index, (key, aggregate)) in db.aggregates.iter().enumerate() {
        let parts: Vec<_> = key.split('|').collect();
        if parts.len() != 7 {
            warnings.push(format!("aggregate row {index} has an invalid profile key"));
            continue;
        }
        let row = PerformanceSummaryRow {
            provider: parts[0].to_string(),
            model: parts[1].to_string(),
            agent_role: profile_value(parts[2]),
            task_type: profile_value(parts[3]),
            complexity: profile_value(parts[4]),
            scope: profile_value(parts[5]),
            risk: profile_value(parts[6]),
            calls: aggregate.calls,
            successes: aggregate.successes,
            failures: aggregate.failures,
            success_rate: rate(aggregate.successes, aggregate.calls),
            failure_rate: rate(aggregate.failures, aggregate.calls),
            parse_failures: aggregate.parse_failures,
            validation_failures: aggregate.validation_failures,
            retries: aggregate.retries,
            ewma_latency_ms: aggregate.ewma_latency_ms,
            ewma_cost_usd: aggregate.ewma_cost_usd,
            last_updated: aggregate.last_updated,
            is_stale: aggregate
                .last_updated
                .map(|timestamp| is_stale(timestamp, query.stale_after_hours))
                .unwrap_or(true),
        };
        if performance_row_matches(&row, query) {
            rows.push(row);
        }
    }
    rows.sort_by(|left, right| {
        (
            &left.provider,
            &left.model,
            &left.agent_role,
            &left.task_type,
            &left.complexity,
            &left.scope,
            &left.risk,
        )
            .cmp(&(
                &right.provider,
                &right.model,
                &right.agent_role,
                &right.task_type,
                &right.complexity,
                &right.scope,
                &right.risk,
            ))
    });
    rows
}

fn profile_value(value: &str) -> Option<String> {
    if value == "*" {
        None
    } else {
        Some(value.to_string())
    }
}

fn performance_row_matches(row: &PerformanceSummaryRow, query: &Query) -> bool {
    matches_filter(&query.provider, &row.provider)
        && matches_filter(&query.model, &row.model)
        && matches_optional_filter(&query.agent_role, &row.agent_role)
        && matches_optional_filter(&query.task_type, &row.task_type)
        && matches_optional_filter(&query.complexity, &row.complexity)
        && matches_optional_filter(&query.scope, &row.scope)
        && matches_optional_filter(&query.risk, &row.risk)
}

fn matches_filter(filter: &Option<String>, value: &str) -> bool {
    filter.as_ref().is_none_or(|filter| filter == value)
}

fn matches_optional_filter(filter: &Option<String>, value: &Option<String>) -> bool {
    filter
        .as_ref()
        .is_none_or(|filter| value.as_deref() == Some(filter.as_str()))
}

fn rate(count: u64, total: u64) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(count as f64 / total as f64)
    }
}

fn is_stale(timestamp: DateTime<Utc>, stale_after_hours: u64) -> bool {
    let Ok(hours) = i64::try_from(stale_after_hours) else {
        return false;
    };
    Utc::now() - timestamp > Duration::hours(hours)
}

struct AnalyticsResponseParts<'a> {
    status: &'a str,
    data_status: &'a str,
    scope: &'a str,
    query: &'a Query,
    summary: Value,
    rows: Vec<Value>,
    raw_events: Value,
    warnings: Vec<String>,
}

fn analytics_response(parts: AnalyticsResponseParts<'_>) -> Value {
    serde_json::json!({
        "status": parts.status,
        "data_status": parts.data_status,
        "generated_at": Utc::now(),
        "scope": parts.scope,
        "filters": normalized_filters(parts.query),
        "summary": parts.summary,
        "rows": parts.rows,
        "raw_events": parts.raw_events,
        "privacy": privacy_statement(),
        "warnings": parts.warnings,
    })
}

fn normalized_filters(query: &Query) -> Value {
    serde_json::json!({
        "provider": query.provider,
        "model": query.model,
        "agent_role": query.agent_role,
        "task_type": query.task_type,
        "complexity": query.complexity,
        "scope": query.scope,
        "risk": query.risk,
        "stale_after_hours": query.stale_after_hours,
        "limit": query.limit,
        "include_raw_events": query.include_raw_events,
    })
}

fn privacy_statement() -> Value {
    serde_json::json!({
        "credential_fingerprints_returned": false,
        "secrets_returned": false,
        "provider_messages_returned": false,
        "raw_prompts_or_completions_returned": false,
        "notes": "Default analytics are aggregate, bounded, local-only, read-only, and redacted."
    })
}

fn access_summary(rows: &[Value]) -> Value {
    let mut status_counts = BTreeMap::from([
        ("accessible", 0_u64),
        ("denied", 0),
        ("auth_failed", 0),
        ("unknown", 0),
    ]);
    let stale_rows = rows
        .iter()
        .filter(|row| row["is_stale"].as_bool().unwrap_or(false))
        .count();
    for row in rows {
        for key in ["accessible", "denied", "auth_failed", "unknown"] {
            if let Some(value) = row["status_counts"][key].as_u64() {
                *status_counts.entry(key).or_default() += value;
            }
        }
    }
    serde_json::json!({
        "row_count": rows.len(),
        "stale_row_count": stale_rows,
        "status_counts": status_counts,
    })
}

fn performance_summary(rows: &[Value]) -> Value {
    let calls: u64 = rows.iter().filter_map(|row| row["calls"].as_u64()).sum();
    let successes: u64 = rows
        .iter()
        .filter_map(|row| row["successes"].as_u64())
        .sum();
    let failures: u64 = rows.iter().filter_map(|row| row["failures"].as_u64()).sum();
    let stale_rows = rows
        .iter()
        .filter(|row| row["is_stale"].as_bool().unwrap_or(false))
        .count();
    serde_json::json!({
        "row_count": rows.len(),
        "stale_row_count": stale_rows,
        "calls": calls,
        "successes": successes,
        "failures": failures,
        "success_rate": rate(successes, calls),
        "failure_rate": rate(failures, calls),
    })
}

fn access_raw_events(home: &Path, query: &Query, warnings: &mut Vec<String>) -> Value {
    let path = model_access_events_path_for_home(home);
    let events = read_recent_jsonl::<ModelAccessEvent, _>(&path, query.limit, warnings, |event| {
        matches_filter(&query.provider, &event.provider)
            && matches_filter(&query.model, &event.model)
    });
    let rows: Vec<Value> = events
        .into_iter()
        .map(|event| {
            serde_json::json!({
                "provider": event.provider,
                "model": event.model,
                "status": event.status,
                "reason_code": event.reason_code,
                "checked_at": event.checked_at,
            })
        })
        .collect();
    warnings.push("raw mode is explicit, bounded, and redacted".to_string());
    Value::Array(rows)
}

fn performance_raw_events(workspace: &Path, query: &Query, warnings: &mut Vec<String>) -> Value {
    let path = model_performance_events_path_for_workspace(workspace);
    let events = read_recent_jsonl::<ModelCallEvent, _>(&path, query.limit, warnings, |event| {
        matches_filter(&query.provider, &event.provider)
            && matches_filter(&query.model, &event.model)
            && matches_optional_filter(&query.agent_role, &event.agent_role)
            && matches_optional_filter(&query.task_type, &event.task_type)
            && matches_optional_filter(&query.complexity, &event.complexity)
            && matches_optional_filter(&query.scope, &event.scope)
            && matches_optional_filter(&query.risk, &event.risk)
    });
    let rows: Vec<Value> = events
        .into_iter()
        .map(|event| {
            serde_json::json!({
                "timestamp": event.timestamp,
                "provider": event.provider,
                "model": event.model,
                "agent_role": event.agent_role,
                "task_type": event.task_type,
                "complexity": event.complexity,
                "scope": event.scope,
                "risk": event.risk,
                "prompt_tokens": event.prompt_tokens,
                "completion_tokens": event.completion_tokens,
                "reasoning_tokens": event.reasoning_tokens,
                "latency_ms": event.latency_ms,
                "first_token_latency_ms": event.first_token_latency_ms,
                "cost_usd": event.cost_usd,
                "tool_parse_success": event.tool_parse_success,
                "patch_count": event.patch_count,
                "validation_error_count": event.validation_errors.len(),
                "retries": event.retries,
                "outcome": event.outcome,
            })
        })
        .collect();
    warnings.push("raw mode is explicit, bounded, and redacted".to_string());
    Value::Array(rows)
}

fn read_recent_jsonl<T, F>(
    path: &Path,
    limit: usize,
    warnings: &mut Vec<String>,
    mut keep: F,
) -> Vec<T>
where
    T: DeserializeOwned,
    F: FnMut(&T) -> bool,
{
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut rows = VecDeque::with_capacity(limit);
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let Ok(line) = line else {
            warnings.push(format!("event log row {index} is unreadable"));
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(&line) {
            Ok(value) => {
                if keep(&value) {
                    if rows.len() == limit {
                        rows.pop_front();
                    }
                    rows.push_back(value);
                }
            }
            Err(_) => warnings.push(format!("event log row {index} is malformed")),
        }
    }
    rows.into_iter().rev().collect()
}

fn access_health(home: &Path, stale_after_hours: u64) -> Value {
    let current_path = model_access_current_path_for_home(home);
    let events_path = model_access_events_path_for_home(home);
    match read_json_file::<ModelAccessDb>(&current_path) {
        StoreRead::Absent => access_health_json("absent", 0, None, 0, &events_path),
        StoreRead::Empty => access_health_json("empty", 0, None, 0, &events_path),
        StoreRead::Malformed(_) => access_health_json("malformed", 0, None, 0, &events_path),
        StoreRead::Unreadable(_) => access_health_json("unreadable", 0, None, 0, &events_path),
        StoreRead::Present(db) => {
            let latest = db.records.values().map(|record| record.last_checked).max();
            let stale_count = db
                .records
                .values()
                .filter(|record| is_stale(record.last_checked, stale_after_hours))
                .count();
            let status = if db.records.is_empty() {
                "empty"
            } else if stale_count == db.records.len() {
                "stale"
            } else if stale_count > 0 {
                "partial"
            } else {
                "present"
            };
            access_health_json(status, db.records.len(), latest, stale_count, &events_path)
        }
    }
}

fn performance_health(workspace: &Path, stale_after_hours: u64) -> Value {
    let aggregate_path = model_performance_aggregates_path_for_workspace(workspace);
    let events_path = model_performance_events_path_for_workspace(workspace);
    match read_json_file::<ModelPerformanceDb>(&aggregate_path) {
        StoreRead::Absent => performance_health_json("absent", 0, None, 0, &events_path),
        StoreRead::Empty => performance_health_json("empty", 0, None, 0, &events_path),
        StoreRead::Malformed(_) => performance_health_json("malformed", 0, None, 0, &events_path),
        StoreRead::Unreadable(_) => performance_health_json("unreadable", 0, None, 0, &events_path),
        StoreRead::Present(db) => {
            let latest = db
                .aggregates
                .values()
                .filter_map(|aggregate| aggregate.last_updated)
                .max();
            let stale_count = db
                .aggregates
                .values()
                .filter(|aggregate| {
                    aggregate
                        .last_updated
                        .map(|timestamp| is_stale(timestamp, stale_after_hours))
                        .unwrap_or(true)
                })
                .count();
            let status = if db.aggregates.is_empty() {
                "empty"
            } else if stale_count == db.aggregates.len() {
                "stale"
            } else if stale_count > 0 {
                "partial"
            } else {
                "present"
            };
            performance_health_json(
                status,
                db.aggregates.len(),
                latest,
                stale_count,
                &events_path,
            )
        }
    }
}

fn access_health_json(
    status: &str,
    count: usize,
    latest: Option<DateTime<Utc>>,
    stale_count: usize,
    events_path: &Path,
) -> Value {
    serde_json::json!({
        "scope": "user_home_model_access",
        "current_status": status,
        "record_count": count,
        "latest_checked": latest,
        "stale_record_count": stale_count,
        "events_status": file_status(events_path),
        "event_log_size_bytes": file_size(events_path),
    })
}

fn performance_health_json(
    status: &str,
    count: usize,
    latest: Option<DateTime<Utc>>,
    stale_count: usize,
    events_path: &Path,
) -> Value {
    serde_json::json!({
        "scope": "workspace_model_performance",
        "aggregate_status": status,
        "aggregate_count": count,
        "latest_updated": latest,
        "stale_aggregate_count": stale_count,
        "events_status": file_status(events_path),
        "event_log_size_bytes": file_size(events_path),
    })
}

fn combined_health_status(home: &Path, workspace: &Path, stale_after_hours: u64) -> &'static str {
    let access = access_health(home, stale_after_hours);
    let performance = performance_health(workspace, stale_after_hours);
    let statuses = [
        access["current_status"].as_str(),
        performance["aggregate_status"].as_str(),
    ];
    if statuses.contains(&Some("malformed")) {
        "malformed"
    } else if statuses.contains(&Some("partial")) || statuses.contains(&Some("unreadable")) {
        "partial"
    } else if statuses.contains(&Some("stale")) {
        "stale"
    } else if statuses
        .iter()
        .all(|status| matches!(status, Some("absent" | "empty")))
    {
        "empty"
    } else {
        "present"
    }
}

fn file_status(path: &Path) -> &'static str {
    if !path.exists() {
        "absent"
    } else if file_size(path) == Some(0) {
        "empty"
    } else {
        "present"
    }
}

fn file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn duumbi_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::model_access::{MODEL_ACCESS_PROBE_VERSION, ModelAccessRecord};
    use crate::agents::model_performance::ModelCallOutcome;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("invariant: create parent");
        }
        fs::write(
            path,
            serde_json::to_string_pretty(value).expect("serialize fixture"),
        )
        .expect("write fixture");
    }

    fn checked_at(hours_ago: i64) -> DateTime<Utc> {
        Utc::now() - Duration::hours(hours_ago)
    }

    #[test]
    fn access_summary_groups_statuses_and_redacts_fingerprints() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let mut records = HashMap::new();
        for (fingerprint, status) in [
            ("sha256:secret-a", ModelAccessStatus::Accessible),
            ("sha256:secret-b", ModelAccessStatus::Denied),
            ("sha256:secret-c", ModelAccessStatus::AuthFailed),
            ("sha256:secret-d", ModelAccessStatus::Unknown),
        ] {
            records.insert(
                format!("{fingerprint}|minimax|MiniMax-M2.7"),
                ModelAccessRecord {
                    credential_fingerprint: fingerprint.to_string(),
                    provider: "minimax".to_string(),
                    model: "MiniMax-M2.7".to_string(),
                    status,
                    reason_code: Some("provider_neutral".to_string()),
                    message: Some("sensitive provider message".to_string()),
                    probe_version: MODEL_ACCESS_PROBE_VERSION.to_string(),
                    last_checked: checked_at(1),
                    last_success: (status == ModelAccessStatus::Accessible).then(|| checked_at(1)),
                },
            );
        }
        let db = serde_json::to_value(ModelAccessDb { records }).expect("serialize db");
        write_json(&model_access_current_path_for_home(temp.path()), &db);

        let response = model_access_summary_from_home(
            temp.path(),
            &Query::parse(&serde_json::json!({}), QueryKind::Access).expect("valid query"),
        )
        .expect("summary");
        let text = serde_json::to_string(&response).expect("serialize response");

        assert_eq!(response["data_status"], "present");
        assert_eq!(response["rows"][0]["status_counts"]["accessible"], 1);
        assert_eq!(response["rows"][0]["status_counts"]["denied"], 1);
        assert_eq!(response["rows"][0]["status_counts"]["auth_failed"], 1);
        assert_eq!(response["rows"][0]["status_counts"]["unknown"], 1);
        assert!(!text.contains("sha256:secret"));
        assert!(!text.contains("sensitive provider message"));
    }

    #[test]
    fn access_summary_marks_stale_records() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let mut records = HashMap::new();
        records.insert(
            "sha256:secret|minimax|MiniMax-M2.7".to_string(),
            ModelAccessRecord {
                credential_fingerprint: "sha256:secret".to_string(),
                provider: "minimax".to_string(),
                model: "MiniMax-M2.7".to_string(),
                status: ModelAccessStatus::Accessible,
                reason_code: None,
                message: None,
                probe_version: MODEL_ACCESS_PROBE_VERSION.to_string(),
                last_checked: checked_at(200),
                last_success: Some(checked_at(200)),
            },
        );
        let db = serde_json::to_value(ModelAccessDb { records }).expect("serialize db");
        write_json(&model_access_current_path_for_home(temp.path()), &db);

        let response = model_access_summary_from_home(
            temp.path(),
            &Query::parse(&serde_json::json!({}), QueryKind::Access).expect("valid query"),
        )
        .expect("summary");

        assert_eq!(response["data_status"], "stale");
        assert_eq!(response["rows"][0]["is_stale"], true);
        assert!(
            response["warnings"][0]
                .as_str()
                .expect("warning")
                .contains("stale")
        );
    }

    #[test]
    fn performance_summary_filters_task_profile_and_computes_rates() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let db = serde_json::json!({
            "aggregates": {
                "anthropic|claude-sonnet|coder|create|simple|single|high": {
                    "calls": 10,
                    "successes": 7,
                    "failures": 3,
                    "ewmaLatencyMs": 120.0,
                    "ewmaCostUsd": 0.02,
                    "parseFailures": 1,
                    "validationFailures": 2,
                    "retries": 4,
                    "lastUpdated": Utc::now(),
                },
                "anthropic|claude-sonnet|coder|fix|simple|single|low": {
                    "calls": 1,
                    "successes": 1,
                    "failures": 0,
                    "ewmaLatencyMs": 80.0,
                    "ewmaCostUsd": 0.01,
                    "parseFailures": 0,
                    "validationFailures": 0,
                    "retries": 0,
                    "lastUpdated": Utc::now(),
                }
            }
        });
        write_json(
            &model_performance_aggregates_path_for_workspace(temp.path()),
            &db,
        );

        let response = model_performance_summary_from_workspace(
            temp.path(),
            &Query::parse(
                &serde_json::json!({"task_type": "create", "risk": "high"}),
                QueryKind::Performance,
            )
            .expect("valid query"),
        )
        .expect("summary");

        assert_eq!(response["data_status"], "present");
        assert_eq!(response["rows"].as_array().expect("rows").len(), 1);
        assert_eq!(response["rows"][0]["calls"], 10);
        assert_eq!(response["rows"][0]["success_rate"], 0.7);
        assert_eq!(response["rows"][0]["failure_rate"], 0.3);
        assert_eq!(response["filters"]["task_type"], "create");
    }

    #[test]
    fn raw_access_mode_requires_explicit_limit() {
        let err = Query::parse(
            &serde_json::json!({"include_raw_events": true}),
            QueryKind::Access,
        )
        .expect_err("raw mode without limit must fail");

        assert!(err.contains("requires an explicit 'limit'"));
    }

    #[test]
    fn health_rejects_raw_event_mode() {
        let err = Query::parse(
            &serde_json::json!({"include_raw_events": true, "limit": 1}),
            QueryKind::Health,
        )
        .expect_err("health raw mode must fail");

        assert!(err.contains("not supported"));
    }

    #[test]
    fn raw_access_events_are_bounded_and_redacted() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let events_path = model_access_events_path_for_home(temp.path());
        fs::create_dir_all(events_path.parent().expect("parent")).expect("create dir");
        let mut lines = Vec::new();
        for index in 0..12 {
            lines.push(
                serde_json::to_string(&ModelAccessEvent {
                    credential_fingerprint: format!("sha256:secret-{index}"),
                    provider: "minimax".to_string(),
                    model: format!("model-{index}"),
                    status: ModelAccessStatus::Accessible,
                    reason_code: Some("ok".to_string()),
                    message: Some("provider message".to_string()),
                    checked_at: checked_at(index),
                })
                .expect("serialize event"),
            );
        }
        fs::write(&events_path, lines.join("\n")).expect("write events");

        let response = model_access_summary_from_home(
            temp.path(),
            &Query::parse(
                &serde_json::json!({"include_raw_events": true, "limit": 10}),
                QueryKind::Access,
            )
            .expect("valid query"),
        )
        .expect("summary");
        let text = serde_json::to_string(&response).expect("serialize response");

        assert_eq!(response["raw_events"].as_array().expect("raw").len(), 10);
        assert_eq!(response["raw_events"][0]["model"], "model-11");
        assert!(!text.contains("sha256:secret"));
        assert!(!text.contains("provider message"));
    }

    #[test]
    fn raw_access_events_honor_provider_and_model_filters() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let events_path = model_access_events_path_for_home(temp.path());
        fs::create_dir_all(events_path.parent().expect("parent")).expect("create dir");
        let events = [
            ModelAccessEvent {
                credential_fingerprint: "sha256:secret-a".to_string(),
                provider: "minimax".to_string(),
                model: "MiniMax-M2.7".to_string(),
                status: ModelAccessStatus::Accessible,
                reason_code: None,
                message: None,
                checked_at: checked_at(3),
            },
            ModelAccessEvent {
                credential_fingerprint: "sha256:secret-b".to_string(),
                provider: "anthropic".to_string(),
                model: "claude-sonnet".to_string(),
                status: ModelAccessStatus::Accessible,
                reason_code: None,
                message: None,
                checked_at: checked_at(2),
            },
            ModelAccessEvent {
                credential_fingerprint: "sha256:secret-c".to_string(),
                provider: "minimax".to_string(),
                model: "MiniMax-M2.7".to_string(),
                status: ModelAccessStatus::Denied,
                reason_code: Some("quota".to_string()),
                message: Some("provider message".to_string()),
                checked_at: checked_at(1),
            },
        ];
        let lines: Vec<_> = events
            .iter()
            .map(|event| serde_json::to_string(event).expect("serialize event"))
            .collect();
        fs::write(&events_path, lines.join("\n")).expect("write events");

        let response = model_access_summary_from_home(
            temp.path(),
            &Query::parse(
                &serde_json::json!({
                    "provider": "minimax",
                    "model": "MiniMax-M2.7",
                    "include_raw_events": true,
                    "limit": 10
                }),
                QueryKind::Access,
            )
            .expect("valid query"),
        )
        .expect("summary");

        let raw = response["raw_events"].as_array().expect("raw events");
        assert_eq!(raw.len(), 2);
        assert!(raw.iter().all(|row| row["provider"] == "minimax"));
        assert!(raw.iter().all(|row| row["model"] == "MiniMax-M2.7"));
        assert_eq!(raw[0]["status"], "denied");
    }

    #[test]
    fn raw_performance_events_honor_task_profile_filters() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let events_path = model_performance_events_path_for_workspace(temp.path());
        fs::create_dir_all(events_path.parent().expect("parent")).expect("create dir");
        let event = |provider: &str, task_type: &str, risk: &str, hours_ago| ModelCallEvent {
            timestamp: checked_at(hours_ago),
            provider: provider.to_string(),
            model: "claude-sonnet".to_string(),
            agent_role: Some("coder".to_string()),
            template_version: Some("v1".to_string()),
            task_type: Some(task_type.to_string()),
            complexity: Some("simple".to_string()),
            scope: Some("single".to_string()),
            risk: Some(risk.to_string()),
            prompt_tokens: Some(10),
            completion_tokens: Some(5),
            reasoning_tokens: None,
            latency_ms: Some(100),
            first_token_latency_ms: Some(20),
            cost_usd: Some(0.01),
            tool_parse_success: true,
            patch_count: 1,
            validation_errors: vec!["E001".to_string()],
            retries: 0,
            outcome: ModelCallOutcome::Success,
        };
        let events = [
            event("anthropic", "create", "high", 3),
            event("anthropic", "fix", "high", 2),
            event("minimax", "create", "high", 1),
        ];
        let lines: Vec<_> = events
            .iter()
            .map(|event| serde_json::to_string(event).expect("serialize event"))
            .collect();
        fs::write(&events_path, lines.join("\n")).expect("write events");

        let response = model_performance_summary_from_workspace(
            temp.path(),
            &Query::parse(
                &serde_json::json!({
                    "provider": "anthropic",
                    "task_type": "create",
                    "risk": "high",
                    "include_raw_events": true,
                    "limit": 10
                }),
                QueryKind::Performance,
            )
            .expect("valid query"),
        )
        .expect("summary");

        let raw = response["raw_events"].as_array().expect("raw events");
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0]["provider"], "anthropic");
        assert_eq!(raw[0]["task_type"], "create");
        assert_eq!(raw[0]["risk"], "high");
        assert_eq!(raw[0]["validation_error_count"], 1);
    }

    #[test]
    fn health_returns_documented_source_specific_fields() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let mut access_records = HashMap::new();
        access_records.insert(
            "sha256:secret|minimax|MiniMax-M2.7".to_string(),
            ModelAccessRecord {
                credential_fingerprint: "sha256:secret".to_string(),
                provider: "minimax".to_string(),
                model: "MiniMax-M2.7".to_string(),
                status: ModelAccessStatus::Accessible,
                reason_code: None,
                message: None,
                probe_version: MODEL_ACCESS_PROBE_VERSION.to_string(),
                last_checked: checked_at(1),
                last_success: Some(checked_at(1)),
            },
        );
        write_json(
            &model_access_current_path_for_home(temp.path()),
            &serde_json::to_value(ModelAccessDb {
                records: access_records,
            })
            .expect("serialize access db"),
        );

        let performance = serde_json::json!({
            "aggregates": {
                "anthropic|claude-sonnet|coder|create|simple|single|high": {
                    "calls": 1,
                    "successes": 1,
                    "failures": 0,
                    "ewmaLatencyMs": 100.0,
                    "ewmaCostUsd": 0.01,
                    "parseFailures": 0,
                    "validationFailures": 0,
                    "retries": 0,
                    "lastUpdated": Utc::now(),
                }
            }
        });
        write_json(
            &model_performance_aggregates_path_for_workspace(temp.path()),
            &performance,
        );

        let access = access_health(temp.path(), DEFAULT_STALE_AFTER_HOURS);
        let performance = performance_health(temp.path(), DEFAULT_STALE_AFTER_HOURS);

        assert_eq!(access["scope"], "user_home_model_access");
        assert_eq!(access["current_status"], "present");
        assert_eq!(access["record_count"], 1);
        assert!(access["latest_checked"].is_string());
        assert_eq!(access["stale_record_count"], 0);
        assert_eq!(performance["scope"], "workspace_model_performance");
        assert_eq!(performance["aggregate_status"], "present");
        assert_eq!(performance["aggregate_count"], 1);
        assert!(performance["latest_updated"].is_string());
        assert_eq!(performance["stale_aggregate_count"], 0);
    }

    #[test]
    fn malformed_performance_key_reports_partial_diagnostic() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let db = serde_json::json!({
            "aggregates": {
                "invalid-profile-key": {
                    "calls": 1,
                    "successes": 0,
                    "failures": 1,
                    "ewmaLatencyMs": null,
                    "ewmaCostUsd": null,
                    "parseFailures": 0,
                    "validationFailures": 0,
                    "retries": 0,
                    "lastUpdated": Utc::now(),
                }
            }
        });
        write_json(
            &model_performance_aggregates_path_for_workspace(temp.path()),
            &db,
        );

        let response = model_performance_summary_from_workspace(
            temp.path(),
            &Query::parse(&serde_json::json!({}), QueryKind::Performance).expect("valid query"),
        )
        .expect("summary");

        assert_eq!(response["status"], "partial");
        assert_eq!(response["data_status"], "partial");
        assert_eq!(response["rows"].as_array().expect("rows").len(), 0);
        assert!(
            response["warnings"][0]
                .as_str()
                .expect("warning")
                .contains("invalid profile key")
        );
    }

    #[test]
    fn malformed_performance_store_reports_diagnostic_without_contents() {
        let temp = TempDir::new().expect("invariant: temp dir");
        let path = model_performance_aggregates_path_for_workspace(temp.path());
        fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        fs::write(&path, "{not-json-secret-value").expect("write malformed file");

        let response = model_performance_summary_from_workspace(
            temp.path(),
            &Query::parse(&serde_json::json!({}), QueryKind::Performance).expect("valid query"),
        )
        .expect("summary");
        let text = serde_json::to_string(&response).expect("serialize response");

        assert_eq!(response["status"], "error");
        assert_eq!(response["data_status"], "malformed");
        assert!(!text.contains("not-json-secret-value"));
        assert_eq!(
            fs::read_to_string(path).expect("malformed file still exists"),
            "{not-json-secret-value"
        );
    }
}
