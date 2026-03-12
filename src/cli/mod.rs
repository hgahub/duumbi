//! CLI module.
//!
//! Command-line interface using `clap` for the duumbi compiler.

pub mod commands;
pub mod deps;
pub mod describe;
pub mod init;
pub mod publish;
pub mod registry;
pub mod repl;
pub mod upgrade;
pub mod yank;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Duumbi — AI-first semantic graph compiler.
#[derive(Parser, Debug)]
#[command(name = "duumbi", version, about)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new duumbi workspace.
    Init {
        /// Optional project name (defaults to current directory name).
        name: Option<String>,
    },

    /// Compile a JSON-LD graph to a native binary.
    Build {
        /// Path to the input `.jsonld` file (optional if in a workspace).
        input: Option<PathBuf>,

        /// Path for the output binary (default: `output`).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Restrict dependency resolution to workspace and vendor layers only.
        /// Fails if any dependency is only available in the cache.
        #[arg(long)]
        offline: bool,
    },

    /// Build and run the compiled binary.
    Run {
        /// Arguments to pass to the compiled binary.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Parse and validate without compiling.
    Check {
        /// Path to the input `.jsonld` file (optional if in a workspace).
        input: Option<PathBuf>,
    },

    /// Describe the program as human-readable pseudo-code.
    Describe {
        /// Path to the input `.jsonld` file (optional if in a workspace).
        input: Option<PathBuf>,
    },

    /// Apply an AI-generated mutation to the graph (requires [llm] config).
    Add {
        /// Natural language description of the desired change.
        request: String,

        /// Apply immediately without confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Undo the last AI mutation (restores from snapshot in `.duumbi/history/`).
    Undo,

    /// Manage local path dependencies declared in `.duumbi/config.toml`.
    Deps {
        /// Dependency subcommand.
        #[command(subcommand)]
        subcommand: DepsSubcommand,
    },

    /// Search for modules in configured registries.
    Search {
        /// Search query (text-based).
        query: String,
        /// Limit search to a specific registry.
        #[arg(long)]
        registry: Option<String>,
    },

    /// Create, review, and execute intent-driven development specs.
    Intent {
        /// Intent subcommand.
        #[command(subcommand)]
        subcommand: IntentSubcommand,
    },

    /// Manage registry configurations and authentication.
    Registry {
        /// Registry subcommand.
        #[command(subcommand)]
        subcommand: RegistrySubcommand,
    },

    /// Package and publish the current module to a registry.
    Publish {
        /// Target registry name (uses default-registry if omitted).
        #[arg(long)]
        registry: Option<String>,

        /// Pack the module without uploading to the registry.
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt and publish immediately.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Mark a published module version as yanked.
    Yank {
        /// Module specifier: `@scope/name@version`.
        specifier: String,

        /// Target registry name (uses default-registry if omitted).
        #[arg(long)]
        registry: Option<String>,

        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Migrate a Phase 4-5 workspace to Phase 7 format.
    Upgrade,

    /// Start the DUUMBI Studio web platform.
    Studio {
        /// Port to listen on.
        #[arg(short, long, default_value_t = 8421)]
        port: u16,

        /// Enable development mode (hot reload).
        #[arg(long)]
        dev: bool,
    },
}

/// Subcommands for `duumbi deps`.
#[derive(Subcommand, Debug)]
pub enum DepsSubcommand {
    /// List all declared dependencies.
    List,

    /// Add a dependency (local path or from registry).
    ///
    /// For local: `duumbi deps add mymod ./path`
    /// For registry: `duumbi deps add @scope/name[@version]`
    Add {
        /// Module name or `@scope/name[@version]` specifier.
        name: String,
        /// Local path to dependency workspace (omit for registry deps).
        path: Option<String>,
        /// Registry to fetch from (overrides default-registry).
        #[arg(long)]
        registry: Option<String>,
    },

    /// Remove a declared dependency.
    Remove {
        /// Dependency name to remove.
        name: String,
    },

    /// Verify integrity of all dependencies against lockfile hashes.
    Audit,

    /// Display the dependency tree.
    Tree {
        /// Maximum tree depth to display.
        #[arg(long, default_value_t = 10)]
        depth: u32,
    },

    /// Update dependencies to latest compatible versions from registries.
    Update {
        /// Specific dependency to update (omit to update all).
        name: Option<String>,
    },

    /// Download and resolve all dependencies from registries into cache.
    Install {
        /// Fail if deps.lock would change (CI/CD reproducibility).
        #[arg(long)]
        frozen: bool,
    },

    /// Copy cached dependencies into `.duumbi/vendor/` for offline builds.
    Vendor {
        /// Vendor all dependencies regardless of config.toml [vendor] rules.
        #[arg(long)]
        all: bool,
        /// Glob pattern to match scoped module names (e.g. `"@company/*"`).
        #[arg(long)]
        include: Option<String>,
    },
}

/// Subcommands for `duumbi registry`.
#[derive(Subcommand, Debug)]
pub enum RegistrySubcommand {
    /// Add a registry endpoint.
    Add {
        /// Short name for the registry (used as key in config.toml).
        name: String,
        /// Base URL of the registry (must be HTTPS, or http://localhost for dev).
        url: String,
    },

    /// List all configured registries.
    List,

    /// Remove a registry endpoint.
    Remove {
        /// Registry name to remove.
        name: String,
    },

    /// Set the default registry for new dependencies.
    Default {
        /// Registry name to set as default.
        name: String,
    },

    /// Authenticate with a registry (stores token in ~/.duumbi/credentials.toml).
    Login {
        /// Registry name to log in to.
        registry: String,
        /// Token for non-interactive / CI use (otherwise prompts interactively).
        #[arg(long)]
        token: Option<String>,
    },

    /// Remove stored credentials for a registry.
    Logout {
        /// Registry name (omit to log out from all).
        registry: Option<String>,
    },
}

/// Subcommands for `duumbi intent`.
#[derive(Subcommand, Debug)]
pub enum IntentSubcommand {
    /// Generate a structured intent spec from a natural language description.
    Create {
        /// Natural language description of what you want to build.
        description: String,

        /// Skip confirmation prompt and save immediately.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Review (list or show details of) intent specs.
    Review {
        /// Intent name/slug to show details for. Omit to list all.
        name: Option<String>,

        /// Open intent in $EDITOR for manual editing.
        #[arg(short, long)]
        edit: bool,
    },

    /// Execute an intent: decompose → mutate graph → verify tests.
    Execute {
        /// Intent name/slug to execute.
        name: String,
    },

    /// Show status of intents (active, in-progress, failed).
    Status {
        /// Intent name/slug to show details for. Omit to list all.
        name: Option<String>,
    },
}
