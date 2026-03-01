//! Shared CLI command implementations.
//!
//! Contains the core build/check/describe logic reused by both the standard
//! CLI dispatch in `main.rs` and the interactive REPL in `repl.rs`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::compiler::{linker, lowering};
use crate::errors::Diagnostic;
use crate::graph::{self, builder, validator};
use crate::parser;
use crate::types;

/// The C runtime source, embedded at compile time.
const RUNTIME_C_SOURCE: &str = include_str!("../../runtime/duumbi_runtime.c");

/// Parses and validates a `.jsonld` file, returning the semantic graph on success.
///
/// Emits structured JSONL diagnostics to stdout and human-readable summaries
/// to stderr on failure.
pub(crate) fn parse_and_validate(input: &Path) -> Result<graph::SemanticGraph> {
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

/// Compiles a JSON-LD graph to a native binary.
///
/// Runs the full pipeline: parse → validate → Cranelift IR → object file → link.
pub(crate) fn build(input: &Path, output: &Path) -> Result<()> {
    let semantic_graph = parse_and_validate(input)?;

    let obj_bytes =
        lowering::compile_to_object(&semantic_graph).context("Cranelift compilation failed")?;

    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp_dir =
        std::env::temp_dir().join(format!("duumbi_build_{}_{}", std::process::id(), unique_id));
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;

    let obj_path = tmp_dir.join("output.o");
    fs::write(&obj_path, &obj_bytes).context("Failed to write object file")?;

    let runtime_c = find_runtime_c()?;
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).context("Failed to compile C runtime")?;

    linker::link(&obj_path, &runtime_o, output).context("Failed to link binary")?;

    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!("Build successful: {}", output.display());
    Ok(())
}

/// Validates a graph file without compiling.
pub(crate) fn check(input: &Path) -> Result<()> {
    match parse_and_validate(input) {
        Ok(_) => {
            eprintln!("Validation passed.");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Prints a human-readable pseudocode description of the graph.
pub(crate) fn describe(input: &Path) -> Result<()> {
    let semantic_graph = parse_and_validate(input)?;
    crate::cli::describe::describe(&semantic_graph);
    Ok(())
}

/// Emits a diagnostic as JSONL to stdout and a human-readable summary to stderr.
pub(crate) fn emit_diagnostic(diag: &Diagnostic) {
    println!("{}", diag.to_jsonl());
    eprintln!("{diag}");
}

/// Provides the `duumbi_runtime.c` source file path.
///
/// Checks known locations on disk first, then writes the embedded source
/// to a temporary file as a fallback.
pub(crate) fn find_runtime_c() -> Result<std::path::PathBuf> {
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

    let tmp_dir = std::env::temp_dir().join("duumbi_build");
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;
    let runtime_path = tmp_dir.join("duumbi_runtime.c");
    fs::write(&runtime_path, RUNTIME_C_SOURCE).context("Failed to write embedded runtime")?;
    Ok(runtime_path)
}
