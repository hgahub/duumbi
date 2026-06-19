//! Duumbi CLI entry point.
//!
//! Orchestrates the full compilation pipeline: parse → graph → validate →
//! compile → link. Uses `anyhow` for error handling at the application boundary.
//! Async runtime (tokio) is needed for `duumbi add` and the interactive REPL,
//! which make LLM API calls.

#[allow(dead_code)] // Binary uses streaming path; non-streaming API is used via lib crate
mod agents;
mod bench;
mod cli;
mod compiler;
mod config;
#[allow(dead_code)] // Used indirectly via intent::execute context enrichment
mod context;
mod contracts;
mod credentials;
mod deps;
mod determinism;
mod errors;
mod examples;
mod graph;
mod hash;
mod intent;
mod interaction;
#[allow(dead_code)] // Binary uses a subset of knowledge API; rest is used via lib crate
mod knowledge;
mod logging;
mod loop_native;
mod manifest;
mod mcp;
mod parser;
mod patch;
mod properties;
#[allow(dead_code, unused_imports)]
// Binary uses query engine through CLI; library exports full API
mod query;
mod registry;
#[allow(dead_code, unused_imports)]
mod rewrite;
#[allow(dead_code)] // Binary uses a subset; full API used via lib crate
mod session;
mod snapshot;
#[allow(dead_code)] // Library and later telemetry cycles use the full module surface.
mod telemetry;
mod tools;
mod types;
#[allow(dead_code)] // Library workflow API is also compiled into the binary crate.
mod workflow;
mod workspace;

use std::fs;
use std::io::{self, IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Parser;

use agents::orchestrator;
use cli::{Cli, Commands};

const EXIT_SUCCESS: i32 = 0;
const EXIT_FAILURE: i32 = 1;

#[tokio::main]
async fn main() {
    // If invoked with no arguments and stdin is a terminal, enter the
    // interactive REPL — even without an initialised workspace.
    if std::env::args().len() == 1 && io::stdin().is_terminal() {
        let workspace_root = cli::repl::resolve_repl_workspace_root(&PathBuf::from("."));
        let config = match config::load_effective_config(&workspace_root) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        };
        let config = match auto_configure_startup_editor(&workspace_root, config) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        };
        let logging_runtime = initialize_logging(
            &workspace_root,
            &config.config,
            &logging::LoggingOverrides::default(),
        );
        let repl_started = logging_runtime
            .performance()
            .map(|performance| performance.record_start("repl"));
        tracing::info!(command = "repl", "duumbi command started");
        let config = match auto_configure_startup_providers(&workspace_root, config).await {
            Ok(config) => config,
            Err(e) => {
                if let (Some(performance), Some(started)) =
                    (logging_runtime.performance(), repl_started)
                {
                    performance.record_error("repl", started, &format!("{e:#}"));
                }
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        };
        if let Err(e) = cli::repl::run(workspace_root, config).await {
            if let (Some(performance), Some(started)) =
                (logging_runtime.performance(), repl_started)
            {
                performance.record_error("repl", started, &format!("{e:#}"));
            }
            eprintln!("error: {e:#}");
            process::exit(1);
        }
        if let (Some(performance), Some(started)) = (logging_runtime.performance(), repl_started) {
            performance.record_success("repl", started);
        }
        tracing::info!(command = "repl", "duumbi command finished");
        return;
    }

    let cli = Cli::parse();
    let workspace_root = logging_workspace_root(&cli.command);
    let command_name = command_name(&cli.command);
    let logging_config = config::load_effective_config(&workspace_root)
        .map(|effective| effective.config)
        .unwrap_or_default();
    let logging_runtime =
        initialize_logging(&workspace_root, &logging_config, &logging_overrides(&cli));
    let command_started = logging_runtime
        .performance()
        .map(|performance| performance.record_start(command_name));
    tracing::info!(command = command_name, "duumbi command started");
    let exit_code = match run(cli).await {
        Ok(exit_code) => exit_code,
        Err(e) => {
            if let (Some(performance), Some(started)) =
                (logging_runtime.performance(), command_started)
            {
                performance.record_error(command_name, started, &format!("{e:#}"));
            }
            tracing::error!(command = command_name, error = %e, "duumbi command failed");
            eprintln!("error: {e:#}");
            process::exit(EXIT_FAILURE);
        }
    };
    if let (Some(performance), Some(started)) = (logging_runtime.performance(), command_started) {
        if exit_code == EXIT_SUCCESS {
            performance.record_success(command_name, started);
        } else {
            performance.record_error(
                command_name,
                started,
                &format!("command exited with status {exit_code}"),
            );
        }
    }
    if exit_code == EXIT_SUCCESS {
        tracing::info!(command = command_name, "duumbi command finished");
    } else {
        tracing::error!(
            command = command_name,
            exit_code,
            "duumbi command exited with non-zero status"
        );
        process::exit(exit_code);
    }
}

fn logging_overrides(cli: &Cli) -> logging::LoggingOverrides {
    logging::LoggingOverrides {
        general_level: cli.log_level.map(Into::into),
        general_path: cli.log_file.clone(),
        general_mode: cli.log_mode.map(Into::into),
        performance_enabled: cli.perf_log.then_some(true),
        performance_path: cli.perf_log_file.clone(),
        performance_mode: cli.perf_log_mode.map(Into::into),
    }
}

fn initialize_logging(
    workspace_root: &Path,
    config: &config::DuumbiConfig,
    overrides: &logging::LoggingOverrides,
) -> logging::RuntimeLogging {
    match logging::initialize(workspace_root, config, overrides) {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("warning: failed to initialize logging: {e}");
            logging::RuntimeLogging::disabled()
        }
    }
}

fn logging_workspace_root(command: &Commands) -> PathBuf {
    match command {
        Commands::Init { name: Some(name) } => PathBuf::from(name),
        Commands::Build {
            input: Some(input), ..
        }
        | Commands::Check {
            input: Some(input), ..
        }
        | Commands::Describe { input: Some(input) } => {
            cli::commands::workspace_root_for_graph_input(input)
                .unwrap_or_else(|| PathBuf::from("."))
        }
        _ => PathBuf::from("."),
    }
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Init { .. } => "init",
        Commands::Build { .. } => "build",
        Commands::Run { .. } => "run",
        Commands::Check { .. } => "check",
        Commands::Describe { .. } => "describe",
        Commands::Add { .. } => "add",
        Commands::Undo => "undo",
        Commands::Rewrite { .. } => "rewrite",
        Commands::Deps { .. } => "deps",
        Commands::Search { .. } => "search",
        Commands::Intent { .. } => "intent",
        Commands::Loop { .. } => "loop",
        Commands::Registry { .. } => "registry",
        Commands::Publish { .. } => "publish",
        Commands::Yank { .. } => "yank",
        Commands::Telemetry { .. } => "telemetry",
        Commands::Upgrade => "upgrade",
        Commands::Benchmark { .. } => "benchmark",
        Commands::Determinism { .. } => "determinism",
        Commands::Phase15E2e { .. } => "phase15-e2e",
        Commands::Completions { .. } => "completions",
        Commands::Studio { .. } => "studio",
        Commands::Knowledge { .. } => "knowledge",
        Commands::Provider { .. } => "provider",
        Commands::Mcp { .. } => "mcp",
    }
}

fn auto_configure_startup_editor(
    workspace_root: &Path,
    effective_config: config::EffectiveConfig,
) -> Result<config::EffectiveConfig> {
    if effective_config.config.editor.is_some() {
        return Ok(effective_config);
    }

    let Some(editor) = config::discover_editor_command() else {
        return Ok(effective_config);
    };

    let mut updated_config = effective_config;
    updated_config.user_config.editor = Some(editor.clone());
    updated_config.config.editor = Some(editor);

    if let Err(e) = config::save_user_config(&updated_config.user_config) {
        eprintln!("warning: failed to save startup editor config: {e}");
        return Ok(updated_config);
    }

    match config::load_effective_config(workspace_root) {
        Ok(reloaded) => Ok(reloaded),
        Err(e) => {
            eprintln!("warning: failed to reload config after startup editor setup: {e}");
            Ok(updated_config)
        }
    }
}

async fn auto_configure_startup_providers(
    workspace_root: &Path,
    effective_config: config::EffectiveConfig,
) -> Result<config::EffectiveConfig> {
    let setups = cli::provider_startup::discover_env_provider_setups(&effective_config);
    if setups.is_empty() {
        return Ok(effective_config);
    }

    let report = run_provider_startup_spinner(effective_config.clone(), setups).await;
    eprintln!();
    for result in &report.results {
        if result.success {
            eprintln!(
                "Provider configured from {}: {}",
                result.env_var, result.message
            );
        } else {
            eprintln!(
                "Provider setup skipped for {} ({}): {}",
                result.provider, result.env_var, result.message
            );
        }
    }

    if report.any_success() {
        config::load_effective_config(workspace_root)
            .map_err(|e| anyhow::anyhow!("Failed to reload provider config: {e}"))
    } else {
        Ok(effective_config)
    }
}

async fn run_provider_startup_spinner(
    effective_config: config::EffectiveConfig,
    setups: Vec<cli::provider_startup::EnvProviderSetup>,
) -> cli::provider_startup::EnvProviderSetupReport {
    use tokio::time::{Duration, interval};

    let handle = tokio::spawn(async move {
        cli::provider_startup::configure_env_providers(&effective_config, setups).await
    });
    let mut frames = interval(Duration::from_millis(250));
    let mut dots = 0usize;

    loop {
        if handle.is_finished() {
            return match handle.await {
                Ok(report) => report,
                Err(e) => {
                    eprint!("\r{: <40}\r", "");
                    io::stderr().flush().ok();
                    eprintln!("Provider setup task failed: {e}");
                    cli::provider_startup::EnvProviderSetupReport { results: vec![] }
                }
            };
        }

        let suffix = ".".repeat(dots);
        eprint!("\rSetting up providers{suffix:<3}");
        io::stderr().flush().ok();
        dots = (dots + 1) % 4;
        frames.tick().await;
    }
}

async fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Commands::Init { name } => {
            let base = match name {
                Some(ref n) => {
                    let p = PathBuf::from(n);
                    fs::create_dir_all(&p)
                        .with_context(|| format!("Failed to create directory '{n}'"))?;
                    p
                }
                None => PathBuf::from("."),
            };
            let summary = cli::init::run_init(&base)?;
            eprintln!(
                "{} Project initialized at {}",
                cli::theme::check_mark(),
                summary.workspace_root.join(".duumbi").display()
            );
            Ok(EXIT_SUCCESS)
        }
        Commands::Build {
            input,
            output,
            trace,
            offline,
        } => {
            if offline {
                eprintln!("Building in offline mode (vendor + workspace only)...");
            }
            let input_path = resolve_input(input.as_deref())?;
            let output_path = resolve_output(output.as_deref())?;
            let telemetry = if trace {
                telemetry::TelemetryBuildMode::Trace
            } else {
                telemetry::TelemetryBuildMode::Off
            };
            let options = telemetry::BuildOptions::new(offline, telemetry);
            success_exit(cli::commands::build_with_options(
                &input_path,
                &output_path,
                options,
            ))
        }
        Commands::Run { args } => {
            let workspace = PathBuf::from(".");
            if workspace.join(".duumbi").exists() {
                let output = workspace::run_workspace_binary(&workspace, &args)?;
                print!("{}", output.stdout);
                eprint!("{}", output.stderr);
                return Ok(output.exit_code);
            }

            let output_path = resolve_output(None)?;
            let status = process::Command::new(&output_path)
                .args(&args)
                .status()
                .with_context(|| format!("Failed to run binary '{}'", output_path.display()))?;
            Ok(status.code().unwrap_or(-1))
        }
        Commands::Check {
            input,
            properties,
            seed,
            cases,
            property_output,
        } => {
            let input_path = resolve_input(input.as_deref())?;
            if properties {
                success_exit(cli::commands::check_with_properties(
                    &input_path,
                    properties::PropertyRunOptions {
                        seed,
                        cases,
                        output_path: property_output,
                        ..Default::default()
                    },
                ))
            } else {
                success_exit(cli::commands::check(&input_path))
            }
        }
        Commands::Describe { input } => {
            let input_path = resolve_input(input.as_deref())?;
            success_exit(cli::commands::describe(&input_path))
        }
        Commands::Add { request, yes } => success_exit(add(&request, yes).await),
        Commands::Undo => success_exit(undo()),
        Commands::Rewrite { subcommand } => {
            let workspace = PathBuf::from(".");
            success_exit(cli::rewrite::run_rewrite(subcommand, &workspace))
        }
        Commands::Search { query, registry } => {
            let workspace = PathBuf::from(".");
            success_exit(cli::deps::run_search(&workspace, &query, registry.as_deref()).await)
        }
        Commands::Deps { subcommand } => {
            let workspace = PathBuf::from(".");
            success_exit(match subcommand {
                cli::DepsSubcommand::List => cli::deps::run_deps_list(&workspace),
                cli::DepsSubcommand::Add {
                    name,
                    path,
                    registry,
                } => {
                    cli::deps::run_deps_add(&workspace, &name, path.as_deref(), registry.as_deref())
                        .await
                }
                cli::DepsSubcommand::Remove { name } => {
                    cli::deps::run_deps_remove(&workspace, &name)
                }
                cli::DepsSubcommand::Audit => cli::deps::run_deps_audit(&workspace),
                cli::DepsSubcommand::Tree { depth } => cli::deps::run_deps_tree(&workspace, depth),
                cli::DepsSubcommand::Update { name } => {
                    cli::deps::run_deps_update(&workspace, name.as_deref()).await
                }
                cli::DepsSubcommand::Install { frozen } => {
                    cli::deps::run_deps_install(&workspace, frozen).await
                }
                cli::DepsSubcommand::Vendor { all, include } => {
                    cli::deps::run_deps_vendor(&workspace, all, include.as_deref())
                }
            })
        }
        Commands::Publish {
            registry,
            dry_run,
            yes,
        } => {
            let workspace = PathBuf::from(".");
            success_exit(
                cli::publish::run_publish(&workspace, registry.as_deref(), dry_run, yes).await,
            )
        }
        Commands::Registry { subcommand } => {
            let workspace = PathBuf::from(".");
            success_exit(run_registry(subcommand, &workspace).await)
        }
        Commands::Intent { subcommand } => {
            let workspace = PathBuf::from(".");
            run_intent(subcommand, workspace).await
        }
        Commands::Loop { subcommand } => {
            let workspace = PathBuf::from(".");
            run_loop(subcommand, workspace)
        }
        Commands::Yank {
            specifier,
            registry,
            yes,
        } => {
            let workspace = PathBuf::from(".");
            success_exit(
                cli::yank::run_yank(&workspace, &specifier, registry.as_deref(), yes).await,
            )
        }
        Commands::Telemetry { subcommand } => {
            let workspace = PathBuf::from(".");
            success_exit(run_telemetry(subcommand, &workspace))
        }
        Commands::Upgrade => success_exit(cli::upgrade::run_upgrade(&PathBuf::from("."))),
        Commands::Knowledge { subcommand } => {
            let workspace = PathBuf::from(".");
            success_exit(run_knowledge(subcommand, workspace))
        }
        Commands::Benchmark {
            suite,
            smoke,
            showcase,
            provider,
            attempts,
            output,
            ci,
            baseline,
        } => {
            run_benchmark(BenchmarkRunArgs {
                suite,
                smoke,
                showcase,
                provider_filter: provider,
                attempts,
                output,
                ci,
                baseline,
            })
            .await
        }
        Commands::Determinism { subcommand } => run_determinism(subcommand).await,
        Commands::Phase15E2e {
            task,
            provider,
            attempts,
            output,
            port,
        } => success_exit(cli::phase15_e2e::run(&task, &provider, attempts, output, port).await),
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut <Cli as clap::CommandFactory>::command(),
                "duumbi",
                &mut std::io::stdout(),
            );
            Ok(EXIT_SUCCESS)
        }
        Commands::Studio { port, dev } => studio(port, dev).await,
        Commands::Provider { subcommand } => {
            let workspace = PathBuf::from(".");
            success_exit(run_provider(subcommand, &workspace).await)
        }
        Commands::Mcp { sse, port } => success_exit(run_mcp(sse, port).await),
    }
}

async fn run_determinism(subcommand: cli::DeterminismSubcommand) -> Result<i32> {
    match subcommand {
        cli::DeterminismSubcommand::Replay {
            suite,
            smoke,
            showcase,
            provider,
            attempts,
            output,
            artifact_dir,
            markdown_output,
            ci,
            min_exact_agreement,
            min_semantic_agreement,
            min_behavioral_agreement,
            keep_workspaces,
        } => {
            let workspace = PathBuf::from(".");
            let effective_config = config::load_effective_config(&workspace)?;
            let provider_source = provider_source_label(effective_config.provider_source);
            let cfg = effective_config.config;
            let providers = cfg.effective_providers();
            if providers.is_empty() {
                anyhow::bail!(
                    "No LLM providers configured. Use `duumbi provider add ...` to save a user-level provider."
                );
            }

            let attempts = attempts.unwrap_or(2);
            let started_at = iso8601_now();
            let run_id = determinism_run_id(&started_at, suite, smoke);
            let config = determinism::runner::ReplayConfig {
                run_id,
                attempts,
                providers,
                showcase_filter: showcase,
                provider_filter: provider,
                suite_filter: suite.map(|suite| match suite {
                    cli::BenchmarkSuiteArg::Core => bench::showcases::ShowcaseSuite::Core,
                    cli::BenchmarkSuiteArg::Scaled => bench::showcases::ShowcaseSuite::Scaled,
                }),
                smoke,
                artifact_dir,
                started_at,
                source_commit: current_git_commit().unwrap_or_else(|| "unknown".to_string()),
                provider_source: provider_source.to_string(),
                keep_workspaces,
            };

            let report = determinism::runner::run_replay(&config, |path| {
                cli::init::run_init(path).map(|_| ())
            })
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            let json = serde_json::to_string_pretty(&report)
                .context("Failed to serialize determinism replay report")?;
            match output {
                Some(path) => {
                    write_text_file(&path, &json).with_context(|| {
                        format!("Failed to write replay report to '{}'", path.display())
                    })?;
                    eprintln!("Replay report written to {}", path.display());
                }
                None => println!("{json}"),
            }
            if let Some(path) = markdown_output {
                write_text_file(&path, &report.to_markdown_summary()).with_context(|| {
                    format!(
                        "Failed to write replay Markdown summary to '{}'",
                        path.display()
                    )
                })?;
                eprintln!("Replay Markdown summary written to {}", path.display());
            }

            let thresholds_pass = determinism::metrics::replay_ci_thresholds_pass(
                &report.metrics.exact_graph_agreement_rate,
                &report.metrics.semantic_graph_agreement_rate,
                &report.metrics.behavioral_agreement_rate,
                min_exact_agreement,
                min_semantic_agreement,
                min_behavioral_agreement,
            );
            if ci && !thresholds_pass {
                Ok(EXIT_FAILURE)
            } else {
                Ok(EXIT_SUCCESS)
            }
        }
    }
}

fn provider_source_label(source: config::ProviderConfigSource) -> &'static str {
    match source {
        config::ProviderConfigSource::None => "none",
        config::ProviderConfigSource::System => "system",
        config::ProviderConfigSource::User => "user",
        config::ProviderConfigSource::Workspace => "workspace",
        config::ProviderConfigSource::LegacySystem => "legacy-system",
        config::ProviderConfigSource::LegacyUser => "legacy-user",
        config::ProviderConfigSource::LegacyWorkspace => "legacy-workspace",
    }
}

fn determinism_run_id(
    started_at: &str,
    suite: Option<cli::BenchmarkSuiteArg>,
    smoke: bool,
) -> String {
    let suite = match suite {
        Some(cli::BenchmarkSuiteArg::Core) | None => "core",
        Some(cli::BenchmarkSuiteArg::Scaled) => "scaled",
    };
    let mode = if smoke { "smoke" } else { "full" };
    let timestamp = started_at
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    format!("duumbi-720-{timestamp}-{suite}-{mode}")
}

fn current_git_commit() -> Option<String> {
    let output = process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?;
    let commit = commit.trim();
    (!commit.is_empty()).then(|| commit.to_string())
}

fn write_text_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory '{}'", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("Failed to write '{}'", path.display()))
}

fn success_exit(result: Result<()>) -> Result<i32> {
    result.map(|()| EXIT_SUCCESS)
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn run_telemetry(subcommand: cli::TelemetrySubcommand, workspace: &Path) -> Result<()> {
    match subcommand {
        cli::TelemetrySubcommand::Inspect {
            telemetry_dir,
            crash,
            map_path,
        } => {
            let telemetry_dir = match telemetry_dir {
                Some(telemetry_dir) => telemetry_dir,
                None => default_telemetry_dir(workspace)?,
            };
            let report = telemetry::inspect_crash_artifacts(
                &telemetry_dir,
                crash.as_deref(),
                map_path.as_deref(),
            )?;
            println!("{}", report.to_cli_output());
            Ok(())
        }
        cli::TelemetrySubcommand::RepairContext {
            telemetry_dir,
            crash,
            map_path,
            graph_sources,
            crash_entry,
        } => {
            let telemetry_dir = match telemetry_dir {
                Some(telemetry_dir) => telemetry_dir,
                None => default_telemetry_dir(workspace)?,
            };
            let mut options = telemetry::RepairContextOptions::new(telemetry_dir);
            options.crash_path = crash;
            options.map_path = map_path;
            options.graph_sources = graph_sources;
            if let Some(line) = crash_entry {
                options.crash_entry = telemetry::CrashEntrySelection::LineNumber(line as usize);
            }

            let context = telemetry::repair_crash_context(&options)?;
            println!("{}", serde_json::to_string_pretty(&context)?);
            Ok(())
        }
        cli::TelemetrySubcommand::RepairValidate {
            context,
            patch,
            graph,
            workspace,
            module,
            tests,
            output,
        } => {
            if workspace.is_some() || module.is_some() {
                anyhow::bail!(
                    "workspace repair validation is not implemented yet; use --graph single-file validation"
                );
            }

            let context_json = fs::read_to_string(&context).with_context(|| {
                format!("Failed to read repair context '{}'", context.display())
            })?;
            let crash_context: telemetry::RepairCrashContext = serde_json::from_str(&context_json)
                .with_context(|| {
                    format!("Failed to parse repair context '{}'", context.display())
                })?;
            if !telemetry::repair_context_includes_graph_source(&crash_context, &graph) {
                anyhow::bail!(
                    "repair validation graph '{}' is not present in repair context graph_sources",
                    graph.display()
                );
            }
            let patch_json = fs::read_to_string(&patch)
                .with_context(|| format!("Failed to read repair patch '{}'", patch.display()))?;
            let patch_value: serde_json::Value = serde_json::from_str(&patch_json)
                .with_context(|| format!("Failed to parse repair patch '{}'", patch.display()))?;

            let request = telemetry::RepairValidationRequest::single_graph(
                crash_context,
                patch_value,
                graph.clone(),
            );
            let exe = std::env::current_exe().context("Failed to resolve current executable")?;
            let mut runner =
                telemetry::RepairValidationCommandRunner::single_graph(exe, graph, tests)?;
            let evidence = telemetry::validate_repair_candidate_with_runner(request, &mut runner)?;
            let evidence_json = serde_json::to_string_pretty(&evidence)?;
            if let Some(output) = output {
                fs::write(&output, &evidence_json).with_context(|| {
                    format!(
                        "Failed to write repair validation evidence '{}'",
                        output.display()
                    )
                })?;
            }
            println!("{evidence_json}");
            Ok(())
        }
    }
}

fn default_telemetry_dir(workspace: &Path) -> Result<PathBuf> {
    let telemetry_dir = config::load_effective_config(workspace)
        .context("Failed to load telemetry config")?
        .config
        .telemetry
        .unwrap_or_default()
        .effective_artifact_dir(workspace);
    Ok(telemetry_dir)
}

/// Resolves the input file path: explicit path or workspace discovery.
fn resolve_input(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }

    let workspace_main = PathBuf::from(".duumbi/graph/main.jsonld");
    if workspace_main.exists() {
        return Ok(workspace_main);
    }

    anyhow::bail!(
        "No input file specified and no workspace found. \
         Use `duumbi init` to create a workspace or specify an input file."
    )
}

/// Resolves the output path: explicit path or workspace default.
fn resolve_output(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }

    let workspace_build = PathBuf::from(".duumbi/build");
    if workspace_build.exists() {
        return Ok(workspace::workspace_output_path(Path::new(".")));
    }

    Ok(PathBuf::from("output"))
}

/// Applies an AI-generated mutation to the graph.
///
/// Loads `.duumbi/config.toml` for LLM provider settings (supports both
/// `[[providers]]` and legacy `[llm]` formats), saves a snapshot of the
/// current graph, calls the LLM, applies the patch, validates, and writes
/// the updated graph if the user confirms (or `--yes` is passed).
async fn add(request: &str, yes: bool) -> Result<()> {
    let workspace_root = PathBuf::from(".");

    let graph_path = workspace_root
        .join(".duumbi")
        .join("graph")
        .join("main.jsonld");

    let source_str = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read '{}'", graph_path.display()))?;

    let source: serde_json::Value =
        serde_json::from_str(&source_str).context("Failed to parse current graph as JSON")?;

    let client = require_llm_client(&workspace_root)?;
    let agent_policy = config::load_effective_config(&workspace_root)
        .map(|effective| {
            let provider = config::ProviderKind::from_provider_name(client.name());
            effective.config.effective_agent_policy(provider.as_ref())
        })
        .unwrap_or_default();

    // Detect multi-module workspace: if there are other .jsonld files besides
    // main.jsonld, skip Call validation (cross-module calls can't be resolved
    // from main.jsonld alone).
    let graph_dir = workspace_root.join(".duumbi/graph");
    let is_multi_module = graph_dir
        .read_dir()
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    let p = e.path();
                    p.extension().is_some_and(|ext| ext == "jsonld")
                        && p.file_name().is_some_and(|n| n != "main.jsonld")
                })
                .count()
                > 0
        })
        .unwrap_or(false);

    {
        let sp = cli::progress::spinner(&format!("Calling {}…", client.name()));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        sp.finish_and_clear();
    }

    let result = orchestrator::mutate_streaming_with_timeout(
        &client,
        &source,
        request,
        agent_policy.mutation_retries,
        agent_policy.mutation_timeout_secs,
        is_multi_module,
        |text| {
            eprint!("{text}");
        },
    )
    .await?;
    eprintln!();

    let result = match result {
        orchestrator::MutationOutcome::Success(r) => r,
        orchestrator::MutationOutcome::NeedsClarification(question) => {
            eprintln!("? {question}");
            return Ok(());
        }
    };

    let diff = orchestrator::describe_changes(&source, &result.patched);
    eprintln!(
        "\nProposed changes ({} tool call{}):\n{}",
        result.ops_count,
        if result.ops_count == 1 { "" } else { "s" },
        diff
    );

    if !yes {
        eprint!("\nApply changes? [y/N] ");
        io::stderr().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read confirmation")?;

        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    snapshot::save_snapshot(&workspace_root, &source_str).context("Failed to save snapshot")?;

    let patched_str = serde_json::to_string_pretty(&result.patched)
        .context("Failed to serialize patched graph")?;

    fs::write(&graph_path, patched_str)
        .with_context(|| format!("Failed to write '{}'", graph_path.display()))?;

    eprintln!("Graph updated. Run `duumbi build` to compile.");
    Ok(())
}

/// Dispatches `duumbi intent` subcommands.
async fn run_intent(subcommand: cli::IntentSubcommand, workspace: PathBuf) -> Result<i32> {
    match subcommand {
        cli::IntentSubcommand::Create { description, yes } => {
            let client = require_llm_client(&workspace)?;
            let mut log = Vec::new();
            intent::create::run_create(&client, &workspace, &description, yes, &mut log).await?;
            for line in &log {
                eprintln!("{line}");
            }
            Ok(EXIT_SUCCESS)
        }
        cli::IntentSubcommand::Review { name, edit } => {
            success_exit(match name {
                None => intent::review::print_intent_list(&workspace)
                    .map_err(|e| anyhow::anyhow!("{e}")),
                Some(ref slug) if edit => intent::review::edit_intent(&workspace, slug)
                    .map_err(|e| anyhow::anyhow!("{e}")),
                Some(ref slug) => intent::review::print_intent_detail(&workspace, slug)
                    .map_err(|e| anyhow::anyhow!("{e}")),
            })
        }
        cli::IntentSubcommand::Execute { name } => {
            let mut log = Vec::new();
            if intent::execute::run_execute_blocking_preflight_with_progress(
                &workspace,
                &name,
                &mut log,
                &|line| {
                    eprintln!("{line}");
                },
            )? {
                return Ok(EXIT_FAILURE);
            }

            let client = require_llm_client(&workspace)?;
            let ok = intent::execute::run_execute_with_progress(
                &client,
                &workspace,
                &name,
                &mut log,
                &|line| {
                    eprintln!("{line}");
                },
            )
            .await?;
            if !ok {
                return Ok(EXIT_FAILURE);
            }
            Ok(EXIT_SUCCESS)
        }
        cli::IntentSubcommand::Status { name } => match name {
            None => success_exit(
                intent::status::print_status_list(&workspace).map_err(|e| anyhow::anyhow!("{e}")),
            ),
            Some(ref slug) => success_exit(
                intent::status::print_status_detail(&workspace, slug)
                    .map_err(|e| anyhow::anyhow!("{e}")),
            ),
        },
    }
}

/// Dispatches `duumbi loop` native workflow subcommands.
fn run_loop(subcommand: cli::LoopSubcommand, workspace: PathBuf) -> Result<i32> {
    match subcommand {
        cli::LoopSubcommand::IntakeSpec { intent, json } => {
            let result = loop_native::run_native_intake_spec(&workspace, &intent)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                eprintln!("Native Loop run {}: {}", result.run_id, result.state);
                for artifact in &result.artifacts {
                    eprintln!("- {}: {}", artifact.artifact_kind, artifact.path);
                }
                for reason in &result.blocking_reasons {
                    eprintln!("- blocked: {reason}");
                }
            }
            if result.state == loop_native::LoopRunState::Completed {
                Ok(EXIT_SUCCESS)
            } else {
                Ok(EXIT_FAILURE)
            }
        }
        cli::LoopSubcommand::ReviewPatch {
            intent,
            patch,
            json,
        } => {
            let contents = fs::read_to_string(&patch)
                .with_context(|| format!("Failed to read '{}'", patch.display()))?;
            let patch_doc: patch::GraphPatch = serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse GraphPatch '{}'", patch.display()))?;
            let target = loop_native::graph_patch_review_target(&workspace, &intent, &patch_doc)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&target)?);
            } else {
                eprintln!("Review target: {:?}", target.kind);
                eprintln!("Work item: {}", target.work_item_id);
                match &target.change_set {
                    loop_native::ChangeSet::GraphPatch {
                        operation_count,
                        affected_nodes,
                    } => {
                        eprintln!("GraphPatch operations: {operation_count}");
                        for node in affected_nodes {
                            eprintln!("- {node}");
                        }
                    }
                    loop_native::ChangeSet::GraphSnapshotDiff { before, after } => {
                        eprintln!("Snapshot diff: {before} -> {after}");
                    }
                    loop_native::ChangeSet::GeneratedArtifactDiff { before, after } => {
                        eprintln!("Artifact diff: {before} -> {after}");
                    }
                }
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

/// Dispatches `duumbi knowledge` subcommands.
fn run_knowledge(subcommand: cli::KnowledgeSubcommand, workspace: PathBuf) -> Result<()> {
    use knowledge::learning;
    use knowledge::store::KnowledgeStore;
    use knowledge::types::KnowledgeNode;

    match subcommand {
        cli::KnowledgeSubcommand::List { r#type } => {
            let store = KnowledgeStore::new(&workspace).map_err(|e| anyhow::anyhow!("{e}"))?;
            let nodes = if let Some(type_filter) = r#type {
                let node_type = match type_filter.as_str() {
                    "success" => knowledge::types::TYPE_SUCCESS,
                    "failure" => knowledge::types::TYPE_FAILURE,
                    "decision" => knowledge::types::TYPE_DECISION,
                    "pattern" => knowledge::types::TYPE_PATTERN,
                    other => {
                        anyhow::bail!(
                            "Unknown type '{other}'. Use: success, failure, decision, pattern"
                        )
                    }
                };
                store.query_by_type(node_type)
            } else {
                store.load_all()
            };

            if nodes.is_empty() {
                eprintln!("No knowledge nodes found.");
            } else {
                let mut table = comfy_table::Table::new();
                table.load_preset(comfy_table::presets::UTF8_FULL_CONDENSED);
                table.set_header(vec!["Type", "ID"]);
                for node in &nodes {
                    table.add_row(vec![node.node_type(), node.id()]);
                }
                eprintln!("{table}");
            }
            Ok(())
        }
        cli::KnowledgeSubcommand::Show { id } => {
            let store = KnowledgeStore::new(&workspace).map_err(|e| anyhow::anyhow!("{e}"))?;
            let all = store.load_all();
            if let Some(node) = all.iter().find(|n| n.id() == id) {
                let json = serde_json::to_string_pretty(node).context("serialize node")?;
                println!("{json}");
            } else {
                eprintln!("Node not found: {id}");
            }
            Ok(())
        }
        cli::KnowledgeSubcommand::Prune { older_than } => {
            let store = KnowledgeStore::new(&workspace).map_err(|e| anyhow::anyhow!("{e}"))?;
            let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(older_than));
            let all = store.load_all();
            let mut removed = 0u32;
            for node in &all {
                let ts = match node {
                    KnowledgeNode::Success(r) => r.timestamp,
                    KnowledgeNode::Failure(r) => r.timestamp,
                    KnowledgeNode::Decision(r) => r.timestamp,
                    KnowledgeNode::Pattern(r) => r.timestamp,
                };
                if ts < cutoff
                    && store
                        .remove_node(node.id())
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                {
                    removed += 1;
                }
            }
            eprintln!("Pruned {removed} node(s) older than {older_than} days.");
            Ok(())
        }
        cli::KnowledgeSubcommand::Stats => {
            let store = KnowledgeStore::new(&workspace).map_err(|e| anyhow::anyhow!("{e}"))?;
            let stats = store.stats();
            let success_count = learning::success_count(&workspace);
            eprintln!("Knowledge store:");
            eprintln!("  Success records:  {}", stats.successes);
            eprintln!("  Failure records:  {}", stats.failures);
            eprintln!("  Decision records: {}", stats.decisions);
            eprintln!("  Pattern records:  {}", stats.patterns);
            eprintln!("  Total:            {}", stats.total());
            eprintln!();
            eprintln!("Learning log: {success_count} entries in successes.jsonl");
            Ok(())
        }
    }
}

/// Dispatches `duumbi provider` subcommands.
async fn run_provider(subcommand: cli::ProviderSubcommand, _workspace: &Path) -> Result<()> {
    let mut cfg = config::load_user_config().unwrap_or_default();
    let mut save_provider_config = false;

    let lines = match subcommand {
        cli::ProviderSubcommand::List => cli::provider::list_providers(&cfg),
        cli::ProviderSubcommand::Add {
            provider_type,
            api_key_env,
            role,
            base_url,
            auth_token_env,
        } => {
            let mut args = format!("{provider_type} {api_key_env}");
            if role != "primary" {
                args.push_str(&format!(" --role {role}"));
            }
            if let Some(ref url) = base_url {
                args.push_str(&format!(" --base-url {url}"));
            }
            if let Some(ref token_env) = auth_token_env {
                args.push_str(&format!(" --auth-token-env {token_env}"));
            }
            save_provider_config = true;
            cli::provider::add_provider(&mut cfg, &args)
        }
        cli::ProviderSubcommand::Remove { selector } => {
            save_provider_config = true;
            cli::provider::remove_provider(&mut cfg, &selector)
        }
        cli::ProviderSubcommand::Set {
            index,
            field,
            value,
        } => {
            save_provider_config = true;
            cli::provider::set_provider_field(&mut cfg, &format!("{index} {field} {value}"))
        }
        cli::ProviderSubcommand::Catalog { subcommand } => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let store = agents::model_catalog::ModelCatalogStore::for_home(home);
            match subcommand {
                cli::ProviderCatalogSubcommand::Status => cli::provider::catalog_status(&store),
                cli::ProviderCatalogSubcommand::Check {
                    catalog_url,
                    sha256_url,
                } => {
                    let urls = cli::provider::catalog_remote_urls(catalog_url, sha256_url);
                    cli::provider::catalog_check(&store, urls, cli::provider::current_unix_secs())
                        .await
                }
                cli::ProviderCatalogSubcommand::Approve {
                    hash,
                    catalog_url,
                    sha256_url,
                } => {
                    let urls = cli::provider::catalog_remote_urls(catalog_url, sha256_url);
                    cli::provider::catalog_approve(
                        &store,
                        urls,
                        Some(hash.as_str()),
                        cli::provider::current_unix_secs(),
                    )
                    .await
                }
                cli::ProviderCatalogSubcommand::Skip { hash } => {
                    cli::provider::catalog_skip(&store, hash.as_deref())
                }
                cli::ProviderCatalogSubcommand::Remind { hours } => {
                    cli::provider::catalog_remind(&store, hours, cli::provider::current_unix_secs())
                }
                cli::ProviderCatalogSubcommand::Disable => cli::provider::catalog_disable(&store),
            }
        }
    };

    cli::provider::print_output_lines(&lines);

    // Persist config if a mutation succeeded.
    if save_provider_config
        && lines
            .iter()
            .any(|l| l.style == cli::mode::OutputStyle::Success)
    {
        config::save_user_config(&cfg)
            .map_err(|e| anyhow::anyhow!("Failed to save config: {e}"))?;
    }

    Ok(())
}

/// Dispatches `duumbi registry` subcommands.
async fn run_registry(subcommand: cli::RegistrySubcommand, workspace: &Path) -> Result<()> {
    match subcommand {
        cli::RegistrySubcommand::Add { name, url } => {
            cli::registry::run_registry_add(workspace, &name, &url)
        }
        cli::RegistrySubcommand::List => cli::registry::run_registry_list(workspace),
        cli::RegistrySubcommand::Remove { name } => {
            cli::registry::run_registry_remove(workspace, &name)
        }
        cli::RegistrySubcommand::Default { name } => {
            cli::registry::run_registry_default(workspace, &name)
        }
        cli::RegistrySubcommand::Login { registry, token } => {
            cli::registry::run_registry_login(workspace, &registry, token.as_deref()).await
        }
        cli::RegistrySubcommand::Logout { registry } => {
            cli::registry::run_registry_logout(registry.as_deref())
        }
    }
}

/// Builds an [`agents::LlmClient`] from workspace config, or bails with a helpful message.
///
/// Uses the `[[providers]]` config if available, falling back to the legacy
/// `[llm]` section for backward compatibility.
fn require_llm_client(workspace: &Path) -> Result<agents::LlmClient> {
    let cfg = config::load_effective_config(workspace)?.config;

    let providers = cfg.effective_providers();
    if providers.is_empty() {
        anyhow::bail!(
            "No LLM provider configured.\n\
             Use `/provider` in the REPL or `duumbi provider add ...` to save a user-level provider."
        );
    }

    agents::factory::create_provider_chain_for_global_access(&providers)
        .map_err(|e| anyhow::anyhow!("Failed to create LLM provider: {e}"))
}

/// Starts the DUUMBI Studio web platform.
///
/// Looks for the `studio` binary next to the running `duumbi` executable
/// (both are built from the same cargo workspace). If found, execs into it;
/// otherwise bails with build instructions.
async fn studio(port: u16, _dev: bool) -> Result<i32> {
    let workspace = PathBuf::from(".");
    if !workspace.join(".duumbi").exists() {
        anyhow::bail!("No duumbi workspace found. Run `duumbi init` first.");
    }

    // Try to find the `studio` binary in the same directory as `duumbi`
    if let Ok(self_path) = std::env::current_exe()
        && let Some(dir) = self_path.parent()
    {
        let studio_bin = dir.join("studio");
        if studio_bin.exists() {
            let workspace_abs = fs::canonicalize(&workspace).unwrap_or_else(|_| workspace.clone());
            let status = process::Command::new(&studio_bin)
                .arg("--workspace")
                .arg(&workspace_abs)
                .arg("--port")
                .arg(port.to_string())
                .status()
                .with_context(|| format!("Failed to execute '{}'", studio_bin.display()))?;
            return Ok(status.code().unwrap_or(EXIT_FAILURE));
        }
    }

    anyhow::bail!(
        "Studio binary not found. Build with: cargo build -p duumbi-studio --features ssr"
    )
}

/// Runs the benchmark suite against configured LLM providers.
struct BenchmarkRunArgs {
    suite: Option<cli::BenchmarkSuiteArg>,
    smoke: bool,
    showcase: Option<Vec<String>>,
    provider_filter: Option<Vec<String>>,
    attempts: Option<u32>,
    output: Option<PathBuf>,
    ci: bool,
    baseline: Option<PathBuf>,
}

async fn run_benchmark(args: BenchmarkRunArgs) -> Result<i32> {
    let BenchmarkRunArgs {
        suite,
        smoke,
        showcase,
        provider_filter,
        attempts: explicit_attempts,
        output,
        ci,
        baseline,
    } = args;
    let workspace = PathBuf::from(".");
    let cfg = config::load_effective_config(&workspace)?.config;

    let providers = cfg.effective_providers();
    if providers.is_empty() {
        anyhow::bail!(
            "No LLM providers configured. Use `duumbi provider add ...` to save a user-level provider."
        );
    }

    // Resolve attempts: explicit > CI default (20) > normal default (5)
    let attempts = explicit_attempts.unwrap_or(if ci { 20 } else { 5 });

    let config = bench::runner::BenchmarkConfig {
        attempts,
        providers,
        showcase_filter: showcase,
        provider_filter,
        suite_filter: suite.map(|suite| match suite {
            cli::BenchmarkSuiteArg::Core => bench::showcases::ShowcaseSuite::Core,
            cli::BenchmarkSuiteArg::Scaled => bench::showcases::ShowcaseSuite::Scaled,
        }),
        smoke,
    };

    let started_at = iso8601_now();

    let results =
        bench::runner::run_benchmark(&config, |path| cli::init::run_init(path).map(|_| ()))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

    let finished_at = iso8601_now();

    let report =
        bench::report::BenchmarkReport::from_results(results, attempts, started_at, finished_at);

    // Print human-readable summary to stderr
    report.print_summary();
    report.print_error_breakdown();

    // Baseline comparison
    if let Some(ref baseline_path) = baseline {
        let base =
            bench::report::load_baseline(baseline_path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let regressions = bench::report::detect_regressions(&report, &base, 0.05);
        bench::report::print_regressions(&regressions);
    }

    // Output JSON
    match output {
        Some(ref path) => {
            report
                .write_to_file(path)
                .with_context(|| format!("Failed to write report to '{}'", path.display()))?;
            eprintln!("Report written to {}", path.display());
        }
        None => {
            let json = report.to_json().context("Failed to serialize report")?;
            println!("{json}");
        }
    }

    // CI exit code
    if ci && !report.kill_criterion_met {
        Ok(EXIT_FAILURE)
    } else {
        Ok(EXIT_SUCCESS)
    }
}

/// Returns the current time as an ISO-8601 string (UTC, second precision).
fn iso8601_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Convert to a basic ISO-8601 UTC timestamp
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    // Simple date calculation (good enough for timestamps)
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{s:02}Z")
}

/// Converts days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Civil days algorithm (Howard Hinnant)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Starts the MCP server using stdio (or SSE when `--sse` is passed).
///
/// Creates an [`mcp::server::McpServer`] rooted at the current working
/// directory and runs the JSON-RPC 2.0 stdio loop.
async fn run_mcp(sse: bool, port: u16) -> Result<()> {
    let workspace = PathBuf::from(".");

    if sse {
        eprintln!(
            "Warning: SSE transport is not yet implemented. \
             Falling back to stdio transport (the MCP default). \
             (Requested port: {port})"
        );
    }

    let server = mcp::server::McpServer::new(workspace);
    tokio::task::spawn_blocking(move || server.run_stdio())
        .await
        .context("MCP server task panicked")?
        .context("MCP server exited with error")
}

/// Reverts the last AI mutation by restoring the most recent snapshot.
fn undo() -> Result<()> {
    let workspace_root = PathBuf::from(".");

    match snapshot::restore_latest(&workspace_root)? {
        true => {
            let remaining = snapshot::snapshot_count(&workspace_root).unwrap_or(0);
            eprintln!("Undo successful. {} snapshot(s) remaining.", remaining);
            Ok(())
        }
        false => {
            anyhow::bail!("Nothing to undo — no snapshots found in .duumbi/history/.");
        }
    }
}
