//! Intent specification types and serialization.
//!
//! An [`IntentSpec`] describes a user's programming intent in structured
//! YAML format: what to build, which modules to create or modify, acceptance
//! criteria, and test cases that must pass after execution.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Status types
// ---------------------------------------------------------------------------

/// Lifecycle status of an intent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    /// Not yet executed.
    #[default]
    Pending,
    /// Currently being executed.
    InProgress,
    /// All tasks completed and all tests passed.
    Completed,
    /// Execution failed on one or more tasks or tests.
    Failed,
}

impl std::fmt::Display for IntentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("pending"),
            Self::InProgress => f.write_str("in_progress"),
            Self::Completed => f.write_str("completed"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Task types (produced by Coordinator, consumed by execute)
// ---------------------------------------------------------------------------

/// What kind of mutation a task represents.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TaskKind {
    /// Create a new module (and its functions).
    CreateModule {
        /// Module path/name to create, e.g. `"calculator/ops"`.
        module_name: String,
    },
    /// Add a function to an existing module.
    AddFunction {
        /// Module that receives the new function.
        module_name: String,
        /// Human-readable description of the function to add.
        description: String,
    },
    /// Modify an existing function in a module.
    ModifyFunction {
        /// Module containing the function.
        module_name: String,
        /// Name of the function to modify.
        function_name: String,
        /// Human-readable description of the modification.
        description: String,
    },
    /// Modify the `main` function to demonstrate/integrate the implementation.
    ModifyMain {
        /// What the main function should do.
        description: String,
    },
}

/// Execution status of a single task.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting to be executed.
    #[default]
    Pending,
    /// Currently executing.
    InProgress,
    /// Executed successfully.
    Completed,
    /// Execution failed with the given error message.
    Failed(String),
}

/// A single unit of work produced by the Coordinator Agent.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Task {
    /// Sequential task number (1-based).
    pub id: usize,
    /// What kind of mutation this task performs.
    pub kind: TaskKind,
    /// Human-readable description shown to the user during execution.
    pub description: String,
    /// Current execution state of this task.
    #[serde(default)]
    pub status: TaskStatus,
}

impl Task {
    /// Returns the mutation prompt to send to the LLM orchestrator.
    pub fn mutation_prompt(&self) -> String {
        match &self.kind {
            TaskKind::CreateModule { module_name } => {
                format!(
                    "Create a new module named '{}'. {}",
                    module_name, self.description
                )
            }
            TaskKind::AddFunction {
                module_name,
                description,
            } => {
                format!(
                    "In module '{}', add a function as follows: {}",
                    module_name, description
                )
            }
            TaskKind::ModifyFunction {
                module_name,
                function_name,
                description,
            } => {
                format!(
                    "In module '{}', modify function '{}': {}",
                    module_name, function_name, description
                )
            }
            TaskKind::ModifyMain { description } => {
                format!("Modify the main function: {}", description)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test case types
// ---------------------------------------------------------------------------

/// A test case to verify after intent execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestCase {
    /// Short name identifying this test, e.g. `"addition"`.
    pub name: String,
    /// Name of the function to call, e.g. `"add"`.
    pub function: String,
    /// Integer arguments to pass to the function.
    pub args: Vec<i64>,
    /// Expected i64 return value.
    pub expected_return: i64,
}

// ---------------------------------------------------------------------------
// Module hints
// ---------------------------------------------------------------------------

/// Which modules the intent creates or modifies.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct IntentModules {
    /// Modules to create from scratch.
    #[serde(default)]
    pub create: Vec<String>,
    /// Existing modules to modify.
    #[serde(default)]
    pub modify: Vec<String>,
}

// ---------------------------------------------------------------------------
// Execution metadata (appended on completion)
// ---------------------------------------------------------------------------

/// Metadata appended to the YAML file after successful execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionMeta {
    /// When the intent finished executing.
    pub completed_at: String,
    /// Number of tasks that completed successfully.
    pub tasks_completed: usize,
    /// Number of test cases that passed.
    pub tests_passed: usize,
    /// Total number of test cases.
    pub tests_total: usize,
}

// ---------------------------------------------------------------------------
// The intent spec itself
// ---------------------------------------------------------------------------

/// A structured intent specification stored as `.duumbi/intents/<slug>.yaml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntentSpec {
    /// Natural language description of the intent.
    pub intent: String,

    /// Schema version (currently always `1`).
    #[serde(default = "default_version")]
    pub version: u32,

    /// Current lifecycle status.
    #[serde(default)]
    pub status: IntentStatus,

    /// Human-readable list of requirements the implementation must satisfy.
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,

    /// Which modules to create or modify.
    #[serde(default)]
    pub modules: IntentModules,

    /// Test cases that the Verifier Agent will run after execution.
    #[serde(default)]
    pub test_cases: Vec<TestCase>,

    /// Names of external dependencies required by this intent.
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// ISO-8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Execution metadata, set on completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionMeta>,
}

fn default_version() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yaml() -> &'static str {
        r#"intent: "Build a simple calculator with 4 operations"
version: 1
status: pending
acceptance_criteria:
  - "4 functions: add, sub, mul, div (i64 -> i64)"
  - "div returns 0 on division by zero"
modules:
  create:
    - "calculator/ops"
  modify:
    - "app/main"
test_cases:
  - name: "addition"
    function: "add"
    args: [3, 5]
    expected_return: 8
  - name: "division_by_zero"
    function: "div"
    args: [10, 0]
    expected_return: 0
dependencies: []
created_at: "2026-03-06T10:00:00Z"
"#
    }

    #[test]
    fn parse_sample_yaml() {
        let spec: IntentSpec = serde_yaml::from_str(sample_yaml()).expect("must parse");
        assert_eq!(spec.intent, "Build a simple calculator with 4 operations");
        assert_eq!(spec.status, IntentStatus::Pending);
        assert_eq!(spec.modules.create, vec!["calculator/ops"]);
        assert_eq!(spec.test_cases.len(), 2);
        assert_eq!(spec.test_cases[0].expected_return, 8);
    }

    #[test]
    fn roundtrip_yaml() {
        let spec: IntentSpec = serde_yaml::from_str(sample_yaml()).expect("parse");
        let serialized = serde_yaml::to_string(&spec).expect("serialize");
        let reparsed: IntentSpec = serde_yaml::from_str(&serialized).expect("reparse");
        assert_eq!(reparsed.intent, spec.intent);
        assert_eq!(reparsed.test_cases.len(), spec.test_cases.len());
    }

    #[test]
    fn intent_status_display() {
        assert_eq!(IntentStatus::Pending.to_string(), "pending");
        assert_eq!(IntentStatus::Completed.to_string(), "completed");
        assert_eq!(IntentStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn task_mutation_prompt_create_module() {
        let task = Task {
            id: 1,
            kind: TaskKind::CreateModule {
                module_name: "calculator/ops".to_string(),
            },
            description: "With functions add, sub, mul, div.".to_string(),
            status: TaskStatus::Pending,
        };
        let prompt = task.mutation_prompt();
        assert!(prompt.contains("calculator/ops"));
        assert!(prompt.contains("With functions"));
    }

    #[test]
    fn task_mutation_prompt_modify_main() {
        let task = Task {
            id: 3,
            kind: TaskKind::ModifyMain {
                description: "Call add(3,5) and print the result.".to_string(),
            },
            description: "Demo all operations.".to_string(),
            status: TaskStatus::Pending,
        };
        assert!(task.mutation_prompt().contains("Modify the main function"));
    }
}
