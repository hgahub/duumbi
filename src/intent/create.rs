//! LLM-assisted intent spec creation.
//!
//! `duumbi intent create "<description>"` uses the LLM to generate a
//! structured [`IntentSpec`] from a natural language description, then
//! saves it to `.duumbi/intents/<slug>.yaml` after user confirmation.

use std::path::Path;

use anyhow::{Context, Result};

use crate::agents::{AgentError, LlmProvider};
use crate::intent::spec::{IntentContext, IntentModules, IntentSpec, IntentStatus, TestCase};
use crate::intent::{IntentError, save_intent, slugify, unique_slug};

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

const INTENT_SYSTEM_PROMPT: &str = "\
You are a software architect for the DUUMBI graph-based compiler. The user will \
describe a programming task in plain, natural language. They will NOT use DUUMBI \
internals. YOUR JOB is to translate their intent into a structured DUUMBI intent spec.

DUUMBI type rules (the user does not know these — you must apply them):
- All function arguments and return values are i64 (64-bit signed integers).
- Boolean results: encode as i64 where 1 = true, 0 = false.
- There are no floating-point, string, or collection types in test cases.
- Integer division truncates toward zero (like Rust / C).

Module naming: use <domain>/<purpose> format. Examples: math/ops, calculator/ops, \
geometry/triangle, convert/temp. For multi-function tasks use one module; for multi-\
domain tasks create separate modules.

Output a JSON object with these fields:
- acceptance_criteria: list of concrete, testable requirements (>= one per function)
- modules_create: list of new module paths to create
- modules_modify: list of existing modules to update (always include \"app/main\")
- test_cases: list of {name, function, args (i64 array), expected_return (i64)}
- dependencies: list of external module names (usually [])

Test case rules:
- Minimum 2 test cases per function: at least one normal case and one edge case.
- Edge cases to consider: zero, negative numbers, boundary values, equal inputs.
- Use descriptive snake_case names like \"gcd_zero\" or \"is_even_negative\".
- Do NOT create test cases for a function named \"main\".

Respond ONLY with valid JSON, no markdown, no explanation.

--- EXAMPLE 1 (simple, boolean result) ---
User: \"Check if a number is even\"
{
  \"acceptance_criteria\": [\
\"is_even(n) returns 1 when n is even\", \
\"is_even(n) returns 0 when n is odd\", \
\"is_even(0) returns 1 (zero is even)\", \
\"is_even handles negative numbers correctly\"],
  \"modules_create\": [\"math/ops\"],
  \"modules_modify\": [\"app/main\"],
  \"test_cases\": [\
{\"name\": \"even_positive\", \"function\": \"is_even\", \"args\": [4], \"expected_return\": 1}, \
{\"name\": \"odd_positive\", \"function\": \"is_even\", \"args\": [7], \"expected_return\": 0}, \
{\"name\": \"zero\", \"function\": \"is_even\", \"args\": [0], \"expected_return\": 1}, \
{\"name\": \"negative_even\", \"function\": \"is_even\", \"args\": [-6], \"expected_return\": 1}, \
{\"name\": \"negative_odd\", \"function\": \"is_even\", \"args\": [-3], \"expected_return\": 0}],
  \"dependencies\": []
}

--- EXAMPLE 2 (medium, edge cases) ---
User: \"Find the greatest common divisor of two numbers\"
{
  \"acceptance_criteria\": [\
\"gcd(a, b) returns the greatest common divisor of two integers\", \
\"gcd(a, 0) returns a\", \
\"gcd handles negative inputs by using absolute values\"],
  \"modules_create\": [\"math/gcd\"],
  \"modules_modify\": [\"app/main\"],
  \"test_cases\": [\
{\"name\": \"gcd_basic\", \"function\": \"gcd\", \"args\": [12, 8], \"expected_return\": 4}, \
{\"name\": \"gcd_coprime\", \"function\": \"gcd\", \"args\": [17, 5], \"expected_return\": 1}, \
{\"name\": \"gcd_equal\", \"function\": \"gcd\", \"args\": [7, 7], \"expected_return\": 7}, \
{\"name\": \"gcd_zero\", \"function\": \"gcd\", \"args\": [15, 0], \"expected_return\": 15}, \
{\"name\": \"gcd_negative\", \"function\": \"gcd\", \"args\": [-12, 8], \"expected_return\": 4}],
  \"dependencies\": []
}

--- EXAMPLE 3 (multi-function module) ---
User: \"Build a math library with double, square, and cube\"
{
  \"acceptance_criteria\": [\
\"double(n) returns 2*n for any integer\", \
\"square(n) returns n*n for any integer\", \
\"cube(n) returns n*n*n for any integer\"],
  \"modules_create\": [\"math/ops\"],
  \"modules_modify\": [\"app/main\"],
  \"test_cases\": [\
{\"name\": \"double_basic\", \"function\": \"double\", \"args\": [5], \"expected_return\": 10}, \
{\"name\": \"double_zero\", \"function\": \"double\", \"args\": [0], \"expected_return\": 0}, \
{\"name\": \"double_negative\", \"function\": \"double\", \"args\": [-4], \"expected_return\": -8}, \
{\"name\": \"square_basic\", \"function\": \"square\", \"args\": [6], \"expected_return\": 36}, \
{\"name\": \"square_negative\", \"function\": \"square\", \"args\": [-3], \"expected_return\": 9}, \
{\"name\": \"cube_basic\", \"function\": \"cube\", \"args\": [3], \"expected_return\": 27}, \
{\"name\": \"cube_negative\", \"function\": \"cube\", \"args\": [-2], \"expected_return\": -8}],
  \"dependencies\": []
}";

const INTENT_CLARIFY_PROMPT: &str = "\
You are preparing a DUUMBI intent before structured spec generation. Decide whether the \
request has enough product and integration context to execute well.

Ask clarification questions only when they materially affect implementation. Focus on:
- target surface: function, command, TUI flow, REST endpoint, whole application, or module
- entrypoint/caller and where the behavior is wired
- runtime environment and constraints
- acceptance behavior that should be verified

DUUMBI currently verifies integer graph behavior best. Capture broader product context, but keep \
generated executable tests within DUUMBI's current i64 graph capabilities.

Respond ONLY with valid JSON:
{
  \"needs_clarification\": true|false,
  \"questions\": [\"...\"],
  \"enhanced_description\": \"...\",
  \"context\": {
    \"scope\": \"...\",
    \"entrypoint\": \"...\",
    \"runtime_surface\": \"...\",
    \"integration_points\": [\"...\"],
    \"constraints\": [\"...\"]
  }
}";

/// TUI intent creation planning result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiIntentCreatePlan {
    /// The request can be converted into an intent immediately.
    Ready {
        /// Description to pass to structured spec generation.
        description: String,
        /// Clarified context to persist with the spec.
        context: Option<IntentContext>,
    },
    /// The user should answer material clarification questions first.
    NeedsClarification {
        /// Questions to show in the chat.
        questions: Vec<String>,
    },
}

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

    let mut spec = parse_llm_response(description, &raw)?;
    let _ = crate::intent::benchmarks::apply_known_benchmark(description, &mut spec);
    Ok(spec)
}

/// Uses an LLM to decide whether a TUI intent request needs clarification first.
pub async fn plan_tui_create(
    client: &dyn LlmProvider,
    workspace: &Path,
    description: &str,
) -> Result<TuiIntentCreatePlan> {
    let user_message = format!(
        "Workspace context:\n{}\n\nUser request:\n{}",
        workspace_context_summary(workspace),
        description
    );
    let raw = call_plain_completion(client, INTENT_CLARIFY_PROMPT, &user_message).await?;
    parse_clarification_response(description, &raw)
}

fn workspace_context_summary(workspace: &Path) -> String {
    let graph_dir = workspace.join(".duumbi/graph");
    let mut modules = Vec::new();
    collect_jsonld_module_paths(&graph_dir, &mut modules);
    if modules.is_empty() {
        return "No graph modules found.".to_string();
    }
    let mut lines = vec!["Existing graph modules:".to_string()];
    for path in modules.into_iter().take(20) {
        if let Ok(relative) = path.strip_prefix(&graph_dir) {
            lines.push(format!("- {}", relative.display()));
        }
    }
    lines.join("\n")
}

fn collect_jsonld_module_paths(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonld_module_paths(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonld") {
            out.push(path);
        }
    }
    out.sort();
}

pub(crate) fn parse_clarification_response(
    original_description: &str,
    raw: &str,
) -> Result<TuiIntentCreatePlan> {
    let Some(json_str) = extract_json(raw) else {
        return Ok(TuiIntentCreatePlan::Ready {
            description: original_description.to_string(),
            context: None,
        });
    };
    let value: serde_json::Value =
        serde_json::from_str(json_str).context("Failed to parse clarification JSON")?;

    let questions: Vec<String> = value["questions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(str::trim))
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    let needs_clarification = value["needs_clarification"].as_bool().unwrap_or(false);
    if needs_clarification && !questions.is_empty() {
        return Ok(TuiIntentCreatePlan::NeedsClarification { questions });
    }

    let description = value["enhanced_description"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(original_description)
        .to_string();
    let context = parse_intent_context(&value["context"]);
    Ok(TuiIntentCreatePlan::Ready {
        description,
        context,
    })
}

fn parse_intent_context(value: &serde_json::Value) -> Option<IntentContext> {
    if !value.is_object() {
        return None;
    }
    let context = IntentContext {
        scope: string_field(value, "scope"),
        entrypoint: string_field(value, "entrypoint"),
        runtime_surface: string_field(value, "runtime_surface"),
        integration_points: string_array_field(value, "integration_points"),
        constraints: string_array_field(value, "constraints"),
        clarification_log: Vec::new(),
    };
    let has_data = context.scope.is_some()
        || context.entrypoint.is_some()
        || context.runtime_surface.is_some()
        || !context.integration_points.is_empty()
        || !context.constraints.is_empty();
    has_data.then_some(context)
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value[key]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn string_array_field(value: &serde_json::Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(str::trim))
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
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
pub(crate) fn extract_json(raw: &str) -> Option<&str> {
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
pub(crate) fn parse_llm_response(description: &str, raw: &str) -> Result<IntentSpec> {
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
        context: None,
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
pub(crate) async fn call_plain_completion(
    client: &dyn LlmProvider,
    system: &str,
    user: &str,
) -> Result<String> {
    match client.answer(system, user).await {
        Ok(text) if !text.trim().is_empty() => return Ok(text),
        Ok(_) | Err(AgentError::NoToolCalls | AgentError::Parse(_)) => {}
        Err(e) => return Err(anyhow::anyhow!("LLM call failed: {e}")),
    }

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

    if let Err(e) = result
        && !matches!(e, crate::agents::AgentError::NoToolCalls)
    {
        return Err(anyhow::anyhow!("LLM call failed: {e}"));
    }

    let text = response
        .into_inner()
        .expect("invariant: mutex not poisoned");
    if text.trim().is_empty() {
        anyhow::bail!(
            "The selected model did not generate a text response.\n\
             This usually means the active provider could not satisfy plain completion for this task.\n\
             Configure another supported provider or retry when provider health improves."
        );
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
/// Returns the slug of the saved intent and a log of status messages.
/// The caller decides where to display the log (stderr for CLI, output
/// buffer for the ratatui REPL).
pub async fn run_create(
    client: &dyn LlmProvider,
    workspace: &Path,
    description: &str,
    yes: bool,
    log: &mut Vec<String>,
) -> Result<String> {
    run_create_with_context(client, workspace, description, None, yes, log).await
}

/// Full create flow with optional clarified context, used by the TUI.
pub async fn run_create_with_context(
    client: &dyn LlmProvider,
    workspace: &Path,
    description: &str,
    context: Option<IntentContext>,
    yes: bool,
    log: &mut Vec<String>,
) -> Result<String> {
    log.push(format!("Generating intent spec for: \"{description}\"…"));

    let mut spec = generate_spec_with_llm(client, description)
        .await
        .context("Failed to generate intent spec")?;
    spec.context = context;

    // Show preview
    crate::intent::review::format_spec_detail("(preview)", &spec, log);

    if !yes {
        eprint!("Save this intent? [Y/n] ");
        use std::io::Write;
        std::io::stderr().flush().ok();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read confirmation")?;
        if input.trim().to_lowercase() == "n" {
            log.push("Aborted.".to_string());
            anyhow::bail!("User aborted intent creation");
        }
    }

    let base_slug = slugify(description);
    let slug = unique_slug(workspace, &base_slug);

    save_intent(workspace, &slug, &spec).map_err(|e: IntentError| anyhow::anyhow!("{e}"))?;

    log.push(format!("Intent saved as '.duumbi/intents/{slug}.yaml'"));
    Ok(slug)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;

    use crate::patch::PatchOp;

    struct PlainMockProvider {
        answer_text: Option<&'static str>,
        tool_text: &'static str,
    }

    impl LlmProvider for PlainMockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn call_with_tools<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            Box::pin(async { Err(AgentError::NoToolCalls) })
        }

        fn call_with_tools_streaming<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
            on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            Box::pin(async move {
                if !self.tool_text.is_empty() {
                    on_text(self.tool_text);
                }
                Err(AgentError::NoToolCalls)
            })
        }

        fn answer<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
            Box::pin(async move {
                self.answer_text
                    .map(ToString::to_string)
                    .ok_or(AgentError::NoToolCalls)
            })
        }
    }

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
    fn parse_clarification_response_ready_with_context() {
        let raw = r#"{
  "needs_clarification": false,
  "questions": [],
  "enhanced_description": "Add add(a,b) to calculator/ops and call it from app/main",
  "context": {
    "scope": "function",
    "entrypoint": "app/main",
    "runtime_surface": "CLI",
    "integration_points": ["calculator/ops", "app/main"],
    "constraints": ["i64 only"]
  }
}"#;

        let plan = parse_clarification_response("Add", raw).expect("parse");

        match plan {
            TuiIntentCreatePlan::Ready {
                description,
                context: Some(context),
            } => {
                assert!(description.contains("calculator/ops"));
                assert_eq!(context.scope.as_deref(), Some("function"));
                assert_eq!(
                    context.integration_points,
                    vec!["calculator/ops", "app/main"]
                );
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn parse_clarification_response_questions() {
        let raw = r#"{
  "needs_clarification": true,
  "questions": ["Where should this be wired?", "Which function should call it?"],
  "enhanced_description": "",
  "context": {}
}"#;

        let plan = parse_clarification_response("Do a thing", raw).expect("parse");

        assert!(matches!(
            plan,
            TuiIntentCreatePlan::NeedsClarification { questions } if questions.len() == 2
        ));
    }

    #[test]
    fn parse_clarification_response_without_json_falls_back_to_ready() {
        let plan = parse_clarification_response(
            "Build a calculator with i64 arithmetic functions",
            "This is clear enough to implement.",
        )
        .expect("fallback");

        assert!(matches!(
            plan,
            TuiIntentCreatePlan::Ready { description, context: None }
                if description == "Build a calculator with i64 arithmetic functions"
        ));
    }

    #[tokio::test]
    async fn call_plain_completion_prefers_plain_answer() {
        let provider = PlainMockProvider {
            answer_text: Some("{\"ok\":true}"),
            tool_text: "",
        };

        let text = call_plain_completion(&provider, "system", "user")
            .await
            .expect("plain answer");

        assert_eq!(text, "{\"ok\":true}");
    }

    #[tokio::test]
    async fn call_plain_completion_falls_back_to_tool_text() {
        let provider = PlainMockProvider {
            answer_text: None,
            tool_text: "{\"fallback\":true}",
        };

        let text = call_plain_completion(&provider, "system", "user")
            .await
            .expect("tool fallback");

        assert_eq!(text, "{\"fallback\":true}");
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
