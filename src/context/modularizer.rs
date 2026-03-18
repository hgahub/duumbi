//! Intelligent modularization — task-to-module mapping and boundary detection.
//!
//! Determines where new functions should be placed, detects module
//! boundaries, and prevents duplicate function definitions.

use crate::context::analyzer::ProjectMap;

/// Suggested module placement for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleSuggestion {
    /// Place in an existing module.
    Existing(String),
    /// Create a new module with the suggested name.
    New(String),
    /// Ambiguous — ask the user for clarification.
    NeedsClarification(String),
}

/// Suggests which module a new function should be placed in.
///
/// Rules:
/// 1. If the function name already exists → return that module (ModifyFunction)
/// 2. If the request mentions an existing module → place there
/// 3. If related functions exist (common prefix) → suggest that module
/// 4. If the project has many modules → suggest creating a new one
/// 5. Otherwise → place in main
#[must_use]
pub fn suggest_module(
    request: &str,
    function_name: Option<&str>,
    project_map: &ProjectMap,
) -> ModuleSuggestion {
    let lower = request.to_lowercase();

    // Rule 1: Function already exists
    if let Some(fname) = function_name
        && let Some(module_name) = project_map.exports.get(fname)
    {
        return ModuleSuggestion::Existing(module_name.clone());
    }

    // Rule 2: Request mentions an existing module
    for module in &project_map.modules {
        if !module.is_main && lower.contains(&module.name.to_lowercase()) {
            return ModuleSuggestion::Existing(module.name.clone());
        }
    }

    // Rule 3: Common prefix with existing functions
    if let Some(fname) = function_name {
        for module in &project_map.modules {
            if !module.is_main {
                for func in &module.functions {
                    if has_common_prefix(fname, &func.name) {
                        return ModuleSuggestion::Existing(module.name.clone());
                    }
                }
            }
        }
    }

    // Rule 4: Extract module name from request keywords
    if let Some(module_name) = extract_module_hint(&lower) {
        // Check if this would be a new module
        let existing = project_map.modules.iter().any(|m| m.name == module_name);
        if existing {
            return ModuleSuggestion::Existing(module_name);
        }
        return ModuleSuggestion::New(module_name);
    }

    // Rule 5: Default — if only main exists, put it there
    if project_map.modules.len() <= 1 {
        return ModuleSuggestion::Existing("main".to_string());
    }

    // Multiple modules exist but no clear target — ask
    ModuleSuggestion::NeedsClarification(
        "Multiple modules exist. Which module should contain the new function?".to_string(),
    )
}

/// Checks if a function name already exists in any module.
///
/// Returns `Some(module_name)` if it does, `None` otherwise.
#[must_use]
pub fn find_duplicate(function_name: &str, project_map: &ProjectMap) -> Option<String> {
    project_map.exports.get(function_name).cloned()
}

/// Checks if a module should be split based on size heuristics.
///
/// Returns a suggestion message if the module has more than 5 functions.
#[must_use]
pub fn check_module_size(module_name: &str, project_map: &ProjectMap) -> Option<String> {
    project_map
        .modules
        .iter()
        .find(|m| m.name == module_name)
        .and_then(|m| {
            if m.functions.len() > 5 {
                Some(format!(
                    "Module '{}' has {} functions — consider splitting into submodules",
                    module_name,
                    m.functions.len()
                ))
            } else {
                None
            }
        })
}

/// Checks if two function names share a meaningful common prefix.
fn has_common_prefix(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // Find common prefix length
    let common: String = a_lower
        .chars()
        .zip(b_lower.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c)
        .collect();

    // At least 3 chars common prefix and not just a common short word
    common.len() >= 3 && common.len() < a.len() && common.len() < b.len()
}

/// Extracts a module name hint from request text.
fn extract_module_hint(request: &str) -> Option<String> {
    // Pattern: "... in/to/for <module> module"
    let patterns = ["in the ", "to the ", "for the ", "in ", "to ", "for "];
    for pattern in &patterns {
        if let Some(idx) = request.find(pattern) {
            let after = &request[idx + pattern.len()..];
            let word = after.split_whitespace().next()?;
            // Skip generic words
            if !["the", "a", "an", "this", "that", "main"].contains(&word) {
                return Some(word.to_string());
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

    fn single_module_map() -> ProjectMap {
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

    fn multi_module_map() -> ProjectMap {
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

    #[test]
    fn suggest_existing_function() {
        let map = multi_module_map();
        let result = suggest_module("modify add", Some("add"), &map);
        assert_eq!(result, ModuleSuggestion::Existing("math".to_string()));
    }

    #[test]
    fn suggest_mentioned_module() {
        let map = multi_module_map();
        let result = suggest_module("add multiply to math", None, &map);
        assert_eq!(result, ModuleSuggestion::Existing("math".to_string()));
    }

    #[test]
    fn suggest_default_to_main() {
        let map = single_module_map();
        let result = suggest_module("add multiply", None, &map);
        assert_eq!(result, ModuleSuggestion::Existing("main".to_string()));
    }

    #[test]
    fn suggest_needs_clarification() {
        let map = multi_module_map();
        let result = suggest_module("add a new function", None, &map);
        assert!(matches!(result, ModuleSuggestion::NeedsClarification(_)));
    }

    #[test]
    fn find_duplicate_exists() {
        let map = multi_module_map();
        assert_eq!(find_duplicate("add", &map), Some("math".to_string()));
    }

    #[test]
    fn find_duplicate_not_found() {
        let map = multi_module_map();
        assert_eq!(find_duplicate("multiply", &map), None);
    }

    #[test]
    fn check_module_size_small() {
        let map = multi_module_map();
        assert!(check_module_size("math", &map).is_none());
    }

    #[test]
    fn check_module_size_large() {
        let mut map = multi_module_map();
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
        assert!(check_module_size("math", &map).is_some());
    }

    #[test]
    fn has_common_prefix_works() {
        assert!(has_common_prefix("string_concat", "string_length"));
        assert!(!has_common_prefix("add", "subtract"));
        assert!(!has_common_prefix("ab", "abc")); // too short prefix
    }
}
