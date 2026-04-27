//! Main REPL application struct with ratatui rendering.
//!
//! [`ReplApp`] owns all REPL state and implements the full terminal UI
//! using ratatui. Key handling delegates to `handle_key` which returns an
//! [`Action`] that the event loop acts on.

use std::cell::Cell;
use std::collections::HashSet;
use std::path::PathBuf;

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui_textarea::{CursorMove, TextArea};

use crate::agents::LlmClient;
use crate::config::{DuumbiConfig, ProviderConfigSource};
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

/// Authentication modes the provider setup TUI can actually configure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderAuthMode {
    /// Direct provider API key stored in an env var or local credentials file.
    ApiKey,
}

const API_KEY_AUTH_MODES: &[ProviderAuthMode] = &[ProviderAuthMode::ApiKey];

impl ProviderAuthMode {
    /// Returns the short label shown in the provider list.
    const fn summary(self) -> &'static str {
        match self {
            Self::ApiKey => "api key",
        }
    }

    /// Returns the title shown in the auth chooser.
    const fn title(self) -> &'static str {
        match self {
            Self::ApiKey => "API Key",
        }
    }

    /// Returns the explanatory text shown in the auth chooser.
    const fn hint(self) -> &'static str {
        match self {
            Self::ApiKey => "Direct provider API key",
        }
    }

    /// Returns whether this mode stores a bearer/subscription token.
    const fn is_subscription(self) -> bool {
        match self {
            Self::ApiKey => false,
        }
    }
}

/// Returns setup modes that are implemented for this direct provider.
fn provider_auth_modes(_kind: &crate::config::ProviderKind) -> &'static [ProviderAuthMode] {
    API_KEY_AUTH_MODES
}

/// Returns a compact list label for configured setup modes.
fn provider_auth_summary(kind: &crate::config::ProviderKind) -> String {
    provider_auth_modes(kind)
        .iter()
        .map(|mode| mode.summary())
        .collect::<Vec<_>>()
        .join("/")
}

/// Returns the selected auth mode, clamped to the provider capability table.
fn provider_auth_mode_by_index(
    kind: &crate::config::ProviderKind,
    selected: usize,
) -> Option<ProviderAuthMode> {
    provider_auth_modes(kind).get(selected).copied()
}

/// Starts the setup wizard for a provider from the main provider list.
fn provider_setup_mode(kind: crate::config::ProviderKind) -> PanelInputMode {
    let modes = provider_auth_modes(&kind);
    if modes.len() == 1 {
        PanelInputMode::AddStep2Model {
            provider: kind,
            is_subscription: modes[0].is_subscription(),
            selected: 0,
            manual_input: None,
        }
    } else {
        PanelInputMode::AddStepAuthType {
            provider: kind,
            selected: 0,
        }
    }
}

/// Returns a short provider label for the provider manager.
fn provider_kind_label(kind: &crate::config::ProviderKind) -> &'static str {
    use crate::config::ProviderKind;
    match kind {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::OpenAI => "openai",
        ProviderKind::Grok => "grok",
        ProviderKind::OpenRouter => "openrouter",
        ProviderKind::MiniMax => "minimax",
    }
}

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

/// Finds the configured provider entry for a provider kind.
fn configured_provider_index(
    config: &DuumbiConfig,
    kind: &crate::config::ProviderKind,
) -> Option<usize> {
    config.providers.iter().position(|p| &p.provider == kind)
}

use super::completion::{SLASH_COMMANDS, SLASH_GROUPS, SlashGroup};
use super::mode::{
    Action, ConversationAction, ConversationBlock, ConversationBlockKind, OutputLine, OutputStyle,
    PanelInputMode, PanelState, ReplMode, SlashMatch, SlashMenuItem,
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

#[derive(Debug, Clone)]
struct ConversationVisualRow {
    line: Line<'static>,
    plain_text: String,
    block_index: Option<usize>,
    menu_button_block: Option<usize>,
    menu_button_range: Option<(u16, u16)>,
}

#[derive(Debug, Clone)]
struct VisibleConversationLayout {
    start_index: usize,
    padding: usize,
    rows: Vec<ConversationVisualRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConversationActionMenu {
    block_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConversationSelectionPoint {
    row: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConversationTextSelection {
    anchor: ConversationSelectionPoint,
    focus: ConversationSelectionPoint,
    dragged: bool,
}

fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| "clipboard stdin closed".to_string())?;
    std::io::Write::write_all(stdin, text.as_bytes()).map_err(|e| e.to_string())?;

    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(status.to_string())
    }
}

fn read_text_from_clipboard() -> Result<String, String> {
    let output = std::process::Command::new("pbpaste")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(output.status.to_string());
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

fn is_clipboard_shortcut(modifiers: KeyModifiers) -> bool {
    modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER | KeyModifiers::META)
}

fn append_single_line_input(buf: &mut String, text: &str) {
    buf.extend(text.chars().filter(|c| !matches!(c, '\r' | '\n')));
}

fn validate_provider_key(key: &str) -> Result<(), &'static str> {
    if key.trim().chars().count() < 8 {
        return Err("API key looks too short.");
    }
    Ok(())
}

fn masked_secret_preview(char_count: usize, max_width: usize) -> String {
    if char_count == 0 {
        return "\u{2588}".to_string();
    }

    let suffix = format!(" ({char_count} chars)");
    let suffix_width = suffix.chars().count() + 1; // plus cursor
    if max_width > suffix_width {
        let dot_count = char_count.min(max_width - suffix_width).max(1);
        format!("{}{}{}", ".".repeat(dot_count), suffix, "\u{2588}")
    } else {
        let dot_count = char_count.min(max_width.saturating_sub(1)).max(1);
        format!("{}{}", ".".repeat(dot_count), "\u{2588}")
    }
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
    /// User-level configuration from `~/.duumbi/config.toml`.
    pub user_config: DuumbiConfig,
    /// Workspace-level configuration from `<workspace>/.duumbi/config.toml`.
    pub workspace_config: DuumbiConfig,
    /// Source layer that provides the active provider settings.
    pub provider_config_source: ProviderConfigSource,
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
    /// Scrollable conversation blocks rendered in the main pane.
    pub conversation_blocks: Vec<ConversationBlock>,
    /// Index of the active output block receiving `push_output` lines.
    current_output_block: Option<usize>,
    /// User block selected by clicking inside the conversation pane.
    selected_conversation_block: Option<usize>,
    /// Open action menu for a selected conversation user block.
    conversation_action_menu: Option<ConversationActionMenu>,
    /// App-managed text selection inside the conversation pane.
    conversation_text_selection: Option<ConversationTextSelection>,
    /// Selected row inside the open conversation action menu.
    conversation_action_selected: usize,
    /// Last rendered conversation pane rectangle, used for mouse hit testing.
    last_conversation_area: Cell<Option<Rect>>,
    /// Scroll offset for the output area (lines from the bottom; 0 = latest).
    pub output_scroll_offset: usize,
    /// All matching slash-command entries (untruncated).
    pub slash_matches: Vec<SlashMatch>,
    /// Rows currently visible in the slash menu.
    pub slash_rows: Vec<SlashMenuItem>,
    /// Current slash-menu filter text.
    pub slash_filter: String,
    /// Expanded discovery groups for the current slash-menu session.
    pub slash_expanded_groups: HashSet<SlashGroup>,
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
    /// Preferred minimum number of slash-menu rows when the terminal has room.
    const SLASH_MENU_VISIBLE: usize = 7;

    /// Maximum number of slash-menu rows visible at once.
    const SLASH_MENU_VISIBLE_MAX: usize = 12;

    /// Maximum number of lines kept in the output buffer.
    const OUTPUT_BUFFER_MAX: usize = 10_000;

    /// Creates a new `ReplApp` with the given workspace context.
    #[must_use]
    #[allow(dead_code)]
    pub fn new(
        config: DuumbiConfig,
        workspace_root: PathBuf,
        client: Option<LlmClient>,
        session_mgr: Option<SessionManager>,
        has_workspace: bool,
        show_tip: bool,
    ) -> Self {
        Self::new_with_config_layers(
            config.clone(),
            DuumbiConfig::default(),
            config,
            ProviderConfigSource::Workspace,
            workspace_root,
            client,
            session_mgr,
            has_workspace,
            show_tip,
        )
    }

    /// Creates a new `ReplApp` with explicit config layer metadata.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_config_layers(
        config: DuumbiConfig,
        user_config: DuumbiConfig,
        workspace_config: DuumbiConfig,
        provider_config_source: ProviderConfigSource,
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
            user_config,
            workspace_config,
            provider_config_source,
            client,
            history: Vec::new(),
            session_mgr,
            has_workspace,
            output_lines: Vec::new(),
            conversation_blocks: Vec::new(),
            current_output_block: None,
            selected_conversation_block: None,
            conversation_action_menu: None,
            conversation_text_selection: None,
            conversation_action_selected: 0,
            last_conversation_area: Cell::new(None),
            output_scroll_offset: 0,
            slash_matches: Vec::new(),
            slash_rows: Vec::new(),
            slash_filter: String::new(),
            slash_expanded_groups: HashSet::new(),
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
        if matches!(self.panel, PanelState::ProviderManager { .. }) {
            return self.handle_provider_panel_key(key, textarea);
        }

        if self.conversation_action_menu.is_some() {
            return self.handle_conversation_action_menu_key(key);
        }

        match key.code {
            KeyCode::Esc if self.conversation_text_selection.take().is_some() => Action::Continue,

            // Shift+Tab: toggle Agent ↔ Intent mode
            KeyCode::BackTab => {
                self.mode = match self.mode {
                    ReplMode::Agent => ReplMode::Intent,
                    ReplMode::Intent => ReplMode::Agent,
                };
                Action::Continue
            }

            // Enter: execute selected slash command, expand discovery groups, or submit input.
            KeyCode::Enter => {
                if let Some(action) = self.activate_selected_slash_row(textarea) {
                    return action;
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
            KeyCode::Up if !self.slash_rows.is_empty() => {
                if self.slash_selected > 0 {
                    self.slash_selected -= 1;
                    if self.slash_selected < self.slash_scroll_offset {
                        self.slash_scroll_offset = self.slash_selected;
                    }
                }
                Action::Continue
            }

            // Down: move slash menu selection down (scrolls window)
            KeyCode::Down if !self.slash_rows.is_empty() => {
                let max = self.slash_rows.len().saturating_sub(1);
                if self.slash_selected < max {
                    self.slash_selected += 1;
                    let visible = Self::SLASH_MENU_VISIBLE;
                    if self.slash_selected >= self.slash_scroll_offset + visible {
                        self.slash_scroll_offset = self.slash_selected + 1 - visible;
                    }
                }
                Action::Continue
            }

            // Ctrl+Up/Ctrl+Down: move conversation block selection without mouse capture.
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.select_previous_user_block();
                Action::Continue
            }

            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.select_next_user_block();
                Action::Continue
            }

            // Ctrl+O: open the selected conversation block action menu.
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_selected_conversation_action_menu();
                Action::Continue
            }

            // Tab: accept selected slash command without submitting
            KeyCode::Tab if !self.slash_rows.is_empty() => {
                if let Some(cmd) = self.selected_slash_command() {
                    textarea.move_cursor(CursorMove::Head);
                    textarea.delete_line_by_end();
                    textarea.insert_str(&cmd);
                    self.clear_slash_menu();
                } else {
                    self.toggle_selected_slash_group();
                }
                Action::Continue
            }

            // Right: expand selected slash discovery group.
            KeyCode::Right if !self.slash_rows.is_empty() => {
                self.expand_selected_slash_group();
                Action::Continue
            }

            // Left: collapse selected slash discovery group.
            KeyCode::Left if !self.slash_rows.is_empty() => {
                self.collapse_selected_slash_group();
                Action::Continue
            }

            // Esc: dismiss slash menu
            KeyCode::Esc => {
                self.clear_slash_menu();
                Action::Continue
            }

            // PageUp: scroll output buffer up (toward older lines)
            KeyCode::PageUp => {
                self.scroll_conversation_up(10, 1, 80);
                Action::Continue
            }

            // PageDown: scroll output buffer down (toward latest)
            KeyCode::PageDown => {
                self.scroll_conversation_down(10);
                Action::Continue
            }

            // Ctrl+D: exit
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Exit,

            // Ctrl+C: friendly quit reminder
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.push_feedback("(Use Ctrl+D to quit)", OutputStyle::Dim);
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

    /// Processes a bracketed paste event.
    ///
    /// When a popup input is active, pasted text belongs to that field instead
    /// of the underlying REPL prompt.
    pub fn handle_paste(&mut self, text: &str, textarea: &mut TextArea<'_>) {
        if matches!(self.panel, PanelState::ProviderManager { .. }) {
            let _ = self.insert_text_into_active_panel_field(text);
            return;
        }

        textarea.insert_str(text);
        let current = textarea.lines().join("\n");
        self.update_slash_matches(&current);
    }

    fn insert_text_into_active_panel_field(&mut self, text: &str) -> bool {
        let PanelState::ProviderManager { input_mode, .. } = &mut self.panel else {
            return false;
        };

        match input_mode {
            Some(PanelInputMode::AddProvider(buf)) => {
                append_single_line_input(buf, text);
                true
            }
            Some(PanelInputMode::AddStep2Model {
                manual_input: Some(manual),
                ..
            }) => {
                append_single_line_input(manual, text);
                true
            }
            Some(PanelInputMode::AddStep3Key { key_buf, .. }) => {
                append_single_line_input(key_buf, text);
                true
            }
            _ => false,
        }
    }

    /// Handles mouse events when the terminal delivers them.
    ///
    /// Handles wheel scrolling, block clicks, and app-managed text selection.
    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> bool {
        use crossterm::event::MouseEventKind;
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_conversation_up(3, 1, 80);
                true
            }
            MouseEventKind::ScrollDown => {
                self.scroll_conversation_down(3);
                true
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_conversation_mouse_down(mouse.column, mouse.row)
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.handle_conversation_mouse_drag(mouse.column, mouse.row)
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.handle_conversation_mouse_up(mouse.column, mouse.row)
            }
            _ => false,
        }
    }

    fn handle_conversation_action_menu_key(&mut self, key: KeyEvent) -> Action {
        let Some(menu) = self.conversation_action_menu else {
            return Action::Continue;
        };
        let actions = self.conversation_menu_actions(menu.block_index);
        match key.code {
            KeyCode::Esc => {
                self.conversation_action_menu = None;
            }
            KeyCode::Up => {
                self.conversation_action_selected =
                    self.conversation_action_selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = actions.len().saturating_sub(1);
                self.conversation_action_selected =
                    (self.conversation_action_selected + 1).min(max);
            }
            KeyCode::Enter => {
                if let Some(action) = actions.get(self.conversation_action_selected).copied() {
                    self.execute_conversation_action(menu.block_index, action);
                }
            }
            KeyCode::Char('c') => {
                self.execute_conversation_action(menu.block_index, ConversationAction::Copy);
            }
            KeyCode::Char('r') if actions.contains(&ConversationAction::Revert) => {
                self.execute_conversation_action(menu.block_index, ConversationAction::Revert);
            }
            _ => {}
        }
        Action::Continue
    }

    /// Handles key events when the provider manager panel is active.
    ///
    /// Extracts panel state by value, processes the key, then writes back.
    fn handle_provider_panel_key(
        &mut self,
        key_event: KeyEvent,
        textarea: &mut TextArea<'_>,
    ) -> Action {
        let (mut selected, mut input_mode) = match &self.panel {
            PanelState::ProviderManager {
                selected,
                input_mode,
                ..
            } => (*selected, input_mode.clone()),
            PanelState::None => return Action::Continue,
        };
        let mut new_status_msg: Option<(String, OutputStyle)> = None;

        if let Some(ref mut mode) = input_mode {
            match mode {
                PanelInputMode::ConfirmDelete => match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let Some(kind) = parse_provider_kind_by_index(selected) else {
                            self.panel = PanelState::ProviderManager {
                                selected,
                                input_mode: None,
                                status_msg: Some((
                                    "Unknown provider selection.".to_string(),
                                    OutputStyle::Error,
                                )),
                            };
                            return Action::Continue;
                        };
                        let removed = self.remove_provider_connection(&kind);
                        input_mode = None;
                        if removed {
                            self.save_config_and_rebuild_client();
                            new_status_msg = Some((
                                format!("{} connection deleted.", provider_kind_label(&kind)),
                                OutputStyle::Success,
                            ));
                        } else {
                            new_status_msg = Some((
                                format!("{} is not configured.", provider_kind_label(&kind)),
                                OutputStyle::Dim,
                            ));
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
                            selected = *step_sel;
                            input_mode = Some(provider_setup_mode(kind));
                        }
                    }
                    _ => {}
                },
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
                    KeyCode::Char(c) if !is_clipboard_shortcut(key_event.modifiers) => {
                        buf.push(c);
                    }
                    _ => {}
                },
                PanelInputMode::AddStepAuthType {
                    provider,
                    selected: auth_sel,
                } => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = None;
                    }
                    KeyCode::Up if *auth_sel > 0 => {
                        *auth_sel -= 1;
                    }
                    KeyCode::Down
                        if *auth_sel < provider_auth_modes(provider).len().saturating_sub(1) =>
                    {
                        *auth_sel += 1;
                    }
                    KeyCode::Enter => {
                        if let Some(auth_mode) = provider_auth_mode_by_index(provider, *auth_sel) {
                            input_mode = Some(PanelInputMode::AddStep2Model {
                                provider: provider.clone(),
                                is_subscription: auth_mode.is_subscription(),
                                selected: 0,
                                manual_input: None,
                            });
                        }
                    }
                    _ => {}
                },
                PanelInputMode::AddStep2Model {
                    provider,
                    is_subscription,
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
                            KeyCode::Char(c) if !is_clipboard_shortcut(key_event.modifiers) => {
                                manual.push(c);
                            }
                            KeyCode::Enter if !manual.is_empty() => {
                                let model = manual.clone();
                                input_mode = Some(PanelInputMode::AddStep3Key {
                                    provider: provider.clone(),
                                    model,
                                    key_buf: String::new(),
                                    is_subscription: *is_subscription,
                                });
                            }
                            _ => {}
                        }
                    } else {
                        // List selection mode
                        let models = recommended_models(provider);
                        match key_event.code {
                            KeyCode::Esc => {
                                input_mode = None;
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
                                    input_mode = Some(PanelInputMode::AddStep3Key {
                                        provider: provider.clone(),
                                        model: (*model_name).to_string(),
                                        key_buf: String::new(),
                                        is_subscription: *is_subscription,
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
                    is_subscription,
                } => match key_event.code {
                    KeyCode::Esc => {
                        input_mode = Some(PanelInputMode::AddStep2Model {
                            provider: provider.clone(),
                            is_subscription: *is_subscription,
                            selected: 0,
                            manual_input: None,
                        });
                    }
                    KeyCode::Backspace => {
                        key_buf.pop();
                    }
                    KeyCode::Char('c' | 'C') if is_clipboard_shortcut(key_event.modifiers) => {
                        if key_buf.is_empty() {
                            new_status_msg =
                                Some(("API key field is empty.".to_string(), OutputStyle::Dim));
                        } else {
                            match copy_text_to_clipboard(key_buf) {
                                Ok(()) => {
                                    new_status_msg = Some((
                                        "API key copied to clipboard.".to_string(),
                                        OutputStyle::Success,
                                    ));
                                }
                                Err(e) => {
                                    new_status_msg = Some((
                                        format!("Clipboard copy failed: {e}"),
                                        OutputStyle::Error,
                                    ));
                                }
                            }
                        }
                    }
                    KeyCode::Char('x' | 'X') if is_clipboard_shortcut(key_event.modifiers) => {
                        if key_buf.is_empty() {
                            new_status_msg =
                                Some(("API key field is empty.".to_string(), OutputStyle::Dim));
                        } else {
                            match copy_text_to_clipboard(key_buf) {
                                Ok(()) => {
                                    key_buf.clear();
                                    new_status_msg = Some((
                                        "API key cut to clipboard.".to_string(),
                                        OutputStyle::Success,
                                    ));
                                }
                                Err(e) => {
                                    new_status_msg = Some((
                                        format!("Clipboard cut failed: {e}"),
                                        OutputStyle::Error,
                                    ));
                                }
                            }
                        }
                    }
                    KeyCode::Char('v' | 'V') if is_clipboard_shortcut(key_event.modifiers) => {
                        match read_text_from_clipboard() {
                            Ok(text) => {
                                append_single_line_input(key_buf, &text);
                            }
                            Err(e) => {
                                new_status_msg = Some((
                                    format!("Clipboard paste failed: {e}"),
                                    OutputStyle::Error,
                                ));
                            }
                        }
                    }
                    KeyCode::Enter if !key_buf.is_empty() => match validate_provider_key(key_buf) {
                        Ok(()) => {
                            match self.save_provider_with_file_credential(
                                provider.clone(),
                                model.clone(),
                                key_buf.clone(),
                                *is_subscription,
                            ) {
                                Ok(()) => {
                                    self.save_config_and_rebuild_client();
                                    input_mode = None;
                                }
                                Err(e) => {
                                    new_status_msg = Some((
                                        format!("Credential save failed: {e}"),
                                        OutputStyle::Error,
                                    ));
                                }
                            }
                        }
                        Err(e) => new_status_msg = Some((e.to_string(), OutputStyle::Error)),
                    },
                    KeyCode::Char(c) if !is_clipboard_shortcut(key_event.modifiers) => {
                        key_buf.push(c);
                    }
                    _ => {}
                },
            }
            self.panel = PanelState::ProviderManager {
                selected,
                input_mode,
                status_msg: new_status_msg,
            };
            return Action::Continue;
        }

        match key_event.code {
            KeyCode::Esc => {
                self.panel = PanelState::None;
                textarea.move_cursor(CursorMove::Head);
                textarea.delete_line_by_end();
                Action::Continue
            }
            KeyCode::Up => {
                selected = selected.saturating_sub(1);
                self.panel = PanelState::ProviderManager {
                    selected,
                    input_mode: None,
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Down => {
                if selected < PROVIDER_KINDS.len().saturating_sub(1) {
                    selected += 1;
                }
                self.panel = PanelState::ProviderManager {
                    selected,
                    input_mode: None,
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Enter => {
                if let Some(kind) = parse_provider_kind_by_index(selected) {
                    input_mode = Some(provider_setup_mode(kind));
                }
                self.panel = PanelState::ProviderManager {
                    selected,
                    input_mode,
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if let Some(kind) = parse_provider_kind_by_index(selected) {
                    input_mode = Some(provider_setup_mode(kind));
                }
                self.panel = PanelState::ProviderManager {
                    selected,
                    input_mode,
                    status_msg: None,
                };
                Action::Continue
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if parse_provider_kind_by_index(selected)
                    .and_then(|kind| configured_provider_index(&self.config, &kind))
                    .is_some()
                {
                    self.panel = PanelState::ProviderManager {
                        selected,
                        input_mode: Some(PanelInputMode::ConfirmDelete),
                        status_msg: None,
                    };
                }
                Action::Continue
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if let Some(kind) = parse_provider_kind_by_index(selected) {
                    new_status_msg = Some(self.test_provider_connection_config(&kind));
                }
                self.panel = PanelState::ProviderManager {
                    selected,
                    input_mode: None,
                    status_msg: new_status_msg,
                };
                Action::Continue
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some(kind) = parse_provider_kind_by_index(selected) {
                    new_status_msg = Some(self.set_provider_priority(&kind));
                    self.save_config_and_rebuild_client();
                }
                self.panel = PanelState::ProviderManager {
                    selected,
                    input_mode: None,
                    status_msg: new_status_msg,
                };
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    /// Inserts or replaces one provider-kind connection while preserving fallback priority.
    fn upsert_provider_connection(&mut self, provider: crate::config::ProviderConfig) {
        let target =
            if configured_provider_index(&self.workspace_config, &provider.provider).is_some() {
                &mut self.workspace_config
            } else {
                &mut self.user_config
            };
        Self::upsert_provider_in_config(target, provider);
        self.refresh_effective_config();
    }

    /// Removes a provider connection and its file-stored credential, when present.
    fn remove_provider_connection(&mut self, kind: &crate::config::ProviderKind) -> bool {
        let removed = if let Some(index) = configured_provider_index(&self.workspace_config, kind) {
            Some(self.workspace_config.providers.remove(index))
        } else if let Some(index) = configured_provider_index(&self.user_config, kind) {
            Some(self.user_config.providers.remove(index))
        } else if let Some(index) = configured_provider_index(&self.config, kind) {
            Some(self.config.providers.remove(index))
        } else {
            None
        };
        let Some(removed) = removed else {
            return false;
        };
        if matches!(removed.key_storage, Some(crate::config::KeyStorage::File)) {
            let _ = super::keystore::delete_api_key(&removed.api_key_env);
            if let Some(token_env) = removed.auth_token_env {
                let _ = super::keystore::delete_api_key(&token_env);
            }
        }
        Self::ensure_primary_provider(&mut self.workspace_config);
        Self::ensure_primary_provider(&mut self.user_config);
        self.refresh_effective_config();
        true
    }

    fn save_provider_with_file_credential(
        &mut self,
        provider: crate::config::ProviderKind,
        model: String,
        key: String,
        is_subscription: bool,
    ) -> Result<(), String> {
        let api_key_env = default_api_key_env(&provider).to_string();
        let token_env = if is_subscription {
            Some(default_auth_token_env(&provider).to_string())
        } else {
            None
        };
        let store_env = token_env.as_deref().unwrap_or(&api_key_env);
        super::keystore::store_api_key(store_env, &key)?;
        // SAFETY: single-threaded CLI — no concurrent env access.
        unsafe {
            std::env::set_var(store_env, &key);
        }
        self.upsert_provider_connection(crate::config::ProviderConfig {
            provider,
            role: crate::config::ProviderRole::Primary,
            model,
            api_key_env,
            base_url: None,
            timeout_secs: None,
            key_storage: Some(crate::config::KeyStorage::File),
            auth_token_env: token_env,
        });
        Ok(())
    }

    /// Marks a configured provider as the first provider in the fallback chain.
    fn set_provider_priority(
        &mut self,
        kind: &crate::config::ProviderKind,
    ) -> (String, OutputStyle) {
        let Some(index) = configured_provider_index(&self.config, kind) else {
            return (
                format!("{} is not configured.", provider_kind_label(kind)),
                OutputStyle::Dim,
            );
        };
        let target = if configured_provider_index(&self.workspace_config, kind).is_some() {
            &mut self.workspace_config
        } else {
            &mut self.user_config
        };
        for (i, provider) in target.providers.iter_mut().enumerate() {
            provider.role = if i == index {
                crate::config::ProviderRole::Primary
            } else {
                crate::config::ProviderRole::Fallback
            };
        }
        self.refresh_effective_config();
        (
            format!(
                "{} is now first in the fallback chain.",
                provider_kind_label(kind)
            ),
            OutputStyle::Success,
        )
    }

    /// Performs a local provider config check without making a network call.
    fn test_provider_connection_config(
        &self,
        kind: &crate::config::ProviderKind,
    ) -> (String, OutputStyle) {
        let Some(index) = configured_provider_index(&self.config, kind) else {
            return (
                format!("{} is not configured.", provider_kind_label(kind)),
                OutputStyle::Dim,
            );
        };
        match crate::agents::factory::create_provider(&self.config.providers[index]) {
            Ok(_) => (
                format!("{} connection config is ready.", provider_kind_label(kind)),
                OutputStyle::Success,
            ),
            Err(e) => (
                format!("{} connection is not ready: {e}", provider_kind_label(kind)),
                OutputStyle::Error,
            ),
        }
    }

    /// Persists the current config to disk and rebuilds the LLM client.
    pub fn save_config_and_rebuild_client(&mut self) {
        let _ = crate::config::save_user_config(&self.user_config);
        if self.has_workspace && self.workspace_root.join(".duumbi/config.toml").exists() {
            let _ = crate::config::save_config(&self.workspace_root, &self.workspace_config);
        }
        self.refresh_effective_config();
        let providers = self.config.effective_providers();
        self.client = if providers.is_empty() {
            None
        } else {
            crate::agents::factory::create_provider_chain(&providers).ok()
        };
        self.keychain_cache = Self::build_keychain_cache(&self.config);
    }

    fn upsert_provider_in_config(
        config: &mut DuumbiConfig,
        mut provider: crate::config::ProviderConfig,
    ) {
        if let Some(index) = configured_provider_index(config, &provider.provider) {
            provider.role = config.providers[index].role.clone();
            config.providers[index] = provider;
        } else {
            provider.role = if config.providers.is_empty() {
                crate::config::ProviderRole::Primary
            } else {
                crate::config::ProviderRole::Fallback
            };
            config.providers.push(provider);
        }
    }

    fn ensure_primary_provider(config: &mut DuumbiConfig) {
        if !config
            .providers
            .iter()
            .any(|p| p.role == crate::config::ProviderRole::Primary)
            && let Some(first) = config.providers.first_mut()
        {
            first.role = crate::config::ProviderRole::Primary;
        }
    }

    fn refresh_effective_config(&mut self) {
        let effective = crate::config::merge_config_layers(
            DuumbiConfig::default(),
            self.user_config.clone(),
            self.workspace_config.clone(),
        );
        self.config = effective.config;
        self.provider_config_source = effective.provider_source;
    }

    fn provider_config_source_label(&self, kind: &crate::config::ProviderKind) -> &'static str {
        if configured_provider_index(&self.workspace_config, kind).is_some() {
            "workspace"
        } else if configured_provider_index(&self.user_config, kind).is_some() {
            "user"
        } else {
            match self.provider_config_source {
                ProviderConfigSource::LegacyWorkspace => "workspace",
                ProviderConfigSource::LegacyUser => "user",
                ProviderConfigSource::System | ProviderConfigSource::LegacySystem => "system",
                _ => "missing",
            }
        }
    }

    /// Reads `~/.duumbi/credentials.toml` once to build a cache of which env
    /// var names have a stored key. Used by the render path to avoid repeated
    /// file reads on every frame.
    fn build_keychain_cache(config: &DuumbiConfig) -> std::collections::HashSet<String> {
        config
            .effective_providers()
            .iter()
            .filter(|p| matches!(p.key_storage, Some(crate::config::KeyStorage::File)))
            .flat_map(|p| {
                let mut names = Vec::new();
                if super::keystore::load_api_key(&p.api_key_env).is_some() {
                    names.push(p.api_key_env.clone());
                }
                if let Some(token_env) = &p.auth_token_env
                    && super::keystore::load_api_key(token_env).is_some()
                {
                    names.push(token_env.clone());
                }
                names
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
        if input.starts_with('/') && !input.contains('\n') {
            let filter_changed = self.slash_filter != input;
            let was_discovery = self.slash_filter == "/";
            let is_discovery = input == "/";
            if !was_discovery || !is_discovery {
                self.slash_expanded_groups.clear();
            }
            self.slash_filter = input.to_string();
            self.slash_matches = SLASH_COMMANDS
                .iter()
                .filter(|entry| entry.command.starts_with(input))
                .map(|entry| SlashMatch {
                    command: entry.command.to_string(),
                    description: entry.description.to_string(),
                    group: entry.group,
                    matched_prefix_len: input.len().min(entry.command.len()),
                })
                .collect();
            self.rebuild_slash_rows();
            if filter_changed {
                self.slash_selected = 0;
                self.slash_scroll_offset = 0;
            }
        } else {
            self.clear_slash_menu();
        }
        self.clamp_slash_selection();
    }

    fn clear_slash_menu(&mut self) {
        self.slash_matches.clear();
        self.slash_rows.clear();
        self.slash_filter.clear();
        self.slash_expanded_groups.clear();
        self.slash_selected = 0;
        self.slash_scroll_offset = 0;
    }

    fn rebuild_slash_rows(&mut self) {
        if self.slash_filter == "/" {
            let mut rows = Vec::new();
            for group in SLASH_GROUPS {
                let group_matches: Vec<SlashMatch> = self
                    .slash_matches
                    .iter()
                    .filter(|sm| sm.group == *group)
                    .cloned()
                    .collect();
                if group_matches.is_empty() {
                    continue;
                }
                let expanded = self.slash_expanded_groups.contains(group);
                rows.push(SlashMenuItem::Group {
                    group: *group,
                    count: group_matches.len(),
                    expanded,
                });
                if expanded {
                    rows.extend(group_matches.into_iter().map(SlashMenuItem::Command));
                }
            }
            self.slash_rows = rows;
        } else {
            self.slash_rows = self
                .slash_matches
                .iter()
                .cloned()
                .map(SlashMenuItem::Command)
                .collect();
        }
    }

    fn clamp_slash_selection(&mut self) {
        if self.slash_rows.is_empty() {
            self.slash_selected = 0;
            self.slash_scroll_offset = 0;
            return;
        }
        self.slash_selected = self
            .slash_selected
            .min(self.slash_rows.len().saturating_sub(1));
        if self.slash_scroll_offset > self.slash_selected {
            self.slash_scroll_offset = self.slash_selected;
        }
        if self.slash_selected >= self.slash_scroll_offset + Self::SLASH_MENU_VISIBLE {
            self.slash_scroll_offset = self.slash_selected + 1 - Self::SLASH_MENU_VISIBLE;
        }
    }

    fn selected_slash_command(&self) -> Option<String> {
        match self.slash_rows.get(self.slash_selected) {
            Some(SlashMenuItem::Command(sm)) => Some(sm.command.clone()),
            _ => None,
        }
    }

    fn selected_slash_group(&self) -> Option<SlashGroup> {
        match self.slash_rows.get(self.slash_selected) {
            Some(SlashMenuItem::Group { group, .. }) => Some(*group),
            _ => None,
        }
    }

    fn activate_selected_slash_row(&mut self, textarea: &mut TextArea<'_>) -> Option<Action> {
        if let Some(cmd) = self.selected_slash_command() {
            textarea.move_cursor(CursorMove::Head);
            textarea.delete_line_by_end();
            self.clear_slash_menu();
            return Some(Action::Submit(cmd));
        }
        if self.selected_slash_group().is_some() {
            self.toggle_selected_slash_group();
            return Some(Action::Continue);
        }
        None
    }

    fn toggle_selected_slash_group(&mut self) {
        if let Some(group) = self.selected_slash_group() {
            if !self.slash_expanded_groups.remove(&group) {
                self.slash_expanded_groups.insert(group);
            }
            self.rebuild_slash_rows();
            self.clamp_slash_selection();
        }
    }

    fn expand_selected_slash_group(&mut self) {
        if let Some(group) = self.selected_slash_group() {
            self.slash_expanded_groups.insert(group);
            self.rebuild_slash_rows();
            self.clamp_slash_selection();
        }
    }

    fn collapse_selected_slash_group(&mut self) {
        if let Some(group) = self.selected_slash_group() {
            self.slash_expanded_groups.remove(&group);
            self.rebuild_slash_rows();
            self.clamp_slash_selection();
        }
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
        let output_idx = self.ensure_output_block();
        for line in text.split('\n') {
            let output_line = OutputLine::new(line.to_string(), style);
            self.output_lines.push(output_line.clone());
            if let Some(block) = self.conversation_blocks.get_mut(output_idx) {
                block.lines.push(output_line);
            }
        }
        if self.output_lines.len() > Self::OUTPUT_BUFFER_MAX {
            let excess = self.output_lines.len() - Self::OUTPUT_BUFFER_MAX;
            self.output_lines.drain(..excess);
        }
        // Reset scroll to bottom when new output arrives.
        self.output_scroll_offset = 0;
    }

    fn push_feedback(&mut self, text: impl Into<String>, style: OutputStyle) {
        self.current_output_block = None;
        self.push_output(text, style);
        self.current_output_block = None;
    }

    /// Starts a new conversation turn with a persistent user input block.
    pub fn begin_user_block(&mut self, input: &str) {
        let submitted_at = Local::now().format("%H:%M").to_string();
        self.conversation_blocks
            .push(ConversationBlock::user(input.to_string(), submitted_at));
        self.output_lines
            .push(OutputLine::new(input.to_string(), OutputStyle::Dim));
        self.current_output_block = None;
        self.selected_conversation_block = None;
        self.conversation_action_menu = None;
        self.conversation_text_selection = None;
        self.output_scroll_offset = 0;
    }

    fn user_block_indices(&self) -> Vec<usize> {
        self.conversation_blocks
            .iter()
            .enumerate()
            .filter_map(|(idx, block)| (block.kind == ConversationBlockKind::User).then_some(idx))
            .collect()
    }

    fn select_previous_user_block(&mut self) {
        let indices = self.user_block_indices();
        if indices.is_empty() {
            return;
        }

        let next = self
            .selected_conversation_block
            .and_then(|selected| indices.iter().position(|idx| *idx == selected))
            .and_then(|pos| pos.checked_sub(1))
            .map_or_else(
                || *indices.last().expect("invariant: non-empty indices"),
                |pos| indices[pos],
            );

        self.selected_conversation_block = Some(next);
        self.conversation_action_menu = None;
        self.conversation_action_selected = 0;
    }

    fn select_next_user_block(&mut self) {
        let indices = self.user_block_indices();
        if indices.is_empty() {
            return;
        }

        let next = self
            .selected_conversation_block
            .and_then(|selected| indices.iter().position(|idx| *idx == selected))
            .and_then(|pos| indices.get(pos + 1).copied())
            .unwrap_or(indices[0]);

        self.selected_conversation_block = Some(next);
        self.conversation_action_menu = None;
        self.conversation_action_selected = 0;
    }

    fn open_selected_conversation_action_menu(&mut self) {
        let Some(block_index) = self.selected_conversation_block else {
            self.select_previous_user_block();
            return;
        };
        if self.conversation_menu_actions(block_index).is_empty() {
            return;
        }
        self.conversation_action_menu = Some(ConversationActionMenu { block_index });
        self.conversation_action_selected = 0;
    }

    /// Marks the latest user block as revertable.
    pub fn mark_latest_user_block_revertable(&mut self, snapshot_path: PathBuf) {
        if let Some(block) = self
            .conversation_blocks
            .iter_mut()
            .rev()
            .find(|block| block.kind == ConversationBlockKind::User)
        {
            if !block.actions.contains(&ConversationAction::Revert) {
                block.actions.push(ConversationAction::Revert);
            }
            block.revert_snapshot = Some(snapshot_path);
        }
    }

    /// Adds a runtime footer to the active output block.
    pub fn finish_current_output_elapsed(&mut self, elapsed: std::time::Duration) {
        if let Some(idx) = self.current_output_block
            && let Some(block) = self.conversation_blocks.get_mut(idx)
        {
            block.elapsed = Some(format!("completed in {:.2}s", elapsed.as_secs_f64()));
        }
    }

    fn scroll_conversation_up(&mut self, amount: usize, fallback_height: u16, fallback_width: u16) {
        let total = self.conversation_line_count(fallback_width);
        let max_scroll = total.saturating_sub(fallback_height as usize);
        self.output_scroll_offset = (self.output_scroll_offset + amount).min(max_scroll);
    }

    fn scroll_conversation_down(&mut self, amount: usize) {
        self.output_scroll_offset = self.output_scroll_offset.saturating_sub(amount);
    }

    fn conversation_line_count(&self, width: u16) -> usize {
        if self.conversation_blocks.is_empty() {
            self.output_lines.len()
        } else {
            self.conversation_visual_rows(width).len()
        }
    }

    fn user_block_text(&self, block_index: usize) -> Option<String> {
        self.conversation_blocks
            .get(block_index)
            .filter(|block| block.kind == ConversationBlockKind::User)
            .and_then(|block| block.lines.first())
            .map(|line| line.text.clone())
    }

    fn copy_user_block(&mut self, block_index: usize) {
        let Some(text) = self.user_block_text(block_index) else {
            self.push_feedback("Nothing to copy.", OutputStyle::Dim);
            return;
        };

        match copy_text_to_clipboard(&text) {
            Ok(()) => {
                self.push_feedback("Copied message.", OutputStyle::Success);
            }
            Err(e) => {
                self.push_feedback(format!("Copy failed: {e}"), OutputStyle::Error);
            }
        }
    }

    fn latest_revertable_block_index(&self) -> Option<usize> {
        self.conversation_blocks
            .iter()
            .enumerate()
            .rev()
            .find(|(_, block)| {
                block.kind == ConversationBlockKind::User
                    && block.actions.contains(&ConversationAction::Revert)
            })
            .map(|(idx, _)| idx)
    }

    fn conversation_menu_actions(&self, block_index: usize) -> Vec<ConversationAction> {
        let Some(block) = self.conversation_blocks.get(block_index) else {
            return Vec::new();
        };
        if block.kind != ConversationBlockKind::User {
            return Vec::new();
        }

        let mut actions = vec![ConversationAction::Copy];
        if block.actions.contains(&ConversationAction::Revert)
            && self.latest_revertable_block_index() == Some(block_index)
        {
            actions.push(ConversationAction::Revert);
        }
        actions
    }

    fn execute_conversation_action(&mut self, block_index: usize, action: ConversationAction) {
        self.conversation_action_menu = None;
        match action {
            ConversationAction::Copy => self.copy_user_block(block_index),
            ConversationAction::Revert => self.revert_user_block_change(block_index),
        }
    }

    fn revert_user_block_change(&mut self, block_index: usize) {
        if self.latest_revertable_block_index() != Some(block_index) {
            self.push_feedback("Nothing to revert.", OutputStyle::Dim);
            return;
        }

        match crate::snapshot::restore_latest(&self.workspace_root) {
            Ok(true) => {
                self.history.pop();
                if let Some(block) = self.conversation_blocks.get_mut(block_index) {
                    block
                        .actions
                        .retain(|action| *action != ConversationAction::Revert);
                    block.revert_snapshot = None;
                }
                self.push_feedback("Reverted latest graph change.", OutputStyle::Success);
            }
            Ok(false) => {
                self.push_feedback("Nothing to revert.", OutputStyle::Dim);
            }
            Err(e) => {
                self.push_feedback(format!("Revert failed: {e:#}"), OutputStyle::Error);
            }
        }
    }

    fn ensure_output_block(&mut self) -> usize {
        if let Some(idx) = self.current_output_block
            && self
                .conversation_blocks
                .get(idx)
                .is_some_and(|block| block.kind == ConversationBlockKind::Output)
        {
            return idx;
        }

        self.conversation_blocks.push(ConversationBlock::output());
        let idx = self.conversation_blocks.len() - 1;
        self.current_output_block = Some(idx);
        idx
    }

    fn handle_conversation_mouse_down(&mut self, column: u16, row: u16) -> bool {
        let Some(area) = self.last_conversation_area.get() else {
            return false;
        };

        if let Some((menu_area, _)) = self.conversation_action_menu_rect(area)
            && self.rect_contains(menu_area, column, row)
        {
            self.conversation_text_selection = None;
            return self.handle_conversation_click(column, row);
        }

        let point = self.conversation_point_at(area, column, row);
        self.conversation_text_selection = point.map(|anchor| ConversationTextSelection {
            anchor,
            focus: anchor,
            dragged: false,
        });
        self.handle_conversation_click(column, row)
    }

    fn handle_conversation_mouse_drag(&mut self, column: u16, row: u16) -> bool {
        let Some(area) = self.last_conversation_area.get() else {
            return false;
        };
        let Some(focus) = self.conversation_point_at(area, column, row) else {
            return false;
        };
        let Some(selection) = self.conversation_text_selection.as_mut() else {
            self.conversation_text_selection = Some(ConversationTextSelection {
                anchor: focus,
                focus,
                dragged: false,
            });
            return false;
        };

        selection.focus = focus;
        selection.dragged = selection.anchor != focus;
        selection.dragged
    }

    fn handle_conversation_mouse_up(&mut self, column: u16, row: u16) -> bool {
        let Some(mut selection) = self.conversation_text_selection else {
            return false;
        };

        if let Some(area) = self.last_conversation_area.get()
            && let Some(focus) = self.conversation_point_at(area, column, row)
        {
            selection.focus = focus;
            selection.dragged = selection.dragged || selection.anchor != focus;
            self.conversation_text_selection = Some(selection);
        }

        if !selection.dragged {
            self.conversation_text_selection = None;
            return false;
        }

        let selected_text = self
            .last_conversation_area
            .get()
            .and_then(|area| self.selected_conversation_text(area));
        self.conversation_text_selection = None;

        if let Some(text) = selected_text
            && !text.trim().is_empty()
            && let Err(e) = copy_text_to_clipboard(&text)
        {
            self.push_feedback(format!("Copy failed: {e}"), OutputStyle::Error);
        }
        true
    }

    fn handle_conversation_click(&mut self, column: u16, row: u16) -> bool {
        let Some(area) = self.last_conversation_area.get() else {
            return false;
        };

        if let Some((menu_area, actions)) = self.conversation_action_menu_rect(area)
            && self.rect_contains(menu_area, column, row)
        {
            let action_idx = row.saturating_sub(menu_area.y + 1) as usize;
            if let Some(action) = actions.get(action_idx).copied()
                && let Some(menu) = self.conversation_action_menu
            {
                self.execute_conversation_action(menu.block_index, action);
            }
            return true;
        }

        if self.conversation_action_menu.is_some() {
            self.conversation_action_menu = None;
            return true;
        }

        if !self.rect_contains(area, column, row) {
            return false;
        }

        let layout = self.visible_conversation_layout(area);
        let relative_y = row.saturating_sub(area.y) as usize;
        if relative_y < layout.padding {
            return false;
        }
        let Some(visual_row) = layout.rows.get(relative_y - layout.padding) else {
            return false;
        };

        if let (Some(block_index), Some((start, end))) =
            (visual_row.menu_button_block, visual_row.menu_button_range)
            && column >= area.x + start
            && column < area.x + end
            && self.selected_conversation_block == Some(block_index)
        {
            self.conversation_action_menu = Some(ConversationActionMenu { block_index });
            return true;
        }

        let Some(block_index) = visual_row.block_index else {
            return false;
        };
        if self
            .conversation_blocks
            .get(block_index)
            .is_none_or(|block| block.kind != ConversationBlockKind::User)
        {
            return false;
        }

        if self.selected_conversation_block == Some(block_index) {
            self.selected_conversation_block = None;
            self.conversation_action_menu = None;
        } else {
            self.selected_conversation_block = Some(block_index);
            self.conversation_action_menu = None;
        }
        true
    }

    fn conversation_point_at(
        &self,
        area: Rect,
        column: u16,
        row: u16,
    ) -> Option<ConversationSelectionPoint> {
        if !self.rect_contains(area, column, row) {
            return None;
        }

        let layout = self.visible_conversation_layout(area);
        let relative_y = row.saturating_sub(area.y) as usize;
        if relative_y < layout.padding {
            return None;
        }

        let visible_row = relative_y - layout.padding;
        let row_data = layout.rows.get(visible_row)?;
        let max_column = row_data.plain_text.chars().count();
        let column = column.saturating_sub(area.x) as usize;

        Some(ConversationSelectionPoint {
            row: layout.start_index + visible_row,
            column: column.min(max_column),
        })
    }

    fn selected_conversation_text(&self, area: Rect) -> Option<String> {
        let selection = self.conversation_text_selection?;
        let rows = if self.conversation_blocks.is_empty() {
            self.legacy_output_rows()
        } else {
            self.conversation_visual_rows(area.width)
        };
        let (start, end) = Self::normalise_selection(selection);
        if start.row >= rows.len() || end.row >= rows.len() {
            return None;
        }

        let mut selected = Vec::new();
        for (row_idx, row) in rows.iter().enumerate().take(end.row + 1).skip(start.row) {
            let text_len = row.plain_text.chars().count();
            let start_col = if row_idx == start.row {
                start.column.min(text_len)
            } else {
                0
            };
            let end_col = if row_idx == end.row {
                end.column.min(text_len)
            } else {
                text_len
            };
            let line = row
                .plain_text
                .chars()
                .skip(start_col)
                .take(end_col.saturating_sub(start_col))
                .collect::<String>()
                .trim_end()
                .to_string();
            selected.push(line);
        }

        Some(selected.join("\n"))
    }

    fn normalise_selection(
        selection: ConversationTextSelection,
    ) -> (ConversationSelectionPoint, ConversationSelectionPoint) {
        if (selection.anchor.row, selection.anchor.column)
            <= (selection.focus.row, selection.focus.column)
        {
            (selection.anchor, selection.focus)
        } else {
            (selection.focus, selection.anchor)
        }
    }

    fn rect_contains(&self, rect: Rect, column: u16, row: u16) -> bool {
        column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
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
        let input_height = if page.width >= 60 { 3u16 } else { 1u16 };
        // header(1) + spacer(1) + spacer-before-controls(1) + mode(1) + prompt + status(1)
        let fixed_ui_rows: u16 = 5 + input_height;
        let card_height = if show_card {
            let desired = if page.width >= 90 { 7u16 } else { 8u16 };
            desired.min(page.height.saturating_sub(fixed_ui_rows))
        } else {
            0
        };

        let chunks = Layout::vertical([
            Constraint::Length(1),            // header
            Constraint::Length(1),            // spacer
            Constraint::Length(card_height),  // empty-state
            Constraint::Min(0),               // conversation
            Constraint::Length(1),            // spacer before controls
            Constraint::Length(1),            // mode row
            Constraint::Length(input_height), // prompt
            Constraint::Length(1),            // status row
        ])
        .split(page);

        self.render_brand_header(frame, chunks[0]);
        if card_height > 0 {
            self.render_empty_state_card(frame, chunks[2]);
        }
        self.render_conversation_pane(frame, chunks[3]);
        self.render_mode_strip(frame, chunks[5]);
        self.render_prompt_well(frame, chunks[6], textarea);
        self.render_status_dock(frame, chunks[7]);

        match &self.panel {
            PanelState::None => {
                if let Some(overlay_area) = self.slash_overlay_rect(page, chunks[6]) {
                    self.render_slash_menu(frame, overlay_area);
                }
            }
            PanelState::ProviderManager {
                selected,
                input_mode,
                status_msg,
            } => {
                if let Some(overlay_height) = self.overlay_height() {
                    let overlay_width = page.width.saturating_sub(4).clamp(52, 110);
                    let overlay_area =
                        Self::centered_overlay_rect(page, overlay_width, overlay_height);
                    self.render_provider_panel(
                        frame,
                        overlay_area,
                        *selected,
                        input_mode,
                        status_msg,
                    )
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
                let total = self.slash_rows.len();
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
            PanelState::ProviderManager { input_mode, .. } => Some(match input_mode {
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
                Some(PanelInputMode::AddStepAuthType { provider, .. }) => {
                    provider_auth_modes(provider).len() as u16 + 6
                }
                Some(PanelInputMode::AddStep3Key { .. }) => 9,
                _ => {
                    let provider_count = PROVIDER_KINDS.len().max(1);
                    let input_line = if input_mode.is_some() { 1 } else { 0 };
                    let status_line = match (&self.panel, input_mode) {
                        (
                            PanelState::ProviderManager {
                                status_msg: Some(_),
                                ..
                            },
                            None,
                        ) => 2,
                        _ => 0,
                    };
                    (provider_count as u16) + 7 + input_line + status_line
                }
            }),
        }
    }

    /// Returns a prompt-aligned slash menu rectangle directly above the prompt.
    fn slash_overlay_rect(&self, page: Rect, anchor: Rect) -> Option<Rect> {
        let total = self.slash_rows.len();
        if total == 0 {
            return None;
        }

        let available_above = anchor.y.saturating_sub(page.y);
        if available_above < 5 {
            return None;
        }

        let max_visible = available_above
            .saturating_sub(4)
            .max(1)
            .min(Self::SLASH_MENU_VISIBLE_MAX as u16) as usize;
        let visible = total.min(max_visible);
        let height = (visible as u16).saturating_add(4).min(available_above);
        let y = anchor.y.saturating_sub(height);
        Some(Rect::new(page.x, y, page.width, height))
    }

    /// Returns a centered overlay rectangle inside the page.
    fn centered_overlay_rect(page: Rect, width: u16, height: u16) -> Rect {
        let width = width.min(page.width).max(1);
        let height = height.min(page.height).max(1);
        let x = page.x + page.width.saturating_sub(width) / 2;
        let y = page.y + page.height.saturating_sub(height) / 2;
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
        use ratatui::widgets::{Block, Padding};

        if area.width < 12 || area.height < 5 {
            return;
        }

        let cols = Layout::horizontal([Constraint::Length(1), Constraint::Min(1)]).split(area);
        let accent = cols[0];
        let card_area = cols[1];

        // U+258F LEFT ONE EIGHTH BLOCK — narrow block char that fills the
        // full cell height, so the bar stays continuous across rows even on
        // terminals where the box-drawing │ leaves vertical gaps.
        let bar_lines: Vec<Line<'_>> = (0..accent.height)
            .map(|_| Line::from(Span::styled("\u{258F}", theme::panel_accent())))
            .collect();
        frame.render_widget(
            Paragraph::new(bar_lines).style(theme::panel_surface()),
            accent,
        );

        let block = Block::default()
            .padding(Padding::new(2, 2, 1, 1))
            .style(theme::panel_surface());
        let inner = block.inner(card_area);
        frame.render_widget(block, card_area);

        let (badge_text, heading, example_lines) = if !self.has_workspace {
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
                        Span::styled(" - natural language works too", theme::out_dim()),
                    ]),
                ],
            )
        };

        let mut body = vec![
            Line::from(vec![
                Span::styled(badge_text, theme::pill_outline()),
                Span::raw("  "),
                Span::styled(heading, theme::label_caps()),
            ]),
            Line::from(""),
        ];
        body.extend(example_lines);
        frame.render_widget(
            Paragraph::new(body)
                .style(theme::panel_surface())
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    /// REPL-03 — scrollable conversation pane, bottom-aligned.
    ///
    /// When `output_scroll_offset > 0`, the view shifts upward to show
    /// older lines. PageUp/PageDown control the offset.
    fn render_conversation_pane(&self, frame: &mut Frame, area: Rect) {
        self.last_conversation_area.set(Some(area));
        let layout = self.visible_conversation_layout(area);

        // Bottom-align: pad with empty lines above content so messages
        // appear just above the header, close to the prompt.
        let mut lines: Vec<Line<'_>> = (0..layout.padding).map(|_| Line::from("")).collect();
        lines.extend(
            layout
                .rows
                .iter()
                .enumerate()
                .map(|(idx, row)| self.conversation_row_line(row, layout.start_index + idx)),
        );

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

        self.render_conversation_action_menu(frame, area);
    }

    fn visible_conversation_layout(&self, area: Rect) -> VisibleConversationLayout {
        let max_lines = area.height as usize;
        let all_rows = if self.conversation_blocks.is_empty() {
            self.legacy_output_rows()
        } else {
            self.conversation_visual_rows(area.width)
        };
        let total = all_rows.len();
        let bottom = total.saturating_sub(self.output_scroll_offset);
        let start = bottom.saturating_sub(max_lines);
        let rows = all_rows[start..bottom].to_vec();
        let padding = max_lines.saturating_sub(rows.len());
        VisibleConversationLayout {
            start_index: start,
            padding,
            rows,
        }
    }

    fn conversation_row_line(
        &self,
        row: &ConversationVisualRow,
        row_index: usize,
    ) -> Line<'static> {
        let Some(selection) = self.conversation_text_selection else {
            return row.line.clone();
        };
        if !selection.dragged {
            return row.line.clone();
        }

        let (start, end) = Self::normalise_selection(selection);
        if row_index < start.row || row_index > end.row {
            return row.line.clone();
        }

        let text_len = row.plain_text.chars().count();
        let start_col = if row_index == start.row {
            start.column.min(text_len)
        } else {
            0
        };
        let end_col = if row_index == end.row {
            end.column.min(text_len)
        } else {
            text_len
        };
        if start_col == end_col {
            return row.line.clone();
        }

        let before = row.plain_text.chars().take(start_col).collect::<String>();
        let selected = row
            .plain_text
            .chars()
            .skip(start_col)
            .take(end_col.saturating_sub(start_col))
            .collect::<String>();
        let after = row.plain_text.chars().skip(end_col).collect::<String>();

        Line::from(vec![
            Span::raw(before),
            Span::styled(selected, theme::conversation_text_selection()),
            Span::raw(after),
        ])
    }

    fn render_conversation_action_menu(&self, frame: &mut Frame, area: Rect) {
        let Some((menu_area, actions)) = self.conversation_action_menu_rect(area) else {
            return;
        };

        let lines = actions
            .iter()
            .enumerate()
            .map(|(idx, action)| {
                let label = match action {
                    ConversationAction::Copy => "Copy message",
                    ConversationAction::Revert => "Revert change",
                };
                let style = if idx == self.conversation_action_selected {
                    theme::conversation_action_menu_selected()
                } else {
                    theme::conversation_action_menu()
                };
                Line::from(Span::styled(format!(" {label:<18}"), style))
            })
            .collect::<Vec<_>>();

        frame.render_widget(Clear, menu_area);
        frame.render_widget(
            Paragraph::new(lines)
                .style(theme::conversation_action_menu())
                .block(
                    Block::bordered()
                        .border_style(theme::conversation_action_menu_border())
                        .style(theme::conversation_action_menu()),
                ),
            menu_area,
        );
    }

    fn conversation_action_menu_rect(&self, area: Rect) -> Option<(Rect, Vec<ConversationAction>)> {
        let menu = self.conversation_action_menu?;
        if self.selected_conversation_block != Some(menu.block_index) {
            return None;
        }
        let actions = self.conversation_menu_actions(menu.block_index);
        if actions.is_empty() {
            return None;
        }

        let layout = self.visible_conversation_layout(area);
        let row_offset = layout
            .rows
            .iter()
            .position(|row| row.block_index == Some(menu.block_index))?;
        let anchor_y = area.y + layout.padding as u16 + row_offset as u16;
        let width = 22_u16.min(area.width);
        let height = (actions.len() as u16 + 2).min(area.height.max(1));
        let x = area.right().saturating_sub(width);
        let y = if anchor_y + height < area.bottom() {
            anchor_y.saturating_add(1)
        } else {
            anchor_y.saturating_sub(height.saturating_sub(1))
        };

        Some((Rect::new(x, y, width, height), actions))
    }

    fn legacy_output_rows(&self) -> Vec<ConversationVisualRow> {
        self.output_lines
            .iter()
            .map(|ol| ConversationVisualRow {
                line: Self::line_from_output(ol),
                plain_text: ol.text.clone(),
                block_index: None,
                menu_button_block: None,
                menu_button_range: None,
            })
            .collect()
    }

    fn conversation_visual_rows(&self, width: u16) -> Vec<ConversationVisualRow> {
        let mut rows = Vec::new();
        for (idx, block) in self.conversation_blocks.iter().enumerate() {
            if !rows.is_empty() {
                rows.push(ConversationVisualRow {
                    line: Line::from(""),
                    plain_text: String::new(),
                    block_index: None,
                    menu_button_block: None,
                    menu_button_range: None,
                });
            }
            match block.kind {
                ConversationBlockKind::User => {
                    rows.extend(self.user_block_rows(idx, block, width));
                }
                ConversationBlockKind::Output => {
                    for ol in &block.lines {
                        rows.push(ConversationVisualRow {
                            line: Self::line_from_output(ol),
                            plain_text: ol.text.clone(),
                            block_index: Some(idx),
                            menu_button_block: None,
                            menu_button_range: None,
                        });
                    }
                    if let Some(elapsed) = &block.elapsed {
                        rows.push(ConversationVisualRow {
                            line: Line::from(Span::styled(elapsed.clone(), theme::out_dim())),
                            plain_text: elapsed.clone(),
                            block_index: Some(idx),
                            menu_button_block: None,
                            menu_button_range: None,
                        });
                    }
                }
            }
        }
        rows
    }

    fn user_block_rows(
        &self,
        block_index: usize,
        block: &ConversationBlock,
        width: u16,
    ) -> Vec<ConversationVisualRow> {
        let width = width as usize;
        let input = block
            .lines
            .first()
            .map(|line| line.text.as_str())
            .unwrap_or("");
        let submitted_at = block.submitted_at.as_deref().unwrap_or("");
        let selected = self.selected_conversation_block == Some(block_index);
        let action_trigger = if selected { "..." } else { "" };
        let (line, range, plain_text) =
            self.user_panel_line(input, action_trigger, width, true, selected);
        let (meta_line, _, meta_plain_text) =
            self.user_panel_line(submitted_at, "", width, false, selected);
        vec![
            ConversationVisualRow {
                line,
                plain_text,
                block_index: Some(block_index),
                menu_button_block: selected.then_some(block_index),
                menu_button_range: range,
            },
            ConversationVisualRow {
                line: meta_line,
                plain_text: meta_plain_text,
                block_index: Some(block_index),
                menu_button_block: None,
                menu_button_range: None,
            },
        ]
    }

    fn user_panel_line(
        &self,
        left: &str,
        right: &str,
        width: usize,
        strong: bool,
        selected: bool,
    ) -> (Line<'static>, Option<(u16, u16)>, String) {
        let accent = "\u{258f} ";
        let right_len = right.chars().count();
        let left_budget = width
            .saturating_sub(accent.chars().count())
            .saturating_sub(right_len)
            .saturating_sub(3);
        let left_text = Self::truncate_inline(left, left_budget);
        let left_len = left_text.chars().count();
        let fill = width.saturating_sub(accent.chars().count() + left_len + right_len);
        let left_style = if strong {
            if selected {
                theme::conversation_user_selected_text()
            } else {
                theme::conversation_user_text()
            }
        } else if selected {
            theme::conversation_user_selected_meta()
        } else {
            theme::conversation_user_meta()
        };
        let surface_style = if selected {
            theme::conversation_user_selected_surface()
        } else {
            theme::panel_surface()
        };
        let accent_style = if selected {
            theme::conversation_user_selected_accent()
        } else {
            theme::panel_accent()
        };
        let right_style = if selected && !right.is_empty() {
            theme::conversation_user_action()
        } else if selected {
            theme::conversation_user_selected_meta()
        } else {
            theme::conversation_user_meta()
        };
        let right_start = accent.chars().count() + left_len + fill;
        let right_range = (!right.is_empty()).then_some((
            right_start.min(u16::MAX as usize) as u16,
            (right_start + right_len).min(u16::MAX as usize) as u16,
        ));
        let plain_text = format!("{accent}{left_text}{}{right}", " ".repeat(fill));
        (
            Line::from(vec![
                Span::styled(accent.to_string(), accent_style),
                Span::styled(left_text.clone(), left_style),
                Span::styled(" ".repeat(fill), surface_style),
                Span::styled(right.to_string(), right_style),
            ]),
            right_range,
            plain_text,
        )
    }

    fn truncate_inline(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_string();
        }
        if max_chars <= 1 {
            return "\u{2026}".to_string();
        }
        let mut out = text.chars().take(max_chars - 1).collect::<String>();
        out.push('\u{2026}');
        out
    }

    fn line_from_output(ol: &OutputLine) -> Line<'static> {
        match ol.style {
            OutputStyle::Help => {
                let text = &ol.text;
                if text.len() > 35 {
                    let (cmd_part, desc_part) = text.split_at(35);
                    Line::from(vec![
                        Span::styled(cmd_part.to_string(), theme::out_help_cmd()),
                        Span::styled(desc_part.to_string(), theme::out_help_desc()),
                    ])
                } else {
                    Line::from(Span::styled(text.clone(), theme::out_help_cmd()))
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
                Line::from(Span::styled(ol.text.clone(), style))
            }
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
            Span::styled(" + ", theme::hairline()),
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
    fn render_slash_menu(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Padding};

        if self.slash_rows.is_empty() {
            return;
        }

        frame.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::panel_border())
            .padding(Padding::new(1, 1, 0, 0))
            .style(theme::panel_surface());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 {
            return;
        }

        let header = if self.slash_filter == "/" {
            Line::from(vec![
                Span::styled("DISCOVER", theme::slash_group()),
                Span::styled(
                    format!(
                        "  {} commands across {} groups",
                        SLASH_COMMANDS.len(),
                        SLASH_GROUPS.len()
                    ),
                    theme::dock_value(),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled("FILTER", theme::slash_group()),
                Span::styled("  matching ", theme::dock_value()),
                Span::styled(self.slash_filter.as_str(), theme::keycap()),
                Span::styled(
                    format!("  {} matches", self.slash_matches.len()),
                    theme::out_dim(),
                ),
            ])
        };

        let body_capacity = inner.height.saturating_sub(2) as usize;
        let total = self.slash_rows.len();
        let visible = total.min(body_capacity);
        if visible == 0 {
            return;
        }
        let offset = self.slash_scroll_offset.min(total.saturating_sub(1));

        let rows = Layout::vertical(
            std::iter::repeat_n(Constraint::Length(1), visible + 2).collect::<Vec<_>>(),
        )
        .split(inner);

        frame.render_widget(Paragraph::new(header), rows[0]);

        for (i, item) in self
            .slash_rows
            .iter()
            .skip(offset)
            .take(visible)
            .enumerate()
        {
            let abs_index = offset + i;
            let is_selected = abs_index == self.slash_selected;
            let row = rows[i + 1];
            if is_selected {
                frame.render_widget(Block::default().style(theme::slash_selected_row()), row);
            }
            match item {
                SlashMenuItem::Group {
                    group,
                    count,
                    expanded,
                } => self.render_slash_group_row(frame, row, *group, *count, *expanded),
                SlashMenuItem::Command(sm) => {
                    self.render_slash_command_row(frame, row, sm, is_selected);
                }
            }
        }

        let pos = self.slash_selected.saturating_add(1).min(total);
        let arrows = match (offset > 0, offset + visible < total) {
            (true, true) => " \u{2191}\u{2193}",
            (true, false) => " \u{2191}",
            (false, true) => " \u{2193}",
            (false, false) => "",
        };
        let footer = if self.slash_filter == "/" {
            format!(
                " \u{2191}\u{2193} navigate   \u{2192}/Enter expand   Tab complete   Esc close   {pos}/{total}{arrows}"
            )
        } else {
            format!(
                " \u{2191}\u{2193} navigate   Tab complete   Enter run   Esc close   {pos}/{total}{arrows}"
            )
        };
        frame.render_widget(
            Paragraph::new(Span::styled(footer, theme::out_dim())),
            rows[visible + 1],
        );
    }

    fn render_slash_group_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        group: SlashGroup,
        count: usize,
        expanded: bool,
    ) {
        let marker = if expanded { "\u{25be}" } else { "\u{203a}" };
        let count_text = count.to_string();
        let label = group.label();
        let pad = area
            .width
            .saturating_sub((label.len() + count_text.len() + 5) as u16) as usize;
        let line = Line::from(vec![
            Span::styled(format!(" {marker} "), theme::slash_selected()),
            Span::styled(label, theme::slash_group()),
            Span::raw(" ".repeat(pad)),
            Span::styled(count_text, theme::out_dim()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_slash_command_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        sm: &SlashMatch,
        selected: bool,
    ) {
        let command_width = area.width.saturating_sub(28).clamp(18, 34);
        let cols = Layout::horizontal([
            Constraint::Length(3),
            Constraint::Length(command_width),
            Constraint::Min(1),
        ])
        .split(area);
        let marker = if selected { "\u{25cf}" } else { "\u{00b7}" };
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {marker} "), theme::slash_selected())),
            cols[0],
        );
        frame.render_widget(Paragraph::new(self.slash_command_line(sm)), cols[1]);
        frame.render_widget(
            Paragraph::new(Span::styled(
                sm.description.as_str(),
                theme::dock_value_muted(),
            )),
            cols[2],
        );
    }

    fn slash_command_line<'a>(&self, sm: &'a SlashMatch) -> Line<'a> {
        let split_at = sm
            .matched_prefix_len
            .min(sm.command.len())
            .min(sm.command.chars().count());
        let byte_split = sm
            .command
            .char_indices()
            .nth(split_at)
            .map_or(sm.command.len(), |(idx, _)| idx);
        let (matched, rest) = sm.command.split_at(byte_split);
        Line::from(vec![
            Span::styled(matched, theme::slash_match()),
            Span::styled(rest, theme::slash_command()),
        ])
    }

    /// Renders the interactive provider connection manager panel.
    fn render_provider_panel(
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

        let mut lines: Vec<Line<'_>> = Vec::new();

        let inner_width = inner.width as usize;

        lines.push(Line::from(vec![
            Span::styled("  Provider Connections", theme::brand_word()),
            Span::raw(" ".repeat(inner_width.saturating_sub(48))),
            Span::styled("(Esc to close)", theme::out_dim()),
        ]));
        lines.push(Line::from(""));

        for (i, (name, desc)) in PROVIDER_KINDS.iter().enumerate() {
            let Some(kind) = parse_provider_kind_by_index(i) else {
                continue;
            };
            let is_sel = i == selected;
            let prefix = if is_sel { "  \u{25cf} " } else { "    " };
            let status = if let Some(index) = configured_provider_index(&self.config, &kind) {
                let provider = &self.config.providers[index];
                let auth = if provider.auth_token_env.is_some() {
                    "subscription"
                } else {
                    "api key"
                };
                let config_source = self.provider_config_source_label(&kind);
                format!("{:<15} {:<10} {:<12}", "configured", config_source, auth)
            } else {
                format!(
                    "{:<15} {:<10} {:<12}",
                    "not configured",
                    "missing",
                    provider_auth_summary(&kind)
                )
            };
            let text = format!("{prefix}{}. {:<11} {:<34} {}", i + 1, name, desc, status);
            let style = if is_sel {
                theme::slash_selected()
            } else {
                theme::out_dim()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }

        match input_mode {
            Some(PanelInputMode::AddProvider(buf)) => {
                lines.push(Line::from(vec![
                    Span::styled("  Add: ", theme::out_help_cmd()),
                    Span::styled(format!("{buf}\u{2588}"), theme::brand_word()),
                    Span::styled("  (provider model api_key_env)", theme::out_dim()),
                ]));
            }
            Some(PanelInputMode::ConfirmDelete) => {
                let provider_name = parse_provider_kind_by_index(selected)
                    .map(|kind| provider_kind_label(&kind))
                    .unwrap_or("provider");
                lines.push(Line::from(Span::styled(
                    format!("  Delete {provider_name} connection? [y/N]"),
                    theme::out_error(),
                )));
            }
            Some(PanelInputMode::AddStep1Provider { selected: step_sel }) => {
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled("  Select Provider", theme::brand_word()),
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
            Some(PanelInputMode::AddStepAuthType {
                provider,
                selected: auth_sel,
            }) => {
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Authentication for {provider}"),
                        theme::brand_word(),
                    ),
                    Span::raw(" ".repeat(inner_width.saturating_sub(60))),
                    Span::styled("(Esc to go back)", theme::out_dim()),
                ]));
                lines.push(Line::from(""));
                let options = provider_auth_modes(provider);
                for (i, auth_mode) in options.iter().enumerate() {
                    let marker = if i == *auth_sel { "> " } else { "  " };
                    let style = if i == *auth_sel {
                        theme::slash_selected()
                    } else {
                        theme::out_dim()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {marker}{}", auth_mode.title()), style),
                        Span::styled(format!("  \u{2014} {}", auth_mode.hint()), theme::out_dim()),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [\u{2191}/\u{2193}] Select  [Enter] Continue  [Esc] Back",
                    theme::out_dim(),
                )));
            }
            Some(PanelInputMode::AddStep2Model {
                provider,
                is_subscription,
                selected: model_sel,
                manual_input,
            }) => {
                lines.clear();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Default Model for {provider}"),
                        theme::brand_word(),
                    ),
                    Span::raw(" ".repeat(inner_width.saturating_sub(56))),
                    Span::styled("(Esc to go back)", theme::out_dim()),
                ]));
                lines.push(Line::from(Span::styled(
                    format!(
                        "  Auth: {}",
                        if *is_subscription {
                            "subscription"
                        } else {
                            "api key"
                        }
                    ),
                    theme::out_dim(),
                )));
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
            Some(PanelInputMode::AddStep3Key {
                provider,
                key_buf,
                is_subscription,
                ..
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
                let field_label = format!("  {label}: ");
                let field_width = field_label.chars().count();
                let masked = masked_secret_preview(
                    key_buf.chars().count(),
                    inner_width.saturating_sub(field_width),
                );
                lines.push(Line::from(vec![
                    Span::styled(field_label, theme::out_help_cmd()),
                    Span::styled(masked, theme::brand_word()),
                ]));
                if let Some((msg, style)) = status_msg {
                    let s = match style {
                        OutputStyle::Success => theme::out_success(),
                        OutputStyle::Error => theme::out_error(),
                        _ => theme::out_dim(),
                    };
                    lines.push(Line::from(Span::styled(format!("  {msg}"), s)));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [Enter] Save  [Esc] Back",
                    theme::out_dim(),
                )));
            }
            None => {
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
                    "  [Enter] Configure  [A] Add  [D] Delete  [P] Priority  [T] Test",
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
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct HomeEnvGuard(Option<OsString>);

    impl HomeEnvGuard {
        fn set(path: &std::path::Path) -> Self {
            let previous = std::env::var_os("HOME");
            // SAFETY: guarded test-only environment mutation.
            unsafe {
                std::env::set_var("HOME", path);
            }
            Self(previous)
        }
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            // SAFETY: guarded test-only environment mutation.
            unsafe {
                if let Some(previous) = &self.0 {
                    std::env::set_var("HOME", previous);
                } else {
                    std::env::remove_var("HOME");
                }
            }
        }
    }

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

    fn make_app_with_workspace_state(
        has_workspace: bool,
        show_tip: bool,
    ) -> (ReplApp, TextArea<'static>) {
        let tmp = tempfile::TempDir::new().expect("invariant: tempdir");
        let session_mgr = if has_workspace {
            Some(SessionManager::load_or_create(tmp.path()).expect("invariant: session manager"))
        } else {
            None
        };
        let app = ReplApp::new(
            DuumbiConfig::default(),
            tmp.path().to_path_buf(),
            None,
            session_mgr,
            has_workspace,
            show_tip,
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
    fn render_app_to_string(
        app: &ReplApp,
        textarea: &TextArea<'_>,
        width: u16,
        height: u16,
    ) -> (String, Vec<String>) {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("invariant: test terminal");
        terminal
            .draw(|frame| app.render(frame, textarea))
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
        (joined, rows)
    }

    fn render_to_string(width: u16, height: u16) -> (String, Vec<String>) {
        let (app, textarea) = make_app();
        render_app_to_string(&app, &textarea, width, height)
    }

    #[test]
    fn empty_workspace_tip_uses_single_column_copy() {
        let (app, textarea) = make_app_with_workspace_state(true, true);
        let (rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);

        assert!(rendered.contains(" empty workspace "));
        assert!(rendered.contains("TRY ONE OF THESE"));
        assert!(rendered.contains("/intent create"));
        assert!(rendered.contains("\"Build a calculator with add and multiply\""));
        assert!(!rendered.contains("Use the prompt"));

        let card_rows = rows.iter().skip(2).take(9).cloned().collect::<Vec<_>>();
        let card_chunk = card_rows.join("\n");
        assert!(
            !card_chunk.contains('\u{2500}'),
            "empty-state card should not render top/bottom borders:\n{card_chunk}"
        );
    }

    #[test]
    fn no_workspace_tip_uses_single_column_copy() {
        let (app, textarea) = make_app_with_workspace_state(false, true);
        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);

        assert!(rendered.contains(" no workspace "));
        assert!(rendered.contains("INITIALISE A WORKSPACE"));
        assert!(rendered.contains("/init"));
        assert!(rendered.contains("duumbi init"));
    }

    #[test]
    fn conversation_render_keeps_user_block_and_output() {
        let (mut app, textarea) = make_app();
        app.begin_user_block("/status");
        app.push_output("Workspace: /tmp/example", OutputStyle::Normal);
        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);

        assert!(rendered.contains("/status"));
        assert!(!rendered.contains("copy"));
        assert!(rendered.contains("Workspace: /tmp/example"));
    }

    #[test]
    fn conversation_render_shows_elapsed_footer() {
        let (mut app, textarea) = make_app();
        app.begin_user_block("/describe");
        app.push_output("function main() -> i64 {", OutputStyle::Normal);
        app.finish_current_output_elapsed(std::time::Duration::from_millis(1250));
        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);

        assert!(rendered.contains("completed in 1.25s"));
    }

    #[test]
    fn conversation_page_up_down_scrolls_rendered_blocks() {
        let (mut app, mut textarea) = make_app();
        for i in 0..20 {
            app.begin_user_block(&format!("/status {i}"));
            app.push_output(format!("line {i}"), OutputStyle::Normal);
        }

        let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        app.handle_key(page_up, &mut textarea);
        assert!(app.output_scroll_offset > 0);

        let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        app.handle_key(page_down, &mut textarea);
        assert_eq!(app.output_scroll_offset, 0);
    }

    #[test]
    fn conversation_feedback_blocks_render_with_spacing() {
        let (mut app, textarea) = make_app();
        app.begin_user_block("/help");
        app.push_output("Slash commands:", OutputStyle::Normal);
        app.push_feedback("Copied message.", OutputStyle::Success);
        app.push_feedback("Copied message.", OutputStyle::Success);

        let (_rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);
        let help_row = rows
            .iter()
            .position(|line| line.contains("Slash commands:"))
            .expect("help output row must render");
        let copied_rows = rows
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| line.contains("Copied message.").then_some(idx))
            .collect::<Vec<_>>();

        assert_eq!(copied_rows.len(), 2);
        assert!(
            rows[help_row + 1].trim().is_empty(),
            "feedback should not touch previous command output"
        );
        assert!(
            rows[copied_rows[0] + 1].trim().is_empty(),
            "consecutive feedback blocks should be separated"
        );
    }

    #[test]
    fn conversation_click_selects_and_toggles_user_block() {
        let (mut app, textarea) = make_app();
        app.begin_user_block("/status");
        let (_rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);
        let row = rows
            .iter()
            .position(|line| line.contains("/status"))
            .expect("user block row must render") as u16;

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.selected_conversation_block, Some(0));

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.selected_conversation_block, None);
    }

    #[test]
    fn conversation_keyboard_selection_opens_action_menu() {
        let (mut app, mut textarea) = make_app();
        app.begin_user_block("/status");
        app.begin_user_block("/help");

        app.handle_key(
            KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL),
            &mut textarea,
        );
        assert_eq!(app.selected_conversation_block, Some(1));

        app.handle_key(
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            &mut textarea,
        );
        assert!(app.conversation_action_menu.is_some());

        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);
        assert!(rendered.contains("Copy message"));
    }

    #[test]
    fn conversation_selected_block_shows_action_menu_trigger() {
        let (mut app, textarea) = make_app();
        app.begin_user_block("/status");
        let (_rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);
        let row = rows
            .iter()
            .position(|line| line.contains("/status"))
            .expect("user block row must render") as u16;

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row,
            modifiers: KeyModifiers::NONE,
        });
        let (rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);
        assert!(rendered.contains("..."));

        let menu_row = rows
            .iter()
            .position(|line| line.contains("..."))
            .expect("selected block must expose menu trigger") as u16;
        let menu_col = rows[menu_row as usize]
            .find("...")
            .expect("menu trigger must be findable") as u16;
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: menu_col,
            row: menu_row,
            modifiers: KeyModifiers::NONE,
        });

        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);
        assert!(rendered.contains("Copy message"));
    }

    #[test]
    fn conversation_drag_selection_collects_text() {
        let (mut app, textarea) = make_app();
        app.push_output("alpha beta gamma", OutputStyle::Normal);
        let (_rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);
        let row = rows
            .iter()
            .position(|line| line.contains("alpha beta gamma"))
            .expect("output row must render") as u16;
        let start_col = rows[row as usize]
            .find("beta")
            .expect("selection start must be findable") as u16;
        let end_col = start_col + "beta".len() as u16;

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: start_col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Drag(MouseButton::Left),
            column: end_col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        let area = app
            .last_conversation_area
            .get()
            .expect("conversation area must be cached");
        assert_eq!(
            app.selected_conversation_text(area).as_deref(),
            Some("beta")
        );
    }

    #[test]
    fn conversation_mouse_up_clears_drag_selection() {
        let (mut app, textarea) = make_app();
        app.push_output("alpha beta gamma", OutputStyle::Normal);
        let (_rendered, rows) = render_app_to_string(&app, &textarea, 120, 30);
        let row = rows
            .iter()
            .position(|line| line.contains("alpha beta gamma"))
            .expect("output row must render") as u16;
        let col = rows[row as usize]
            .find("beta")
            .expect("selection point must be findable") as u16;
        let area = app
            .last_conversation_area
            .get()
            .expect("conversation area must be cached");
        let point = app
            .conversation_point_at(area, col, row)
            .expect("point must map to conversation row");
        app.conversation_text_selection = Some(ConversationTextSelection {
            anchor: point,
            focus: point,
            dragged: true,
        });

        let redrew = app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Up(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert!(redrew);
        assert!(app.conversation_text_selection.is_none());
    }

    #[test]
    fn conversation_menu_limits_revert_to_latest_revertable_block() {
        let (mut app, _textarea) = make_app();
        app.begin_user_block("first change");
        app.conversation_blocks[0]
            .actions
            .push(ConversationAction::Revert);
        app.begin_user_block("second change");
        app.conversation_blocks[1]
            .actions
            .push(ConversationAction::Revert);

        assert_eq!(
            app.conversation_menu_actions(0),
            vec![ConversationAction::Copy]
        );
        assert_eq!(
            app.conversation_menu_actions(1),
            vec![ConversationAction::Copy, ConversationAction::Revert]
        );
    }

    #[test]
    fn slash_menu_discovery_render_shows_collapsed_groups() {
        let (mut app, textarea) = make_app();
        app.update_slash_matches("/");
        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);

        assert!(rendered.contains("DISCOVER"));
        assert!(rendered.contains("BUILD & RUN"));
        assert!(rendered.contains("INTENT"));
        assert!(rendered.contains("SYSTEM"));
        assert!(!rendered.contains("/build"));
    }

    #[test]
    fn slash_menu_filter_render_shows_prefix_matches() {
        let (mut app, textarea) = make_app();
        app.update_slash_matches("/in");
        let (rendered, _rows) = render_app_to_string(&app, &textarea, 120, 30);

        assert!(rendered.contains("FILTER"));
        assert!(rendered.contains("/intent"));
        assert!(rendered.contains("/intent create"));
        assert!(rendered.contains("/init"));
    }

    #[test]
    fn slash_menu_enter_expands_group_then_runs_command() {
        let (mut app, mut textarea) = make_app();
        app.update_slash_matches("/");

        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = app.handle_key(enter, &mut textarea);
        assert!(matches!(action, Action::Continue));
        assert!(
            app.slash_rows
                .iter()
                .any(|row| matches!(row, SlashMenuItem::Command(sm) if sm.command == "/build"))
        );

        app.slash_selected = 1;
        let action = app.handle_key(enter, &mut textarea);
        assert!(matches!(action, Action::Submit(cmd) if cmd == "/build"));
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
        assert!(!app.slash_rows.is_empty());
        assert!(app.slash_matches.iter().any(|m| m.command == "/build"));
    }

    #[test]
    fn update_slash_matches_clears_without_slash() {
        let (mut app, _) = make_app();
        app.update_slash_matches("/bui");
        assert!(!app.slash_matches.is_empty());
        app.update_slash_matches("hello");
        assert!(app.slash_matches.is_empty());
        assert!(app.slash_rows.is_empty());
    }

    #[test]
    fn slash_menu_collects_all_matches() {
        let (mut app, _) = make_app();
        // "/" matches everything — should return all commands
        app.update_slash_matches("/");
        // There are more than 5 total slash commands
        assert!(app.slash_matches.len() > 5);
        assert!(app.slash_rows.iter().all(|row| matches!(
            row,
            SlashMenuItem::Group {
                expanded: false,
                ..
            }
        )));
    }

    #[test]
    fn slash_menu_filter_rows_include_exact_match_while_typing() {
        let (mut app, _) = make_app();
        app.update_slash_matches("/build");

        assert!(app.slash_matches.iter().any(|m| m.command == "/build"));
        assert!(
            app.slash_rows
                .iter()
                .any(|row| matches!(row, SlashMenuItem::Command(sm) if sm.command == "/build"))
        );
    }

    #[test]
    fn slash_menu_scroll_offset_adjusts_on_down() {
        let (mut app, mut textarea) = make_app();
        app.update_slash_matches("/de");
        let total = app.slash_rows.len();
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
        app.update_slash_matches("/de");

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
        assert!(app.slash_rows.is_empty());
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
    fn provider_panel_esc_closes_panel() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
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
    fn provider_panel_up_down_stays_in_bounds() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 0,
            input_mode: None,
            status_msg: None,
        };
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(down, &mut textarea);
        if let PanelState::ProviderManager { selected, .. } = &app.panel {
            assert_eq!(*selected, 1);
        }

        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        app.handle_key(up, &mut textarea);
        if let PanelState::ProviderManager { selected, .. } = &app.panel {
            assert_eq!(*selected, 0);
        }
    }

    #[test]
    fn provider_capability_table_exposes_only_api_key_setup() {
        for kind in [
            crate::config::ProviderKind::Anthropic,
            crate::config::ProviderKind::OpenAI,
            crate::config::ProviderKind::Grok,
            crate::config::ProviderKind::OpenRouter,
            crate::config::ProviderKind::MiniMax,
        ] {
            assert_eq!(provider_auth_summary(&kind), "api key");
            assert_eq!(provider_auth_modes(&kind), &[ProviderAuthMode::ApiKey]);
        }
    }

    #[test]
    fn provider_panel_a_starts_setup_for_selected_provider() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 1,
            input_mode: None,
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(matches!(
                input_mode,
                Some(PanelInputMode::AddStep2Model {
                    provider: crate::config::ProviderKind::OpenAI,
                    is_subscription: false,
                    ..
                })
            ));
        }
    }

    #[test]
    fn provider_panel_add_provider_esc_cancels() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 0,
            input_mode: Some(PanelInputMode::AddProvider("partial".to_string())),
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(input_mode.is_none());
        } else {
            panic!("panel should still be ProviderManager after Esc in AddProvider mode");
        }
    }

    #[test]
    fn provider_panel_enter_skips_auth_step_for_api_key_only_provider() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 1,
            input_mode: None,
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(matches!(
                input_mode,
                Some(PanelInputMode::AddStep2Model {
                    provider: crate::config::ProviderKind::OpenAI,
                    is_subscription: false,
                    ..
                })
            ));
        } else {
            panic!("panel should remain ProviderManager");
        }
    }

    #[test]
    fn provider_panel_direct_providers_do_not_offer_subscription_setup() {
        for selected in 0..PROVIDER_KINDS.len() {
            let (mut app, mut textarea) = make_app();
            app.panel = PanelState::ProviderManager {
                selected,
                input_mode: None,
                status_msg: None,
            };
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            app.handle_key(key, &mut textarea);
            if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
                assert!(matches!(
                    input_mode,
                    Some(PanelInputMode::AddStep2Model {
                        is_subscription: false,
                        ..
                    })
                ));
            } else {
                panic!("panel should remain ProviderManager");
            }
        }
    }

    #[test]
    fn provider_panel_model_enter_opens_api_key_input() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: Some(PanelInputMode::AddStep2Model {
                provider: crate::config::ProviderKind::MiniMax,
                is_subscription: false,
                selected: 0,
                manual_input: None,
            }),
            status_msg: None,
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);

        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(matches!(
                input_mode,
                Some(PanelInputMode::AddStep3Key {
                    provider: crate::config::ProviderKind::MiniMax,
                    model,
                    key_buf,
                    is_subscription: false,
                }) if model == "MiniMax-M2.7" && key_buf.is_empty()
            ));
        } else {
            panic!("panel should remain ProviderManager");
        }
    }

    #[test]
    fn provider_panel_upsert_without_workspace_targets_user_config() {
        let (mut app, _) = make_app();
        app.has_workspace = false;

        app.upsert_provider_connection(crate::config::ProviderConfig {
            provider: crate::config::ProviderKind::MiniMax,
            role: crate::config::ProviderRole::Primary,
            model: "MiniMax-M2.7".to_string(),
            api_key_env: "MINIMAX_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        });

        assert!(
            configured_provider_index(&app.user_config, &crate::config::ProviderKind::MiniMax)
                .is_some()
        );
        assert!(
            configured_provider_index(&app.config, &crate::config::ProviderKind::MiniMax).is_some()
        );
        assert_eq!(app.provider_config_source, ProviderConfigSource::User);
    }

    #[test]
    fn provider_panel_workspace_provider_overrides_user_provider() {
        let (mut app, _) = make_app();
        app.user_config
            .providers
            .push(crate::config::ProviderConfig {
                provider: crate::config::ProviderKind::OpenAI,
                role: crate::config::ProviderRole::Primary,
                model: "gpt-4o-mini".to_string(),
                api_key_env: "OPENAI_API_KEY".to_string(),
                base_url: None,
                timeout_secs: None,
                key_storage: None,
                auth_token_env: None,
            });
        app.workspace_config
            .providers
            .push(crate::config::ProviderConfig {
                provider: crate::config::ProviderKind::OpenAI,
                role: crate::config::ProviderRole::Primary,
                model: "gpt-4o".to_string(),
                api_key_env: "OPENAI_API_KEY".to_string(),
                base_url: None,
                timeout_secs: None,
                key_storage: None,
                auth_token_env: None,
            });

        app.refresh_effective_config();

        assert_eq!(app.provider_config_source, ProviderConfigSource::Workspace);
        assert_eq!(app.config.providers[0].model, "gpt-4o");
        assert_eq!(
            app.provider_config_source_label(&crate::config::ProviderKind::OpenAI),
            "workspace"
        );
    }

    #[test]
    fn provider_panel_api_key_enter_saves_file_and_returns_to_list() {
        let _guard = ENV_LOCK.lock().expect("invariant: env lock");
        let home = tempfile::TempDir::new().expect("invariant: temp home");
        let _home_guard = HomeEnvGuard::set(home.path());

        let (mut app, mut textarea) = make_app();
        app.has_workspace = false;
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: Some(PanelInputMode::AddStep3Key {
                provider: crate::config::ProviderKind::MiniMax,
                model: "MiniMax-M2.7".to_string(),
                key_buf: "sk-test-key".to_string(),
                is_subscription: false,
            }),
            status_msg: None,
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);

        if let PanelState::ProviderManager {
            input_mode,
            status_msg,
            ..
        } = &app.panel
        {
            assert!(input_mode.is_none());
            assert!(status_msg.is_none());
        } else {
            panic!("panel should remain ProviderManager");
        }
        let provider = app
            .config
            .providers
            .iter()
            .find(|provider| provider.provider == crate::config::ProviderKind::MiniMax)
            .expect("invariant: provider should be configured");
        assert_eq!(provider.key_storage, Some(crate::config::KeyStorage::File));
        let credentials = std::fs::read_to_string(home.path().join(".duumbi/credentials.toml"))
            .expect("invariant: credentials file should exist");
        assert!(credentials.contains("MINIMAX_API_KEY"));
        assert!(credentials.contains("sk-test-key"));
    }

    #[test]
    fn provider_panel_invalid_api_key_stays_in_input_with_error() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: Some(PanelInputMode::AddStep3Key {
                provider: crate::config::ProviderKind::MiniMax,
                model: "MiniMax-M2.7".to_string(),
                key_buf: "123".to_string(),
                is_subscription: false,
            }),
            status_msg: None,
        };

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);

        if let PanelState::ProviderManager {
            input_mode,
            status_msg,
            ..
        } = &app.panel
        {
            assert!(matches!(
                input_mode,
                Some(PanelInputMode::AddStep3Key { .. })
            ));
            assert_eq!(
                status_msg.as_ref().map(|(msg, _)| msg.as_str()),
                Some("API key looks too short.")
            );
        } else {
            panic!("panel should remain ProviderManager");
        }
        assert!(app.config.providers.is_empty());
    }

    #[test]
    fn provider_panel_configured_status_is_compact() {
        let (mut app, textarea) = make_app();
        app.user_config
            .providers
            .push(crate::config::ProviderConfig {
                provider: crate::config::ProviderKind::MiniMax,
                role: crate::config::ProviderRole::Primary,
                model: "MiniMax-M2.7".to_string(),
                api_key_env: "MINIMAX_API_KEY".to_string(),
                base_url: None,
                timeout_secs: None,
                key_storage: Some(crate::config::KeyStorage::File),
                auth_token_env: None,
            });
        app.refresh_effective_config();
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: None,
            status_msg: None,
        };

        let (rendered, _) = render_app_to_string(&app, &textarea, 180, 40);

        assert!(rendered.contains("configured      user       api key"));
        assert!(rendered.contains("not configured  missing    api key"));
        assert!(!rendered.contains("priority"));
        assert!(!rendered.contains("fallback"));
        assert!(!rendered.contains("default MiniMax-M2.7"));
    }

    #[test]
    fn provider_panel_paste_goes_to_api_key_field_not_prompt() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: Some(PanelInputMode::AddStep3Key {
                provider: crate::config::ProviderKind::MiniMax,
                model: "MiniMax-M2.7".to_string(),
                key_buf: String::new(),
                is_subscription: false,
            }),
            status_msg: None,
        };

        app.handle_paste("sk-test-key\n", &mut textarea);

        assert_eq!(textarea.lines(), &[""]);
        if let PanelState::ProviderManager {
            input_mode:
                Some(PanelInputMode::AddStep3Key {
                    key_buf,
                    is_subscription,
                    ..
                }),
            ..
        } = &app.panel
        {
            assert_eq!(key_buf, "sk-test-key");
            assert!(!is_subscription);
        } else {
            panic!("panel should remain in API key input mode");
        }
    }

    #[test]
    fn provider_panel_main_list_paste_does_not_change_prompt() {
        let (mut app, mut textarea) = make_app();
        textarea.insert_str("/provider");
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: None,
            status_msg: None,
        };

        app.handle_paste("API key megadásakor", &mut textarea);

        assert_eq!(textarea.lines(), &["/provider"]);
        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(input_mode.is_none());
        } else {
            panic!("panel should remain ProviderManager");
        }
    }

    #[test]
    fn masked_secret_preview_shows_paste_feedback_without_full_secret() {
        let preview = masked_secret_preview(64, 24);
        assert!(preview.contains("(64 chars)"));
        assert!(preview.starts_with('.'));
        assert!(!preview.contains("sk-"));
        assert!(preview.chars().count() <= 24);
    }

    #[test]
    fn provider_panel_clipboard_shortcut_does_not_type_into_api_key_field() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 4,
            input_mode: Some(PanelInputMode::AddStep3Key {
                provider: crate::config::ProviderKind::MiniMax,
                model: "MiniMax-M2.7".to_string(),
                key_buf: String::new(),
                is_subscription: false,
            }),
            status_msg: None,
        };

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.handle_key(key, &mut textarea);

        if let PanelState::ProviderManager {
            input_mode: Some(PanelInputMode::AddStep3Key { key_buf, .. }),
            ..
        } = &app.panel
        {
            assert!(key_buf.is_empty());
        } else {
            panic!("panel should remain in API key input mode");
        }
    }

    #[test]
    fn provider_panel_delete_removes_configured_connection() {
        let (mut app, mut textarea) = make_app();
        app.config.providers.push(crate::config::ProviderConfig {
            provider: crate::config::ProviderKind::Anthropic,
            role: crate::config::ProviderRole::Primary,
            model: "claude-sonnet-4-6".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        });
        app.panel = PanelState::ProviderManager {
            selected: 0,
            input_mode: Some(PanelInputMode::ConfirmDelete),
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        assert!(app.config.providers.is_empty());
    }

    #[test]
    fn provider_panel_auth_back_navigation_returns_to_main_list() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 0,
            input_mode: Some(PanelInputMode::AddStepAuthType {
                provider: crate::config::ProviderKind::Anthropic,
                selected: 0,
            }),
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(input_mode.is_none());
        } else {
            panic!("panel should remain ProviderManager");
        }
    }

    #[test]
    fn provider_panel_model_back_navigation_returns_to_main_list() {
        let (mut app, mut textarea) = make_app();
        app.panel = PanelState::ProviderManager {
            selected: 0,
            input_mode: Some(PanelInputMode::AddStep2Model {
                provider: crate::config::ProviderKind::Anthropic,
                is_subscription: false,
                selected: 0,
                manual_input: None,
            }),
            status_msg: None,
        };
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(key, &mut textarea);
        if let PanelState::ProviderManager { input_mode, .. } = &app.panel {
            assert!(input_mode.is_none());
        } else {
            panic!("panel should remain ProviderManager");
        }
    }
}
