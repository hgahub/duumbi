//! REPL mode system for Query / Agent / Intent interaction.
//!
//! The REPL supports three modes toggled by Shift+Tab:
//! - **Query** — read-only explanation and inspection (default)
//! - **Agent** — free-form AI mutation
//! - **Intent** — intent-focused planning and modification

use std::path::PathBuf;

/// REPL interaction mode.
pub use crate::interaction::InteractionMode as ReplMode;

/// Output line style for the scrollable output buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStyle {
    /// Normal text.
    Normal,
    /// Error text (red).
    Error,
    /// Success text (green).
    Success,
    /// Dimmed/secondary text.
    Dim,
    /// AI streaming text (cyan).
    Ai,
    /// Model-emitted thinking text.
    Thinking,
    /// Help command entry: command name (magenta) + description (white).
    Help,
}

/// A single line in the output buffer.
#[derive(Debug, Clone)]
pub struct OutputLine {
    /// Text content of the line.
    pub text: String,
    /// Visual style for rendering.
    pub style: OutputStyle,
}

impl OutputLine {
    /// Creates a new output line.
    #[must_use]
    pub fn new(text: impl Into<String>, style: OutputStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

/// Kind of a conversation-pane block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationBlockKind {
    /// User-submitted prompt or selected slash command.
    User,
    /// Assistant or command output.
    Output,
}

/// Rendering mode for assistant or command output blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputRenderMode {
    /// Plain line-by-line terminal output.
    Plain,
    /// Markdown answer text rendered into terminal styles.
    Markdown,
    /// Headered command or assistant output that can be collapsed.
    Collapsible {
        /// Header shown next to the disclosure marker.
        header: String,
        /// Whether the body is currently visible.
        expanded: bool,
        /// Style applied to the header and body text.
        style: OutputStyle,
    },
}

/// Action shown in a conversation block header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationAction {
    /// Copy the block text to the clipboard.
    Copy,
    /// Revert graph changes associated with this turn.
    Revert,
}

/// One scrollable conversation-pane block.
#[derive(Debug, Clone)]
pub struct ConversationBlock {
    /// Block type.
    pub kind: ConversationBlockKind,
    /// Output rendering behavior.
    pub render_mode: OutputRenderMode,
    /// Lines rendered inside the block.
    pub lines: Vec<OutputLine>,
    /// Time the user submitted the block, formatted for display.
    pub submitted_at: Option<String>,
    /// Runtime footer text, if applicable.
    pub elapsed: Option<String>,
    /// Header actions shown for the block.
    pub actions: Vec<ConversationAction>,
    /// Snapshot file associated with this user turn, if it changed the graph.
    pub revert_snapshot: Option<PathBuf>,
}

impl ConversationBlock {
    /// Creates a user block.
    #[must_use]
    pub fn user(input: impl Into<String>, submitted_at: impl Into<String>) -> Self {
        Self {
            kind: ConversationBlockKind::User,
            render_mode: OutputRenderMode::Plain,
            lines: vec![OutputLine::new(input, OutputStyle::Normal)],
            submitted_at: Some(submitted_at.into()),
            elapsed: None,
            actions: vec![ConversationAction::Copy],
            revert_snapshot: None,
        }
    }

    /// Creates an empty output block.
    #[must_use]
    pub fn output() -> Self {
        Self {
            kind: ConversationBlockKind::Output,
            render_mode: OutputRenderMode::Plain,
            lines: Vec::new(),
            submitted_at: None,
            elapsed: None,
            actions: Vec::new(),
            revert_snapshot: None,
        }
    }
}

/// A slash command match for the inline menu.
#[derive(Debug, Clone)]
pub struct SlashMatch {
    /// Full command string (e.g. "/intent review").
    pub command: String,
    /// Description text.
    pub description: String,
    /// Discovery-menu group.
    pub group: crate::cli::completion::SlashGroup,
    /// Number of leading command characters matched by the current input.
    pub matched_prefix_len: usize,
}

/// A visible row in the slash-command menu.
#[derive(Debug, Clone)]
pub enum SlashMenuItem {
    /// Collapsible discovery group header.
    Group {
        /// Group identity.
        group: crate::cli::completion::SlashGroup,
        /// Number of commands in the group.
        count: usize,
        /// Whether commands under this group are currently visible.
        expanded: bool,
    },
    /// Executable slash command row.
    Command(SlashMatch),
}

/// Action returned by key handling to control the event loop.
#[derive(Debug)]
pub enum Action {
    /// Continue the event loop (no-op).
    Continue,
    /// Exit the REPL.
    Exit,
    /// Submit the given input for processing.
    Submit(String),
    /// Test and save an API key submitted from the provider manager.
    ProviderKeySubmitted {
        /// Provider kind being configured.
        provider: crate::config::ProviderKind,
        /// Raw API key or token to test.
        key: String,
        /// If true, the key is a subscription/Bearer token.
        is_subscription: bool,
    },
    /// Initialize a workspace from the interactive init panel.
    InitWorkspaceSubmitted {
        /// Validated workspace display name.
        workspace_name: String,
        /// Whether an existing `.duumbi/` directory may be deleted first.
        overwrite_existing: bool,
    },
    /// Delete the active intent after confirmation.
    IntentDeleteConfirmed {
        /// Intent slug to remove from active work.
        slug: String,
    },
}

// ---------------------------------------------------------------------------
// Interactive panel state
// ---------------------------------------------------------------------------

/// Active interactive panel rendered above the REPL.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub enum PanelState {
    /// No panel open — normal REPL mode.
    #[default]
    None,
    /// Provider connection manager panel.
    ProviderManager {
        /// Index of highlighted provider kind (0-based).
        selected: usize,
        /// Sub-mode for inline input within the panel.
        input_mode: Option<PanelInputMode>,
        /// Optional status message shown in the panel footer. Cleared on next key press.
        status_msg: Option<(String, OutputStyle)>,
    },
    /// Workspace initialization panel.
    InitWorkspace {
        /// Editable workspace name buffer.
        name_buf: String,
        /// Default name offered from the current directory.
        default_name: String,
        /// Whether the current `.duumbi/` directory exists and contains entries.
        existing_non_empty: bool,
        /// Whether the panel is currently waiting for destructive re-init confirmation.
        confirm_overwrite: bool,
        /// Optional status message shown in the panel.
        status_msg: Option<(String, OutputStyle)>,
    },
    /// Active intent picker.
    IntentPicker {
        /// Loaded active intent rows. Row zero in the UI is always "new intent mode".
        intents: Vec<IntentPickerItem>,
        /// Highlighted row, including row zero.
        selected: usize,
        /// Optional status message shown in the panel.
        status_msg: Option<(String, OutputStyle)>,
    },
    /// Confirmation panel for removing an active intent from the active list.
    ConfirmIntentDelete {
        /// Intent slug being deleted.
        slug: String,
    },
}

/// Sub-mode within an interactive panel for text input or confirmation.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PanelInputMode {
    /// Step 2: Choose authentication type (API Key vs Subscription Token).
    AddStepAuthType {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// 0 = API Key, 1 = Subscription Token (Bearer).
        selected: usize,
    },
    /// Step 3: Entering the API key or subscription token (characters are masked).
    AddStep3Key {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// Raw key text (shown masked in UI).
        key_buf: String,
        /// If true, the key is a subscription/Bearer token.
        is_subscription: bool,
    },
    /// Waiting for y/N confirmation to delete the selected provider.
    ConfirmDelete,
}

/// One active intent row shown by the TUI intent picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentPickerItem {
    /// Intent slug.
    pub slug: String,
    /// Lifecycle status text.
    pub status: String,
    /// Short natural-language intent description.
    pub description: String,
    /// Number of test cases attached to the intent.
    pub test_count: usize,
}

/// Pending TUI intent creation clarification state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentDraft {
    /// Original user request before clarification.
    pub original_request: String,
    /// Questions asked by the planner model.
    pub questions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_default_is_query() {
        assert_eq!(ReplMode::default(), ReplMode::Query);
    }

    #[test]
    fn mode_labels() {
        assert_eq!(ReplMode::Query.label(), "query");
        assert_eq!(ReplMode::Agent.label(), "agent");
        assert_eq!(ReplMode::Intent.label(), "intent");
    }

    #[test]
    fn output_line_new() {
        let line = OutputLine::new("hello", OutputStyle::Normal);
        assert_eq!(line.text, "hello");
        assert_eq!(line.style, OutputStyle::Normal);
    }
}
