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
mod credentials;
mod deps;
mod errors;
mod examples;
mod graph;
mod hash;
mod intent;
mod interaction;
#[allow(dead_code)] // Binary uses a subset of knowledge API; rest is used via lib crate
mod knowledge;
mod manifest;
mod mcp;
mod parser;
mod patch;
#[allow(dead_code, unused_imports)]
// Binary uses query engine through CLI; library exports full API
mod query;
mod registry;
#[allow(dead_code)] // Binary uses a subset; full API used via lib crate
mod session;
mod snapshot;
mod tools;
mod types;

use std::fs;
use std::io::{self, IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Parser;

use agents::orchestrator;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    // If invoked with no arguments and stdin is a terminal, enter the
    // interactive REPL — even without an initialised workspace.
    if std::env::args().len() == 1 && io::stdin().is_terminal() {
        let workspace_root = PathBuf::from(".");
        let config = match config::load_effective_config(&workspace_root) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        };
        let config = match auto_configure_startup_providers(&workspace_root, config).await {
            Ok(config) => config,
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        };
        if let Err(e) = cli::repl::run(workspace_root, config).await {
            eprintln!("error: {e:#}");
            process::exit(1);
        }
        return;
    }

    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {e:#}");
        process::exit(1);
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

async fn run(cli: Cli) -> Result<()> {
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
            Ok(())
        }
        Commands::Build {
            input,
            output,
            offline,
        } => {
            if offline {
                eprintln!("Building in offline mode (vendor + workspace only)...");
            }
            let input_path = resolve_input(input.as_deref())?;
            let output_path = resolve_output(output.as_deref())?;
            cli::commands::build_with_opts(&input_path, &output_path, offline)
        }
        Commands::Run { args } => {
            let binary = resolve_output(None)?;
            if !binary.exists() {
                anyhow::bail!(
                    "Binary not found at '{}'. Run `duumbi build` first.",
                    binary.display()
                );
            }
            let status = process::Command::new(&binary)
                .args(&args)
                .status()
                .with_context(|| format!("Failed to execute '{}'", binary.display()))?;
            process::exit(status.code().unwrap_or(1));
        }
        Commands::Check { input } => {
            let input_path = resolve_input(input.as_deref())?;
            cli::commands::check(&input_path)
        }
        Commands::Describe { input } => {
            let input_path = resolve_input(input.as_deref())?;
            cli::commands::describe(&input_path)
        }
        Commands::Add { request, yes } => add(&request, yes).await,
        Commands::Undo => undo(),
        Commands::Search { query, registry } => {
            let workspace = PathBuf::from(".");
            cli::deps::run_search(&workspace, &query, registry.as_deref()).await
        }
        Commands::Deps { subcommand } => {
            let workspace = PathBuf::from(".");
            match subcommand {
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
            }
        }
        Commands::Publish {
            registry,
            dry_run,
            yes,
        } => {
            let workspace = PathBuf::from(".");
            cli::publish::run_publish(&workspace, registry.as_deref(), dry_run, yes).await
        }
        Commands::Registry { subcommand } => {
            let workspace = PathBuf::from(".");
            run_registry(subcommand, &workspace).await
        }
        Commands::Intent { subcommand } => {
            let workspace = PathBuf::from(".");
            run_intent(subcommand, workspace).await
        }
        Commands::Yank {
            specifier,
            registry,
            yes,
        } => {
            let workspace = PathBuf::from(".");
            cli::yank::run_yank(&workspace, &specifier, registry.as_deref(), yes).await
        }
        Commands::Upgrade => cli::upgrade::run_upgrade(&PathBuf::from(".")),
        Commands::Knowledge { subcommand } => {
            let workspace = PathBuf::from(".");
            run_knowledge(subcommand, workspace)
        }
        Commands::Benchmark {
            showcase,
            provider,
            attempts,
            output,
            ci,
            baseline,
        } => run_benchmark(showcase, provider, attempts, output, ci, baseline).await,
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut <Cli as clap::CommandFactory>::command(),
                "duumbi",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        Commands::Studio { port, dev } => studio(port, dev).await,
        Commands::Provider { subcommand } => {
            let workspace = PathBuf::from(".");
            run_provider(subcommand, &workspace)
        }
        Commands::Mcp { sse, port } => run_mcp(sse, port).await,
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

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
        return Ok(workspace_build.join("output"));
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

    let result =
        orchestrator::mutate_streaming(&client, &source, request, 3, is_multi_module, |text| {
            eprint!("{text}");
        })
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
async fn run_intent(subcommand: cli::IntentSubcommand, workspace: PathBuf) -> Result<()> {
    match subcommand {
        cli::IntentSubcommand::Create { description, yes } => {
            let client = require_llm_client(&workspace)?;
            let mut log = Vec::new();
            intent::create::run_create(&client, &workspace, &description, yes, &mut log).await?;
            for line in &log {
                eprintln!("{line}");
            }
            Ok(())
        }
        cli::IntentSubcommand::Review { name, edit } => {
            match name {
                None => intent::review::print_intent_list(&workspace)
                    .map_err(|e| anyhow::anyhow!("{e}")),
                Some(ref slug) if edit => intent::review::edit_intent(&workspace, slug)
                    .map_err(|e| anyhow::anyhow!("{e}")),
                Some(ref slug) => intent::review::print_intent_detail(&workspace, slug)
                    .map_err(|e| anyhow::anyhow!("{e}")),
            }
        }
        cli::IntentSubcommand::Execute { name } => {
            let client = require_llm_client(&workspace)?;
            let mut log = Vec::new();
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
                process::exit(1);
            }
            Ok(())
        }
        cli::IntentSubcommand::Status { name } => match name {
            None => {
                intent::status::print_status_list(&workspace).map_err(|e| anyhow::anyhow!("{e}"))
            }
            Some(ref slug) => intent::status::print_status_detail(&workspace, slug)
                .map_err(|e| anyhow::anyhow!("{e}")),
        },
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
                    "decision" => knowledge::types::TYPE_DECISION,
                    "pattern" => knowledge::types::TYPE_PATTERN,
                    other => {
                        anyhow::bail!("Unknown type '{other}'. Use: success, decision, pattern")
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
fn run_provider(subcommand: cli::ProviderSubcommand, _workspace: &Path) -> Result<()> {
    let mut cfg = config::load_user_config().unwrap_or_default();

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
            cli::provider::add_provider(&mut cfg, &args)
        }
        cli::ProviderSubcommand::Remove { selector } => {
            cli::provider::remove_provider(&mut cfg, &selector)
        }
        cli::ProviderSubcommand::Set {
            index,
            field,
            value,
        } => cli::provider::set_provider_field(&mut cfg, &format!("{index} {field} {value}")),
    };

    cli::provider::print_output_lines(&lines);

    // Persist config if a mutation succeeded.
    if lines
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
async fn studio(port: u16, _dev: bool) -> Result<()> {
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
            process::exit(status.code().unwrap_or(1));
        }
    }

    anyhow::bail!(
        "Studio binary not found. Build with: cargo build -p duumbi-studio --features ssr"
    )
}

/// Runs the benchmark suite against configured LLM providers.
async fn run_benchmark(
    showcase: Option<Vec<String>>,
    provider_filter: Option<Vec<String>>,
    explicit_attempts: Option<u32>,
    output: Option<PathBuf>,
    ci: bool,
    baseline: Option<PathBuf>,
) -> Result<()> {
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
        process::exit(1);
    }

    Ok(())
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
