mod cli;
mod compiler;
mod errors;
mod graph;
mod parser;
mod types;

use std::fs;
use std::path::Path;
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
        Commands::Build { input, output } => build(&input, &output),
    }
}

fn build(input: &Path, output: &Path) -> Result<()> {
    // 1. Parse
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

    // 2. Build graph
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

    // 3. Validate
    let diagnostics = validator::validate(&semantic_graph);
    if !diagnostics.is_empty() {
        for diag in &diagnostics {
            emit_diagnostic(diag);
        }
        anyhow::bail!("Validation failed with {} error(s)", diagnostics.len());
    }

    // 4. Compile to object
    let obj_bytes =
        lowering::compile_to_object(&semantic_graph).context("Cranelift compilation failed")?;

    // 5. Write object file to temp
    let tmp_dir = std::env::temp_dir().join("duumbi_build");
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;

    let obj_path = tmp_dir.join("output.o");
    fs::write(&obj_path, &obj_bytes).context("Failed to write object file")?;

    // 6. Compile C runtime
    let runtime_c = find_runtime_c()?;
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).context("Failed to compile C runtime")?;

    // 7. Link
    linker::link(&obj_path, &runtime_o, output).context("Failed to link binary")?;

    eprintln!("Build successful: {}", output.display());
    Ok(())
}

/// Locates the `duumbi_runtime.c` file.
///
/// Searches relative to the executable, then falls back to `runtime/` in the
/// current working directory.
fn find_runtime_c() -> Result<std::path::PathBuf> {
    // Try relative to the cargo manifest dir (for development)
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

    anyhow::bail!(
        "Could not find duumbi_runtime.c. Searched: {}",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Emits a diagnostic as JSONL to stdout and a human summary to stderr.
fn emit_diagnostic(diag: &Diagnostic) {
    println!("{}", diag.to_jsonl());
    eprintln!("{diag}");
}
