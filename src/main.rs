//! Duumbi CLI entry point.
//!
//! Orchestrates the full compilation pipeline: parse → graph → validate →
//! compile → link. Uses `anyhow` for error handling at the application boundary.
//! Async runtime (tokio) is needed for `duumbi add` and the interactive REPL,
//! which make LLM API calls.

mod agents;
mod cli;
mod compiler;
mod config;
mod deps;
mod errors;
mod examples;
mod graph;
mod hash;
mod intent;
mod manifest;
mod parser;
mod patch;
mod registry;
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
    // interactive REPL instead of showing help.
    if std::env::args().len() == 1 && io::stdin().is_terminal() {
        let workspace_root = PathBuf::from(".");
        if workspace_root.join(".duumbi").exists() {
            let config = config::load_config(&workspace_root).unwrap_or_default();
            if let Err(e) = cli::repl::run(workspace_root, config).await {
                eprintln!("error: {e:#}");
                process::exit(1);
            }
            return;
        }
        // No workspace — fall through to normal CLI parsing (shows help).
    }

    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {e:#}");
        process::exit(1);
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
            cli::init::run_init(&base)
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
                cli::DepsSubcommand::Vendor { all, include } => {
                    cli::deps::run_deps_vendor(&workspace, all, include.as_deref())
                }
            }
        }
        Commands::Intent { subcommand } => {
            let workspace = PathBuf::from(".");
            run_intent(subcommand, workspace).await
        }
        Commands::Upgrade => cli::upgrade::run_upgrade(&PathBuf::from(".")),
        Commands::Studio { port, dev } => studio(port, dev).await,
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
/// Loads `.duumbi/config.toml` for LLM provider settings, saves a snapshot
/// of the current graph, calls the LLM, applies the patch, validates, and
/// writes the updated graph if the user confirms (or `--yes` is passed).
async fn add(request: &str, yes: bool) -> Result<()> {
    let workspace_root = PathBuf::from(".");

    let cfg = config::load_config(&workspace_root).context(
        "Cannot run 'duumbi add': no .duumbi/config.toml found or [llm] section missing.\n\
         Run `duumbi init` and add an [llm] section to .duumbi/config.toml.",
    )?;

    let llm_cfg = cfg.llm.ok_or_else(|| {
        anyhow::anyhow!(
            "No [llm] section in .duumbi/config.toml.\n\
             Add provider, model, and api_key_env settings."
        )
    })?;

    let api_key = llm_cfg
        .resolve_api_key()
        .context("Failed to resolve LLM API key")?;

    let graph_path = workspace_root
        .join(".duumbi")
        .join("graph")
        .join("main.jsonld");

    let source_str = fs::read_to_string(&graph_path)
        .with_context(|| format!("Failed to read '{}'", graph_path.display()))?;

    let source: serde_json::Value =
        serde_json::from_str(&source_str).context("Failed to parse current graph as JSON")?;

    let client = match llm_cfg.provider {
        config::LlmProvider::Anthropic => agents::LlmClient::anthropic(&llm_cfg.model, api_key),
        config::LlmProvider::OpenAI => agents::LlmClient::openai(&llm_cfg.model, api_key),
    };

    eprintln!("Calling {} ({})…", llm_cfg.provider, llm_cfg.model);

    let result = orchestrator::mutate(&client, &source, request, 3).await?;

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
            intent::create::run_create(&client, &workspace, &description, yes).await?;
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
            let ok = intent::execute::run_execute(&client, &workspace, &name).await?;
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

/// Builds an [`agents::LlmClient`] from workspace config, or bails with a helpful message.
fn require_llm_client(workspace: &Path) -> Result<agents::LlmClient> {
    let cfg = config::load_config(workspace).context(
        "Cannot run intent commands: no .duumbi/config.toml found.\n\
         Run `duumbi init` and add an [llm] section to .duumbi/config.toml.",
    )?;
    let llm_cfg = cfg.llm.ok_or_else(|| {
        anyhow::anyhow!(
            "No [llm] section in .duumbi/config.toml.\n\
             Add provider, model, and api_key_env settings."
        )
    })?;
    let api_key = llm_cfg
        .resolve_api_key()
        .context("Failed to resolve LLM API key")?;
    Ok(match llm_cfg.provider {
        config::LlmProvider::Anthropic => agents::LlmClient::anthropic(&llm_cfg.model, api_key),
        config::LlmProvider::OpenAI => agents::LlmClient::openai(&llm_cfg.model, api_key),
    })
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
