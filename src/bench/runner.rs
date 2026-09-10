//! Benchmark execution loop.
//!
//! Iterates over (showcase × provider × attempt), creating a temporary
//! workspace for each run, executing the intent pipeline, and collecting
//! [`BenchmarkResult`] entries.
//!
//! Providers are executed **concurrently** per showcase via `tokio::spawn`,
//! so two providers with 3 attempts each take roughly the same time as one
//! provider with 3 attempts (instead of 2×). Each provider gets its own
//! isolated [`tempfile::TempDir`] per attempt — no shared-state conflicts.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::agents::LlmProvider;
use crate::agents::factory;
use crate::bench::report::{BenchmarkEvidence, BenchmarkResult, ProviderUsageSummary};
use crate::bench::showcases::{self, Showcase, ShowcaseSuite, ShowcaseVerification};
use crate::config::ProviderConfig;
use crate::intent::attempt::{
    AttemptRequest, DEFAULT_BENCHMARK_ARTIFACT_DIR, RunRetentionState, generate_run_id,
    run_isolated_attempt,
};

/// Configuration for a benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of attempts per (showcase, provider) pair.
    pub attempts: u32,
    /// Provider configs to test.
    pub providers: Vec<ProviderConfig>,
    /// Optional showcase name filter.
    pub showcase_filter: Option<Vec<String>>,
    /// Optional provider name filter.
    pub provider_filter: Option<Vec<String>>,
    /// Optional benchmark suite filter.
    pub suite_filter: Option<ShowcaseSuite>,
    /// Whether to run only the selected suite's low-budget smoke subset.
    pub smoke: bool,
    /// Artifact root for retained attempt evidence.
    pub artifact_dir: PathBuf,
    /// Copy graph/intent snapshots.
    pub keep_workspaces: bool,
    /// Capture current-attempt model I/O.
    pub capture_model_io: bool,
}

/// Runs the full benchmark suite.
///
/// Providers are executed concurrently per showcase. Each attempt within a
/// single provider runs sequentially to avoid rate-limit issues.
///
/// `init_workspace` is called to set up each temporary workspace (typically
/// `cli::init::run_init`). It is passed as a callback because the `cli`
/// module is binary-only and not available from `lib.rs`.
///
/// # Errors
///
/// Returns an error if no providers or showcases are available.
pub async fn run_benchmark<F>(
    config: &BenchmarkConfig,
    init_workspace: F,
) -> Result<Vec<BenchmarkResult>, String>
where
    F: Fn(&Path) -> Result<(), anyhow::Error> + Send + Sync + 'static,
{
    run_benchmark_with_provider_factory(config, init_workspace, |prov_config| {
        factory::create_provider(prov_config)
            .map(Arc::from)
            .map_err(|e| {
                format!(
                    "failed to create provider '{}': {e}",
                    provider_name(prov_config)
                )
            })
    })
    .await
}

/// Runs the benchmark suite with an injected provider factory.
///
/// # Errors
///
/// Returns an error if no providers or showcases are available.
pub async fn run_benchmark_with_provider_factory<F, P>(
    config: &BenchmarkConfig,
    init_workspace: F,
    create_provider: P,
) -> Result<Vec<BenchmarkResult>, String>
where
    F: Fn(&Path) -> Result<(), anyhow::Error> + Send + Sync + 'static,
    P: Fn(&ProviderConfig) -> Result<Arc<dyn LlmProvider>, String>,
{
    let showcase_refs: Vec<&Showcase> = if config.suite_filter.is_none() && !config.smoke {
        showcases::filter_showcases(config.showcase_filter.as_deref())
    } else {
        showcases::filter_showcases_with_options(
            config.showcase_filter.as_deref(),
            config.suite_filter,
            config.smoke,
        )
    };

    if showcase_refs.is_empty() {
        return Err("no showcases match the given filter".to_string());
    }

    let provider_configs = filter_providers(&config.providers, config.provider_filter.as_deref());

    if provider_configs.is_empty() {
        return Err("no providers match the given filter".to_string());
    }

    let total_runs = showcase_refs.len() * provider_configs.len() * config.attempts as usize;
    eprintln!(
        "Benchmark: {} showcase(s) × {} provider(s) × {} attempt(s) = {} total runs",
        showcase_refs.len(),
        provider_configs.len(),
        config.attempts,
        total_runs,
    );
    eprintln!();

    // Wrap init_workspace in Arc so it can be shared across spawned tasks.
    let init_workspace = Arc::new(init_workspace);
    let run_id = generate_run_id();
    let artifact_dir = if config.artifact_dir.as_os_str().is_empty() {
        PathBuf::from(DEFAULT_BENCHMARK_ARTIFACT_DIR)
    } else {
        config.artifact_dir.clone()
    };
    let retention = Arc::new(Mutex::new(RunRetentionState::default()));

    let mut all_results: Vec<BenchmarkResult> = Vec::with_capacity(total_runs);

    for showcase in showcase_refs {
        let spec =
            showcases::parse_showcase(showcase).map_err(|e| format!("invalid showcase: {e}"))?;

        let spec = Arc::new(spec);
        let showcase_name = showcase.name;

        // Spawn one task per provider; attempts within each task are sequential.
        let mut handles = Vec::with_capacity(provider_configs.len());
        for prov_config in &provider_configs {
            let provider: Arc<dyn LlmProvider> = create_provider(prov_config)?;

            let spec_clone = Arc::clone(&spec);
            let init_clone = Arc::clone(&init_workspace);
            let attempts = config.attempts;
            let prov_name = provider.name().to_string();
            let artifact_dir = artifact_dir.clone();
            let run_id = run_id.clone();
            let retention = Arc::clone(&retention);
            let keep_workspaces = config.keep_workspaces;
            let capture_model_io = config.capture_model_io;

            let handle = tokio::spawn(async move {
                let mut results = Vec::with_capacity(attempts as usize);
                for attempt in 1..=attempts {
                    eprintln!("  [{showcase_name} / {prov_name}] attempt {attempt}/{attempts}",);

                    let execute = {
                        let state = retention.lock().expect("invariant: retention mutex");
                        !state.stop_artifact_attempts
                    };
                    let result = run_single(
                        showcase,
                        provider.as_ref(),
                        &spec_clone,
                        attempt,
                        &*init_clone,
                        SingleAttemptOptions {
                            run_id: &run_id,
                            artifact_dir: &artifact_dir,
                            keep_workspaces,
                            capture_model_io,
                            execute,
                            provider_key: &prov_name,
                            retention: Arc::clone(&retention),
                        },
                    )
                    .await;

                    if result.success {
                        eprintln!(
                            "    ✓ passed ({}/{} tests, {:.1}s)",
                            result.tests_passed, result.tests_total, result.duration_secs,
                        );
                    } else {
                        eprintln!(
                            "    ✗ failed: {} ({})",
                            result
                                .error_category
                                .as_ref()
                                .map_or_else(|| "unknown".to_string(), ToString::to_string),
                            result.error_message.as_deref().unwrap_or("no details"),
                        );
                    }

                    results.push(result);
                }
                results
            });

            handles.push(handle);
        }

        // Await all provider tasks for this showcase.
        for handle in handles {
            let provider_results = handle.await.map_err(|e| format!("task panicked: {e}"))?;
            all_results.extend(provider_results);
        }

        eprintln!();
    }

    // Sort results into deterministic order: showcase → provider → attempt.
    all_results.sort_by(|a, b| {
        a.showcase
            .cmp(&b.showcase)
            .then(a.provider.cmp(&b.provider))
            .then(a.attempt.cmp(&b.attempt))
    });

    Ok(all_results)
}

struct SingleAttemptOptions<'a> {
    run_id: &'a str,
    artifact_dir: &'a Path,
    keep_workspaces: bool,
    capture_model_io: bool,
    execute: bool,
    provider_key: &'a str,
    retention: Arc<Mutex<RunRetentionState>>,
}

/// Runs a single benchmark attempt in an isolated temp workspace.
async fn run_single<F>(
    showcase: &Showcase,
    provider: &dyn LlmProvider,
    spec: &crate::intent::spec::IntentSpec,
    attempt: u32,
    init_workspace: &F,
    options: SingleAttemptOptions<'_>,
) -> BenchmarkResult
where
    F: Fn(&Path) -> Result<(), anyhow::Error> + Send + Sync,
{
    let process_verification = matches!(
        showcase.verification,
        ShowcaseVerification::ProcessEvidence { .. }
    );
    let evidence = run_isolated_attempt(
        AttemptRequest {
            run_id: options.run_id,
            task_id: showcase.name,
            spec,
            provider,
            provider_key: options.provider_key,
            attempt,
            artifact_dir: options.artifact_dir,
            keep_workspace: options.keep_workspaces,
            capture_model_io: options.capture_model_io,
            slug: "benchmark-showcase",
            execute: options.execute,
            process_verification,
            force_persist_failure: false,
            force_io_after_repair: false,
            credentials_missing: false,
            captured_model_io: Vec::new(),
        },
        init_workspace,
        options.retention.as_ref(),
    )
    .await;

    benchmark_result_from_evidence(showcase, provider.name(), attempt, spec, evidence)
}

pub(crate) fn process_benchmark_evidence(
    showcase: &Showcase,
    evidence: &crate::intent::attempt::AttemptEvidence,
) -> Option<BenchmarkEvidence> {
    let ShowcaseVerification::ProcessEvidence {
        evidence_kind,
        expected_route,
        expected_json_fields,
    } = showcase.verification
    else {
        return None;
    };
    let last = evidence.process_evidence.last();
    Some(BenchmarkEvidence {
        kind: evidence_kind.into(),
        status: match last {
            None => "not_run",
            Some(p) if p.failure.is_none() => "passed",
            Some(_) => "failed",
        }
        .into(),
        detail: match last {
            None => "Process verification was not reached; inspect the authoring/preflight outcome"
                .into(),
            Some(p) => p.failure.as_ref().map_or_else(
                || "Built and verified two fresh SQLite datasets over loopback HTTP".into(),
                ToString::to_string,
            ),
        },
        command: last.map(|p| p.build_command.clone()),
        expected_route: Some(expected_route.into()),
        expected_json_fields: expected_json_fields.iter().map(|f| (*f).into()).collect(),
        verification_gap: None,
        artifact_path: evidence
            .artifact_paths
            .iter()
            .find(|p| {
                Path::new(p)
                    .file_name()
                    .is_some_and(|name| name == "process-evidence.json")
            })
            .cloned(),
        process: evidence.process_evidence.clone(),
    })
}

fn benchmark_result_from_evidence(
    showcase: &Showcase,
    provider_name: &str,
    attempt: u32,
    spec: &crate::intent::spec::IntentSpec,
    evidence: crate::intent::attempt::AttemptEvidence,
) -> BenchmarkResult {
    let tests_total = if evidence.outcome.tests_total == 0 {
        spec.test_cases.len()
    } else {
        evidence.outcome.tests_total
    };
    let process_evidence = process_benchmark_evidence(showcase, &evidence);
    BenchmarkResult {
        showcase: showcase.name.to_string(),
        task_id: Some(showcase.name.to_string()),
        suite: Some(showcase.suite.as_str().to_string()),
        tags: showcase.tags.iter().map(|tag| (*tag).to_string()).collect(),
        provider: provider_name.to_string(),
        attempt,
        success: evidence.outcome.success,
        first_pass_success: Some(evidence.outcome.first_pass_success),
        repair_attempted: evidence.outcome.repair_attempted,
        repair_applied: evidence.outcome.repair_applied,
        repair_success: evidence.outcome.repair_success,
        root_cause: evidence.root_cause,
        root_cause_attribution: evidence.root_cause_attribution,
        error_category: evidence.error_category,
        error_message: if evidence.outcome.success {
            None
        } else {
            Some(
                evidence
                    .sanitized_log
                    .last()
                    .cloned()
                    .unwrap_or_else(|| evidence.outcome.terminal_status.clone()),
            )
        },
        dominant_error_code: evidence.outcome.dominant_error_code,
        mutation_retry_count: evidence.outcome.mutation_retry_count,
        repair_retry_count: evidence.outcome.repair_retry_count,
        total_retry_count: match (
            evidence.outcome.mutation_retry_count,
            evidence.outcome.repair_retry_count,
        ) {
            (Some(mutation), Some(repair)) => Some(mutation.saturating_add(repair)),
            (Some(mutation), None) => Some(mutation),
            (None, Some(repair)) => Some(repair),
            (None, None) => None,
        },
        provider_usage: ProviderUsageSummary::unavailable("provider_response_did_not_expose_usage"),
        evidence: process_evidence,
        phase_evidence: Some(evidence.phase_evidence),
        evidence_persistence: Some(evidence.evidence_persistence),
        artifact_paths: evidence.artifact_paths,
        hashes: Some(evidence.hashes),
        model_io_status: Some(evidence.model_io_status),
        omission_reason: evidence.omission_reason,
        executed: Some(evidence.executed),
        persistence_error: evidence.persistence_error,
        graph_failure: Some(evidence.graph_failure),
        tests_passed: evidence.outcome.tests_passed,
        tests_total,
        duration_secs: evidence.duration_secs,
    }
}

pub(crate) fn extract_error_codes(text: &str) -> Vec<String> {
    let mut codes: Vec<String> = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| {
            word.len() == 4
                && word.starts_with('E')
                && word[1..].chars().all(|c| c.is_ascii_digit())
        })
        .map(ToString::to_string)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    codes.sort();
    codes
}

/// Tries to load an archived intent (moved to history/ after execution).
pub(crate) fn load_archived_intent(
    workspace: &Path,
    slug: &str,
) -> Result<crate::intent::spec::IntentSpec, crate::intent::IntentError> {
    let history_path = workspace
        .join(".duumbi")
        .join("intents")
        .join("history")
        .join(format!("{slug}.yaml"));

    if history_path.exists() {
        let contents = std::fs::read_to_string(&history_path).map_err(|source| {
            crate::intent::IntentError::Io {
                path: history_path.display().to_string(),
                source,
            }
        })?;
        serde_yaml::from_str(&contents).map_err(|source| crate::intent::IntentError::Parse {
            path: history_path.display().to_string(),
            source,
        })
    } else {
        Err(crate::intent::IntentError::NotFound {
            name: slug.to_string(),
        })
    }
}

/// Filters provider configs by name.
pub(crate) fn filter_providers<'a>(
    providers: &'a [ProviderConfig],
    filter: Option<&[String]>,
) -> Vec<&'a ProviderConfig> {
    match filter {
        None => providers.iter().collect(),
        Some(names) => providers
            .iter()
            .filter(|p| {
                let name = provider_name(p);
                names.iter().any(|n| n == &name)
            })
            .collect(),
    }
}

/// Builds a stable, unique provider identifier from config.
///
/// Legacy configs with an explicit model keep the historical `provider:model`
/// identifier. Provider-only configs include role, credential env, and base URL
/// so multiple entries for the same provider remain filterable.
pub(crate) fn provider_name(config: &ProviderConfig) -> String {
    if let Some(model) = config.model.as_deref() {
        return format!("{}:{model}", config.provider);
    }

    let role = match config.role {
        crate::config::ProviderRole::Primary => "primary",
        crate::config::ProviderRole::Fallback => "fallback",
    };
    let mut name = format!("{}:auto:{role}:{}", config.provider, config.api_key_env);
    if let Some(base_url) = config.base_url.as_deref() {
        name.push(':');
        name.push_str(base_url);
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderKind, ProviderRole};

    fn provider_config(
        provider: ProviderKind,
        role: ProviderRole,
        api_key_env: &str,
        model: Option<&str>,
    ) -> ProviderConfig {
        ProviderConfig {
            provider,
            role,
            model: model.map(ToString::to_string),
            api_key_env: api_key_env.to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        }
    }

    #[test]
    fn provider_name_preserves_legacy_model_identifier() {
        let config = provider_config(
            ProviderKind::Anthropic,
            ProviderRole::Primary,
            "ANTHROPIC_API_KEY",
            Some("claude-sonnet-4-6"),
        );

        assert_eq!(provider_name(&config), "anthropic:claude-sonnet-4-6");
    }

    #[test]
    fn provider_name_distinguishes_provider_only_configs() {
        let first = provider_config(
            ProviderKind::Anthropic,
            ProviderRole::Primary,
            "ANTHROPIC_API_KEY",
            None,
        );
        let second = provider_config(
            ProviderKind::Anthropic,
            ProviderRole::Fallback,
            "ANTHROPIC_FALLBACK_API_KEY",
            None,
        );

        assert_ne!(provider_name(&first), provider_name(&second));
        assert_eq!(
            provider_name(&first),
            "anthropic:auto:primary:ANTHROPIC_API_KEY"
        );
        assert_eq!(
            provider_name(&second),
            "anthropic:auto:fallback:ANTHROPIC_FALLBACK_API_KEY"
        );
    }

    const FIXTURE_SHOWCASE: Showcase = Showcase {
        name: "repair_fixture",
        yaml: "",
        suite: ShowcaseSuite::Core,
        smoke: true,
        tags: &["fixture"],
        verification: ShowcaseVerification::I64Tests,
    };

    #[tokio::test]
    async fn bench_run_single_execute_through_repair_no_patch() {
        use crate::intent::attempt::RunRetentionState;
        use crate::intent::test_support::{
            RepairScript, ScriptedRepairProvider, init_skeleton_workspace, repair_fixture_spec,
        };

        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = repair_fixture_spec();
        let provider = ScriptedRepairProvider::new(RepairScript::NoPatch);
        let retention = Arc::new(Mutex::new(RunRetentionState::default()));
        let run_id = "run-bench-repair".to_string();
        let result = run_single(
            &FIXTURE_SHOWCASE,
            &provider,
            &spec,
            1,
            &init_skeleton_workspace,
            SingleAttemptOptions {
                run_id: &run_id,
                artifact_dir: artifacts.path(),
                keep_workspaces: false,
                capture_model_io: false,
                execute: true,
                provider_key: "mock",
                retention,
            },
        )
        .await;

        assert!(!result.success);
        assert!(result.repair_attempted);
        assert!(!result.repair_applied);
        assert_eq!(result.repair_success, Some(false));
        assert!(!result.artifact_paths.is_empty());
        assert_eq!(result.executed, Some(true));
        assert_eq!(
            result.evidence_persistence,
            Some(crate::intent::attempt::EvidencePersistence::Complete)
        );
    }

    #[tokio::test]
    async fn bench_run_single_does_not_replace_ok_false_with_logic_error() {
        use crate::intent::attempt::RunRetentionState;
        use crate::intent::test_support::{
            RepairScript, ScriptedRepairProvider, init_skeleton_workspace, repair_fixture_spec,
        };

        let artifacts = tempfile::TempDir::new().expect("artifacts");
        let spec = repair_fixture_spec();
        let provider = ScriptedRepairProvider::new(RepairScript::RepairedSuccess);
        let retention = Arc::new(Mutex::new(RunRetentionState::default()));
        let run_id = "run-bench-success".to_string();
        let result = run_single(
            &FIXTURE_SHOWCASE,
            &provider,
            &spec,
            1,
            &init_skeleton_workspace,
            SingleAttemptOptions {
                run_id: &run_id,
                artifact_dir: artifacts.path(),
                keep_workspaces: false,
                capture_model_io: false,
                execute: true,
                provider_key: "mock",
                retention,
            },
        )
        .await;

        assert!(
            result.success,
            "error={:?} terminal paths={:?}",
            result.error_message, result.artifact_paths
        );
        assert!(result.repair_attempted);
        assert!(result.repair_applied);
        assert_eq!(result.repair_success, Some(true));
        assert_eq!(result.first_pass_success, Some(false));
    }
}
