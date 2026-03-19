//! Tab-completion and hinting for the interactive REPL.
//!
//! Implements [`reedline::Completer`] for slash commands and dynamic arguments
//! (intent slugs, session indices), and [`reedline::Hinter`] for inline ghost-text.

use std::path::Path;

use reedline::{Completer, Hinter, Span, Suggestion};

// ---------------------------------------------------------------------------
// Static slash command list
// ---------------------------------------------------------------------------

/// All known slash commands with descriptions (used for completion and help).
pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/build", "Compile the current graph to a native binary"),
    ("/run", "Run the compiled binary"),
    ("/check", "Validate the graph without compiling"),
    ("/describe", "Print human-readable pseudocode"),
    ("/undo", "Restore the previous graph snapshot"),
    ("/status", "Show workspace and session info"),
    ("/history", "Show session conversation history"),
    ("/model", "Show the current LLM model"),
    ("/intent", "Manage intent specs"),
    ("/intent create", "Generate a new intent spec"),
    ("/intent review", "Show intent details"),
    ("/intent execute", "Execute an intent end-to-end"),
    ("/intent status", "Show intent status"),
    ("/knowledge", "Show knowledge statistics"),
    ("/knowledge list", "List all knowledge nodes"),
    ("/knowledge stats", "Show learning statistics"),
    ("/resume", "List or resume archived sessions"),
    ("/search", "Search registries for modules"),
    ("/publish", "Package and publish current module"),
    ("/registry list", "List configured registries"),
    ("/deps", "Manage dependencies"),
    ("/deps list", "List declared dependencies"),
    ("/deps audit", "Verify dependency integrity"),
    ("/deps tree", "Show the dependency tree"),
    ("/deps update", "Update dependencies"),
    ("/deps vendor", "Vendor cached dependencies"),
    ("/deps install", "Download and resolve all dependencies"),
    ("/clear", "Clear session state"),
    ("/clear chat", "Clear current chat history"),
    ("/clear session", "Clear and archive current session"),
    ("/clear all", "Clear history and session state"),
    ("/help", "Show available commands"),
    ("/exit", "Exit the REPL"),
    ("/quit", "Exit the REPL"),
];

/// Creates a [`Suggestion`] with all required fields.
fn make_suggestion(
    value: String,
    description: Option<String>,
    span: Span,
    append_whitespace: bool,
) -> Suggestion {
    Suggestion {
        value,
        display_override: None,
        description,
        style: None,
        extra: None,
        span,
        append_whitespace,
        match_indices: None,
    }
}

// ---------------------------------------------------------------------------
// SlashCommandCompleter
// ---------------------------------------------------------------------------

/// Reedline [`Completer`] for REPL slash commands with dynamic argument completion.
pub struct SlashCommandCompleter {
    /// Workspace root for dynamic scanning (intent slugs, sessions).
    workspace_root: std::path::PathBuf,
}

impl SlashCommandCompleter {
    /// Creates a new completer bound to the given workspace.
    #[must_use]
    pub fn new(workspace_root: std::path::PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl Completer for SlashCommandCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let input = &line[..pos];

        // Only complete slash commands
        if !input.starts_with('/') {
            return Vec::new();
        }

        // Check if we need dynamic completion (e.g. "/intent execute <slug>")
        if let Some(suggestions) = try_dynamic_completion(input, &self.workspace_root) {
            return suggestions;
        }

        // Static slash command prefix matching
        let mut suggestions = Vec::new();
        for &(cmd, desc) in SLASH_COMMANDS {
            if cmd.starts_with(input) && cmd != input {
                suggestions.push(make_suggestion(
                    cmd.to_string(),
                    Some(desc.to_string()),
                    Span::new(0, pos),
                    true,
                ));
            }
        }

        suggestions
    }
}

// ---------------------------------------------------------------------------
// Dynamic completion helpers
// ---------------------------------------------------------------------------

/// Attempts dynamic argument completion for commands that take runtime arguments.
///
/// Returns `Some(suggestions)` if the input matches a pattern with dynamic args,
/// `None` to fall back to static completion.
fn try_dynamic_completion(input: &str, workspace: &Path) -> Option<Vec<Suggestion>> {
    let parts: Vec<&str> = input.splitn(3, ' ').collect();

    match parts.as_slice() {
        // /intent execute <TAB>
        ["/intent", "execute", prefix] => Some(complete_intent_slugs(workspace, prefix, input)),
        ["/intent", "execute"] => Some(complete_intent_slugs(workspace, "", input)),
        // /intent review <TAB>
        ["/intent", "review", prefix] => Some(complete_intent_slugs(workspace, prefix, input)),
        ["/intent", "review"] => Some(complete_intent_slugs(workspace, "", input)),
        // /intent status <TAB>
        ["/intent", "status", prefix] => Some(complete_intent_slugs(workspace, prefix, input)),
        ["/intent", "status"] => Some(complete_intent_slugs(workspace, "", input)),
        // /resume <TAB>
        ["/resume", prefix] => Some(complete_session_indices(workspace, prefix, input)),
        ["/resume"] if input.ends_with(' ') => Some(complete_session_indices(workspace, "", input)),
        _ => None,
    }
}

/// Scans `.duumbi/intents/` for YAML files and suggests matching slugs.
fn complete_intent_slugs(workspace: &Path, prefix: &str, input: &str) -> Vec<Suggestion> {
    let intents_dir = workspace.join(".duumbi/intents");
    let entries = match std::fs::read_dir(&intents_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let prefix_start = input.rfind(' ').map(|p| p + 1).unwrap_or(0);

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                return None;
            }
            let slug = path.file_stem()?.to_str()?.to_string();
            if slug.starts_with(prefix) {
                let base_cmd = &input[..prefix_start];
                Some(make_suggestion(
                    format!("{base_cmd}{slug}"),
                    None,
                    Span::new(0, input.len()),
                    false,
                ))
            } else {
                None
            }
        })
        .collect()
}

/// Lists archived session indices for `/resume <N>` completion.
fn complete_session_indices(workspace: &Path, prefix: &str, input: &str) -> Vec<Suggestion> {
    let history_dir = workspace.join(".duumbi/session/history");
    let count = match std::fs::read_dir(&history_dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .count(),
        Err(_) => return Vec::new(),
    };

    if count == 0 {
        return Vec::new();
    }

    let prefix_start = input.rfind(' ').map(|p| p + 1).unwrap_or(0);
    let base_cmd = &input[..prefix_start];

    (1..=count)
        .map(|i| i.to_string())
        .filter(|s| s.starts_with(prefix))
        .map(|idx| {
            make_suggestion(
                format!("{base_cmd}{idx}"),
                None,
                Span::new(0, input.len()),
                false,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SlashCommandHinter
// ---------------------------------------------------------------------------

/// Reedline [`Hinter`] that shows ghost-text completion for slash commands.
pub struct SlashCommandHinter {
    current_hint: String,
}

impl SlashCommandHinter {
    /// Creates a new hinter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_hint: String::new(),
        }
    }
}

impl Default for SlashCommandHinter {
    fn default() -> Self {
        Self::new()
    }
}

impl Hinter for SlashCommandHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        _history: &dyn reedline::History,
        use_ansi_coloring: bool,
        _cwd: &str,
    ) -> String {
        self.current_hint.clear();
        let input = &line[..pos];

        if !input.starts_with('/') || input.contains(' ') {
            return String::new();
        }

        // Find the first matching slash command
        for &(cmd, _desc) in SLASH_COMMANDS {
            if cmd.starts_with(input) && cmd != input {
                let suffix = &cmd[input.len()..];
                self.current_hint = suffix.to_string();
                if use_ansi_coloring {
                    return format!("\x1b[90m{suffix}\x1b[0m");
                }
                return suffix.to_string();
            }
        }

        String::new()
    }

    fn complete_hint(&self) -> String {
        self.current_hint.clone()
    }

    fn next_hint_token(&self) -> String {
        // Return the whole hint as a single token
        self.current_hint.clone()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_completion_prefix_matching() {
        let tmp = tempfile::TempDir::new().expect("invariant: tempdir");
        let mut completer = SlashCommandCompleter::new(tmp.path().to_path_buf());

        let suggestions = completer.complete("/bui", 4);
        assert!(
            suggestions.iter().any(|s| s.value == "/build"),
            "should suggest /build for /bui prefix"
        );
    }

    #[test]
    fn no_completion_without_slash() {
        let tmp = tempfile::TempDir::new().expect("invariant: tempdir");
        let mut completer = SlashCommandCompleter::new(tmp.path().to_path_buf());

        let suggestions = completer.complete("hello", 5);
        assert!(suggestions.is_empty(), "no completions for non-slash input");
    }

    #[test]
    fn intent_subcommand_completion() {
        let tmp = tempfile::TempDir::new().expect("invariant: tempdir");
        let mut completer = SlashCommandCompleter::new(tmp.path().to_path_buf());

        let suggestions = completer.complete("/intent ", 8);
        assert!(
            suggestions.iter().any(|s| s.value.contains("create")),
            "should suggest /intent create"
        );
    }

    #[test]
    fn dynamic_intent_slug_completion() {
        let tmp = tempfile::TempDir::new().expect("invariant: tempdir");
        let intents_dir = tmp.path().join(".duumbi/intents");
        std::fs::create_dir_all(&intents_dir).expect("invariant: create intents dir");
        std::fs::write(intents_dir.join("calculator.yaml"), "intent: test").expect("write");

        let mut completer = SlashCommandCompleter::new(tmp.path().to_path_buf());
        let suggestions = completer.complete("/intent execute ", 16);
        assert!(
            suggestions.iter().any(|s| s.value.contains("calculator")),
            "should suggest calculator slug"
        );
    }

    #[test]
    fn hinter_suggests_suffix() {
        let history = reedline::FileBackedHistory::new(0).expect("invariant: history");
        let mut h = SlashCommandHinter::new();
        let hint = h.handle("/bui", 4, &history, false, ".");
        assert_eq!(hint, "ld", "should suggest 'ld' to complete /build");
    }

    #[test]
    fn hinter_no_suggestion_for_full_command() {
        let history = reedline::FileBackedHistory::new(0).expect("invariant: history");
        let mut h = SlashCommandHinter::new();
        let hint = h.handle("/build", 6, &history, false, ".");
        assert!(hint.is_empty(), "no hint for exact match");
    }

    #[test]
    fn hinter_no_suggestion_after_space() {
        let history = reedline::FileBackedHistory::new(0).expect("invariant: history");
        let mut h = SlashCommandHinter::new();
        let hint = h.handle("/intent create", 14, &history, false, ".");
        assert!(hint.is_empty(), "no hint when input contains space");
    }
}
