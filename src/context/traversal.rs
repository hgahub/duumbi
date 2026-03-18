//! Per-TaskType traversal strategy for context collection.
//!
//! Each [`TaskType`] has a deterministic traversal plan that specifies
//! which graph nodes to include in the LLM context, in priority order.

use crate::context::analyzer::ProjectMap;
use crate::context::classifier::TaskType;

/// A step in a traversal plan, defining what to collect and at what priority.
#[derive(Debug, Clone)]
pub struct TraversalStep {
    /// What category of nodes to collect.
    pub kind: StepKind,

    /// Priority (lower = more important, included first in budget).
    pub priority: u32,

    /// Whether to include full node bodies or just signatures.
    pub signatures_only: bool,
}

/// The kind of traversal step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Include all module names and their exported function signatures.
    AllModuleSignatures,

    /// Include the full content of a specific module.
    FullModule(String),

    /// Include the full content of the main module.
    MainModule,

    /// Include function signatures from stdlib/cache/vendor modules.
    DependencySignatures,

    /// Include nodes within N hops of a target node.
    Neighborhood {
        /// Target node pattern (module or function name).
        target: String,
        /// Number of hops to traverse.
        hops: u32,
    },

    /// Include recent error context (past error codes and fixes).
    ErrorContext,
}

/// An ordered traversal plan for context collection.
#[derive(Debug, Clone)]
pub struct TraversalPlan {
    /// Ordered steps (by priority — first step is highest priority).
    pub steps: Vec<TraversalStep>,
}

/// Builds a traversal plan for the given task type and request.
///
/// The plan is deterministic for the same inputs.
#[must_use]
pub fn build_plan(task_type: &TaskType, request: &str, project_map: &ProjectMap) -> TraversalPlan {
    let steps = match task_type {
        TaskType::CreateModule => vec![
            TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 1,
                signatures_only: true,
            },
            TraversalStep {
                kind: StepKind::DependencySignatures,
                priority: 3,
                signatures_only: true,
            },
        ],

        TaskType::AddFunction => {
            let mut steps = vec![TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 1,
                signatures_only: true,
            }];
            // If we can guess the target module, include it fully
            if let Some(module_name) = guess_target_module(request, project_map) {
                steps.push(TraversalStep {
                    kind: StepKind::FullModule(module_name),
                    priority: 2,
                    signatures_only: false,
                });
            }
            steps.push(TraversalStep {
                kind: StepKind::DependencySignatures,
                priority: 4,
                signatures_only: true,
            });
            steps
        }

        TaskType::ModifyFunction => {
            let mut steps = Vec::new();
            // Target function's module gets full context
            if let Some(module_name) = guess_target_module(request, project_map) {
                steps.push(TraversalStep {
                    kind: StepKind::FullModule(module_name.clone()),
                    priority: 1,
                    signatures_only: false,
                });
                // 1-hop neighborhood
                steps.push(TraversalStep {
                    kind: StepKind::Neighborhood {
                        target: module_name,
                        hops: 1,
                    },
                    priority: 2,
                    signatures_only: true,
                });
            }
            steps.push(TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 3,
                signatures_only: true,
            });
            steps
        }

        TaskType::ModifyMain => vec![
            TraversalStep {
                kind: StepKind::MainModule,
                priority: 1,
                signatures_only: false,
            },
            TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 2,
                signatures_only: true,
            },
        ],

        TaskType::FixError => {
            let mut steps = vec![TraversalStep {
                kind: StepKind::ErrorContext,
                priority: 1,
                signatures_only: false,
            }];
            if let Some(module_name) = guess_target_module(request, project_map) {
                steps.push(TraversalStep {
                    kind: StepKind::FullModule(module_name.clone()),
                    priority: 2,
                    signatures_only: false,
                });
                steps.push(TraversalStep {
                    kind: StepKind::Neighborhood {
                        target: module_name,
                        hops: 2,
                    },
                    priority: 3,
                    signatures_only: true,
                });
            }
            steps.push(TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 4,
                signatures_only: true,
            });
            steps
        }

        TaskType::RefactorModule => {
            let mut steps = vec![TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 2,
                signatures_only: true,
            }];
            if let Some(module_name) = guess_target_module(request, project_map) {
                steps.insert(
                    0,
                    TraversalStep {
                        kind: StepKind::FullModule(module_name),
                        priority: 1,
                        signatures_only: false,
                    },
                );
            }
            steps
        }

        TaskType::AddTest => vec![
            TraversalStep {
                kind: StepKind::AllModuleSignatures,
                priority: 1,
                signatures_only: true,
            },
            TraversalStep {
                kind: StepKind::MainModule,
                priority: 2,
                signatures_only: false,
            },
        ],
    };

    TraversalPlan { steps }
}

/// Guesses the target module name from the request text and project map.
fn guess_target_module(request: &str, project_map: &ProjectMap) -> Option<String> {
    let lower = request.to_lowercase();

    // Check if any module name is mentioned
    for module in &project_map.modules {
        if lower.contains(&module.name.to_lowercase()) {
            return Some(module.name.clone());
        }
    }

    // Check if any function name is mentioned → return its module
    for module in &project_map.modules {
        for func in &module.functions {
            if lower.contains(&func.name.to_lowercase()) {
                return Some(module.name.clone());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::analyzer::{FunctionSummary, ModuleInfo};
    use std::collections::HashMap;

    fn sample_map() -> ProjectMap {
        ProjectMap {
            modules: vec![
                ModuleInfo {
                    name: "main".to_string(),
                    functions: vec![FunctionSummary {
                        name: "main".to_string(),
                        params: Vec::new(),
                        return_type: "i64".to_string(),
                    }],
                    is_main: true,
                },
                ModuleInfo {
                    name: "ops".to_string(),
                    functions: vec![FunctionSummary {
                        name: "add".to_string(),
                        params: vec![
                            ("a".to_string(), "i64".to_string()),
                            ("b".to_string(), "i64".to_string()),
                        ],
                        return_type: "i64".to_string(),
                    }],
                    is_main: false,
                },
            ],
            exports: HashMap::new(),
        }
    }

    #[test]
    fn create_module_plan_has_signatures() {
        let plan = build_plan(&TaskType::CreateModule, "create math module", &sample_map());
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(s.kind, StepKind::AllModuleSignatures))
        );
        assert!(plan.steps.iter().all(|s| s.signatures_only));
    }

    #[test]
    fn add_function_plan_includes_target_module() {
        let plan = build_plan(&TaskType::AddFunction, "add subtract to ops", &sample_map());
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(&s.kind, StepKind::FullModule(name) if name == "ops"))
        );
    }

    #[test]
    fn modify_main_plan() {
        let plan = build_plan(&TaskType::ModifyMain, "call add from main", &sample_map());
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(s.kind, StepKind::MainModule))
        );
    }

    #[test]
    fn fix_error_plan_has_error_context() {
        let plan = build_plan(&TaskType::FixError, "fix E001 in add", &sample_map());
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(s.kind, StepKind::ErrorContext))
        );
    }

    #[test]
    fn guess_target_module_by_function() {
        let map = sample_map();
        assert_eq!(
            guess_target_module("modify add", &map),
            Some("ops".to_string())
        );
    }

    #[test]
    fn guess_target_module_by_name() {
        let map = sample_map();
        assert_eq!(
            guess_target_module("change ops module", &map),
            Some("ops".to_string())
        );
    }

    #[test]
    fn guess_target_module_none() {
        let map = sample_map();
        assert_eq!(guess_target_module("do something random", &map), None);
    }
}
