//! Intent execution pipeline.
//!
//! `duumbi intent execute [name]` loads an intent spec, decomposes it into
//! tasks via the Coordinator, runs each task through the mutation orchestrator
//! with 3-step retry, then verifies test cases with the Verifier Agent.

use std::path::Path;

use anyhow::{Context, Result};

use serde_json::json;

use crate::agents::{LlmProvider, orchestrator};
use crate::intent::coordinator;
use crate::intent::spec::{ExecutionMeta, IntentSpec, IntentStatus, TaskKind, TaskStatus};
use crate::intent::verifier;
use crate::intent::{IntentError, load_intent, save_intent};
use crate::knowledge::learning;
use crate::knowledge::types::SuccessRecord;
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
pub async fn run_execute(client: &dyn LlmProvider, workspace: &Path, slug: &str) -> Result<bool> {
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
    let graph_dir = workspace.join(".duumbi/graph");
    let mut tasks_completed = 0;
    for task in &mut tasks {
        eprintln!("[{}/{}] {}…", task.id, total, task.description);
        task.status = TaskStatus::InProgress;

        // For CreateModule tasks, use an empty module template as source and
        // write the result to a new file. For other tasks, mutate main.jsonld.
        let (source, target_path) = match &task.kind {
            TaskKind::CreateModule { module_name } => {
                let file_name = module_name_to_filename(module_name);
                let target = graph_dir.join(&file_name);
                let template = empty_module_template(module_name);
                (template, target)
            }
            _ => {
                let source: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&graph_path)?)
                        .context("Failed to parse current graph")?;
                (source, graph_path.clone())
            }
        };

        let mut prompt = build_task_prompt(&spec, task.mutation_prompt().as_str());
        // Intent execution is always multi-module: skip intra-module Call
        // validation for all tasks. Cross-module call resolution is handled
        // by Program::load and the verifier, not the single-module builder.
        let skip_call_validation = true;

        // For non-library tasks, tell the LLM about available exports from
        // other modules so it knows these functions exist and should only be
        // called, not re-defined.
        let is_create_module = matches!(&task.kind, TaskKind::CreateModule { .. });
        if !is_create_module {
            let exports_summary = collect_module_exports(&graph_dir);
            if !exports_summary.is_empty() {
                prompt.push_str(&format!(
                    "\n\nAvailable functions from other modules (do NOT re-define these, \
                     just call them):\n{exports_summary}\n\
                     IMPORTANT: When creating Call ops to these functions, use ONLY the plain \
                     function name in \"duumbi:function\" (e.g., \"add\", NOT \"ops:add\" or \
                     \"module:add\"). The module resolution is automatic."
                ));
            }
        }

        let result = orchestrator::mutate_streaming(
            client,
            &source,
            &prompt,
            3,
            skip_call_validation,
            |text| {
                eprint!("{text}");
            },
        )
        .await;

        eprintln!(); // newline after streamed output

        match result {
            Ok(orchestrator::MutationOutcome::NeedsClarification(question)) => {
                eprintln!("  ⚠ Clarification needed: {question}");
                eprintln!("    Intent execution does not support interactive clarification.");
                task.status = TaskStatus::Failed(format!("Clarification needed: {question}"));

                // Halt intent — can't proceed without user input
                spec.status = IntentStatus::Failed;
                save_intent(workspace, slug, &spec)
                    .map_err(|ie: IntentError| anyhow::anyhow!("{ie}"))?;

                eprintln!();
                eprintln!("Intent failed at task {}/{}.", task.id, total);
                return Ok(false);
            }
            Ok(orchestrator::MutationOutcome::Success(mut mutation_result)) => {
                // For library modules, ensure all functions are exported
                if is_create_module {
                    ensure_exports(&mut mutation_result.patched);
                }

                // Auto-wire: add new module as local dependency in config.toml (#354)
                if let TaskKind::CreateModule { module_name } = &task.kind {
                    auto_wire_module(workspace, module_name);
                }

                // Write patched graph to the appropriate file
                let patched_str = serde_json::to_string_pretty(&mutation_result.patched)
                    .context("Serialize patched graph")?;
                std::fs::write(&target_path, &patched_str)
                    .with_context(|| format!("Write '{}'", target_path.display()))?;

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

                // Log success for few-shot learning (#363)
                let task_type_str = match &task.kind {
                    TaskKind::CreateModule { .. } => "CreateModule",
                    TaskKind::AddFunction { .. } => "AddFunction",
                    TaskKind::ModifyFunction { .. } => "ModifyFunction",
                    TaskKind::ModifyMain { .. } => "ModifyMain",
                };
                let mut record = SuccessRecord::new(&task.description, task_type_str);
                record.ops_count = mutation_result.ops_count;
                record.module = match &task.kind {
                    TaskKind::CreateModule { module_name } => module_name.clone(),
                    _ => "main".to_string(),
                };
                // Best-effort: don't fail the intent if logging fails
                let _ = learning::append_success(workspace, &record);
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

/// Ensures all functions in a library module are listed in `duumbi:exports`.
///
/// LLMs frequently forget to populate the exports array even when prompted.
/// This post-processing step deterministically collects all function names
/// from `duumbi:functions` and sets them as the exports list.
fn ensure_exports(module: &mut serde_json::Value) {
    let function_names: Vec<serde_json::Value> = module["duumbi:functions"]
        .as_array()
        .map(|funcs| {
            funcs
                .iter()
                .filter_map(|f| f["duumbi:name"].as_str().map(|s| json!(s)))
                .collect()
        })
        .unwrap_or_default();

    module["duumbi:exports"] = serde_json::Value::Array(function_names);
}

/// Converts a module name like `"calculator/ops"` to a flat filename `"calculator_ops.jsonld"`.
fn module_name_to_filename(module_name: &str) -> String {
    let sanitized = module_name.replace('/', "_");
    format!("{sanitized}.jsonld")
}

/// Creates an empty module template for a new module.
///
/// The template includes `duumbi:exports` as an empty array — the LLM is
/// expected to populate it with the names of functions it creates. The
/// system prompt in [`build_task_prompt`] reminds the LLM to do this.
fn empty_module_template(module_name: &str) -> serde_json::Value {
    // Use the last path component as the short name for ids
    let short_name = module_name.rsplit('/').next().unwrap_or(module_name);

    json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": format!("duumbi:{short_name}"),
        "duumbi:name": short_name,
        "duumbi:exports": [],
        "duumbi:functions": []
    })
}

/// Scans the graph directory for non-main `.jsonld` modules and collects their
/// exported function names + parameter signatures.
///
/// Returns a human-readable summary like:
/// ```text
/// - module "ops": add(a: i64, b: i64) -> i64, multiply(a: i64, b: i64) -> i64
/// ```
fn collect_module_exports(graph_dir: &Path) -> String {
    let entries = match std::fs::read_dir(graph_dir) {
        Ok(e) => e,
        Err(_) => return String::new(),
    };

    let mut lines = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonld") {
            continue;
        }
        if path
            .file_name()
            .map(|f| f == "main.jsonld")
            .unwrap_or(false)
        {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let module_name = value["duumbi:name"].as_str().unwrap_or("unknown");
        let exports: Vec<&str> = value["duumbi:exports"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        if exports.is_empty() {
            continue;
        }

        // Build function signatures from duumbi:functions
        let mut sigs = Vec::new();
        if let Some(funcs) = value["duumbi:functions"].as_array() {
            for func in funcs {
                let fname = match func["duumbi:name"].as_str() {
                    Some(n) if exports.contains(&n) => n,
                    _ => continue,
                };
                let ret_type = func["duumbi:returnType"].as_str().unwrap_or("i64");
                let params: Vec<String> = func["duumbi:params"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| {
                                let name = p["duumbi:name"].as_str()?;
                                let ptype = p["duumbi:paramType"].as_str().unwrap_or("i64");
                                Some(format!("{name}: {ptype}"))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                sigs.push(format!("{}({}) -> {}", fname, params.join(", "), ret_type));
            }
        }

        if !sigs.is_empty() {
            lines.push(format!(
                "- from module \"{}\": {} (call with plain name, e.g. \"add\" not \"{}:add\")",
                module_name,
                sigs.join(", "),
                module_name
            ));
        }
    }

    lines.join("\n")
}

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
// Auto-wiring (#354)
// ---------------------------------------------------------------------------

/// Adds a newly created module as a local path dependency in `config.toml`.
///
/// This is a deterministic post-processing step — no LLM involved.
/// Best-effort: logs a warning if the config cannot be updated.
fn auto_wire_module(workspace: &Path, module_name: &str) {
    let config_path = workspace.join(".duumbi/config.toml");
    if !config_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Check if dependency already exists
    if content.contains(&format!("\"{module_name}\"")) {
        return;
    }

    // Compute the relative path from workspace to the module's graph dir
    let file_name = module_name_to_filename(module_name);
    let rel_path = format!(".duumbi/graph/{file_name}");

    // Append dependency to [dependencies] section
    let dep_line = format!("\"{module_name}\" = {{ path = \"{rel_path}\" }}\n");

    let updated = if content.contains("[dependencies]") {
        // Insert after [dependencies] line
        content.replacen(
            "[dependencies]\n",
            &format!("[dependencies]\n{dep_line}"),
            1,
        )
    } else {
        // Add [dependencies] section
        format!("{content}\n[dependencies]\n{dep_line}")
    };

    if std::fs::write(&config_path, updated).is_ok() {
        eprintln!("  Auto-wired: added \"{module_name}\" to config.toml dependencies");
    }
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
