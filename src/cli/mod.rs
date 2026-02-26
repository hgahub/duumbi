//! CLI module.
//!
//! Command-line interface using `clap` for the duumbi compiler.

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
    /// Compile a JSON-LD graph to a native binary.
    Build {
        /// Path to the input `.jsonld` file.
        input: PathBuf,

        /// Path for the output binary (default: `output`).
        #[arg(short, long, default_value = "output")]
        output: PathBuf,
    },
}
