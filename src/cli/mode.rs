//! REPL mode system for Agent / Intent dual-mode interaction.
//!
//! The REPL supports two modes toggled by Shift+Tab:
//! - **Agent** — free-form AI mutation (default)
//! - **Intent** — intent-focused planning and modification

use std::path::PathBuf;

/// The two REPL interaction modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplMode {
    /// Free-form AI mutation mode (default).
    #[default]
    Agent,
    /// Intent-focused mode: input is interpreted in context of a focused intent.
    Intent,
}

impl ReplMode {
    /// Human-readable label for the current mode.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Intent => "intent",
        }
    }
}

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
    /// Full command string (e.g. "/intent create").
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
}

/// Sub-mode within an interactive panel for text input or confirmation.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PanelInputMode {
    /// Legacy: single-line text input for adding a provider.
    AddProvider(String),
    /// Step 1: Selecting provider type from a list.
    AddStep1Provider {
        /// Index of the highlighted provider kind.
        selected: usize,
    },
    /// Step 2: Choose authentication type (API Key vs Subscription Token).
    AddStepAuthType {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// 0 = API Key, 1 = Subscription Token (Bearer).
        selected: usize,
    },
    /// Step 3: Selecting a default model for the chosen provider.
    AddStep2Model {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// If true, the key is a subscription/Bearer token.
        is_subscription: bool,
        /// Index of highlighted model in the recommendations list.
        selected: usize,
        /// When `Some`, the user is typing a manual model name.
        manual_input: Option<String>,
    },
    /// Step 4: Choose whether to use an environment variable or enter a key.
    AddStepCredentialSource {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// The model chosen in step 2.
        model: String,
        /// If true, the key is a subscription/Bearer token.
        is_subscription: bool,
        /// 0 = environment variable, 1 = entered key stored in credentials.
        selected: usize,
    },
    /// Step 5: Entering the API key or subscription token (characters are masked).
    AddStep3Key {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// The model chosen in step 2.
        model: String,
        /// Raw key text (shown masked in UI).
        key_buf: String,
        /// If true, the key is a subscription/Bearer token.
        is_subscription: bool,
    },
    /// Waiting for y/N confirmation to delete the selected provider.
    ConfirmDelete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_default_is_agent() {
        assert_eq!(ReplMode::default(), ReplMode::Agent);
    }

    #[test]
    fn mode_labels() {
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
