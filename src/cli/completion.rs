//! Slash-command definitions for the interactive REPL.
//!
//! The static [`SLASH_COMMANDS`] table is the single source of truth for all
//! known slash commands. It is consumed by the inline menu in
//! [`super::app::ReplApp`] and by the `/help` display.

// ---------------------------------------------------------------------------
// Static slash command list
// ---------------------------------------------------------------------------

/// Slash command groups shown by the discovery menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlashGroup {
    /// Build, run, and inspect the current graph.
    BuildRun,
    /// Intent-driven development commands.
    Intent,
    /// Persistent agent knowledge commands.
    Knowledge,
    /// Dependency, registry, and publishing commands.
    DependenciesRegistry,
    /// Session and history commands.
    Session,
    /// Workspace setup and configuration commands.
    WorkspaceConfig,
    /// Help and process control commands.
    System,
}

impl SlashGroup {
    /// User-facing group label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BuildRun => "BUILD & RUN",
            Self::Intent => "INTENT",
            Self::Knowledge => "KNOWLEDGE",
            Self::DependenciesRegistry => "DEPENDENCIES & REGISTRY",
            Self::Session => "SESSION",
            Self::WorkspaceConfig => "WORKSPACE & CONFIG",
            Self::System => "SYSTEM",
        }
    }
}

/// Display order for slash-command groups.
pub const SLASH_GROUPS: &[SlashGroup] = &[
    SlashGroup::BuildRun,
    SlashGroup::Intent,
    SlashGroup::Knowledge,
    SlashGroup::DependenciesRegistry,
    SlashGroup::Session,
    SlashGroup::WorkspaceConfig,
    SlashGroup::System,
];

/// A slash command definition.
#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    /// Full command string.
    pub command: &'static str,
    /// One-line description.
    pub description: &'static str,
    /// Discovery-menu group.
    pub group: SlashGroup,
}

/// All known slash commands with descriptions (used for completion and help).
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        command: "/build",
        description: "Compile the current graph to a native binary",
        group: SlashGroup::BuildRun,
    },
    SlashCommand {
        command: "/run",
        description: "Run the compiled binary",
        group: SlashGroup::BuildRun,
    },
    SlashCommand {
        command: "/check",
        description: "Validate the graph without compiling",
        group: SlashGroup::BuildRun,
    },
    SlashCommand {
        command: "/describe",
        description: "Print human-readable pseudocode",
        group: SlashGroup::BuildRun,
    },
    SlashCommand {
        command: "/intent",
        description: "Select or clear the active intent",
        group: SlashGroup::Intent,
    },
    SlashCommand {
        command: "/intent review",
        description: "Review the active intent",
        group: SlashGroup::Intent,
    },
    SlashCommand {
        command: "/intent execute",
        description: "Execute the active intent",
        group: SlashGroup::Intent,
    },
    SlashCommand {
        command: "/intent edit",
        description: "Edit the active intent YAML",
        group: SlashGroup::Intent,
    },
    SlashCommand {
        command: "/intent delete",
        description: "Remove the active intent from active work",
        group: SlashGroup::Intent,
    },
    SlashCommand {
        command: "/query",
        description: "Ask a read-only question about the workspace",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/ask",
        description: "Alias for /query",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/agent",
        description: "Run one explicit graph mutation request",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/mode",
        description: "Switch interaction mode",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/knowledge",
        description: "Show knowledge statistics",
        group: SlashGroup::Knowledge,
    },
    SlashCommand {
        command: "/knowledge list",
        description: "List all knowledge nodes",
        group: SlashGroup::Knowledge,
    },
    SlashCommand {
        command: "/knowledge stats",
        description: "Show learning statistics",
        group: SlashGroup::Knowledge,
    },
    SlashCommand {
        command: "/knowledge show",
        description: "Show details of a knowledge node",
        group: SlashGroup::Knowledge,
    },
    SlashCommand {
        command: "/knowledge prune",
        description: "Remove old knowledge nodes",
        group: SlashGroup::Knowledge,
    },
    SlashCommand {
        command: "/search",
        description: "Search registries for modules",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/publish",
        description: "Package and publish current module",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/registry list",
        description: "List configured registries",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps",
        description: "Manage dependencies",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps list",
        description: "List declared dependencies",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps add",
        description: "Add a dependency",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps remove",
        description: "Remove a dependency",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps audit",
        description: "Verify dependency integrity",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps tree",
        description: "Show the dependency tree",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps update",
        description: "Update dependencies",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps vendor",
        description: "Vendor cached dependencies",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/deps install",
        description: "Download and resolve all dependencies",
        group: SlashGroup::DependenciesRegistry,
    },
    SlashCommand {
        command: "/status",
        description: "Show workspace and session info",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/history",
        description: "Show session conversation history",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/resume",
        description: "List or resume archived sessions",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/clear",
        description: "Clear chat history and screen",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/clear chat",
        description: "Clear chat history and screen",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/clear session",
        description: "Clear and archive current session",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/clear all",
        description: "Clear history and archive session",
        group: SlashGroup::Session,
    },
    SlashCommand {
        command: "/init",
        description: "Initialise a new workspace",
        group: SlashGroup::WorkspaceConfig,
    },
    SlashCommand {
        command: "/provider",
        description: "Manage LLM provider connections",
        group: SlashGroup::WorkspaceConfig,
    },
    SlashCommand {
        command: "/help",
        description: "Show available commands",
        group: SlashGroup::System,
    },
    SlashCommand {
        command: "/exit",
        description: "Exit the REPL",
        group: SlashGroup::System,
    },
    SlashCommand {
        command: "/quit",
        description: "Exit the REPL",
        group: SlashGroup::System,
    },
];

/// Returns the top `limit` slash commands whose name starts with `prefix`.
///
/// Excludes exact matches (i.e. when input equals the command name).
#[must_use]
#[allow(dead_code)]
pub fn match_commands(prefix: &str, limit: usize) -> Vec<(&'static str, &'static str)> {
    SLASH_COMMANDS
        .iter()
        .filter(|entry| entry.command.starts_with(prefix) && entry.command != prefix)
        .take(limit)
        .map(|entry| (entry.command, entry.description))
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
        assert!(results.iter().any(|(cmd, _)| *cmd == "/intent review"));
        assert!(results.iter().any(|(cmd, _)| *cmd == "/intent edit"));
        assert!(!results.iter().any(|(cmd, _)| *cmd == "/intent create"));
    }

    #[test]
    fn slash_commands_has_tui_intent_commands_only() {
        assert!(
            SLASH_COMMANDS
                .iter()
                .any(|entry| entry.command == "/intent")
        );
        assert!(
            SLASH_COMMANDS
                .iter()
                .any(|entry| entry.command == "/intent delete")
        );
        for removed in [
            "/intent create",
            "/intent status",
            "/intent focus",
            "/intent unfocus",
        ] {
            assert!(!SLASH_COMMANDS.iter().any(|entry| entry.command == removed));
        }
    }

    #[test]
    fn provider_command_replaces_model_in_completion() {
        assert!(
            SLASH_COMMANDS
                .iter()
                .any(|entry| entry.command == "/provider")
        );
        assert!(!SLASH_COMMANDS.iter().any(|entry| entry.command == "/model"));
    }
}
