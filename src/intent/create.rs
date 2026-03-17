//! LLM-assisted intent spec creation.
//!
//! `duumbi intent create "<description>"` uses the LLM to generate a
//! structured [`IntentSpec`] from a natural language description, then
//! saves it to `.duumbi/intents/<slug>.yaml` after user confirmation.

use std::path::Path;

use anyhow::{Context, Result};

use crate::agents::LlmProvider;
use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};
use crate::intent::{IntentError, save_intent, slugify, unique_slug};

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

const INTENT_SYSTEM_PROMPT: &str = "\
You are a software architect helping to create a structured intent specification \
for a DUUMBI graph-based program. DUUMBI programs are composed of JSON-LD modules \
containing typed functions (i64 arithmetic).

Given a natural language description, generate a structured intent spec as JSON \
with these fields:
- acceptance_criteria: list of concrete, testable requirements
- modules_create: list of module paths to create (e.g. \"calculator/ops\")
- modules_modify: list of module paths to modify (usually [\"app/main\"])
- test_cases: list of {name, function, args (i64 array), expected_return (i64)}
- dependencies: list of external module names (usually [])

Keep test cases simple, concrete, and verifiable via exit codes.
Respond ONLY with valid JSON, no markdown, no explanation.

Example response:
{
  \"acceptance_criteria\": [\"add(a, b) returns a+b for i64\"],
  \"modules_create\": [\"calculator/ops\"],
  \"modules_modify\": [\"app/main\"],
  \"test_cases\": [{\"name\": \"basic_add\", \"function\": \"add\", \"args\": [3, 5], \"expected_return\": 8}],
  \"dependencies\": []
}";

// ---------------------------------------------------------------------------
// LLM-based spec generation
// ---------------------------------------------------------------------------

/// Calls the LLM to generate an `IntentSpec` from a natural language description.
///
/// On success, returns the spec with the given intent string populated.
pub async fn generate_spec_with_llm(
    client: &dyn LlmProvider,
    description: &str,
) -> Result<IntentSpec> {
    let user_message = format!("Create an intent spec for this programming task:\n\n{description}");

    // Use the existing call_with_tools infrastructure. Since we need a plain
    // text/JSON response rather than graph patch tools, we call with an empty
    // tool list and parse the text response directly.
    // Note: this calls the LLM's chat completion path (no tool use).
    let raw = call_plain_completion(client, INTENT_SYSTEM_PROMPT, &user_message).await?;

    parse_llm_response(description, &raw)
}

/// Extracts the first valid JSON object from an LLM response.
///
/// Handles common LLM response patterns:
/// - Pure JSON
/// - JSON wrapped in markdown code fences (```json ... ```)
/// - JSON with trailing text/explanation after the closing brace
///
/// Returns a slice of `raw` containing just the JSON object, or `None` if
/// no balanced `{ ... }` is found.
fn extract_json(raw: &str) -> Option<&str> {
    // Strip markdown code fences if present
    let stripped = raw.trim();
    let content = if stripped.starts_with("```") {
        let after_fence = stripped
            .strip_prefix("```json")
            .or_else(|| stripped.strip_prefix("```"))
            .unwrap_or(stripped);
        // Find the closing fence
        if let Some(end) = after_fence.rfind("```") {
            &after_fence[..end]
        } else {
            after_fence
        }
    } else {
        stripped
    };

    // Find the first '{' and its matching '}'
    let start = content.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in content[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&content[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }

    None
}

/// Parses the LLM's JSON response into an `IntentSpec`.
fn parse_llm_response(description: &str, raw: &str) -> Result<IntentSpec> {
    let json_str = extract_json(raw)
        .context("Failed to parse LLM response as JSON: no valid JSON object found")?;

    let value: serde_json::Value =
        serde_json::from_str(json_str).context("Failed to parse LLM response as JSON")?;

    let criteria: Vec<String> = value["acceptance_criteria"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let modules_create: Vec<String> = value["modules_create"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let modules_modify: Vec<String> = value["modules_modify"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| vec!["app/main".to_string()]);

    let test_cases: Vec<TestCase> = value["test_cases"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    let function = tc["function"].as_str()?.to_string();
                    // Skip "main" test cases — they are inherently fragile because
                    // the expected return value is speculative and often contradicts
                    // the ModifyMain task instruction ("exit with result of first
                    // call"). Individual function tests already validate correctness.
                    if function == "main" {
                        return None;
                    }
                    Some(TestCase {
                        name: tc["name"].as_str()?.to_string(),
                        function,
                        args: tc["args"]
                            .as_array()?
                            .iter()
                            .filter_map(|v| v.as_i64())
                            .collect(),
                        expected_return: tc["expected_return"].as_i64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let dependencies: Vec<String> = value["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let now = chrono_now();

    Ok(IntentSpec {
        intent: description.to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: criteria,
        modules: IntentModules {
            create: modules_create,
            modify: modules_modify,
        },
        test_cases,
        dependencies,
        created_at: Some(now),
        execution: None,
    })
}

/// Returns current UTC time as an ISO-8601 string (best-effort). Public for use by execute.rs.
pub fn chrono_now_pub() -> String {
    chrono_now()
}

/// Returns current UTC time as an ISO-8601 string (best-effort).
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as approximate ISO-8601 (no external date crate needed)
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Days since Unix epoch (2026-01-01 ≈ day 20454)
    let year = 1970 + days / 365;
    format!("{year}-01-01T{h:02}:{m:02}:{s:02}Z")
}

// ---------------------------------------------------------------------------
// Plain chat completion (no tool use)
// ---------------------------------------------------------------------------

/// Makes a plain chat completion call without tool use, returning the text response.
///
/// This bypasses the graph mutation tool infrastructure and returns the raw
/// assistant message text.
async fn call_plain_completion(
    client: &dyn LlmProvider,
    system: &str,
    user: &str,
) -> Result<String> {
    // Mutex lets the closure satisfy Fn + Send + Sync while accumulating chunks.
    let response = std::sync::Mutex::new(String::new());

    // AgentError::NoToolCalls is the expected success path here (the LLM outputs
    // plain text, not tool calls). All other errors (Http, ApiError, Timeout,
    // RateLimited, Parse) are fatal and must be surfaced to the caller.
    let result = client
        .call_with_tools_streaming(system, user, &|chunk: &str| {
            response
                .lock()
                .expect("invariant: mutex not poisoned")
                .push_str(chunk);
        })
        .await;

    if let Err(e) = result {
        if !matches!(e, crate::agents::AgentError::NoToolCalls) {
            return Err(anyhow::anyhow!("LLM call failed: {e}"));
        }
    }

    let text = response
        .into_inner()
        .expect("invariant: mutex not poisoned");
    if text.is_empty() {
        anyhow::bail!("LLM returned empty response for intent spec generation");
    }
    Ok(text)
}

// ---------------------------------------------------------------------------
// CLI flow helper
// ---------------------------------------------------------------------------

/// Full flow for `duumbi intent create`:
/// 1. Call LLM to generate spec
/// 2. Display the spec to the user
/// 3. Ask for confirmation
/// 4. Save to `.duumbi/intents/<slug>.yaml`
///
/// Returns the slug of the saved intent.
pub async fn run_create(
    client: &dyn LlmProvider,
    workspace: &Path,
    description: &str,
    yes: bool,
) -> Result<String> {
    eprintln!("Generating intent spec for: \"{description}\"…");

    let spec = generate_spec_with_llm(client, description)
        .await
        .context("Failed to generate intent spec")?;

    // Show preview
    crate::intent::review::print_spec_detail("(preview)", &spec);

    if !yes {
        eprint!("Save this intent? [Y/n] ");
        use std::io::Write;
        std::io::stderr().flush().ok();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read confirmation")?;
        if input.trim().to_lowercase() == "n" {
            eprintln!("Aborted.");
            anyhow::bail!("User aborted intent creation");
        }
    }

    let base_slug = slugify(description);
    let slug = unique_slug(workspace, &base_slug);

    save_intent(workspace, &slug, &spec).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;

    eprintln!("Intent saved as '.duumbi/intents/{slug}.yaml'");
    Ok(slug)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_llm_response() {
        let raw = r#"{
  "acceptance_criteria": ["add(a, b) returns a+b"],
  "modules_create": ["calculator/ops"],
  "modules_modify": ["app/main"],
  "test_cases": [{"name": "basic", "function": "add", "args": [3, 5], "expected_return": 8}],
  "dependencies": []
}"#;
        let spec = parse_llm_response("Build a calculator", raw).expect("must parse");
        assert_eq!(spec.acceptance_criteria.len(), 1);
        assert_eq!(spec.modules.create, vec!["calculator/ops"]);
        assert_eq!(spec.test_cases[0].expected_return, 8);
    }

    #[test]
    fn parse_llm_response_strips_markdown() {
        let raw = "```json\n{\"acceptance_criteria\": [], \"modules_create\": [], \"modules_modify\": [\"main\"], \"test_cases\": [], \"dependencies\": []}\n```";
        let spec = parse_llm_response("Test", raw).expect("must parse");
        assert!(spec.acceptance_criteria.is_empty());
    }

    #[test]
    fn parse_llm_response_invalid_json_returns_error() {
        let result = parse_llm_response("Test", "not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_llm_response_populates_intent_field() {
        let raw = r#"{"acceptance_criteria":[],"modules_create":[],"modules_modify":[],"test_cases":[],"dependencies":[]}"#;
        let spec = parse_llm_response("My custom intent", raw).expect("parse");
        assert_eq!(spec.intent, "My custom intent");
    }

    #[test]
    fn parse_llm_response_with_trailing_text() {
        let raw = r#"{"acceptance_criteria":["add works"],"modules_create":[],"modules_modify":["app/main"],"test_cases":[],"dependencies":[]}

This intent creates an addition function that..."#;
        let spec =
            parse_llm_response("Test trailing", raw).expect("must parse despite trailing text");
        assert_eq!(spec.acceptance_criteria, vec!["add works"]);
    }

    #[test]
    fn extract_json_pure_json() {
        let raw = r#"{"key": "value"}"#;
        assert_eq!(extract_json(raw), Some(r#"{"key": "value"}"#));
    }

    #[test]
    fn extract_json_with_trailing_text() {
        let raw = "{\"a\": 1}\nSome extra explanation";
        assert_eq!(extract_json(raw), Some("{\"a\": 1}"));
    }

    #[test]
    fn extract_json_nested_braces() {
        let raw = r#"{"outer": {"inner": 1}}"#;
        assert_eq!(extract_json(raw), Some(raw));
    }

    #[test]
    fn extract_json_braces_in_string() {
        let raw = r#"{"key": "a {b} c"}"#;
        assert_eq!(extract_json(raw), Some(raw));
    }

    #[test]
    fn extract_json_no_json() {
        assert_eq!(extract_json("no json here"), None);
    }
}
