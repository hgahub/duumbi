//! REPL mode system for Agent / Intent dual-mode interaction.
//!
//! The REPL supports two modes toggled by Shift+Tab:
//! - **Agent** — free-form AI mutation (default)
//! - **Intent** — intent-focused planning and modification

/// The two REPL interaction modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplMode {
    /// Free-form AI mutation mode (default).
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

impl Default for ReplMode {
    fn default() -> Self {
        Self::Agent
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

/// A slash command match for the inline menu.
#[derive(Debug, Clone)]
pub struct SlashMatch {
    /// Full command string (e.g. "/intent create").
    pub command: String,
    /// Description text.
    pub description: String,
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

/// Active interactive panel rendered below the prompt.
// ModelSelector is constructed by the /model slash command handler in repl.rs
// and by tests; the variant is intentionally used only when the panel is opened.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub enum PanelState {
    /// No panel open — normal REPL mode.
    #[default]
    None,
    /// Model/provider selector panel.
    ModelSelector {
        /// Index of highlighted provider (0-based).
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
    /// Step 2: Selecting a model for the chosen provider.
    AddStep2Model {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// Index of highlighted model in the recommendations list.
        selected: usize,
        /// When `Some`, the user is typing a manual model name.
        manual_input: Option<String>,
    },
    /// Step 3: Entering the API key (characters are masked).
    AddStep3Key {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// The model chosen in step 2.
        model: String,
        /// Raw key text (shown masked in UI).
        key_buf: String,
    },
    /// Step 3 confirmation: choose keychain vs session-only storage.
    AddStep3Confirm {
        /// The provider kind chosen in step 1.
        provider: crate::config::ProviderKind,
        /// The model chosen in step 2.
        model: String,
        /// The API key entered in step 3.
        key: String,
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
