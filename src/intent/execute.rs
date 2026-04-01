//! Intent execution pipeline.
//!
//! `duumbi intent execute [name]` loads an intent spec, decomposes it into
//! tasks via the Coordinator, runs each task through the mutation orchestrator
//! with 3-step retry, then verifies test cases with the Verifier Agent.

use std::path::Path;

use anyhow::{Context, Result};

use serde_json::json;

use owo_colors::OwoColorize;

use crate::agents::agent_knowledge::{AgentKnowledgeStore, FailurePattern};
use crate::agents::analyzer as agent_analyzer;
use crate::agents::assembler;
use crate::agents::template::TemplateStore;
use crate::agents::{LlmProvider, orchestrator};
use crate::context;
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
/// Status messages are appended to `log` and also sent to `on_progress`
/// (if provided) for real-time display. The CLI uses `eprintln!` for
/// immediate output; the REPL collects them in its output buffer.
pub async fn run_execute(
    client: &dyn LlmProvider,
    workspace: &Path,
    slug: &str,
    log: &mut Vec<String>,
) -> Result<bool> {
    run_execute_with_progress(client, workspace, slug, log, &|_| {}).await
}

/// Like [`run_execute`] but with a real-time progress callback.
///
/// Each status line is passed to `on_progress` immediately when generated,
/// in addition to being collected in `log`.
pub async fn run_execute_with_progress(
    client: &dyn LlmProvider,
    workspace: &Path,
    slug: &str,
    log: &mut Vec<String>,
    on_progress: &(dyn Fn(&str) + Send + Sync),
) -> Result<bool> {
    // Helper: push to log AND emit via callback for real-time display.
    macro_rules! emit {
        ($msg:expr) => {{
            let s: String = $msg;
            on_progress(&s);
            log.push(s);
        }};
    }

    let graph_path = workspace.join(".duumbi/graph/main.jsonld");

    // 1. Load spec
    let mut spec = load_intent(workspace, slug).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;

    emit!(format!("Executing intent: \"{}\"", spec.intent));

    // 2. Mark in progress + save snapshot
    spec.status = IntentStatus::InProgress;
    save_intent(workspace, slug, &spec).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;

    let source_str = std::fs::read_to_string(&graph_path)
        .with_context(|| format!("Cannot read '{}'", graph_path.display()))?;
    snapshot::save_snapshot(workspace, &source_str).context("Failed to save snapshot")?;

    // 3. Analyze task profile and decompose into tasks
    let profile = agent_analyzer::analyze(&spec);
    let template_store = TemplateStore::load(workspace);
    let team = assembler::assemble(&profile, &template_store);
    emit!(format!(
        "Task profile: {:?} | {:?} | {:?} | {:?}",
        profile.complexity, profile.task_type, profile.scope, profile.risk
    ));
    emit!(format!(
        "Agent team: {:?} ({:?})",
        team.agents, team.strategy
    ));

    let mut tasks = coordinator::decompose(&spec);
    let total = tasks.len();
    emit!(format!(
        "Plan ({total} task{}):",
        if total == 1 { "" } else { "s" }
    ));
    for t in &tasks {
        emit!(format!("  [{}/{}] {}", t.id, total, t.description));
    }
    emit!(String::new());

    // 4. Execute each task
    let graph_dir = workspace.join(".duumbi/graph");
    let mut tasks_completed = 0;
    for task in &mut tasks {
        emit!(format!("[{}/{}] {}…", task.id, total, task.description));
        emit!(format!("  Calling LLM (provider: {})…", client.name()));
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

        // Context enrichment: add module signatures, few-shot examples from
        // past successes, and relevant graph fragments via the Phase 10 pipeline.
        match context::assemble_context(&prompt, workspace, &[]) {
            Ok(bundle) => {
                emit!(format!(
                    "  Context: ~{} tokens, {} module(s), {} few-shot example(s)",
                    bundle.token_estimate,
                    bundle.modules_referenced.len(),
                    bundle
                        .enriched_message
                        .matches("Similar successful mutations")
                        .count()
                ));
                prompt = bundle.enriched_message;
            }
            Err(e) => {
                // Non-fatal: fall back to the base prompt if context assembly fails.
                emit!(format!("  Context assembly skipped: {e}"));
            }
        }

        // Streaming callback collects LLM text chunks into the log buffer.
        let log_clone = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let log_tx = log_clone.clone();
        let result = orchestrator::mutate_streaming(
            client,
            &source,
            &prompt,
            3,
            skip_call_validation,
            move |text| {
                log_tx
                    .lock()
                    .expect("invariant: mutex not poisoned")
                    .push(text.to_string());
            },
        )
        .await;

        // Drain streamed chunks into log.
        if let Ok(chunks) = log_clone.lock()
            && !chunks.is_empty()
        {
            emit!(chunks.concat());
        }

        match result {
            Ok(orchestrator::MutationOutcome::NeedsClarification(question)) => {
                emit!(format!("  ⚠ Clarification needed: {question}"));
                emit!(
                    "    Intent execution does not support interactive clarification.".to_string()
                );
                task.status = TaskStatus::Failed(format!("Clarification needed: {question}"));

                spec.status = IntentStatus::Failed;
                save_intent(workspace, slug, &spec)
                    .map_err(|ie: IntentError| anyhow::anyhow!("{ie}"))?;

                emit!(format!("Intent failed at task {}/{}.", task.id, total));
                return Ok(false);
            }
            Ok(orchestrator::MutationOutcome::Success(mut mutation_result)) => {
                if is_create_module {
                    ensure_exports(&mut mutation_result.patched);

                    let expected_fns = expected_exports_for_module(&spec, &task.kind);
                    let missing = find_missing_functions(&mutation_result.patched, &expected_fns);

                    if !missing.is_empty() {
                        emit!(format!(
                            "  ⚠ Missing functions: [{}]. Retrying…",
                            missing.join(", ")
                        ));
                        let retry_prompt = format!(
                            "{}\n\nCRITICAL: The previous attempt only created some functions. \
                             The following functions are STILL MISSING and MUST be added: [{}]. \
                             Add ALL missing functions in this single response using add_function tool calls.",
                            prompt,
                            missing.join(", ")
                        );
                        let retry_log =
                            std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
                        let retry_tx = retry_log.clone();
                        let retry_result = orchestrator::mutate_streaming(
                            client,
                            &mutation_result.patched,
                            &retry_prompt,
                            1,
                            skip_call_validation,
                            move |text| {
                                retry_tx
                                    .lock()
                                    .expect("invariant: mutex not poisoned")
                                    .push(text.to_string());
                            },
                        )
                        .await;

                        if let Ok(chunks) = retry_log.lock()
                            && !chunks.is_empty()
                        {
                            emit!(chunks.concat());
                        }

                        if let Ok(orchestrator::MutationOutcome::Success(mut retry_mr)) =
                            retry_result
                        {
                            ensure_exports(&mut retry_mr.patched);
                            mutation_result = retry_mr;
                        }
                    }
                }

                let patched_str = serde_json::to_string_pretty(&mutation_result.patched)
                    .context("Serialize patched graph")?;
                std::fs::write(&target_path, &patched_str)
                    .with_context(|| format!("Write '{}'", target_path.display()))?;

                let diff = orchestrator::describe_changes(&source, &mutation_result.patched);
                emit!(format!(
                    "  {} Done ({} op{}). {}",
                    "\u{2713}".green().bold(),
                    mutation_result.ops_count,
                    if mutation_result.ops_count == 1 {
                        ""
                    } else {
                        "s"
                    },
                    diff.lines().next().unwrap_or("")
                ));
                task.status = TaskStatus::Completed;
                tasks_completed += 1;

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
                // Enrich record with function names from intent test cases.
                record.functions = spec
                    .test_cases
                    .iter()
                    .map(|tc| tc.function.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                record.retry_count = mutation_result.retry_count;
                record.error_codes = mutation_result.error_codes_encountered.clone();
                let _ = learning::append_success(workspace, &record);
            }
            Err(e) => {
                emit!(format!("  {} Task failed: {e:#}", "\u{2717}".red().bold()));
                task.status = TaskStatus::Failed(e.to_string());

                spec.status = IntentStatus::Failed;
                save_intent(workspace, slug, &spec)
                    .map_err(|ie: IntentError| anyhow::anyhow!("{ie}"))?;

                emit!(format!("Intent failed at task {}/{}.", task.id, total));
                emit!("(Use `duumbi undo` to revert the graph to before this intent.)".to_string());
                return Ok(false);
            }
        }
    }

    emit!(format!(
        "All {tasks_completed} task{} completed.",
        if tasks_completed == 1 { "" } else { "s" }
    ));

    // 5. Run verifier
    if spec.test_cases.is_empty() {
        emit!("No test cases defined — skipping verification.".to_string());
        archive_success(workspace, slug, tasks_completed, 0, 0)?;
        return Ok(true);
    }

    emit!(format!(
        "Running {} test{}…",
        spec.test_cases.len(),
        if spec.test_cases.len() == 1 { "" } else { "s" }
    ));
    let mut report = verifier::run_tests(&spec, workspace);
    emit!(report.display());

    // --- Repair cycle: if some tests failed, attempt one LLM repair ---
    if !report.all_passed() && report.failed > 0 {
        let failed_details: Vec<String> = report
            .results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| {
                if let Some(ref err) = r.error {
                    format!(
                        "- {}({}): error — {}",
                        r.function,
                        r.args
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        err
                    )
                } else {
                    format!(
                        "- {}({}) = {} (expected {})",
                        r.function,
                        r.args
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        r.actual.map_or("?".to_string(), |v| v.to_string()),
                        r.expected
                    )
                }
            })
            .collect();

        emit!(format!(
            "[Repair] Attempting repair for {} failed test(s)…",
            report.failed
        ));
        emit!(format!("  Calling LLM (provider: {})…", client.name()));

        let repair_prompt = format!(
            "REVIEW FIRST: Before making changes, inspect the semantic graph and identify \
             type errors, missing return ops, orphan references, unexported functions, \
             and structural issues. Then apply the minimal fix.\n\n\
             The following test cases FAILED after intent execution. \
             Fix the graph so ALL tests pass.\n\n\
             Failed tests:\n{}\n\n\
             Common fixes:\n\
             - E010 (unresolved reference): add missing function name to duumbi:exports array\n\
             - Wrong return value: check the algorithm logic in the function's blocks\n\
             - Compile error: check SSA ordering (ops must reference lower-index ops only)\n\n\
             Do NOT recreate functions that already work — only fix the broken behavior. \
             Use replace_block to rewrite blocks that produce wrong results.",
            failed_details.join("\n")
        );

        // Attempt repair on all module files (bug may be in library or main)
        let mut repaired = false;
        for entry in std::fs::read_dir(&graph_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonld") {
                continue;
            }

            let module_source: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path)?)
                    .context("Failed to parse module for repair")?;

            let repair_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let repair_tx = repair_log.clone();
            let repair_result = orchestrator::mutate_streaming(
                client,
                &module_source,
                &repair_prompt,
                1,
                true, // skip call validation
                move |text| {
                    repair_tx
                        .lock()
                        .expect("invariant: mutex not poisoned")
                        .push(text.to_string());
                },
            )
            .await;

            if let Ok(chunks) = repair_log.lock()
                && !chunks.is_empty()
            {
                emit!(chunks.concat());
            }

            if let Ok(orchestrator::MutationOutcome::Success(mr)) = repair_result {
                let patched_str = serde_json::to_string_pretty(&mr.patched)
                    .context("Serialize repaired graph")?;
                std::fs::write(&path, &patched_str)
                    .with_context(|| format!("Write repaired '{}'", path.display()))?;
                repaired = true;
                emit!(format!(
                    "  {} Repair applied to {} ({} op{}).",
                    "\u{2713}".green().bold(),
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    mr.ops_count,
                    if mr.ops_count == 1 { "" } else { "s" }
                ));
            }
        }

        if repaired {
            emit!("[Repair] Re-running tests…".to_string());
            report = verifier::run_tests(&spec, workspace);
            emit!(report.display());
        } else {
            emit!("  No repair patches applied.".to_string());
        }
    }

    let all_passed = report.all_passed();
    archive_success(
        workspace,
        slug,
        tasks_completed,
        report.passed,
        report.passed + report.failed,
    )?;

    if all_passed {
        emit!("Intent completed successfully.".to_string());
    } else {
        // Record failure patterns for future learning.
        let error_codes: Vec<String> = report
            .results
            .iter()
            .filter(|r| !r.passed)
            .filter_map(|r| {
                r.error.as_ref().and_then(|e| {
                    // Extract error code like E010, E009 from error message
                    e.split_whitespace()
                        .find(|w| w.starts_with("[E") && w.ends_with(']'))
                        .map(|c| c.trim_matches(|ch| ch == '[' || ch == ']').to_string())
                })
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !error_codes.is_empty() {
            let pattern_desc = format!(
                "Intent '{}' failed with {} test(s): {}",
                spec.intent,
                report.failed,
                error_codes.join(", ")
            );
            let mitigation = report
                .results
                .iter()
                .filter(|r| !r.passed)
                .filter_map(|r| r.error.clone())
                .collect::<Vec<_>>()
                .join("; ");

            let pattern = FailurePattern::new(
                "duumbi:template/coder",
                &pattern_desc,
                error_codes,
                &mitigation,
            );
            let _ = AgentKnowledgeStore::save_failure_pattern(workspace, &pattern);
        }

        spec.status = IntentStatus::Failed;
        save_intent(workspace, slug, &spec).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;
        emit!(format!(
            "Intent failed: {} test(s) did not pass.",
            report.failed
        ));
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
// Post-mutation validation helpers
// ---------------------------------------------------------------------------

/// Returns the list of function names that a CreateModule task should produce,
/// based on the intent spec's test cases.
fn expected_exports_for_module(spec: &IntentSpec, task_kind: &TaskKind) -> Vec<String> {
    let _module_name = match task_kind {
        TaskKind::CreateModule { module_name } => module_name,
        _ => return Vec::new(),
    };

    // Collect all non-main function names from test cases
    spec.test_cases
        .iter()
        .map(|tc| tc.function.as_str())
        .filter(|&f| f != "main")
        .map(|f| f.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Checks which expected function names are missing from a module's duumbi:functions.
fn find_missing_functions(module: &serde_json::Value, expected: &[String]) -> Vec<String> {
    let present: std::collections::HashSet<&str> = module["duumbi:functions"]
        .as_array()
        .map(|funcs| {
            funcs
                .iter()
                .filter_map(|f| f["duumbi:name"].as_str())
                .collect()
        })
        .unwrap_or_default();

    expected
        .iter()
        .filter(|name| !present.contains(name.as_str()))
        .cloned()
        .collect()
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
