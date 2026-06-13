use std::fs;
use std::process::Command;

use serde_json::{Value, json};
use tempfile::TempDir;

fn duumbi() -> Command {
    Command::new(env!("CARGO_BIN_EXE_duumbi"))
}

fn setup_workspace() -> TempDir {
    let tmp = TempDir::new().expect("invariant: temp dir must be created");
    let graph_dir = tmp.path().join(".duumbi").join("graph");
    fs::create_dir_all(&graph_dir).expect("invariant: graph dir must be created");
    fs::write(graph_dir.join("main.jsonld"), source_fixture().to_string())
        .expect("invariant: source fixture must be written");
    tmp
}

fn source_fixture() -> Value {
    json!({
        "@context": {"duumbi": "https://duumbi.dev/ontology#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:rewrite",
        "duumbi:name": "rewrite",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:rewrite/main",
            "duumbi:name": "main",
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:rewrite/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [
                    {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:rewrite/main/entry/left",
                        "duumbi:value": 42,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Const",
                        "@id": "duumbi:rewrite/main/entry/zero",
                        "duumbi:value": 0,
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Add",
                        "@id": "duumbi:rewrite/main/entry/add",
                        "duumbi:left": {"@id": "duumbi:rewrite/main/entry/left"},
                        "duumbi:right": {"@id": "duumbi:rewrite/main/entry/zero"},
                        "duumbi:resultType": "i64"
                    },
                    {
                        "@type": "duumbi:Return",
                        "@id": "duumbi:rewrite/main/entry/return",
                        "duumbi:operand": {"@id": "duumbi:rewrite/main/entry/add"}
                    }
                ]
            }]
        }]
    })
}

#[test]
fn rewrite_list_json_exposes_rule_metadata() {
    let output = duumbi()
        .args(["rewrite", "list", "--json"])
        .output()
        .expect("invariant: duumbi binary must run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rules: Value =
        serde_json::from_slice(&output.stdout).expect("invariant: rules output is JSON");
    let first = rules
        .as_array()
        .expect("invariant: rules output is array")
        .iter()
        .find(|rule| rule["id"] == "i64-add-zero-right")
        .expect("expected i64-add-zero-right rule");

    assert_eq!(first["category"], "local-optimization");
    assert_eq!(first["safetyClass"], "local-semantics-preserving");
    assert_eq!(first["applyCapable"], true);
    assert!(
        first["preconditions"]
            .as_str()
            .is_some_and(|text| text.contains("Const(0)"))
    );
}

#[test]
fn rewrite_preview_json_is_read_only_and_stable() {
    let tmp = setup_workspace();
    let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");
    let before = fs::read_to_string(&graph_path).expect("invariant: graph must be readable");

    let output = duumbi()
        .current_dir(tmp.path())
        .args([
            "rewrite",
            "preview",
            "--rule",
            "i64-add-zero-right",
            "--json",
        ])
        .output()
        .expect("invariant: duumbi binary must run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let preview: Value =
        serde_json::from_slice(&output.stdout).expect("invariant: preview output is JSON");
    assert_eq!(
        preview["matches"][0]["matchId"],
        "i64-add-zero-right:rewrite:duumbi:rewrite/main/entry/add:0"
    );
    assert_eq!(
        fs::read_to_string(&graph_path).expect("invariant: graph must remain readable"),
        before
    );
    assert!(!tmp.path().join(".duumbi").join("history").exists());
}

#[test]
fn rewrite_apply_json_writes_after_snapshot_and_validation() {
    let tmp = setup_workspace();
    let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");

    let output = duumbi()
        .current_dir(tmp.path())
        .args([
            "rewrite",
            "apply",
            "--rule",
            "i64-add-zero-right",
            "--match",
            "i64-add-zero-right:rewrite:duumbi:rewrite/main/entry/add:0",
            "--yes",
            "--json",
        ])
        .output()
        .expect("invariant: duumbi binary must run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value =
        serde_json::from_slice(&output.stdout).expect("invariant: apply output is JSON");
    assert_eq!(result["status"], "success");
    assert_eq!(result["validation"]["valid"], true);
    assert!(
        result["snapshotPath"]
            .as_str()
            .is_some_and(|path| path.ends_with("000001.jsonld"))
    );

    let rewritten: Value = serde_json::from_str(
        &fs::read_to_string(&graph_path).expect("invariant: graph must be readable"),
    )
    .expect("invariant: rewritten graph must be JSON");
    assert_eq!(
        rewritten["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][3]["duumbi:operand"]["@id"],
        "duumbi:rewrite/main/entry/left"
    );
    assert!(
        tmp.path()
            .join(".duumbi")
            .join("history")
            .join("000001.jsonld")
            .exists()
    );
}

#[test]
fn rewrite_apply_explicit_path_uses_source_module_name_for_match_ids() {
    let tmp = setup_workspace();
    let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");
    let graph_arg = graph_path
        .to_str()
        .expect("invariant: temp graph path is UTF-8");

    let output = duumbi()
        .current_dir(tmp.path())
        .args([
            "rewrite",
            "apply",
            "--module",
            graph_arg,
            "--rule",
            "i64-add-zero-right",
            "--match",
            "i64-add-zero-right:rewrite:duumbi:rewrite/main/entry/add:0",
            "--yes",
            "--json",
        ])
        .output()
        .expect("invariant: duumbi binary must run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value =
        serde_json::from_slice(&output.stdout).expect("invariant: apply output is JSON");
    assert_eq!(result["status"], "success");
}

#[test]
fn rewrite_apply_experimental_rule_is_rejected_without_snapshot() {
    let tmp = setup_workspace();
    let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");
    let before = fs::read_to_string(&graph_path).expect("invariant: graph must be readable");

    let output = duumbi()
        .current_dir(tmp.path())
        .args([
            "rewrite",
            "apply",
            "--rule",
            "experimental-fold-i64-const-add",
            "--all",
            "--max-matches",
            "1",
            "--yes",
        ])
        .output()
        .expect("invariant: duumbi binary must run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("preview-only"));
    assert_eq!(
        fs::read_to_string(&graph_path).expect("invariant: graph must remain readable"),
        before
    );
    assert!(!tmp.path().join(".duumbi").join("history").exists());
}

#[test]
fn rewrite_mcp_tools_are_discoverable_with_safety_descriptions() {
    use duumbi::mcp::server::McpServer;

    let tmp = setup_workspace();
    let server = McpServer::new(tmp.path().to_path_buf());
    let tools = server.list_tools();
    let preview = tools
        .iter()
        .find(|tool| tool.name == "rewrite_preview")
        .expect("rewrite_preview must be listed");
    let apply = tools
        .iter()
        .find(|tool| tool.name == "rewrite_apply")
        .expect("rewrite_apply must be listed");

    assert!(preview.description.contains("Read-only"));
    assert!(apply.description.contains("Write-capable"));
    assert_eq!(
        preview.input_schema["additionalProperties"],
        serde_json::json!(false)
    );
    assert_eq!(
        apply.input_schema["additionalProperties"],
        serde_json::json!(false)
    );
}

#[test]
fn rewrite_mcp_preview_is_read_only() {
    use duumbi::mcp::tools::rewrite;

    let tmp = setup_workspace();
    let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");
    let before = fs::read_to_string(&graph_path).expect("invariant: graph must be readable");

    let preview = rewrite::rewrite_preview(
        tmp.path(),
        &serde_json::json!({"rule_id": "i64-add-zero-right"}),
    )
    .expect("MCP preview must succeed");

    assert_eq!(
        preview["matches"][0]["matchId"],
        "i64-add-zero-right:rewrite:duumbi:rewrite/main/entry/add:0"
    );
    assert_eq!(
        fs::read_to_string(&graph_path).expect("invariant: graph must remain readable"),
        before
    );
    assert!(!tmp.path().join(".duumbi").join("history").exists());
}

#[test]
fn rewrite_mcp_apply_is_snapshot_backed() {
    use duumbi::mcp::tools::rewrite;

    let tmp = setup_workspace();
    let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");

    let result = rewrite::rewrite_apply(
        tmp.path(),
        &serde_json::json!({
            "rule_id": "i64-add-zero-right",
            "match_id": "i64-add-zero-right:rewrite:duumbi:rewrite/main/entry/add:0"
        }),
    )
    .expect("MCP apply must succeed");

    assert_eq!(result["status"], "success");
    assert_eq!(result["validation"]["valid"], true);
    assert!(
        result["snapshotPath"]
            .as_str()
            .is_some_and(|path| path.ends_with("000001.jsonld"))
    );

    let rewritten: Value = serde_json::from_str(
        &fs::read_to_string(&graph_path).expect("invariant: graph must be readable"),
    )
    .expect("invariant: rewritten graph must be JSON");
    assert_eq!(
        rewritten["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][3]["duumbi:operand"]["@id"],
        "duumbi:rewrite/main/entry/left"
    );
}

#[test]
fn rewrite_mcp_rejects_unknown_fields() {
    use duumbi::mcp::tools::rewrite;

    let tmp = setup_workspace();
    let err = rewrite::rewrite_preview(
        tmp.path(),
        &serde_json::json!({
            "rule_id": "i64-add-zero-right",
            "unexpected": true
        }),
    )
    .expect_err("unknown fields must be rejected");

    assert!(err.contains("Unknown field"));
}
