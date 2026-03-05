//! Shared CLI command implementations.
//!
//! Contains the core build/check/describe logic reused by both the standard
//! CLI dispatch in `main.rs` and the interactive REPL in `repl.rs`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::compiler::{linker, lowering};
use crate::deps;
use crate::errors::Diagnostic;
use crate::graph::{self, builder, program::ProgramError, validator};
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
    if let Some(workspace_root) = workspace_root_for_graph_input(input) {
        return build_workspace_program(&workspace_root, output);
    }

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

/// Compiles all modules in a workspace (including declared dependencies) and links them.
fn build_workspace_program(workspace_root: &Path, output: &Path) -> Result<()> {
    let program = deps::load_program_with_deps(workspace_root).map_err(|e| {
        emit_program_error_diagnostics(&e);
        anyhow::anyhow!("Graph construction failed: {e}")
    })?;

    let objects = lowering::compile_program(&program).context("Cranelift compilation failed")?;

    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp_dir =
        std::env::temp_dir().join(format!("duumbi_build_{}_{}", std::process::id(), unique_id));
    fs::create_dir_all(&tmp_dir).context("Failed to create temp build directory")?;

    let mut module_names: Vec<&String> = objects.keys().collect();
    module_names.sort();

    let mut object_paths = Vec::with_capacity(module_names.len());
    for module_name in module_names {
        let obj_bytes = objects
            .get(module_name)
            .ok_or_else(|| anyhow::anyhow!("Missing object bytes for module '{module_name}'"))?;
        let obj_path = tmp_dir.join(format!("{module_name}.o"));
        fs::write(&obj_path, obj_bytes)
            .with_context(|| format!("Failed to write object file '{}'", obj_path.display()))?;
        object_paths.push(obj_path);
    }

    let runtime_c = find_runtime_c()?;
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).context("Failed to compile C runtime")?;

    let object_path_refs: Vec<&Path> = object_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&object_path_refs, &runtime_o, output).context("Failed to link binary")?;

    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!("Build successful: {}", output.display());
    Ok(())
}

/// Validates a graph file without compiling.
pub(crate) fn check(input: &Path) -> Result<()> {
    if let Some(workspace_root) = workspace_root_for_graph_input(input) {
        return check_workspace_program(&workspace_root);
    }

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
    if let Some(workspace_root) = workspace_root_for_graph_input(input) {
        return describe_workspace_program(&workspace_root, input);
    }

    let semantic_graph = parse_and_validate(input)?;
    crate::cli::describe::describe(&semantic_graph);
    Ok(())
}

/// If `input` is `<workspace>/.duumbi/graph/*.jsonld`, returns `<workspace>`.
fn workspace_root_for_graph_input(input: &Path) -> Option<std::path::PathBuf> {
    let parent = input.parent()?;
    if parent.file_name().and_then(|s| s.to_str()) != Some("graph") {
        return None;
    }
    let duumbi_dir = parent.parent()?;
    if duumbi_dir.file_name().and_then(|s| s.to_str()) != Some(".duumbi") {
        return None;
    }
    duumbi_dir.parent().map(std::path::Path::to_path_buf)
}

/// Validates all modules for the workspace and its declared dependencies.
fn check_workspace_program(workspace_root: &Path) -> Result<()> {
    match deps::load_program_with_deps(workspace_root) {
        Ok(_) => {
            eprintln!("Validation passed.");
            Ok(())
        }
        Err(e) => {
            emit_program_error_diagnostics(&e);
            anyhow::bail!("Graph construction failed: {e}");
        }
    }
}

/// Describes a module in a workspace program after cross-module validation.
fn describe_workspace_program(workspace_root: &Path, input: &Path) -> Result<()> {
    let source = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file '{}'", input.display()))?;

    let module_ast = parser::parse_jsonld(&source).map_err(|e| {
        let diag = match &e {
            parser::ParseError::Json { code, .. } => Diagnostic::error(code, e.to_string()),
            parser::ParseError::MissingField { code, node_id, .. } => {
                Diagnostic::error(code, e.to_string()).with_node(&types::NodeId(node_id.clone()))
            }
            parser::ParseError::UnknownOp { code, node_id, .. } => {
                Diagnostic::error(code, e.to_string()).with_node(&types::NodeId(node_id.clone()))
            }
            parser::ParseError::SchemaInvalid { code, .. } => {
                Diagnostic::error(code, e.to_string())
            }
        };
        emit_diagnostic(&diag);
        anyhow::anyhow!("Parse failed")
    })?;

    let program = deps::load_program_with_deps(workspace_root).map_err(|e| {
        emit_program_error_diagnostics(&e);
        anyhow::anyhow!("Graph construction failed: {e}")
    })?;

    let module_graph = program.modules.get(&module_ast.name).ok_or_else(|| {
        anyhow::anyhow!("Module '{}' not found in loaded program", module_ast.name.0)
    })?;

    crate::cli::describe::describe(module_graph);
    Ok(())
}

fn emit_program_error_diagnostics(err: &deps::DepsError) {
    if let deps::DepsError::Program(errors) = err {
        for e in errors {
            match e {
                ProgramError::UnresolvedCrossModuleRef { code, .. } => {
                    let diag = Diagnostic::error(code, e.to_string());
                    emit_diagnostic(&diag);
                }
                ProgramError::GraphError { error, .. } => {
                    let diag = Diagnostic::error(error.code(), e.to_string());
                    emit_diagnostic(&diag);
                }
                ProgramError::LoadFailed { .. } => {
                    let diag = Diagnostic::error("INTERNAL", e.to_string());
                    emit_diagnostic(&diag);
                }
            }
        }
    }
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
