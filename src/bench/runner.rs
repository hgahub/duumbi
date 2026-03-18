//! Benchmark execution loop.
//!
//! Iterates over (showcase × provider × attempt), creating a temporary
//! workspace for each run, executing the intent pipeline, and collecting
//! [`BenchmarkResult`] entries.

use std::path::Path;
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
    F: Fn(&Path) -> Result<(), anyhow::Error>,
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

    let mut results = Vec::with_capacity(total_runs);
    let mut run_idx: usize = 0;

    for showcase in &showcase_refs {
        let spec =
            showcases::parse_showcase(showcase).map_err(|e| format!("invalid showcase: {e}"))?;

        for prov_config in &provider_configs {
            let provider = factory::create_provider(prov_config).map_err(|e| {
                format!(
                    "failed to create provider '{}': {e}",
                    provider_name(prov_config)
                )
            })?;

            for attempt in 1..=config.attempts {
                run_idx += 1;
                eprintln!(
                    "[{run_idx}/{total_runs}] {} / {} (attempt {attempt}/{})",
                    showcase.name,
                    provider.name(),
                    config.attempts,
                );

                let result = run_single(
                    showcase.name,
                    provider.as_ref(),
                    &spec,
                    attempt,
                    &init_workspace,
                )
                .await;

                if result.success {
                    eprintln!(
                        "  ✓ passed ({}/{} tests, {:.1}s)",
                        result.tests_passed, result.tests_total, result.duration_secs,
                    );
                } else {
                    eprintln!(
                        "  ✗ failed: {} ({})",
                        result
                            .error_category
                            .as_ref()
                            .map_or("unknown", |c| match c {
                                ErrorCategory::SchemaError => "schema_error",
                                ErrorCategory::TypeError => "type_error",
                                ErrorCategory::LogicError => "logic_error",
                                ErrorCategory::Crash => "crash",
                                ErrorCategory::ProviderError => "provider_error",
                                ErrorCategory::MutationFailed => "mutation_failed",
                            }),
                        result.error_message.as_deref().unwrap_or("no details"),
                    );
                }

                results.push(result);
            }
        }
    }

    Ok(results)
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
    F: Fn(&Path) -> Result<(), anyhow::Error>,
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
    let ok = intent::execute::run_execute(provider, workspace, slug)
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

/// Extracts a human-readable provider name from config.
fn provider_name(config: &ProviderConfig) -> String {
    format!("{:?}", config.provider).to_lowercase()
}
