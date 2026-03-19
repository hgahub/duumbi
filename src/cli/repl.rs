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
use crate::session::SessionManager;
use crate::snapshot;

use super::commands;
use super::completion::{SlashCommandCompleter, SlashCommandHinter};
use super::progress;
use super::theme;

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
    /// Persistent session manager for cross-restart state.
    session_mgr: SessionManager,
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

    // Initialize persistent session — always start fresh.
    // Previous sessions are available via /resume.
    let mut session_mgr = SessionManager::load_or_create(&workspace_root)
        .map_err(|e| anyhow::anyhow!("session init: {e}"))?;

    // If there's an unsaved session from a crash, archive it
    if session_mgr.has_pending_session() {
        session_mgr
            .archive()
            .map_err(|e| anyhow::anyhow!("session archive: {e}"))?;
    }

    print_header(&config, &workspace_root);

    // Empty workspace detection: if main.jsonld only has the skeleton (Const 0 + Return),
    // show guided suggestions
    let graph_path = workspace_root.join(".duumbi/graph/main.jsonld");
    if let Ok(content) = fs::read_to_string(&graph_path)
        && content.contains("\"duumbi:value\": 0")
        && !content.contains("\"duumbi:Add\"")
        && !content.contains("\"duumbi:Call\"")
    {
        eprintln!(
            "{} This is an empty workspace. Try one of these:",
            theme::info("Tip:"),
        );
        eprintln!(
            "  {}  {}",
            theme::command("/intent create"),
            theme::dim("\"Build a calculator with add and multiply\""),
        );
        eprintln!(
            "  {}",
            theme::dim("  or type a request directly: \"Add a function that adds two numbers\""),
        );
        eprintln!();
    }

    let mut session = Session {
        workspace_root,
        config,
        client,
        history: Vec::new(),
        session_mgr,
    };

    let completer = Box::new(SlashCommandCompleter::new(session.workspace_root.clone()));
    let hinter = Box::new(SlashCommandHinter::new());
    let mut editor = Reedline::create()
        .with_completer(completer)
        .with_hinter(hinter);
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
                            // Archive session on exit
                            session
                                .session_mgr
                                .archive()
                                .map_err(|e| anyhow::anyhow!("session archive: {e}"))?;
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
                session
                    .session_mgr
                    .archive()
                    .map_err(|e| anyhow::anyhow!("session archive: {e}"))?;
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

    eprintln!(
        "{} {} · {} · workspace: {}",
        theme::bold("duumbi"),
        theme::dim(&format!("v{version}")),
        theme::info(model),
        theme::bold(&workspace_name),
    );
    eprintln!(
        "Type a request or {} for commands. {} to exit.",
        theme::command("/help"),
        theme::dim("Ctrl+D"),
    );
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

            "/knowledge" => {
                self.handle_knowledge_slash(arg);
            }

            "/resume" => {
                self.handle_resume_slash(arg);
            }

            "/clear" => {
                self.handle_clear(arg);
            }

            "/help" => print_help(),

            "/exit" | "/quit" => return Ok(true),

            _ => {
                // "Did you mean?" suggestion using Levenshtein distance
                let known_cmds: Vec<&str> = super::completion::SLASH_COMMANDS
                    .iter()
                    .filter(|(c, _)| !c.contains(' ')) // only top-level commands
                    .map(|(c, _)| *c)
                    .collect();
                if let Some(suggestion) = find_closest_command(cmd, &known_cmds) {
                    eprintln!(
                        "Unknown command: {}. Did you mean {}?",
                        theme::warning(cmd),
                        theme::command(suggestion),
                    );
                } else {
                    eprintln!("Unknown command: {}", theme::warning(cmd));
                }
                eprintln!("Try {} for available commands.", theme::command("/help"));
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
                "{} AI mutations are not available.",
                theme::warning("Warning:")
            );
            eprintln!();
            eprintln!("Add a provider to {}:", theme::bold(".duumbi/config.toml"));
            eprintln!("{}", theme::dim("  [[providers]]"));
            eprintln!("{}", theme::dim("  provider = \"anthropic\""));
            eprintln!("{}", theme::dim("  role = \"primary\""));
            eprintln!("{}", theme::dim("  model = \"claude-sonnet-4-6\""));
            eprintln!("{}", theme::dim("  api_key_env = \"ANTHROPIC_API_KEY\""));
            eprintln!();
            eprintln!(
                "Then set {} and restart the REPL.",
                theme::info("ANTHROPIC_API_KEY"),
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

        // Detect multi-module workspace to skip Call validation for cross-module refs
        let graph_dir = self.workspace_root.join(".duumbi/graph");
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
            let sp = progress::spinner(&format!("Thinking… (~{ctx_k:.1}k context)"));
            // Brief pause to show spinner, then clear before streaming
            std::thread::sleep(std::time::Duration::from_millis(100));
            sp.finish_and_clear();
        }

        // Run AI mutation with streaming text output
        let outcome = match orchestrator::mutate_streaming(
            client,
            &source,
            &prompt,
            3,
            is_multi_module,
            |text| {
                eprint!("{text}");
            },
        )
        .await
        {
            Ok(o) => o,
            Err(e) => {
                eprintln!();
                eprintln!("{}", theme::error(&format!("{e:#}")));
                return Ok(());
            }
        };
        eprintln!(); // newline after streamed text

        // Handle clarification requests
        let result = match outcome {
            orchestrator::MutationOutcome::Success(r) => r,
            orchestrator::MutationOutcome::NeedsClarification(question) => {
                eprintln!("\n? {question}");
                // The next user input will include this context
                self.history.push(Turn {
                    request: request.to_string(),
                    summary: format!("Clarification needed: {question}"),
                });
                return Ok(());
            }
        };

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
        let diff_clone = diff.clone();
        self.history.push(Turn {
            request: request.to_string(),
            summary: diff,
        });

        // Persist session state
        self.session_mgr.add_turn(request, &diff_clone, "Mutation");
        let _ = self.session_mgr.save();

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
    // /knowledge handler
    // -------------------------------------------------------------------------

    /// Handles `/knowledge [subcommand]` within the REPL.
    ///
    /// Supported forms:
    /// - `/knowledge` or `/knowledge stats` — show aggregated statistics
    /// - `/knowledge list` — list all knowledge nodes
    fn handle_knowledge_slash(&self, arg: &str) {
        use crate::knowledge::learning;
        use crate::knowledge::store::KnowledgeStore;

        let sub = arg.split_whitespace().next().unwrap_or("stats");

        match sub {
            "list" => match KnowledgeStore::new(&self.workspace_root) {
                Ok(store) => {
                    let nodes = store.load_all();
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
                }
                Err(e) => eprintln!("Knowledge store error: {e}"),
            },
            "" | "stats" => match KnowledgeStore::new(&self.workspace_root) {
                Ok(store) => {
                    let stats = store.stats();
                    let success_count = learning::success_count(&self.workspace_root);
                    eprintln!(
                        "Knowledge: {} success, {} decision, {} pattern ({} total)",
                        stats.successes,
                        stats.decisions,
                        stats.patterns,
                        stats.total()
                    );
                    eprintln!("Learning log: {success_count} entries");
                }
                Err(e) => eprintln!("Knowledge store error: {e}"),
            },
            _ => {
                eprintln!("Usage: /knowledge [list|stats]");
                eprintln!("  /knowledge list   — list all knowledge nodes");
                eprintln!("  /knowledge stats  — show aggregated statistics");
            }
        }
    }

    // -------------------------------------------------------------------------
    // /resume handler
    // -------------------------------------------------------------------------

    /// Handles `/resume [N]` within the REPL.
    ///
    /// - `/resume` — list archived sessions with index numbers
    /// - `/resume <N>` — load session N's turns into current history
    fn handle_resume_slash(&mut self, arg: &str) {
        let history_dir = self.workspace_root.join(".duumbi/session/history");

        // List archived sessions
        let mut sessions = list_archived_sessions(&history_dir);
        if sessions.is_empty() {
            eprintln!("No archived sessions found.");
            return;
        }

        // Sort by filename (timestamp-based, newest first)
        sessions.sort_by(|a, b| b.0.cmp(&a.0));

        let sub = arg.trim();
        if sub.is_empty() {
            // List mode
            eprintln!("Archived sessions:");
            for (i, (filename, turns, _)) in sessions.iter().enumerate() {
                let display_name = filename.trim_end_matches(".json").replace('_', " ");
                eprintln!("  [{}] {} ({} turn(s))", i + 1, display_name, turns);
            }
            eprintln!();
            eprintln!("Use /resume <N> to load a session's context.");
        } else {
            // Load mode
            let idx: usize = match sub.parse::<usize>() {
                Ok(n) if n >= 1 && n <= sessions.len() => n - 1,
                _ => {
                    eprintln!(
                        "Invalid session number. Use 1–{} (from /resume list).",
                        sessions.len()
                    );
                    return;
                }
            };

            let (filename, _turns, loaded_turns) = &sessions[idx];
            // Merge loaded turns into current history
            for turn in loaded_turns {
                self.history.push(Turn {
                    request: turn.request.clone(),
                    summary: turn.summary.clone(),
                });
            }
            let display_name = filename.trim_end_matches(".json").replace('_', " ");
            eprintln!(
                "Resumed session '{}' ({} turn(s) loaded into context).",
                display_name,
                loaded_turns.len()
            );
        }
    }

    // -------------------------------------------------------------------------
    // /status helper
    // -------------------------------------------------------------------------

    /// Handles `/clear [chat|session|all]` command.
    fn handle_clear(&mut self, arg: &str) {
        match arg.trim() {
            "" | "chat" => {
                self.history.clear();
                eprintln!("{} Chat history cleared.", theme::check_mark());
            }
            "session" => {
                self.history.clear();
                self.session_mgr
                    .archive()
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .unwrap_or_else(|e| eprintln!("Warning: {e}"));
                eprintln!("{} Session archived and cleared.", theme::check_mark());
            }
            "all" => {
                self.history.clear();
                self.session_mgr
                    .archive()
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .unwrap_or_else(|e| eprintln!("Warning: {e}"));
                eprintln!("{} History, session cleared.", theme::check_mark(),);
            }
            other => {
                eprintln!(
                    "Unknown clear target: {}. Use: {}, {}, or {}",
                    theme::warning(other),
                    theme::command("/clear chat"),
                    theme::command("/clear session"),
                    theme::command("/clear all"),
                );
            }
        }
    }

    /// Prints workspace status: graph path, binary, history depth, model.
    fn print_status(&self) {
        let graph_path = self.workspace_root.join(".duumbi/graph/main.jsonld");
        let output_path = self.workspace_root.join(".duumbi/build/output");
        let history_count = snapshot::snapshot_count(&self.workspace_root).unwrap_or(0);
        let session_turns = self.history.len();

        eprintln!(
            "{}",
            theme::bold(&format!("Workspace: {}", self.workspace_root.display()))
        );
        eprintln!(
            "  Graph:        {} {}",
            theme::dim(&graph_path.display().to_string()),
            if graph_path.exists() {
                theme::check_mark()
            } else {
                theme::cross_mark() + " missing"
            }
        );
        eprintln!(
            "  Binary:       {} {}",
            theme::dim(&output_path.display().to_string()),
            if output_path.exists() {
                theme::check_mark()
            } else {
                theme::dim("(not built)")
            }
        );
        eprintln!(
            "  Snapshots:    {history_count} {}",
            theme::dim("(undo depth)")
        );
        eprintln!("  Session turns: {session_turns}");
        if let Some(llm) = &self.config.llm {
            eprintln!(
                "  Model:        {} {}",
                theme::info(&llm.model),
                theme::dim(&format!("({})", llm.provider))
            );
        } else {
            eprintln!("  Model:        {}", theme::warning("not configured"));
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
// Session archive helpers
// ---------------------------------------------------------------------------

/// Lists archived session files, returning (filename, turn_count, turns) tuples.
fn list_archived_sessions(
    history_dir: &Path,
) -> Vec<(String, usize, Vec<crate::session::PersistentTurn>)> {
    let entries = match std::fs::read_dir(history_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(state) = serde_json::from_str::<crate::session::SessionState>(&content)
        {
            let turn_count = state.turns.len();
            results.push((filename, turn_count, state.turns));
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Help text
// ---------------------------------------------------------------------------

/// Finds the closest matching command using Levenshtein distance.
///
/// Returns `Some(cmd)` if the closest match is within 3 edits, `None` otherwise.
fn find_closest_command<'a>(input: &str, commands: &[&'a str]) -> Option<&'a str> {
    let mut best: Option<(&str, f64)> = None;
    for &cmd in commands {
        let dist = strsim::normalized_levenshtein(input, cmd);
        if let Some((_, best_dist)) = best {
            if dist > best_dist {
                best = Some((cmd, dist));
            }
        } else {
            best = Some((cmd, dist));
        }
    }
    // Only suggest if similarity > 0.5
    best.filter(|(_, d)| *d > 0.5).map(|(cmd, _)| cmd)
}

/// Prints the available slash commands to stderr.
fn print_help() {
    let c = theme::command;
    let d = theme::dim;
    eprintln!("{}", theme::bold("Slash commands:"));
    eprintln!(
        "  {}              {}",
        c("/build"),
        d("Compile the current graph to a native binary")
    );
    eprintln!(
        "  {} {}       {}",
        c("/run"),
        d("[args]"),
        d("Run the compiled binary")
    );
    eprintln!(
        "  {}              {}",
        c("/check"),
        d("Validate the graph without compiling")
    );
    eprintln!(
        "  {}           {}",
        c("/describe"),
        d("Print human-readable pseudocode of the graph")
    );
    eprintln!(
        "  {}               {}",
        c("/undo"),
        d("Restore the previous graph snapshot")
    );
    eprintln!(
        "  {}             {}",
        c("/status"),
        d("Show workspace, model, and session information")
    );
    eprintln!(
        "  {}            {}",
        c("/history"),
        d("Show session conversation history")
    );
    eprintln!(
        "  {}              {}",
        c("/model"),
        d("Show the current LLM model")
    );
    eprintln!(
        "  {} {}   {}",
        c("/clear"),
        d("[chat|session|all]"),
        d("Clear session state")
    );
    eprintln!();
    eprintln!("{}", theme::bold("Intent commands:"));
    eprintln!(
        "  {}             {}",
        c("/intent"),
        d("List all active intents")
    );
    eprintln!(
        "  {} {}   {}",
        c("/intent create"),
        d("<desc>"),
        d("Generate and save a new intent spec")
    );
    eprintln!(
        "  {} {}  {}",
        c("/intent review"),
        d("[name]"),
        d("Show intent details")
    );
    eprintln!(
        "  {} {} {}",
        c("/intent execute"),
        d("<name>"),
        d("Execute an intent end-to-end")
    );
    eprintln!(
        "  {} {}  {}",
        c("/intent status"),
        d("[name]"),
        d("Show intent execution status")
    );
    eprintln!();
    eprintln!("{}", theme::bold("Knowledge commands:"));
    eprintln!(
        "  {}          {}",
        c("/knowledge"),
        d("Show knowledge statistics")
    );
    eprintln!(
        "  {}     {}",
        c("/knowledge list"),
        d("List all knowledge nodes")
    );
    eprintln!();
    eprintln!("{}", theme::bold("Session commands:"));
    eprintln!(
        "  {}             {}",
        c("/resume"),
        d("List archived sessions")
    );
    eprintln!(
        "  {} {}        {}",
        c("/resume"),
        d("<N>"),
        d("Load session N's history into current context")
    );
    eprintln!();
    eprintln!("{}", theme::bold("Registry & dependency commands:"));
    eprintln!(
        "  {} {}   {}",
        c("/search"),
        d("<query>"),
        d("Search registries for modules")
    );
    eprintln!(
        "  {}            {}",
        c("/publish"),
        d("Package and publish the current module")
    );
    eprintln!(
        "  {}      {}",
        c("/registry list"),
        d("List configured registries")
    );
    eprintln!(
        "  {}          {}",
        c("/deps list"),
        d("List declared dependencies")
    );
    eprintln!(
        "  {}         {}",
        c("/deps audit"),
        d("Verify dependency integrity")
    );
    eprintln!(
        "  {}          {}",
        c("/deps tree"),
        d("Show the dependency tree")
    );
    eprintln!(
        "  {} {} {}",
        c("/deps update"),
        d("[name]"),
        d("Update dependencies")
    );
    eprintln!(
        "  {}        {}",
        c("/deps vendor"),
        d("Vendor cached dependencies")
    );
    eprintln!();
    eprintln!(
        "  {}               {}",
        c("/help"),
        d("Show this help text")
    );
    eprintln!("  {}               {}", c("/exit"), d("Exit the REPL"));
    eprintln!();
    eprintln!(
        "{}",
        theme::dim("Any other input is sent to the AI as a mutation request.")
    );
}
