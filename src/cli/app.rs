//! Main REPL application struct with ratatui rendering.
//!
//! [`ReplApp`] owns all REPL state and implements the full terminal UI
//! using ratatui. Key handling delegates to `handle_key` which returns an
//! [`Action`] that the event loop acts on.

use std::path::PathBuf;

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui_textarea::{CursorMove, TextArea};

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

/// Returns the conventional environment variable name for subscription/Bearer tokens.
///
/// Used when the user selects subscription-based authentication instead of
/// a traditional API key. The token is sent as `Authorization: Bearer`.
fn default_auth_token_env(kind: &crate::config::ProviderKind) -> &'static str {
    use crate::config::ProviderKind;
    match kind {
        ProviderKind::Anthropic => "ANTHROPIC_AUTH_TOKEN",
        ProviderKind::OpenAI => "OPENAI_AUTH_TOKEN",
        ProviderKind::Grok => "XAI_AUTH_TOKEN",
        ProviderKind::OpenRouter => "OPENROUTER_AUTH_TOKEN",
        ProviderKind::MiniMax => "MINIMAX_AUTH_TOKEN",
    }
}

/// Truncates a path string to fit within `max_chars` columns by inserting
/// a single ellipsis (`…`) into the middle. Returns the original string
/// when it already fits.
///
/// Operates on Unicode scalar values, not bytes, so multi-byte path
/// segments are not split mid-character.
#[must_use]
fn truncate_path(path: &str, max_chars: usize) -> String {
    let total: usize = path.chars().count();
    if total <= max_chars || max_chars < 4 {
        return path.to_string();
    }
    // Reserve one column for the ellipsis; split the remaining budget
    // unevenly to favour the basename (right side).
    let budget = max_chars - 1;
    let right = (budget * 2) / 3;
    let left = budget - right;
    let head: String = path.chars().take(left).collect();
    let tail: String = path.chars().skip(total - right).collect();
    format!("{head}\u{2026}{tail}")
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
use super::theme::tui as theme;

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
    /// True while an async operation (LLM call, build, etc.) is in progress.
    /// Displayed as an animated spinner in the output area.
    pub working: bool,
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
            working: false,
        }
    }

    /// Braille spinner frames for the working indicator (80 ms per frame).
    const SPINNER: &'static [char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    // -----------------------------------------------------------------------
    // Animation
    // -----------------------------------------------------------------------

    /// Returns the current monotonic-ish time in milliseconds since the
    /// Unix epoch. Used as the source of truth for spinner and pulse phase
    /// so that all animated widgets stay in sync within a single frame.
    fn animation_now_ms() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }

    /// Returns the static glyph for the active mode label.
    ///
    /// The dot is drawn solid and unchanging — pulsing felt distracting in
    /// the user's terminal, so the animation was retired. The helper is kept
    /// (rather than inlined) so re-introducing motion later is a one-line
    /// change.
    fn mode_dot_glyph() -> &'static str {
        "\u{25CF}"
    }

    /// Returns `true` whenever the mode-dot animation should drive a redraw.
    /// Static dot now, so this is always `false` — only an active spinner
    /// (`self.working`) keeps the event loop ticking.
    #[must_use]
    pub fn mode_dot_animated(&self) -> bool {
        false
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
                    KeyCode::Up if *step_sel > 0 => {
                        *step_sel -= 1;
                    }
                    KeyCode::Down if *step_sel < PROVIDER_KINDS.len() - 1 => {
                        *step_sel += 1;
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
                                input_mode = Some(PanelInputMode::AddStepAuthType {
                                    provider: provider.clone(),
                                    model,
                                    selected: 0,
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
                            KeyCode::Up if *model_sel > 0 => {
                                *model_sel -= 1;
                            }
                            KeyCode::Down if *model_sel < models.len().saturating_sub(1) => {
                                *model_sel += 1;
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                *manual_input = Some(String::new());
                            }
                            KeyCode::Enter => {
                                if let Some((model_name, _)) = models.get(*model_sel) {
                                    input_mode = Some(PanelInputMode::AddStepAuthType {
                                        provider: provider.clone(),
                                        model: (*model_name).to_string(),
                                        selected: 0,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                PanelInputMode::AddStepAuthType {
                    provider,
                    model,
                    selected: auth_sel,
                } => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = Some(PanelInputMode::AddStep2Model {
                            provider: provider.clone(),
                            selected: 0,
                            manual_input: None,
                        });
                    }
                    KeyCode::Up | KeyCode::Down => {
                        *auth_sel = 1 - *auth_sel;
                    }
                    KeyCode::Enter => {
                        let is_subscription = *auth_sel == 1;
                        input_mode = Some(PanelInputMode::AddStep3Key {
                            provider: provider.clone(),
                            model: model.clone(),
                            key_buf: String::new(),
                            is_subscription,
                        });
                    }
                    _ => {}
                },
                PanelInputMode::AddStep3Key {
                    provider,
                    model,
                    key_buf,
                    is_subscription,
                } => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = Some(PanelInputMode::AddStepAuthType {
                            provider: provider.clone(),
                            model: model.clone(),
                            selected: if *is_subscription { 1 } else { 0 },
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
                            is_subscription: *is_subscription,
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
                    is_subscription,
                } => match key_event.code {
                    KeyCode::Char('k') | KeyCode::Char('K') => {
                        let api_key_env = default_api_key_env(provider).to_string();
                        let token_env = if *is_subscription {
                            Some(default_auth_token_env(provider).to_string())
                        } else {
                            None
                        };
                        let store_env = token_env.as_deref().unwrap_or(&api_key_env);
                        let prov_clone = provider.clone();
                        let model_clone = model.clone();
                        let key_clone = api_key_value.clone();
                        match super::keystore::store_api_key(store_env, &key_clone) {
                            Ok(()) => {
                                // SAFETY: single-threaded CLI — no concurrent env access.
                                unsafe {
                                    std::env::set_var(store_env, &key_clone);
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
                                    auth_token_env: token_env,
                                });
                                self.save_config_and_rebuild_client();
                                selected = self.config.providers.len() - 1;
                                let auth_msg = if *is_subscription {
                                    "Provider added (subscription/Bearer). Token saved to ~/.duumbi/credentials.toml."
                                } else {
                                    "Provider added. Key saved to ~/.duumbi/credentials.toml."
                                };
                                new_status_msg = Some((auth_msg.to_string(), OutputStyle::Success));
                            }
                            Err(e) => {
                                // File storage failed — fall back to session-only.
                                // SAFETY: single-threaded CLI — no concurrent env access.
                                unsafe {
                                    std::env::set_var(store_env, &key_clone);
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
                                    auth_token_env: token_env,
                                });
                                self.save_config_and_rebuild_client();
                                selected = self.config.providers.len() - 1;
                                new_status_msg = Some((
                                    format!("File storage error: {e}. Token set for session only."),
                                    OutputStyle::Error,
                                ));
                            }
                        }
                        input_mode = None; // back to list view
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
                        let api_key_env = default_api_key_env(provider).to_string();
                        let token_env = if *is_subscription {
                            Some(default_auth_token_env(provider).to_string())
                        } else {
                            None
                        };
                        let store_env_name =
                            token_env.as_deref().unwrap_or(&api_key_env).to_string();
                        let prov_clone = provider.clone();
                        let model_clone = model.clone();
                        let key_clone = api_key_value.clone();
                        // SAFETY: single-threaded CLI — no concurrent env access.
                        unsafe {
                            std::env::set_var(&store_env_name, &key_clone);
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
                            auth_token_env: token_env,
                        });
                        self.save_config_and_rebuild_client();
                        selected = self.config.providers.len() - 1;
                        let auth_msg = if *is_subscription {
                            format!(
                                "Provider added (subscription/Bearer, {store_env_name}, session only)."
                            )
                        } else {
                            format!("Provider added ({api_key_env}, session only).")
                        };
                        new_status_msg = Some((auth_msg, OutputStyle::Success));
                        input_mode = None; // back to list view
                    }
                    KeyCode::Esc => {
                        // Back to key entry step with the key pre-filled.
                        input_mode = Some(PanelInputMode::AddStep3Key {
                            provider: provider.clone(),
                            model: model.clone(),
                            key_buf: api_key_value.clone(),
                            is_subscription: *is_subscription,
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
    /// The screen uses an inset page layout: content fills the upper portion,
    /// while a stable footer stack keeps mode, prompt, and status information
    /// anchored near the bottom. Transient menus render as overlays so the
    /// footer does not jump when assistance panels open.
    pub fn render(&self, frame: &mut Frame, textarea: &TextArea<'_>) {
        // Paint the canvas first. Every subsequent widget renders with its
        // own style on top; Spans without an explicit `bg` preserve the
        // canvas colour set here.
        frame.render_widget(Block::default().style(theme::canvas()), frame.area());

        let has_output = !self.output_lines.is_empty();
        let show_card = self.show_tip && !has_output;
        let outer = frame.area();
        let page = Self::page_area(outer);
        let card_height = if show_card {
            if page.width >= 90 { 7u16 } else { 8u16 }
        } else {
            0
        };
        let input_height = if page.width >= 60 { 3u16 } else { 1u16 };

        let chunks = Layout::vertical([
            Constraint::Length(1),            // header
            Constraint::Length(1),            // spacer
            Constraint::Length(card_height),  // empty-state
            Constraint::Min(0),               // conversation
            Constraint::Length(1),            // spacer before controls
            Constraint::Length(1),            // mode row
            Constraint::Length(1),            // separator
            Constraint::Length(input_height), // prompt
            Constraint::Length(1),            // separator
            Constraint::Length(1),            // status row
        ])
        .split(page);

        self.render_brand_header(frame, chunks[0]);
        if card_height > 0 {
            self.render_empty_state_card(frame, chunks[2]);
        }
        self.render_conversation_pane(frame, chunks[3]);
        self.render_mode_strip(frame, chunks[5]);
        self.render_hairline_with_dots(frame, chunks[6]);
        self.render_prompt_well(frame, chunks[7], textarea);
        self.render_hairline_with_dots(frame, chunks[8]);
        self.render_status_dock(frame, chunks[9]);

        if let Some(overlay_height) = self.overlay_height() {
            let overlay_width = match &self.panel {
                PanelState::None => page.width.saturating_sub(6).clamp(40, 96),
                PanelState::ModelSelector { .. } => page.width.saturating_sub(4).clamp(52, 110),
            };
            let overlay_area = Self::overlay_rect(page, chunks[7], overlay_width, overlay_height);
            match &self.panel {
                PanelState::None => self.render_slash_menu(frame, overlay_area),
                PanelState::ModelSelector {
                    selected,
                    input_mode,
                    status_msg,
                } => {
                    self.render_model_panel(frame, overlay_area, *selected, input_mode, status_msg)
                }
            }
        }
    }

    /// Returns the inset content page used for the main layout.
    fn page_area(area: Rect) -> Rect {
        let horizontal = match area.width {
            0..=79 => 1,
            80..=139 => 2,
            _ => 3,
        };
        let vertical = if area.height >= 26 { 1 } else { 0 };
        area.inner(Margin {
            vertical,
            horizontal,
        })
    }

    /// Computes the height of the currently open overlay panel, if any.
    fn overlay_height(&self) -> Option<u16> {
        match &self.panel {
            PanelState::None => {
                let total = self.slash_matches.len();
                if total == 0 {
                    None
                } else {
                    let visible = total.min(Self::SLASH_MENU_VISIBLE) as u16;
                    Some(if total > Self::SLASH_MENU_VISIBLE {
                        visible + 3
                    } else {
                        visible + 2
                    })
                }
            }
            PanelState::ModelSelector { input_mode, .. } => Some(match input_mode {
                Some(PanelInputMode::AddStep1Provider { .. }) => (PROVIDER_KINDS.len() as u16) + 4,
                Some(PanelInputMode::AddStep2Model {
                    provider,
                    manual_input,
                    ..
                }) => {
                    if manual_input.is_some() {
                        5
                    } else {
                        let models = recommended_models(provider);
                        (models.len() as u16) + 6
                    }
                }
                Some(PanelInputMode::AddStepAuthType { .. }) => 8,
                Some(PanelInputMode::AddStep3Key { .. }) => 10,
                Some(PanelInputMode::AddStep3Confirm { .. }) => 5,
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
                    (provider_count as u16) + 6 + input_line + status_line
                }
            }),
        }
    }

    /// Returns an overlay rectangle anchored above the prompt well.
    fn overlay_rect(page: Rect, anchor: Rect, width: u16, height: u16) -> Rect {
        let width = width.min(page.width).max(1);
        let height = height.min(page.height).max(1);
        let x = page.x + page.width.saturating_sub(width) / 2;
        let min_y = page.y.saturating_add(1);
        let preferred_y = anchor.y.saturating_sub(height.saturating_add(1));
        let y = preferred_y.max(min_y);
        Rect::new(x, y, width, height)
    }

    // -----------------------------------------------------------------------
    // Individual render helpers
    // -----------------------------------------------------------------------

    /// REPL-01 — brand header (always visible at the top).
    fn render_brand_header(&self, frame: &mut Frame, area: Rect) {
        let version = env!("CARGO_PKG_VERSION");

        let line = Line::from(vec![
            Span::styled("duumbi", theme::brand_word()),
            Span::raw(" "),
            Span::styled(format!("v{version}"), theme::version_badge()),
            Span::styled("  ·  ", theme::hairline()),
            Span::styled("Type a request or ", theme::dock_value()),
            Span::styled("/help", theme::helper()),
            Span::styled(" for commands. ", theme::dock_value()),
            Span::raw("   "),
            Span::styled(" Ctrl ", theme::keycap()),
            Span::styled(" + ", theme::hairline()),
            Span::styled(" D ", theme::keycap()),
            Span::styled(" to exit.", theme::dock_value()),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    /// REPL-02 — inset empty-state card with examples and stronger onboarding.
    fn render_empty_state_card(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Padding};

        if area.width < 12 || area.height < 5 {
            return;
        }

        let cols = Layout::horizontal([Constraint::Length(1), Constraint::Min(1)]).split(area);
        let accent = cols[0];
        let card_area = cols[1];

        frame.render_widget(Block::default().style(theme::panel_accent_bar()), accent);

        // Only top + bottom borders — the rust pillar on the left already
        // visually anchors the card; a full border made the right edge feel
        // too heavy in the rendered output.
        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(theme::panel_border())
            .padding(Padding::new(2, 2, 0, 0))
            .style(theme::panel_surface());
        let inner = block.inner(card_area);
        frame.render_widget(block, card_area);

        let (badge_text, heading, left_lines, right_lines) = if !self.has_workspace {
            (
                " no workspace ",
                "INITIALISE A WORKSPACE",
                vec![
                    Line::from(vec![
                        Span::styled("/init", theme::brand_word()),
                        Span::styled("  create a new DUUMBI workspace here", theme::dock_value()),
                    ]),
                    Line::from(vec![Span::styled(
                        "or just run `duumbi init` in this folder",
                        theme::out_dim(),
                    )]),
                ],
                vec![Line::from(vec![Span::styled(
                    "Once initialised, natural language requests work directly in the prompt.",
                    theme::out_dim(),
                )])],
            )
        } else {
            (
                " empty workspace ",
                "TRY ONE OF THESE",
                vec![
                    Line::from(vec![
                        Span::styled("/intent create", theme::brand_word()),
                        Span::raw("  "),
                        Span::styled(
                            "\"Build a calculator with add and multiply\"",
                            theme::dock_value(),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("or just type", theme::out_dim()),
                        Span::raw("  "),
                        Span::styled(
                            "\"Add a function that adds two numbers\"",
                            theme::dock_value(),
                        ),
                        Span::styled("  natural language works too", theme::out_dim()),
                    ]),
                ],
                vec![Line::from(vec![Span::styled(
                    "Use the prompt below for direct requests, or switch to intent mode when you want focused planning.",
                    theme::out_dim(),
                )])],
            )
        };

        if inner.width >= 70 {
            let rows = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);
            let header = Line::from(vec![
                Span::styled(badge_text, theme::pill_outline()),
                Span::raw("  "),
                Span::styled(heading, theme::label_caps()),
            ]);
            frame.render_widget(Paragraph::new(header), rows[0]);

            let cols = Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)])
                .split(rows[2]);
            frame.render_widget(
                Paragraph::new(left_lines).wrap(Wrap { trim: false }),
                cols[0],
            );
            frame.render_widget(
                Paragraph::new(right_lines).wrap(Wrap { trim: false }),
                cols[1],
            );
        } else {
            let body = vec![
                Line::from(vec![
                    Span::styled(badge_text, theme::pill_outline()),
                    Span::raw("  "),
                    Span::styled(heading, theme::label_caps()),
                ]),
                Line::from(""),
                left_lines[0].clone(),
                left_lines.get(1).cloned().unwrap_or_else(|| Line::from("")),
                Line::from(""),
                right_lines[0].clone(),
            ];
            frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), inner);
        }
    }

    /// REPL-03 — scrollable conversation pane, bottom-aligned.
    ///
    /// When `output_scroll_offset > 0`, the view shifts upward to show
    /// older lines. PageUp/PageDown control the offset.
    fn render_conversation_pane(&self, frame: &mut Frame, area: Rect) {
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
                    // Split at column 35: command in rust, description in parchment.
                    let text = &ol.text;
                    if text.len() > 35 {
                        let (cmd_part, desc_part) = text.split_at(35);
                        lines.push(Line::from(vec![
                            Span::styled(cmd_part.to_string(), theme::out_help_cmd()),
                            Span::styled(desc_part.to_string(), theme::out_help_desc()),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(
                            text.clone(),
                            theme::out_help_cmd(),
                        )));
                    }
                }
                _ => {
                    let style = match ol.style {
                        OutputStyle::Normal => theme::out_normal(),
                        OutputStyle::Error => theme::out_error(),
                        OutputStyle::Success => theme::out_success(),
                        OutputStyle::Dim => theme::out_dim(),
                        OutputStyle::Ai => theme::out_ai(),
                        OutputStyle::Help => unreachable!(),
                    };
                    lines.push(Line::from(Span::styled(ol.text.clone(), style)));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
        // The "Working…" spinner now lives in the status dock activity slot
        // (see render_activity_button). No inline spinner overlay here.

        // Scroll indicator overlay when scrolled up.
        if self.output_scroll_offset > 0 {
            let indicator = format!(" \u{2191} {} lines above ", self.output_scroll_offset);
            let x = area.right().saturating_sub(indicator.len() as u16);
            let indicator_area = Rect::new(x, area.y, indicator.len() as u16, 1);
            frame.render_widget(
                Paragraph::new(Span::styled(indicator, theme::out_dim())),
                indicator_area,
            );
        }
    }

    /// REPL-06 — mode strip with shortcut hint, active mode pill, and intent label.
    fn render_mode_strip(&self, frame: &mut Frame, area: Rect) {
        let dot_glyph = Self::mode_dot_glyph();
        let mode_label = self.mode.label();

        // Right-side intent indicator: "intent —" (empty) or "intent [slug]".
        let intent_label = "intent ";
        let intent_value: String = self
            .focused_intent
            .as_deref()
            .map(|s| format!("[{s}]"))
            .unwrap_or_else(|| "—".to_string());

        let hint_prefix = 14usize;
        let mode_len = 4 + mode_label.len();
        let right_len = intent_label.len() + intent_value.chars().count();
        let left_len = hint_prefix + mode_len;
        let padding = (area.width as usize).saturating_sub(left_len + right_len);

        let mut spans = vec![
            Span::styled(" Shift ", theme::keycap()),
            Span::raw(" "),
            Span::styled(" Tab ", theme::keycap()),
            Span::styled(" switch mode  ", theme::mode_hint()),
            Span::styled(format!(" {dot_glyph} {mode_label} "), theme::mode_pill()),
            Span::raw("  "),
        ];
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(intent_label, theme::label_caps_inline()));
        if self.focused_intent.is_some() {
            spans.push(Span::styled(intent_value, theme::intent_slug()));
        } else {
            spans.push(Span::styled(intent_value, theme::out_dim()));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// REPL-07 / REPL-09 — hairline separator with rust dots at both edges.
    ///
    /// On terminals narrower than 4 columns, falls back to a plain dim line.
    fn render_hairline_with_dots(&self, frame: &mut Frame, area: Rect) {
        let width = area.width as usize;
        if width < 4 {
            let line_str = "─".repeat(width);
            frame.render_widget(
                Paragraph::new(Span::styled(line_str, theme::hairline_dim())),
                area,
            );
            return;
        }
        // Body: dim hairline between the two rust dots.
        let body = "─".repeat(width.saturating_sub(2));
        let line = Line::from(vec![
            Span::styled("\u{2022}", theme::chevron()),
            Span::styled(body, theme::hairline_dim()),
            Span::styled("\u{2022}", theme::chevron()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    /// REPL-08 — prompt input well with rust focus ring and placeholder.
    ///
    /// On terminals < 60 cols, the focus border is dropped and the prompt
    /// renders as a single line with a `› ` prefix.
    fn render_prompt_well(&self, frame: &mut Frame, area: Rect, textarea: &TextArea<'_>) {
        use ratatui::widgets::{Block, BorderType, Borders};

        let is_empty = textarea.lines().iter().all(|line| line.is_empty());
        let placeholder = match self.mode {
            ReplMode::Agent => "e.g. \"create a module that parses CSV\" or /help",
            ReplMode::Intent => "e.g. \"plan a calculator module\" or /intent create",
        };

        if area.height >= 3 {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(theme::focus_border())
                .style(theme::panel_surface());
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let chunks =
                Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).split(inner);
            frame.render_widget(
                Paragraph::new(Span::styled("\u{203A} ", theme::chevron())),
                chunks[0],
            );
            frame.render_widget(textarea, chunks[1]);
            if is_empty {
                frame.render_widget(
                    Paragraph::new(Span::styled(placeholder, theme::placeholder())),
                    chunks[1],
                );
            }
        } else {
            let chunks =
                Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).split(area);
            frame.render_widget(
                Paragraph::new(Span::styled("\u{203A} ", theme::chevron())),
                chunks[0],
            );
            frame.render_widget(textarea, chunks[1]);
            if is_empty {
                frame.render_widget(
                    Paragraph::new(Span::styled(placeholder, theme::placeholder())),
                    chunks[1],
                );
            }
        }
    }

    /// REPL-10 — compact single-row status dock.
    fn render_status_dock(&self, frame: &mut Frame, area: Rect) {
        if area.width < 40 {
            self.render_compact_status(frame, area);
            return;
        }
        let time_str = Local::now().format("%H:%M").to_string();
        let workspace_name = self
            .config
            .workspace
            .as_ref()
            .map(|w| w.name.as_str())
            .unwrap_or("unnamed");
        let cwd = self
            .workspace_root
            .canonicalize()
            .unwrap_or_else(|_| self.workspace_root.clone())
            .display()
            .to_string();
        // Lowercase labels feel "smaller" than uppercase in monospace fonts;
        // paired with the dim hairline colour they recede visually so the
        // values (workspace name, time) remain the read targets.
        let lbl_time = "time";
        let lbl_ws = "workspace";
        let lbl_cwd = "cwd";
        let lbl_activity = "activity";
        let prefix_len = lbl_time.len()
            + 1
            + time_str.len()
            + 3
            + lbl_ws.len()
            + 1
            + workspace_name.len()
            + 3
            + lbl_cwd.len()
            + 1;
        let activity_len = if self.working {
            lbl_activity.len() + 1 + 6 + 3
        } else {
            0
        };
        let cwd_budget = area
            .width
            .saturating_sub((prefix_len + activity_len) as u16)
            .max(12) as usize;
        let cwd_truncated = truncate_path(&cwd, cwd_budget);

        let mut spans = vec![
            Span::styled(format!("{lbl_time} "), theme::label_caps_inline()),
            Span::styled(time_str, theme::dock_value_muted()),
            Span::raw("   "),
            Span::styled(format!("{lbl_ws} "), theme::label_caps_inline()),
            Span::styled(workspace_name.to_string(), theme::workspace_value()),
            Span::raw("   "),
            Span::styled(format!("{lbl_cwd} "), theme::label_caps_inline()),
            Span::styled(cwd_truncated, theme::dock_value_muted()),
        ];
        if self.working {
            let frame_idx = (Self::animation_now_ms() / 80) as usize;
            let glyph = Self::SPINNER[frame_idx % Self::SPINNER.len()];
            spans.push(Span::raw("   "));
            spans.push(Span::styled(
                format!("{lbl_activity} "),
                theme::label_caps_inline(),
            ));
            spans.push(Span::styled(format!("{glyph} work"), theme::chevron()));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Compact single-row status fallback for narrow terminals.
    fn render_compact_status(&self, frame: &mut Frame, area: Rect) {
        let time_str = Local::now().format("%H:%M").to_string();
        let workspace_name = self
            .config
            .workspace
            .as_ref()
            .map(|w| w.name.as_str())
            .unwrap_or("unnamed");
        let activity = if self.working { "  work" } else { "" };
        let line = Line::from(vec![
            Span::styled("time ", theme::label_caps_inline()),
            Span::styled(time_str, theme::dock_value_muted()),
            Span::raw("   "),
            Span::styled("workspace ", theme::label_caps_inline()),
            Span::styled(workspace_name.to_string(), theme::workspace_value()),
            Span::styled(activity.to_string(), theme::out_dim()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    /// Renders the inline slash-command completion menu as an overlay.
    ///
    /// The selected entry is highlighted with a cyan foreground; all others
    /// use the default terminal style. When the total number of matches
    /// exceeds [`Self::SLASH_MENU_VISIBLE`], a scroll indicator (e.g.
    /// `3/12`) is shown at the bottom.
    fn render_slash_menu(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Padding};

        if self.slash_matches.is_empty() {
            return;
        }

        frame.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border())
            .padding(Padding::new(1, 1, 1, 1))
            .style(theme::panel_surface());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let total = self.slash_matches.len();
        let visible = total.min(Self::SLASH_MENU_VISIBLE);
        let has_indicator = total > Self::SLASH_MENU_VISIBLE;
        let row_count = if has_indicator { visible + 1 } else { visible };

        let row_areas = Layout::vertical(
            std::iter::repeat_n(Constraint::Length(1), row_count).collect::<Vec<_>>(),
        )
        .split(inner);

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
                theme::slash_selected()
            } else {
                theme::out_dim()
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
            let style = theme::out_dim();
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
        use ratatui::widgets::{Block, Borders, Padding};

        frame.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border())
            .padding(Padding::new(1, 1, 1, 1))
            .style(theme::panel_surface());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let providers = self.config.effective_providers();
        let mut lines: Vec<Line<'_>> = Vec::new();

        let inner_width = inner.width as usize;

        // Header
        lines.push(Line::from(vec![
            Span::styled("  Select Model", theme::brand_word()),
            Span::raw(" ".repeat(inner_width.saturating_sub(40))),
            Span::styled("(Esc to close)", theme::out_dim()),
        ]));

        // Empty line
        lines.push(Line::from(""));

        // Provider list
        if providers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No providers configured. Press [A] to add one.",
                theme::out_dim(),
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
                    theme::slash_selected()
                } else {
                    theme::out_dim()
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
                    Span::styled("  Add: ", theme::out_help_cmd()),
                    Span::styled(format!("{buf}\u{2588}"), theme::brand_word()),
                    Span::styled("  (provider model api_key_env)", theme::out_dim()),
                ]));
            }
            Some(PanelInputMode::ConfirmDelete) => {
                lines.push(Line::from(Span::styled(
                    format!("  Delete provider #{}? [y/N]", selected + 1),
                    theme::out_error(),
                )));
            }
            Some(PanelInputMode::AddStep1Provider { selected: step_sel }) => {
                // Replace entire panel with provider selection.
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled("  Add Provider", theme::brand_word()),
                    Span::raw(" ".repeat(inner_width.saturating_sub(45))),
                    Span::styled("(Esc to cancel)", theme::out_dim()),
                ]));
                lines.push(Line::from(""));
                for (i, (name, desc)) in PROVIDER_KINDS.iter().enumerate() {
                    let is_sel = i == *step_sel;
                    let prefix = if is_sel { "  \u{25cf} " } else { "    " };
                    let text = format!("{prefix}{}. {:<14} {}", i + 1, name, desc);
                    let style = if is_sel {
                        theme::slash_selected()
                    } else {
                        theme::out_dim()
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
                        theme::brand_word(),
                    ),
                    Span::raw(" ".repeat(inner_width.saturating_sub(55))),
                    Span::styled("(Esc to go back)", theme::out_dim()),
                ]));
                lines.push(Line::from(""));
                if let Some(manual) = manual_input {
                    lines.push(Line::from(vec![
                        Span::styled("  Model: ", theme::out_help_cmd()),
                        Span::styled(format!("{manual}\u{2588}"), theme::brand_word()),
                    ]));
                } else {
                    let models = recommended_models(provider);
                    for (i, (name, desc)) in models.iter().enumerate() {
                        let is_sel = i == *model_sel;
                        let prefix = if is_sel { "  \u{25cf} " } else { "    " };
                        let text = format!("{prefix}{}. {:<30} {}", i + 1, name, desc);
                        let style = if is_sel {
                            theme::slash_selected()
                        } else {
                            theme::out_dim()
                        };
                        lines.push(Line::from(Span::styled(text, style)));
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  [M] Enter model name manually",
                        theme::out_dim(),
                    )));
                }
            }
            Some(PanelInputMode::AddStepAuthType {
                provider,
                model,
                selected: auth_sel,
            }) => {
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Authentication for {provider} ({model})"),
                        theme::brand_word(),
                    ),
                    Span::raw(" ".repeat(inner_width.saturating_sub(60))),
                    Span::styled("(Esc to go back)", theme::out_dim()),
                ]));
                lines.push(Line::from(""));
                let options = [
                    ("API Key", "Traditional API key (X-Api-Key header)"),
                    (
                        "Subscription Token",
                        "Claude Pro/Max or OAuth token (Bearer header, via `claude setup-token`)",
                    ),
                ];
                for (i, (label, hint)) in options.iter().enumerate() {
                    let marker = if i == *auth_sel { "> " } else { "  " };
                    let style = if i == *auth_sel {
                        theme::slash_selected()
                    } else {
                        theme::out_dim()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {marker}{label}"), style),
                        Span::styled(format!("  \u{2014} {hint}"), theme::out_dim()),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [\u{2191}/\u{2193}] Select  [Enter] Continue  [Esc] Back",
                    theme::out_dim(),
                )));
            }
            Some(PanelInputMode::AddStep3Key {
                provider,
                model,
                key_buf,
                is_subscription,
            }) => {
                let (env_name, label) = if *is_subscription {
                    (default_auth_token_env(provider), "Subscription token")
                } else {
                    (default_api_key_env(provider), "API key")
                };
                let key_set = std::env::var(env_name).is_ok();
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(format!("  {label} for {provider}"), theme::brand_word()),
                    Span::raw(" ".repeat(inner_width.saturating_sub(50))),
                    Span::styled("(Esc to go back)", theme::out_dim()),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  Model: {model}"),
                    theme::out_dim(),
                )));
                let hint = if *is_subscription {
                    "  Tip: generate a token with `claude setup-token`"
                } else {
                    ""
                };
                lines.push(Line::from(Span::styled(
                    format!(
                        "  {label} env: {env_name}  ({})",
                        if key_set {
                            "\u{2713} already set \u{2014} will reuse"
                        } else {
                            "\u{2717} not set \u{2014} enter below"
                        }
                    ),
                    theme::out_dim(),
                )));
                if !hint.is_empty() {
                    lines.push(Line::from(Span::styled(hint, theme::label_caps())));
                }
                lines.push(Line::from(""));
                let masked = "\u{25cf}".repeat(key_buf.len());
                lines.push(Line::from(vec![
                    Span::styled(format!("  {label}: "), theme::out_help_cmd()),
                    Span::styled(format!("{masked}\u{2588}"), theme::brand_word()),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [Enter] Continue  [Esc] Back",
                    theme::out_dim(),
                )));
            }
            Some(PanelInputMode::AddStep3Confirm {
                provider,
                model,
                is_subscription,
                ..
            }) => {
                let label = if *is_subscription {
                    "subscription token"
                } else {
                    "API key"
                };
                lines.clear();
                lines.push(Line::from(Span::styled(
                    format!("  Store {label} for {provider} ({model})?"),
                    theme::brand_word(),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [K] Save to ~/.duumbi/credentials.toml  [E] Session only  [Esc] Back",
                    theme::out_dim(),
                )));
            }
            None => {
                // Show status message if present (e.g. "Provider added").
                if let Some((msg, style)) = status_msg {
                    let s = match style {
                        OutputStyle::Success => theme::out_success(),
                        OutputStyle::Error => theme::out_error(),
                        _ => theme::out_dim(),
                    };
                    lines.push(Line::from(Span::styled(format!("  {msg}"), s)));
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(Span::styled(
                    "  [A] Add  [D] Delete  [T] Toggle role  [Enter] Select primary",
                    theme::out_dim(),
                )));
            }
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
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
    fn truncate_path_returns_input_when_short() {
        assert_eq!(truncate_path("/a/b", 80), "/a/b");
    }

    #[test]
    fn truncate_path_inserts_ellipsis() {
        let p = "/Users/foo/space/hgahub/duumbi-cli-ux/target/debug/duumbi-binary";
        let out = truncate_path(p, 30);
        assert!(out.chars().count() <= 30);
        assert!(out.contains('\u{2026}'));
        // Tail should be preserved (it carries the most identifying suffix).
        assert!(out.ends_with("ary"));
    }

    #[test]
    fn truncate_path_handles_unicode() {
        let p = "/föö/bär/baz/qüüx";
        let out = truncate_path(p, 10);
        assert!(out.chars().count() <= 10);
        assert!(out.contains('\u{2026}'));
    }

    #[test]
    fn mode_dot_glyph_returns_solid_circle() {
        // Static dot now; only the solid bullet (U+25CF) is allowed.
        assert_eq!(ReplApp::mode_dot_glyph(), "\u{25CF}");
    }

    /// Returns the full buffer content for a rendered frame as a single
    /// string (cells joined row by row, no separator). Used by the status
    /// dock render tests to assert what the user actually sees.
    fn render_to_string(width: u16, height: u16) -> (String, Vec<String>) {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let (mut app, textarea) = make_app();
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("invariant: test terminal");
        terminal
            .draw(|frame| app.render(frame, &textarea))
            .expect("invariant: draw");
        let cells = terminal.backend().buffer().content();
        let joined: String = cells.iter().map(|c| c.symbol()).collect();
        // Split by width to also get per-row dump for debugging.
        let rows: Vec<String> = (0..height as usize)
            .map(|r| {
                cells[r * width as usize..(r + 1) * width as usize]
                    .iter()
                    .map(|c| c.symbol())
                    .collect()
            })
            .collect();
        let _ = &mut app;
        let _ = &textarea;
        (joined, rows)
    }

    #[test]
    fn full_render_draws_status_dock_labels_at_30_rows() {
        let (_buf, rows) = render_to_string(120, 30);
        let last_rows = rows.iter().rev().take(4).cloned().collect::<Vec<_>>();
        let last_chunk = last_rows.join("\n");
        assert!(
            last_chunk.contains("time") && last_chunk.contains("workspace"),
            "expected status-dock labels in the last rows; got:\n{last_chunk}"
        );
    }

    #[test]
    fn full_render_draws_status_dock_labels_at_small_heights() {
        // The status dock must render even when the terminal is short,
        // because the Min(0) conversation pane should collapse first.
        for h in [20u16, 22, 24, 28] {
            let (_buf, rows) = render_to_string(120, h);
            let last_rows = rows.iter().rev().take(5).cloned().collect::<Vec<_>>();
            let last_chunk = last_rows.join("\n");
            assert!(
                last_chunk.contains("time") && last_chunk.contains("workspace"),
                "h={h}: status-dock missing from last rows:\n{last_chunk}"
            );
        }
    }

    #[test]
    fn full_render_draws_status_dock_labels() {
        // Render a full frame into an in-memory buffer and verify the
        // lowercase status-dock labels actually reach the buffer. If this
        // ever fails silently, the status row has fallen out of the layout
        // (e.g. from a bad constraint total or an errant early return).
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let (mut app, mut textarea) = make_app();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("invariant: test terminal");
        terminal
            .draw(|frame| app.render(frame, &textarea))
            .expect("invariant: draw");

        let buf_dump = terminal.backend().buffer().content();
        let rendered: String = buf_dump.iter().map(|c| c.symbol()).collect();

        assert!(
            rendered.contains("time"),
            "status dock should render 'time' label; buffer:\n{rendered}"
        );
        assert!(
            rendered.contains("workspace"),
            "status dock should render 'workspace' label"
        );
        assert!(
            rendered.contains("cwd"),
            "status dock should render 'cwd' label"
        );

        // Unused to silence the mut warning in case the test harness evolves.
        let _ = &mut textarea;
        let _ = &mut app;
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
