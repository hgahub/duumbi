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

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use crate::agents::LlmProvider;
use crate::agents::factory;
use crate::bench::report::{BenchmarkResult, ErrorCategory, categorize_error};
use crate::bench::showcases::{self, Showcase};
use crate::config::ProviderConfig;
use crate::intent;
use crate::intent::spec::IntentStatus;

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
    let showcase_refs: Vec<&Showcase> =
        showcases::filter_showcases(config.showcase_filter.as_deref());

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

    let mut all_results: Vec<BenchmarkResult> = Vec::with_capacity(total_runs);

    for showcase in &showcase_refs {
        let spec =
            showcases::parse_showcase(showcase).map_err(|e| format!("invalid showcase: {e}"))?;

        let spec = Arc::new(spec);
        let showcase_name = showcase.name;

        // Spawn one task per provider; attempts within each task are sequential.
        let mut handles = Vec::with_capacity(provider_configs.len());
        for prov_config in &provider_configs {
            let provider: Arc<dyn LlmProvider> =
                Arc::from(factory::create_provider(prov_config).map_err(|e| {
                    format!(
                        "failed to create provider '{}': {e}",
                        provider_name(prov_config)
                    )
                })?);

            let spec_clone = Arc::clone(&spec);
            let init_clone = Arc::clone(&init_workspace);
            let attempts = config.attempts;
            let prov_name = provider.name().to_string();

            let handle = tokio::spawn(async move {
                let mut results = Vec::with_capacity(attempts as usize);
                for attempt in 1..=attempts {
                    eprintln!("  [{showcase_name} / {prov_name}] attempt {attempt}/{attempts}",);

                    let result = run_single(
                        showcase_name,
                        provider.as_ref(),
                        &spec_clone,
                        attempt,
                        &*init_clone,
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

/// Runs a single benchmark attempt in an isolated temp workspace.
async fn run_single<F>(
    showcase_name: &str,
    provider: &dyn LlmProvider,
    spec: &crate::intent::spec::IntentSpec,
    attempt: u32,
    init_workspace: &F,
) -> BenchmarkResult
where
    F: Fn(&Path) -> Result<(), anyhow::Error> + Send + Sync,
{
    let start = Instant::now();

    let result = run_in_temp_workspace(provider, spec, init_workspace).await;

    let duration_secs = start.elapsed().as_secs_f64();

    match result {
        Ok((tests_passed, tests_total)) => BenchmarkResult {
            showcase: showcase_name.to_string(),
            provider: provider.name().to_string(),
            attempt,
            success: tests_passed == tests_total,
            error_category: if tests_passed < tests_total {
                Some(ErrorCategory::LogicError)
            } else {
                None
            },
            error_message: if tests_passed < tests_total {
                Some(format!("only {tests_passed}/{tests_total} tests passed"))
            } else {
                None
            },
            tests_passed,
            tests_total,
            duration_secs,
        },
        Err(msg) => {
            let category = categorize_error(&msg);
            BenchmarkResult {
                showcase: showcase_name.to_string(),
                provider: provider.name().to_string(),
                attempt,
                success: false,
                error_category: Some(category),
                error_message: Some(msg),
                tests_passed: 0,
                tests_total: spec.test_cases.len(),
                duration_secs,
            }
        }
    }
}

/// Creates an isolated workspace, saves the intent, and runs `run_execute`.
///
/// Returns `(tests_passed, tests_total)` on success.
async fn run_in_temp_workspace<F>(
    provider: &dyn LlmProvider,
    spec: &crate::intent::spec::IntentSpec,
    init_workspace: &F,
) -> Result<(usize, usize), String>
where
    F: Fn(&Path) -> Result<(), anyhow::Error>,
{
    let tmp = tempfile::TempDir::new().map_err(|e| format!("tempdir creation failed: {e}"))?;
    let workspace = tmp.path();

    // Initialize workspace
    init_workspace(workspace).map_err(|e| format!("init failed: {e}"))?;

    // Save intent spec
    let slug = "benchmark-showcase";
    let mut run_spec = spec.clone();
    run_spec.status = IntentStatus::Pending;
    intent::save_intent(workspace, slug, &run_spec)
        .map_err(|e| format!("failed to save intent: {e}"))?;

    // Execute intent
    let mut log = Vec::new();
    let ok = intent::execute::run_execute(provider, workspace, slug, &mut log)
        .await
        .map_err(|e| format!("{e}"))?;

    // Read back the spec to get test results
    let final_spec = intent::load_intent(workspace, slug)
        .or_else(|_| load_archived_intent(workspace, slug))
        .map_err(|e| format!("failed to read final spec: {e}"))?;

    let tests_total = spec.test_cases.len();
    let tests_passed = final_spec.execution.as_ref().map_or(0, |e| e.tests_passed);

    if ok {
        Ok((tests_passed, tests_total))
    } else {
        // The intent pipeline returned false (failure)
        Err(format!(
            "intent execution failed: {tests_passed}/{tests_total} tests passed"
        ))
    }
}

/// Tries to load an archived intent (moved to history/ after execution).
fn load_archived_intent(
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
fn filter_providers<'a>(
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
/// Includes provider kind and model to distinguish multiple entries for the
/// same provider (e.g. two Anthropic configs with different models).
fn provider_name(config: &ProviderConfig) -> String {
    let resolved = crate::agents::model_catalog::resolve_provider_config(
        config,
        &crate::agents::model_catalog::ModelSelectionContext::default(),
    );
    format!("{}:{}", resolved.provider, resolved.model)
}
