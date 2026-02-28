//! CLI module.
//!
//! Command-line interface using `clap` for the duumbi compiler.

pub mod describe;
pub mod init;

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

    /// Start the web visualizer for the semantic graph.
    Viz {
        /// Port to listen on.
        #[arg(short, long, default_value_t = 8420)]
        port: u16,

        /// Serve frontend assets from disk (development mode).
        #[arg(long)]
        dev: bool,

        /// Path to the input `.jsonld` file (optional if in a workspace).
        input: Option<PathBuf>,
    },
}
