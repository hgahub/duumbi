//! LLM-assisted intent modification.
//!
//! Allows natural language modification of an existing [`IntentSpec`]. The
//! current spec is serialised to YAML, sent alongside the user's instruction
//! to the LLM, and the returned JSON is parsed back into an `IntentSpec`.
//!
//! Reuses [`super::create::extract_json`], [`super::create::call_plain_completion`],
//! and [`super::create::parse_llm_response`] from the creation pipeline.

use anyhow::{Context, Result};

use crate::agents::LlmProvider;
use crate::intent::spec::IntentSpec;

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

const INTENT_MODIFY_PROMPT: &str = "\
You are modifying an existing intent specification for a DUUMBI graph-based program.

You will receive the current intent spec as YAML and a natural language instruction
describing how to change it. Apply the requested modifications and return the FULL
modified spec as a JSON object — not a diff, not a partial update, the complete spec.

The JSON must have these fields:
- acceptance_criteria: list of concrete, testable requirements
- modules_create: list of module paths to create (e.g. \"calculator/ops\")
- modules_modify: list of module paths to modify (usually [\"app/main\"])
- test_cases: list of {name, function, args (i64 array), expected_return (i64)}
- dependencies: list of external module names (usually [])

Respond ONLY with valid JSON, no markdown, no explanation.";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Modifies an existing intent spec based on a natural language instruction.
///
/// Sends the current spec (as YAML) and the user's request to the LLM,
/// parses the response, and returns the modified spec. The caller is
/// responsible for saving the result.
#[must_use = "returns the modified spec — caller must save it"]
pub async fn modify_intent_with_llm(
    client: &dyn LlmProvider,
    current_spec: &IntentSpec,
    user_request: &str,
) -> Result<IntentSpec> {
    let current_yaml =
        serde_yaml::to_string(current_spec).context("Failed to serialise current intent spec")?;

    let user_message = format!(
        "Current intent spec:\n```yaml\n{current_yaml}```\n\n\
         Modify the spec as follows:\n{user_request}"
    );

    let raw = super::create::call_plain_completion(client, INTENT_MODIFY_PROMPT, &user_message)
        .await
        .context("LLM call for intent modification failed")?;

    // Re-use the same parser as intent create. The `description` parameter
    // populates `IntentSpec.intent` — we preserve the original intent string.
    let mut modified = super::create::parse_llm_response(&current_spec.intent, &raw)
        .context("Failed to parse LLM response for intent modification")?;

    // Preserve metadata from the original spec.
    modified.created_at = current_spec.created_at.clone();
    modified.status = current_spec.status.clone();
    modified.execution = current_spec.execution.clone();
    modified.context = current_spec.context.clone();

    Ok(modified)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};

    /// Builds a minimal spec for testing.
    fn sample_spec() -> IntentSpec {
        IntentSpec {
            intent: "Build a calculator".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["add(a,b) returns a+b".to_string()],
            modules: IntentModules {
                create: vec!["calculator/ops".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: vec![TestCase {
                name: "basic_add".to_string(),
                function: "add".to_string(),
                args: vec![3, 5],
                expected_return: 8,
            }],
            dependencies: vec![],
            context: None,
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            execution: None,
        }
    }

    #[test]
    fn sample_spec_serialises_to_yaml() {
        let spec = sample_spec();
        let yaml = serde_yaml::to_string(&spec).expect("yaml serialisation");
        assert!(yaml.contains("calculator"));
        assert!(yaml.contains("add"));
    }

    #[test]
    fn parse_modified_response_preserves_intent_field() {
        let raw = r#"{
            "acceptance_criteria": ["add works", "sub works"],
            "modules_create": ["calculator/ops"],
            "modules_modify": ["app/main"],
            "test_cases": [
                {"name": "add_test", "function": "add", "args": [1,2], "expected_return": 3},
                {"name": "sub_test", "function": "sub", "args": [5,3], "expected_return": 2}
            ],
            "dependencies": []
        }"#;
        let spec =
            super::super::create::parse_llm_response("Build a calculator", raw).expect("parse");
        assert_eq!(spec.intent, "Build a calculator");
        assert_eq!(spec.test_cases.len(), 2);
        assert_eq!(spec.acceptance_criteria.len(), 2);
    }
}
