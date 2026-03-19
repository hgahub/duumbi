//! Context assembly pipeline for intelligent LLM prompt enrichment.
//!
//! The 5-step pipeline: classify → traverse → collect → budget → few-shot.
//! [`assemble_context`] chains these steps and returns a [`ContextBundle`]
//! with the enriched message ready for `mutate_streaming()`.
//!
//! # Design
//!
//! Context assembly is a **pre-processing step** that runs before the
//! orchestrator. It does not modify `mutate_streaming()` internals — it
//! produces an enriched prompt string that replaces the raw user request.

pub mod analyzer;
pub mod budget;
pub mod classifier;
pub mod collector;
pub mod fewshot;
pub mod modularizer;
pub mod traversal;

use std::path::Path;

use crate::context::budget::TokenEstimator;
use crate::session::PersistentTurn;

/// Errors from the context assembly pipeline.
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    /// I/O error reading workspace files.
    #[error("context I/O error: {0}")]
    Io(String),

    /// JSON-LD parsing error.
    #[error("context parse error: {0}")]
    Parse(String),
}

/// Result of the context assembly pipeline.
///
/// Contains the enriched message and metadata about what was included.
#[derive(Debug, Clone)]
pub struct ContextBundle {
    /// The enriched message to pass to `mutate_streaming()`.
    pub enriched_message: String,

    /// The classified task type.
    pub task_type: classifier::TaskType,

    /// Estimated token count of the enriched message.
    pub token_estimate: usize,

    /// Module names referenced in the context.
    pub modules_referenced: Vec<String>,
}

/// Assembles enriched context for an LLM mutation request.
///
/// Chains the 5-step pipeline:
/// 1. **Classify** the request into a [`TaskType`](classifier::TaskType)
/// 2. **Traverse** the graph to determine which nodes to include
/// 3. **Collect** the relevant JSON-LD fragments
/// 4. **Budget** fit the fragments within the token limit
/// 5. **Few-shot** inject relevant past successes
///
/// # Arguments
///
/// * `request` — the raw user request
/// * `workspace` — path to the project root (contains `.duumbi/`)
/// * `session_history` — recent conversation turns for context
///
/// # Errors
///
/// Returns a [`ContextError`] if workspace scanning fails.
pub fn assemble_context(
    request: &str,
    workspace: &Path,
    session_history: &[PersistentTurn],
) -> Result<ContextBundle, ContextError> {
    // Step 1: Classify the request
    let project_map = analyzer::analyze_workspace(workspace)?;
    let task_type = classifier::classify(request, &project_map);

    // Step 2: Build traversal plan
    let plan = traversal::build_plan(&task_type, request, &project_map);

    // Step 3: Collect nodes
    let context_nodes = collector::collect(workspace, &plan, &project_map)?;

    // Step 4: Fit within budget
    let max_tokens = budget::default_max_tokens();
    let estimator = budget::CharEstimator;
    let fitted = budget::fit_to_budget(&context_nodes, max_tokens, &estimator);

    // Step 5: Few-shot injection (extract error codes from request for FixError scoring)
    let error_codes = extract_error_codes(request);
    let few_shot = fewshot::select_examples(
        workspace,
        &task_type,
        request,
        &error_codes,
        &estimator,
        500,
    );

    // Build the enriched message
    let mut parts = Vec::new();

    // Session history context (last 5 turns)
    if !session_history.is_empty() {
        let recent: Vec<_> = session_history.iter().rev().take(5).rev().collect();
        let history_text: Vec<String> = recent
            .iter()
            .enumerate()
            .map(|(i, turn)| format!("{}. '{}'", i + 1, turn.request))
            .collect();
        parts.push(format!(
            "Context from this session: {}",
            history_text.join(" ")
        ));
    }

    // Available modules and functions
    if !project_map.modules.is_empty() {
        let module_list = analyzer::format_module_summary(&project_map);
        parts.push(format!("Available modules:\n{module_list}"));
    }

    // Collected graph context
    if !fitted.is_empty() {
        parts.push(format!("Relevant graph context:\n{fitted}"));
    }

    // Few-shot examples
    if !few_shot.is_empty() {
        parts.push(format!(
            "Similar successful mutations:\n{}",
            few_shot.join("\n---\n")
        ));
    }

    // The actual request
    parts.push(request.to_string());

    let enriched_message = parts.join("\n\n");
    let token_estimate = estimator.estimate(&enriched_message);
    let modules_referenced = project_map.modules.iter().map(|m| m.name.clone()).collect();

    Ok(ContextBundle {
        enriched_message,
        task_type,
        token_estimate,
        modules_referenced,
    })
}

/// Extracts DUUMBI error codes (E001–E099) from a request string.
fn extract_error_codes(request: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for word in request.split_whitespace() {
        let upper = word.to_uppercase();
        if upper.len() == 4
            && upper.starts_with('E')
            && upper[1..].chars().all(|c| c.is_ascii_digit())
        {
            codes.push(upper);
        }
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn assemble_context_empty_workspace() {
        let tmp = TempDir::new().expect("temp dir");
        // Create minimal .duumbi structure
        std::fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("mkdir");

        let result = assemble_context("add multiply function", tmp.path(), &[]);
        assert!(result.is_ok());
        let bundle = result.expect("must succeed");
        assert_eq!(bundle.task_type, classifier::TaskType::AddFunction);
        assert!(bundle.enriched_message.contains("add multiply function"));
    }

    #[test]
    fn assemble_context_with_session_history() {
        let tmp = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("mkdir");

        let history = vec![PersistentTurn {
            request: "add helper".to_string(),
            summary: "added helper function".to_string(),
            timestamp: chrono::Utc::now(),
            task_type: "AddFunction".to_string(),
        }];

        let result = assemble_context("modify main", tmp.path(), &history);
        assert!(result.is_ok());
        let bundle = result.expect("must succeed");
        assert!(bundle.enriched_message.contains("add helper"));
    }

    #[test]
    fn assemble_context_determinism() {
        let tmp = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("mkdir");

        let r1 = assemble_context("add foo", tmp.path(), &[]).expect("first");
        let r2 = assemble_context("add foo", tmp.path(), &[]).expect("second");
        assert_eq!(r1.enriched_message, r2.enriched_message);
        assert_eq!(r1.task_type, r2.task_type);
    }
}
