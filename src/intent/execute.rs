//! Intent execution pipeline.
//!
//! `duumbi intent execute [name]` loads an intent spec, decomposes it into
//! tasks via the Coordinator, runs each task through the mutation orchestrator
//! with 3-step retry, then verifies test cases with the Verifier Agent.

use std::path::Path;

use anyhow::{Context, Result};

use crate::agents::{LlmClient, orchestrator};
use crate::intent::coordinator;
use crate::intent::spec::{ExecutionMeta, IntentSpec, IntentStatus, TaskStatus};
use crate::intent::verifier;
use crate::intent::{IntentError, load_intent, save_intent};
use crate::snapshot;

// ---------------------------------------------------------------------------
// Execution entry point
// ---------------------------------------------------------------------------

/// Executes an intent spec end-to-end.
///
/// Flow:
/// 1. Load spec → mark `InProgress`
/// 2. Save snapshot (rollback point)
/// 3. Coordinator decomposes spec → `Vec<Task>`
/// 4. For each task: mutate graph with 3-step retry
/// 5. Verifier runs test cases
/// 6. If all pass → archive as `Completed`; otherwise mark `Failed`
///
/// Returns `Ok(true)` if all tasks and tests passed, `Ok(false)` if failed.
pub async fn run_execute(client: &LlmClient, workspace: &Path, slug: &str) -> Result<bool> {
    let graph_path = workspace.join(".duumbi/graph/main.jsonld");

    // 1. Load spec
    let mut spec = load_intent(workspace, slug).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;

    eprintln!();
    eprintln!("Executing intent: \"{}\"", spec.intent);
    eprintln!();

    // 2. Mark in progress + save snapshot
    spec.status = IntentStatus::InProgress;
    save_intent(workspace, slug, &spec).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;

    let source_str = std::fs::read_to_string(&graph_path)
        .with_context(|| format!("Cannot read '{}'", graph_path.display()))?;
    snapshot::save_snapshot(workspace, &source_str).context("Failed to save snapshot")?;

    // 3. Decompose into tasks
    let mut tasks = coordinator::decompose(&spec);
    let total = tasks.len();
    eprintln!("Plan ({total} task{}):", if total == 1 { "" } else { "s" });
    for t in &tasks {
        eprintln!("  [{}/{}] {}", t.id, total, t.description);
    }
    eprintln!();

    // 4. Execute each task
    let mut tasks_completed = 0;
    for task in &mut tasks {
        eprintln!("[{}/{}] {}…", task.id, total, task.description);
        task.status = TaskStatus::InProgress;

        let source: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&graph_path)?)
                .context("Failed to parse current graph")?;

        let prompt = build_task_prompt(&spec, task.mutation_prompt().as_str());

        let result = orchestrator::mutate_streaming(client, &source, &prompt, 3, |text| {
            eprint!("{text}");
        })
        .await;

        eprintln!(); // newline after streamed output

        match result {
            Ok(mutation_result) => {
                // Write patched graph
                let patched_str = serde_json::to_string_pretty(&mutation_result.patched)
                    .context("Serialize patched graph")?;
                std::fs::write(&graph_path, &patched_str)
                    .with_context(|| format!("Write '{}'", graph_path.display()))?;

                let diff = orchestrator::describe_changes(&source, &mutation_result.patched);
                eprintln!(
                    "  ✓ Done ({} op{}). {}",
                    mutation_result.ops_count,
                    if mutation_result.ops_count == 1 {
                        ""
                    } else {
                        "s"
                    },
                    diff.lines().next().unwrap_or("")
                );
                task.status = TaskStatus::Completed;
                tasks_completed += 1;
            }
            Err(e) => {
                eprintln!("  ✗ Task failed: {e:#}");
                task.status = TaskStatus::Failed(e.to_string());

                // Mark intent failed and exit
                spec.status = IntentStatus::Failed;
                save_intent(workspace, slug, &spec)
                    .map_err(|ie: IntentError| anyhow::anyhow!("{ie}"))?;

                eprintln!();
                eprintln!("Intent failed at task {}/{}.", task.id, total);
                eprintln!("(Use `duumbi undo` to revert the graph to before this intent.)");
                return Ok(false);
            }
        }
    }

    eprintln!();
    eprintln!(
        "All {tasks_completed} task{} completed.",
        if tasks_completed == 1 { "" } else { "s" }
    );

    // 5. Run verifier
    if spec.test_cases.is_empty() {
        eprintln!("No test cases defined — skipping verification.");

        archive_success(workspace, slug, tasks_completed, 0, 0)?;
        return Ok(true);
    }

    eprintln!();
    eprintln!(
        "Running {} test{}…",
        spec.test_cases.len(),
        if spec.test_cases.len() == 1 { "" } else { "s" }
    );
    let report = verifier::run_tests(&spec, workspace);
    eprintln!("{}", report.display());

    let all_passed = report.all_passed();
    archive_success(
        workspace,
        slug,
        tasks_completed,
        report.passed,
        report.passed + report.failed,
    )?;

    if all_passed {
        eprintln!("Intent completed successfully.");
    } else {
        spec.status = IntentStatus::Failed;
        save_intent(workspace, slug, &spec).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;
        eprintln!("Intent failed: {} test(s) did not pass.", report.failed);
    }

    Ok(all_passed)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds the full mutation prompt for a task, including the intent context.
fn build_task_prompt(spec: &IntentSpec, task_prompt: &str) -> String {
    let criteria = spec
        .acceptance_criteria
        .iter()
        .enumerate()
        .map(|(i, c)| format!("  {}. {c}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Intent: \"{}\"\n\nAcceptance criteria:\n{}\n\nCurrent task:\n{}",
        spec.intent, criteria, task_prompt
    )
}

/// Archives a successfully completed intent.
fn archive_success(
    workspace: &Path,
    slug: &str,
    tasks_completed: usize,
    tests_passed: usize,
    tests_total: usize,
) -> Result<()> {
    let now = crate::intent::create::chrono_now_pub();

    crate::intent::status::archive_intent(
        workspace,
        slug,
        ExecutionMeta {
            completed_at: now,
            tasks_completed,
            tests_passed,
            tests_total,
        },
    )
    .map_err(|e: IntentError| anyhow::anyhow!("{e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus};

    #[test]
    fn build_task_prompt_includes_criteria() {
        let spec = IntentSpec {
            intent: "Build calculator".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["add(a,b) returns a+b".to_string()],
            modules: IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            created_at: None,
            execution: None,
        };
        let prompt = build_task_prompt(&spec, "Create module ops");
        assert!(prompt.contains("Build calculator"));
        assert!(prompt.contains("add(a,b) returns a+b"));
        assert!(prompt.contains("Create module ops"));
    }
}
