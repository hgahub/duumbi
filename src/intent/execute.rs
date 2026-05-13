//! Intent execution pipeline.
//!
//! `duumbi intent execute [name]` loads an intent spec, decomposes it into
//! tasks via the Coordinator, runs each task through the mutation orchestrator
//! with 3-step retry, then verifies test cases with the Verifier Agent.

use std::path::{Path, PathBuf};

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
use crate::knowledge::types::{FailureRecord, SuccessRecord};
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
    let provider_kind = crate::config::ProviderKind::from_provider_name(client.name());
    let agent_policy = crate::config::load_effective_config(workspace)
        .map(|effective| {
            effective
                .config
                .effective_agent_policy(provider_kind.as_ref())
        })
        .unwrap_or_default();

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
                let target = module_name_to_path(&graph_dir, module_name);
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
                     IMPORTANT: When creating cross-module Call ops to these functions, set \
                     \"duumbi:module\" to the owning module name and keep \"duumbi:function\" \
                     as the plain function name."
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
        // AI-AGENT: Arc<Mutex<>> is intentional here — the closure passed to
        // mutate_streaming() must be 'static + Send + Sync, so we cannot borrow
        // `log` directly. The Mutex guards the chunk buffer; it is drained once
        // after the await returns. Do NOT replace with a channel: we need the
        // chunks collected in order AND accessible after the future completes.
        let log_clone = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let log_tx = log_clone.clone();
        let result = orchestrator::mutate_streaming_with_timeout(
            client,
            &source,
            &prompt,
            agent_policy.mutation_retries,
            agent_policy.mutation_timeout_secs,
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
                record_task_failure(
                    workspace,
                    client,
                    &task.description,
                    &task.kind,
                    &spec,
                    "clarification_needed",
                    agent_policy.mutation_retries,
                    Vec::new(),
                    &question,
                );

                spec.status = IntentStatus::Failed;
                save_intent(workspace, slug, &spec)
                    .map_err(|ie: IntentError| anyhow::anyhow!("{ie}"))?;

                emit!(format!("Intent failed at task {}/{}.", task.id, total));
                return Ok(false);
            }
            Ok(orchestrator::MutationOutcome::Success(mut mutation_result)) => {
                if is_create_module {
                    let expected_fns = expected_exports_for_module(&spec, &task.kind);
                    if let TaskKind::CreateModule { module_name } = &task.kind {
                        cleanup_create_module_output(
                            &mut mutation_result.patched,
                            module_name,
                            should_remove_library_main_for_spec(&spec, module_name),
                        );
                    }

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
                        let retry_result = orchestrator::mutate_streaming_with_timeout(
                            client,
                            &mutation_result.patched,
                            &retry_prompt,
                            agent_policy.repair_retries,
                            agent_policy.mutation_timeout_secs,
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
                            if let TaskKind::CreateModule { module_name } = &task.kind {
                                cleanup_create_module_output(
                                    &mut retry_mr.patched,
                                    module_name,
                                    should_remove_library_main_for_spec(&spec, module_name),
                                );
                            }
                            mutation_result = retry_mr;
                        }
                    }
                }

                let patched_str = serde_json::to_string_pretty(&mutation_result.patched)
                    .context("Serialize patched graph")?;
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Create '{}'", parent.display()))?;
                }
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
                let _ = learning::append_success_with_user_cache(workspace, &record);
            }
            Err(e) => {
                emit!(format!("  {} Task failed: {e:#}", "\u{2717}".red().bold()));
                task.status = TaskStatus::Failed(e.to_string());
                let summary = format!("{e:#}");
                record_task_failure(
                    workspace,
                    client,
                    &task.description,
                    &task.kind,
                    &spec,
                    &classify_failure_category(&summary),
                    agent_policy.mutation_retries,
                    extract_error_codes_from_text(&summary),
                    &summary,
                );

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

        let repair_prompt = build_repair_prompt(&spec, &failed_details);

        // Attempt repair on all module files (bug may be in library or main)
        let mut repaired = false;
        for path in collect_jsonld_paths(&graph_dir) {
            let module_source: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path)?)
                    .context("Failed to parse module for repair")?;

            // AI-AGENT: Same Arc<Mutex> pattern as the main streaming callback above.
            let repair_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let repair_tx = repair_log.clone();
            let repair_result = orchestrator::mutate_streaming_with_timeout(
                client,
                &module_source,
                &repair_prompt,
                agent_policy.repair_retries,
                agent_policy.mutation_timeout_secs,
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

            if let Ok(orchestrator::MutationOutcome::Success(mut mr)) = repair_result {
                cleanup_repaired_module_output(&mut mr.patched, &graph_dir, &path, &spec);
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
            let summary = failed_details.join("; ");
            record_intent_failure(
                workspace,
                client,
                &spec.intent,
                "VerifierRepair",
                "verifier_failure_after_repair",
                agent_policy.repair_retries,
                extract_error_codes_from_text(&summary),
                &summary,
                "all",
                intent_functions(&spec),
            );
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
        let summary = report.display();
        record_intent_failure(
            workspace,
            client,
            &spec.intent,
            "Verifier",
            "verifier_failure_after_repair",
            agent_policy.repair_retries,
            extract_error_codes_from_text(&summary),
            &summary,
            "all",
            intent_functions(&spec),
        );
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

fn cleanup_create_module_output(
    module: &mut serde_json::Value,
    module_name: &str,
    remove_library_main: bool,
) {
    if remove_library_main
        && !is_entry_module_name(module_name)
        && let Some(functions) = module["duumbi:functions"].as_array_mut()
    {
        functions.retain(|function| function["duumbi:name"].as_str() != Some("main"));
    }
    ensure_exports(module);
}

fn cleanup_repaired_module_output(
    module: &mut serde_json::Value,
    graph_dir: &Path,
    path: &Path,
    spec: &IntentSpec,
) {
    let Some(module_name) = graph_path_to_module_name(graph_dir, path) else {
        return;
    };
    if !is_entry_module_name(&module_name) {
        cleanup_create_module_output(
            module,
            &module_name,
            should_remove_library_main_for_spec(spec, &module_name),
        );
    }
}

fn should_remove_library_main_for_spec(spec: &IntentSpec, module_name: &str) -> bool {
    !is_entry_module_name(module_name)
        && spec
            .modules
            .create
            .iter()
            .any(|module| module == module_name)
        && crate::intent::benchmarks::expected_functions_for_benchmark(&spec.intent)
            .is_some_and(|functions| !functions.contains(&"main"))
}

fn is_entry_module_name(module_name: &str) -> bool {
    matches!(module_name.trim(), "main" | "app/main")
}

/// Converts a module name like `"calculator/ops"` to a nested graph path.
fn module_name_to_path(graph_dir: &Path, module_name: &str) -> PathBuf {
    graph_dir.join(module_name_to_relative_path(module_name))
}

fn module_name_to_relative_path(module_name: &str) -> PathBuf {
    let normalized = module_name.replace('\\', "/");
    let mut path = PathBuf::new();
    for raw_segment in normalized.split('/') {
        let segment = raw_segment.trim();
        if segment.is_empty() {
            continue;
        }
        let sanitized: String = segment
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
            continue;
        }
        path.push(sanitized);
    }
    if path.as_os_str().is_empty() {
        path.push("module");
    }
    path.set_extension("jsonld");
    path
}

fn graph_path_to_module_name(graph_dir: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(graph_dir).ok()?;
    if relative
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("jsonld")
    {
        return None;
    }

    let mut module_path = relative.to_path_buf();
    module_path.set_extension("");
    let segments: Option<Vec<&str>> = module_path.iter().map(|segment| segment.to_str()).collect();
    let module_name = segments?.join("/");
    if module_name.is_empty() {
        None
    } else {
        Some(module_name)
    }
}

/// Creates an empty module template for a new module.
///
/// The template includes `duumbi:exports` as an empty array — the LLM is
/// expected to populate it with the names of functions it creates. The
/// system prompt in [`build_task_prompt`] reminds the LLM to do this.
fn empty_module_template(module_name: &str) -> serde_json::Value {
    json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": format!("duumbi:{module_name}"),
        "duumbi:name": module_name,
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
    let mut lines = Vec::new();

    for path in collect_jsonld_paths(graph_dir) {
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
                "- from module \"{}\": {} (call with duumbi:module \"{}\" and plain duumbi:function)",
                module_name,
                sigs.join(", "),
                module_name
            ));
        }
    }

    lines.join("\n")
}

fn collect_jsonld_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_jsonld_paths_into(dir, &mut paths);
    paths
}

fn collect_jsonld_paths_into(dir: &Path, paths: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonld_paths_into(&path, paths);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonld") {
            paths.push(path);
        }
    }
}

fn task_type_name(task_kind: &TaskKind) -> &'static str {
    match task_kind {
        TaskKind::CreateModule { .. } => "CreateModule",
        TaskKind::AddFunction { .. } => "AddFunction",
        TaskKind::ModifyFunction { .. } => "ModifyFunction",
        TaskKind::ModifyMain { .. } => "ModifyMain",
    }
}

fn task_module_name(task_kind: &TaskKind) -> String {
    match task_kind {
        TaskKind::CreateModule { module_name } => module_name.clone(),
        _ => "main".to_string(),
    }
}

fn intent_functions(spec: &IntentSpec) -> Vec<String> {
    spec.test_cases
        .iter()
        .map(|tc| tc.function.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

fn classify_failure_category(summary: &str) -> String {
    let lower = summary.to_ascii_lowercase();
    if lower.contains("no tool calls") {
        "no_tool_calls".to_string()
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "provider_timeout".to_string()
    } else if lower.contains("status 401") || lower.contains("status 403") {
        "provider_auth".to_string()
    } else if lower.contains("status 429") || lower.contains("rate limited") {
        "provider_rate_limit".to_string()
    } else if lower.contains("status 5") {
        "provider_server_error".to_string()
    } else if lower.contains("validation failed") {
        "validation_retry_exhaustion".to_string()
    } else {
        "mutation_failed".to_string()
    }
}

fn extract_error_codes_from_text(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| {
            word.len() == 4
                && word.starts_with('E')
                && word[1..].chars().all(|c| c.is_ascii_digit())
        })
        .map(ToString::to_string)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn record_task_failure(
    workspace: &Path,
    client: &dyn LlmProvider,
    request: &str,
    task_kind: &TaskKind,
    spec: &IntentSpec,
    category: &str,
    retry_count: u32,
    error_codes: Vec<String>,
    summary: &str,
) {
    record_intent_failure(
        workspace,
        client,
        request,
        task_type_name(task_kind),
        category,
        retry_count,
        error_codes,
        summary,
        &task_module_name(task_kind),
        intent_functions(spec),
    );
}

#[allow(clippy::too_many_arguments)]
fn record_intent_failure(
    workspace: &Path,
    client: &dyn LlmProvider,
    request: &str,
    task_type: &str,
    category: &str,
    retry_count: u32,
    error_codes: Vec<String>,
    summary: &str,
    module: &str,
    functions: Vec<String>,
) {
    let mut record = FailureRecord::new(request, task_type, category);
    record.provider = client.name().to_string();
    record.model_label = client.model_label();
    record.module = module.to_string();
    record.functions = functions;
    record.retry_count = retry_count;
    record.error_codes = error_codes;
    record.error_summary = learning::sanitize_error_summary(summary);
    let _ = learning::append_failure_with_user_cache(workspace, &record);
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
    let context = spec
        .context
        .as_ref()
        .map(format_intent_context)
        .unwrap_or_else(|| "No additional clarified context.".to_string());
    let benchmark_guidance =
        benchmark_guidance_section(&spec.intent, "Benchmark-specific guidance");

    format!(
        "Intent: \"{}\"\n\nClarified context:\n{}\n\nAcceptance criteria:\n{}{}\n\nCurrent task:\n{}",
        spec.intent, context, criteria, benchmark_guidance, task_prompt
    )
}

fn build_repair_prompt(spec: &IntentSpec, failed_details: &[String]) -> String {
    let benchmark_guidance =
        benchmark_guidance_section(&spec.intent, "Benchmark-specific repair guidance");
    format!(
        "REVIEW FIRST: Before making changes, inspect the semantic graph and identify \
         type errors, missing return ops, orphan references, unexported functions, \
         and structural issues. Then apply the minimal fix.\n\n\
         The following test cases FAILED after intent execution. \
         Fix the graph so ALL tests pass.\n\n\
         Failed tests:\n{}\n\n\
         Common fixes:\n\
         - E010 (unresolved reference): add missing function name to duumbi:exports array\n\
         - Wrong return value: check the algorithm logic in the function's blocks\n\
         - Compile error: check SSA ordering (ops must reference lower-index ops only){}\n\n\
         Do NOT recreate functions that already work — only fix the broken behavior. \
         Use replace_block to rewrite blocks that produce wrong results.",
        failed_details.join("\n"),
        benchmark_guidance
    )
}

fn benchmark_guidance_section(intent: &str, heading: &str) -> String {
    crate::intent::benchmarks::guidance_for_benchmark(intent)
        .map(|guidance| format!("\n\n{heading}:\n{guidance}"))
        .unwrap_or_default()
}

fn format_intent_context(context: &crate::intent::spec::IntentContext) -> String {
    let mut lines = Vec::new();
    if let Some(scope) = &context.scope {
        lines.push(format!("- Scope: {scope}"));
    }
    if let Some(entrypoint) = &context.entrypoint {
        lines.push(format!("- Entrypoint: {entrypoint}"));
    }
    if let Some(surface) = &context.runtime_surface {
        lines.push(format!("- Runtime surface: {surface}"));
    }
    for point in &context.integration_points {
        lines.push(format!("- Integration point: {point}"));
    }
    for constraint in &context.constraints {
        lines.push(format!("- Constraint: {constraint}"));
    }
    if lines.is_empty() {
        "No additional clarified context.".to_string()
    } else {
        lines.join("\n")
    }
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
    let module_name = match task_kind {
        TaskKind::CreateModule { module_name } => module_name,
        _ => return Vec::new(),
    };

    let mut expected: std::collections::HashSet<String> = spec
        .test_cases
        .iter()
        .map(|tc| tc.function.as_str())
        .filter(|&f| f != "main")
        .map(|f| f.to_string())
        .collect();

    if spec.modules.create.iter().any(|m| m == module_name)
        && let Some(functions) =
            crate::intent::benchmarks::expected_functions_for_benchmark(&spec.intent)
    {
        expected.extend(functions.iter().map(|function| (*function).to_string()));
    }

    expected.into_iter().collect()
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
    use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TaskKind, TestCase};

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
            context: None,
            created_at: None,
            execution: None,
        };
        let prompt = build_task_prompt(&spec, "Create module ops");
        assert!(prompt.contains("Build calculator"));
        assert!(prompt.contains("add(a,b) returns a+b"));
        assert!(prompt.contains("Create module ops"));
    }

    #[test]
    fn build_task_prompt_includes_string_utils_benchmark_guidance() {
        let spec = IntentSpec {
            intent: "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec![r#"reverse("duumbi") demonstrates "ibmuud""#.to_string()],
            modules: IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            context: None,
            created_at: None,
            execution: None,
        };

        let prompt = build_task_prompt(&spec, "Create module string/utils");

        assert!(prompt.contains("Benchmark-specific guidance"));
        assert!(prompt.contains("representative sample behavior"));
        assert!(prompt.contains("does not support substring indexing"));
        assert!(prompt.contains(r#"reverse("duumbi")"#));
    }

    #[test]
    fn build_task_prompt_does_not_add_benchmark_guidance_for_generic_prompt() {
        let spec = IntentSpec {
            intent: "Create a parser".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["parse input".to_string()],
            modules: IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            context: None,
            created_at: None,
            execution: None,
        };

        let prompt = build_task_prompt(&spec, "Create module parser");

        assert!(!prompt.contains("Benchmark-specific guidance"));
        assert!(!prompt.contains("representative sample behavior"));
    }

    #[test]
    fn repair_prompt_includes_string_utils_benchmark_guidance() {
        let spec = IntentSpec {
            intent: "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: Vec::new(),
            modules: IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            context: None,
            created_at: None,
            execution: None,
        };
        let failed = vec!["- main() = -1 (expected 0)".to_string()];

        let prompt = build_repair_prompt(&spec, &failed);

        assert!(prompt.contains("Benchmark-specific repair guidance"));
        assert!(prompt.contains("Return ConstI64(0)"));
        assert!(prompt.contains("Do NOT recreate functions"));
    }

    #[test]
    fn module_name_to_relative_path_preserves_slash_modules() {
        assert_eq!(
            module_name_to_relative_path("calculator/ops"),
            PathBuf::from("calculator/ops.jsonld")
        );
        assert_eq!(
            module_name_to_relative_path("../calculator weird/ops"),
            PathBuf::from("calculator_weird/ops.jsonld")
        );
    }

    #[test]
    fn graph_path_to_module_name_preserves_nested_modules() {
        let graph_dir = PathBuf::from("/tmp/workspace/.duumbi/graph");

        assert_eq!(
            graph_path_to_module_name(&graph_dir, &graph_dir.join("string/utils.jsonld")),
            Some("string/utils".to_string())
        );
        assert_eq!(
            graph_path_to_module_name(&graph_dir, &graph_dir.join("main.jsonld")),
            Some("main".to_string())
        );
        assert_eq!(
            graph_path_to_module_name(&graph_dir, &graph_dir.join("string/utils.json")),
            None
        );
    }

    #[test]
    fn empty_module_template_preserves_full_module_identity() {
        let template = empty_module_template("calculator/ops");
        assert_eq!(template["@id"], "duumbi:calculator/ops");
        assert_eq!(template["duumbi:name"], "calculator/ops");
    }

    #[test]
    fn cleanup_create_module_output_removes_main_from_library_module() {
        let mut module = json!({
            "duumbi:functions": [
                { "duumbi:name": "reverse" },
                { "duumbi:name": "main" },
                { "duumbi:name": "count_vowels" }
            ],
            "duumbi:exports": ["main", "reverse"]
        });

        cleanup_create_module_output(&mut module, "string/utils", true);

        let function_names: Vec<&str> = module["duumbi:functions"]
            .as_array()
            .expect("functions")
            .iter()
            .filter_map(|function| function["duumbi:name"].as_str())
            .collect();
        let exports: Vec<&str> = module["duumbi:exports"]
            .as_array()
            .expect("exports")
            .iter()
            .filter_map(|export| export.as_str())
            .collect();

        assert_eq!(function_names, vec!["reverse", "count_vowels"]);
        assert_eq!(exports, vec!["reverse", "count_vowels"]);
    }

    #[test]
    fn cleanup_repaired_module_output_removes_main_from_nested_library_module() {
        let graph_dir = PathBuf::from("/tmp/workspace/.duumbi/graph");
        let path = graph_dir.join("string/utils.jsonld");
        let spec = string_utils_spec();
        let mut module = json!({
            "duumbi:functions": [
                { "duumbi:name": "reverse" },
                { "duumbi:name": "main" },
                { "duumbi:name": "is_palindrome" }
            ],
            "duumbi:exports": ["main", "reverse"]
        });

        cleanup_repaired_module_output(&mut module, &graph_dir, &path, &spec);

        let function_names: Vec<&str> = module["duumbi:functions"]
            .as_array()
            .expect("functions")
            .iter()
            .filter_map(|function| function["duumbi:name"].as_str())
            .collect();
        let exports: Vec<&str> = module["duumbi:exports"]
            .as_array()
            .expect("exports")
            .iter()
            .filter_map(|export| export.as_str())
            .collect();

        assert_eq!(function_names, vec!["reverse", "is_palindrome"]);
        assert_eq!(exports, vec!["reverse", "is_palindrome"]);
    }

    #[test]
    fn cleanup_repaired_module_output_preserves_entry_module_main() {
        let graph_dir = PathBuf::from("/tmp/workspace/.duumbi/graph");
        let path = graph_dir.join("main.jsonld");
        let spec = string_utils_spec();
        let mut module = json!({
            "duumbi:functions": [
                { "duumbi:name": "main" },
                { "duumbi:name": "helper" }
            ]
        });

        cleanup_repaired_module_output(&mut module, &graph_dir, &path, &spec);

        let function_names: Vec<&str> = module["duumbi:functions"]
            .as_array()
            .expect("functions")
            .iter()
            .filter_map(|function| function["duumbi:name"].as_str())
            .collect();

        assert_eq!(function_names, vec!["main", "helper"]);
        assert!(module.get("duumbi:exports").is_none());
    }

    #[test]
    fn cleanup_create_module_output_preserves_main_for_entry_modules() {
        for module_name in ["main", "app/main"] {
            let mut module = json!({
                "duumbi:functions": [
                    { "duumbi:name": "main" },
                    { "duumbi:name": "helper" }
                ],
                "duumbi:exports": []
            });

            cleanup_create_module_output(&mut module, module_name, true);

            let function_names: Vec<&str> = module["duumbi:functions"]
                .as_array()
                .expect("functions")
                .iter()
                .filter_map(|function| function["duumbi:name"].as_str())
                .collect();

            assert_eq!(function_names, vec!["main", "helper"]);
        }
    }

    #[test]
    fn cleanup_create_module_output_preserves_main_for_generic_library_module() {
        let mut module = json!({
            "duumbi:functions": [
                { "duumbi:name": "main" },
                { "duumbi:name": "helper" }
            ],
            "duumbi:exports": []
        });

        cleanup_create_module_output(&mut module, "foo/utils", false);

        let function_names: Vec<&str> = module["duumbi:functions"]
            .as_array()
            .expect("functions")
            .iter()
            .filter_map(|function| function["duumbi:name"].as_str())
            .collect();
        let exports: Vec<&str> = module["duumbi:exports"]
            .as_array()
            .expect("exports")
            .iter()
            .filter_map(|export| export.as_str())
            .collect();

        assert_eq!(function_names, vec!["main", "helper"]);
        assert_eq!(exports, vec!["main", "helper"]);
    }

    fn string_utils_spec() -> IntentSpec {
        IntentSpec {
            intent: "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: Vec::new(),
            modules: IntentModules {
                create: vec!["string/utils".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: Vec::new(),
            dependencies: Vec::new(),
            context: None,
            created_at: None,
            execution: None,
        }
    }

    #[test]
    fn string_utils_benchmark_expected_exports_include_canonical_functions() {
        let spec = IntentSpec {
            intent: "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: Vec::new(),
            modules: IntentModules {
                create: vec!["string/utils".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: vec![TestCase {
                name: "main_returns_zero".to_string(),
                function: "main".to_string(),
                args: Vec::new(),
                expected_return: 0,
            }],
            dependencies: Vec::new(),
            context: None,
            created_at: None,
            execution: None,
        };
        let task_kind = TaskKind::CreateModule {
            module_name: "string/utils".to_string(),
        };

        let mut exports = expected_exports_for_module(&spec, &task_kind);
        exports.sort();

        assert_eq!(exports, vec!["count_vowels", "is_palindrome", "reverse"]);
    }

    #[test]
    fn main_only_non_benchmark_expected_exports_are_empty() {
        let spec = IntentSpec {
            intent: "Create a demo module".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: Vec::new(),
            modules: IntentModules {
                create: vec!["demo/ops".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: vec![TestCase {
                name: "main_returns_zero".to_string(),
                function: "main".to_string(),
                args: Vec::new(),
                expected_return: 0,
            }],
            dependencies: Vec::new(),
            context: None,
            created_at: None,
            execution: None,
        };
        let task_kind = TaskKind::CreateModule {
            module_name: "demo/ops".to_string(),
        };

        let exports = expected_exports_for_module(&spec, &task_kind);

        assert!(exports.is_empty());
    }
}
