//! Main REPL application struct with ratatui rendering.
//!
//! [`ReplApp`] owns all REPL state and implements the full terminal UI
//! using ratatui. Key handling delegates to `handle_key` which returns an
//! [`Action`] that the event loop acts on.

use std::path::PathBuf;

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use tui_textarea::{CursorMove, TextArea};

use crate::agents::LlmClient;
use crate::config::DuumbiConfig;
use crate::session::SessionManager;

// ---------------------------------------------------------------------------
// Wizard helpers
// ---------------------------------------------------------------------------

/// Provider kinds available for selection in the wizard.
const PROVIDER_KINDS: &[(&str, &str)] = &[
    ("anthropic", "Anthropic (Claude)"),
    ("openai", "OpenAI (GPT)"),
    ("grok", "xAI (Grok)"),
    ("openrouter", "OpenRouter (multi-model gateway)"),
    ("minimax", "MiniMax"),
];

/// Returns recommended models for a given provider kind.
fn recommended_models(kind: &crate::config::ProviderKind) -> Vec<(&'static str, &'static str)> {
    use crate::config::ProviderKind;
    match kind {
        ProviderKind::Anthropic => vec![
            ("claude-sonnet-4-6", "Recommended — fast, capable"),
            ("claude-opus-4-6", "Most capable, slower"),
            ("claude-haiku-4-5", "Fastest, most affordable"),
        ],
        ProviderKind::OpenAI => vec![
            ("gpt-4o", "Recommended — fast, capable"),
            ("gpt-4o-mini", "Fastest, most affordable"),
            ("o3", "Most capable reasoning model"),
        ],
        ProviderKind::Grok => vec![
            ("grok-3", "Recommended — full capability"),
            ("grok-3-mini", "Fast, lightweight"),
        ],
        ProviderKind::OpenRouter => vec![
            ("anthropic/claude-sonnet-4-6", "Anthropic via OpenRouter"),
            ("google/gemini-2.5-pro", "Google Gemini via OpenRouter"),
            ("openai/gpt-4o", "OpenAI via OpenRouter"),
        ],
        ProviderKind::MiniMax => vec![
            ("MiniMax-M2.7", "Latest flagship model"),
            ("MiniMax-M2.5", "Best for coding (SWE-Bench 80.2%)"),
            ("MiniMax-Text-01", "456B params, 4M context"),
        ],
    }
}

/// Returns the conventional environment variable name for the API key.
fn default_api_key_env(kind: &crate::config::ProviderKind) -> &'static str {
    use crate::config::ProviderKind;
    match kind {
        ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
        ProviderKind::OpenAI => "OPENAI_API_KEY",
        ProviderKind::Grok => "XAI_API_KEY",
        ProviderKind::OpenRouter => "OPENROUTER_API_KEY",
        ProviderKind::MiniMax => "MINIMAX_API_KEY",
    }
}

/// Parses a provider kind by wizard list index.
fn parse_provider_kind_by_index(idx: usize) -> Option<crate::config::ProviderKind> {
    use crate::config::ProviderKind;
    match idx {
        0 => Some(ProviderKind::Anthropic),
        1 => Some(ProviderKind::OpenAI),
        2 => Some(ProviderKind::Grok),
        3 => Some(ProviderKind::OpenRouter),
        4 => Some(ProviderKind::MiniMax),
        _ => None,
    }
}

use super::completion::SLASH_COMMANDS;
use super::mode::{
    Action, OutputLine, OutputStyle, PanelInputMode, PanelState, ReplMode, SlashMatch,
};

// ---------------------------------------------------------------------------
// Turn
// ---------------------------------------------------------------------------

/// A single completed conversation turn held in memory during the session.
#[derive(Debug, Clone)]
pub struct Turn {
    /// The original user request.
    pub request: String,
    /// Human-readable summary of the changes made.
    pub summary: String,
}

// ---------------------------------------------------------------------------
// ReplApp
// ---------------------------------------------------------------------------

/// Full REPL application state rendered through ratatui.
///
/// The struct owns all mutable state for the terminal UI: the current mode,
/// scrollable output buffer, slash-command menu, and workspace metadata.
/// Rendering is driven by [`ReplApp::render`]; key handling by
/// [`ReplApp::handle_key`].
pub struct ReplApp {
    /// Current interaction mode (Agent or Intent).
    pub mode: ReplMode,
    /// Intent slug that is currently focused, if any.
    pub focused_intent: Option<String>,
    /// Absolute path to the workspace root.
    pub workspace_root: PathBuf,
    /// Parsed workspace configuration.
    pub config: DuumbiConfig,
    /// LLM client, or `None` when no provider is configured.
    pub client: Option<LlmClient>,
    /// Completed conversation turns for context injection.
    pub history: Vec<Turn>,
    /// Persistent session manager (None when workspace is not initialised).
    pub session_mgr: Option<SessionManager>,
    /// Whether the workspace has been initialised (`.duumbi/` exists).
    pub has_workspace: bool,
    /// Scrollable output buffer.
    pub output_lines: Vec<OutputLine>,
    /// Scroll offset for the output area (lines from the bottom; 0 = latest).
    pub output_scroll_offset: usize,
    /// All matching slash-command entries (untruncated).
    pub slash_matches: Vec<SlashMatch>,
    /// Index of the highlighted entry in the slash menu.
    pub slash_selected: usize,
    /// Scroll offset for the slash menu (first visible row index).
    pub slash_scroll_offset: usize,
    /// Whether to show the empty-workspace onboarding tip.
    pub show_tip: bool,
    /// Active interactive panel below the prompt (None = normal mode).
    pub panel: PanelState,
    /// Cached set of env var names that have a key stored in `~/.duumbi/credentials.toml`.
    /// Populated once at startup and refreshed after provider mutations.
    keychain_cache: std::collections::HashSet<String>,
}

impl ReplApp {
    /// Maximum number of slash-menu rows visible at once.
    const SLASH_MENU_VISIBLE: usize = 5;

    /// Maximum number of lines kept in the output buffer.
    const OUTPUT_BUFFER_MAX: usize = 10_000;

    /// Creates a new `ReplApp` with the given workspace context.
    #[must_use]
    pub fn new(
        config: DuumbiConfig,
        workspace_root: PathBuf,
        client: Option<LlmClient>,
        session_mgr: Option<SessionManager>,
        has_workspace: bool,
        show_tip: bool,
    ) -> Self {
        let keychain_cache = Self::build_keychain_cache(&config);
        Self {
            mode: ReplMode::default(),
            focused_intent: None,
            workspace_root,
            config,
            client,
            history: Vec::new(),
            session_mgr,
            has_workspace,
            output_lines: Vec::new(),
            output_scroll_offset: 0,
            slash_matches: Vec::new(),
            slash_selected: 0,
            slash_scroll_offset: 0,
            show_tip,
            panel: PanelState::default(),
            keychain_cache,
        }
    }

    // -----------------------------------------------------------------------
    // Key handling
    // -----------------------------------------------------------------------

    /// Processes a key event and returns the [`Action`] the event loop should take.
    ///
    /// Mutates `textarea` for ordinary printable keys; mutates `self` for
    /// mode toggles, slash-menu navigation, and output buffer updates.
    pub fn handle_key(&mut self, key: KeyEvent, textarea: &mut TextArea<'_>) -> Action {
        // Interactive panel gets priority over normal key handling.
        if matches!(self.panel, PanelState::ModelSelector { .. }) {
            return self.handle_model_panel_key(key, textarea);
        }

        match key.code {
            // Shift+Tab: toggle Agent ↔ Intent mode
            KeyCode::BackTab => {
                self.mode = match self.mode {
                    ReplMode::Agent => ReplMode::Intent,
                    ReplMode::Intent => ReplMode::Agent,
                };
                Action::Continue
            }

            // Enter: if single slash match → execute directly; multi → accept into textarea; else submit
            KeyCode::Enter => {
                if self.slash_matches.len() == 1 {
                    // Single match: execute it directly.
                    let cmd = self.slash_matches[0].command.clone();
                    textarea.move_cursor(CursorMove::Head);
                    textarea.delete_line_by_end();
                    self.slash_matches.clear();
                    self.slash_selected = 0;
                    return Action::Submit(cmd);
                }
                if !self.slash_matches.is_empty() {
                    // Multiple matches: execute the highlighted command directly.
                    let cmd = self.slash_matches[self.slash_selected].command.clone();
                    textarea.move_cursor(CursorMove::Head);
                    textarea.delete_line_by_end();
                    self.slash_matches.clear();
                    self.slash_selected = 0;
                    self.slash_scroll_offset = 0;
                    return Action::Submit(cmd);
                }

                let input: String = textarea.lines().join("\n");
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    return Action::Continue;
                }

                // Clear the textarea fully (select all + delete).
                textarea.select_all();
                textarea.cut();

                Action::Submit(trimmed)
            }

            // Up: move slash menu selection up (scrolls window)
            KeyCode::Up if !self.slash_matches.is_empty() => {
                if self.slash_selected > 0 {
                    self.slash_selected -= 1;
                    if self.slash_selected < self.slash_scroll_offset {
                        self.slash_scroll_offset = self.slash_selected;
                    }
                }
                Action::Continue
            }

            // Down: move slash menu selection down (scrolls window)
            KeyCode::Down if !self.slash_matches.is_empty() => {
                let max = self.slash_matches.len().saturating_sub(1);
                if self.slash_selected < max {
                    self.slash_selected += 1;
                    let visible = Self::SLASH_MENU_VISIBLE;
                    if self.slash_selected >= self.slash_scroll_offset + visible {
                        self.slash_scroll_offset = self.slash_selected + 1 - visible;
                    }
                }
                Action::Continue
            }

            // Tab: accept selected slash command without submitting
            KeyCode::Tab if !self.slash_matches.is_empty() => {
                let cmd = self.slash_matches[self.slash_selected].command.clone();
                textarea.move_cursor(CursorMove::Head);
                textarea.delete_line_by_end();
                textarea.insert_str(&cmd);
                self.slash_matches.clear();
                self.slash_selected = 0;
                self.slash_scroll_offset = 0;
                Action::Continue
            }

            // Esc: dismiss slash menu
            KeyCode::Esc => {
                self.slash_matches.clear();
                self.slash_selected = 0;
                self.slash_scroll_offset = 0;
                Action::Continue
            }

            // PageUp: scroll output buffer up (toward older lines)
            KeyCode::PageUp => {
                let max_scroll = self.output_lines.len().saturating_sub(1);
                self.output_scroll_offset = (self.output_scroll_offset + 10).min(max_scroll);
                Action::Continue
            }

            // PageDown: scroll output buffer down (toward latest)
            KeyCode::PageDown => {
                self.output_scroll_offset = self.output_scroll_offset.saturating_sub(10);
                Action::Continue
            }

            // Ctrl+D: exit
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Exit,

            // Ctrl+C: friendly quit reminder
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.push_output("(Use Ctrl+D to quit)", OutputStyle::Dim);
                Action::Continue
            }

            // All other keys go to the textarea
            _ => {
                textarea.input(key);
                let current = textarea.lines().join("\n");
                self.update_slash_matches(&current);
                Action::Continue
            }
        }
    }

    /// Handles mouse events (scroll wheel for output buffer scrolling).
    ///
    /// Currently unused — mouse capture is disabled to allow native text
    /// selection. Scroll is available via keyboard (PageUp/PageDown).
    #[allow(dead_code)]
    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                let max_scroll = self.output_lines.len().saturating_sub(1);
                self.output_scroll_offset = (self.output_scroll_offset + 1).min(max_scroll);
            }
            MouseEventKind::ScrollDown => {
                self.output_scroll_offset = self.output_scroll_offset.saturating_sub(1);
            }
            _ => {}
        }
    }

    /// Handles key events when the model selector panel is active.
    ///
    /// Extracts panel state by value, processes the key, then writes back.
    fn handle_model_panel_key(
        &mut self,
        key_event: KeyEvent,
        textarea: &mut TextArea<'_>,
    ) -> Action {
        // Extract current panel state (take ownership to avoid borrow issues).
        let (mut selected, mut input_mode) = match &self.panel {
            PanelState::ModelSelector {
                selected,
                input_mode,
                ..
            } => (*selected, input_mode.clone()),
            PanelState::None => return Action::Continue,
        };
        // Status message is cleared on every key press; handlers may set a new one.
        let mut new_status_msg: Option<(String, OutputStyle)> = None;

        // --- Sub-mode input handling ---
        if let Some(ref mut mode) = input_mode {
            match mode {
                PanelInputMode::AddProvider(buf) => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = None;
                    }
                    KeyCode::Enter => {
                        let input_str = buf.clone();
                        input_mode = None;
                        let lines = super::provider::add_provider(&mut self.config, &input_str);
                        for line in lines {
                            self.output_lines.push(line);
                        }
                        self.save_config_and_rebuild_client();
                    }
                    KeyCode::Backspace => {
                        buf.pop();
                    }
                    KeyCode::Char(c) => {
                        buf.push(c);
                    }
                    _ => {}
                },
                PanelInputMode::ConfirmDelete => match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let selector = (selected + 1).to_string();
                        input_mode = None;
                        let lines = super::provider::remove_provider(&mut self.config, &selector);
                        for line in lines {
                            self.output_lines.push(line);
                        }
                        self.save_config_and_rebuild_client();
                        let count = self.config.effective_providers().len();
                        if selected >= count && count > 0 {
                            selected = count - 1;
                        }
                    }
                    _ => {
                        input_mode = None;
                    }
                },
                PanelInputMode::AddStep1Provider { selected: step_sel } => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = None;
                    }
                    KeyCode::Up => {
                        if *step_sel > 0 {
                            *step_sel -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if *step_sel < PROVIDER_KINDS.len() - 1 {
                            *step_sel += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(kind) = parse_provider_kind_by_index(*step_sel) {
                            input_mode = Some(PanelInputMode::AddStep2Model {
                                provider: kind,
                                selected: 0,
                                manual_input: None,
                            });
                        }
                    }
                    _ => {}
                },
                PanelInputMode::AddStep2Model {
                    provider,
                    selected: model_sel,
                    manual_input,
                } => {
                    if let Some(manual) = manual_input {
                        // Manual input mode
                        match key_event.code {
                            KeyCode::Esc => {
                                *manual_input = None;
                            }
                            KeyCode::Backspace => {
                                manual.pop();
                            }
                            KeyCode::Char(c) => {
                                manual.push(c);
                            }
                            KeyCode::Enter if !manual.is_empty() => {
                                let model = manual.clone();
                                input_mode = Some(PanelInputMode::AddStep3Key {
                                    provider: provider.clone(),
                                    model,
                                    key_buf: String::new(),
                                });
                            }
                            _ => {}
                        }
                    } else {
                        // List selection mode
                        let models = recommended_models(provider);
                        match key_event.code {
                            KeyCode::Esc => {
                                input_mode = Some(PanelInputMode::AddStep1Provider { selected: 0 });
                            }
                            KeyCode::Up => {
                                if *model_sel > 0 {
                                    *model_sel -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if *model_sel < models.len().saturating_sub(1) {
                                    *model_sel += 1;
                                }
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                *manual_input = Some(String::new());
                            }
                            KeyCode::Enter => {
                                if let Some((model_name, _)) = models.get(*model_sel) {
                                    input_mode = Some(PanelInputMode::AddStep3Key {
                                        provider: provider.clone(),
                                        model: (*model_name).to_string(),
                                        key_buf: String::new(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                PanelInputMode::AddStep3Key {
                    provider,
                    model,
                    key_buf,
                } => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = Some(PanelInputMode::AddStep2Model {
                            provider: provider.clone(),
                            selected: 0,
                            manual_input: None,
                        });
                    }
                    KeyCode::Backspace => {
                        key_buf.pop();
                    }
                    KeyCode::Enter if !key_buf.is_empty() => {
                        // Transition to storage-choice confirmation step.
                        input_mode = Some(PanelInputMode::AddStep3Confirm {
                            provider: provider.clone(),
                            model: model.clone(),
                            key: key_buf.clone(),
                        });
                    }
                    KeyCode::Char(c) => {
                        key_buf.push(c);
                    }
                    _ => {}
                },
                PanelInputMode::AddStep3Confirm {
                    provider,
                    model,
                    key: api_key_value,
                } => match key_event.code {
                    KeyCode::Char('k') | KeyCode::Char('K') => {
                        let api_key_env = default_api_key_env(provider).to_string();
                        let prov_clone = provider.clone();
                        let model_clone = model.clone();
                        let key_clone = api_key_value.clone();
                        match super::keystore::store_api_key(&api_key_env, &key_clone) {
                            Ok(()) => {
                                // SAFETY: single-threaded CLI — no concurrent env access.
                                unsafe {
                                    std::env::set_var(&api_key_env, &key_clone);
                                }
                                self.config.providers.push(crate::config::ProviderConfig {
                                    provider: prov_clone,
                                    role: if self.config.providers.is_empty() {
                                        crate::config::ProviderRole::Primary
                                    } else {
                                        crate::config::ProviderRole::Fallback
                                    },
                                    model: model_clone,
                                    api_key_env: api_key_env.clone(),
                                    base_url: None,
                                    timeout_secs: None,
                                    key_storage: Some(crate::config::KeyStorage::File),
                                    auth_token_env: None,
                                });
                                self.save_config_and_rebuild_client();
                                selected = self.config.providers.len() - 1;
                                new_status_msg = Some((
                                    "Provider added. Key saved to ~/.duumbi/credentials.toml."
                                        .to_string(),
                                    OutputStyle::Success,
                                ));
                            }
                            Err(e) => {
                                // File storage failed — fall back to session-only.
                                // SAFETY: single-threaded CLI — no concurrent env access.
                                unsafe {
                                    std::env::set_var(&api_key_env, &key_clone);
                                }
                                self.config.providers.push(crate::config::ProviderConfig {
                                    provider: prov_clone,
                                    role: if self.config.providers.is_empty() {
                                        crate::config::ProviderRole::Primary
                                    } else {
                                        crate::config::ProviderRole::Fallback
                                    },
                                    model: model_clone,
                                    api_key_env,
                                    base_url: None,
                                    timeout_secs: None,
                                    key_storage: None,
                                    auth_token_env: None,
                                });
                                self.save_config_and_rebuild_client();
                                selected = self.config.providers.len() - 1;
                                new_status_msg = Some((
                                    format!("File storage error: {e}. Key set for session only."),
                                    OutputStyle::Error,
                                ));
                            }
                        }
                        input_mode = None; // back to list view
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
                        let api_key_env = default_api_key_env(provider).to_string();
                        let prov_clone = provider.clone();
                        let model_clone = model.clone();
                        let key_clone = api_key_value.clone();
                        // SAFETY: single-threaded CLI — no concurrent env access.
                        unsafe {
                            std::env::set_var(&api_key_env, &key_clone);
                        }
                        self.config.providers.push(crate::config::ProviderConfig {
                            provider: prov_clone,
                            role: if self.config.providers.is_empty() {
                                crate::config::ProviderRole::Primary
                            } else {
                                crate::config::ProviderRole::Fallback
                            },
                            model: model_clone,
                            api_key_env: api_key_env.clone(),
                            base_url: None,
                            timeout_secs: None,
                            key_storage: None,
                            auth_token_env: None,
                        });
                        self.save_config_and_rebuild_client();
                        selected = self.config.providers.len() - 1;
                        new_status_msg = Some((
                            format!("Provider added ({api_key_env}, session only)."),
                            OutputStyle::Success,
                        ));
                        input_mode = None; // back to list view
                    }
                    KeyCode::Esc => {
                        // Back to key entry step with the key pre-filled.
                        input_mode = Some(PanelInputMode::AddStep3Key {
                            provider: provider.clone(),
                            model: model.clone(),
                            key_buf: api_key_value.clone(),
                        });
                    }
                    _ => {}
                },
            }
            self.panel = PanelState::ModelSelector {
                selected,
                input_mode,
                status_msg: new_status_msg,
            };
            return Action::Continue;
        }

        // --- Main panel key handling (no sub-mode) ---
        let provider_count = self.config.effective_providers().len();

        match key_event.code {
            KeyCode::Esc => {
                self.panel = PanelState::None;
                textarea.move_cursor(CursorMove::Head);
                textarea.delete_line_by_end();
                Action::Continue
            }
            KeyCode::Up => {
                selected = selected.saturating_sub(1);
                self.panel = PanelState::ModelSelector {
                    selected,
                    input_mode: None,
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Down => {
                if provider_count > 0 && selected < provider_count - 1 {
                    selected += 1;
                }
                self.panel = PanelState::ModelSelector {
                    selected,
                    input_mode: None,
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Enter => {
                // Set selected provider as primary, all others as fallback.
                if provider_count > 0 && selected < self.config.providers.len() {
                    for (i, p) in self.config.providers.iter_mut().enumerate() {
                        p.role = if i == selected {
                            crate::config::ProviderRole::Primary
                        } else {
                            crate::config::ProviderRole::Fallback
                        };
                    }
                    let model = self.config.providers[selected].model.clone();
                    self.push_output(format!("Selected: {model}"), OutputStyle::Success);
                    self.save_config_and_rebuild_client();
                }
                self.panel = PanelState::None;
                textarea.move_cursor(CursorMove::Head);
                textarea.delete_line_by_end();
                Action::Continue
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.panel = PanelState::ModelSelector {
                    selected,
                    input_mode: Some(PanelInputMode::AddStep1Provider { selected: 0 }),
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if provider_count > 0 {
                    self.panel = PanelState::ModelSelector {
                        selected,
                        input_mode: Some(PanelInputMode::ConfirmDelete),
                        status_msg: None,
                    };
                }
                Action::Continue
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if provider_count > 0 && selected < self.config.providers.len() {
                    self.config.providers[selected].role =
                        match self.config.providers[selected].role {
                            crate::config::ProviderRole::Primary => {
                                crate::config::ProviderRole::Fallback
                            }
                            crate::config::ProviderRole::Fallback => {
                                crate::config::ProviderRole::Primary
                            }
                        };
                    self.save_config_and_rebuild_client();
                }
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    /// Persists the current config to disk and rebuilds the LLM client.
    pub fn save_config_and_rebuild_client(&mut self) {
        if self.has_workspace {
            let _ = crate::config::save_config(&self.workspace_root, &self.config);
        }
        let providers = self.config.effective_providers();
        self.client = if providers.is_empty() {
            None
        } else {
            crate::agents::factory::create_provider_chain(&providers).ok()
        };
        self.keychain_cache = Self::build_keychain_cache(&self.config);
    }

    /// Reads `~/.duumbi/credentials.toml` once to build a cache of which env
    /// var names have a stored key. Used by the render path to avoid repeated
    /// file reads on every frame.
    fn build_keychain_cache(config: &DuumbiConfig) -> std::collections::HashSet<String> {
        config
            .effective_providers()
            .iter()
            .filter(|p| matches!(p.key_storage, Some(crate::config::KeyStorage::File)))
            .filter_map(|p| {
                if super::keystore::load_api_key(&p.api_key_env).is_some() {
                    Some(p.api_key_env.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Slash-menu filtering
    // -----------------------------------------------------------------------

    /// Updates the slash-command menu based on the current input line.
    ///
    /// If `input` starts with `/`, filters [`SLASH_COMMANDS`] by prefix and
    /// populates `slash_matches` with all results. Clears matches when input
    /// does not start with `/`.
    pub fn update_slash_matches(&mut self, input: &str) {
        if input.starts_with('/') {
            self.slash_matches = SLASH_COMMANDS
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(input) && *cmd != input)
                .map(|(cmd, desc)| SlashMatch {
                    command: (*cmd).to_string(),
                    description: (*desc).to_string(),
                })
                .collect();
        } else {
            self.slash_matches.clear();
        }
        self.slash_selected = 0;
        self.slash_scroll_offset = 0;
    }

    // -----------------------------------------------------------------------
    // Output buffer
    // -----------------------------------------------------------------------

    /// Appends text to the output buffer, splitting on newlines.
    ///
    /// Trims the buffer to [`Self::OUTPUT_BUFFER_MAX`] lines to prevent
    /// unbounded memory growth.
    pub fn push_output(&mut self, text: impl Into<String>, style: OutputStyle) {
        let text = text.into();
        for line in text.split('\n') {
            self.output_lines
                .push(OutputLine::new(line.to_string(), style));
        }
        if self.output_lines.len() > Self::OUTPUT_BUFFER_MAX {
            let excess = self.output_lines.len() - Self::OUTPUT_BUFFER_MAX;
            self.output_lines.drain(..excess);
        }
        // Reset scroll to bottom when new output arrives.
        self.output_scroll_offset = 0;
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Renders the full terminal UI into `frame`.
    ///
    /// Layout (top to bottom):
    /// 1. Header bar (1 line)
    /// 2. Output area (fills remaining space)
    /// 3. Mode line (1 line)
    /// 4. Top separator (1 line)
    /// 5. Input line (1 line)
    /// 6. Bottom separator (1 line)
    /// 7. Status bar (1 line)
    /// 8. Slash menu or interactive panel (0–N lines)
    pub fn render(&self, frame: &mut Frame, textarea: &TextArea<'_>) {
        let bottom_height = match &self.panel {
            PanelState::None => {
                let total = self.slash_matches.len();
                let visible = total.min(Self::SLASH_MENU_VISIBLE) as u16;
                // Extra line for the "N/M" indicator when there are more matches
                if total > Self::SLASH_MENU_VISIBLE {
                    visible + 1
                } else {
                    visible
                }
            }
            PanelState::ModelSelector { input_mode, .. } => match input_mode {
                Some(PanelInputMode::AddStep1Provider { .. }) => {
                    // header + empty + N items
                    (PROVIDER_KINDS.len() as u16) + 2
                }
                Some(PanelInputMode::AddStep2Model {
                    provider,
                    manual_input,
                    ..
                }) => {
                    if manual_input.is_some() {
                        // header + empty + input line
                        3
                    } else {
                        let models = recommended_models(provider);
                        // header + empty + N items + empty + [M] hint
                        (models.len() as u16) + 4
                    }
                }
                Some(PanelInputMode::AddStep3Key { .. }) => {
                    // header + empty + model + env + empty + input + empty + hint
                    8
                }
                Some(PanelInputMode::AddStep3Confirm { .. }) => {
                    // title + empty + options
                    3
                }
                _ => {
                    let provider_count = self.config.effective_providers().len().max(1);
                    let input_line = if input_mode.is_some() { 1 } else { 0 };
                    let status_line = match (&self.panel, input_mode) {
                        (
                            PanelState::ModelSelector {
                                status_msg: Some(_),
                                ..
                            },
                            None,
                        ) => 2,
                        _ => 0,
                    };
                    // header + empty + providers + empty + status? + footer
                    (provider_count as u16) + 4 + input_line + status_line
                }
            },
        };
        let has_output = !self.output_lines.is_empty();
        let header_height = if has_output { 0u16 } else { 1u16 };
        let mid_height = if self.show_tip && !has_output {
            5u16 // 1 empty + 3 tip lines + 1 empty
        } else {
            0
        };

        let chunks = Layout::vertical([
            Constraint::Min(0),                // 0 output area
            Constraint::Length(header_height), // 1 header (duumbi line, hidden when output present)
            Constraint::Length(mid_height),    // 2 tip block
            Constraint::Length(1),             // 3 empty spacer above mode line (always)
            Constraint::Length(1),             // 4 mode line
            Constraint::Length(1),             // 5 top separator
            Constraint::Length(1),             // 6 input
            Constraint::Length(1),             // 7 bottom separator
            Constraint::Length(1),             // 8 status bar
            Constraint::Length(bottom_height), // 9 bottom zone (slash menu OR panel)
        ])
        .split(frame.area());

        self.render_output(frame, chunks[0]);
        if header_height > 0 {
            self.render_header(frame, chunks[1]);
        }
        if mid_height > 0 {
            self.render_tip(frame, chunks[2]);
        }
        // chunks[3] is always an empty spacer line — no render needed
        self.render_mode_line(frame, chunks[4]);
        self.render_separator(frame, chunks[5]);
        self.render_input(frame, chunks[6], textarea);
        self.render_separator(frame, chunks[7]);
        self.render_status_bar(frame, chunks[8]);
        if bottom_height > 0 {
            match &self.panel {
                PanelState::None => self.render_slash_menu(frame, chunks[9]),
                PanelState::ModelSelector {
                    selected,
                    input_mode,
                    status_msg,
                } => {
                    self.render_model_panel(frame, chunks[9], *selected, input_mode, status_msg);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Individual render helpers
    // -----------------------------------------------------------------------

    /// Renders the top header bar.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let version = env!("CARGO_PKG_VERSION");

        let line = Line::from(vec![
            Span::styled(
                "duumbi",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" v{version}"),
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::raw(" · Type a request or /help for commands. Ctrl+D to exit."),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    /// Renders the tip block (onboarding hint).
    fn render_tip(&self, frame: &mut Frame, area: Rect) {
        let tip = if !self.has_workspace {
            // No .duumbi/ directory — suggest /init
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Tip: No workspace found. Initialise one with:",
                    Style::default().add_modifier(Modifier::DIM),
                )),
                Line::from(Span::styled("  /init", Style::default().fg(Color::Cyan))),
                Line::from(""),
            ]
        } else {
            // Empty workspace — suggest intent or direct mutation
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Tip: This is an empty workspace. Try one of these:",
                    Style::default().add_modifier(Modifier::DIM),
                )),
                Line::from(Span::styled(
                    "  /intent create  \"Build a calculator with add and multiply\"",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(Span::styled(
                    "  or type a request directly: \"Add a function that adds two numbers\"",
                    Style::default().add_modifier(Modifier::DIM),
                )),
            ]
        };
        frame.render_widget(Paragraph::new(tip).wrap(Wrap { trim: false }), area);
    }

    /// Renders the scrollable output area, bottom-aligned.
    ///
    /// When `output_scroll_offset > 0`, the view shifts upward to show
    /// older lines. PageUp/PageDown control the offset.
    fn render_output(&self, frame: &mut Frame, area: Rect) {
        let max_lines = area.height as usize;
        let total = self.output_lines.len();
        let bottom = total.saturating_sub(self.output_scroll_offset);
        let start = bottom.saturating_sub(max_lines);
        let visible = &self.output_lines[start..bottom];

        // Bottom-align: pad with empty lines above content so messages
        // appear just above the header, close to the prompt.
        let padding = max_lines.saturating_sub(visible.len());
        let mut lines: Vec<Line<'_>> = (0..padding).map(|_| Line::from("")).collect();

        for ol in visible {
            match ol.style {
                OutputStyle::Help => {
                    // Split at column 35: command in magenta, description in white.
                    let text = &ol.text;
                    if text.len() > 35 {
                        let (cmd_part, desc_part) = text.split_at(35);
                        lines.push(Line::from(vec![
                            Span::styled(cmd_part.to_string(), Style::default().fg(Color::Magenta)),
                            Span::styled(desc_part.to_string(), Style::default().fg(Color::White)),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(
                            text.clone(),
                            Style::default().fg(Color::Magenta),
                        )));
                    }
                }
                _ => {
                    let style = match ol.style {
                        OutputStyle::Normal => Style::default().fg(Color::White),
                        OutputStyle::Error => {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        }
                        OutputStyle::Success => Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                        OutputStyle::Dim => Style::default().fg(Color::Gray),
                        OutputStyle::Ai => Style::default().fg(Color::Cyan),
                        OutputStyle::Help => unreachable!(),
                    };
                    lines.push(Line::from(Span::styled(ol.text.clone(), style)));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), area);

        // Scroll indicator overlay when scrolled up.
        if self.output_scroll_offset > 0 {
            let indicator = format!(" \u{2191} {} lines above ", self.output_scroll_offset);
            let style = Style::default().fg(Color::Gray);
            let x = area.right().saturating_sub(indicator.len() as u16);
            let indicator_area = Rect::new(x, area.y, indicator.len() as u16, 1);
            frame.render_widget(
                Paragraph::new(Span::styled(indicator, style)),
                indicator_area,
            );
        }
    }

    /// Renders the mode indicator line below the output area.
    fn render_mode_line(&self, frame: &mut Frame, area: Rect) {
        let hint = Span::styled(
            "Shift+Tab switch mode",
            Style::default().add_modifier(Modifier::DIM),
        );
        let sep = Span::raw("  ");
        let mode_label = self.mode.label();
        let mode_span = Span::styled(mode_label, Style::default().fg(Color::Yellow));

        let right_text = if let Some(ref slug) = self.focused_intent {
            format!("[{slug}]")
        } else {
            String::new()
        };

        let width = area.width as usize;
        let left_len = "Shift+Tab switch mode".len() + 2 + mode_label.len();
        let right_len = right_text.len();
        let padding = width.saturating_sub(left_len + right_len);

        let mut spans = vec![hint, sep, mode_span, Span::raw(" ".repeat(padding))];

        if !right_text.is_empty() {
            spans.push(Span::styled(right_text, Style::default().fg(Color::Cyan)));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Renders a full-width horizontal separator line.
    fn render_separator(&self, frame: &mut Frame, area: Rect) {
        let width = area.width as usize;
        let line_str = "─".repeat(width);
        frame.render_widget(
            Paragraph::new(Span::styled(
                line_str,
                Style::default().add_modifier(Modifier::DIM),
            )),
            area,
        );
    }

    /// Renders the single-line text input with a `❯ ` prefix.
    fn render_input(&self, frame: &mut Frame, area: Rect, textarea: &TextArea<'_>) {
        let chunks = Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).split(area);

        // Chevron prefix
        frame.render_widget(
            Paragraph::new(Span::styled("❯ ", Style::default().fg(Color::Cyan))),
            chunks[0],
        );

        // Textarea (implements Widget)
        frame.render_widget(textarea, chunks[1]);
    }

    /// Renders the bottom status bar with time, workspace path, name, and model.
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let time_str = Local::now().format("%H:%M").to_string();
        let full_path = self
            .workspace_root
            .canonicalize()
            .unwrap_or_else(|_| self.workspace_root.clone())
            .display()
            .to_string();

        let workspace_name = self
            .config
            .workspace
            .as_ref()
            .map(|w| w.name.as_str())
            .unwrap_or("unnamed");

        let model_str = {
            let providers = self.config.effective_providers();
            providers
                .iter()
                .find(|p| p.role == crate::config::ProviderRole::Primary)
                .or(providers.first())
                .map(|p| p.model.clone())
                .unwrap_or_else(|| "no model".to_string())
        };

        let right_str = format!("workspace: {workspace_name}  {model_str}");
        let left_str = format!("{time_str}  {full_path}");
        let width = area.width as usize;
        let padding = width.saturating_sub(left_str.len() + right_str.len());

        let spans: Vec<Span<'_>> = vec![
            Span::styled(
                format!("{time_str}  "),
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::styled(full_path, Style::default().fg(Color::Green)),
            Span::raw(" ".repeat(padding)),
            Span::styled("workspace: ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                workspace_name.to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled(model_str, Style::default().fg(Color::Magenta)),
        ];

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Renders the inline slash-command completion menu with scrolling.
    ///
    /// The selected entry is highlighted with a cyan foreground; all others
    /// use the default terminal style. When the total number of matches
    /// exceeds [`Self::SLASH_MENU_VISIBLE`], a scroll indicator (e.g.
    /// `3/12`) is shown at the bottom.
    fn render_slash_menu(&self, frame: &mut Frame, area: Rect) {
        if self.slash_matches.is_empty() {
            return;
        }

        let total = self.slash_matches.len();
        let visible = total.min(Self::SLASH_MENU_VISIBLE);
        let has_indicator = total > Self::SLASH_MENU_VISIBLE;
        let row_count = if has_indicator { visible + 1 } else { visible };

        let row_areas = Layout::vertical(
            std::iter::repeat_n(Constraint::Length(1), row_count).collect::<Vec<_>>(),
        )
        .split(area);

        let offset = self.slash_scroll_offset;
        for (i, sm) in self
            .slash_matches
            .iter()
            .skip(offset)
            .take(visible)
            .enumerate()
        {
            let abs_index = offset + i;
            let is_selected = abs_index == self.slash_selected;
            let prefix = if is_selected { "> " } else { "  " };
            let text = format!("{prefix}{:<20}  {}", sm.command, sm.description);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::DIM)
            };

            frame.render_widget(Paragraph::new(Span::styled(text, style)), row_areas[i]);
        }

        // Scroll indicator: "  3/12 ↑↓"
        if has_indicator {
            let pos = self.slash_selected + 1;
            let arrows = match (offset > 0, offset + visible < total) {
                (true, true) => " \u{2191}\u{2193}",
                (true, false) => " \u{2191}",
                (false, true) => " \u{2193}",
                (false, false) => "",
            };
            let indicator = format!("  {pos}/{total}{arrows}");
            let style = Style::default().add_modifier(Modifier::DIM);
            frame.render_widget(
                Paragraph::new(Span::styled(indicator, style)),
                row_areas[visible],
            );
        }
    }

    /// Renders the interactive model/provider selector panel.
    fn render_model_panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        selected: usize,
        input_mode: &Option<PanelInputMode>,
        status_msg: &Option<(String, OutputStyle)>,
    ) {
        let providers = self.config.effective_providers();
        let mut lines: Vec<Line<'_>> = Vec::new();

        // Header
        lines.push(Line::from(vec![
            Span::styled(
                "  Select Model",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(area.width.saturating_sub(40) as usize)),
            Span::styled(
                "(Esc to close)",
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]));

        // Empty line
        lines.push(Line::from(""));

        // Provider list
        if providers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No providers configured. Press [A] to add one.",
                Style::default().add_modifier(Modifier::DIM),
            )));
        } else {
            for (i, p) in providers.iter().enumerate() {
                let is_sel = i == selected;
                let prefix = if is_sel { "  \u{25cf} " } else { "    " };
                let key_indicator = if std::env::var(&p.api_key_env).is_ok() {
                    "key (env)"
                } else if self.keychain_cache.contains(&p.api_key_env) {
                    "key (file)"
                } else {
                    "no key"
                };
                let role_str = format!("{:?}", p.role).to_lowercase();

                let text = format!(
                    "{prefix}{}. {:<12} {:<25} ({:<8})  {}",
                    i + 1,
                    format!("{:?}", p.provider).to_lowercase(),
                    p.model,
                    role_str,
                    key_indicator,
                );

                let style = if is_sel {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                };

                lines.push(Line::from(Span::styled(text, style)));
            }
        }

        // Empty line
        lines.push(Line::from(""));

        // Footer / input mode — wizard steps replace the footer entirely.
        match input_mode {
            Some(PanelInputMode::AddProvider(buf)) => {
                lines.push(Line::from(vec![
                    Span::styled("  Add: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{buf}\u{2588}"),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  (provider model api_key_env)",
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                ]));
            }
            Some(PanelInputMode::ConfirmDelete) => {
                lines.push(Line::from(Span::styled(
                    format!("  Delete provider #{}? [y/N]", selected + 1),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )));
            }
            Some(PanelInputMode::AddStep1Provider { selected: step_sel }) => {
                // Replace entire panel with provider selection.
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(
                        "  Add Provider",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" ".repeat(area.width.saturating_sub(45) as usize)),
                    Span::styled(
                        "(Esc to cancel)",
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                ]));
                lines.push(Line::from(""));
                for (i, (name, desc)) in PROVIDER_KINDS.iter().enumerate() {
                    let is_sel = i == *step_sel;
                    let prefix = if is_sel { "  \u{25cf} " } else { "    " };
                    let text = format!("{prefix}{}. {:<14} {}", i + 1, name, desc);
                    let style = if is_sel {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    };
                    lines.push(Line::from(Span::styled(text, style)));
                }
            }
            Some(PanelInputMode::AddStep2Model {
                provider,
                selected: model_sel,
                manual_input,
            }) => {
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Select Model for {provider}"),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" ".repeat(area.width.saturating_sub(55) as usize)),
                    Span::styled(
                        "(Esc to go back)",
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                ]));
                lines.push(Line::from(""));
                if let Some(manual) = manual_input {
                    lines.push(Line::from(vec![
                        Span::styled("  Model: ", Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("{manual}\u{2588}"),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    let models = recommended_models(provider);
                    for (i, (name, desc)) in models.iter().enumerate() {
                        let is_sel = i == *model_sel;
                        let prefix = if is_sel { "  \u{25cf} " } else { "    " };
                        let text = format!("{prefix}{}. {:<30} {}", i + 1, name, desc);
                        let style = if is_sel {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().add_modifier(Modifier::DIM)
                        };
                        lines.push(Line::from(Span::styled(text, style)));
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  [M] Enter model name manually",
                        Style::default().add_modifier(Modifier::DIM),
                    )));
                }
            }
            Some(PanelInputMode::AddStep3Key {
                provider,
                model,
                key_buf,
            }) => {
                let env_name = default_api_key_env(provider);
                let key_set = std::env::var(env_name).is_ok();
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  API Key for {provider}"),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" ".repeat(area.width.saturating_sub(50) as usize)),
                    Span::styled(
                        "(Esc to go back)",
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  Model: {model}"),
                    Style::default().add_modifier(Modifier::DIM),
                )));
                lines.push(Line::from(Span::styled(
                    format!(
                        "  API key env: {env_name}  ({})",
                        if key_set {
                            "\u{2713} already set \u{2014} will reuse"
                        } else {
                            "\u{2717} not set \u{2014} enter key below"
                        }
                    ),
                    Style::default().add_modifier(Modifier::DIM),
                )));
                lines.push(Line::from(""));
                let masked = "\u{25cf}".repeat(key_buf.len());
                lines.push(Line::from(vec![
                    Span::styled("  API key: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{masked}\u{2588}"),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [Enter] Continue  [Esc] Back",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
            Some(PanelInputMode::AddStep3Confirm {
                provider, model, ..
            }) => {
                lines.clear();
                lines.push(Line::from(Span::styled(
                    format!("  Store API key for {provider} ({model})?"),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [K] Save to ~/.duumbi/credentials.toml  [E] Session only  [Esc] Back",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
            None => {
                // Show status message if present (e.g. "Provider added").
                if let Some((msg, style)) = status_msg {
                    let s = match style {
                        OutputStyle::Success => Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                        OutputStyle::Error => {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        }
                        _ => Style::default().add_modifier(Modifier::DIM),
                    };
                    lines.push(Line::from(Span::styled(format!("  {msg}"), s)));
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(Span::styled(
                    "  [A] Add  [D] Delete  [T] Toggle role  [Enter] Select primary",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;

    fn make_app() -> (ReplApp, TextArea<'static>) {
        let tmp = tempfile::TempDir::new().expect("invariant: tempdir");
        let session_mgr =
            SessionManager::load_or_create(tmp.path()).expect("invariant: session manager");
        let app = ReplApp::new(
            DuumbiConfig::default(),
            tmp.path().to_path_buf(),
            None,
            Some(session_mgr),
            true,
            false,
        );
        let textarea = TextArea::default();
        (app, textarea)
    }

    #[test]
    fn new_starts_in_agent_mode() {
        let (app, _) = make_app();
        assert_eq!(app.mode, ReplMode::Agent);
    }

    #[test]
    fn backtab_toggles_mode() {
        let (mut app, mut textarea) = make_app();
        let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        assert_eq!(app.mode, ReplMode::Intent);

        let key2 = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
        app.handle_key(key2, &mut textarea);
        assert_eq!(app.mode, ReplMode::Agent);
    }

    #[test]
    fn ctrl_d_returns_exit() {
        let (mut app, mut textarea) = make_app();
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        let action = app.handle_key(key, &mut textarea);
        assert!(matches!(action, Action::Exit));
    }

    #[test]
    fn empty_enter_is_continue() {
        let (mut app, mut textarea) = make_app();
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = app.handle_key(key, &mut textarea);
        assert!(matches!(action, Action::Continue));
    }

    #[test]
    fn push_output_splits_newlines() {
        let (mut app, _) = make_app();
        app.push_output("line1\nline2\nline3", OutputStyle::Normal);
        assert_eq!(app.output_lines.len(), 3);
        assert_eq!(app.output_lines[0].text, "line1");
        assert_eq!(app.output_lines[2].text, "line3");
    }

    #[test]
    fn update_slash_matches_filters_by_prefix() {
        let (mut app, _) = make_app();
        app.update_slash_matches("/bui");
        assert!(!app.slash_matches.is_empty());
        assert!(app.slash_matches.iter().any(|m| m.command == "/build"));
    }

    #[test]
    fn update_slash_matches_clears_without_slash() {
        let (mut app, _) = make_app();
        app.update_slash_matches("/bui");
        assert!(!app.slash_matches.is_empty());
        app.update_slash_matches("hello");
        assert!(app.slash_matches.is_empty());
    }

    #[test]
    fn slash_menu_collects_all_matches() {
        let (mut app, _) = make_app();
        // "/" matches everything — should return all commands
        app.update_slash_matches("/");
        // There are more than 5 total slash commands
        assert!(app.slash_matches.len() > 5);
    }

    #[test]
    fn slash_menu_scroll_offset_adjusts_on_down() {
        let (mut app, mut textarea) = make_app();
        app.update_slash_matches("/");
        let total = app.slash_matches.len();
        assert!(total > ReplApp::SLASH_MENU_VISIBLE);

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        // Move down past the visible window
        for _ in 0..ReplApp::SLASH_MENU_VISIBLE {
            app.handle_key(down, &mut textarea);
        }
        // Scroll offset should have moved
        assert!(app.slash_scroll_offset > 0);
        // Selected should be at SLASH_MENU_VISIBLE
        assert_eq!(app.slash_selected, ReplApp::SLASH_MENU_VISIBLE);
    }

    #[test]
    fn slash_menu_scroll_offset_adjusts_on_up() {
        let (mut app, mut textarea) = make_app();
        app.update_slash_matches("/");

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        // Move down past the window
        for _ in 0..ReplApp::SLASH_MENU_VISIBLE + 2 {
            app.handle_key(down, &mut textarea);
        }
        let offset_after_down = app.slash_scroll_offset;
        assert!(offset_after_down > 0);

        // Move up back past the window top
        for _ in 0..ReplApp::SLASH_MENU_VISIBLE + 2 {
            app.handle_key(up, &mut textarea);
        }
        assert_eq!(app.slash_selected, 0);
        assert_eq!(app.slash_scroll_offset, 0);
    }

    #[test]
    fn esc_clears_slash_matches() {
        let (mut app, mut textarea) = make_app();
        app.update_slash_matches("/bui");
        assert!(!app.slash_matches.is_empty());

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        assert!(app.slash_matches.is_empty());
    }

    #[test]
    fn down_up_navigation_stays_in_bounds() {
        let (mut app, mut textarea) = make_app();
        app.update_slash_matches("/intent");
        let count = app.slash_matches.len();
        if count < 2 {
            return; // not enough matches to test navigation
        }

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(down, &mut textarea);
        assert_eq!(app.slash_selected, 1);

        // Move up past the beginning — should stay at 0
        let up1 = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        app.handle_key(up1, &mut textarea);
        let up2 = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        app.handle_key(up2, &mut textarea);
        assert_eq!(app.slash_selected, 0);
    }

    #[test]
    fn model_panel_esc_closes_panel() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ModelSelector {
            selected: 0,
            input_mode: None,
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = app.handle_key(key, &mut textarea);
        assert!(matches!(action, Action::Continue));
        assert!(matches!(app.panel, PanelState::None));
    }

    #[test]
    fn model_panel_up_down_stays_in_bounds() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ModelSelector {
            selected: 0,
            input_mode: None,
            status_msg: None,
        };
        // No providers — Down should not panic or change selected.
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(down, &mut textarea);
        // Still 0 because no providers.
        if let PanelState::ModelSelector { selected, .. } = &app.panel {
            assert_eq!(*selected, 0);
        }

        // Up from 0 should also stay at 0.
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        app.handle_key(up, &mut textarea);
        if let PanelState::ModelSelector { selected, .. } = &app.panel {
            assert_eq!(*selected, 0);
        }
    }

    #[test]
    fn model_panel_a_opens_add_provider_mode() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ModelSelector {
            selected: 0,
            input_mode: None,
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        if let PanelState::ModelSelector { input_mode, .. } = &app.panel {
            assert!(matches!(
                input_mode,
                Some(PanelInputMode::AddStep1Provider { .. })
            ));
        }
    }

    #[test]
    fn model_panel_add_provider_esc_cancels() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ModelSelector {
            selected: 0,
            input_mode: Some(PanelInputMode::AddProvider("partial".to_string())),
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        // input_mode cleared, panel still open
        if let PanelState::ModelSelector { input_mode, .. } = &app.panel {
            assert!(input_mode.is_none());
        } else {
            panic!("panel should still be ModelSelector after Esc in AddProvider mode");
        }
    }
}
