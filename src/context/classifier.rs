//! Task classifier for LLM mutation requests.
//!
//! Classifies a natural-language request into one of 7 [`TaskType`] variants
//! using rule-based keyword matching and project structure analysis.

use crate::context::analyzer::ProjectMap;

/// The classified task type for a mutation request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// Create a new module (new `.jsonld` file).
    CreateModule,
    /// Add a new function to an existing module.
    AddFunction,
    /// Modify an existing function's implementation.
    ModifyFunction,
    /// Modify the main function or main module's wiring.
    ModifyMain,
    /// Fix a compilation or validation error.
    FixError,
    /// Refactor or restructure a module without changing behavior.
    RefactorModule,
    /// Add a test case.
    AddTest,
}

impl TaskType {
    /// Returns the string name of this task type.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::CreateModule => "CreateModule",
            TaskType::AddFunction => "AddFunction",
            TaskType::ModifyFunction => "ModifyFunction",
            TaskType::ModifyMain => "ModifyMain",
            TaskType::FixError => "FixError",
            TaskType::RefactorModule => "RefactorModule",
            TaskType::AddTest => "AddTest",
        }
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Classifies a user request into a [`TaskType`].
///
/// Uses rule-based keyword matching combined with project structure analysis:
/// - Error-related keywords → [`FixError`](TaskType::FixError)
/// - Module creation keywords → [`CreateModule`](TaskType::CreateModule)
/// - Function addition keywords → [`AddFunction`](TaskType::AddFunction)
/// - Existing function names in request → [`ModifyFunction`](TaskType::ModifyFunction)
/// - Main-related keywords → [`ModifyMain`](TaskType::ModifyMain)
/// - Refactor keywords → [`RefactorModule`](TaskType::RefactorModule)
/// - Test keywords → [`AddTest`](TaskType::AddTest)
/// - Default → [`AddFunction`](TaskType::AddFunction)
#[must_use]
pub fn classify(request: &str, project_map: &ProjectMap) -> TaskType {
    let lower = request.to_lowercase();

    // Priority 1: Error fixing
    if contains_any(
        &lower,
        &[
            "fix", "error", "bug", "broken", "e001", "e002", "e003", "e004", "e005", "e006",
            "e007", "e008", "e009", "e010",
        ],
    ) {
        return TaskType::FixError;
    }

    // Priority 2: Test addition
    if contains_any(&lower, &["test", "verify", "assert", "check"])
        && contains_any(&lower, &["add", "create", "write", "new"])
    {
        return TaskType::AddTest;
    }

    // Priority 3: Refactoring
    if contains_any(
        &lower,
        &[
            "refactor",
            "restructure",
            "reorganize",
            "split",
            "extract",
            "rename",
        ],
    ) {
        return TaskType::RefactorModule;
    }

    // Priority 4: Module creation
    if contains_any(
        &lower,
        &[
            "create module",
            "new module",
            "add module",
            "create a module",
        ],
    ) {
        return TaskType::CreateModule;
    }

    // Priority 5: Check if request mentions an existing function → modify
    for module in &project_map.modules {
        for func in &module.functions {
            if lower.contains(&func.name.to_lowercase()) {
                if module.is_main && func.name == "main" {
                    return TaskType::ModifyMain;
                }
                return TaskType::ModifyFunction;
            }
        }
    }

    // Priority 6: Main modification
    if contains_any(
        &lower,
        &[
            "modify main",
            "change main",
            "update main",
            "wire",
            "connect",
            "hook up",
            "call from main",
        ],
    ) {
        return TaskType::ModifyMain;
    }

    // Priority 7: Module creation from "create <name>" pattern
    if lower.starts_with("create ") && !contains_any(&lower, &["function", "func"]) {
        return TaskType::CreateModule;
    }

    // Default: AddFunction
    TaskType::AddFunction
}

/// Returns `true` if any of the given patterns are found in the input.
fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| input.contains(p))
}

/// Returns `true` if the classifier detects ambiguity in the request.
///
/// Ambiguity triggers a clarification question instead of proceeding.
#[must_use]
pub fn is_ambiguous(request: &str, project_map: &ProjectMap) -> bool {
    let lower = request.to_lowercase();

    // Very short requests are ambiguous
    if lower.split_whitespace().count() <= 2 {
        return true;
    }

    // Request mentions multiple modules
    let module_mentions: usize = project_map
        .modules
        .iter()
        .filter(|m| lower.contains(&m.name.to_lowercase()))
        .count();
    if module_mentions > 1 {
        return true;
    }

    // Request is vague ("do something", "change it", "make it work")
    if contains_any(&lower, &["something", "somehow", "whatever", "it"])
        && lower.split_whitespace().count() <= 4
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::analyzer::{FunctionSummary, ModuleInfo};

    fn empty_map() -> ProjectMap {
        ProjectMap {
            modules: Vec::new(),
            exports: std::collections::HashMap::new(),
        }
    }

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
                    name: "calculator/ops".to_string(),
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
            exports: [("add".to_string(), "calculator/ops".to_string())]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn classify_fix_error() {
        assert_eq!(
            classify("fix the E001 error", &empty_map()),
            TaskType::FixError
        );
        assert_eq!(
            classify("there is a bug in add", &empty_map()),
            TaskType::FixError
        );
        assert_eq!(
            classify("fix broken function", &empty_map()),
            TaskType::FixError
        );
    }

    #[test]
    fn classify_add_function() {
        assert_eq!(
            classify("add multiply function", &empty_map()),
            TaskType::AddFunction
        );
        assert_eq!(
            classify("implement a fibonacci function", &empty_map()),
            TaskType::AddFunction
        );
    }

    #[test]
    fn classify_modify_existing_function() {
        let map = sample_map();
        assert_eq!(
            classify("modify the add function to handle f64", &map),
            TaskType::ModifyFunction
        );
    }

    #[test]
    fn classify_modify_main() {
        let map = sample_map();
        assert_eq!(classify("call add from main", &map), TaskType::ModifyMain);
        assert_eq!(
            classify("update main to use new function", &empty_map()),
            TaskType::ModifyMain
        );
    }

    #[test]
    fn classify_create_module() {
        assert_eq!(
            classify("create module calculator", &empty_map()),
            TaskType::CreateModule
        );
        assert_eq!(
            classify("new module for string ops", &empty_map()),
            TaskType::CreateModule
        );
    }

    #[test]
    fn classify_refactor() {
        assert_eq!(
            classify("refactor the math module", &empty_map()),
            TaskType::RefactorModule
        );
        assert_eq!(
            classify("split the large module", &empty_map()),
            TaskType::RefactorModule
        );
    }

    #[test]
    fn classify_add_test() {
        assert_eq!(
            classify("add test for multiply", &empty_map()),
            TaskType::AddTest
        );
        assert_eq!(
            classify("create a test case for division", &empty_map()),
            TaskType::AddTest
        );
    }

    #[test]
    fn classify_default_is_add_function() {
        assert_eq!(
            classify("implement abs", &empty_map()),
            TaskType::AddFunction
        );
    }

    #[test]
    fn task_type_display() {
        assert_eq!(TaskType::AddFunction.to_string(), "AddFunction");
        assert_eq!(TaskType::CreateModule.as_str(), "CreateModule");
    }

    #[test]
    fn ambiguity_short_request() {
        assert!(is_ambiguous("do something", &empty_map()));
        assert!(is_ambiguous("add", &empty_map()));
    }

    #[test]
    fn ambiguity_multiple_modules() {
        let map = ProjectMap {
            modules: vec![
                ModuleInfo {
                    name: "math".to_string(),
                    functions: Vec::new(),
                    is_main: false,
                },
                ModuleInfo {
                    name: "string".to_string(),
                    functions: Vec::new(),
                    is_main: false,
                },
            ],
            exports: std::collections::HashMap::new(),
        };
        assert!(is_ambiguous(
            "change something in math and string modules",
            &map
        ));
    }

    #[test]
    fn not_ambiguous_clear_request() {
        assert!(!is_ambiguous(
            "add a multiply function that takes two i64 parameters",
            &empty_map()
        ));
    }
}
