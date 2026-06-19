//! Provider-backed determinism replay runner.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::agents::LlmProvider;
use crate::agents::factory;
use crate::bench::report::{
    BenchmarkEvidence, ErrorCategory, ProviderUsageSummary, categorize_error,
};
use crate::bench::runner::{
    extract_error_codes, filter_providers, load_archived_intent, provider_name,
};
use crate::bench::showcases::{self, Showcase, ShowcaseSuite, ShowcaseVerification};
use crate::config::ProviderConfig;
use crate::hash;
use crate::intent;
use crate::intent::bdd::{DEFAULT_BDD_CONTEXT_LIMIT, load_bdd_report, render_bdd_prompt_context};
use crate::intent::spec::{IntentSpec, IntentStatus};

use super::digest::{
    exact_graph_digest, safe_artifact_key, sha256_hex_bytes, workspace_state_hashes,
};
use super::evidence::{
    LedgerEvent, LedgerEventKind, ModelIdentity, PromptHashes, ReplayAttempt, ReplayEnvironment,
    ReplayInputs, ReplayMetrics, ReplayReport, ReplayTask,
};
use super::ledger::LedgerWriter;

/// Configuration for one determinism replay run.
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Stable run identifier.
    pub run_id: String,
    /// Number of attempts per selected task/provider pair.
    pub attempts: u32,
    /// Provider configs to replay.
    pub providers: Vec<ProviderConfig>,
    /// Optional showcase name filter.
    pub showcase_filter: Option<Vec<String>>,
    /// Optional provider route filter.
    pub provider_filter: Option<Vec<String>>,
    /// Optional benchmark suite filter.
    pub suite_filter: Option<ShowcaseSuite>,
    /// Whether to select only the low-budget smoke subset.
    pub smoke: bool,
    /// Replay artifact bundle root.
    pub artifact_dir: PathBuf,
    /// UTC start timestamp as RFC3339 text.
    pub started_at: String,
    /// Source commit used for report metadata.
    pub source_commit: String,
    /// Provider configuration source label.
    pub provider_source: String,
    /// Retain isolated attempt workspaces under the replay bundle.
    pub keep_workspaces: bool,
}

/// Runs determinism replay for selected benchmark showcases.
///
/// # Errors
///
/// Returns an error string when selection, provider creation, artifact writing,
/// or workspace initialization fails before a report can be produced.
#[must_use = "determinism replay report should be inspected or written"]
pub async fn run_replay<F>(config: &ReplayConfig, init_workspace: F) -> Result<ReplayReport, String>
where
    F: Fn(&Path) -> Result<(), anyhow::Error> + Send + Sync,
{
    run_replay_with_provider_factory(config, init_workspace, |provider_config| {
        factory::create_provider(provider_config).map_err(|error| {
            format!(
                "failed to create provider '{}': {error}",
                provider_name(provider_config)
            )
        })
    })
    .await
}

async fn run_replay_with_provider_factory<F, P>(
    config: &ReplayConfig,
    init_workspace: F,
    create_provider: P,
) -> Result<ReplayReport, String>
where
    F: Fn(&Path) -> Result<(), anyhow::Error> + Send + Sync,
    P: Fn(&ProviderConfig) -> Result<Box<dyn LlmProvider>, String>,
{
    let showcase_refs = showcases::filter_showcases_with_options(
        config.showcase_filter.as_deref(),
        config.suite_filter,
        config.smoke,
    );
    if showcase_refs.is_empty() {
        return Err("no showcases match the given filter".to_string());
    }

    let provider_configs = filter_providers(&config.providers, config.provider_filter.as_deref());
    if provider_configs.is_empty() {
        return Err("no providers match the given filter".to_string());
    }

    let run_dir = config.artifact_dir.join(&config.run_id);
    std::fs::create_dir_all(&run_dir).map_err(|source| {
        format!(
            "failed to create replay run dir '{}': {source}",
            run_dir.display()
        )
    })?;
    let mut ledger =
        LedgerWriter::open(&run_dir.join("ledger.jsonl")).map_err(|error| format!("{error}"))?;
    append_ledger(
        &mut ledger,
        LedgerEvent::new(
            &config.run_id,
            LedgerEventKind::RunStarted,
            1,
            &config.started_at,
            serde_json::json!({"artifact_dir": run_dir.display().to_string()}),
        ),
    )?;

    let workspace_state =
        workspace_state_hashes(Path::new(".")).map_err(|error| format!("{error}"))?;
    let inputs = ReplayInputs {
        suite: config
            .suite_filter
            .map_or_else(|| "core".to_string(), |suite| suite.as_str().to_string()),
        smoke: config.smoke,
        showcases: showcase_refs
            .iter()
            .map(|showcase| showcase.name.to_string())
            .collect(),
        providers: provider_configs
            .iter()
            .map(|provider| provider_name(provider))
            .collect(),
        attempts: config.attempts,
    };
    let environment = ReplayEnvironment {
        provider_source: config.provider_source.clone(),
        registry_state_hash: workspace_state.registry_state_hash,
        lockfile_hash: workspace_state.lockfile_hash,
        workspace_dependency_config_hash: workspace_state.workspace_dependency_config_hash,
    };

    let mut report = ReplayReport::new(
        &config.run_id,
        &config.started_at,
        &config.started_at,
        env!("CARGO_PKG_VERSION"),
        &config.source_commit,
        inputs,
        environment,
    );

    let mut sequence = 2u64;
    for showcase in showcase_refs {
        let spec = showcases::parse_showcase(showcase)?;
        report.tasks.push(ReplayTask {
            task_id: showcase.name.to_string(),
            suite: showcase.suite.as_str().to_string(),
            tags: showcase.tags.iter().map(|tag| (*tag).to_string()).collect(),
        });
        append_ledger(
            &mut ledger,
            event_with_task(
                &config.run_id,
                LedgerEventKind::TaskSelected,
                sequence,
                &config.started_at,
                showcase.name,
                serde_json::json!({"suite": showcase.suite.as_str(), "tags": showcase.tags}),
            ),
        )?;
        sequence += 1;

        for provider_config in &provider_configs {
            let provider = create_provider(provider_config)?;
            let provider_route = provider_name(provider_config);
            let provider_key = safe_artifact_key(&provider_route, "provider");
            let model_identity = ModelIdentity::Available {
                label: provider.model_label(),
            };
            for attempt in 1..=config.attempts {
                let attempt_dir = run_dir
                    .join("attempts")
                    .join(safe_artifact_key(showcase.name, "task"))
                    .join(&provider_key)
                    .join(attempt.to_string());
                std::fs::create_dir_all(&attempt_dir).map_err(|source| {
                    format!(
                        "failed to create replay attempt dir '{}': {source}",
                        attempt_dir.display()
                    )
                })?;
                append_ledger(
                    &mut ledger,
                    event_with_attempt(AttemptEvent {
                        run_id: &config.run_id,
                        event: LedgerEventKind::AttemptStarted,
                        sequence,
                        timestamp: &config.started_at,
                        task_id: showcase.name,
                        provider: &provider_route,
                        attempt,
                        payload: serde_json::json!({"artifact_dir": attempt_dir.display().to_string()}),
                    }),
                )?;
                sequence += 1;

                let replay_attempt = run_single_replay(SingleReplayRequest {
                    showcase,
                    provider: provider.as_ref(),
                    model_identity: model_identity.clone(),
                    provider_route: &provider_route,
                    spec: &spec,
                    attempt,
                    attempt_dir: &attempt_dir,
                    keep_workspace: config.keep_workspaces,
                    init_workspace: &init_workspace,
                })
                .await;

                append_ledger(
                    &mut ledger,
                    event_with_attempt(AttemptEvent {
                        run_id: &config.run_id,
                        event: if replay_attempt.success {
                            LedgerEventKind::AttemptCompleted
                        } else {
                            LedgerEventKind::AttemptFailed
                        },
                        sequence,
                        timestamp: &utc_now(),
                        task_id: showcase.name,
                        provider: &provider_route,
                        attempt,
                        payload: serde_json::json!({
                            "success": replay_attempt.success,
                            "tests_passed": replay_attempt.tests_passed,
                            "tests_total": replay_attempt.tests_total,
                            "error": replay_attempt.dominant_error_code,
                        }),
                    }),
                )?;
                sequence += 1;
                report.attempts.push(replay_attempt);
            }
        }
    }

    report.metrics = ReplayMetrics::from_attempts(&report.attempts);
    report.finished_at = utc_now();
    append_ledger(
        &mut ledger,
        LedgerEvent::new(
            &config.run_id,
            LedgerEventKind::RunCompleted,
            sequence,
            &report.finished_at,
            serde_json::json!({
                "attempts_total": report.metrics.attempts_total,
                "attempts_completed": report.metrics.attempts_completed,
            }),
        ),
    )?;

    Ok(report)
}

struct SingleReplayRequest<'a, F>
where
    F: Fn(&Path) -> Result<(), anyhow::Error>,
{
    showcase: &'a Showcase,
    provider: &'a dyn LlmProvider,
    model_identity: ModelIdentity,
    provider_route: &'a str,
    spec: &'a IntentSpec,
    attempt: u32,
    attempt_dir: &'a Path,
    keep_workspace: bool,
    init_workspace: &'a F,
}

async fn run_single_replay<F>(request: SingleReplayRequest<'_, F>) -> ReplayAttempt
where
    F: Fn(&Path) -> Result<(), anyhow::Error>,
{
    let SingleReplayRequest {
        showcase,
        provider,
        model_identity,
        provider_route,
        spec,
        attempt,
        attempt_dir,
        keep_workspace,
        init_workspace,
    } = request;

    let start = Instant::now();
    if let ShowcaseVerification::ProcessEvidence {
        evidence_kind,
        expected_route,
        expected_json_fields,
        verification_gap,
    } = showcase.verification
    {
        return replay_attempt_from_parts(ReplayAttemptParts {
            success: false,
            tests_passed: 0,
            tests_total: spec.test_cases.len(),
            error_category: Some(ErrorCategory::EvidenceRequired),
            dominant_error_code: Some("broader_evidence_required".to_string()),
            benchmark_evidence: Some(BenchmarkEvidence {
                kind: evidence_kind.to_string(),
                status: "broader_evidence_required".to_string(),
                detail: verification_gap.to_string(),
                command: None,
                expected_route: Some(expected_route.to_string()),
                expected_json_fields: expected_json_fields
                    .iter()
                    .map(|field| (*field).to_string())
                    .collect(),
                verification_gap: Some(verification_gap.to_string()),
                artifact_path: None,
            }),
            duration_secs: start.elapsed().as_secs_f64(),
            ..ReplayAttemptParts::new(showcase, provider_route, model_identity, attempt)
        });
    }

    let tmp = match tempfile::TempDir::new() {
        Ok(tmp) => tmp,
        Err(error) => {
            return replay_attempt_from_error(
                showcase,
                provider_route,
                model_identity,
                attempt,
                spec.test_cases.len(),
                format!("tempdir creation failed: {error}"),
                start.elapsed().as_secs_f64(),
            );
        }
    };
    let workspace = tmp.path();
    if let Err(error) = init_workspace(workspace) {
        return replay_attempt_from_error(
            showcase,
            provider_route,
            model_identity,
            attempt,
            spec.test_cases.len(),
            format!("init failed: {error}"),
            start.elapsed().as_secs_f64(),
        );
    }

    let graph_dir = workspace.join(".duumbi/graph");
    let initial_graph_exact_hash = exact_graph_digest(&graph_dir).ok();
    let initial_graph_semantic_hash = hash::semantic_hash(&graph_dir).ok();

    let slug = "determinism-replay";
    let mut run_spec = spec.clone();
    run_spec.status = IntentStatus::Pending;
    if let Err(error) = intent::save_intent(workspace, slug, &run_spec) {
        return replay_attempt_from_error(
            showcase,
            provider_route,
            model_identity,
            attempt,
            spec.test_cases.len(),
            format!("failed to save intent: {error}"),
            start.elapsed().as_secs_f64(),
        );
    }
    let context_hashes =
        replay_context_hashes(showcase, provider_route, &run_spec, workspace, slug);

    let mut log = Vec::new();
    let execute_result = intent::execute::run_execute(provider, workspace, slug, &mut log).await;
    let final_spec = intent::load_intent(workspace, slug)
        .or_else(|_| load_archived_intent(workspace, slug))
        .ok();
    let tests_total = spec.test_cases.len();
    let tests_passed = final_spec
        .as_ref()
        .and_then(|loaded| loaded.execution.as_ref())
        .map_or(0, |execution| execution.tests_passed);
    let final_graph_exact_hash = exact_graph_digest(&graph_dir).ok();
    let final_graph_semantic_hash = hash::semantic_hash(&graph_dir).ok();
    let error_text = execute_result.as_ref().err().map(ToString::to_string);
    let dominant_error_code = extract_error_codes(&log.join("\n"))
        .into_iter()
        .next()
        .or_else(|| {
            error_text
                .as_ref()
                .and_then(|text| extract_error_codes(text).into_iter().next())
        });
    let success = execute_result.unwrap_or(false) && tests_passed == tests_total;
    let error_category = if success {
        None
    } else {
        Some(
            error_text
                .as_deref()
                .map_or(ErrorCategory::LogicError, categorize_error),
        )
    };
    let mut artifact_paths = retain_attempt_log(attempt_dir, &log);
    if keep_workspace {
        artifact_paths.extend(retain_workspace_snapshot(workspace, attempt_dir));
    }
    let behavior_signature = Some(format!(
        "success={success};tests={tests_passed}/{tests_total};error={}",
        error_category
            .map(|category| category.to_string())
            .unwrap_or_else(|| "none".to_string())
    ));

    ReplayAttempt {
        task_id: showcase.name.to_string(),
        suite: showcase.suite.as_str().to_string(),
        tags: showcase.tags.iter().map(|tag| (*tag).to_string()).collect(),
        provider: provider_route.to_string(),
        model_identity,
        attempt,
        workspace_strategy: "isolated_tempdir".to_string(),
        initial_graph_exact_hash,
        initial_graph_semantic_hash,
        final_graph_exact_hash,
        final_graph_semantic_hash,
        intent_spec_hash: context_hashes.intent_spec_hash,
        bdd_context_hash: context_hashes.bdd_context_hash,
        context_pack_hash: context_hashes.context_pack_hash,
        prompt_hashes: context_hashes.prompt_hashes,
        success,
        tests_passed,
        tests_total,
        bdd_readiness: context_hashes.bdd_readiness,
        bdd_coverage: context_hashes.bdd_coverage,
        behavior_signature,
        error_category,
        dominant_error_code,
        provider_usage: ProviderUsageSummary::unavailable("provider_response_did_not_expose_usage"),
        benchmark_evidence: None,
        artifact_paths,
        duration_secs: start.elapsed().as_secs_f64(),
    }
}

struct ReplayContextHashes {
    intent_spec_hash: Option<String>,
    bdd_context_hash: Option<String>,
    context_pack_hash: Option<String>,
    prompt_hashes: PromptHashes,
    bdd_readiness: Option<String>,
    bdd_coverage: Vec<String>,
}

fn replay_context_hashes(
    showcase: &Showcase,
    provider_route: &str,
    spec: &IntentSpec,
    workspace: &Path,
    slug: &str,
) -> ReplayContextHashes {
    let intent_spec_hash = stable_yaml_hash(spec);
    let bdd_report = load_bdd_report(spec, workspace, slug);
    let bdd_prompt_context = render_bdd_prompt_context(&bdd_report, DEFAULT_BDD_CONTEXT_LIMIT);
    let bdd_context_hash = Some(hash_lines(
        "duumbi-determinism-bdd-context-v1",
        &bdd_prompt_context,
    ));
    let context_pack_hash = stable_json_hash(&serde_json::json!({
        "schema": "duumbi-determinism-context-pack-v1",
        "task_id": showcase.name,
        "suite": showcase.suite.as_str(),
        "tags": showcase.tags,
        "provider": provider_route,
        "intent_spec_hash": intent_spec_hash,
        "bdd_context_hash": bdd_context_hash,
        "acceptance_criteria_count": spec.acceptance_criteria.len(),
        "test_cases_count": spec.test_cases.len(),
    }));

    let mut hashes = BTreeMap::new();
    if let Some(hash) = &intent_spec_hash {
        hashes.insert("intent_spec".to_string(), hash.clone());
    }
    if let Some(hash) = &bdd_context_hash {
        hashes.insert("bdd_context".to_string(), hash.clone());
    }
    if let Some(hash) = &context_pack_hash {
        hashes.insert("context_pack".to_string(), hash.clone());
    }

    let prompt_hashes = if hashes.is_empty() {
        PromptHashes::Unavailable {
            reason: "context_hash_serialization_failed".to_string(),
        }
    } else {
        PromptHashes::Partial {
            reason: "final provider prompt capture is not exposed by intent execute".to_string(),
            hashes,
        }
    };

    ReplayContextHashes {
        intent_spec_hash,
        bdd_context_hash,
        context_pack_hash,
        prompt_hashes,
        bdd_readiness: Some(bdd_report.readiness.label().to_ascii_lowercase()),
        bdd_coverage: bdd_report
            .coverage
            .iter()
            .map(|coverage| format!("{}:{}", coverage.scenario, coverage.classification.label()))
            .collect(),
    }
}

fn stable_yaml_hash<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_yaml::to_string(value)
        .ok()
        .map(|text| hash_text("duumbi-determinism-yaml-v1", &text))
}

fn stable_json_hash<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_json::to_vec(value)
        .ok()
        .map(|bytes| hash_bytes("duumbi-determinism-json-v1", &bytes))
}

fn hash_lines(domain: &str, lines: &[String]) -> String {
    hash_text(domain, &lines.join("\n"))
}

fn hash_text(domain: &str, text: &str) -> String {
    hash_bytes(domain, text.as_bytes())
}

fn hash_bytes(domain: &str, bytes: &[u8]) -> String {
    let mut input = Vec::with_capacity(domain.len() + bytes.len() + 2);
    input.extend_from_slice(domain.as_bytes());
    input.push(0);
    input.extend_from_slice(bytes.len().to_string().as_bytes());
    input.push(0);
    input.extend_from_slice(bytes);
    sha256_hex_bytes(&input)
}

struct ReplayAttemptParts<'a> {
    showcase: &'a Showcase,
    provider_route: &'a str,
    model_identity: ModelIdentity,
    attempt: u32,
    success: bool,
    tests_passed: usize,
    tests_total: usize,
    error_category: Option<ErrorCategory>,
    dominant_error_code: Option<String>,
    benchmark_evidence: Option<BenchmarkEvidence>,
    duration_secs: f64,
}

impl<'a> ReplayAttemptParts<'a> {
    fn new(
        showcase: &'a Showcase,
        provider_route: &'a str,
        model_identity: ModelIdentity,
        attempt: u32,
    ) -> Self {
        Self {
            showcase,
            provider_route,
            model_identity,
            attempt,
            success: false,
            tests_passed: 0,
            tests_total: 0,
            error_category: None,
            dominant_error_code: None,
            benchmark_evidence: None,
            duration_secs: 0.0,
        }
    }
}

fn replay_attempt_from_parts(parts: ReplayAttemptParts<'_>) -> ReplayAttempt {
    ReplayAttempt {
        task_id: parts.showcase.name.to_string(),
        suite: parts.showcase.suite.as_str().to_string(),
        tags: parts
            .showcase
            .tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        provider: parts.provider_route.to_string(),
        model_identity: parts.model_identity,
        attempt: parts.attempt,
        workspace_strategy: "not_applicable".to_string(),
        initial_graph_exact_hash: None,
        initial_graph_semantic_hash: None,
        final_graph_exact_hash: None,
        final_graph_semantic_hash: None,
        intent_spec_hash: None,
        bdd_context_hash: None,
        context_pack_hash: None,
        prompt_hashes: PromptHashes::Unavailable {
            reason: "no provider mutation prompt for broader-evidence placeholder".to_string(),
        },
        success: parts.success,
        tests_passed: parts.tests_passed,
        tests_total: parts.tests_total,
        bdd_readiness: None,
        bdd_coverage: Vec::new(),
        behavior_signature: Some(format!(
            "success={};tests={}/{};error={}",
            parts.success,
            parts.tests_passed,
            parts.tests_total,
            parts
                .error_category
                .map(|category| category.to_string())
                .unwrap_or_else(|| "none".to_string())
        )),
        error_category: parts.error_category,
        dominant_error_code: parts.dominant_error_code,
        provider_usage: ProviderUsageSummary::unavailable("process_evidence_not_executed"),
        benchmark_evidence: parts.benchmark_evidence,
        artifact_paths: Vec::new(),
        duration_secs: parts.duration_secs,
    }
}

fn replay_attempt_from_error(
    showcase: &Showcase,
    provider_route: &str,
    model_identity: ModelIdentity,
    attempt: u32,
    tests_total: usize,
    message: String,
    duration_secs: f64,
) -> ReplayAttempt {
    let category = categorize_error(&message);
    replay_attempt_from_parts(ReplayAttemptParts {
        showcase,
        provider_route,
        model_identity,
        attempt,
        tests_total,
        error_category: Some(category),
        dominant_error_code: extract_error_codes(&message).into_iter().next(),
        duration_secs,
        ..ReplayAttemptParts::new(
            showcase,
            provider_route,
            ModelIdentity::unavailable("unused"),
            attempt,
        )
    })
}

fn retain_attempt_log(attempt_dir: &Path, log: &[String]) -> Vec<String> {
    if log.is_empty() {
        return Vec::new();
    }
    let path = attempt_dir.join("execute.log");
    if std::fs::write(&path, log.join("\n")).is_ok() {
        vec![path.display().to_string()]
    } else {
        Vec::new()
    }
}

fn retain_workspace_snapshot(workspace: &Path, attempt_dir: &Path) -> Vec<String> {
    let source = workspace.join(".duumbi");
    let destination = attempt_dir.join("workspace").join(".duumbi");
    if copy_dir_recursive(&source, &destination).is_ok() {
        vec![destination.display().to_string()]
    } else {
        Vec::new()
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn utc_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn append_ledger(writer: &mut LedgerWriter, event: LedgerEvent) -> Result<(), String> {
    writer.append(&event).map_err(|error| format!("{error}"))
}

fn event_with_task(
    run_id: &str,
    event: LedgerEventKind,
    sequence: u64,
    timestamp: &str,
    task_id: &str,
    payload: serde_json::Value,
) -> LedgerEvent {
    let mut event = LedgerEvent::new(run_id, event, sequence, timestamp, payload);
    event.task_id = Some(task_id.to_string());
    event
}

struct AttemptEvent<'a> {
    run_id: &'a str,
    event: LedgerEventKind,
    sequence: u64,
    timestamp: &'a str,
    task_id: &'a str,
    provider: &'a str,
    attempt: u32,
    payload: serde_json::Value,
}

fn event_with_attempt(params: AttemptEvent<'_>) -> LedgerEvent {
    let mut event = event_with_task(
        params.run_id,
        params.event,
        params.sequence,
        params.timestamp,
        params.task_id,
        params.payload,
    );
    event.provider = Some(params.provider.to_string());
    event.attempt = Some(params.attempt);
    event
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;

    use super::*;
    use crate::agents::{AgentError, LlmProvider};
    use crate::config::{ProviderKind, ProviderRole};
    use crate::determinism::digest::exact_graph_digest;
    use crate::intent::spec::{IntentModules, TestCase};
    use crate::patch::PatchOp;

    struct MockReplayProvider;

    impl LlmProvider for MockReplayProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn model_name(&self) -> Option<&str> {
            Some("determinism-fixture")
        }

        fn call_with_tools<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn call_with_tools_streaming<'a>(
            &'a self,
            system_prompt: &'a str,
            user_message: &'a str,
            _on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            self.call_with_tools(system_prompt, user_message)
        }
    }

    fn provider_config(model: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            provider: ProviderKind::OpenAI,
            role: ProviderRole::Primary,
            model: model.map(ToString::to_string),
            api_key_env: "OPENAI_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        }
    }

    #[test]
    fn replay_inputs_use_benchmark_provider_routes() {
        let providers = vec![provider_config(Some("gpt-test"))];
        let filtered = filter_providers(&providers, None);

        assert_eq!(provider_name(filtered[0]), "openai:gpt-test");
    }

    #[test]
    fn replay_context_hashes_record_partial_prompt_evidence() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let spec = IntentSpec {
            intent: "Build calculator".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["add(a, b) returns a + b".to_string()],
            modules: IntentModules {
                create: vec!["calculator/ops".to_string()],
                modify: Vec::new(),
            },
            test_cases: vec![TestCase {
                name: "addition".to_string(),
                function: "add".to_string(),
                args: vec![3, 5],
                expected_return: 8,
            }],
            dependencies: Vec::new(),
            bdd: Default::default(),
            context: None,
            created_at: None,
            execution: None,
        };
        let showcase = showcases::filter_showcases_with_options(
            Some(&["calculator".to_string()]),
            Some(ShowcaseSuite::Core),
            false,
        )
        .pop()
        .expect("calculator showcase exists");

        let hashes =
            replay_context_hashes(showcase, "mock:determinism", &spec, temp_dir.path(), "calc");

        assert!(hashes.intent_spec_hash.is_some());
        assert!(hashes.bdd_context_hash.is_some());
        assert!(hashes.context_pack_hash.is_some());
        assert_eq!(hashes.bdd_readiness.as_deref(), Some("warning"));
        assert!(hashes.bdd_coverage.is_empty());
        assert!(matches!(
            hashes.prompt_hashes,
            PromptHashes::Partial { ref hashes, .. }
                if hashes.contains_key("intent_spec")
                    && hashes.contains_key("bdd_context")
                    && hashes.contains_key("context_pack")
        ));
    }

    #[tokio::test]
    async fn replay_runner_records_attempts_without_mutating_active_graph() {
        let active_workspace = tempfile::TempDir::new().expect("active workspace");
        let active_graph_dir = active_workspace.path().join(".duumbi/graph");
        std::fs::create_dir_all(&active_graph_dir).expect("active graph dir");
        std::fs::write(
            active_graph_dir.join("main.jsonld"),
            r#"{"@context":{},"@graph":[]}"#,
        )
        .expect("active graph write");
        let before = exact_graph_digest(&active_graph_dir).expect("active graph digest");

        let artifact_dir = tempfile::TempDir::new().expect("artifact dir");
        let config = ReplayConfig {
            run_id: "run-fixture".to_string(),
            attempts: 2,
            providers: vec![provider_config(Some("determinism-fixture"))],
            showcase_filter: Some(vec!["calculator".to_string()]),
            provider_filter: None,
            suite_filter: Some(ShowcaseSuite::Core),
            smoke: false,
            artifact_dir: artifact_dir.path().join("replays"),
            started_at: "2026-06-19T00:00:00Z".to_string(),
            source_commit: "test-commit".to_string(),
            provider_source: "test".to_string(),
            keep_workspaces: false,
        };

        let report = run_replay_with_provider_factory(
            &config,
            |workspace| {
                let graph_dir = workspace.join(".duumbi/graph");
                std::fs::create_dir_all(&graph_dir)?;
                std::fs::write(
                    graph_dir.join("main.jsonld"),
                    r#"{"@context":{},"@graph":[]}"#,
                )?;
                Ok(())
            },
            |_| Ok(Box::new(MockReplayProvider)),
        )
        .await
        .expect("fixture replay should produce report");

        let after =
            exact_graph_digest(&active_graph_dir).expect("active graph digest after replay");
        assert_eq!(before, after);
        assert_eq!(report.tasks.len(), 1);
        assert_eq!(report.attempts.len(), 2);
        assert!(report.attempts.iter().all(|attempt| {
            attempt.workspace_strategy == "isolated_tempdir"
                && attempt.initial_graph_exact_hash.is_some()
                && attempt.final_graph_exact_hash.is_some()
                && matches!(attempt.prompt_hashes, PromptHashes::Partial { .. })
        }));

        let ledger_path = artifact_dir.path().join("replays/run-fixture/ledger.jsonl");
        let ledger = std::fs::read_to_string(ledger_path).expect("ledger should be written");
        assert!(ledger.contains(r#""event":"run_started""#));
        assert_eq!(ledger.matches(r#""event":"attempt_started""#).count(), 2);
        assert_eq!(ledger.matches(r#""event":"attempt_failed""#).count(), 2);
        assert!(ledger.contains(r#""event":"run_completed""#));
    }
}
