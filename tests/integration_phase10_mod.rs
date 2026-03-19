//! Integration tests for Phase 10 Track D: Intelligent Modularization.
//!
//! Tests module mapping decisions, duplicate detection, and boundary heuristics.

use std::collections::HashMap;

use duumbi::context::analyzer::{FunctionSummary, ModuleInfo, ProjectMap};
use duumbi::context::modularizer::{
    ModuleSuggestion, check_module_size, find_duplicate, suggest_module,
};

fn single_main() -> ProjectMap {
    ProjectMap {
        modules: vec![ModuleInfo {
            name: "main".to_string(),
            functions: vec![FunctionSummary {
                name: "main".to_string(),
                params: Vec::new(),
                return_type: "i64".to_string(),
            }],
            is_main: true,
        }],
        exports: HashMap::new(),
    }
}

fn multi_module() -> ProjectMap {
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
                name: "math".to_string(),
                functions: vec![
                    FunctionSummary {
                        name: "add".to_string(),
                        params: vec![
                            ("a".to_string(), "i64".to_string()),
                            ("b".to_string(), "i64".to_string()),
                        ],
                        return_type: "i64".to_string(),
                    },
                    FunctionSummary {
                        name: "subtract".to_string(),
                        params: vec![
                            ("a".to_string(), "i64".to_string()),
                            ("b".to_string(), "i64".to_string()),
                        ],
                        return_type: "i64".to_string(),
                    },
                ],
                is_main: false,
            },
        ],
        exports: [
            ("add".to_string(), "math".to_string()),
            ("subtract".to_string(), "math".to_string()),
        ]
        .into_iter()
        .collect(),
    }
}

// ---------------------------------------------------------------------------
// Module mapping decisions
// ---------------------------------------------------------------------------

#[test]
fn suggest_existing_function_maps_to_module() {
    let map = multi_module();
    assert_eq!(
        suggest_module("modify add", Some("add"), &map),
        ModuleSuggestion::Existing("math".to_string())
    );
}

#[test]
fn suggest_mentioned_module_name() {
    let map = multi_module();
    assert_eq!(
        suggest_module("add multiply to math", None, &map),
        ModuleSuggestion::Existing("math".to_string())
    );
}

#[test]
fn suggest_default_to_main_single_module() {
    let map = single_main();
    assert_eq!(
        suggest_module("add multiply", None, &map),
        ModuleSuggestion::Existing("main".to_string())
    );
}

#[test]
fn suggest_clarification_multi_module_unclear() {
    let map = multi_module();
    let result = suggest_module("add a new function", None, &map);
    assert!(matches!(result, ModuleSuggestion::NeedsClarification(_)));
}

// ---------------------------------------------------------------------------
// Duplicate detection
// ---------------------------------------------------------------------------

#[test]
fn detect_duplicate_function() {
    let map = multi_module();
    assert_eq!(find_duplicate("add", &map), Some("math".to_string()));
    assert_eq!(find_duplicate("subtract", &map), Some("math".to_string()));
}

#[test]
fn no_duplicate_new_function() {
    let map = multi_module();
    assert_eq!(find_duplicate("multiply", &map), None);
    assert_eq!(find_duplicate("divide", &map), None);
}

// ---------------------------------------------------------------------------
// Boundary heuristics
// ---------------------------------------------------------------------------

#[test]
fn module_size_small_ok() {
    let map = multi_module();
    assert!(check_module_size("math", &map).is_none());
}

#[test]
fn module_size_large_suggests_split() {
    let mut map = multi_module();
    let math = map
        .modules
        .iter_mut()
        .find(|m| m.name == "math")
        .expect("math");
    for i in 0..10 {
        math.functions.push(FunctionSummary {
            name: format!("func_{i}"),
            params: Vec::new(),
            return_type: "i64".to_string(),
        });
    }
    let suggestion = check_module_size("math", &map);
    assert!(suggestion.is_some());
    assert!(suggestion.expect("msg").contains("consider splitting"));
}

#[test]
fn module_size_nonexistent_module() {
    let map = multi_module();
    assert!(check_module_size("nonexistent", &map).is_none());
}

// ---------------------------------------------------------------------------
// Empty workspace handling
// ---------------------------------------------------------------------------

#[test]
fn suggest_module_empty_workspace() {
    let map = ProjectMap {
        modules: Vec::new(),
        exports: HashMap::new(),
    };
    // No modules → defaults to main
    let result = suggest_module("add function", None, &map);
    assert_eq!(result, ModuleSuggestion::Existing("main".to_string()));
}
