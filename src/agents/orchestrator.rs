//! AI mutation orchestrator.
//!
//! Combines LLM provider calls, GraphPatch application, validation,
//! and retry logic into a single [`mutate`] entry point.
//!
//! # Retry pipeline
//!
//! When a mutation fails validation, the orchestrator retries up to
//! `max_retries` times with escalating context:
//!
//! - **Retry 1:** Structured error feedback (error code + nodeId + fix hint)
//! - **Retry 2:** Same as above + a relevant few-shot example
//! - **Retry 3:** Same as above + a simplified instruction to use `replace_block`
//!
//! Each retry sends the full original user message plus the escalating context,
//! always working from the original `source` graph (not a partially-patched one).

use anyhow::{Context, Result};

use crate::agents::LlmClient;
use crate::errors::Diagnostic;
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
- To REWRITE a block body (change Add to Call, insert ops before Return, etc.):\n\
  Use replace_block — provide the block_id and the COMPLETE new ops array ending with Return/Branch.\n\
  This is the PREFERRED approach: one tool call, no risk of partial state.\n\
  Example: replace_block 'duumbi:main/main/entry' with [Const/0, Const/1, Call/2, Print/3, Return/4].\n\
- Only use remove_node + add_op for truly additive changes (e.g. appending a new block after an existing one).\n\
- NEVER use remove_node without add_op in the same response.\n\
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
    /// Number of patch operations applied.
    pub ops_count: usize,
}

/// Runs the full mutation loop: prompt → LLM → patch → validate → retry.
///
/// `source` is the current JSON-LD module value.
/// `user_request` is the natural language mutation request.
/// `max_retries` is the maximum number of additional attempts after the first
/// failure (0 = no retry, 3 = up to 3 retries = 4 total attempts).
///
/// Retry escalation:
/// - Retry 1: structured error feedback (code + nodeId + fix hint)
/// - Retry 2: same + a relevant few-shot example
/// - Retry 3: same + a simplified `replace_block` instruction
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

    let base_message = format!(
        "Current program graph:\n```json\n{graph_json}\n```\n\nRequested change: {user_request}"
    );

    // First attempt
    let ops = client
        .call_with_tools(SYSTEM_PROMPT, &base_message)
        .await
        .map_err(|e| anyhow::anyhow!("LLM call failed: {e}"))?;

    if ops.is_empty() {
        anyhow::bail!("LLM returned no tool calls — nothing to apply");
    }

    let ops_count = ops.len();
    let patch = GraphPatch { ops };

    match try_apply_collecting_diagnostics(source, &patch, false) {
        Ok(patched) => Ok(MutationResult { patched, ops_count }),
        Err(_) if max_retries == 0 => {
            anyhow::bail!("Patch validation failed. Run `duumbi check` for details.");
        }
        Err((mut last_error_msg, mut last_diagnostics)) => {
            for attempt in 0..max_retries {
                let attempt_num = attempt + 1;
                eprintln!("Attempt {attempt_num} failed, retry {attempt_num}/{max_retries}…");

                let retry_msg = build_retry_message(
                    &base_message,
                    attempt,
                    &last_diagnostics,
                    Some(&last_error_msg),
                    user_request,
                );

                let retry_ops = client
                    .call_with_tools(SYSTEM_PROMPT, &retry_msg)
                    .await
                    .map_err(|e| anyhow::anyhow!("LLM retry call failed: {e}"))?;

                if retry_ops.is_empty() {
                    anyhow::bail!("LLM returned no tool calls on retry {attempt_num}");
                }

                let retry_count = retry_ops.len();
                let retry_patch = GraphPatch { ops: retry_ops };

                match try_apply_collecting_diagnostics(source, &retry_patch, false) {
                    Ok(patched) => {
                        return Ok(MutationResult {
                            patched,
                            ops_count: retry_count,
                        });
                    }
                    Err((new_msg, new_diags)) => {
                        last_error_msg = new_msg;
                        last_diagnostics = new_diags;
                        if attempt + 1 >= max_retries {
                            let summary = format_retry_feedback_with_message(
                                &last_diagnostics,
                                Some(&last_error_msg),
                            );
                            anyhow::bail!(
                                "All {max_retries} retries exhausted. Last errors:\n{summary}"
                            );
                        }
                    }
                }
            }
            unreachable!("retry loop must return or bail before this point");
        }
    }
}

/// Runs the full mutation loop with streaming text output via `on_text`.
///
/// Identical to [`mutate`] but calls [`LlmClient::call_with_tools_streaming`]
/// so the provider can surface its reasoning text in real time. The `on_text`
/// callback is invoked once per streamed text chunk.
///
/// When `library_mode` is `true`, the validation step uses
/// [`build_graph_no_call_check`] which skips the `main` function requirement
/// and intra-module `Call` validation — appropriate for library modules.
///
/// # Errors
///
/// See [`mutate`] — same error conditions apply.
pub async fn mutate_streaming<F>(
    client: &LlmClient,
    source: &serde_json::Value,
    user_request: &str,
    max_retries: u32,
    library_mode: bool,
    on_text: F,
) -> Result<MutationResult>
where
    F: Fn(&str),
{
    let graph_json = serde_json::to_string_pretty(source)
        .context("Failed to serialize current graph for context")?;

    let base_message = format!(
        "Current program graph:\n```json\n{graph_json}\n```\n\nRequested change: {user_request}"
    );

    let ops = client
        .call_with_tools_streaming(SYSTEM_PROMPT, &base_message, &on_text)
        .await
        .map_err(|e| anyhow::anyhow!("LLM call failed: {e}"))?;

    if ops.is_empty() {
        anyhow::bail!("LLM returned no tool calls — nothing to apply");
    }

    let ops_count = ops.len();
    let patch = GraphPatch { ops };

    match try_apply_collecting_diagnostics(source, &patch, library_mode) {
        Ok(patched) => Ok(MutationResult { patched, ops_count }),
        Err(_) if max_retries == 0 => {
            anyhow::bail!("Patch validation failed. Run `duumbi check` for details.");
        }
        Err((mut last_error_msg, mut last_diagnostics)) => {
            for attempt in 0..max_retries {
                let attempt_num = attempt + 1;
                eprintln!("Attempt {attempt_num} failed, retry {attempt_num}/{max_retries}…");

                let retry_msg = build_retry_message(
                    &base_message,
                    attempt,
                    &last_diagnostics,
                    Some(&last_error_msg),
                    user_request,
                );

                let retry_ops = client
                    .call_with_tools_streaming(SYSTEM_PROMPT, &retry_msg, &on_text)
                    .await
                    .map_err(|e| anyhow::anyhow!("LLM retry call failed: {e}"))?;

                if retry_ops.is_empty() {
                    anyhow::bail!("LLM returned no tool calls on retry {attempt_num}");
                }

                let retry_count = retry_ops.len();
                let retry_patch = GraphPatch { ops: retry_ops };

                match try_apply_collecting_diagnostics(source, &retry_patch, library_mode) {
                    Ok(patched) => {
                        return Ok(MutationResult {
                            patched,
                            ops_count: retry_count,
                        });
                    }
                    Err((new_msg, new_diags)) => {
                        last_error_msg = new_msg;
                        last_diagnostics = new_diags;
                        if attempt + 1 >= max_retries {
                            let summary = format_retry_feedback_with_message(
                                &last_diagnostics,
                                Some(&last_error_msg),
                            );
                            anyhow::bail!(
                                "All {max_retries} retries exhausted. Last errors:\n{summary}"
                            );
                        }
                    }
                }
            }
            unreachable!("retry loop must return or bail before this point");
        }
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Applies a patch to `source` and validates the result, returning structured
/// diagnostics on failure.
///
/// When `library_mode` is `true`, the graph builder skips the `main` entry
/// function requirement and intra-module `Call` validation — appropriate for
/// library modules that only export functions.
///
/// Returns `Ok(patched_value)` on success, or
/// `Err((summary_string, diagnostics))` on failure.
fn try_apply_collecting_diagnostics(
    source: &serde_json::Value,
    patch: &GraphPatch,
    library_mode: bool,
) -> Result<serde_json::Value, (String, Vec<Diagnostic>)> {
    let patched = apply_patch(source, patch).map_err(|e| (e.to_string(), vec![]))?;

    let json_str = serde_json::to_string(&patched).map_err(|e| (e.to_string(), vec![]))?;

    let module_ast = crate::parser::parse_jsonld(&json_str)
        .map_err(|e| (format!("Parse error: {e}"), vec![]))?;

    let build_result = if library_mode {
        crate::graph::builder::build_graph_no_call_check(&module_ast)
    } else {
        crate::graph::builder::build_graph(&module_ast)
    };

    let semantic_graph = build_result.map_err(|errors| {
        let msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        // GraphError is a separate type from Diagnostic; no structured diagnostics here.
        (format!("Graph build errors: {msg}"), vec![])
    })?;

    let diagnostics = crate::graph::validator::validate(&semantic_graph);
    if !diagnostics.is_empty() {
        let msg = diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err((format!("Validation errors: {msg}"), diagnostics));
    }

    Ok(patched)
}

/// Applies a patch to `source` and validates the result.
///
/// Returns the patched value on success, or a descriptive error string on
/// failure. Used internally by tests; prefer
/// [`try_apply_collecting_diagnostics`] for new code.
#[cfg(test)]
pub(crate) fn try_apply_and_validate(
    source: &serde_json::Value,
    patch: &GraphPatch,
) -> std::result::Result<serde_json::Value, String> {
    try_apply_collecting_diagnostics(source, patch, false).map_err(|(msg, _)| msg)
}

// ---------------------------------------------------------------------------
// Retry feedback formatting
// ---------------------------------------------------------------------------

/// Formats structured error feedback for the LLM retry prompt.
///
/// Produces a block with per-error details (code, nodeId, message) and
/// per-code fix hints. When diagnostics are empty but an error message
/// is available, includes that message instead of a generic fallback.
#[cfg(test)]
pub fn format_retry_feedback(diagnostics: &[Diagnostic]) -> String {
    format_retry_feedback_with_message(diagnostics, None)
}

/// Like [`format_retry_feedback`] but accepts an optional error message
/// from the patch application / graph building phase.
pub fn format_retry_feedback_with_message(
    diagnostics: &[Diagnostic],
    error_message: Option<&str>,
) -> String {
    if diagnostics.is_empty() {
        let detail = error_message
            .map(|msg| format!("Previous attempt failed: {msg}"))
            .unwrap_or_else(|| {
                "Previous attempt failed. No specific diagnostic information available.".to_string()
            });
        return format!(
            "{detail}\n\
             Fix hints:\n\
             - Check all required fields are present for the op type\n\
             - Ensure all @id references are valid and unique\n\
             - Use replace_block for atomic block rewrites"
        );
    }

    let mut lines = vec!["Previous attempt failed. Errors:".to_string()];
    for d in diagnostics {
        let node_info = d
            .node_id
            .as_deref()
            .map(|n| format!(" at {n}"))
            .unwrap_or_default();
        lines.push(format!(
            "- {} {}{}: {}",
            d.code, d.code, node_info, d.message
        ));
    }

    lines.push(String::new());
    lines.push("Fix hints:".to_string());

    // Deduplicate codes to avoid repeated hints
    let mut seen = std::collections::HashSet::new();
    for d in diagnostics {
        if seen.insert(d.code.as_str())
            && let Some(hint) = hint_for_code(&d.code)
        {
            lines.push(format!("- For {}: {}", d.code, hint));
        }
    }

    lines.join("\n")
}

/// Returns a fix hint string for the given error code, or `None` if unknown.
fn hint_for_code(code: &str) -> Option<&'static str> {
    match code {
        "E001" => Some(
            "ensure binary ops (Add, Sub, Mul, Div, Compare) have operands with matching \
             resultType (both i64 or both f64)",
        ),
        "E002" => Some(
            "use a valid @type: Const, ConstF64, ConstBool, Add, Sub, Mul, Div, Compare, \
             Branch, Call, Load, Store, Print, Return",
        ),
        "E003" => Some(
            "ensure all required fields are present for this op type — \
             Add/Sub/Mul/Div need duumbi:left + duumbi:right + duumbi:resultType; \
             Return needs duumbi:operand; Branch needs duumbi:condition + duumbi:trueBlock + duumbi:falseBlock",
        ),
        "E004" => Some(
            "ensure all @id references point to existing nodes defined earlier in the same module",
        ),
        "E005" => Some(
            "ensure all @id values are globally unique — use format \
             duumbi:<module>/<function>/<block>/<index>",
        ),
        "E006" => Some("add a function named 'main' with duumbi:returnType"),
        "E007" => Some("remove the circular data-flow dependency between ops"),
        "E008" => Some("check linker configuration and that all referenced functions are compiled"),
        "E009" => Some(
            "check JSON-LD structure: every node needs @type, @id; \
             every block must end with Return or Branch",
        ),
        "E010" => Some(
            "ensure all called functions are exported by their module and listed in \
             duumbi:imports on the calling module",
        ),
        "E011" => Some(
            "ensure the dependency module exists in workspace (.duumbi/graph/), \
             vendor (.duumbi/vendor/), or cache (.duumbi/cache/) layer",
        ),
        "E012" => Some(
            "use explicit scope qualifiers (@scope/module) to resolve the module name conflict",
        ),
        _ => None,
    }
}

/// Builds the full retry prompt message, escalating context per attempt number.
///
/// - `attempt` 0: structured error feedback only
/// - `attempt` 1: + a relevant few-shot example
/// - `attempt` 2+: + simplified `replace_block` instruction
fn build_retry_message(
    base_user_message: &str,
    attempt: u32,
    diagnostics: &[Diagnostic],
    error_message: Option<&str>,
    user_request: &str,
) -> String {
    let feedback = format_retry_feedback_with_message(diagnostics, error_message);
    let mut msg = format!("{base_user_message}\n\n{feedback}");

    // Step 2: inject a relevant few-shot example
    if attempt >= 1
        && let Some(example) = crate::examples::select_example(diagnostics, user_request)
    {
        msg.push_str(&format!(
            "\n\nRelevant example (similar successful mutation):\n{example}"
        ));
    }

    // Step 3: simplified instruction
    if attempt >= 2 {
        msg.push_str(
            "\n\nSimplified instruction: Use replace_block to atomically rewrite any block \
             that needs changes. Provide the COMPLETE new ops array: all Loads first, then \
             the operation, then Print (if needed), then Return as the LAST op. \
             Never leave a block without a Return or Branch as its final op.",
        );
    }

    msg
}

// ---------------------------------------------------------------------------
// Diff summary
// ---------------------------------------------------------------------------

/// Builds a concise human-readable diff of changes between two JSON-LD values.
///
/// Only shows changed fields (not full objects), for user confirmation.
#[must_use]
pub fn describe_changes(original: &serde_json::Value, patched: &serde_json::Value) -> String {
    if original == patched {
        return "No changes".to_string();
    }

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    fn try_apply_and_validate_invalid_patch_does_not_panic() {
        let source = minimal_module();
        let patch = GraphPatch {
            ops: vec![crate::patch::PatchOp::RemoveNode {
                node_id: "duumbi:main/main/entry/1".to_string(),
            }],
        };
        let _ = try_apply_and_validate(&source, &patch);
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

    #[test]
    fn format_retry_feedback_empty_diagnostics() {
        let result = format_retry_feedback(&[]);
        assert!(result.contains("No specific diagnostic"));
        assert!(result.contains("replace_block"));
    }

    #[test]
    fn format_retry_feedback_with_diagnostics() {
        let diags = vec![
            crate::errors::Diagnostic::error(
                crate::errors::codes::E003_MISSING_FIELD,
                "field 'duumbi:right' required",
            ),
            crate::errors::Diagnostic::error(
                crate::errors::codes::E001_TYPE_MISMATCH,
                "Add expects matching operand types",
            ),
        ];
        let result = format_retry_feedback(&diags);
        assert!(result.contains("Previous attempt failed"));
        assert!(result.contains("E003"));
        assert!(result.contains("E001"));
        assert!(result.contains("Fix hints"));
        assert!(result.contains("duumbi:right"));
    }

    #[test]
    fn format_retry_feedback_deduplicates_codes() {
        let diags = vec![
            crate::errors::Diagnostic::error(crate::errors::codes::E003_MISSING_FIELD, "err 1"),
            crate::errors::Diagnostic::error(crate::errors::codes::E003_MISSING_FIELD, "err 2"),
        ];
        let result = format_retry_feedback(&diags);
        // E003 hint should appear only once
        let count = result.matches("For E003:").count();
        assert_eq!(count, 1, "deduplicated hint must appear exactly once");
    }

    #[test]
    fn build_retry_message_escalates_per_attempt() {
        let base = "Current program graph:\n```json\n{}\n```\n\nRequested change: add function";
        let diags = vec![crate::errors::Diagnostic::error(
            crate::errors::codes::E003_MISSING_FIELD,
            "missing field",
        )];

        let msg0 = build_retry_message(base, 0, &diags, None, "add function");
        let msg1 = build_retry_message(base, 1, &diags, None, "add function");
        let msg2 = build_retry_message(base, 2, &diags, None, "add function");

        // Attempt 0: no example, no simplified instruction
        assert!(!msg0.contains("Simplified instruction"));

        // Attempt 2: simplified instruction present
        assert!(msg2.contains("Simplified instruction"));

        // Each subsequent message should be longer (more context)
        assert!(msg1.len() >= msg0.len());
        assert!(msg2.len() >= msg1.len());
    }
}
