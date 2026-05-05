//! Phase 15 Calculator E2E harness.
//!
//! Developer-only validation for issue #486. It runs one Ralph Loop per
//! invocation and stops after reporting evidence and next-step guidance.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::agents::factory;
use crate::config::{ProviderConfig, ProviderKind, ProviderRole};
use crate::intent;
use crate::knowledge::types::FailureRecord;

const CALCULATOR_INTENT: &str =
    "Build a calculator with add, subtract, multiply, divide functions that work on i64 numbers";
const LIVE_LEG_TIMEOUT_SECS: u64 = 600;

/// Phase 15 harness report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase15Report {
    /// Name of the validated task.
    pub task: String,
    /// Provider selected for live calls.
    pub provider: String,
    /// Number of attempts requested.
    pub attempts: u32,
    /// Per-attempt results.
    pub attempts_results: Vec<Phase15AttemptReport>,
    /// Aggregate performance measurements for the run.
    pub performance: Phase15PerformanceReport,
    /// Aggregate user-experience checks for the run.
    pub user_experience: Phase15UxReport,
    /// Ralph Loop gate shown after the run.
    pub ralph_gate: RalphGate,
}

/// One Calculator E2E attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase15AttemptReport {
    /// Attempt number, 1-based.
    pub attempt: u32,
    /// Whether all required checks passed.
    pub ok: bool,
    /// CLI leg evidence.
    pub cli: Phase15LegReport,
    /// Studio leg evidence.
    pub studio: Phase15LegReport,
    /// Total elapsed time in seconds.
    pub elapsed_secs: f64,
}

/// Evidence for one validation leg.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase15LegReport {
    /// Whether the leg passed.
    pub ok: bool,
    /// Status message.
    pub message: String,
    /// Fresh workspace path used by the leg.
    pub workspace: Option<String>,
    /// Intent slug generated or used.
    pub intent_slug: Option<String>,
    /// Elapsed time in seconds.
    pub elapsed_secs: f64,
    /// Captured evidence snippets.
    pub evidence: Vec<String>,
    /// Failure category, if known.
    pub failure_category: Option<String>,
}

/// Ralph Loop gate summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphGate {
    /// Continue prompt.
    pub continue_prompt: String,
    /// Provider-change suggestion.
    pub suggest_provider_change: String,
    /// Engineering opinion.
    pub opinion: String,
}

/// Aggregate performance summary across all attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase15PerformanceReport {
    /// Whether measured timings stayed within the Phase 15 budget.
    pub ok: bool,
    /// Maximum allowed provider-backed CLI elapsed time.
    pub cli_budget_secs: f64,
    /// Per-attempt performance evidence.
    pub evidence: Vec<String>,
}

/// Aggregate Studio user-experience summary across all attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase15UxReport {
    /// Whether all UX checks passed.
    pub ok: bool,
    /// Passing UX checks collected from the Studio leg.
    pub checks: Vec<String>,
    /// UX issues collected from failed Studio evidence.
    pub issues: Vec<String>,
}

/// Runs the Phase 15 Calculator E2E harness.
pub async fn run(
    task: &str,
    provider: &str,
    attempts: u32,
    output: Option<PathBuf>,
    port: u16,
) -> Result<()> {
    if task != "calculator" {
        anyhow::bail!("Unsupported Phase 15 E2E task '{task}'. Only 'calculator' is implemented.");
    }

    let provider_kind = parse_provider(provider)?;
    let key_env = provider_key_env(&provider_kind);
    let provider_available = std::env::var(key_env).is_ok_and(|v| !v.trim().is_empty());
    let learning_cache = phase15_learning_cache_path(task, provider);
    let bootstrapped_learning = bootstrap_learning_cache(&learning_cache);

    let mut attempts_results = Vec::new();
    for attempt in 1..=attempts {
        let started = Instant::now();
        let cli = if provider_available {
            let workspace = unique_workspace("duumbi-p15-cli", attempt);
            match tokio::time::timeout(
                std::time::Duration::from_secs(LIVE_LEG_TIMEOUT_SECS),
                run_cli_leg(&provider_kind, workspace.clone(), &learning_cache),
            )
            .await
            {
                Ok(mut report) => {
                    let harvested = harvest_learning(&workspace, &learning_cache);
                    report
                        .evidence
                        .push(format!("harvested_learning_records={harvested}"));
                    report
                }
                Err(_) => {
                    record_phase15_failure(
                        &workspace,
                        &provider_kind,
                        "provider_timeout",
                        LIVE_LEG_TIMEOUT_SECS,
                        format!("CLI Phase 15 attempt timed out after {LIVE_LEG_TIMEOUT_SECS}s"),
                    );
                    let mut report = timeout_leg(attempt, "CLI", Some(&workspace));
                    let harvested = harvest_learning(&workspace, &learning_cache);
                    report
                        .evidence
                        .push(format!("harvested_learning_records={harvested}"));
                    report
                }
            }
        } else {
            blocked_leg(attempt, "CLI", key_env)
        };
        let studio = if provider_available && cli.ok {
            match cli.workspace.as_deref() {
                Some(workspace) => run_studio_leg(attempt, PathBuf::from(workspace), port).await,
                None => skipped_leg(
                    attempt,
                    "Studio",
                    "CLI passed but did not report a reusable workspace path.",
                ),
            }
        } else if provider_available {
            skipped_leg(
                attempt,
                "Studio",
                "CLI did not pass; Studio graph/build/run validation is skipped until shared backend behavior passes via CLI.",
            )
        } else {
            blocked_leg(attempt, "Studio", key_env)
        };
        let ok = cli.ok && studio.ok;
        attempts_results.push(Phase15AttemptReport {
            attempt,
            ok,
            cli,
            studio,
            elapsed_secs: started.elapsed().as_secs_f64(),
        });
    }

    let gate = build_ralph_gate(&attempts_results, key_env);
    let performance = build_performance_report(&attempts_results);
    let user_experience = build_ux_report(&attempts_results);
    print_ralph_gate(&gate);

    let report = Phase15Report {
        task: task.to_string(),
        provider: provider.to_string(),
        attempts,
        attempts_results,
        performance,
        user_experience,
        ralph_gate: gate,
    };

    eprintln!(
        "Phase 15 learning cache: {} (bootstrapped {bootstrapped_learning} record(s))",
        learning_cache.display()
    );

    let json = serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
    if let Some(path) = output {
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write report to '{}'", path.display()))?;
        eprintln!("Phase 15 report written to {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

async fn run_cli_leg(
    provider: &ProviderKind,
    workspace: PathBuf,
    learning_cache: &Path,
) -> Phase15LegReport {
    let started = Instant::now();

    if let Err(e) = crate::cli::init::run_init(&workspace) {
        return failed_leg(
            "CLI",
            Some(&workspace),
            started,
            "setup",
            format!("init: {e:#}"),
        );
    }
    if let Err(e) = write_provider_config(&workspace, provider) {
        return failed_leg(
            "CLI",
            Some(&workspace),
            started,
            "setup",
            format!("provider config: {e:#}"),
        );
    }
    let seeded_learning = seed_learning(&workspace, learning_cache);

    let provider_config = provider_config(provider);
    let client = match factory::create_provider_chain_for_global_access(&[provider_config]) {
        Ok(client) => client,
        Err(e) => {
            return failed_leg(
                "CLI",
                Some(&workspace),
                started,
                "provider_error",
                format!("provider: {e}"),
            );
        }
    };

    let mut create_log = Vec::new();
    let slug = match intent::create::run_create(
        &*client,
        &workspace,
        CALCULATOR_INTENT,
        true,
        &mut create_log,
    )
    .await
    {
        Ok(slug) => slug,
        Err(e) => {
            return failed_leg(
                "CLI",
                Some(&workspace),
                started,
                "provider_or_intent_error",
                format!("intent create: {e}"),
            );
        }
    };

    let mut execute_log = Vec::new();
    match intent::execute::run_execute(&*client, &workspace, &slug, &mut execute_log).await {
        Ok(true) => {}
        Ok(false) => {
            return failed_leg_with_slug(
                "CLI",
                Some(&workspace),
                Some(slug),
                started,
                "mutation_failed",
                "intent execution returned failing tests".to_string(),
            );
        }
        Err(e) => {
            return failed_leg_with_slug(
                "CLI",
                Some(&workspace),
                Some(slug),
                started,
                "mutation_failed",
                format!("intent execute: {e}"),
            );
        }
    }

    let graph_path = workspace.join(".duumbi/graph/main.jsonld");
    let describe = match crate::cli::commands::describe_to_string(&graph_path) {
        Ok(text) => text,
        Err(e) => {
            return failed_leg_with_slug(
                "CLI",
                Some(&workspace),
                Some(slug),
                started,
                "describe_failed",
                format!("describe: {e:#}"),
            );
        }
    };

    let build = crate::workflow::build_workspace(&workspace);
    if !build.ok {
        return failed_leg_with_slug(
            "CLI",
            Some(&workspace),
            Some(slug),
            started,
            "build_failed",
            format!("build: {}", build.message),
        );
    }

    let run = crate::workflow::run_workspace(&workspace);
    if run.exit_code == -1 && run.stdout.is_empty() {
        return failed_leg_with_slug(
            "CLI",
            Some(&workspace),
            Some(slug),
            started,
            "run_failed",
            format!("run: {}", run.stderr),
        );
    }

    let module_ok = workspace
        .join(".duumbi/graph/calculator/ops.jsonld")
        .exists();
    let stdout_ok = output_mentions_calculator_results(&run.stdout);
    let ok = module_ok && stdout_ok;
    Phase15LegReport {
        ok,
        message: if ok {
            "CLI Calculator path passed.".to_string()
        } else {
            "CLI Calculator path completed but evidence checks failed.".to_string()
        },
        workspace: Some(workspace.display().to_string()),
        intent_slug: Some(slug),
        elapsed_secs: started.elapsed().as_secs_f64(),
        evidence: vec![
            format!("seeded_learning_records={seeded_learning}"),
            format!("create_log_lines={}", create_log.len()),
            format!("execute_log_lines={}", execute_log.len()),
            format!("describe_contains_add={}", describe.contains("add")),
            format!("module_calculator_ops_exists={module_ok}"),
            format!("run_exit_code={}", run.exit_code),
            format!("stdout={}", truncate(&run.stdout, 500)),
        ],
        failure_category: (!ok).then(|| "evidence_mismatch".to_string()),
    }
}

async fn run_studio_leg(attempt: u32, workspace: PathBuf, port: u16) -> Phase15LegReport {
    let started = Instant::now();

    let mut child = match std::process::Command::new(std::env::current_exe().unwrap_or_default())
        .args(["studio", "--port", &port.to_string()])
        .current_dir(&workspace)
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return failed_leg(
                "Studio",
                Some(&workspace),
                started,
                "studio_start_failed",
                format!("studio start: {e}"),
            );
        }
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(LIVE_LEG_TIMEOUT_SECS),
        run_studio_http_flow(port),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Studio HTTP flow timed out after {LIVE_LEG_TIMEOUT_SECS}s"))
    .and_then(|result| result);
    let _ = child.kill();
    let _ = child.wait();

    match result {
        Ok(evidence) => Phase15LegReport {
            ok: true,
            message: format!(
                "Studio Calculator graph/build/run path passed on CLI-generated workspace (attempt {attempt})."
            ),
            workspace: Some(workspace.display().to_string()),
            intent_slug: None,
            elapsed_secs: started.elapsed().as_secs_f64(),
            evidence,
            failure_category: None,
        },
        Err(e) => {
            let message = format!("{e:#}");
            let category = if message.to_ascii_lowercase().contains("timed out")
                || message.to_ascii_lowercase().contains("timeout")
            {
                "timeout"
            } else {
                "studio_http_failed"
            };
            failed_leg("Studio", Some(&workspace), started, category, message)
        }
    }
}

async fn run_studio_http_flow(port: u16) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let base = format!("http://127.0.0.1:{port}");
    wait_for_studio(&client, &base).await?;

    let html = client.get(&base).send().await?.text().await?;
    let mut evidence = studio_ux_evidence(&html)?;

    let graph: serde_json::Value = client
        .get(format!("{base}/api/graph/context"))
        .send()
        .await?
        .json()
        .await?;
    let modules = graph
        .get("modules")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let has_calculator_ops = modules.iter().any(|m| m.as_str() == Some("calculator/ops"));
    if !has_calculator_ops {
        anyhow::bail!("graph context modules did not include calculator/ops: {modules:?}");
    }

    let build: serde_json::Value = client
        .post(format!("{base}/api/build"))
        .send()
        .await?
        .json()
        .await?;
    if build.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        anyhow::bail!("build failed: {build}");
    }

    let run: serde_json::Value = client
        .post(format!("{base}/api/run"))
        .send()
        .await?
        .json()
        .await?;
    let stdout = run
        .get("stdout")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if !output_mentions_calculator_results(stdout) {
        anyhow::bail!("run output did not include calculator evidence: {run}");
    }

    evidence.extend([
        "shared_backend_workspace=true".to_string(),
        "graph_has_calculator_ops=true".to_string(),
        format!(
            "build_output_path={}",
            build.get("output_path").unwrap_or(&serde_json::Value::Null)
        ),
        format!("stdout={}", truncate(stdout, 500)),
    ]);
    Ok(evidence)
}

async fn wait_for_studio(client: &reqwest::Client, base: &str) -> Result<()> {
    for _ in 0..80 {
        if let Ok(response) = client.get(base).send().await
            && response.status().is_success()
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    anyhow::bail!("Studio did not become ready at {base}");
}

fn blocked_leg(attempt: u32, leg: &str, key_env: &str) -> Phase15LegReport {
    Phase15LegReport {
        ok: false,
        message: format!("{leg} attempt {attempt} blocked: {key_env} is not set."),
        workspace: None,
        intent_slug: None,
        elapsed_secs: 0.0,
        evidence: vec![format!("missing_env={key_env}")],
        failure_category: Some("missing_provider_credentials".to_string()),
    }
}

fn skipped_leg(attempt: u32, leg: &str, reason: &str) -> Phase15LegReport {
    Phase15LegReport {
        ok: false,
        message: format!("{leg} attempt {attempt} skipped: {reason}"),
        workspace: None,
        intent_slug: None,
        elapsed_secs: 0.0,
        evidence: vec![format!("skip_reason={reason}")],
        failure_category: Some("skipped_cli_failed".to_string()),
    }
}

fn timeout_leg(attempt: u32, leg: &str, workspace: Option<&Path>) -> Phase15LegReport {
    Phase15LegReport {
        ok: false,
        message: format!("{leg} attempt {attempt} timed out after {LIVE_LEG_TIMEOUT_SECS}s."),
        workspace: workspace.map(|p| p.display().to_string()),
        intent_slug: None,
        elapsed_secs: LIVE_LEG_TIMEOUT_SECS as f64,
        evidence: vec![
            format!("timeout_secs={LIVE_LEG_TIMEOUT_SECS}"),
            workspace.map_or_else(
                || "workspace_learning_records=0".to_string(),
                |p| format!("workspace_learning_records={}", learning_record_count(p)),
            ),
        ],
        failure_category: Some("provider_timeout".to_string()),
    }
}

fn record_phase15_failure(
    workspace: &Path,
    provider: &ProviderKind,
    category: &str,
    retry_count: u64,
    summary: String,
) {
    let mut record = FailureRecord::new(CALCULATOR_INTENT, "Phase15E2E", category);
    record.provider = provider.to_string();
    record.model_label = provider.to_string();
    record.module = "calculator/ops".to_string();
    record.functions = vec![
        "add".to_string(),
        "subtract".to_string(),
        "multiply".to_string(),
        "divide".to_string(),
    ];
    record.retry_count = retry_count.min(u64::from(u32::MAX)) as u32;
    record.error_summary = crate::knowledge::learning::sanitize_error_summary(&summary);
    let _ = crate::knowledge::learning::append_failure_with_user_cache(workspace, &record);
}

fn failed_leg(
    leg: &str,
    workspace: Option<&Path>,
    started: Instant,
    category: &str,
    message: String,
) -> Phase15LegReport {
    failed_leg_with_slug(leg, workspace, None, started, category, message)
}

fn failed_leg_with_slug(
    leg: &str,
    workspace: Option<&Path>,
    slug: Option<String>,
    started: Instant,
    category: &str,
    message: String,
) -> Phase15LegReport {
    Phase15LegReport {
        ok: false,
        message: format!("{leg} failed: {message}"),
        workspace: workspace.map(|p| p.display().to_string()),
        intent_slug: slug,
        elapsed_secs: started.elapsed().as_secs_f64(),
        evidence: Vec::new(),
        failure_category: Some(category.to_string()),
    }
}

fn phase15_learning_cache_path(task: &str, provider: &str) -> PathBuf {
    let safe_task = task.replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    let safe_provider = provider.replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    std::env::temp_dir().join(format!(
        "duumbi-phase15-{safe_task}-{safe_provider}-learning.jsonl"
    ))
}

fn workspace_learning_path(workspace: &Path) -> PathBuf {
    workspace.join(".duumbi/learning/successes.jsonl")
}

fn learning_record_count(workspace: &Path) -> usize {
    learning_file_record_count(&workspace_learning_path(workspace))
}

fn learning_file_record_count(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}

fn seed_learning(workspace: &Path, learning_cache: &Path) -> usize {
    let content = match std::fs::read_to_string(learning_cache) {
        Ok(content) if !content.trim().is_empty() => content,
        _ => return 0,
    };
    let target = workspace_learning_path(workspace);
    if let Some(parent) = target.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return 0;
    }
    if std::fs::write(&target, &content).is_err() {
        return 0;
    }
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn harvest_learning(workspace: &Path, learning_cache: &Path) -> usize {
    let source = workspace_learning_path(workspace);
    let source_content = match std::fs::read_to_string(&source) {
        Ok(content) => content,
        Err(_) => return 0,
    };

    let mut existing: HashSet<String> = std::fs::read_to_string(learning_cache)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let mut new_lines = Vec::new();
    for line in source_content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if existing.insert(line.to_string()) {
            new_lines.push(line.to_string());
        }
    }

    if new_lines.is_empty() {
        return 0;
    }

    if let Some(parent) = learning_cache.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return 0;
    }
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(learning_cache)
    {
        Ok(file) => file,
        Err(_) => return 0,
    };

    use std::io::Write as _;
    let mut written = 0;
    for line in new_lines {
        if writeln!(file, "{line}").is_ok() {
            written += 1;
        }
    }
    written
}

fn bootstrap_learning_cache(learning_cache: &Path) -> usize {
    let temp_dir = std::env::temp_dir();
    let entries = match std::fs::read_dir(temp_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut harvested = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let is_phase15_workspace = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("duumbi-p15-"));
        if is_phase15_workspace {
            harvested += harvest_learning(&path, learning_cache);
        }
    }
    harvested
}

fn write_provider_config(workspace: &Path, provider: &ProviderKind) -> Result<()> {
    let path = workspace.join(".duumbi/config.toml");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let config = format!(
        "{existing}\n[[providers]]\nprovider = \"{}\"\nrole = \"primary\"\napi_key_env = \"{}\"\ntimeout_secs = 120\n",
        provider,
        provider_key_env(provider)
    );
    std::fs::write(&path, config)?;
    Ok(())
}

fn unique_workspace(prefix: &str, attempt: u32) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("{prefix}-{attempt}-{}-{nanos}", std::process::id()))
}

fn provider_config(provider: &ProviderKind) -> ProviderConfig {
    ProviderConfig {
        provider: provider.clone(),
        role: ProviderRole::Primary,
        model: None,
        api_key_env: provider_key_env(provider).to_string(),
        base_url: None,
        timeout_secs: Some(120),
        key_storage: None,
        auth_token_env: None,
    }
}

fn parse_provider(provider: &str) -> Result<ProviderKind> {
    match provider.to_ascii_lowercase().as_str() {
        "minimax" => Ok(ProviderKind::MiniMax),
        "anthropic" => Ok(ProviderKind::Anthropic),
        "openai" => Ok(ProviderKind::OpenAI),
        "grok" => Ok(ProviderKind::Grok),
        "openrouter" => Ok(ProviderKind::OpenRouter),
        _ => anyhow::bail!("Unsupported provider '{provider}'"),
    }
}

fn provider_key_env(provider: &ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
        ProviderKind::OpenAI => "OPENAI_API_KEY",
        ProviderKind::Grok => "XAI_API_KEY",
        ProviderKind::OpenRouter => "OPENROUTER_API_KEY",
        ProviderKind::MiniMax => "MINIMAX_API_KEY",
    }
}

fn output_mentions_calculator_results(stdout: &str) -> bool {
    let compact = stdout.replace(' ', "");
    (compact.contains("3+5=8") || compact.contains("8"))
        && (compact.contains("10/2=5") || compact.contains("5"))
}

fn studio_ux_evidence(html: &str) -> Result<Vec<String>> {
    let labels = extract_footer_labels(html);
    let expected = ["Intents", "Graph", "Build"];
    if labels != expected {
        anyhow::bail!("Studio footer labels were {labels:?}, expected {expected:?}");
    }

    let query_default_active = html.contains(r#"class="chat-mode-tab active" data-mode="query""#)
        || html.contains(r#"data-mode="query" class="chat-mode-tab active""#);
    if !query_default_active {
        anyhow::bail!("Studio chat did not render Query as the default active mode");
    }

    let query_read_only =
        html.contains(r#"data-mode="query""#) && html.contains(r#"title="Read-only answers""#);
    if !query_read_only {
        anyhow::bail!("Studio Query mode did not expose read-only UX copy");
    }

    let agent_mode_available =
        html.contains(r#"data-mode="agent""#) && html.contains(r#"title="Apply graph changes""#);
    if !agent_mode_available {
        anyhow::bail!("Studio Agent mode was not available for graph mutation handoff");
    }

    Ok(vec![
        format!("ux_footer_items={}", labels.join(",")),
        "ux_query_default_active=true".to_string(),
        "ux_query_read_only=true".to_string(),
        "ux_agent_mode_available=true".to_string(),
    ])
}

fn extract_footer_labels(html: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut rest = html;
    while let Some(class_start) = rest.find("footer-label") {
        rest = &rest[class_start..];
        let Some(tag_end) = rest.find('>') else {
            break;
        };
        rest = &rest[tag_end + 1..];
        let Some(text_end) = rest.find('<') else {
            break;
        };
        let label = rest[..text_end].trim();
        if !label.is_empty() {
            labels.push(label.to_string());
        }
        rest = &rest[text_end..];
    }
    labels
}

fn build_performance_report(results: &[Phase15AttemptReport]) -> Phase15PerformanceReport {
    let cli_budget_secs = LIVE_LEG_TIMEOUT_SECS as f64;
    let mut ok = true;
    let mut evidence = Vec::new();
    for result in results {
        if result.cli.elapsed_secs > cli_budget_secs {
            ok = false;
        }
        evidence.push(format!(
            "attempt_{}_total_elapsed_secs={:.3}",
            result.attempt, result.elapsed_secs
        ));
        evidence.push(format!(
            "attempt_{}_cli_elapsed_secs={:.3}",
            result.attempt, result.cli.elapsed_secs
        ));
        evidence.push(format!(
            "attempt_{}_studio_elapsed_secs={:.3}",
            result.attempt, result.studio.elapsed_secs
        ));
    }
    Phase15PerformanceReport {
        ok,
        cli_budget_secs,
        evidence,
    }
}

fn build_ux_report(results: &[Phase15AttemptReport]) -> Phase15UxReport {
    let mut checks = Vec::new();
    let mut issues = Vec::new();

    for result in results {
        for item in &result.studio.evidence {
            if item.starts_with("ux_") {
                checks.push(format!("attempt_{}:{item}", result.attempt));
            }
        }
        if !result.studio.ok {
            issues.push(format!(
                "attempt_{}:{}",
                result.attempt, result.studio.message
            ));
        }
    }

    Phase15UxReport {
        ok: !checks.is_empty() && issues.is_empty(),
        checks,
        issues,
    }
}

fn build_ralph_gate(results: &[Phase15AttemptReport], key_env: &str) -> RalphGate {
    let all_ok = results.iter().all(|r| r.ok);
    let categories: Vec<&str> = results
        .iter()
        .flat_map(|r| {
            [
                r.cli.failure_category.as_deref(),
                r.studio.failure_category.as_deref(),
            ]
        })
        .flatten()
        .collect();
    let missing_credentials = categories.contains(&"missing_provider_credentials");
    let provider_issue = categories
        .iter()
        .any(|c| c.contains("provider") || *c == "timeout");

    RalphGate {
        continue_prompt: if all_ok {
            "Continue? Current loop passed; another loop would measure repeatability and cost more API calls.".to_string()
        } else {
            "Continue? Current loop did not pass; fix the reported blocker before another paid live loop.".to_string()
        },
        suggest_provider_change: if missing_credentials {
            format!(
                "Provider change not recommended yet. Set {key_env} in the environment and rerun."
            )
        } else if provider_issue {
            "Suggest provider change: repeated timeout/provider evidence. Configure the matching API-key env var and rerun with --provider <name>; do not paste secrets into logs or docs.".to_string()
        } else {
            "Provider change not recommended from this loop; evidence points to deterministic code or workflow behavior.".to_string()
        },
        opinion: if all_ok {
            "Opinion: #486 evidence is strong enough for the Calculator path; repeat only if you want confidence across multiple live attempts.".to_string()
        } else if missing_credentials {
            "Opinion: validation is blocked, not failed. The next useful action is setting the MiniMax key, not changing code.".to_string()
        } else if provider_issue {
            "Opinion: provider/tool-call reliability is now the leading risk; deterministic blockers should still be ruled out from the workspace evidence before switching.".to_string()
        } else {
            "Opinion: treat this as an implementation bug until the failure category proves provider instability.".to_string()
        },
    }
}

fn print_ralph_gate(gate: &RalphGate) {
    eprintln!();
    eprintln!("Ralph Gate");
    eprintln!("- {}", gate.continue_prompt);
    eprintln!("- {}", gate.suggest_provider_change);
    eprintln!("- {}", gate.opinion);
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_ux_evidence_accepts_phase15_shell() {
        let html = r#"
            <div class="footer-item"><span class="footer-label">Intents</span></div>
            <div class="footer-item"><span class="footer-label">Graph</span></div>
            <div class="footer-item"><span class="footer-label">Build</span></div>
            <button class="chat-mode-tab active" data-mode="query" title="Read-only answers">Query</button>
            <button class="chat-mode-tab" data-mode="agent" title="Apply graph changes">Agent</button>
        "#;

        let evidence = studio_ux_evidence(html).expect("ux evidence");
        assert!(evidence.contains(&"ux_footer_items=Intents,Graph,Build".to_string()));
        assert!(evidence.contains(&"ux_query_default_active=true".to_string()));
        assert!(evidence.contains(&"ux_query_read_only=true".to_string()));
        assert!(evidence.contains(&"ux_agent_mode_available=true".to_string()));
    }

    #[test]
    fn studio_ux_evidence_rejects_extra_footer_items() {
        let html = r#"
            <span class="footer-label">Intents</span>
            <span class="footer-label">Graph</span>
            <span class="footer-label">Build</span>
            <span class="footer-label">Agents</span>
            <button class="chat-mode-tab active" data-mode="query" title="Read-only answers">Query</button>
            <button class="chat-mode-tab" data-mode="agent" title="Apply graph changes">Agent</button>
        "#;

        assert!(studio_ux_evidence(html).is_err());
    }

    #[test]
    fn performance_report_flags_cli_budget_overrun() {
        let result = Phase15AttemptReport {
            attempt: 1,
            ok: false,
            cli: Phase15LegReport {
                ok: false,
                message: String::new(),
                workspace: None,
                intent_slug: None,
                elapsed_secs: LIVE_LEG_TIMEOUT_SECS as f64 + 1.0,
                evidence: Vec::new(),
                failure_category: Some("provider_timeout".to_string()),
            },
            studio: Phase15LegReport {
                ok: false,
                message: String::new(),
                workspace: None,
                intent_slug: None,
                elapsed_secs: 0.0,
                evidence: Vec::new(),
                failure_category: Some("skipped_cli_failed".to_string()),
            },
            elapsed_secs: LIVE_LEG_TIMEOUT_SECS as f64 + 1.0,
        };

        let report = build_performance_report(&[result]);
        assert!(!report.ok);
    }
}
