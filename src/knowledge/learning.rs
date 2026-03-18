//! Success logger for few-shot learning.
//!
//! Appends successful mutation records to `.duumbi/learning/successes.jsonl`
//! (one JSON object per line). Provides query and scoring functions for
//! the context assembly pipeline.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::knowledge::KnowledgeError;
use crate::knowledge::types::SuccessRecord;

/// Returns the path to the successes JSONL file.
fn successes_path(workspace: &Path) -> PathBuf {
    workspace.join(".duumbi/learning/successes.jsonl")
}

/// Appends a success record as a single JSONL line.
///
/// Creates the `.duumbi/learning/` directory if it does not exist.
///
/// # Errors
///
/// Returns an error if directory creation, serialization, or file write fails.
pub fn append_success(workspace: &Path, record: &SuccessRecord) -> Result<(), KnowledgeError> {
    let path = successes_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| KnowledgeError::Io(format!("creating learning dir: {e}")))?;
    }

    let line =
        serde_json::to_string(record).map_err(|e| KnowledgeError::Serialize(e.to_string()))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| KnowledgeError::Io(format!("opening {}: {e}", path.display())))?;

    writeln!(file, "{line}")
        .map_err(|e| KnowledgeError::Io(format!("writing to {}: {e}", path.display())))?;

    Ok(())
}

/// Reads the last `limit` success records from the JSONL file.
///
/// Returns records in chronological order (oldest first).
/// Silently skips malformed lines.
///
/// # Errors
///
/// Returns an error if the file cannot be opened (returns empty vec if
/// the file does not exist).
#[must_use]
pub fn query_successes(workspace: &Path, limit: usize) -> Vec<SuccessRecord> {
    let path = successes_path(workspace);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut records: Vec<SuccessRecord> = reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<SuccessRecord>(&line).ok())
        .collect();

    // Keep only the last `limit` records
    if records.len() > limit {
        records = records.split_off(records.len() - limit);
    }

    records
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
/// - Module match: +2
/// - Op kind overlap: +1 per matching kind
/// - Recency: +1 (if within the last 100 records — caller responsibility)
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
