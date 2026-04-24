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

use super::theme;

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
            emit_error_suggestions(ErrorKind::Graph);
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
            emit_error_suggestions(ErrorKind::Graph);
            anyhow::bail!("Graph construction failed with {} error(s)", errors.len());
        }
    };

    let diagnostics = validator::validate(&semantic_graph);
    if !diagnostics.is_empty() {
        for diag in &diagnostics {
            emit_diagnostic(diag);
        }
        emit_error_suggestions(ErrorKind::Graph);
        anyhow::bail!("Validation failed with {} error(s)", diagnostics.len());
    }

    Ok(semantic_graph)
}

/// Compiles a JSON-LD graph to a native binary.
///
/// Runs the full pipeline: parse → validate → Cranelift IR → object file → link.
pub(crate) fn build(input: &Path, output: &Path) -> Result<()> {
    build_with_opts(input, output, false)
}

/// Builds a program with optional offline mode.
///
/// When `offline` is `true`, dependency resolution skips the cache layer.
pub(crate) fn build_with_opts(input: &Path, output: &Path, offline: bool) -> Result<()> {
    if let Some(workspace_root) = workspace_root_for_graph_input(input) {
        return build_workspace_program(&workspace_root, output, offline);
    }

    let semantic_graph = parse_and_validate(input)?;

    let obj_bytes = lowering::compile_to_object(&semantic_graph)
        .map_err(|e| {
            emit_error_suggestions(ErrorKind::Compilation);
            anyhow::Error::new(e)
        })
        .context("Cranelift compilation failed")?;

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

    linker::link(&obj_path, &runtime_o, output)
        .map_err(|e| {
            emit_error_suggestions(ErrorKind::Link);
            anyhow::Error::new(e)
        })
        .context("Failed to link binary")?;

    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!(
        "{} Build successful: {}",
        theme::check_mark(),
        output.display()
    );
    Ok(())
}

/// Compiles all modules in a workspace (including declared dependencies) and links them.
fn build_workspace_program(workspace_root: &Path, output: &Path, offline: bool) -> Result<()> {
    let program = deps::load_program_with_deps_opts(workspace_root, offline).map_err(|e| {
        emit_program_error_diagnostics(&e);
        emit_error_suggestions(ErrorKind::Graph);
        anyhow::anyhow!("Graph construction failed: {e}")
    })?;

    let objects = lowering::compile_program(&program)
        .map_err(|e| {
            emit_error_suggestions(ErrorKind::Compilation);
            anyhow::Error::new(e)
        })
        .context("Cranelift compilation failed")?;

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
        if let Some(parent) = obj_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create dir for '{}'", obj_path.display()))?;
        }
        fs::write(&obj_path, obj_bytes)
            .with_context(|| format!("Failed to write object file '{}'", obj_path.display()))?;
        object_paths.push(obj_path);
    }

    let runtime_c = find_runtime_c()?;
    let runtime_o = tmp_dir.join("duumbi_runtime.o");
    linker::compile_runtime(&runtime_c, &runtime_o).context("Failed to compile C runtime")?;

    let object_path_refs: Vec<&Path> = object_paths.iter().map(|p| p.as_path()).collect();
    linker::link_multi(&object_path_refs, &runtime_o, output)
        .map_err(|e| {
            emit_error_suggestions(ErrorKind::Link);
            anyhow::Error::new(e)
        })
        .context("Failed to link binary")?;

    let _ = fs::remove_dir_all(&tmp_dir);

    eprintln!(
        "{} Build successful: {}",
        theme::check_mark(),
        output.display()
    );
    Ok(())
}

/// Validates a graph file without compiling.
pub(crate) fn check(input: &Path) -> Result<()> {
    if let Some(workspace_root) = workspace_root_for_graph_input(input) {
        return check_workspace_program(&workspace_root);
    }

    match parse_and_validate(input) {
        Ok(_) => {
            eprintln!("{} Validation passed.", theme::check_mark());
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

/// Returns a human-readable pseudocode description of the graph.
pub(crate) fn describe_to_string(input: &Path) -> Result<String> {
    if let Some(workspace_root) = workspace_root_for_graph_input(input) {
        return describe_workspace_program_to_string(&workspace_root, input);
    }

    let semantic_graph = parse_and_validate(input)?;
    Ok(crate::cli::describe::describe_to_string(&semantic_graph))
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
            eprintln!("{} Validation passed.", theme::check_mark());
            Ok(())
        }
        Err(e) => {
            emit_program_error_diagnostics(&e);
            emit_error_suggestions(ErrorKind::Graph);
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

fn describe_workspace_program_to_string(workspace_root: &Path, input: &Path) -> Result<String> {
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

    Ok(crate::cli::describe::describe_to_string(module_graph))
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

/// Classifies a build/check failure for actionable suggestion lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    /// Parse, validation, or graph construction error.
    Graph,
    /// Cranelift compilation error.
    Compilation,
    /// Linker invocation failure.
    Link,
}

/// Returns contextual suggestions for a given error kind.
///
/// The returned slice contains human-readable next-step hints.
#[must_use]
pub(crate) fn suggestions_for(kind: ErrorKind) -> &'static [&'static str] {
    match kind {
        ErrorKind::Graph => &[
            "duumbi add \"fix the error\" — ask the AI to fix it",
            "duumbi undo                 — revert the last change",
            "duumbi describe             — inspect the current graph",
        ],
        ErrorKind::Compilation => &[
            "duumbi check                — validate graph without compiling",
            "duumbi describe             — inspect the graph structure",
        ],
        ErrorKind::Link => &[
            "Ensure a C compiler (cc) is on PATH, or set $CC",
            "duumbi check                — validate graph without linking",
        ],
    }
}

/// Prints actionable next-step suggestions to stderr after a build or check failure.
pub(crate) fn emit_error_suggestions(kind: ErrorKind) {
    let suggestions = suggestions_for(kind);
    eprintln!();
    eprintln!("  {}", theme::dim("Suggestions:"));
    for s in suggestions {
        eprintln!("    {} {}", theme::dim("\u{2192}"), theme::info(s));
    }
}

/// Emits a diagnostic as JSONL to stdout and a human-readable summary to stderr.
///
/// The error code is highlighted in red and any node IDs in blue for visual clarity.
/// Diagnostic Display format: `[E001] error: message (at duumbi:node/path)`
pub(crate) fn emit_diagnostic(diag: &Diagnostic) {
    println!("{}", diag.to_jsonl());
    // Colorize: [E001] → [colored_code]
    let msg = format!("{diag}");
    let colored = if let Some((code_with_brackets, rest)) = msg.split_once(' ')
        && code_with_brackets.starts_with('[')
        && code_with_brackets.ends_with(']')
        && code_with_brackets.len() > 2
    {
        let inner_code = &code_with_brackets[1..code_with_brackets.len() - 1];
        let colored_code = format!("[{}]", theme::error_code(inner_code));
        format!("{colored_code} {rest}")
    } else {
        msg
    };
    // Colorize node IDs (duumbi:...)
    let colored = if let Some(pos) = colored.find("duumbi:") {
        let end = colored[pos..]
            .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
            .map(|i| pos + i)
            .unwrap_or(colored.len());
        let node = &colored[pos..end];
        colored.replacen(node, &theme::node_id(node), 1)
    } else {
        colored
    };
    eprintln!("{colored}");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_error_suggests_add_undo_describe() {
        let s = suggestions_for(ErrorKind::Graph);
        assert_eq!(s.len(), 3);
        assert!(s[0].contains("duumbi add"));
        assert!(s[1].contains("duumbi undo"));
        assert!(s[2].contains("duumbi describe"));
    }

    #[test]
    fn compilation_error_suggests_check_describe() {
        let s = suggestions_for(ErrorKind::Compilation);
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("duumbi check"));
        assert!(s[1].contains("duumbi describe"));
    }

    #[test]
    fn link_error_suggests_cc_and_check() {
        let s = suggestions_for(ErrorKind::Link);
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("$CC"));
        assert!(s[1].contains("duumbi check"));
    }

    #[test]
    fn emit_does_not_panic() {
        // Verify all variants can be printed without panicking.
        emit_error_suggestions(ErrorKind::Graph);
        emit_error_suggestions(ErrorKind::Compilation);
        emit_error_suggestions(ErrorKind::Link);
    }
}
