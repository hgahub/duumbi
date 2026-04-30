//! Few-shot example injection from learning history.
//!
//! Scores past successful mutations against the current request and
//! selects the best matches for prompt enrichment.

use crate::context::budget::TokenEstimator;
use crate::context::classifier::TaskType;
use crate::knowledge::learning;

/// Selects the best few-shot examples from learning history.
///
/// Returns up to 3 formatted examples, each within `max_tokens_per_example`
/// tokens. Skips injection entirely if no example scores above 2.
///
/// # Arguments
///
/// * `workspace` — project root path
/// * `task_type` — classified task type for scoring
/// * `request` — the user's request text
/// * `error_codes` — any current error codes (for fix-related scoring)
/// * `estimator` — token estimator for budget compliance
/// * `max_tokens_per_example` — maximum tokens per individual example
#[must_use]
pub fn select_examples(
    workspace: &std::path::Path,
    task_type: &TaskType,
    request: &str,
    error_codes: &[String],
    estimator: &dyn TokenEstimator,
    max_tokens_per_example: usize,
) -> Vec<String> {
    let successes = learning::query_successes(workspace, 50);
    if successes.is_empty() {
        return Vec::new();
    }

    let task_type_str = task_type.as_str();

    // Score all successes
    let mut scored_records: Vec<(u32, String)> = successes
        .iter()
        .map(|record| {
            let score = learning::score_for_request(record, task_type_str, request, error_codes);
            let formatted = format_example(record);
            (score, formatted)
        })
        .filter(|(score, _)| *score > 2)
        .collect();

    // Sort by score descending
    scored_records.sort_by_key(|b| std::cmp::Reverse(b.0));

    // Take top 3 within token budget
    scored_records
        .into_iter()
        .take(3)
        .filter(|(_, text)| estimator.estimate(text) <= max_tokens_per_example)
        .map(|(_, text)| text)
        .collect()
}

/// Formats a success record as a few-shot example string.
fn format_example(record: &crate::knowledge::types::SuccessRecord) -> String {
    let mut lines = vec![format!("Request: \"{}\"", record.request)];
    lines.push(format!("Task: {}", record.task_type));
    if !record.module.is_empty() {
        lines.push(format!("Module: {}", record.module));
    }
    if !record.functions.is_empty() {
        lines.push(format!("Functions: {}", record.functions.join(", ")));
    }
    lines.push(format!("Ops: {}", record.ops_count));
    if !record.error_codes.is_empty() {
        lines.push(format!(
            "Resolved errors: {}",
            record.error_codes.join(", ")
        ));
    }
    if record.retry_count > 0 {
        lines.push(format!("Retries needed: {}", record.retry_count));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::types::SuccessRecord;
    use tempfile::TempDir;

    #[test]
    fn select_examples_empty_history() {
        let tmp = TempDir::new().expect("temp dir");
        let estimator = crate::context::budget::CharEstimator;
        let examples = select_examples(
            tmp.path(),
            &TaskType::AddFunction,
            "add multiply",
            &[],
            &estimator,
            500,
        );
        assert!(examples.is_empty());
    }

    #[test]
    fn select_examples_with_matching_history() {
        let tmp = TempDir::new().expect("temp dir");

        // Add some history
        let mut r = SuccessRecord::new("add helper function", "AddFunction");
        r.ops_count = 1;
        r.module = "main".to_string();
        r.functions = vec!["helper".to_string()];
        learning::append_success(tmp.path(), &r).expect("append");

        let estimator = crate::context::budget::CharEstimator;
        let examples = select_examples(
            tmp.path(),
            &TaskType::AddFunction,
            "add multiply function",
            &[],
            &estimator,
            500,
        );
        // task_type match = +3, which is > 2 threshold
        assert_eq!(examples.len(), 1);
        assert!(examples[0].contains("add helper function"));
    }

    #[test]
    fn select_examples_respects_score_threshold() {
        let tmp = TempDir::new().expect("temp dir");

        let r = SuccessRecord::new("unrelated task", "RefactorModule");
        learning::append_success(tmp.path(), &r).expect("append");

        let estimator = crate::context::budget::CharEstimator;
        let examples = select_examples(
            tmp.path(),
            &TaskType::AddFunction,
            "add something new",
            &[],
            &estimator,
            500,
        );
        // No match should score > 2
        assert!(examples.is_empty());
    }

    #[test]
    fn select_examples_max_three() {
        let tmp = TempDir::new().expect("temp dir");

        for i in 0..5 {
            let mut r = SuccessRecord::new(format!("add func {i}"), "AddFunction");
            r.ops_count = 1;
            learning::append_success(tmp.path(), &r).expect("append");
        }

        let estimator = crate::context::budget::CharEstimator;
        let examples = select_examples(
            tmp.path(),
            &TaskType::AddFunction,
            "add function",
            &[],
            &estimator,
            500,
        );
        assert!(examples.len() <= 3);
    }

    #[test]
    fn format_example_output() {
        let mut r = SuccessRecord::new("add multiply", "AddFunction");
        r.module = "ops".to_string();
        r.functions = vec!["multiply".to_string()];
        r.ops_count = 3;
        r.error_codes = vec!["E003".to_string()];
        r.retry_count = 1;

        let text = format_example(&r);
        assert!(text.contains("add multiply"));
        assert!(text.contains("AddFunction"));
        assert!(text.contains("ops"));
        assert!(text.contains("multiply"));
        assert!(text.contains("E003"));
        assert!(text.contains("Retries needed: 1"));
    }
}
