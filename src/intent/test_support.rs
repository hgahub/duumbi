//! Shared fixtures for DUUMBI-779 execute-through-repair tests.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;

use crate::agents::{AgentError, LlmProvider};
use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};
use crate::patch::PatchOp;

/// Skeleton `main.jsonld` used by isolated-attempt fixtures.
pub(crate) const SKELETON_MAIN: &str = r#"{
  "@context": {
    "duumbi": "https://duumbi.dev/ns/core#"
  },
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [
    {
      "@type": "duumbi:Function",
      "@id": "duumbi:main/main",
      "duumbi:name": "main",
      "duumbi:returnType": "i64",
      "duumbi:blocks": [
        {
          "@type": "duumbi:Block",
          "@id": "duumbi:main/main/entry",
          "duumbi:label": "entry",
          "duumbi:ops": [
            {
              "@type": "duumbi:Const",
              "@id": "duumbi:main/main/entry/0",
              "duumbi:value": 0
            },
            {
              "@type": "duumbi:Return",
              "@id": "duumbi:main/main/entry/1",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
            }
          ]
        }
      ]
    }
  ]
}
"#;

/// Minimal intent that exercises verifier repair for `add(3, 5) = 8`.
#[must_use]
pub(crate) fn repair_fixture_spec() -> IntentSpec {
    IntentSpec {
        intent: "Build add".to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: vec!["add(a, b) returns a + b".to_string()],
        modules: IntentModules {
            create: Vec::new(),
            modify: vec!["app/main".to_string()],
        },
        test_cases: vec![TestCase {
            name: "addition".to_string(),
            function: "add".to_string(),
            args: vec![3, 5],
            expected_return: 8,
        }],
        dependencies: Vec::new(),
        bdd: Default::default(),
        context: None,
        created_at: None,
        execution: None,
    }
}

/// Writes a skeleton graph workspace for isolated execute tests.
///
/// # Errors
///
/// Returns an error when the graph directory or skeleton file cannot be written.
pub(crate) fn init_skeleton_workspace(path: &Path) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(path.join(".duumbi/graph"))?;
    std::fs::write(path.join(".duumbi/graph/main.jsonld"), SKELETON_MAIN)?;
    Ok(())
}

fn add_const_function(value: i64) -> serde_json::Value {
    json!({
        "@type": "duumbi:Function",
        "@id": "duumbi:main/add",
        "duumbi:name": "add",
        "duumbi:returnType": "i64",
        "duumbi:params": [
            {"duumbi:name": "a", "duumbi:paramType": "i64"},
            {"duumbi:name": "b", "duumbi:paramType": "i64"}
        ],
        "duumbi:blocks": [{
            "@type": "duumbi:Block",
            "@id": "duumbi:main/add/entry",
            "duumbi:label": "entry",
            "duumbi:ops": [
                {
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/add/entry/0",
                    "duumbi:value": value,
                    "duumbi:resultType": "i64"
                },
                {
                    "@type": "duumbi:Return",
                    "@id": "duumbi:main/add/entry/1",
                    "duumbi:operand": {"@id": "duumbi:main/add/entry/0"}
                }
            ]
        }]
    })
}

/// Adds `add` that always returns `0`.
#[must_use]
pub(crate) fn mutation_add_const_zero() -> Vec<PatchOp> {
    vec![PatchOp::AddFunction {
        function: add_const_function(0),
    }]
}

/// Changes the `add` constant to `value`.
#[must_use]
pub(crate) fn modify_add_const(value: i64) -> Vec<PatchOp> {
    vec![PatchOp::ModifyOp {
        node_id: "duumbi:main/add/entry/0".to_string(),
        field: "duumbi:value".to_string(),
        value: json!(value),
    }]
}

/// Scripted repair behavior for execute-through-repair fixtures.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RepairScript {
    /// Repair cycle is entered but no patch is written.
    NoPatch,
    /// Repair writes a still-wrong constant.
    PatchedFail,
    /// Repair writes `Const 8` so `add(3, 5) = 8` passes.
    RepairedSuccess,
}

/// Deterministic provider that drives mutation then optional repair.
pub(crate) struct ScriptedRepairProvider {
    script: RepairScript,
    /// Number of provider calls observed.
    pub calls: AtomicUsize,
}

impl ScriptedRepairProvider {
    /// Creates a provider for the given repair script.
    #[must_use]
    pub(crate) fn new(script: RepairScript) -> Self {
        Self {
            script,
            calls: AtomicUsize::new(0),
        }
    }

    fn ops_for(&self, user_message: &str) -> Result<Vec<PatchOp>, AgentError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let is_repair = user_message.contains("The following test cases FAILED");
        if !is_repair {
            return Ok(mutation_add_const_zero());
        }
        match self.script {
            RepairScript::NoPatch => Err(AgentError::NoToolCalls),
            RepairScript::PatchedFail => Ok(modify_add_const(1)),
            RepairScript::RepairedSuccess => Ok(modify_add_const(8)),
        }
    }
}

impl LlmProvider for ScriptedRepairProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn call_with_tools<'a>(
        &'a self,
        _system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        let result = self.ops_for(user_message);
        Box::pin(async move { result })
    }

    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        _on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        self.call_with_tools(system_prompt, user_message)
    }
}
