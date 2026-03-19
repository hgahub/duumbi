//! Integration tests for Phase 10 Track C: Codebase Awareness.
//!
//! Tests project analyzer, module summary formatting, and workspace scanning.

use std::fs;

use duumbi::context::analyzer::{analyze_workspace, format_module_summary};
use tempfile::TempDir;

fn create_module(workspace: &std::path::Path, filename: &str, json: &serde_json::Value) {
    let graph_dir = workspace.join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("mkdir");
    fs::write(
        graph_dir.join(filename),
        serde_json::to_string_pretty(json).expect("serialize"),
    )
    .expect("write");
}

fn main_module() -> serde_json::Value {
    serde_json::json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    { "@type": "duumbi:Const", "@id": "duumbi:main/main/entry/0", "duumbi:value": 0, "duumbi:resultType": "i64" },
                    { "@type": "duumbi:Return", "@id": "duumbi:main/main/entry/1", "duumbi:operand": { "@id": "duumbi:main/main/entry/0" } }
                ]
            }]
        }]
    })
}

fn ops_module() -> serde_json::Value {
    serde_json::json!({
        "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
        "@type": "duumbi:Module",
        "@id": "duumbi:ops",
        "duumbi:name": "ops",
        "duumbi:functions": [
            {
                "@type": "duumbi:Function",
                "@id": "duumbi:ops/add",
                "duumbi:name": "add",
                "duumbi:returnType": "i64",
                "duumbi:params": [
                    { "duumbi:name": "a", "duumbi:paramType": "i64" },
                    { "duumbi:name": "b", "duumbi:paramType": "i64" }
                ],
                "duumbi:blocks": []
            },
            {
                "@type": "duumbi:Function",
                "@id": "duumbi:ops/subtract",
                "duumbi:name": "subtract",
                "duumbi:returnType": "i64",
                "duumbi:params": [
                    { "duumbi:name": "a", "duumbi:paramType": "i64" },
                    { "duumbi:name": "b", "duumbi:paramType": "i64" }
                ],
                "duumbi:blocks": []
            }
        ]
    })
}

// ---------------------------------------------------------------------------
// Analyzer: single module workspace
// ---------------------------------------------------------------------------

#[test]
fn analyzer_single_module() {
    let tmp = TempDir::new().expect("temp dir");
    create_module(tmp.path(), "main.jsonld", &main_module());

    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert_eq!(map.modules.len(), 1);
    assert_eq!(map.modules[0].name, "main");
    assert!(map.modules[0].is_main);
    assert_eq!(map.modules[0].functions.len(), 1);
    assert_eq!(map.modules[0].functions[0].name, "main");
    assert_eq!(map.modules[0].functions[0].return_type, "i64");
}

// ---------------------------------------------------------------------------
// Analyzer: multi-module workspace
// ---------------------------------------------------------------------------

#[test]
fn analyzer_multi_module() {
    let tmp = TempDir::new().expect("temp dir");
    create_module(tmp.path(), "main.jsonld", &main_module());
    create_module(tmp.path(), "ops.jsonld", &ops_module());

    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert_eq!(map.modules.len(), 2);

    // Check exports
    assert_eq!(map.exports.get("add"), Some(&"ops".to_string()));
    assert_eq!(map.exports.get("subtract"), Some(&"ops".to_string()));
    assert_eq!(map.exports.get("main"), Some(&"main".to_string()));
}

// ---------------------------------------------------------------------------
// Analyzer: export collection accuracy
// ---------------------------------------------------------------------------

#[test]
fn analyzer_export_collection() {
    let tmp = TempDir::new().expect("temp dir");
    create_module(tmp.path(), "ops.jsonld", &ops_module());

    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert_eq!(map.exports.len(), 2);
    assert!(map.exports.contains_key("add"));
    assert!(map.exports.contains_key("subtract"));
}

// ---------------------------------------------------------------------------
// Analyzer: function params extraction
// ---------------------------------------------------------------------------

#[test]
fn analyzer_function_params() {
    let tmp = TempDir::new().expect("temp dir");
    create_module(tmp.path(), "ops.jsonld", &ops_module());

    let map = analyze_workspace(tmp.path()).expect("analyze");
    let ops = &map.modules[0];
    let add_fn = &ops.functions[0];
    assert_eq!(add_fn.params.len(), 2);
    assert_eq!(add_fn.params[0], ("a".to_string(), "i64".to_string()));
    assert_eq!(add_fn.params[1], ("b".to_string(), "i64".to_string()));
}

// ---------------------------------------------------------------------------
// Module summary formatting
// ---------------------------------------------------------------------------

#[test]
fn format_summary_includes_signatures() {
    let tmp = TempDir::new().expect("temp dir");
    create_module(tmp.path(), "main.jsonld", &main_module());
    create_module(tmp.path(), "ops.jsonld", &ops_module());

    let map = analyze_workspace(tmp.path()).expect("analyze");
    let summary = format_module_summary(&map);

    assert!(summary.contains("[main (main)]") || summary.contains("main"));
    assert!(summary.contains("add(a: i64, b: i64) -> i64"));
    assert!(summary.contains("subtract(a: i64, b: i64) -> i64"));
}

// ---------------------------------------------------------------------------
// Analyzer: empty / no .duumbi directory
// ---------------------------------------------------------------------------

#[test]
fn analyzer_empty_workspace() {
    let tmp = TempDir::new().expect("temp dir");
    fs::create_dir_all(tmp.path().join(".duumbi/graph")).expect("mkdir");
    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert!(map.modules.is_empty());
    assert!(map.exports.is_empty());
}

#[test]
fn analyzer_no_duumbi_dir() {
    let tmp = TempDir::new().expect("temp dir");
    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert!(map.modules.is_empty());
}

// ---------------------------------------------------------------------------
// Analyzer: ignores non-jsonld files
// ---------------------------------------------------------------------------

#[test]
fn analyzer_ignores_non_jsonld() {
    let tmp = TempDir::new().expect("temp dir");
    let graph_dir = tmp.path().join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("mkdir");
    fs::write(graph_dir.join("readme.txt"), "not a module").expect("write");
    fs::write(graph_dir.join("config.toml"), "[workspace]").expect("write");

    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert!(map.modules.is_empty());
}

// ---------------------------------------------------------------------------
// Analyzer: malformed jsonld gracefully skipped
// ---------------------------------------------------------------------------

#[test]
fn analyzer_malformed_jsonld_skipped() {
    let tmp = TempDir::new().expect("temp dir");
    let graph_dir = tmp.path().join(".duumbi/graph");
    fs::create_dir_all(&graph_dir).expect("mkdir");
    fs::write(graph_dir.join("bad.jsonld"), "not valid json").expect("write");
    create_module(tmp.path(), "main.jsonld", &main_module());

    let map = analyze_workspace(tmp.path()).expect("analyze");
    assert_eq!(map.modules.len(), 1); // Only the valid module
}

// ---------------------------------------------------------------------------
// Context assembly with multi-module workspace
// ---------------------------------------------------------------------------

#[test]
fn context_assembly_multi_module_enrichment() {
    let tmp = TempDir::new().expect("temp dir");
    create_module(tmp.path(), "main.jsonld", &main_module());
    create_module(tmp.path(), "ops.jsonld", &ops_module());

    let bundle = duumbi::context::assemble_context("add multiply to ops", tmp.path(), &[])
        .expect("assemble");
    // Should classify correctly and include module info
    assert!(
        bundle.enriched_message.contains("ops") || bundle.enriched_message.contains("add"),
        "should reference existing modules/functions"
    );
}
