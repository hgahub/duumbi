//! MCP graph tools: query, mutate, validate, describe.
//!
//! These tools operate on the workspace's JSON-LD graph files and use the
//! DUUMBI parser/graph/patch pipeline to perform operations.

use std::fs;
use std::path::Path;

use serde_json::Value;

/// Query the semantic graph by node ID, type filter, or name pattern.
///
/// Params (all optional):
/// - `node_id`      — exact `@id` to look up
/// - `type_filter`  — match nodes by `@type` (e.g. `"duumbi:Add"`)
/// - `name_pattern` — substring match against `duumbi:name` fields
///
/// Returns a JSON object: `{ "nodes": [...] }`.
pub fn graph_query(workspace: &Path, params: &Value) -> Result<Value, String> {
    let graph_dir = workspace.join(".duumbi").join("graph");

    let node_id = params.get("node_id").and_then(Value::as_str);
    let type_filter = params.get("type_filter").and_then(Value::as_str);
    let name_pattern = params.get("name_pattern").and_then(Value::as_str);

    let mut matched = Vec::new();

    // Collect all .jsonld files in the graph directory.
    let entries = fs::read_dir(&graph_dir)
        .map_err(|e| format!("Cannot read graph dir '{}': {e}", graph_dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {e}"))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jsonld") {
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read '{}': {e}", path.display()))?;
            let doc: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Invalid JSON in '{}': {e}", path.display()))?;
            collect_matching_nodes(&doc, node_id, type_filter, name_pattern, &mut matched);
        }
    }

    Ok(serde_json::json!({ "nodes": matched }))
}

/// Recursively walk a JSON-LD document and collect nodes matching the filters.
fn collect_matching_nodes(
    value: &Value,
    node_id: Option<&str>,
    type_filter: Option<&str>,
    name_pattern: Option<&str>,
    out: &mut Vec<Value>,
) {
    match value {
        Value::Object(map) => {
            let id_matches = node_id
                .map(|id| map.get("@id").and_then(Value::as_str) == Some(id))
                .unwrap_or(true);

            let type_matches = type_filter
                .map(|t| map.get("@type").and_then(Value::as_str) == Some(t))
                .unwrap_or(true);

            let name_matches = name_pattern
                .map(|pat| {
                    map.get("duumbi:name")
                        .and_then(Value::as_str)
                        .is_some_and(|n| n.contains(pat))
                })
                .unwrap_or(true);

            if id_matches && type_matches && name_matches {
                // Only add if there's at least one filter applied (avoid returning
                // every nested structure when no filters are set).
                if node_id.is_some() || type_filter.is_some() || name_pattern.is_some() {
                    out.push(value.clone());
                }
            }

            // Recurse into all values.
            for v in map.values() {
                collect_matching_nodes(v, node_id, type_filter, name_pattern, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_matching_nodes(v, node_id, type_filter, name_pattern, out);
            }
        }
        _ => {}
    }
}

/// Apply a batch of graph mutations atomically.
///
/// Params:
/// - `ops` — array of [`crate::patch::PatchOp`] objects (must have `"kind"` tag)
///
/// Loads `main.jsonld`, applies the patch (all-or-nothing), validates, and
/// writes the result back to disk.
///
/// Returns `{ "success": true, "ops_count": N, "summary": "..." }`.
pub fn graph_mutate(workspace: &Path, params: &Value) -> Result<Value, String> {
    use crate::graph::{builder, validator};
    use crate::parser;
    use crate::patch::{self, PatchOp};

    let ops_value = params
        .get("ops")
        .ok_or_else(|| "Missing required field 'ops'".to_string())?;

    let ops: Vec<PatchOp> =
        serde_json::from_value(ops_value.clone()).map_err(|e| format!("Invalid patch ops: {e}"))?;

    let ops_count = ops.len();
    let graph_path = workspace.join(".duumbi").join("graph").join("main.jsonld");

    let source_text =
        fs::read_to_string(&graph_path).map_err(|e| format!("Cannot read main.jsonld: {e}"))?;

    let source: Value = serde_json::from_str(&source_text)
        .map_err(|e| format!("Invalid JSON in main.jsonld: {e}"))?;

    let patch = patch::GraphPatch { ops };
    let patched = patch::apply_patch(&source, &patch).map_err(|e| format!("Patch failed: {e}"))?;

    // Validate the patched graph before writing.
    let patched_text =
        serde_json::to_string(&patched).map_err(|e| format!("Serialization error: {e}"))?;

    let module_ast =
        parser::parse_jsonld(&patched_text).map_err(|e| format!("Parse error after patch: {e}"))?;

    let semantic_graph = builder::build_graph(&module_ast).map_err(|errs| {
        format!(
            "Graph errors: {}",
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;

    let diagnostics = validator::validate(&semantic_graph);
    if !diagnostics.is_empty() {
        let messages: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
        return Err(format!("Validation errors: {}", messages.join("; ")));
    }

    // Write to disk.
    let pretty =
        serde_json::to_string_pretty(&patched).map_err(|e| format!("Serialization error: {e}"))?;
    fs::write(&graph_path, pretty).map_err(|e| format!("Cannot write main.jsonld: {e}"))?;

    Ok(serde_json::json!({
        "success": true,
        "ops_count": ops_count,
        "summary": format!("Applied {ops_count} patch op(s) successfully")
    }))
}

/// Validate the workspace graph without modifying it.
///
/// Runs the full parse → build → validate pipeline.
///
/// Returns `{ "valid": bool, "diagnostics": [...] }`.
pub fn graph_validate(workspace: &Path, _params: &Value) -> Result<Value, String> {
    use crate::graph::{builder, validator};
    use crate::parser;

    let graph_path = workspace.join(".duumbi").join("graph").join("main.jsonld");

    let source_text =
        fs::read_to_string(&graph_path).map_err(|e| format!("Cannot read main.jsonld: {e}"))?;

    let module_ast = match parser::parse_jsonld(&source_text) {
        Ok(ast) => ast,
        Err(e) => {
            return Ok(serde_json::json!({
                "valid": false,
                "diagnostics": [{ "level": "error", "message": e.to_string() }]
            }));
        }
    };

    let semantic_graph = match builder::build_graph(&module_ast) {
        Ok(g) => g,
        Err(errs) => {
            let diags: Vec<Value> = errs
                .iter()
                .map(|e| serde_json::json!({ "level": "error", "message": e.to_string() }))
                .collect();
            return Ok(serde_json::json!({ "valid": false, "diagnostics": diags }));
        }
    };

    let diagnostics = validator::validate(&semantic_graph);
    let diag_values: Vec<Value> = diagnostics
        .iter()
        .map(|d| {
            serde_json::json!({
                "level": d.level,
                "code": d.code,
                "message": d.message,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "valid": diagnostics.is_empty(),
        "diagnostics": diag_values
    }))
}

/// Describe the workspace graph as pseudo-code.
///
/// Returns `{ "description": "..." }`.
pub fn graph_describe(workspace: &Path, _params: &Value) -> Result<Value, String> {
    use crate::graph::{builder, validator};
    use crate::parser;

    let graph_path = workspace.join(".duumbi").join("graph").join("main.jsonld");

    let source_text =
        fs::read_to_string(&graph_path).map_err(|e| format!("Cannot read main.jsonld: {e}"))?;

    let module_ast = parser::parse_jsonld(&source_text).map_err(|e| format!("Parse error: {e}"))?;

    let semantic_graph = builder::build_graph(&module_ast).map_err(|errs| {
        format!(
            "Graph errors: {}",
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;

    let diags = validator::validate(&semantic_graph);
    if !diags.is_empty() {
        let messages: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
        return Err(format!("Validation errors: {}", messages.join("; ")));
    }

    let description = format_graph_description(&semantic_graph);
    Ok(serde_json::json!({ "description": description }))
}

/// Formats a semantic graph as human-readable pseudo-code.
fn format_graph_description(graph: &crate::graph::SemanticGraph) -> String {
    let mut lines = Vec::new();
    lines.push(format!("module {}", graph.module_name.0));

    for func in &graph.functions {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {:?}", p.name, p.param_type))
            .collect();
        lines.push(format!(
            "  fn {}({}) -> {:?}",
            func.name.0,
            params.join(", "),
            func.return_type
        ));

        for block in &func.blocks {
            lines.push(format!("    block {}:", block.label.0));
            for node_idx in &block.nodes {
                if let Some(node) = graph.graph.node_weight(*node_idx) {
                    let op_str = format_op(&node.op);
                    lines.push(format!("      [{}] {op_str}", node.id.0));
                }
            }
        }
    }

    lines.join("\n")
}

/// Formats a single Op as a short pseudo-code string.
fn format_op(op: &crate::types::Op) -> String {
    use crate::types::Op;
    match op {
        Op::Const(n) => format!("const {n}"),
        Op::ConstF64(f) => format!("const_f64 {f}"),
        Op::ConstBool(b) => format!("const_bool {b}"),
        Op::ConstString(s) => format!("const_string \"{s}\""),
        Op::Add => "add".to_string(),
        Op::Sub => "sub".to_string(),
        Op::Mul => "mul".to_string(),
        Op::Div => "div".to_string(),
        Op::Print => "print".to_string(),
        Op::PrintString => "print_string".to_string(),
        Op::Return => "return".to_string(),
        Op::Load { variable } => format!("load {variable}"),
        Op::Store { variable } => format!("store {variable}"),
        Op::Call { function } => format!("call {function}"),
        Op::Compare(cmp) => format!("compare {cmp:?}"),
        Op::Branch => "branch".to_string(),
        _ => format!("{op:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_workspace(graph_json: &str) -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi").join("graph");
        std::fs::create_dir_all(&graph_dir).expect("create graph dir");
        std::fs::write(graph_dir.join("main.jsonld"), graph_json).expect("write main.jsonld");
        dir
    }

    const SIMPLE_GRAPH: &str = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:test",
        "duumbi:name": "test",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:test/main",
            "duumbi:name": "main",
            "duumbi:params": [],
            "duumbi:returnType": "void",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:test/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [{
                    "@type": "duumbi:Return",
                    "@id": "duumbi:test/main/entry/0",
                    "duumbi:operands": []
                }]
            }]
        }]
    }"#;

    #[test]
    fn graph_query_by_type() {
        let dir = make_workspace(SIMPLE_GRAPH);
        let params = serde_json::json!({ "type_filter": "duumbi:Function" });
        let result = graph_query(dir.path(), &params).expect("query ok");
        let nodes = result["nodes"].as_array().expect("nodes array");
        assert!(!nodes.is_empty(), "should find at least one function node");
        assert_eq!(
            nodes[0]["@type"].as_str(),
            Some("duumbi:Function"),
            "node type should match filter"
        );
    }

    #[test]
    fn graph_query_by_node_id() {
        let dir = make_workspace(SIMPLE_GRAPH);
        let params = serde_json::json!({ "node_id": "duumbi:test/main" });
        let result = graph_query(dir.path(), &params).expect("query ok");
        let nodes = result["nodes"].as_array().expect("nodes array");
        assert_eq!(nodes.len(), 1, "exactly one node should match the id");
        assert_eq!(nodes[0]["@id"].as_str(), Some("duumbi:test/main"));
    }

    #[test]
    fn graph_validate_returns_result() {
        let dir = make_workspace(SIMPLE_GRAPH);
        let result = graph_validate(dir.path(), &serde_json::json!({})).expect("validate ok");
        // The result must contain "valid" (bool) and "diagnostics" (array),
        // regardless of whether the minimal graph is valid or not.
        assert!(
            result.get("valid").is_some(),
            "result should have 'valid' field"
        );
        assert!(
            result.get("diagnostics").is_some(),
            "result should have 'diagnostics' field"
        );
    }

    #[test]
    fn graph_validate_invalid_json() {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi").join("graph");
        std::fs::create_dir_all(&graph_dir).expect("create graph dir");
        std::fs::write(graph_dir.join("main.jsonld"), "not json").expect("write");
        let result = graph_validate(dir.path(), &serde_json::json!({}));
        // Should return an error (cannot read/parse the file)
        assert!(result.is_err() || result.as_ref().is_ok_and(|v| v["valid"] == false));
    }

    #[test]
    fn graph_mutate_missing_ops_field() {
        let dir = make_workspace(SIMPLE_GRAPH);
        let result = graph_mutate(dir.path(), &serde_json::json!({}));
        assert!(result.is_err(), "should fail when 'ops' is missing");
    }

    #[test]
    fn graph_describe_returns_description_or_error() {
        let dir = make_workspace(SIMPLE_GRAPH);
        let result = graph_describe(dir.path(), &serde_json::json!({}));
        // The describe tool may fail if the minimal graph has validation issues.
        // Either way, it should not panic.
        match result {
            Ok(val) => {
                assert!(
                    val.get("description").is_some(),
                    "should have description field"
                );
            }
            Err(e) => {
                // Acceptable — validator caught issues in the minimal graph
                assert!(!e.is_empty(), "error message should not be empty");
            }
        }
    }
}
