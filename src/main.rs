//! Duumbi CLI entry point.
//!
//! Orchestrates the full compilation pipeline: parse → graph → validate →
//! compile → link. Uses `anyhow` for error handling at the application boundary.

mod cli;
mod compiler;
mod errors;
mod graph;
mod parser;
mod types;

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Commands};
use compiler::linker;
use compiler::lowering;
use errors::Diagnostic;
use graph::builder;
use graph::validator;

fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e:#}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { name } => {
            let base = match name {
                Some(ref n) => {
                    let p = PathBuf::from(n);
                    fs::create_dir_all(&p)
                        .with_context(|| format!("Failed to create directory '{n}'"))?;
                    p
                }
                None => PathBuf::from("."),
            };
            cli::init::run_init(&base)
        }
        Commands::Build { input, output } => {
            let input_path = resolve_input(input.as_deref())?;
            let output_path = resolve_output(output.as_deref())?;
            build(&input_path, &output_path)
        }
        Commands::Run { args } => {
            let binary = resolve_output(None)?;
            if !binary.exists() {
                anyhow::bail!(
                    "Binary not found at '{}'. Run `duumbi build` first.",
                    binary.display()
                );
            }
            let status = process::Command::new(&binary)
                .args(&args)
                .status()
                .with_context(|| format!("Failed to execute '{}'", binary.display()))?;
            process::exit(status.code().unwrap_or(1));
        }
        Commands::Check { input } => {
            let input_path = resolve_input(input.as_deref())?;
            check(&input_path)
        }
        Commands::Describe { input } => {
            let input_path = resolve_input(input.as_deref())?;
            describe(&input_path)
        }
    }
}

/// Resolves the input file path: explicit path or workspace discovery.
fn resolve_input(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }

    let workspace_main = PathBuf::from(".duumbi/graph/main.jsonld");
    if workspace_main.exists() {
        return Ok(workspace_main);
    }

    anyhow::bail!(
        "No input file specified and no workspace found. \
         Use `duumbi init` to create a workspace or specify an input file."
    )
}

/// Resolves the output path: explicit path or workspace default.
fn resolve_output(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }

    let workspace_build = PathBuf::from(".duumbi/build");
    if workspace_build.exists() {
        return Ok(workspace_build.join("output"));
    }

    Ok(PathBuf::from("output"))
}

/// Parses and validates a source file, returning the semantic graph on success.
fn parse_and_validate(input: &Path) -> Result<graph::SemanticGraph> {
    let source = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file '{}'", input.display()))?;

    let module_ast = match parser::parse_jsonld(&source) {
        Ok(ast) => ast,
        Err(e) => {
            let diag = match &e {
                parser::ParseError::Json { code, .. } => Diagnostic::error(code, e.to_string()),
                parser::ParseError::MissingField { code, node_id, .. } => {
                    Diagnostic::error(code, e.to_string())
                        .with_node(&types::NodeId(node_id.clone()))
                }
                parser::ParseError::UnknownOp { code, node_id, .. } => {
                    Diagnostic::error(code, e.to_string())
                        .with_node(&types::NodeId(node_id.clone()))
                }
                parser::ParseError::SchemaInvalid { code, .. } => {
                    Diagnostic::error(code, e.to_string())
                }
            };
            emit_diagnostic(&diag);
            anyhow::bail!("Parse failed");
        }
    };

    let semantic_graph = match builder::build_graph(&module_ast) {
        Ok(sg) => sg,
        Err(errors) => {
            for err in &errors {
                let diag = Diagnostic::error(err.code(), err.to_string());
                emit_diagnostic(&diag);
            }
            anyhow::bail!("Graph construction failed with {} error(s)", errors.len());
        }
    };

    let diagnostics = validator::validate(&semantic_graph);
    if !diagnostics.is_empty() {
        for diag in &diagnostics {
            emit_diagnostic(diag);
        }
        anyhow::bail!("Validation failed with {} error(s)", diagnostics.len());
    }

    Ok(semantic_graph)
}

fn build(input: &Path, output: &Path) -> Result<()> {
    let semantic_graph = parse_and_validate(input)?;

    // Compile to object
    let obj_bytes =
        lowering::compile_to_object(&semantic_graph).context("Cranelift compilation failed")?;

    // Write object file to a unique temp directory (avoid race conditions)
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp_dir =
        std::env::temp_dir().join(format!("duumbi_build_{}_{}", std::process::id(), unique_id));
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;

    let obj_path = tmp_dir.join("output.o");
    fs::write(&obj_path, &obj_bytes).context("Failed to write object file")?;

    // Compile C runtime
    let runtime_c = find_runtime_c()?;
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).context("Failed to compile C runtime")?;

    // Link
    linker::link(&obj_path, &runtime_o, output).context("Failed to link binary")?;

    // Clean up temp build artifacts
    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!("Build successful: {}", output.display());
    Ok(())
}

fn check(input: &Path) -> Result<()> {
    match parse_and_validate(input) {
        Ok(_) => {
            eprintln!("Validation passed.");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn describe(input: &Path) -> Result<()> {
    let semantic_graph = parse_and_validate(input)?;
    cli::describe::describe(&semantic_graph);
    Ok(())
}

/// The C runtime source, embedded at compile time.
const RUNTIME_C_SOURCE: &str = include_str!("../runtime/duumbi_runtime.c");

/// Provides the `duumbi_runtime.c` file, writing it to a temp location if needed.
///
/// First checks for the file on disk (relative to CWD or executable), then
/// falls back to writing the embedded source to a temp file.
fn find_runtime_c() -> Result<std::path::PathBuf> {
    let candidates = [
        std::path::PathBuf::from("runtime/duumbi_runtime.c"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("runtime/duumbi_runtime.c")))
            .unwrap_or_default(),
    ];

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    // Fall back to writing the embedded runtime source
    let tmp_dir = std::env::temp_dir().join("duumbi_build");
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;
    let runtime_path = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_path, RUNTIME_C_SOURCE).context("Failed to write embedded runtime")?;
    Ok(runtime_path)
}

/// Emits a diagnostic as JSONL to stdout and a human summary to stderr.
fn emit_diagnostic(diag: &Diagnostic) {
    println!("{}", diag.to_jsonl());
    eprintln!("{diag}");
}
