//! Slash-command definitions for the interactive REPL.
//!
//! The static [`SLASH_COMMANDS`] table is the single source of truth for all
//! known slash commands. It is consumed by the inline menu in
//! [`super::app::ReplApp`] and by the `/help` display.

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
    ("/model", "Manage LLM providers and models"),
    ("/intent", "Manage intent specs"),
    ("/intent create", "Generate a new intent spec"),
    ("/intent review", "Show intent details"),
    ("/intent execute", "Execute an intent end-to-end"),
    ("/intent status", "Show intent status"),
    ("/intent focus", "Focus an intent by slug"),
    ("/intent unfocus", "Clear the focused intent"),
    ("/knowledge", "Show knowledge statistics"),
    ("/knowledge list", "List all knowledge nodes"),
    ("/knowledge stats", "Show learning statistics"),
    ("/knowledge show", "Show details of a knowledge node"),
    ("/knowledge prune", "Remove old knowledge nodes"),
    ("/resume", "List or resume archived sessions"),
    ("/search", "Search registries for modules"),
    ("/publish", "Package and publish current module"),
    ("/registry list", "List configured registries"),
    ("/deps", "Manage dependencies"),
    ("/deps list", "List declared dependencies"),
    ("/deps add", "Add a dependency"),
    ("/deps remove", "Remove a dependency"),
    ("/deps audit", "Verify dependency integrity"),
    ("/deps tree", "Show the dependency tree"),
    ("/deps update", "Update dependencies"),
    ("/deps vendor", "Vendor cached dependencies"),
    ("/deps install", "Download and resolve all dependencies"),
    ("/clear", "Clear chat history and screen"),
    ("/clear chat", "Clear chat history and screen"),
    ("/clear session", "Clear and archive current session"),
    ("/clear all", "Clear history and archive session"),
    ("/init", "Initialise a new workspace"),
    ("/help", "Show available commands"),
    ("/exit", "Exit the REPL"),
    ("/quit", "Exit the REPL"),
];

/// Returns the top `limit` slash commands whose name starts with `prefix`.
///
/// Excludes exact matches (i.e. when input equals the command name).
#[must_use]
#[allow(dead_code)]
pub fn match_commands(prefix: &str, limit: usize) -> Vec<(&'static str, &'static str)> {
    SLASH_COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(prefix) && *cmd != prefix)
        .take(limit)
        .copied()
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_commands_prefix() {
        let results = match_commands("/bui", 5);
        assert!(results.iter().any(|(cmd, _)| *cmd == "/build"));
    }

    #[test]
    fn match_commands_no_match_without_slash() {
        let results = match_commands("hello", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn match_commands_excludes_exact() {
        let results = match_commands("/build", 5);
        assert!(!results.iter().any(|(cmd, _)| *cmd == "/build"));
    }

    #[test]
    fn match_commands_respects_limit() {
        let results = match_commands("/", 3);
        assert!(results.len() <= 3);
    }

    #[test]
    fn intent_subcommands_found() {
        let results = match_commands("/intent ", 10);
        assert!(results.iter().any(|(cmd, _)| *cmd == "/intent create"));
    }

    #[test]
    fn slash_commands_has_new_intent_commands() {
        assert!(
            SLASH_COMMANDS
                .iter()
                .any(|(cmd, _)| *cmd == "/intent focus")
        );
        assert!(
            SLASH_COMMANDS
                .iter()
                .any(|(cmd, _)| *cmd == "/intent unfocus")
        );
    }
}
