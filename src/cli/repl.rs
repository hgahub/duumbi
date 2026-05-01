//! Interactive REPL for the duumbi CLI.
//!
//! Uses ratatui for full terminal UI with a status bar, inline slash menu,
//! and two-mode (Agent/Intent) interaction. Key handling and rendering are
//! delegated to [`super::app::ReplApp`]; this module owns the event loop and
//! the async command dispatch.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::SystemTime;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event};
use crossterm::execute;
use ratatui_textarea::TextArea;

use crate::agents::analyzer::{Complexity, Risk, Scope, TaskProfile, TaskType};
use crate::agents::model_catalog::ModelSelectionContext;
use crate::agents::template::AgentRole;
use crate::agents::{LlmClient, orchestrator};
use crate::config::{DuumbiConfig, EffectiveConfig, ProviderConfigSource, ProviderRole};
use crate::intent;
use crate::interaction::router;
use crate::query::{ModeHandoff, QueryAnswer, QueryEngine, QueryRequest, split_thinking_blocks};
use crate::session::SessionManager;
use crate::snapshot;

use super::app::{ReplApp, Turn};
use super::commands;
use super::mode::{OutputStyle, ReplMode};

const ENABLE_BALANCED_MOUSE_REPORTING: &str = "\x1b[?1000h\x1b[?1002h\x1b[?1006h";
const DISABLE_ALL_MOUSE_REPORTING: &str = "\x1b[?1006l\x1b[?1015l\x1b[?1003l\x1b[?1002l\x1b[?1000l";

struct PendingProviderProbe {
    provider: crate::config::ProviderKind,
    key: String,
    is_subscription: bool,
    receiver: tokio::sync::oneshot::Receiver<
        Result<crate::agents::model_access::ProviderProbeReport, String>,
    >,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Runs the interactive REPL session until the user exits.
///
/// Initialises a ratatui terminal, creates [`ReplApp`] with all workspace
/// state, and drives the event loop. On exit the session is archived.
pub async fn run(workspace_root: PathBuf, effective_config: EffectiveConfig) -> Result<()> {
    let config = effective_config.config.clone();
    let client = build_client(&config, &workspace_root);
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

    let mut app = ReplApp::new_with_config_layers(
        config,
        effective_config.system_config,
        effective_config.user_config,
        effective_config.workspace_config,
        effective_config.provider_source,
        workspace_root,
        client,
        session_mgr,
        has_workspace,
        show_tip,
    );

    // Initialise ratatui (enters alternate screen, enables raw mode).
    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableBracketedPaste)?;
    enable_balanced_mouse_reporting()?;

    // Single-line textarea — we intercept Enter ourselves.
    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(ratatui::style::Style::default());

    let result = event_loop(&mut terminal, &mut app, &mut textarea).await;

    // Always restore the terminal, even on error.
    disable_balanced_mouse_reporting().ok();
    execute!(std::io::stdout(), DisableBracketedPaste).ok();
    ratatui::restore();

    result
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

/// Drives the ratatui event loop until the user exits or an I/O error occurs.
///
/// Polls events on a 40 ms tick (a clean divisor of the 80 ms spinner
/// frame). Redraws fire on three triggers: a real terminal event, an
/// active animation (working spinner or pulsing mode dot), or the first
/// iteration of the loop. When idle and no animation is running the
/// terminal is left untouched, keeping CPU usage near zero.
async fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
) -> Result<()> {
    use std::time::{Duration, Instant};

    const TICK: Duration = Duration::from_millis(40);
    // Initial paint so the user sees the UI immediately.
    terminal.draw(|frame| app.render(frame, textarea))?;
    let mut last_draw = Instant::now();
    let mut pending_provider_probe: Option<PendingProviderProbe> = None;

    loop {
        let mut should_redraw = false;
        if let Some(pending) = pending_provider_probe.as_mut() {
            match pending.receiver.try_recv() {
                Ok(probe_result) => {
                    let pending = pending_provider_probe
                        .take()
                        .expect("invariant: pending probe exists");
                    app.working = false;
                    match probe_result {
                        Ok(probe_report) => {
                            if let Err(e) = app.save_tested_provider_key(
                                pending.provider,
                                pending.key,
                                pending.is_subscription,
                                probe_report,
                            ) {
                                app.provider_key_test_failed(format!(
                                    "Credential save failed: {e}"
                                ));
                            }
                        }
                        Err(e) => app.provider_key_test_failed(e),
                    }
                    should_redraw = true;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    pending_provider_probe = None;
                    app.working = false;
                    app.provider_key_test_failed(
                        "Provider connection test was interrupted.".into(),
                    );
                    should_redraw = true;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
            }
        }

        let event_ready = event::poll(TICK)?;
        if event_ready {
            match event::read()? {
                Event::Key(key) => match app.handle_key(key, textarea) {
                    super::mode::Action::Continue => {
                        should_redraw = true;
                    }
                    super::mode::Action::Exit => {
                        if let Some(ref mut mgr) = app.session_mgr {
                            mgr.archive().ok();
                        }
                        app.push_output("Goodbye!", OutputStyle::Dim);
                        terminal.draw(|frame| app.render(frame, textarea))?;
                        break;
                    }
                    super::mode::Action::Submit(input) => {
                        app.working = true;
                        terminal.draw(|frame| app.render(frame, textarea))?;
                        last_draw = Instant::now();

                        if process_input(terminal, app, textarea, &input).await {
                            if let Some(ref mut mgr) = app.session_mgr {
                                mgr.archive().ok();
                            }
                            break;
                        }
                        app.working = false;
                        should_redraw = true;
                    }
                    super::mode::Action::ProviderKeySubmitted {
                        provider,
                        key,
                        is_subscription,
                    } => {
                        if pending_provider_probe.is_none() {
                            let config = app.provider_config_for_key_submission(
                                provider.clone(),
                                is_subscription,
                                None,
                            );
                            let probe_key = key.clone();
                            let (sender, receiver) = tokio::sync::oneshot::channel();
                            tokio::spawn(async move {
                                let result = super::app::probe_provider_config_with_key(
                                    config,
                                    probe_key,
                                    is_subscription,
                                )
                                .await;
                                let _ = sender.send(result);
                            });
                            pending_provider_probe = Some(PendingProviderProbe {
                                provider,
                                key,
                                is_subscription,
                                receiver,
                            });
                            app.working = true;
                            terminal.draw(|frame| app.render(frame, textarea))?;
                            last_draw = Instant::now();
                        }
                        should_redraw = true;
                    }
                },
                Event::Paste(text) => {
                    app.handle_paste(&text, textarea);
                    should_redraw = true;
                }
                Event::Mouse(mouse) => {
                    should_redraw = app.handle_mouse(mouse);
                }
                _ => {
                    should_redraw = true;
                }
            }
        }

        let needs_anim = app.needs_animation();
        if should_redraw || (needs_anim && last_draw.elapsed() >= TICK) {
            terminal.draw(|frame| app.render(frame, textarea))?;
            last_draw = Instant::now();
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
    let trimmed = input.trim();
    if matches!(trimmed, "/exit" | "/quit") {
        return handle_slash(terminal, app, textarea, input).await;
    }

    app.begin_user_block(input);
    let started = std::time::Instant::now();
    let show_elapsed = should_show_elapsed(input);

    if input.starts_with('/') {
        let should_exit = handle_slash(terminal, app, textarea, input).await;
        if show_elapsed && !should_exit {
            app.finish_current_output_elapsed(started.elapsed());
        }
        should_exit
    } else {
        match app.mode {
            ReplMode::Query => {
                handle_query_input(terminal, app, textarea, input).await;
                if show_elapsed {
                    app.finish_current_output_elapsed(started.elapsed());
                }
                false
            }
            ReplMode::Agent => {
                if router::is_question_like(input) {
                    app.push_output(
                        "This looks like a question. Use /query to keep it read-only, or /agent to force a mutation request.",
                        OutputStyle::Dim,
                    );
                } else {
                    handle_ai_request(terminal, app, textarea, input).await;
                }
                if show_elapsed {
                    app.finish_current_output_elapsed(started.elapsed());
                }
                false
            }
            ReplMode::Intent => {
                handle_intent_input(app, input).await;
                if show_elapsed {
                    app.finish_current_output_elapsed(started.elapsed());
                }
                false
            }
        }
    }
}

fn should_show_elapsed(input: &str) -> bool {
    let mut parts = input.splitn(2, ' ');
    match parts.next().unwrap_or("") {
        "/help" | "/status" => false,
        cmd if cmd.starts_with('/') => true,
        _ => true,
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

        "/describe" => match commands::describe_to_string(&graph_path) {
            Ok(description) => {
                app.push_output(description, OutputStyle::Normal);
            }
            Err(e) => {
                app.push_output(format!("Describe failed: {e:#}"), OutputStyle::Error);
            }
        },

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

        "/mode" => {
            if arg.is_empty() {
                app.push_output(
                    format!("Current mode: {}", app.mode.label()),
                    OutputStyle::Normal,
                );
                app.push_output("Usage: /mode <query|agent|intent>", OutputStyle::Dim);
            } else {
                match arg.parse::<ReplMode>() {
                    Ok(mode) => {
                        app.mode = mode;
                        app.push_output(format!("Mode: {}", mode.label()), OutputStyle::Success);
                    }
                    Err(e) => app.push_output(e.to_string(), OutputStyle::Error),
                }
            }
        }

        "/query" | "/ask" => {
            if arg.is_empty() {
                app.push_output("Usage: /query <question>", OutputStyle::Dim);
            } else {
                handle_query_input(terminal, app, textarea, arg).await;
            }
        }

        "/agent" => {
            if arg.is_empty() {
                app.push_output("Usage: /agent <mutation request>", OutputStyle::Dim);
            } else {
                handle_ai_request(terminal, app, textarea, arg).await;
            }
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
                let workspace_root = app.workspace_root.clone();
                let init_result = run_with_terminal_restore(terminal, app, textarea, || {
                    super::init::run_init(&workspace_root)
                });
                match init_result {
                    Ok(()) => {
                        app.has_workspace = true;
                        match crate::config::load_effective_config(&app.workspace_root) {
                            Ok(effective) => {
                                app.config = effective.config;
                                app.system_config = effective.system_config;
                                app.user_config = effective.user_config;
                                app.workspace_config = effective.workspace_config;
                                app.provider_config_source = effective.provider_source;
                                app.client = build_client(&app.config, &app.workspace_root);
                                app.session_mgr =
                                    SessionManager::load_or_create(&app.workspace_root).ok();
                                app.show_tip = true;
                                app.push_output("Workspace initialised.", OutputStyle::Success);
                            }
                            Err(e) => {
                                app.push_output(
                                    format!("Workspace initialised, but config reload failed: {e}"),
                                    OutputStyle::Error,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        app.push_output(format!("Init failed: {e:#}"), OutputStyle::Error);
                    }
                }
            }
        }

        "/provider" => {
            app.panel = super::mode::PanelState::ProviderManager {
                selected: 0,
                input_mode: None,
                status_msg: None,
            };
            textarea.move_cursor(ratatui_textarea::CursorMove::Head);
            textarea.delete_line_by_end();
        }

        "/help" => {
            print_help_to_buffer(app);
        }

        "/exit" | "/quit" => return true,

        _ => {
            // "Did you mean?" suggestion using Levenshtein distance.
            let known_cmds: Vec<&str> = super::completion::SLASH_COMMANDS
                .iter()
                .filter(|entry| !entry.command.contains(' '))
                .map(|entry| entry.command)
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
fn run_with_terminal_restore<F, R>(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
    f: F,
) -> R
where
    F: FnOnce() -> R,
{
    // Leave alternate screen so the command's stderr output is visible.
    disable_balanced_mouse_reporting().ok();
    ratatui::restore();

    let result = f();

    // Prompt the user to return to the TUI.
    eprintln!("\n[Press Enter to return to the REPL]");
    let _ = std::io::stdin().read_line(&mut String::new());

    // Re-enter alternate screen.
    *terminal = ratatui::init();
    enable_balanced_mouse_reporting().ok();
    // Redraw immediately so the TUI is not blank.
    let _ = terminal.draw(|frame| app.render(frame, textarea));
    result
}

fn enable_balanced_mouse_reporting() -> io::Result<()> {
    write_terminal_sequence(DISABLE_ALL_MOUSE_REPORTING)?;
    write_terminal_sequence(ENABLE_BALANCED_MOUSE_REPORTING)
}

fn disable_balanced_mouse_reporting() -> io::Result<()> {
    write_terminal_sequence(DISABLE_ALL_MOUSE_REPORTING)
}

fn write_terminal_sequence(sequence: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(sequence.as_bytes())?;
    stdout.flush()
}

// ---------------------------------------------------------------------------
// AI mutation handler
// ---------------------------------------------------------------------------

/// Handles a read-only natural-language query.
async fn handle_query_input(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut ReplApp,
    textarea: &mut TextArea<'_>,
    input: &str,
) {
    if router::is_mutation_like(input) {
        let mode = match router::classify_request(input).preferred_mode() {
            Some(ReplMode::Intent) => "intent",
            Some(ReplMode::Agent) | Some(ReplMode::Query) | None => "agent",
        };
        app.push_output(
            "Query mode is read-only. This looks like a change request.",
            OutputStyle::Dim,
        );
        app.push_output(
            format!("Suggested {mode} request: {input}"),
            OutputStyle::Ai,
        );
        if let Some(ref mut mgr) = app.session_mgr {
            mgr.add_turn(input, "Suggested write-capable handoff", "Query");
            let _ = mgr.save();
        }
        return;
    }

    let context = query_model_context(input);
    let Some(client) = select_client_for_context(app, &context) else {
        app.push_output(
            "Query mode needs an available LLM provider. Use /provider to configure or test one.",
            OutputStyle::Error,
        );
        return;
    };

    let session_turns = app
        .session_mgr
        .as_ref()
        .map(|mgr| mgr.turns().to_vec())
        .unwrap_or_default();
    let mut request = QueryRequest::new(&app.workspace_root, input);
    request.session_turns = session_turns;

    let pending_label = format!("Reviewer agent ({}) is answering", client.model_label());
    app.push_output(pending_status_text(&pending_label, 0), OutputStyle::Dim);
    let _ = terminal.draw(|frame| app.render(frame, textarea));

    let streamed = std::sync::Mutex::new(String::new());
    let engine = QueryEngine::new();
    let on_text = |text: &str| {
        streamed
            .lock()
            .expect("invariant: query stream mutex not poisoned")
            .push_str(text);
    };
    let result = {
        let query = engine.answer_streaming(client.as_ref(), request, &on_text);
        tokio::pin!(query);
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(280));
        let mut frame = 1usize;
        loop {
            tokio::select! {
                result = &mut query => break result,
                _ = tick.tick() => {
                    app.replace_last_output_line(pending_status_text(&pending_label, frame), OutputStyle::Dim);
                    let _ = terminal.draw(|frame| app.render(frame, textarea));
                    frame = frame.wrapping_add(1);
                }
            }
        }
    };
    app.pop_last_output_line();
    let _ = terminal.draw(|frame| app.render(frame, textarea));
    app.client = Some(client);

    match result {
        Ok(answer) => {
            let streamed = streamed
                .into_inner()
                .expect("invariant: query stream mutex not poisoned");
            let text = if streamed.trim().is_empty() {
                answer.text.as_str()
            } else {
                streamed.trim()
            };
            push_query_answer(app, &answer, text);
            if let Some(handoff) = answer.suggested_handoff {
                push_handoff(app, &handoff);
            }
            if let Some(ref mut mgr) = app.session_mgr {
                mgr.add_turn(input, text, "Query");
                let _ = mgr.save();
            }
        }
        Err(e) => {
            app.push_output(format!("Query failed: {e:#}"), OutputStyle::Error);
        }
    }
}

fn pending_status_text(label: &str, frame: usize) -> String {
    let dots = ".".repeat(frame % 4);
    format!("{label}{dots}")
}

fn push_query_answer(app: &mut ReplApp, answer: &QueryAnswer, text: &str) {
    let display = split_thinking_blocks(text);
    if let Some(thinking) = display.thinking.as_deref() {
        app.push_output("", OutputStyle::Normal);
        push_thinking_block(app, thinking);
        app.push_output("", OutputStyle::Normal);
    }
    if !display.answer.is_empty() {
        app.push_output(display.answer, OutputStyle::Normal);
        app.push_output("", OutputStyle::Normal);
    }
    app.push_output(
        format!(
            "Sources: {} | Confidence: {:?} | Model: {}",
            answer.sources.len(),
            answer.confidence,
            answer.model
        ),
        OutputStyle::Dim,
    );
}

fn push_thinking_block(app: &mut ReplApp, thinking: &str) {
    app.push_output("▌ Thinking:", OutputStyle::Thinking);
    for line in thinking.lines() {
        app.push_output(format!("▌ {}", line.trim_end()), OutputStyle::Thinking);
    }
}

fn push_handoff(app: &mut ReplApp, handoff: &ModeHandoff) {
    let prefix = match handoff.mode {
        ReplMode::Query => "/query",
        ReplMode::Agent => "/agent",
        ReplMode::Intent => "/intent create",
    };
    app.push_output(
        format!("{prefix} {}", handoff.suggested_request),
        OutputStyle::Dim,
    );
}

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
    let model_context = agent_model_context(request, ctx_chars / 4, is_multi_module);
    let Some(client) = select_client_for_context(app, &model_context) else {
        app.push_output(
            "AI mutations need an available LLM provider. Use /provider to configure or test one.",
            OutputStyle::Error,
        );
        return;
    };

    // Collect streamed text into a local String. We cannot update the TUI
    // mid-stream, so we accumulate and push the result after completion.
    let workspace = app.workspace_root.clone();
    let (outcome, streamed) = {
        let buf = std::sync::Mutex::new(String::new());
        let res = orchestrator::mutate_streaming(
            client.as_ref(),
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
    app.client = Some(client);

    // Push streamed AI text to the output buffer.
    if !streamed.trim().is_empty() {
        app.push_output(streamed.trim().to_string(), OutputStyle::Ai);
    }

    let result = match outcome {
        Ok(orchestrator::MutationOutcome::Success(r)) => r,
        Ok(orchestrator::MutationOutcome::NeedsClarification(question)) => {
            app.push_output(format!("Question: {question}"), OutputStyle::Ai);
            app.push_output(
                "Reply in the prompt to continue this turn.",
                OutputStyle::Dim,
            );
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
    match snapshot::save_snapshot(&workspace, &source_str) {
        Ok(snapshot_path) => app.mark_latest_user_block_revertable(snapshot_path),
        Err(e) => {
            app.push_output(
                format!("Warning: snapshot save failed: {e:#}"),
                OutputStyle::Error,
            );
        }
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
    let mut hints = Vec::new();

    app.push_output("Workspace", OutputStyle::Normal);
    app.push_output(
        format!(
            "  Root:         {}{}",
            app.workspace_root.display(),
            workspace_identity_suffix(&app.config)
        ),
        OutputStyle::Normal,
    );
    let graph_status = graph_status_line(&app.workspace_root, &graph_path);
    if !graph_path.exists() {
        hints.push("run /init to create a workspace graph".to_string());
    } else if graph_status.is_invalid {
        hints.push("fix the graph JSON or run /undo if a recent change broke it".to_string());
    }
    app.push_output(
        format!("  Graph:        {}", graph_status.text),
        graph_status.style,
    );

    let build_status = build_status_line(&output_path);
    if !output_path.exists() || build_status.is_invalid {
        hints.push("run /build to create the binary".to_string());
    }
    app.push_output(
        format!("  Binary:       {}", build_status.text),
        build_status.style,
    );

    app.push_output(
        format!("  Undo:         {history_count} snapshot(s)"),
        OutputStyle::Normal,
    );
    app.push_output(
        format!("  Dependencies: {}", dependency_status_line(app)),
        OutputStyle::Normal,
    );

    app.push_output("Session / AI", OutputStyle::Normal);
    app.push_output(
        format!("  Session:      {}", session_status_line(app)),
        OutputStyle::Normal,
    );

    let providers_empty = app.config.effective_providers().is_empty();
    app.push_output(
        format!("  Providers:    {}", providers_status_line(app)),
        if providers_empty {
            OutputStyle::Error
        } else {
            OutputStyle::Normal
        },
    );
    if providers_empty {
        hints.push("run /provider to configure AI access".to_string());
    }

    if let Some(hint) = hints.first() {
        app.push_output(format!("  Next:         {hint}"), OutputStyle::Dim);
    }
}

struct StatusLine {
    text: String,
    style: OutputStyle,
    is_invalid: bool,
}

fn workspace_identity_suffix(config: &DuumbiConfig) -> String {
    let Some(workspace) = &config.workspace else {
        return String::new();
    };
    let mut parts = Vec::new();
    if !workspace.name.is_empty() {
        parts.push(format!("name: {}", workspace.name));
    }
    if !workspace.namespace.is_empty() {
        parts.push(format!("namespace: {}", workspace.namespace));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

fn graph_status_line(workspace_root: &Path, graph_path: &Path) -> StatusLine {
    if !graph_path.exists() {
        return StatusLine {
            text: format!("{} [missing]", graph_path.display()),
            style: OutputStyle::Error,
            is_invalid: false,
        };
    }

    match graph_summary(workspace_root, graph_path) {
        Ok((modules, functions, nodes)) => StatusLine {
            text: format!(
                "{} [ok] ({} module{}, {} function{}, {} node{})",
                graph_path.display(),
                modules,
                plural(modules),
                functions,
                plural(functions),
                nodes,
                plural(nodes)
            ),
            style: OutputStyle::Success,
            is_invalid: false,
        },
        Err(err) => StatusLine {
            text: format!("{} [invalid: {err}]", graph_path.display()),
            style: OutputStyle::Error,
            is_invalid: true,
        },
    }
}

fn graph_summary(
    workspace_root: &Path,
    graph_path: &Path,
) -> Result<(usize, usize, usize), String> {
    let graph_dir = workspace_root.join(".duumbi").join("graph");
    let mut paths = Vec::new();

    if graph_dir.exists() {
        let entries = fs::read_dir(&graph_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", graph_dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonld") {
                paths.push(path);
            }
        }
    }

    if paths.is_empty() {
        paths.push(graph_path.to_path_buf());
    }
    paths.sort();

    let mut modules = 0usize;
    let mut functions = 0usize;
    let mut nodes = 0usize;
    for path in paths {
        let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let module =
            crate::parser::parse_jsonld(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        modules += 1;
        functions += module.functions.len();
        nodes += module
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .map(|block| block.ops.len())
            .sum::<usize>();
    }

    Ok((modules, functions, nodes))
}

fn build_status_line(output_path: &Path) -> StatusLine {
    if !output_path.exists() {
        return StatusLine {
            text: format!("{} (not built)", output_path.display()),
            style: OutputStyle::Dim,
            is_invalid: false,
        };
    }

    let metadata = match output_path.metadata() {
        Ok(metadata) => metadata,
        Err(e) => {
            return StatusLine {
                text: format!("{} [invalid: {e}]", output_path.display()),
                style: OutputStyle::Error,
                is_invalid: true,
            };
        }
    };

    if !metadata.is_file() {
        return StatusLine {
            text: format!("{} [invalid: not a file]", output_path.display()),
            style: OutputStyle::Error,
            is_invalid: true,
        };
    }

    if metadata.len() == 0 {
        return StatusLine {
            text: format!("{} (not built: empty file)", output_path.display()),
            style: OutputStyle::Dim,
            is_invalid: true,
        };
    }

    let modified = metadata
        .modified()
        .ok()
        .map(format_system_time)
        .map(|time| format!(", modified {time}"))
        .unwrap_or_default();

    StatusLine {
        text: format!("{} [ok]{modified}", output_path.display()),
        style: OutputStyle::Success,
        is_invalid: false,
    }
}

fn dependency_status_line(app: &ReplApp) -> String {
    let declared = app.config.dependencies.len();
    let lock = match crate::deps::load_lockfile(&app.workspace_root) {
        Ok(lock) => lock,
        Err(e) => return format!("{declared} declared, lockfile unreadable: {e}"),
    };
    let locked = lock.dependencies.len();
    let mismatch = if declared != locked {
        " (config/lock mismatch)"
    } else {
        ""
    };
    format!("{declared} declared, {locked} locked{mismatch}")
}

fn session_status_line(app: &ReplApp) -> String {
    let context_turns = app.history.len();
    match &app.session_mgr {
        Some(mgr) => format!(
            "{} (started {}, {} persisted turn{}, {} context turn{})",
            short_session_id(mgr.session_id()),
            format_started_at(mgr.started_at()),
            mgr.turns().len(),
            plural(mgr.turns().len()),
            context_turns,
            plural(context_turns)
        ),
        None => format!(
            "unavailable ({} context turn{})",
            context_turns,
            plural(context_turns)
        ),
    }
}

fn providers_status_line(app: &ReplApp) -> String {
    let providers = app.config.effective_providers();
    let source = provider_source_label(app.provider_config_source);
    if providers.is_empty() {
        return format!("not configured (source: {source})");
    }

    let labels = providers
        .iter()
        .map(|provider| {
            format!(
                "{} {}",
                provider.provider,
                provider_role_label(&provider.role)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{labels} (source: {source})")
}

fn provider_role_label(role: &ProviderRole) -> &'static str {
    match role {
        ProviderRole::Primary => "primary",
        ProviderRole::Fallback => "fallback",
    }
}

fn provider_source_label(source: ProviderConfigSource) -> &'static str {
    match source {
        ProviderConfigSource::None => "none",
        ProviderConfigSource::System => "system",
        ProviderConfigSource::User => "user",
        ProviderConfigSource::Workspace => "workspace",
        ProviderConfigSource::LegacySystem => "legacy system",
        ProviderConfigSource::LegacyUser => "legacy user",
        ProviderConfigSource::LegacyWorkspace => "legacy workspace",
    }
}

fn short_session_id(id: &str) -> String {
    const MAX_LEN: usize = 18;
    if id.chars().count() <= MAX_LEN {
        id.to_string()
    } else {
        let tail = id
            .chars()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        format!("session…{tail}")
    }
}

fn format_started_at(started_at: DateTime<Utc>) -> String {
    let local: DateTime<Local> = started_at.with_timezone(&Local);
    format!(
        "{} {}",
        local.format("%Y-%m-%d %H:%M"),
        format_age(started_at)
    )
}

fn format_age(started_at: DateTime<Utc>) -> String {
    let elapsed = Utc::now()
        .signed_duration_since(started_at)
        .max(chrono::Duration::zero());
    if elapsed.num_hours() >= 1 {
        format!("({}h ago)", elapsed.num_hours())
    } else if elapsed.num_minutes() >= 1 {
        format!("({}m ago)", elapsed.num_minutes())
    } else {
        "(<1m ago)".to_string()
    }
}

fn format_system_time(time: SystemTime) -> String {
    let local: DateTime<Local> = time.into();
    local.format("%Y-%m-%d %H:%M").to_string()
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

// ---------------------------------------------------------------------------
// /help
// ---------------------------------------------------------------------------

/// Pushes the available slash commands into the output buffer.
fn print_help_to_buffer(app: &mut ReplApp) {
    app.push_output("Slash commands:", OutputStyle::Normal);
    for group in super::completion::SLASH_GROUPS {
        app.push_output("", OutputStyle::Normal);
        app.push_output(group.label(), OutputStyle::Normal);
        for entry in super::completion::SLASH_COMMANDS
            .iter()
            .filter(|entry| entry.group == *group)
        {
            app.push_output(
                format!("  {:<32} {}", entry.command, entry.description),
                OutputStyle::Help,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Client construction
// ---------------------------------------------------------------------------

fn select_client_for_context(
    app: &mut ReplApp,
    context: &ModelSelectionContext,
) -> Option<LlmClient> {
    refresh_effective_config_from_disk(app);
    build_client_for_context(&app.config, &app.workspace_root, context)
}

fn refresh_effective_config_from_disk(app: &mut ReplApp) {
    if let Ok(effective) = crate::config::load_effective_config(&app.workspace_root) {
        app.config = effective.config;
        app.system_config = effective.system_config;
        app.user_config = effective.user_config;
        app.workspace_config = effective.workspace_config;
        app.provider_config_source = effective.provider_source;
    }
}

fn query_model_context(question: &str) -> ModelSelectionContext {
    ModelSelectionContext {
        agent_role: Some(AgentRole::Reviewer),
        prompt_tokens: Some(estimate_tokens(question)),
        requires_tools: false,
        ..ModelSelectionContext::default()
    }
}

fn agent_model_context(
    request: &str,
    prompt_tokens: usize,
    is_multi_module: bool,
) -> ModelSelectionContext {
    let profile = task_profile_from_request(request, is_multi_module);
    let agent_role = match profile.task_type {
        TaskType::Fix | TaskType::Refactor => AgentRole::Repair,
        _ => AgentRole::Coder,
    };
    ModelSelectionContext {
        agent_role: Some(agent_role),
        task_profile: Some(profile),
        prompt_tokens: Some(prompt_tokens),
        requires_tools: true,
        ..ModelSelectionContext::default()
    }
}

fn task_profile_from_request(request: &str, is_multi_module: bool) -> TaskProfile {
    let lower = request.to_lowercase();
    let task_type = match router::classify_request(request) {
        router::RequestShape::Intent => TaskType::Create,
        router::RequestShape::Mutation => {
            if ["fix", "bug", "error"]
                .iter()
                .any(|word| lower.contains(word))
            {
                TaskType::Fix
            } else if ["refactor", "rename", "reorganize", "reorganise"]
                .iter()
                .any(|word| lower.contains(word))
            {
                TaskType::Refactor
            } else if ["test", "verify"].iter().any(|word| lower.contains(word)) {
                TaskType::Test
            } else if ["add", "create", "implement"]
                .iter()
                .any(|word| lower.contains(word))
            {
                TaskType::Create
            } else {
                TaskType::Modify
            }
        }
        _ => TaskType::Modify,
    };
    let complexity = if request.len() > 900 || lower.contains("complex") {
        Complexity::Complex
    } else if request.len() > 240 || lower.contains("several") || lower.contains("multiple") {
        Complexity::Moderate
    } else {
        Complexity::Simple
    };
    let scope = if is_multi_module || lower.contains("module") || lower.contains("modules") {
        Scope::MultiModule
    } else {
        Scope::SingleModule
    };
    let touches_main = lower.contains("main");
    let risk = if matches!(scope, Scope::MultiModule) && touches_main {
        Risk::High
    } else if touches_main
        || matches!(scope, Scope::MultiModule)
        || matches!(task_type, TaskType::Fix | TaskType::Refactor)
    {
        Risk::Medium
    } else {
        Risk::Low
    };

    TaskProfile {
        complexity,
        task_type,
        scope,
        risk,
    }
}

fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Builds an [`LlmClient`] from the workspace config, or returns `None` with
/// a warning if the provider is not configured or the API key is missing.
fn build_client(config: &DuumbiConfig, workspace: &std::path::Path) -> Option<LlmClient> {
    build_client_for_context(config, workspace, &ModelSelectionContext::default())
}

fn build_client_for_context(
    config: &DuumbiConfig,
    _workspace: &std::path::Path,
    context: &ModelSelectionContext,
) -> Option<LlmClient> {
    let providers = config.effective_providers();
    if providers.is_empty() {
        return None;
    }

    load_file_credentials_for_providers(&providers);

    match crate::agents::factory::create_available_provider_chain_for_global_access_context(
        &providers, context,
    ) {
        Ok(client) => Some(client),
        Err(e) => {
            eprintln!("Warning: LLM provider not available ({e}). AI mutations disabled.");
            None
        }
    }
}

fn load_file_credentials_for_providers(providers: &[crate::config::ProviderConfig]) {
    for p in providers {
        if std::env::var(&p.api_key_env).is_err()
            && let Some(key) = crate::credentials::load_api_key(&p.api_key_env)
        {
            // SAFETY: single-threaded CLI — no concurrent env access.
            unsafe {
                std::env::set_var(&p.api_key_env, &key);
            }
        }
        if let Some(token_env) = &p.auth_token_env
            && std::env::var(token_env).is_err()
            && let Some(token) = crate::credentials::load_api_key(token_env)
        {
            // SAFETY: single-threaded CLI — no concurrent env access.
            unsafe {
                std::env::set_var(token_env, &token);
            }
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
    use crate::config::{ProviderConfig, ProviderKind, WorkspaceSection};
    use tempfile::TempDir;

    fn status_test_app(
        dir: &TempDir,
        config: DuumbiConfig,
        source: ProviderConfigSource,
        session_mgr: Option<SessionManager>,
    ) -> ReplApp {
        ReplApp::new_with_config_layers(
            config.clone(),
            DuumbiConfig::default(),
            DuumbiConfig::default(),
            config,
            source,
            dir.path().to_path_buf(),
            None,
            session_mgr,
            true,
            false,
        )
    }

    fn status_output(app: &ReplApp) -> String {
        app.output_lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn write_main_graph(dir: &TempDir, graph: &str) {
        let graph_dir = dir.path().join(".duumbi").join("graph");
        fs::create_dir_all(&graph_dir).expect("graph dir must be created");
        fs::write(graph_dir.join("main.jsonld"), graph).expect("graph must be written");
    }

    fn minimal_graph() -> &'static str {
        r#"{
  "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [{
    "@type": "duumbi:Function",
    "@id": "duumbi:main/main",
    "duumbi:name": "main",
    "duumbi:returnType": "i64",
    "duumbi:blocks": [{
      "@type": "duumbi:Block",
      "@id": "duumbi:main/main/entry",
      "duumbi:label": "entry",
      "duumbi:ops": [
        {"@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0",
         "duumbi:value": 1, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1",
         "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
      ]
    }]
  }]
}"#
    }

    fn provider(kind: ProviderKind, role: ProviderRole) -> ProviderConfig {
        ProviderConfig {
            provider: kind,
            role,
            model: None,
            api_key_env: "TEST_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        }
    }

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

    #[test]
    fn query_answer_formats_thinking_answer_and_model_metadata() {
        let mut app = ReplApp::new(
            crate::config::DuumbiConfig::default(),
            std::path::PathBuf::from("."),
            None,
            None,
            true,
            false,
        );
        let answer = QueryAnswer {
            text: "<think>inspect workspace</think>Hello.".to_string(),
            model: "minimax/MiniMax-M2.7".to_string(),
            sources: Vec::new(),
            confidence: crate::query::AnswerConfidence::Low,
            suggested_handoff: None,
        };

        push_query_answer(&mut app, &answer, &answer.text);

        assert_eq!(app.output_lines[0].text, "");
        assert_eq!(app.output_lines[1].style, OutputStyle::Thinking);
        assert_eq!(app.output_lines[2].style, OutputStyle::Thinking);
        assert_eq!(app.output_lines[3].text, "");
        assert_eq!(app.output_lines[4].style, OutputStyle::Normal);
        assert_eq!(app.output_lines[4].text, "Hello.");
        assert_eq!(app.output_lines[5].text, "");
        assert!(app.output_lines[6].text.contains("Confidence: Low"));
        assert!(
            app.output_lines[6]
                .text
                .contains("Model: minimax/MiniMax-M2.7")
        );
    }

    #[test]
    fn query_pending_status_animates_three_dots() {
        assert_eq!(
            pending_status_text("Reviewer agent is answering", 0),
            "Reviewer agent is answering"
        );
        assert_eq!(
            pending_status_text("Reviewer agent is answering", 1),
            "Reviewer agent is answering."
        );
        assert_eq!(
            pending_status_text("Reviewer agent is answering", 2),
            "Reviewer agent is answering.."
        );
        assert_eq!(
            pending_status_text("Reviewer agent is answering", 3),
            "Reviewer agent is answering..."
        );
        assert_eq!(
            pending_status_text("Reviewer agent is answering", 4),
            "Reviewer agent is answering"
        );
    }

    #[test]
    fn elapsed_policy_skips_internal_commands() {
        assert!(!should_show_elapsed("/help"));
        assert!(!should_show_elapsed("/status"));
        assert!(should_show_elapsed("/describe"));
        assert!(should_show_elapsed("create a calculator"));
    }

    #[test]
    fn balanced_mouse_reporting_enables_app_drag_without_all_motion() {
        assert!(ENABLE_BALANCED_MOUSE_REPORTING.contains("?1000h"));
        assert!(ENABLE_BALANCED_MOUSE_REPORTING.contains("?1002h"));
        assert!(ENABLE_BALANCED_MOUSE_REPORTING.contains("?1006h"));
        assert!(!ENABLE_BALANCED_MOUSE_REPORTING.contains("?1003h"));
        assert!(!ENABLE_BALANCED_MOUSE_REPORTING.contains("?1015h"));

        assert!(DISABLE_ALL_MOUSE_REPORTING.contains("?1006l"));
        assert!(DISABLE_ALL_MOUSE_REPORTING.contains("?1015l"));
        assert!(DISABLE_ALL_MOUSE_REPORTING.contains("?1003l"));
        assert!(DISABLE_ALL_MOUSE_REPORTING.contains("?1002l"));
        assert!(DISABLE_ALL_MOUSE_REPORTING.contains("?1000l"));
    }

    #[test]
    fn help_uses_slash_group_order() {
        let mut app = ReplApp::new(
            crate::config::DuumbiConfig::default(),
            std::path::PathBuf::from("."),
            None,
            None,
            true,
            false,
        );

        print_help_to_buffer(&mut app);
        let rendered = app
            .output_lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let build = rendered.find("BUILD & RUN").expect("build group");
        let intent = rendered.find("INTENT").expect("intent group");
        let system = rendered.find("SYSTEM").expect("system group");
        assert!(build < intent);
        assert!(intent < system);
    }

    #[test]
    fn status_shows_configured_workspace_graph_provider_and_session() {
        let dir = TempDir::new().expect("tempdir");
        write_main_graph(&dir, minimal_graph());
        let build_dir = dir.path().join(".duumbi").join("build");
        fs::create_dir_all(&build_dir).expect("build dir must be created");
        fs::write(build_dir.join("output"), b"binary").expect("binary must be written");

        let mut config = DuumbiConfig {
            workspace: Some(WorkspaceSection {
                name: "status-test".to_string(),
                namespace: "hgahub".to_string(),
                default_registry: None,
            }),
            ..DuumbiConfig::default()
        };
        config
            .providers
            .push(provider(ProviderKind::MiniMax, ProviderRole::Primary));
        config
            .providers
            .push(provider(ProviderKind::Grok, ProviderRole::Fallback));
        let session_mgr = SessionManager::load_or_create(dir.path()).expect("session manager");
        let mut app = status_test_app(&dir, config, ProviderConfigSource::User, Some(session_mgr));

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("Workspace"));
        assert!(rendered.contains("name: status-test"));
        assert!(rendered.contains("namespace: hgahub"));
        assert!(rendered.contains("[ok] (1 module, 1 function, 2 nodes)"));
        assert!(rendered.contains("Binary:"));
        assert!(rendered.contains("[ok], modified"));
        assert!(rendered.contains("Session / AI"));
        assert!(rendered.contains("minimax primary, grok fallback (source: user)"));
        assert!(rendered.contains("0 persisted turns, 0 context turns"));
        assert!(!rendered.contains("LLM calls"));
    }

    #[test]
    fn status_reports_unconfigured_provider_and_first_actionable_hint() {
        let dir = TempDir::new().expect("tempdir");
        let mut app = status_test_app(
            &dir,
            DuumbiConfig::default(),
            ProviderConfigSource::None,
            None,
        );

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("Graph:"));
        assert!(rendered.contains("[missing]"));
        assert!(rendered.contains("Binary:"));
        assert!(rendered.contains("(not built)"));
        assert!(rendered.contains("Providers:    not configured (source: none)"));
        assert!(rendered.contains("Session:      unavailable (0 context turns)"));
        assert!(rendered.contains("Next:         run /init to create a workspace graph"));
    }

    #[test]
    fn status_reports_invalid_graph() {
        let dir = TempDir::new().expect("tempdir");
        write_main_graph(&dir, "{not json");
        let mut app = status_test_app(
            &dir,
            DuumbiConfig::default(),
            ProviderConfigSource::Workspace,
            None,
        );

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("[invalid:"));
        assert!(rendered.contains("Next:         fix the graph JSON or run /undo"));
    }

    #[test]
    fn status_rejects_directory_build_output() {
        let dir = TempDir::new().expect("tempdir");
        write_main_graph(&dir, minimal_graph());
        fs::create_dir_all(dir.path().join(".duumbi/build/output"))
            .expect("output directory must be created");
        let mut app = status_test_app(
            &dir,
            DuumbiConfig::default(),
            ProviderConfigSource::None,
            None,
        );

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("Binary:"));
        assert!(rendered.contains("[invalid: not a file]"));
        assert!(rendered.contains("Next:         run /build to create the binary"));
    }

    #[test]
    fn status_rejects_empty_build_output() {
        let dir = TempDir::new().expect("tempdir");
        write_main_graph(&dir, minimal_graph());
        let build_dir = dir.path().join(".duumbi").join("build");
        fs::create_dir_all(&build_dir).expect("build dir must be created");
        fs::write(build_dir.join("output"), b"").expect("empty output must be written");
        let mut app = status_test_app(
            &dir,
            DuumbiConfig::default(),
            ProviderConfigSource::None,
            None,
        );

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("Binary:"));
        assert!(rendered.contains("(not built: empty file)"));
        assert!(rendered.contains("Next:         run /build to create the binary"));
    }

    #[test]
    fn status_shows_legacy_workspace_provider_source() {
        let dir = TempDir::new().expect("tempdir");
        write_main_graph(&dir, minimal_graph());
        let config = DuumbiConfig {
            llm: Some(crate::config::LlmConfig {
                provider: crate::config::LlmProvider::Anthropic,
                model: "legacy-model".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
            }),
            ..DuumbiConfig::default()
        };
        let mut app = status_test_app(&dir, config, ProviderConfigSource::LegacyWorkspace, None);

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("anthropic primary (source: legacy workspace)"));
    }

    #[test]
    fn status_reports_dependency_lock_mismatch() {
        let dir = TempDir::new().expect("tempdir");
        write_main_graph(&dir, minimal_graph());
        let mut config = DuumbiConfig::default();
        config.dependencies.insert(
            "local-lib".to_string(),
            crate::config::DependencyConfig::Path {
                path: "../local-lib".to_string(),
            },
        );
        let mut app = status_test_app(&dir, config, ProviderConfigSource::None, None);

        print_status_to_buffer(&mut app);
        let rendered = status_output(&app);

        assert!(rendered.contains("Dependencies: 1 declared, 0 locked (config/lock mismatch)"));
    }
}
