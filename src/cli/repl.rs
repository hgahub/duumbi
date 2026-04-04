//! Interactive REPL for the duumbi CLI.
//!
//! Uses ratatui for full terminal UI with a status bar, inline slash menu,
//! and two-mode (Agent/Intent) interaction. Key handling and rendering are
//! delegated to [`super::app::ReplApp`]; this module owns the event loop and
//! the async command dispatch.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event};
use crossterm::execute;
use ratatui_textarea::TextArea;

use crate::agents::{LlmClient, orchestrator};
use crate::config::DuumbiConfig;
use crate::intent;
use crate::session::SessionManager;
use crate::snapshot;

use super::app::{ReplApp, Turn};
use super::commands;
use super::mode::{OutputStyle, ReplMode};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Runs the interactive REPL session until the user exits.
///
/// Initialises a ratatui terminal, creates [`ReplApp`] with all workspace
/// state, and drives the event loop. On exit the session is archived.
pub async fn run(workspace_root: PathBuf, config: DuumbiConfig) -> Result<()> {
    let client = build_client(&config);
    let has_workspace = workspace_root.join(".duumbi").exists();

    // Initialise persistent session if workspace exists.
    let session_mgr = if has_workspace {
        match SessionManager::load_or_create(&workspace_root) {
            Ok(mut mgr) => {
                if mgr.has_pending_session() {
                    mgr.archive().ok();
                }
                Some(mgr)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Tip detection: no workspace → show /init tip; empty workspace → show usage tip.
    let show_tip = if !has_workspace {
        true
    } else {
        let graph_path = workspace_root.join(".duumbi/graph/main.jsonld");
        if let Ok(content) = fs::read_to_string(&graph_path) {
            content.contains("\"duumbi:value\": 0")
                && !content.contains("\"duumbi:Add\"")
                && !content.contains("\"duumbi:Call\"")
        } else {
            false
        }
    };

    let mut app = ReplApp::new(
        config,
        workspace_root,
        client,
        session_mgr,
        has_workspace,
        show_tip,
    );

    // Initialise ratatui (enters alternate screen, enables raw mode).
    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableBracketedPaste)?;

    // Single-line textarea — we intercept Enter ourselves.
    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(ratatui::style::Style::default());

    let result = event_loop(&mut terminal, &mut app, &mut textarea).await;

    // Always restore the terminal, even on error.
    execute!(std::io::stdout(), DisableBracketedPaste).ok();
    ratatui::restore();

    result
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

/// Drives the ratatui event loop until the user exits or an I/O error occurs.
async fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| app.render(frame, textarea))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => match app.handle_key(key, textarea) {
                    super::mode::Action::Continue => {}
                    super::mode::Action::Exit => {
                        if let Some(ref mut mgr) = app.session_mgr {
                            mgr.archive().ok();
                        }
                        app.push_output("Goodbye!", OutputStyle::Dim);
                        // Draw one last frame so the farewell message is visible.
                        terminal.draw(|frame| app.render(frame, textarea))?;
                        break;
                    }
                    super::mode::Action::Submit(input) => {
                        app.working = true;
                        terminal.draw(|frame| app.render(frame, textarea))?;

                        if process_input(terminal, app, textarea, &input).await {
                            if let Some(ref mut mgr) = app.session_mgr {
                                mgr.archive().ok();
                            }
                            break;
                        }
                        app.working = false;
                    }
                },
                Event::Paste(text) => {
                    textarea.insert_str(&text);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Input dispatch
// ---------------------------------------------------------------------------

/// Dispatches a submitted line to the appropriate handler.
///
/// Returns `true` when the REPL should exit (e.g. `/exit`).
async fn process_input(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
    input: &str,
) -> bool {
    if input.starts_with('/') {
        handle_slash(terminal, app, textarea, input).await
    } else {
        // Visual separator before user-submitted input output.
        if !app.output_lines.is_empty() {
            app.push_output("", OutputStyle::Normal);
        }
        app.push_output(format!("\u{25cf} {input}"), OutputStyle::Dim);

        match app.mode {
            ReplMode::Agent => {
                handle_ai_request(terminal, app, textarea, input).await;
                false
            }
            ReplMode::Intent => {
                handle_intent_input(app, input).await;
                false
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Slash command dispatcher
// ---------------------------------------------------------------------------

/// Dispatches a `/command [args]` line.
///
/// Returns `true` if the REPL should exit (`/exit`, `/quit`).
async fn handle_slash(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
    input: &str,
) -> bool {
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg = parts.next().unwrap_or("").trim();

    let graph_path = app.workspace_root.join(".duumbi/graph/main.jsonld");
    let output_path = app.workspace_root.join(".duumbi/build/output");

    // Visual separator: empty line + bullet header before each command's output.
    if !matches!(cmd, "/exit" | "/quit") {
        if !app.output_lines.is_empty() {
            app.push_output("", OutputStyle::Normal);
        }
        app.push_output(format!("\u{25cf} {input}"), OutputStyle::Normal);
    }

    match cmd {
        "/build" => {
            run_with_terminal_restore(terminal, app, textarea, || {
                commands::build(&graph_path, &output_path).unwrap_or_else(|e| {
                    eprintln!("Build failed: {e:#}");
                });
            });
        }

        "/run" => {
            if !output_path.exists() {
                app.push_output("No binary found. Run /build first.", OutputStyle::Error);
            } else {
                run_with_terminal_restore(terminal, app, textarea, || match process::Command::new(
                    &output_path,
                )
                .args(arg.split_whitespace())
                .status()
                {
                    Ok(status) if !status.success() => {
                        eprintln!("Process exited with {status}");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to execute '{}': {e}", output_path.display());
                    }
                });
            }
        }

        "/check" => {
            run_with_terminal_restore(terminal, app, textarea, || {
                commands::check(&graph_path).unwrap_or_else(|e| {
                    eprintln!("Check failed: {e:#}");
                });
            });
        }

        "/describe" => {
            run_with_terminal_restore(terminal, app, textarea, || {
                commands::describe(&graph_path).unwrap_or_else(|e| {
                    eprintln!("Describe failed: {e:#}");
                });
            });
        }

        "/undo" => match snapshot::restore_latest(&app.workspace_root) {
            Ok(true) => {
                let remaining = snapshot::snapshot_count(&app.workspace_root).unwrap_or(0);
                app.history.pop();
                app.push_output(
                    format!("Undo successful. {remaining} snapshot(s) remaining."),
                    OutputStyle::Success,
                );
            }
            Ok(false) => {
                app.push_output("Nothing to undo.", OutputStyle::Dim);
            }
            Err(e) => {
                app.push_output(format!("Undo failed: {e:#}"), OutputStyle::Error);
            }
        },

        "/status" => {
            print_status_to_buffer(app);
        }

        "/model" => {
            // Open the interactive model selector panel with the primary provider selected.
            let primary_idx = app
                .config
                .effective_providers()
                .iter()
                .position(|p| p.role == crate::config::ProviderRole::Primary)
                .unwrap_or(0);
            app.panel = super::mode::PanelState::ModelSelector {
                selected: primary_idx,
                input_mode: None,
                status_msg: None,
            };
            textarea.move_cursor(ratatui_textarea::CursorMove::Head);
            textarea.delete_line_by_end();
        }

        "/history" => {
            if app.history.is_empty() {
                app.push_output("No session history yet.", OutputStyle::Dim);
            } else {
                app.push_output(
                    format!(
                        "Session history ({} turn{}):",
                        app.history.len(),
                        if app.history.len() == 1 { "" } else { "s" }
                    ),
                    OutputStyle::Normal,
                );
                // Collect to avoid borrow conflict.
                let lines: Vec<String> = app
                    .history
                    .iter()
                    .enumerate()
                    .flat_map(|(i, turn)| {
                        vec![
                            format!("  {}. \"{}\"", i + 1, turn.request),
                            format!("     {}", turn.summary),
                        ]
                    })
                    .collect();
                for line in lines {
                    app.push_output(line, OutputStyle::Normal);
                }
            }
        }

        "/intent" => {
            handle_intent_slash(app, arg).await;
        }

        "/search" => {
            if arg.is_empty() {
                app.push_output("Usage: /search <query>", OutputStyle::Dim);
            } else {
                let workspace = app.workspace_root.clone();
                match super::deps::run_search(&workspace, arg, None).await {
                    Ok(()) => {}
                    Err(e) => {
                        app.push_output(format!("Search failed: {e:#}"), OutputStyle::Error);
                    }
                }
            }
        }

        "/deps" => {
            handle_deps_slash(app, arg).await;
        }

        "/publish" => {
            let workspace = app.workspace_root.clone();
            match super::publish::run_publish(&workspace, None, false, false).await {
                Ok(()) => {}
                Err(e) => {
                    app.push_output(format!("Publish failed: {e:#}"), OutputStyle::Error);
                }
            }
        }

        "/registry" => {
            handle_registry_slash(app, arg);
        }

        "/knowledge" => {
            handle_knowledge_slash(app, arg);
        }

        "/resume" => {
            handle_resume_slash(app, arg);
        }

        "/clear" => {
            handle_clear(app, arg);
        }

        "/init" => {
            if app.has_workspace {
                app.push_output("Workspace already initialised.", OutputStyle::Dim);
            } else {
                run_with_terminal_restore(terminal, app, textarea, || {
                    // run_init writes messages to stderr — visible outside alternate screen.
                });
                ratatui::restore();
                let init_result = super::init::run_init(&app.workspace_root);
                *terminal = ratatui::init();
                let _ = terminal.draw(|frame| app.render(frame, textarea));
                match init_result {
                    Ok(()) => {
                        app.has_workspace = true;
                        app.config =
                            crate::config::load_config(&app.workspace_root).unwrap_or_default();
                        app.client = build_client(&app.config);
                        app.session_mgr = SessionManager::load_or_create(&app.workspace_root).ok();
                        app.show_tip = true;
                        app.push_output("Workspace initialised.", OutputStyle::Success);
                    }
                    Err(e) => {
                        app.push_output(format!("Init failed: {e:#}"), OutputStyle::Error);
                    }
                }
            }
        }

        "/provider" => {
            app.push_output("The /provider command has been removed.", OutputStyle::Dim);
            app.push_output(
                "Use /model for interactive provider management, or `duumbi provider` from the CLI.",
                OutputStyle::Dim,
            );
        }

        "/help" => {
            print_help_to_buffer(app);
        }

        "/exit" | "/quit" => return true,

        _ => {
            // "Did you mean?" suggestion using Levenshtein distance.
            let known_cmds: Vec<&str> = super::completion::SLASH_COMMANDS
                .iter()
                .filter(|(c, _)| !c.contains(' '))
                .map(|(c, _)| *c)
                .collect();
            if let Some(suggestion) = find_closest_command(cmd, &known_cmds) {
                app.push_output(
                    format!("Unknown command: {cmd}. Did you mean {suggestion}?"),
                    OutputStyle::Error,
                );
            } else {
                app.push_output(format!("Unknown command: {cmd}"), OutputStyle::Error);
            }
            app.push_output("Try /help for available commands.", OutputStyle::Dim);
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Terminal-restore wrapper
// ---------------------------------------------------------------------------

/// Temporarily restores the terminal, runs `f` (which may print to stderr),
/// then re-initialises ratatui.
///
/// This is necessary for commands like `/build`, `/run`, `/check`, and
/// `/describe` that write diagnostics directly to stderr — output that would
/// be hidden inside the alternate screen.
fn run_with_terminal_restore<F>(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
    f: F,
) where
    F: FnOnce(),
{
    // Leave alternate screen so the command's stderr output is visible.
    ratatui::restore();

    f();

    // Prompt the user to return to the TUI.
    eprintln!("\n[Press Enter to return to the REPL]");
    let _ = std::io::stdin().read_line(&mut String::new());

    // Re-enter alternate screen.
    *terminal = ratatui::init();
    // Redraw immediately so the TUI is not blank.
    let _ = terminal.draw(|frame| app.render(frame, textarea));
}

// ---------------------------------------------------------------------------
// AI mutation handler
// ---------------------------------------------------------------------------

/// Handles a natural language AI mutation request in Agent mode.
///
/// Prepends session history for context, calls the LLM via
/// [`orchestrator::mutate_streaming`], applies the patch, and auto-builds.
async fn handle_ai_request(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
    request: &str,
) {
    if app.client.is_none() {
        app.push_output(
            "AI mutations are not available. Add a provider to .duumbi/config.toml:",
            OutputStyle::Error,
        );
        app.push_output("  [[providers]]", OutputStyle::Dim);
        app.push_output("  provider = \"anthropic\"", OutputStyle::Dim);
        app.push_output("  role = \"primary\"", OutputStyle::Dim);
        app.push_output("  model = \"claude-sonnet-4-6\"", OutputStyle::Dim);
        app.push_output("  api_key_env = \"ANTHROPIC_API_KEY\"", OutputStyle::Dim);
        app.push_output(
            "Then set ANTHROPIC_API_KEY and restart the REPL.",
            OutputStyle::Dim,
        );
        return;
    }

    let graph_path = app.workspace_root.join(".duumbi/graph/main.jsonld");

    // Read the current graph.
    let source_str = match fs::read_to_string(&graph_path) {
        Ok(s) => s,
        Err(e) => {
            app.push_output(format!("Failed to read graph: {e:#}"), OutputStyle::Error);
            return;
        }
    };
    let source: serde_json::Value = match serde_json::from_str(&source_str) {
        Ok(v) => v,
        Err(e) => {
            app.push_output(
                format!("Failed to parse graph JSON: {e:#}"),
                OutputStyle::Error,
            );
            return;
        }
    };

    // Estimate context size.
    let ctx_chars: usize = source_str.len()
        + app
            .history
            .iter()
            .map(|t| t.request.len() + t.summary.len())
            .sum::<usize>();
    let ctx_k = ctx_chars as f64 / 4000.0;

    app.push_output(
        format!("Thinking… (~{ctx_k:.1}k context)"),
        OutputStyle::Dim,
    );
    // Draw so the "Thinking…" message is visible before the await.
    let _ = terminal.draw(|frame| app.render(frame, textarea));

    // Build prompt with conversation history.
    let prompt = build_prompt_with_history(request, &app.history);

    // Detect multi-module workspace.
    let graph_dir = app.workspace_root.join(".duumbi/graph");
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

    // Collect streamed text into a local String. We cannot update the TUI
    // mid-stream, so we accumulate and push the result after completion.
    // The client reference is scoped to this block so the `&mut app` borrow
    // used afterwards does not overlap.
    let workspace = app.workspace_root.clone();
    let (outcome, streamed) = {
        let client_ref: &dyn crate::agents::LlmProvider = app
            .client
            .as_ref()
            .map(|c| c.as_ref())
            .expect("invariant: client is_some checked above");
        let buf = std::sync::Mutex::new(String::new());
        let res = orchestrator::mutate_streaming(
            client_ref,
            &source,
            &prompt,
            3,
            is_multi_module,
            |text| {
                buf.lock()
                    .expect("invariant: mutex not poisoned")
                    .push_str(text);
            },
        )
        .await;
        let streamed = buf.into_inner().expect("invariant: mutex not poisoned");
        (res, streamed)
    };

    // Push streamed AI text to the output buffer.
    if !streamed.trim().is_empty() {
        app.push_output(streamed.trim().to_string(), OutputStyle::Ai);
    }

    let result = match outcome {
        Ok(orchestrator::MutationOutcome::Success(r)) => r,
        Ok(orchestrator::MutationOutcome::NeedsClarification(question)) => {
            app.push_output(format!("? {question}"), OutputStyle::Ai);
            app.history.push(Turn {
                request: request.to_string(),
                summary: format!("Clarification needed: {question}"),
            });
            return;
        }
        Err(e) => {
            app.push_output(format!("Mutation error: {e:#}"), OutputStyle::Error);
            return;
        }
    };

    // Show diff summary.
    let diff = orchestrator::describe_changes(&source, &result.patched);
    app.push_output(
        format!(
            "{} tool call{} applied:\n{}",
            result.ops_count,
            if result.ops_count == 1 { "" } else { "s" },
            diff
        ),
        OutputStyle::Normal,
    );

    // Save snapshot and write the updated graph.
    if let Err(e) = snapshot::save_snapshot(&workspace, &source_str) {
        app.push_output(
            format!("Warning: snapshot save failed: {e:#}"),
            OutputStyle::Error,
        );
    }
    let patched_str = match serde_json::to_string_pretty(&result.patched) {
        Ok(s) => s,
        Err(e) => {
            app.push_output(format!("Serialisation error: {e:#}"), OutputStyle::Error);
            return;
        }
    };
    if let Err(e) = fs::write(&graph_path, &patched_str) {
        app.push_output(format!("Write error: {e:#}"), OutputStyle::Error);
        return;
    }

    // Auto-build after mutation.
    app.push_output("Building…", OutputStyle::Dim);
    let _ = terminal.draw(|frame| app.render(frame, textarea));

    let output_path = workspace.join(".duumbi/build/output");
    match commands::build(&graph_path, &output_path) {
        Ok(()) => {
            app.push_output(
                format!("Build successful: {}", output_path.display()),
                OutputStyle::Success,
            );
        }
        Err(e) => {
            app.push_output(format!("Build failed: {e:#}"), OutputStyle::Error);
            app.push_output(
                "(Graph saved. Use /undo to revert or describe the fix in your next request.)",
                OutputStyle::Dim,
            );
        }
    }

    // Record turn in session history.
    let diff_clone = diff.clone();
    app.history.push(Turn {
        request: request.to_string(),
        summary: diff,
    });

    if let Some(ref mut mgr) = app.session_mgr {
        mgr.add_turn(request, &diff_clone, "Mutation");
        let _ = mgr.save();
    }
}

// ---------------------------------------------------------------------------
// Intent mode input
// ---------------------------------------------------------------------------

/// Handles free-form text input when in Intent mode.
///
/// - If no intent is focused, the input is treated as an intent description
///   and forwarded to [`intent::create::run_create`].
/// - If the input is "execute" or "run", delegates to intent execute.
/// - Otherwise, modifies the focused intent via LLM based on the input.
async fn handle_intent_input(app: &mut ReplApp, input: &str) {
    let trimmed = input.trim();
    if app.focused_intent.is_none() {
        // Treat as intent create.
        if app.client.is_none() {
            app.push_output(
                "AI not available — add [llm] section to .duumbi/config.toml.",
                OutputStyle::Error,
            );
            return;
        }
        let workspace = app.workspace_root.clone();
        let (result, log) = {
            let client_ref: &dyn crate::agents::LlmProvider = app
                .client
                .as_ref()
                .map(|c| c.as_ref())
                .expect("invariant: checked above");
            let mut log = Vec::new();
            let r =
                // REPL always auto-confirms — interactive stdin is not available
                // in ratatui raw mode.
                intent::create::run_create(client_ref, &workspace, trimmed, true, &mut log).await;
            (r, log)
        };
        for line in &log {
            app.push_output(line, OutputStyle::Dim);
        }
        match result {
            Ok(slug) => {
                app.focused_intent = Some(slug.clone());
            }
            Err(e) => {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        }
        return;
    }

    match trimmed {
        "execute" | "run" => {
            let slug = app
                .focused_intent
                .clone()
                .expect("invariant: focused_intent checked above");
            handle_intent_execute(app, &slug).await;
        }
        _ => {
            // Modify the focused intent via LLM.
            if app.client.is_none() {
                app.push_output(
                    "AI not available — add [[providers]] to .duumbi/config.toml.",
                    OutputStyle::Error,
                );
                return;
            }
            let slug = app
                .focused_intent
                .clone()
                .expect("invariant: focused_intent checked above");
            let workspace = app.workspace_root.clone();

            let spec = match intent::load_intent(&workspace, &slug) {
                Ok(s) => s,
                Err(e) => {
                    app.push_output(format!("Failed to load intent: {e}"), OutputStyle::Error);
                    return;
                }
            };

            app.push_output(format!("Modifying intent '{slug}'…"), OutputStyle::Dim);

            let result = {
                let client_ref: &dyn crate::agents::LlmProvider = app
                    .client
                    .as_ref()
                    .map(|c| c.as_ref())
                    .expect("invariant: checked above");
                intent::modify::modify_intent_with_llm(client_ref, &spec, trimmed).await
            };

            match result {
                Ok(modified) => {
                    // Show the modified spec.
                    let yaml = serde_yaml::to_string(&modified).unwrap_or_default();
                    app.push_output(yaml, OutputStyle::Ai);

                    // Save (auto-save in intent mode for now).
                    match intent::save_intent(&workspace, &slug, &modified) {
                        Ok(()) => {
                            app.push_output(
                                format!("Intent '{slug}' updated."),
                                OutputStyle::Success,
                            );
                        }
                        Err(e) => {
                            app.push_output(format!("Failed to save: {e}"), OutputStyle::Error);
                        }
                    }
                }
                Err(e) => {
                    app.push_output(format!("Modification failed: {e:#}"), OutputStyle::Error);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// /intent slash handler
// ---------------------------------------------------------------------------

/// Handles `/intent <subcommand> [args]` within the REPL.
///
/// Supported subcommands: `list`, `create`, `review`, `execute`, `status`,
/// `focus`, `unfocus`.
async fn handle_intent_slash(app: &mut ReplApp, arg: &str) {
    let mut parts = arg.splitn(2, ' ');
    let subcmd = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();

    match subcmd {
        "" | "list" => {
            let workspace = app.workspace_root.clone();
            match collect_intent_list(&workspace) {
                Ok(lines) => {
                    for line in lines {
                        app.push_output(line, OutputStyle::Normal);
                    }
                }
                Err(e) => {
                    app.push_output(format!("Error: {e}"), OutputStyle::Error);
                }
            }
        }

        "create" => {
            if rest.is_empty() {
                app.push_output("Usage: /intent create <description>", OutputStyle::Dim);
                return;
            }
            if app.client.is_none() {
                app.push_output(
                    "AI not available — add [llm] section to .duumbi/config.toml.",
                    OutputStyle::Error,
                );
                return;
            }
            let workspace = app.workspace_root.clone();
            let (result, log) = {
                let client_ref: &dyn crate::agents::LlmProvider = app
                    .client
                    .as_ref()
                    .map(|c| c.as_ref())
                    .expect("invariant: checked above");
                let mut log = Vec::new();
                let r =
                    // REPL always auto-confirms (no interactive stdin in ratatui).
                    intent::create::run_create(client_ref, &workspace, rest, true, &mut log).await;
                (r, log)
            };
            for line in &log {
                app.push_output(line, OutputStyle::Dim);
            }
            if let Err(e) = result {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        }

        "review" => {
            let workspace = app.workspace_root.clone();
            if rest.is_empty() {
                match collect_intent_list(&workspace) {
                    Ok(lines) => {
                        for line in lines {
                            app.push_output(line, OutputStyle::Normal);
                        }
                    }
                    Err(e) => {
                        app.push_output(format!("Error: {e}"), OutputStyle::Error);
                    }
                }
            } else {
                match collect_intent_detail(&workspace, rest) {
                    Ok(lines) => {
                        for line in lines {
                            app.push_output(line, OutputStyle::Normal);
                        }
                    }
                    Err(e) => {
                        app.push_output(format!("Error: {e}"), OutputStyle::Error);
                    }
                }
            }
        }

        "execute" => {
            if rest.is_empty() {
                app.push_output("Usage: /intent execute <name>", OutputStyle::Dim);
                return;
            }
            handle_intent_execute(app, rest).await;
        }

        "status" => {
            let workspace = app.workspace_root.clone();
            if rest.is_empty() {
                match collect_intent_status_list(&workspace) {
                    Ok(lines) => {
                        for line in lines {
                            app.push_output(line, OutputStyle::Normal);
                        }
                    }
                    Err(e) => {
                        app.push_output(format!("Error: {e}"), OutputStyle::Error);
                    }
                }
            } else {
                match collect_intent_status_detail(&workspace, rest) {
                    Ok(lines) => {
                        for line in lines {
                            app.push_output(line, OutputStyle::Normal);
                        }
                    }
                    Err(e) => {
                        app.push_output(format!("Error: {e}"), OutputStyle::Error);
                    }
                }
            }
        }

        "focus" => {
            if rest.is_empty() {
                app.push_output("Usage: /intent focus <slug>", OutputStyle::Dim);
            } else {
                app.focused_intent = Some(rest.to_string());
                app.push_output(format!("Focused intent: {rest}"), OutputStyle::Success);
            }
        }

        "unfocus" => {
            app.focused_intent = None;
            app.push_output("Intent focus cleared.", OutputStyle::Dim);
        }

        _ => {
            app.push_output(
                format!("Unknown intent subcommand: {subcmd}"),
                OutputStyle::Error,
            );
            app.push_output(
                "Available: list, create <desc>, review [name], execute <name>, \
                 status [name], focus <slug>, unfocus",
                OutputStyle::Dim,
            );
        }
    }
}

/// Executes an intent by slug and pushes the result into the output buffer.
async fn handle_intent_execute(app: &mut ReplApp, slug: &str) {
    if app.client.is_none() {
        app.push_output(
            "AI not available — add [llm] section to .duumbi/config.toml.",
            OutputStyle::Error,
        );
        return;
    }
    app.push_output(format!("Executing intent '{slug}'…"), OutputStyle::Dim);

    let workspace = app.workspace_root.clone();
    let (result, log) = {
        let client_ref: &dyn crate::agents::LlmProvider = app
            .client
            .as_ref()
            .map(|c| c.as_ref())
            .expect("invariant: checked above");
        let mut log = Vec::new();
        let r = intent::execute::run_execute(client_ref, &workspace, slug, &mut log).await;
        (r, log)
    };
    for line in &log {
        app.push_output(line, OutputStyle::Dim);
    }

    match result {
        Ok(true) => {
            app.push_output(
                format!("Intent '{slug}' completed successfully."),
                OutputStyle::Success,
            );
        }
        Ok(false) => {
            app.push_output(format!("Intent '{slug}' failed."), OutputStyle::Error);
        }
        Err(e) => {
            app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
        }
    }
}

// ---------------------------------------------------------------------------
// Intent output helpers (capture-to-string wrappers)
// ---------------------------------------------------------------------------

/// Captures [`intent::review::print_intent_list`] output as lines.
fn collect_intent_list(workspace: &Path) -> Result<Vec<String>> {
    // The underlying function prints to stderr; we can't easily redirect it.
    // Instead, re-implement a lightweight version that returns strings.
    let intents_dir = workspace.join(".duumbi/intents");
    let mut lines = Vec::new();
    if !intents_dir.exists() {
        lines.push("No active intents.".to_string());
        return Ok(lines);
    }
    let entries: Vec<_> = std::fs::read_dir(&intents_dir)
        .context("read intents dir")?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        })
        .collect();
    if entries.is_empty() {
        lines.push("No active intents.".to_string());
    } else {
        lines.push(format!("Active intents ({}):", entries.len()));
        for entry in &entries {
            let name = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            lines.push(format!("  {name}"));
        }
    }
    Ok(lines)
}

/// Captures [`intent::review::print_intent_detail`] output as lines.
fn collect_intent_detail(workspace: &Path, name: &str) -> Result<Vec<String>> {
    let path = workspace.join(format!(".duumbi/intents/{name}.yaml"));
    if !path.exists() {
        anyhow::bail!("Intent '{name}' not found.");
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading intent '{name}'"))?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

/// Captures intent status list as lines.
fn collect_intent_status_list(workspace: &Path) -> Result<Vec<String>> {
    // Delegate to the list for now — status details live in the YAML.
    collect_intent_list(workspace)
}

/// Captures intent status detail as lines.
fn collect_intent_status_detail(workspace: &Path, name: &str) -> Result<Vec<String>> {
    collect_intent_detail(workspace, name)
}

// ---------------------------------------------------------------------------
// /deps slash handler
// ---------------------------------------------------------------------------

/// Handles `/deps <subcommand>` within the REPL.
async fn handle_deps_slash(app: &mut ReplApp, arg: &str) {
    let mut parts = arg.splitn(2, ' ');
    let subcmd = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();

    let workspace = app.workspace_root.clone();

    match subcmd {
        "" | "list" => match super::deps::run_deps_list(&workspace) {
            Ok(()) => {}
            Err(e) => {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        },
        "audit" => match super::deps::run_deps_audit(&workspace) {
            Ok(()) => {}
            Err(e) => {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        },
        "tree" => match super::deps::run_deps_tree(&workspace, 10) {
            Ok(()) => {}
            Err(e) => {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        },
        "update" => {
            let name = if rest.is_empty() { None } else { Some(rest) };
            match super::deps::run_deps_update(&workspace, name).await {
                Ok(()) => {}
                Err(e) => {
                    app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
                }
            }
        }
        "vendor" => match super::deps::run_deps_vendor(&workspace, false, None) {
            Ok(()) => {}
            Err(e) => {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        },
        "install" => {
            let frozen = rest == "--frozen";
            match super::deps::run_deps_install(&workspace, frozen).await {
                Ok(()) => {}
                Err(e) => {
                    app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
                }
            }
        }
        "add" => {
            if rest.is_empty() {
                app.push_output(
                    "Usage: /deps add <name> [path] [--registry <name>]",
                    OutputStyle::Dim,
                );
            } else {
                // Parse: <name> [path] [--registry <reg>]
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                let name = tokens[0];
                let mut path: Option<&str> = None;
                let mut registry: Option<&str> = None;
                let mut i = 1;
                while i < tokens.len() {
                    if tokens[i] == "--registry" {
                        i += 1;
                        if i < tokens.len() {
                            registry = Some(tokens[i]);
                        }
                    } else if path.is_none() {
                        path = Some(tokens[i]);
                    }
                    i += 1;
                }
                match super::deps::run_deps_add(&workspace, name, path, registry).await {
                    Ok(()) => {}
                    Err(e) => {
                        app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
                    }
                }
            }
        }
        "remove" => {
            if rest.is_empty() {
                app.push_output("Usage: /deps remove <name>", OutputStyle::Dim);
            } else {
                match super::deps::run_deps_remove(&workspace, rest) {
                    Ok(()) => {}
                    Err(e) => {
                        app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
                    }
                }
            }
        }
        _ => {
            app.push_output(
                format!("Unknown deps subcommand: {subcmd}"),
                OutputStyle::Error,
            );
            app.push_output(
                "Available: list, add, remove, audit, tree, update, vendor, install",
                OutputStyle::Dim,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// /registry slash handler
// ---------------------------------------------------------------------------

/// Handles `/registry <subcommand>` within the REPL.
fn handle_registry_slash(app: &mut ReplApp, arg: &str) {
    let subcmd = arg.split(' ').next().unwrap_or("").trim();
    let workspace = app.workspace_root.clone();

    match subcmd {
        "" | "list" => match super::registry::run_registry_list(&workspace) {
            Ok(()) => {}
            Err(e) => {
                app.push_output(format!("Error: {e:#}"), OutputStyle::Error);
            }
        },
        _ => {
            app.push_output(
                format!("Unknown registry subcommand: {subcmd}"),
                OutputStyle::Error,
            );
            app.push_output(
                "Available: list. For other registry operations, use the CLI directly.",
                OutputStyle::Dim,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// /knowledge slash handler
// ---------------------------------------------------------------------------

/// Handles `/knowledge [subcommand]` within the REPL.
///
/// Subcommands: `stats` (default), `list`, `show <id>`, `prune [days]`.
fn handle_knowledge_slash(app: &mut ReplApp, arg: &str) {
    use crate::knowledge::learning;
    use crate::knowledge::store::KnowledgeStore;
    use crate::knowledge::types::KnowledgeNode;

    let mut parts = arg.splitn(2, ' ');
    let sub = parts.next().unwrap_or("stats");
    let rest = parts.next().unwrap_or("").trim();
    let workspace = app.workspace_root.clone();

    match sub {
        "list" => match KnowledgeStore::new(&workspace) {
            Ok(store) => {
                let nodes = store.load_all();
                if nodes.is_empty() {
                    app.push_output("No knowledge nodes found.", OutputStyle::Dim);
                } else {
                    app.push_output(
                        format!("Knowledge nodes ({}):", nodes.len()),
                        OutputStyle::Normal,
                    );
                    for node in &nodes {
                        app.push_output(
                            format!("  [{:}] {}", node.node_type(), node.id()),
                            OutputStyle::Normal,
                        );
                    }
                }
            }
            Err(e) => {
                app.push_output(format!("Knowledge store error: {e}"), OutputStyle::Error);
            }
        },

        "show" => {
            if rest.is_empty() {
                app.push_output("Usage: /knowledge show <id>", OutputStyle::Dim);
            } else {
                match KnowledgeStore::new(&workspace) {
                    Ok(store) => {
                        let all = store.load_all();
                        if let Some(node) = all.iter().find(|n| n.id() == rest) {
                            match serde_json::to_string_pretty(node) {
                                Ok(json) => {
                                    for line in json.lines() {
                                        app.push_output(line, OutputStyle::Normal);
                                    }
                                }
                                Err(e) => {
                                    app.push_output(
                                        format!("Serialize error: {e}"),
                                        OutputStyle::Error,
                                    );
                                }
                            }
                        } else {
                            app.push_output(format!("Node not found: {rest}"), OutputStyle::Error);
                        }
                    }
                    Err(e) => {
                        app.push_output(format!("Knowledge store error: {e}"), OutputStyle::Error);
                    }
                }
            }
        }

        "prune" => {
            let days: u32 = rest.parse().unwrap_or(90);
            match KnowledgeStore::new(&workspace) {
                Ok(store) => {
                    let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(days));
                    let all = store.load_all();
                    let mut removed = 0u32;
                    for node in &all {
                        let ts = match node {
                            KnowledgeNode::Success(r) => r.timestamp,
                            KnowledgeNode::Decision(r) => r.timestamp,
                            KnowledgeNode::Pattern(r) => r.timestamp,
                        };
                        if ts < cutoff && store.remove_node(node.id()).unwrap_or(false) {
                            removed += 1;
                        }
                    }
                    app.push_output(
                        format!("Pruned {removed} node(s) older than {days} days."),
                        OutputStyle::Success,
                    );
                }
                Err(e) => {
                    app.push_output(format!("Knowledge store error: {e}"), OutputStyle::Error);
                }
            }
        }

        "" | "stats" => match KnowledgeStore::new(&workspace) {
            Ok(store) => {
                let stats = store.stats();
                let success_count = learning::success_count(&workspace);
                app.push_output(
                    format!(
                        "Knowledge: {} success, {} decision, {} pattern ({} total)",
                        stats.successes,
                        stats.decisions,
                        stats.patterns,
                        stats.total()
                    ),
                    OutputStyle::Normal,
                );
                app.push_output(
                    format!("Learning log: {success_count} entries"),
                    OutputStyle::Dim,
                );
            }
            Err(e) => {
                app.push_output(format!("Knowledge store error: {e}"), OutputStyle::Error);
            }
        },

        _ => {
            app.push_output(
                "Usage: /knowledge [list|stats|show <id>|prune [days]]",
                OutputStyle::Dim,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// /resume slash handler
// ---------------------------------------------------------------------------

/// Handles `/resume [N]` within the REPL.
///
/// - `/resume` — list archived sessions with index numbers.
/// - `/resume <N>` — load session N's turns into current history.
fn handle_resume_slash(app: &mut ReplApp, arg: &str) {
    let history_dir = app.workspace_root.join(".duumbi/session/history");
    let mut sessions = list_archived_sessions(&history_dir);

    if sessions.is_empty() {
        app.push_output("No archived sessions found.", OutputStyle::Dim);
        return;
    }

    // Newest first.
    sessions.sort_by(|a, b| b.0.cmp(&a.0));

    let sub = arg.trim();
    if sub.is_empty() {
        app.push_output("Archived sessions:", OutputStyle::Normal);
        for (i, (filename, turns, _)) in sessions.iter().enumerate() {
            let display_name = filename.trim_end_matches(".json").replace('_', " ");
            app.push_output(
                format!("  [{}] {} ({} turn(s))", i + 1, display_name, turns),
                OutputStyle::Normal,
            );
        }
        app.push_output(
            "Use /resume <N> to load a session's context.",
            OutputStyle::Dim,
        );
    } else {
        let idx: usize = match sub.parse::<usize>() {
            Ok(n) if n >= 1 && n <= sessions.len() => n - 1,
            _ => {
                app.push_output(
                    format!(
                        "Invalid session number. Use 1–{} (from /resume).",
                        sessions.len()
                    ),
                    OutputStyle::Error,
                );
                return;
            }
        };

        let (filename, _turns, loaded_turns) = &sessions[idx];
        let count = loaded_turns.len();
        for turn in loaded_turns {
            app.history.push(Turn {
                request: turn.request.clone(),
                summary: turn.summary.clone(),
            });
        }
        let display_name = filename.trim_end_matches(".json").replace('_', " ");
        app.push_output(
            format!("Resumed session '{display_name}' ({count} turn(s) loaded into context)."),
            OutputStyle::Success,
        );
    }
}

// ---------------------------------------------------------------------------
// /clear handler
// ---------------------------------------------------------------------------

/// Handles `/clear [chat|session|all]`.
fn handle_clear(app: &mut ReplApp, arg: &str) {
    match arg.trim() {
        "" | "chat" => {
            app.history.clear();
            app.output_lines.clear();
            app.push_output("Chat history and screen cleared.", OutputStyle::Success);
        }
        "session" => {
            app.history.clear();
            if let Some(ref mut mgr) = app.session_mgr {
                let _ = mgr.archive();
            }
            app.push_output("Session archived and cleared.", OutputStyle::Success);
        }
        "all" => {
            app.history.clear();
            app.output_lines.clear();
            if let Some(ref mut mgr) = app.session_mgr {
                let _ = mgr.archive();
            }
            app.push_output(
                "Chat history and screen cleared, session archived.",
                OutputStyle::Success,
            );
        }
        other => {
            app.push_output(
                format!(
                    "Unknown clear target: {other}. Use: /clear chat, /clear session, or /clear all"
                ),
                OutputStyle::Error,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// /status helper
// ---------------------------------------------------------------------------

/// Prints workspace status into the output buffer.
fn print_status_to_buffer(app: &mut ReplApp) {
    let graph_path = app.workspace_root.join(".duumbi/graph/main.jsonld");
    let output_path = app.workspace_root.join(".duumbi/build/output");
    let history_count = snapshot::snapshot_count(&app.workspace_root).unwrap_or(0);
    let session_turns = app.history.len();

    app.push_output(
        format!("Workspace: {}", app.workspace_root.display()),
        OutputStyle::Normal,
    );
    app.push_output(
        format!(
            "  Graph:        {} {}",
            graph_path.display(),
            if graph_path.exists() {
                "[ok]"
            } else {
                "[missing]"
            }
        ),
        OutputStyle::Normal,
    );
    app.push_output(
        format!(
            "  Binary:       {} {}",
            output_path.display(),
            if output_path.exists() {
                "[ok]"
            } else {
                "(not built)"
            }
        ),
        OutputStyle::Normal,
    );
    app.push_output(
        format!("  Snapshots:    {history_count} (undo depth)"),
        OutputStyle::Normal,
    );
    app.push_output(
        format!("  Session turns: {session_turns}"),
        OutputStyle::Normal,
    );
    if let Some(llm) = &app.config.llm {
        app.push_output(
            format!("  Model:        {} ({})", llm.model, llm.provider),
            OutputStyle::Normal,
        );
    } else {
        app.push_output("  Model:        not configured", OutputStyle::Error);
    }
}

// ---------------------------------------------------------------------------
// /help
// ---------------------------------------------------------------------------

/// Pushes the available slash commands into the output buffer.
fn print_help_to_buffer(app: &mut ReplApp) {
    let entries: &[(&str, &str)] = &[
        ("Slash commands:", ""),
        ("  /build", "Compile the current graph to a native binary"),
        ("  /run [args]", "Run the compiled binary"),
        ("  /check", "Validate the graph without compiling"),
        (
            "  /describe",
            "Print human-readable pseudocode of the graph",
        ),
        ("  /undo", "Restore the previous graph snapshot"),
        (
            "  /status",
            "Show workspace, model, and session information",
        ),
        ("  /history", "Show session conversation history"),
        ("  /model", "Show the current LLM model"),
        (
            "  /clear [chat|session|all]",
            "Clear chat history and screen",
        ),
        ("", ""),
        ("Intent commands:", ""),
        ("  /intent", "List all active intents"),
        (
            "  /intent create <desc>",
            "Generate and save a new intent spec",
        ),
        ("  /intent review [name]", "Show intent details"),
        ("  /intent execute <name>", "Execute an intent end-to-end"),
        ("  /intent status [name]", "Show intent execution status"),
        ("  /intent focus <slug>", "Focus an intent (Intent mode)"),
        ("  /intent unfocus", "Clear focused intent"),
        ("", ""),
        ("Knowledge commands:", ""),
        ("  /knowledge", "Show knowledge statistics"),
        ("  /knowledge list", "List all knowledge nodes"),
        ("  /knowledge show <id>", "Show knowledge node details"),
        ("  /knowledge prune [days]", "Prune old knowledge nodes"),
        ("", ""),
        ("Session commands:", ""),
        ("  /resume", "List archived sessions"),
        (
            "  /resume <N>",
            "Load session N's history into current context",
        ),
        ("", ""),
        ("Registry & dependency commands:", ""),
        ("  /search <query>", "Search registries for modules"),
        ("  /publish", "Package and publish the current module"),
        ("  /registry list", "List configured registries"),
        ("  /deps list", "List declared dependencies"),
        ("  /deps add <name> [path]", "Add a dependency"),
        ("  /deps remove <name>", "Remove a dependency"),
        ("  /deps audit", "Verify dependency integrity"),
        ("  /deps tree", "Show the dependency tree"),
        ("  /deps update [name]", "Update dependencies"),
        ("  /deps vendor", "Vendor cached dependencies"),
        ("", ""),
        ("  /help", "Show this help text"),
        ("  /exit", "Exit the REPL"),
    ];
    for (cmd, desc) in entries {
        if cmd.is_empty() {
            app.push_output("", OutputStyle::Normal);
        } else if desc.is_empty() {
            app.push_output(*cmd, OutputStyle::Normal);
        } else {
            app.push_output(format!("{cmd:<35} {desc}"), OutputStyle::Help);
        }
    }
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

    // Load API keys from keychain for providers that use it.
    for p in &providers {
        if matches!(p.key_storage, Some(crate::config::KeyStorage::File))
            && std::env::var(&p.api_key_env).is_err()
            && let Some(key) = super::keystore::load_api_key(&p.api_key_env)
        {
            // SAFETY: single-threaded CLI — no concurrent env access.
            unsafe {
                std::env::set_var(&p.api_key_env, &key);
            }
        }
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
// Prompt with history
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

/// Lists archived session files, returning `(filename, turn_count, turns)` tuples.
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
// Closest-command suggestion
// ---------------------------------------------------------------------------

/// Finds the closest matching command using normalised Levenshtein similarity.
///
/// Returns `Some(cmd)` if the closest match has similarity > 0.5, `None`
/// otherwise.
fn find_closest_command<'a>(input: &str, commands: &[&'a str]) -> Option<&'a str> {
    let mut best: Option<(&str, f64)> = None;
    for &cmd in commands {
        let dist = strsim::normalized_levenshtein(input, cmd);
        match best {
            Some((_, best_dist)) if dist > best_dist => best = Some((cmd, dist)),
            None => best = Some((cmd, dist)),
            _ => {}
        }
    }
    best.filter(|(_, d)| *d > 0.5).map(|(cmd, _)| cmd)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_closest_build() {
        let cmds = &["/build", "/check", "/run", "/help"];
        assert_eq!(find_closest_command("/bild", cmds), Some("/build"));
    }

    #[test]
    fn find_closest_no_match() {
        let cmds = &["/build", "/check"];
        assert!(find_closest_command("/xyz", cmds).is_none());
    }

    #[test]
    fn build_prompt_no_history() {
        let prompt = build_prompt_with_history("add a function", &[]);
        assert_eq!(prompt, "add a function");
    }

    #[test]
    fn build_prompt_with_turns() {
        let history = vec![Turn {
            request: "add add function".to_string(),
            summary: "Added add".to_string(),
        }];
        let prompt = build_prompt_with_history("add multiply", &history);
        assert!(prompt.contains("Context from this session"));
        assert!(prompt.contains("add add function"));
        assert!(prompt.ends_with("add multiply"));
    }
}
