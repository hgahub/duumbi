//! Coordinator Agent — decomposes an [`IntentSpec`] into an ordered [`Vec<Task>`].
//!
//! # Strategy (M5)
//!
//! The M5 coordinator uses a **rule-based** decomposition that deterministically
//! generates tasks from the intent spec without an LLM call:
//!
//! 1. For each module in `modules.create` → `CreateModule` task
//! 2. For each module in `modules.modify` → `AddFunction` tasks (one per criterion)
//! 3. Always ends with a `ModifyMain` task
//!
//! This gives a reliable, fast execution path that works without an API key.
//! LLM-based decomposition will be added in a later milestone.

use super::spec::{IntentSpec, Task, TaskKind, TaskStatus};

/// Decomposes an [`IntentSpec`] into an ordered list of [`Task`]s.
///
/// Tasks are ordered so that dependencies are respected:
/// modules are created before they are referenced by other tasks.
pub fn decompose(spec: &IntentSpec) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut id = 1;

    // Phase 1: Create new modules
    for module_name in &spec.modules.create {
        let criteria_summary = spec
            .acceptance_criteria
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");

        // Collect unique function names targeted at this module from test_cases.
        // These must all appear in the module's duumbi:exports array.
        let module_stem = module_name.split('/').next_back().unwrap_or(module_name);
        let mut export_names: Vec<&str> = spec
            .test_cases
            .iter()
            .map(|tc| tc.function.as_str())
            .filter(|&f| f != "main")
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        export_names.sort_unstable();
        let exports_hint = if export_names.is_empty() {
            String::new()
        } else {
            format!(
                " IMPORTANT: the duumbi:exports array MUST include ALL of these functions: [{}].",
                export_names.join(", ")
            )
        };

        let description = if criteria_summary.is_empty() {
            format!("Create module '{module_name}' as described in the intent.{exports_hint}")
        } else {
            format!("Create module '{module_name}'. Requirements: {criteria_summary}{exports_hint}")
        };
        let _ = module_stem; // used only for context above

        tasks.push(Task {
            id,
            kind: TaskKind::CreateModule {
                module_name: module_name.clone(),
            },
            description,
            status: TaskStatus::Pending,
        });
        id += 1;
    }

    // Phase 2: Modify existing modules (one task per module)
    for module_name in &spec.modules.modify {
        if module_name == "app/main" || module_name == "main" {
            // Main modification is handled in phase 3
            continue;
        }

        let description = format!(
            "Modify module '{}' to satisfy the acceptance criteria: {}",
            module_name,
            spec.acceptance_criteria.join("; ")
        );

        tasks.push(Task {
            id,
            kind: TaskKind::AddFunction {
                module_name: module_name.clone(),
                description: spec.acceptance_criteria.join("; "),
            },
            description,
            status: TaskStatus::Pending,
        });
        id += 1;
    }

    // Phase 3: Modify main — always last
    let all_modules: Vec<&str> = spec
        .modules
        .create
        .iter()
        .chain(spec.modules.modify.iter())
        .map(|s| s.as_str())
        .collect();

    let test_summary = spec
        .test_cases
        .iter()
        .map(|tc| {
            format!(
                "{}({}) = {}",
                tc.function,
                tc.args
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                tc.expected_return
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let main_desc = if test_summary.is_empty() {
        format!(
            "Update the main function to demonstrate the implementation. Modules: {}",
            all_modules.join(", ")
        )
    } else {
        format!(
            "Update the main function to call and demonstrate: {}. The binary must exit with the result of the first call.",
            test_summary
        )
    };

    tasks.push(Task {
        id,
        kind: TaskKind::ModifyMain {
            description: main_desc.clone(),
        },
        description: main_desc,
        status: TaskStatus::Pending,
    });

    tasks
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};

    fn sample_spec() -> IntentSpec {
        IntentSpec {
            intent: "Build a calculator".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec![
                "add(a, b) returns a + b".to_string(),
                "sub(a, b) returns a - b".to_string(),
            ],
            modules: IntentModules {
                create: vec!["calculator/ops".to_string()],
                modify: vec!["app/main".to_string()],
            },
            test_cases: vec![TestCase {
                name: "addition".to_string(),
                function: "add".to_string(),
                args: vec![3, 5],
                expected_return: 8,
            }],
            dependencies: vec![],
            created_at: None,
            execution: None,
        }
    }

    #[test]
    fn decompose_generates_create_and_main_tasks() {
        let spec = sample_spec();
        let tasks = decompose(&spec);
        // Should have: 1 CreateModule + 1 ModifyMain (app/main is skipped in phase 2)
        assert!(tasks.len() >= 2, "must generate at least 2 tasks");
    }

    #[test]
    fn first_task_is_create_module() {
        let spec = sample_spec();
        let tasks = decompose(&spec);
        assert!(
            matches!(&tasks[0].kind, TaskKind::CreateModule { module_name } if module_name == "calculator/ops"),
            "first task must create calculator/ops"
        );
    }

    #[test]
    fn last_task_is_modify_main() {
        let spec = sample_spec();
        let tasks = decompose(&spec);
        assert!(
            matches!(&tasks.last().unwrap().kind, TaskKind::ModifyMain { .. }),
            "last task must modify main"
        );
    }

    #[test]
    fn tasks_have_sequential_ids() {
        let spec = sample_spec();
        let tasks = decompose(&spec);
        for (i, task) in tasks.iter().enumerate() {
            assert_eq!(task.id, i + 1, "task IDs must be sequential 1-based");
        }
    }

    #[test]
    fn tasks_contain_acceptance_criteria() {
        let spec = sample_spec();
        let tasks = decompose(&spec);
        let create_task = tasks
            .iter()
            .find(|t| matches!(&t.kind, TaskKind::CreateModule { .. }))
            .expect("must have create task");
        assert!(
            create_task.description.contains("add(a, b) returns a + b"),
            "create task must reference acceptance criteria"
        );
    }

    #[test]
    fn empty_spec_generates_only_modify_main() {
        let spec = IntentSpec {
            intent: "Minimal".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec![],
            modules: IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            created_at: None,
            execution: None,
        };
        let tasks = decompose(&spec);
        assert_eq!(tasks.len(), 1, "only ModifyMain for empty spec");
        assert!(matches!(tasks[0].kind, TaskKind::ModifyMain { .. }));
    }
}
