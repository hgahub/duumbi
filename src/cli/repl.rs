//! Interactive REPL for the duumbi CLI.
//!
//! Entered when `duumbi` is invoked with no subcommand arguments and stdin is
//! connected to a terminal. Supports natural language AI mutations and `/`-prefixed
//! slash commands.
//!
//! # Architecture
//!
//! The REPL maintains a [`Session`] that holds the LLM client (if configured)
//! and conversation history. Each user turn is either:
//! - A **slash command** (`/build`, `/check`, etc.) dispatched to the existing
//!   CLI command implementations in [`super::commands`].
//! - A **natural language request** forwarded to [`orchestrator::mutate`], with
//!   session history prepended as context.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use reedline::{Prompt, PromptEditMode, PromptHistorySearch, Reedline, Signal};

use crate::agents::{LlmClient, orchestrator};
use crate::config::DuumbiConfig;
use crate::intent;
use crate::snapshot;

use super::commands;

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

/// A single completed conversation turn in the REPL session.
struct Turn {
    /// The original user request.
    request: String,
    /// Human-readable summary of the changes made.
    summary: String,
}

/// Active REPL session holding all mutable state.
struct Session {
    workspace_root: PathBuf,
    config: DuumbiConfig,
    /// LLM client, or `None` if the provider is not configured / key not found.
    client: Option<LlmClient>,
    /// Completed turns, used to build context for subsequent requests.
    history: Vec<Turn>,
}

/// Minimal REPL prompt that renders a single `> ` marker.
struct ReplPrompt;

impl Prompt for ReplPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        // Suppress reedline's default mode indicator (`〉`) so we only show one prompt marker.
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Owned(format!("(reverse-search: {}) ", history_search.term))
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Runs the interactive REPL session until the user exits.
///
/// Prints a status bar, then enters a read-eval-print loop using `reedline`
/// for line editing. Slash commands are dispatched directly; other input is
/// forwarded to the AI mutation pipeline.
pub async fn run(workspace_root: PathBuf, config: DuumbiConfig) -> Result<()> {
    let client = build_client(&config);

    print_header(&config, &workspace_root);

    let mut session = Session {
        workspace_root,
        config,
        client,
        history: Vec::new(),
    };

    let mut editor = Reedline::create();
    let prompt = ReplPrompt;

    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(buffer)) => {
                let input = buffer.trim().to_string();
                if input.is_empty() {
                    continue;
                }
                if input.starts_with('/') {
                    match session.handle_slash(&input).await {
                        Ok(true) => {
                            eprintln!("Goodbye!");
                            break;
                        }
                        Ok(false) => {}
                        Err(e) => {
                            eprintln!("Command error: {e:#}");
                        }
                    }
                } else if let Err(e) = session.handle_ai_request(&input).await {
                    eprintln!("Error: {e:#}");
                }
            }
            Ok(Signal::CtrlC) => {
                eprintln!("(Use /exit or Ctrl+D to quit)");
            }
            Ok(Signal::CtrlD) => {
                eprintln!("Goodbye!");
                break;
            }
            Err(e) => {
                eprintln!("REPL read error: {e}");
                break;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Client construction
// ---------------------------------------------------------------------------

/// Builds an [`LlmClient`] from the workspace config, or returns `None` with
/// a warning if the provider is not configured or the API key is missing.
fn build_client(config: &DuumbiConfig) -> Option<LlmClient> {
    let providers = config.effective_providers();
    if providers.is_empty() {
        return None;
    }

    match crate::agents::factory::create_provider_chain(&providers) {
        Ok(client) => Some(client),
        Err(e) => {
            eprintln!("Warning: LLM provider not available ({e}). AI mutations disabled.");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Status bar (#54)
// ---------------------------------------------------------------------------

/// Prints the welcome status bar to stderr.
///
/// Shows: `duumbi vX.Y.Z · model · workspace: name`
fn print_header(config: &DuumbiConfig, workspace_root: &Path) {
    let version = env!("CARGO_PKG_VERSION");
    let model = config
        .llm
        .as_ref()
        .map(|l| l.model.as_str())
        .unwrap_or("no model configured");

    let workspace_name = workspace_root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "workspace".to_string());

    eprintln!("duumbi v{version} · {model} · workspace: {workspace_name}");
    eprintln!("Type a request or /help for commands. Ctrl+D to exit.");
    eprintln!();
}

// ---------------------------------------------------------------------------
// Slash command dispatcher (#53)
// ---------------------------------------------------------------------------

impl Session {
    /// Dispatches a `/command [args]` line. Returns `true` if the REPL should exit.
    async fn handle_slash(&mut self, input: &str) -> Result<bool> {
        let mut parts = input.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("").trim();

        let graph_path = self.workspace_root.join(".duumbi/graph/main.jsonld");
        let output_path = self.workspace_root.join(".duumbi/build/output");

        match cmd {
            "/build" => {
                commands::build(&graph_path, &output_path).unwrap_or_else(|e| {
                    eprintln!("Build failed: {e:#}");
                });
            }

            "/run" => {
                if !output_path.exists() {
                    eprintln!("No binary found. Run /build first.");
                } else {
                    let exit_status = process::Command::new(&output_path)
                        .args(arg.split_whitespace())
                        .status()
                        .with_context(|| {
                            format!("Failed to execute '{}'", output_path.display())
                        })?;
                    if !exit_status.success() {
                        eprintln!("Process exited with {exit_status}");
                    }
                }
            }

            "/check" => {
                commands::check(&graph_path).unwrap_or_else(|e| {
                    eprintln!("Check failed: {e:#}");
                });
            }

            "/describe" => {
                commands::describe(&graph_path).unwrap_or_else(|e| {
                    eprintln!("Describe failed: {e:#}");
                });
            }

            "/undo" => {
                match snapshot::restore_latest(&self.workspace_root) {
                    Ok(true) => {
                        let remaining = snapshot::snapshot_count(&self.workspace_root).unwrap_or(0);
                        eprintln!("Undo successful. {remaining} snapshot(s) remaining.");
                        // Pop the last history entry to keep context consistent.
                        self.history.pop();
                    }
                    Ok(false) => eprintln!("Nothing to undo."),
                    Err(e) => eprintln!("Undo failed: {e:#}"),
                }
            }

            "/status" => {
                self.print_status();
            }

            "/model" => {
                let model = self
                    .config
                    .llm
                    .as_ref()
                    .map(|l| l.model.as_str())
                    .unwrap_or("not configured");
                eprintln!("Current model: {model}");
                if !arg.is_empty() {
                    eprintln!(
                        "Model switching mid-session is not yet supported.\n\
                         Edit .duumbi/config.toml and restart the REPL."
                    );
                }
            }

            "/history" => {
                if self.history.is_empty() {
                    eprintln!("No session history yet.");
                } else {
                    eprintln!(
                        "Session history ({} turn{}):",
                        self.history.len(),
                        if self.history.len() == 1 { "" } else { "s" }
                    );
                    for (i, turn) in self.history.iter().enumerate() {
                        eprintln!("  {}. \"{}\"", i + 1, turn.request);
                        eprintln!("     {}", turn.summary);
                    }
                }
            }

            "/intent" => {
                self.handle_intent_slash(arg).await?;
            }

            "/search" => {
                if arg.is_empty() {
                    eprintln!("Usage: /search <query>");
                } else {
                    super::deps::run_search(&self.workspace_root, arg, None)
                        .await
                        .unwrap_or_else(|e| eprintln!("Search failed: {e:#}"));
                }
            }

            "/deps" => {
                self.handle_deps_slash(arg).await?;
            }

            "/publish" => {
                super::publish::run_publish(&self.workspace_root, None, false, false)
                    .await
                    .unwrap_or_else(|e| eprintln!("Publish failed: {e:#}"));
            }

            "/registry" => {
                self.handle_registry_slash(arg);
            }

            "/help" => print_help(),

            "/exit" | "/quit" => return Ok(true),

            _ => {
                eprintln!("Unknown command: {cmd}");
                eprintln!("Try /help for available commands.");
            }
        }

        Ok(false)
    }

    // -------------------------------------------------------------------------
    // AI mutation handler (#52, #55, #57)
    // -------------------------------------------------------------------------

    /// Handles a natural language AI mutation request.
    ///
    /// Prepends session history for context (#55), calls the LLM, applies the
    /// patch, then auto-validates and auto-builds (#57).
    async fn handle_ai_request(&mut self, request: &str) -> Result<()> {
        let Some(ref client) = self.client else {
            eprintln!(
                "AI mutations are not available.\n\
                 Add an [llm] section to .duumbi/config.toml and restart."
            );
            return Ok(());
        };

        let graph_path = self.workspace_root.join(".duumbi/graph/main.jsonld");

        // Read current graph
        let source_str = fs::read_to_string(&graph_path)
            .with_context(|| format!("Failed to read '{}'", graph_path.display()))?;
        let source: serde_json::Value =
            serde_json::from_str(&source_str).context("Failed to parse graph JSON")?;

        // Estimate context size for the status line (#54)
        let ctx_chars: usize = source_str.len()
            + self
                .history
                .iter()
                .map(|t| t.request.len() + t.summary.len())
                .sum::<usize>();
        let ctx_k = ctx_chars as f64 / 4000.0;

        // Build prompt with conversation history (#55)
        let prompt = build_prompt_with_history(request, &self.history);

        eprint!("Thinking… (~{ctx_k:.1}k context)");

        // Run AI mutation with streaming text output
        let result =
            match orchestrator::mutate_streaming(client, &source, &prompt, 3, false, |text| {
                eprint!("{text}");
            })
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!();
                    eprintln!("{e:#}");
                    return Ok(());
                }
            };
        eprintln!(); // newline after streamed text (or after "Thinking…" if no text)

        // Show diff summary
        let diff = orchestrator::describe_changes(&source, &result.patched);
        eprintln!(
            "\n{} tool call{} applied:\n{}",
            result.ops_count,
            if result.ops_count == 1 { "" } else { "s" },
            diff
        );

        // Save snapshot + write updated graph
        snapshot::save_snapshot(&self.workspace_root, &source_str)
            .context("Failed to save snapshot")?;
        let patched_str = serde_json::to_string_pretty(&result.patched)
            .context("Failed to serialize patched graph")?;
        fs::write(&graph_path, &patched_str)
            .with_context(|| format!("Failed to write '{}'", graph_path.display()))?;

        // Auto-build after mutation (#57)
        let output_path = self.workspace_root.join(".duumbi/build/output");
        eprint!("\nBuilding… ");
        match commands::build(&graph_path, &output_path) {
            Ok(()) => {} // build() already prints "Build successful: ..."
            Err(e) => {
                eprintln!("Build failed: {e:#}");
                eprintln!(
                    "(Graph saved. Use /undo to revert or describe the fix in your next request.)"
                );
            }
        }

        // Record turn in session history (#55)
        self.history.push(Turn {
            request: request.to_string(),
            summary: diff,
        });

        eprintln!();
        Ok(())
    }

    // -------------------------------------------------------------------------
    // /intent handler (#86)
    // -------------------------------------------------------------------------

    /// Handles `/intent <subcommand> [args]` within the REPL.
    ///
    /// Supported forms:
    /// - `/intent` or `/intent list` — list active intents
    /// - `/intent create <description>` — generate + save an intent spec
    /// - `/intent review [name]` — show intent details
    /// - `/intent execute <name>` — execute an intent
    /// - `/intent status [name]` — show intent status
    async fn handle_intent_slash(&mut self, arg: &str) -> Result<()> {
        let mut parts = arg.splitn(2, ' ');
        let subcmd = parts.next().unwrap_or("").trim();
        let rest = parts.next().unwrap_or("").trim();

        match subcmd {
            "" | "list" => {
                intent::review::print_intent_list(&self.workspace_root)
                    .unwrap_or_else(|e| eprintln!("Error: {e}"));
            }
            "create" => {
                if rest.is_empty() {
                    eprintln!("Usage: /intent create <description>");
                    return Ok(());
                }
                let Some(ref client) = self.client else {
                    eprintln!("AI not available — add [llm] section to .duumbi/config.toml.");
                    return Ok(());
                };
                match intent::create::run_create(client, &self.workspace_root, rest, false).await {
                    Ok(slug) => eprintln!("Intent '{slug}' saved."),
                    Err(e) => eprintln!("Error: {e:#}"),
                }
            }
            "review" => {
                if rest.is_empty() {
                    intent::review::print_intent_list(&self.workspace_root)
                        .unwrap_or_else(|e| eprintln!("Error: {e}"));
                } else {
                    intent::review::print_intent_detail(&self.workspace_root, rest)
                        .unwrap_or_else(|e| eprintln!("Error: {e}"));
                }
            }
            "execute" => {
                if rest.is_empty() {
                    eprintln!("Usage: /intent execute <name>");
                    return Ok(());
                }
                let Some(ref client) = self.client else {
                    eprintln!("AI not available — add [llm] section to .duumbi/config.toml.");
                    return Ok(());
                };
                match intent::execute::run_execute(client, &self.workspace_root, rest).await {
                    Ok(true) => eprintln!("Intent '{rest}' completed successfully."),
                    Ok(false) => eprintln!("Intent '{rest}' failed."),
                    Err(e) => eprintln!("Error: {e:#}"),
                }
            }
            "status" => {
                if rest.is_empty() {
                    intent::status::print_status_list(&self.workspace_root)
                        .unwrap_or_else(|e| eprintln!("Error: {e}"));
                } else {
                    intent::status::print_status_detail(&self.workspace_root, rest)
                        .unwrap_or_else(|e| eprintln!("Error: {e}"));
                }
            }
            _ => {
                eprintln!("Unknown intent subcommand: {subcmd}");
                eprintln!(
                    "Available: /intent list, /intent create <desc>, \
                     /intent review [name], /intent execute <name>, /intent status [name]"
                );
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // /deps handler
    // -------------------------------------------------------------------------

    /// Handles `/deps <subcommand>` within the REPL.
    async fn handle_deps_slash(&mut self, arg: &str) -> Result<()> {
        let mut parts = arg.splitn(2, ' ');
        let subcmd = parts.next().unwrap_or("").trim();
        let rest = parts.next().unwrap_or("").trim();

        match subcmd {
            "" | "list" => {
                super::deps::run_deps_list(&self.workspace_root)
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            "audit" => {
                super::deps::run_deps_audit(&self.workspace_root)
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            "tree" => {
                super::deps::run_deps_tree(&self.workspace_root, 10)
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            "update" => {
                let name = if rest.is_empty() { None } else { Some(rest) };
                super::deps::run_deps_update(&self.workspace_root, name)
                    .await
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            "vendor" => {
                super::deps::run_deps_vendor(&self.workspace_root, false, None)
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            "install" => {
                let frozen = rest == "--frozen";
                super::deps::run_deps_install(&self.workspace_root, frozen)
                    .await
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            _ => {
                eprintln!("Unknown deps subcommand: {subcmd}");
                eprintln!(
                    "Available: /deps list, /deps audit, /deps tree, /deps update, /deps vendor, /deps install"
                );
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // /registry handler
    // -------------------------------------------------------------------------

    /// Handles `/registry <subcommand>` within the REPL.
    fn handle_registry_slash(&self, arg: &str) {
        let subcmd = arg.split(' ').next().unwrap_or("").trim();

        match subcmd {
            "" | "list" => {
                super::registry::run_registry_list(&self.workspace_root)
                    .unwrap_or_else(|e| eprintln!("Error: {e:#}"));
            }
            _ => {
                eprintln!("Unknown registry subcommand: {subcmd}");
                eprintln!("Available: /registry list");
                eprintln!("For other registry operations, use the CLI directly.");
            }
        }
    }

    // -------------------------------------------------------------------------
    // /status helper
    // -------------------------------------------------------------------------

    /// Prints workspace status: graph path, binary, history depth, model.
    fn print_status(&self) {
        let graph_path = self.workspace_root.join(".duumbi/graph/main.jsonld");
        let output_path = self.workspace_root.join(".duumbi/build/output");
        let history_count = snapshot::snapshot_count(&self.workspace_root).unwrap_or(0);
        let session_turns = self.history.len();

        eprintln!("Workspace: {}", self.workspace_root.display());
        eprintln!(
            "  Graph:        {} {}",
            graph_path.display(),
            if graph_path.exists() {
                "✓"
            } else {
                "✗ missing"
            }
        );
        eprintln!(
            "  Binary:       {} {}",
            output_path.display(),
            if output_path.exists() {
                "✓"
            } else {
                "(not built)"
            }
        );
        eprintln!("  Snapshots:    {history_count} (undo depth)");
        eprintln!("  Session turns: {session_turns}");
        if let Some(llm) = &self.config.llm {
            eprintln!("  Model:        {} ({})", llm.model, llm.provider);
        } else {
            eprintln!("  Model:        not configured");
        }
    }
}

// ---------------------------------------------------------------------------
// Prompt with history (#55)
// ---------------------------------------------------------------------------

/// Builds a mutation prompt that includes the session conversation history.
///
/// The history is prepended as a context note so the LLM understands what
/// has already been done in this session.
fn build_prompt_with_history(request: &str, history: &[Turn]) -> String {
    if history.is_empty() {
        return request.to_string();
    }

    let mut ctx = String::from(
        "Context from this session (these changes are already applied, do not repeat):\n",
    );
    for (i, turn) in history.iter().enumerate() {
        ctx.push_str(&format!("  {}. \"{}\"\n", i + 1, turn.request));
    }
    ctx.push('\n');
    ctx.push_str(request);
    ctx
}

// ---------------------------------------------------------------------------
// Help text
// ---------------------------------------------------------------------------

/// Prints the available slash commands to stderr.
fn print_help() {
    eprintln!(
        "\
Slash commands:
  /build              Compile the current graph to a native binary
  /run [args]         Run the compiled binary
  /check              Validate the graph without compiling
  /describe           Print human-readable pseudocode of the graph
  /undo               Restore the previous graph snapshot
  /status             Show workspace, model, and session information
  /history            Show session conversation history (sent as context to LLM)
  /model              Show the current LLM model

Intent commands:
  /intent             List all active intents
  /intent create <description>   Generate and save a new intent spec
  /intent review [name]          Show intent details
  /intent execute <name>         Execute an intent end-to-end
  /intent status [name]          Show intent execution status

Registry & dependency commands:
  /search <query>     Search registries for modules
  /publish            Package and publish the current module
  /registry list      List configured registries
  /deps list          List declared dependencies
  /deps audit         Verify dependency integrity
  /deps tree          Show the dependency tree
  /deps update [name] Update dependencies to latest compatible versions
  /deps vendor        Copy cached dependencies into vendor/

  /help               Show this help text
  /exit               Exit the REPL

Any other input is sent to the AI as a mutation request."
    );
}
