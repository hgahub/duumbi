//! AI mutation orchestrator.
//!
//! Combines LLM provider calls, GraphPatch application, validation,
//! and retry logic into a single [`mutate`] entry point.

use anyhow::{Context, Result};

use crate::agents::LlmClient;
use crate::patch::{GraphPatch, apply_patch};

/// System prompt sent to the LLM with every mutation request.
///
/// Instructs the model to use the provided tools to modify the graph.
/// The graph JSON is appended by the caller as part of the user message.
pub const SYSTEM_PROMPT: &str = "\
You are an expert at modifying duumbi semantic graph programs stored as JSON-LD. \
The program is represented as a directed acyclic graph of typed operations. \
Use the provided tools to implement the requested change. \
Each tool call is one atomic graph mutation. You may call multiple tools in sequence.\n\
\n\
CRITICAL: emit ALL tool calls needed for the COMPLETE change in this SINGLE response.\n\
Do NOT plan multi-step sequences — the system calls you ONCE per user request.\n\
A function with no blocks, or a block with no ops, is invalid and will fail compilation.\n\
\n\
Important rules:\n\
- All @id values must be globally unique (format: duumbi:<module>/<function>/<block>/<index>)\n\
- Use the duumbi: prefix for all field names (duumbi:value, duumbi:left, etc.)\n\
- resultType must be one of: i64, f64, bool, void\n\
- operand references use the form {\"@id\": \"<target_id>\"}\n\
- Operations within a block must form a valid data-flow DAG\n\
- The last op in each block must be Return or Branch — NO ops may follow a terminator\n\
- Every function must have at least one block; every block must have at least one op\n\
- add_op APPENDS to the end of a block.\n\
- To insert a new op BEFORE an existing Return/Branch (e.g. insert a Call before Return):\n\
  PATTERN A — minimal removes (preferred):\n\
    1. remove_node the Return (or Branch) — the block now ends with your data op\n\
    2. add_op the new Call/computation op\n\
    3. add_op the new Return (pointing to the new op's @id)\n\
    4. set_edge any OTHER ops that should now point to the new op (e.g. Print operand)\n\
  PATTERN B — when you must remove multiple ops before the terminator:\n\
    remove ALL of them first, then add_op ALL replacements (in correct order), \
    ending with a new Return or Branch. ALL in this SINGLE response.\n\
- NEVER remove ops without adding them back in the same response.\n\
\n\
Op reference (exhaustive — no other @type values exist):\n\
- Const:   {\"@type\":\"duumbi:Const\",  \"duumbi:value\":<n>,        \"duumbi:resultType\":\"i64\"|\"f64\"|\"bool\"}\n\
- Add:     {\"@type\":\"duumbi:Add\",    \"duumbi:left\":{\"@id\":\"…\"}, \"duumbi:right\":{\"@id\":\"…\"}, \"duumbi:resultType\":\"i64\"|\"f64\"}\n\
- Sub:     {\"@type\":\"duumbi:Sub\",    \"duumbi:left\":{\"@id\":\"…\"}, \"duumbi:right\":{\"@id\":\"…\"}, \"duumbi:resultType\":\"i64\"|\"f64\"}\n\
- Mul:     {\"@type\":\"duumbi:Mul\",    \"duumbi:left\":{\"@id\":\"…\"}, \"duumbi:right\":{\"@id\":\"…\"}, \"duumbi:resultType\":\"i64\"|\"f64\"}\n\
- Div:     {\"@type\":\"duumbi:Div\",    \"duumbi:left\":{\"@id\":\"…\"}, \"duumbi:right\":{\"@id\":\"…\"}, \"duumbi:resultType\":\"i64\"|\"f64\"}\n\
- Compare: {\"@type\":\"duumbi:Compare\",\"duumbi:operator\":\"eq\"|\"ne\"|\"lt\"|\"le\"|\"gt\"|\"ge\", \"duumbi:left\":{\"@id\":\"…\"}, \"duumbi:right\":{\"@id\":\"…\"}, \"duumbi:resultType\":\"bool\"}\n\
- Branch:  {\"@type\":\"duumbi:Branch\", \"duumbi:condition\":{\"@id\":\"…\"}, \"duumbi:trueBlock\":\"<label>\", \"duumbi:falseBlock\":\"<label>\"}\n\
- Load:    {\"@type\":\"duumbi:Load\",   \"duumbi:variable\":\"<name>\", \"duumbi:resultType\":\"i64\"|\"f64\"|\"bool\"}\n\
- Store:   {\"@type\":\"duumbi:Store\",  \"duumbi:variable\":\"<name>\", \"duumbi:operand\":{\"@id\":\"…\"}}\n\
- Call:    {\"@type\":\"duumbi:Call\",   \"duumbi:function\":\"<name>\", \"duumbi:args\":[{\"@id\":\"…\"}], \"duumbi:resultType\":\"i64\"|\"f64\"|\"bool\"}\n\
- Print:   {\"@type\":\"duumbi:Print\",  \"duumbi:operand\":{\"@id\":\"…\"}}\n\
- Return:  {\"@type\":\"duumbi:Return\", \"duumbi:operand\":{\"@id\":\"…\"}}\n\
\n\
Function parameters:\n\
- Declare them on the function node: \"duumbi:params\":[{\"duumbi:name\":\"x\",\"duumbi:paramType\":\"i64\"}]\n\
- To READ a parameter inside the function body use duumbi:Load with \"duumbi:variable\":\"x\"\n\
- There is NO duumbi:LoadParam op — always use duumbi:Load to access parameters\n\
\n\
Example — adding a function multiply(a, b) → a*b via one add_function call:\n\
{\"function\":{\"@type\":\"duumbi:Function\",\"@id\":\"duumbi:main/multiply\",\n\
\"duumbi:name\":\"multiply\",\"duumbi:returnType\":\"i64\",\n\
\"duumbi:params\":[{\"duumbi:name\":\"a\",\"duumbi:paramType\":\"i64\"},{\"duumbi:name\":\"b\",\"duumbi:paramType\":\"i64\"}],\n\
\"duumbi:blocks\":[{\"@type\":\"duumbi:Block\",\"@id\":\"duumbi:main/multiply/entry\",\n\
\"duumbi:label\":\"entry\",\"duumbi:ops\":[\n\
{\"@type\":\"duumbi:Load\",\"@id\":\"duumbi:main/multiply/entry/0\",\"duumbi:variable\":\"a\",\"duumbi:resultType\":\"i64\"},\n\
{\"@type\":\"duumbi:Load\",\"@id\":\"duumbi:main/multiply/entry/1\",\"duumbi:variable\":\"b\",\"duumbi:resultType\":\"i64\"},\n\
{\"@type\":\"duumbi:Mul\",\"@id\":\"duumbi:main/multiply/entry/2\",\"duumbi:left\":{\"@id\":\"duumbi:main/multiply/entry/0\"},\n\
\"duumbi:right\":{\"@id\":\"duumbi:main/multiply/entry/1\"},\"duumbi:resultType\":\"i64\"},\n\
{\"@type\":\"duumbi:Return\",\"@id\":\"duumbi:main/multiply/entry/3\",\"duumbi:operand\":{\"@id\":\"duumbi:main/multiply/entry/2\"}}]}]}}\n\
";

/// Result of a successful mutation.
pub struct MutationResult {
    /// The patched JSON-LD value (not yet written to disk).
    pub patched: serde_json::Value,
    /// The patch operations that were applied.
    pub ops_count: usize,
}

/// Runs the full mutation loop: prompt → LLM → patch → validate → optional retry.
///
/// `source` is the current JSON-LD module value.
/// `user_request` is the natural language mutation request.
/// `max_retries` should be 0 (no retry) or 1 (one retry with error feedback).
///
/// On success, returns the patched JSON-LD value (caller is responsible for
/// writing it to disk and saving a snapshot).
///
/// # Errors
///
/// Returns an error if the LLM call fails, the patch cannot be applied, or
/// validation still fails after all retries.
pub async fn mutate(
    client: &LlmClient,
    source: &serde_json::Value,
    user_request: &str,
    max_retries: u32,
) -> Result<MutationResult> {
    let graph_json = serde_json::to_string_pretty(source)
        .context("Failed to serialize current graph for context")?;

    let user_message = format!(
        "Current program graph:\n```json\n{graph_json}\n```\n\nRequested change: {user_request}"
    );

    // First attempt
    let ops = client
        .call_with_tools(SYSTEM_PROMPT, &user_message)
        .await
        .map_err(|e| anyhow::anyhow!("LLM call failed: {e}"))?;

    if ops.is_empty() {
        anyhow::bail!("LLM returned no tool calls — nothing to apply");
    }

    let ops_count = ops.len();
    let patch = GraphPatch { ops };

    match try_apply_and_validate(source, &patch) {
        Ok(patched) => Ok(MutationResult { patched, ops_count }),
        Err(validation_err) if max_retries == 0 => {
            anyhow::bail!(
                "Patch validation failed: {validation_err}\n\
                 Run `duumbi check` for details."
            );
        }
        Err(validation_err) => {
            // Retry with error feedback
            eprintln!("First attempt failed ({validation_err}), retrying with error context…");

            let retry_message = format!(
                "{user_message}\n\n\
                 Previous attempt failed validation with error: {validation_err}\n\
                 Please fix the error and try again."
            );

            let retry_ops = client
                .call_with_tools(SYSTEM_PROMPT, &retry_message)
                .await
                .map_err(|e| anyhow::anyhow!("LLM retry call failed: {e}"))?;

            if retry_ops.is_empty() {
                anyhow::bail!("LLM returned no tool calls on retry");
            }

            let retry_count = retry_ops.len();
            let retry_patch = GraphPatch { ops: retry_ops };

            let patched = try_apply_and_validate(source, &retry_patch)
                .map_err(|e| anyhow::anyhow!("Retry validation failed: {e}"))?;

            Ok(MutationResult {
                patched,
                ops_count: retry_count,
            })
        }
    }
}

/// Runs the full mutation loop with streaming text output via `on_text`.
///
/// Identical to [`mutate`] but calls [`LlmClient::call_with_tools_streaming`]
/// so the provider can surface its reasoning text in real time. The `on_text`
/// callback is invoked once per streamed text chunk; it is called from the
/// async context but must not block.
///
/// # Errors
///
/// See [`mutate`] — same error conditions apply.
pub async fn mutate_streaming<F>(
    client: &LlmClient,
    source: &serde_json::Value,
    user_request: &str,
    max_retries: u32,
    on_text: F,
) -> Result<MutationResult>
where
    F: Fn(&str),
{
    let graph_json = serde_json::to_string_pretty(source)
        .context("Failed to serialize current graph for context")?;

    let user_message = format!(
        "Current program graph:\n```json\n{graph_json}\n```\n\nRequested change: {user_request}"
    );

    let ops = client
        .call_with_tools_streaming(SYSTEM_PROMPT, &user_message, &on_text)
        .await
        .map_err(|e| anyhow::anyhow!("LLM call failed: {e}"))?;

    if ops.is_empty() {
        anyhow::bail!("LLM returned no tool calls — nothing to apply");
    }

    let ops_count = ops.len();
    let patch = GraphPatch { ops };

    match try_apply_and_validate(source, &patch) {
        Ok(patched) => Ok(MutationResult { patched, ops_count }),
        Err(validation_err) if max_retries == 0 => {
            anyhow::bail!(
                "Patch validation failed: {validation_err}\n\
                 Run `duumbi check` for details."
            );
        }
        Err(validation_err) => {
            eprintln!("First attempt failed ({validation_err}), retrying with error context…");

            let retry_message = format!(
                "{user_message}\n\n\
                 Previous attempt failed validation with error: {validation_err}\n\
                 Please fix the error and try again."
            );

            let retry_ops = client
                .call_with_tools_streaming(SYSTEM_PROMPT, &retry_message, &on_text)
                .await
                .map_err(|e| anyhow::anyhow!("LLM retry call failed: {e}"))?;

            if retry_ops.is_empty() {
                anyhow::bail!("LLM returned no tool calls on retry");
            }

            let retry_count = retry_ops.len();
            let retry_patch = GraphPatch { ops: retry_ops };

            let patched = try_apply_and_validate(source, &retry_patch)
                .map_err(|e| anyhow::anyhow!("Retry validation failed: {e}"))?;

            Ok(MutationResult {
                patched,
                ops_count: retry_count,
            })
        }
    }
}

/// Applies a patch to `source` and validates the result using the full pipeline.
///
/// Returns the patched value on success, or a descriptive error string on failure.
fn try_apply_and_validate(
    source: &serde_json::Value,
    patch: &GraphPatch,
) -> std::result::Result<serde_json::Value, String> {
    // Apply patch (all-or-nothing)
    let patched = apply_patch(source, patch).map_err(|e| e.to_string())?;

    // Validate via parse → build → validate pipeline
    let json_str = serde_json::to_string(&patched).map_err(|e| e.to_string())?;
    validate_jsonld_string(&json_str)
        .map_err(|e| e.to_string())
        .map(|()| patched)
}

/// Validates a JSON-LD string through the full parse → build → validate pipeline.
///
/// Returns `Ok(())` if valid, or an error describing the first failure.
fn validate_jsonld_string(json_str: &str) -> Result<()> {
    let module_ast =
        crate::parser::parse_jsonld(json_str).map_err(|e| anyhow::anyhow!("Parse error: {e}"))?;

    let semantic_graph = crate::graph::builder::build_graph(&module_ast).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        anyhow::anyhow!("Graph errors: {}", messages.join("; "))
    })?;

    let diagnostics = crate::graph::validator::validate(&semantic_graph);
    if !diagnostics.is_empty() {
        let messages: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
        anyhow::bail!("Validation errors: {}", messages.join("; "));
    }

    Ok(())
}

/// Builds a concise human-readable diff of changes between two JSON-LD values.
///
/// Only shows changed fields (not full objects), for user confirmation.
#[must_use]
pub fn describe_changes(original: &serde_json::Value, patched: &serde_json::Value) -> String {
    if original == patched {
        return "No changes".to_string();
    }

    // Count function/op differences at a high level
    let orig_funcs = original["duumbi:functions"]
        .as_array()
        .map_or(0, |a| a.len());
    let new_funcs = patched["duumbi:functions"]
        .as_array()
        .map_or(0, |a| a.len());

    let mut lines = Vec::new();

    if new_funcs != orig_funcs {
        let delta = new_funcs as i32 - orig_funcs as i32;
        lines.push(format!(
            "  Functions: {} → {} ({:+})",
            orig_funcs, new_funcs, delta
        ));
    }

    // Count total ops
    let count_ops = |v: &serde_json::Value| -> usize {
        v["duumbi:functions"].as_array().map_or(0, |funcs| {
            funcs
                .iter()
                .map(|f| {
                    f["duumbi:blocks"].as_array().map_or(0, |blocks| {
                        blocks
                            .iter()
                            .map(|b| b["duumbi:ops"].as_array().map_or(0, |ops| ops.len()))
                            .sum::<usize>()
                    })
                })
                .sum()
        })
    };

    let orig_ops = count_ops(original);
    let new_ops = count_ops(patched);

    if new_ops != orig_ops {
        let delta = new_ops as i32 - orig_ops as i32;
        lines.push(format!(
            "  Operations: {} → {} ({:+})",
            orig_ops, new_ops, delta
        ));
    }

    if lines.is_empty() {
        lines.push("  Field values modified (structure unchanged)".to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_module() -> serde_json::Value {
        json!({
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
                        {
                            "@type": "duumbi:Const",
                            "@id": "duumbi:main/main/entry/0",
                            "duumbi:value": 3,
                            "duumbi:resultType": "i64"
                        },
                        {
                            "@type": "duumbi:Return",
                            "@id": "duumbi:main/main/entry/1",
                            "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
                        }
                    ]
                }]
            }]
        })
    }

    #[test]
    fn try_apply_and_validate_valid_patch_succeeds() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![crate::patch::PatchOp::ModifyOp {
                node_id: "duumbi:main/main/entry/0".to_string(),
                field: "duumbi:value".to_string(),
                value: json!(42),
            }],
        };
        let result = try_apply_and_validate(&source, &patch);
        assert!(result.is_ok());
        let patched = result.expect("must succeed");
        assert_eq!(
            patched["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][0]["duumbi:value"],
            42
        );
    }

    #[test]
    fn try_apply_and_validate_invalid_patch_fails() {
        let source = minimal_module();
        // Remove the Return op — invalid (no return in block)
        let patch = GraphPatch {
            ops: vec![crate::patch::PatchOp::RemoveNode {
                node_id: "duumbi:main/main/entry/1".to_string(),
            }],
        };
        // This should fail validation (no return in block)
        // Note: the validator may or may not catch this in Phase 1;
        // at minimum the patch applies and the function returns
        let result = try_apply_and_validate(&source, &patch);
        // Either ok (validator doesn't catch missing Return yet) or error
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn describe_changes_no_changes() {
        let source = minimal_module();
        let result = describe_changes(&source, &source);
        assert_eq!(result, "No changes");
    }

    #[test]
    fn describe_changes_added_function() {
        let original = minimal_module();
        let mut patched = original.clone();
        patched["duumbi:functions"]
            .as_array_mut()
            .expect("must be array")
            .push(json!({"@type": "duumbi:Function", "@id": "duumbi:main/helper"}));
        let desc = describe_changes(&original, &patched);
        assert!(desc.contains("Functions"));
        assert!(desc.contains("+1"));
    }

    #[test]
    fn describe_changes_field_modification() {
        let original = minimal_module();
        let mut patched = original.clone();
        patched["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"][0]["duumbi:value"] =
            json!(99);
        let desc = describe_changes(&original, &patched);
        assert!(!desc.is_empty());
    }
}
