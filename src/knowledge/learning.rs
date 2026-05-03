//! Success logger for few-shot learning.
//!
//! Appends successful mutation records to `.duumbi/learning/successes.jsonl`
//! (one JSON object per line). Provides query and scoring functions for
//! the context assembly pipeline.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::knowledge::KnowledgeError;
use crate::knowledge::types::{FailureRecord, SuccessRecord};

#[cfg(test)]
static TEST_USER_LEARNING_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Returns the path to the successes JSONL file.
fn successes_path(workspace: &Path) -> PathBuf {
    workspace.join(".duumbi/learning/successes.jsonl")
}

/// Returns the path to the failures JSONL file.
fn failures_path(workspace: &Path) -> PathBuf {
    workspace.join(".duumbi/learning/failures.jsonl")
}

/// Returns the user-local learning directory for a home directory.
#[must_use]
pub fn user_learning_dir_for_home(home: &Path) -> PathBuf {
    home.join(".duumbi").join("learning")
}

/// Returns the user-local learning directory.
#[must_use]
pub fn user_learning_dir() -> PathBuf {
    #[cfg(test)]
    {
        return TEST_USER_LEARNING_DIR
            .lock()
            .expect("invariant: test learning dir mutex not poisoned")
            .clone()
            .unwrap_or_else(|| PathBuf::from("/tmp/duumbi-test-learning-disabled"));
    }

    #[cfg(not(test))]
    crate::config::user_config_path().parent().map_or_else(
        || PathBuf::from(".duumbi/learning"),
        |dir| dir.join("learning"),
    )
}

#[cfg(test)]
#[allow(dead_code)]
fn set_test_user_learning_dir(path: PathBuf) {
    *TEST_USER_LEARNING_DIR
        .lock()
        .expect("invariant: test learning dir mutex not poisoned") = Some(path);
}

/// Returns the user-local successes JSONL path.
#[must_use]
pub fn user_successes_path() -> PathBuf {
    user_learning_dir().join("successes.jsonl")
}

/// Returns the user-local failures JSONL path.
#[must_use]
pub fn user_failures_path() -> PathBuf {
    user_learning_dir().join("failures.jsonl")
}

fn append_jsonl<T: serde::Serialize>(path: &Path, record: &T) -> Result<(), KnowledgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| KnowledgeError::Io(format!("creating learning dir: {e}")))?;
    }

    let line =
        serde_json::to_string(record).map_err(|e| KnowledgeError::Serialize(e.to_string()))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| KnowledgeError::Io(format!("opening {}: {e}", path.display())))?;

    writeln!(file, "{line}")
        .map_err(|e| KnowledgeError::Io(format!("writing to {}: {e}", path.display())))?;

    Ok(())
}

fn query_jsonl<T: serde::de::DeserializeOwned>(path: &Path, limit: usize) -> Vec<T> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let mut records: Vec<T> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<T>(&line).ok())
        .collect();

    if records.len() > limit {
        records = records.split_off(records.len() - limit);
    }

    records
}

/// Appends a success record as a single JSONL line.
///
/// Creates the `.duumbi/learning/` directory if it does not exist.
///
/// # Errors
///
/// Returns an error if directory creation, serialization, or file write fails.
pub fn append_success(workspace: &Path, record: &SuccessRecord) -> Result<(), KnowledgeError> {
    append_jsonl(&successes_path(workspace), record)
}

/// Appends a success record to user-local learning.
pub fn append_user_success(record: &SuccessRecord) -> Result<(), KnowledgeError> {
    append_jsonl(&user_successes_path(), record)
}

/// Appends a success record to workspace-local and user-local learning.
pub fn append_success_with_user_cache(
    workspace: &Path,
    record: &SuccessRecord,
) -> Result<(), KnowledgeError> {
    append_success(workspace, record)?;
    append_user_success(record)
}

/// Reads the last `limit` success records from the JSONL file.
///
/// Returns records in chronological order (oldest first).
/// Silently skips malformed lines. Returns an empty vec if the file
/// does not exist or cannot be read.
#[must_use]
pub fn query_successes(workspace: &Path, limit: usize) -> Vec<SuccessRecord> {
    query_jsonl(&successes_path(workspace), limit)
}

/// Reads the last `limit` user-local success records.
#[must_use]
pub fn query_user_successes(limit: usize) -> Vec<SuccessRecord> {
    query_jsonl(&user_successes_path(), limit)
}

/// Reads success records from workspace-local first, then user-local, deduped by id.
#[must_use]
pub fn query_combined_successes(workspace: &Path, limit: usize) -> Vec<SuccessRecord> {
    let mut seen = HashSet::new();
    query_successes(workspace, limit)
        .into_iter()
        .chain(query_user_successes(limit))
        .filter(|record| seen.insert(record.id.clone()))
        .take(limit)
        .collect()
}

/// Appends a failure record as a single JSONL line.
pub fn append_failure(workspace: &Path, record: &FailureRecord) -> Result<(), KnowledgeError> {
    append_jsonl(&failures_path(workspace), record)
}

/// Appends a failure record to user-local learning.
pub fn append_user_failure(record: &FailureRecord) -> Result<(), KnowledgeError> {
    append_jsonl(&user_failures_path(), record)
}

/// Appends a failure record to workspace-local and user-local learning.
pub fn append_failure_with_user_cache(
    workspace: &Path,
    record: &FailureRecord,
) -> Result<(), KnowledgeError> {
    append_failure(workspace, record)?;
    append_user_failure(record)
}

/// Reads the last `limit` workspace-local failure records.
#[must_use]
pub fn query_failures(workspace: &Path, limit: usize) -> Vec<FailureRecord> {
    query_jsonl(&failures_path(workspace), limit)
}

/// Reads the last `limit` user-local failure records.
#[must_use]
pub fn query_user_failures(limit: usize) -> Vec<FailureRecord> {
    query_jsonl(&user_failures_path(), limit)
}

/// Reads failure records from workspace-local first, then user-local, deduped by id.
#[must_use]
pub fn query_combined_failures(workspace: &Path, limit: usize) -> Vec<FailureRecord> {
    let mut seen = HashSet::new();
    query_failures(workspace, limit)
        .into_iter()
        .chain(query_user_failures(limit))
        .filter(|record| seen.insert(record.id.clone()))
        .take(limit)
        .collect()
}

/// Sanitizes provider-facing failure text before it is stored in learning logs.
#[must_use]
pub fn sanitize_error_summary(summary: &str) -> String {
    let mut sanitized = summary
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            !lower.contains("authorization:")
                && !lower.contains("api-key")
                && !lower.contains("api_key")
                && !lower.contains("bearer ")
        })
        .collect::<Vec<_>>()
        .join("\n");

    const MAX_SUMMARY_CHARS: usize = 800;
    if sanitized.len() > MAX_SUMMARY_CHARS {
        sanitized.truncate(MAX_SUMMARY_CHARS);
        sanitized.push_str("...");
    }
    sanitized
}

/// Returns the total count of success records in the JSONL file.
#[must_use]
pub fn success_count(workspace: &Path) -> usize {
    let path = successes_path(workspace);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    BufReader::new(file).lines().map_while(Result::ok).count()
}

/// Scores a success record's relevance to a given request and task type.
///
/// Higher scores indicate more relevant examples for few-shot injection.
///
/// Scoring rules:
/// - Task type match: +3
/// - Error code overlap: +5 per matching code
/// - Module name match (request contains module name part): +2
/// - Function name match (request contains function name): +1 per match
#[must_use]
pub fn score_for_request(
    record: &SuccessRecord,
    task_type: &str,
    request: &str,
    error_codes: &[String],
) -> u32 {
    let mut score = 0u32;

    // Task type match
    if record.task_type == task_type {
        score += 3;
    }

    // Error code overlap
    for code in error_codes {
        if record.error_codes.contains(code) {
            score += 5;
        }
    }

    // Module match: check if any word in the request matches the module name
    if !record.module.is_empty() {
        let request_lower = request.to_lowercase();
        let module_lower = record.module.to_lowercase();
        // Check module name parts
        for part in module_lower.split('/') {
            if request_lower.contains(part) {
                score += 2;
                break;
            }
        }
    }

    // Function name overlap with request
    for func in &record.functions {
        if request.to_lowercase().contains(&func.to_lowercase()) {
            score += 1;
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::types::SuccessRecord;
    use tempfile::TempDir;

    #[test]
    fn append_and_query_roundtrip() {
        let tmp = TempDir::new().expect("temp dir");

        let mut r1 = SuccessRecord::new("add multiply", "AddFunction");
        r1.ops_count = 1;
        r1.module = "main".to_string();
        append_success(tmp.path(), &r1).expect("append");

        let mut r2 = SuccessRecord::new("fix error", "FixError");
        r2.ops_count = 2;
        append_success(tmp.path(), &r2).expect("append");

        let records = query_successes(tmp.path(), 10);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].request, "add multiply");
        assert_eq!(records[1].request, "fix error");
    }

    #[test]
    fn query_with_limit() {
        let tmp = TempDir::new().expect("temp dir");
        for i in 0..5 {
            let r = SuccessRecord::new(format!("request {i}"), "AddFunction");
            append_success(tmp.path(), &r).expect("append");
        }
        let records = query_successes(tmp.path(), 3);
        assert_eq!(records.len(), 3);
        // Should be the last 3
        assert_eq!(records[0].request, "request 2");
        assert_eq!(records[2].request, "request 4");
    }

    #[test]
    fn query_empty_workspace() {
        let tmp = TempDir::new().expect("temp dir");
        let records = query_successes(tmp.path(), 10);
        assert!(records.is_empty());
    }

    #[test]
    fn append_and_query_failures_roundtrip() {
        let tmp = TempDir::new().expect("temp dir");

        let mut r = FailureRecord::new("add multiply", "AddFunction", "no_tool_calls");
        r.provider = "minimax".to_string();
        r.error_summary = "LLM returned no tool calls".to_string();
        append_failure(tmp.path(), &r).expect("append");

        let records = query_failures(tmp.path(), 10);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].request, "add multiply");
        assert_eq!(records[0].failure_category, "no_tool_calls");
        assert_eq!(records[0].provider, "minimax");
    }

    #[test]
    fn combined_successes_deduplicate_workspace_first() {
        let tmp = TempDir::new().expect("temp dir");
        let mut workspace_record = SuccessRecord::new("workspace", "AddFunction");
        workspace_record.id = "duumbi:success/shared".to_string();
        append_success(tmp.path(), &workspace_record).expect("append");

        let mut user_record = workspace_record.clone();
        user_record.request = "user duplicate".to_string();
        append_jsonl(&tmp.path().join("user-successes.jsonl"), &user_record).expect("append");

        let mut seen = HashSet::new();
        let combined: Vec<_> = query_successes(tmp.path(), 10)
            .into_iter()
            .chain(query_jsonl::<SuccessRecord>(
                &tmp.path().join("user-successes.jsonl"),
                10,
            ))
            .filter(|record| seen.insert(record.id.clone()))
            .collect();

        assert_eq!(combined.len(), 1);
        assert_eq!(combined[0].request, "workspace");
    }

    #[test]
    fn user_learning_dir_uses_duumbi_learning() {
        let home = Path::new("/tmp/example-home");
        assert_eq!(
            user_learning_dir_for_home(home),
            PathBuf::from("/tmp/example-home/.duumbi/learning")
        );
    }

    #[test]
    fn sanitize_error_summary_removes_secret_bearing_lines_and_truncates() {
        let summary = format!(
            "first\nAuthorization: Bearer secret\nx-api-key: secret\n{}",
            "a".repeat(900)
        );
        let sanitized = sanitize_error_summary(&summary);

        assert!(!sanitized.to_ascii_lowercase().contains("bearer secret"));
        assert!(!sanitized.to_ascii_lowercase().contains("x-api-key"));
        assert!(sanitized.len() <= 803);
    }

    #[test]
    fn success_count_works() {
        let tmp = TempDir::new().expect("temp dir");
        assert_eq!(success_count(tmp.path()), 0);

        append_success(tmp.path(), &SuccessRecord::new("r1", "t1")).expect("append");
        append_success(tmp.path(), &SuccessRecord::new("r2", "t2")).expect("append");
        assert_eq!(success_count(tmp.path()), 2);
    }

    #[test]
    fn score_task_type_match() {
        let mut r = SuccessRecord::new("add func", "AddFunction");
        r.module = "math".to_string();

        let score = score_for_request(&r, "AddFunction", "add something", &[]);
        assert!(score >= 3, "task type match should give >= 3, got {score}");
    }

    #[test]
    fn score_error_code_match() {
        let mut r = SuccessRecord::new("fix", "FixError");
        r.error_codes = vec!["E001".to_string(), "E003".to_string()];

        let score = score_for_request(
            &r,
            "FixError",
            "fix error",
            &["E001".to_string(), "E005".to_string()],
        );
        // task_type +3, E001 match +5 = 8
        assert_eq!(score, 8);
    }

    #[test]
    fn score_module_match() {
        let mut r = SuccessRecord::new("add func", "AddFunction");
        r.module = "calculator/ops".to_string();

        let score = score_for_request(&r, "CreateModule", "create calculator module", &[]);
        // module match +2 (calculator in request)
        assert!(score >= 2, "module match expected, got {score}");
    }

    #[test]
    fn score_no_match() {
        let r = SuccessRecord::new("unrelated", "RefactorModule");
        let score = score_for_request(&r, "AddFunction", "add foo", &[]);
        assert_eq!(score, 0);
    }

    #[test]
    fn creates_directory_on_first_append() {
        let tmp = TempDir::new().expect("temp dir");
        let learning_dir = tmp.path().join(".duumbi/learning");
        assert!(!learning_dir.exists());

        append_success(tmp.path(), &SuccessRecord::new("r", "t")).expect("append");
        assert!(learning_dir.exists());
    }
}
