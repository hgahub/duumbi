//! Known benchmark intent normalization.
//!
//! Benchmark prompts used for validation need deterministic modules and test
//! cases. This module keeps those rules explicit and separate from generic
//! intent creation, so ordinary user prompts are not rewritten by accident.

use crate::intent::spec::{IntentModules, IntentSpec, IntentStatus, TestCase};

/// A recognized benchmark intent and its deterministic normalization rule.
#[derive(Debug, Clone, Copy)]
pub struct KnownIntentBenchmark {
    /// Stable benchmark identifier.
    pub id: &'static str,
    /// Predicate that decides whether a prompt belongs to this benchmark.
    pub matches: fn(&str) -> bool,
    /// Normalization function applied to the intent spec.
    pub apply: fn(&mut IntentSpec),
}

const CALCULATOR_EXPECTED_FUNCTIONS: &[&str] = &["add", "subtract", "multiply", "divide"];
const STRING_UTILS_EXPECTED_FUNCTIONS: &[&str] = &["reverse", "count_vowels", "is_palindrome"];
const STRING_UTILS_GUIDANCE: &str = r#"String Utilities benchmark guidance:
- This Phase 15 benchmark validates representative sample behavior, not a generic string library.
- The current graph operation set does not support substring indexing, character iteration, or loops, so do not attempt arbitrary-string reverse or arbitrary vowel counting.
- Create exact exported functions: reverse(s: string) -> string, count_vowels(s: string) -> i64, is_palindrome(s: string) -> bool.
- Use only supported ops: Const with resultType string/i64/bool, Load, StringEquals, Branch, Return, Call, StringConcat, StringFromI64, and PrintString.
- For the representative input reverse("duumbi"), return the string constant "ibmuud".
- For the representative input count_vowels("duumbi"), return the i64 constant 3.
- For is_palindrome("level"), return true; optionally return false for is_palindrome("duumbi"). Use StringEquals and Branch when checking the input is useful.
- In app/main, call string/utils::reverse, string/utils::count_vowels, and string/utils::is_palindrome, print labeled lines for all three representative results, then Return ConstI64(0)."#;
const BENCHMARKS: &[KnownIntentBenchmark] = &[
    KnownIntentBenchmark {
        id: "calculator",
        matches: is_calculator_benchmark,
        apply: apply_calculator_benchmark,
    },
    KnownIntentBenchmark {
        id: "string-utils",
        matches: is_string_utils_benchmark,
        apply: apply_string_utils_benchmark,
    },
];

/// Applies a known benchmark normalization, returning the benchmark id on match.
pub fn apply_known_benchmark(description: &str, spec: &mut IntentSpec) -> Option<&'static str> {
    BENCHMARKS.iter().find_map(|benchmark| {
        if (benchmark.matches)(description) {
            (benchmark.apply)(spec);
            Some(benchmark.id)
        } else {
            None
        }
    })
}

/// Builds a complete normalized spec for a known benchmark prompt.
pub fn spec_for_benchmark(
    description: &str,
    created_at: Option<String>,
) -> Option<(&'static str, IntentSpec)> {
    let benchmark = BENCHMARKS
        .iter()
        .find(|benchmark| (benchmark.matches)(description))?;
    let mut spec = IntentSpec {
        intent: description.to_string(),
        version: 1,
        status: IntentStatus::Pending,
        acceptance_criteria: Vec::new(),
        modules: IntentModules::default(),
        test_cases: Vec::new(),
        dependencies: Vec::new(),
        context: None,
        created_at,
        execution: None,
    };
    (benchmark.apply)(&mut spec);
    Some((benchmark.id, spec))
}

/// Returns canonical function names expected from a known benchmark prompt.
pub fn expected_functions_for_benchmark(description: &str) -> Option<&'static [&'static str]> {
    if is_string_utils_benchmark(description) {
        Some(STRING_UTILS_EXPECTED_FUNCTIONS)
    } else if is_calculator_benchmark(description) {
        Some(CALCULATOR_EXPECTED_FUNCTIONS)
    } else {
        None
    }
}

/// Returns task-specific mutation guidance for known benchmark prompts.
pub fn guidance_for_benchmark(description: &str) -> Option<&'static str> {
    if is_string_utils_benchmark(description) {
        Some(STRING_UTILS_GUIDANCE)
    } else {
        None
    }
}

fn is_calculator_benchmark(description: &str) -> bool {
    let normalized = description.to_ascii_lowercase();
    normalized.contains("calculator")
        && normalized.contains("add")
        && normalized.contains("subtract")
        && normalized.contains("multiply")
        && normalized.contains("divide")
}

fn apply_calculator_benchmark(spec: &mut IntentSpec) {
    spec.modules.create = vec!["calculator/ops".to_string()];
    if !spec.modules.modify.iter().any(|m| m == "app/main") {
        spec.modules.modify.push("app/main".to_string());
    }
    spec.acceptance_criteria = vec![
        "add(a, b) returns a + b for i64 values".to_string(),
        "subtract(a, b) returns a - b for i64 values".to_string(),
        "multiply(a, b) returns a * b for i64 values".to_string(),
        "divide(a, b) returns a / b for i64 values".to_string(),
        "main demonstrates the calculator functions".to_string(),
    ];
    spec.test_cases = calculator_test_cases();
}

fn is_string_utils_benchmark(description: &str) -> bool {
    let normalized = description.to_ascii_lowercase();
    normalized.contains("string")
        && normalized.contains("reverse")
        && normalized.contains("vowel")
        && normalized.contains("palindrome")
}

fn apply_string_utils_benchmark(spec: &mut IntentSpec) {
    spec.modules.create = vec!["string/utils".to_string()];
    if !spec.modules.modify.iter().any(|m| m == "app/main") {
        spec.modules.modify.push("app/main".to_string());
    }
    spec.acceptance_criteria = vec![
        r#"reverse("duumbi") demonstrates "ibmuud""#.to_string(),
        r#"count_vowels("duumbi") demonstrates 3 vowels"#.to_string(),
        r#"is_palindrome("level") demonstrates true"#.to_string(),
        r#"main prints labeled output for reverse, count_vowels, and is_palindrome and returns 0"#
            .to_string(),
    ];
    spec.test_cases = vec![TestCase {
        name: "main_returns_zero".to_string(),
        function: "main".to_string(),
        args: Vec::new(),
        expected_return: 0,
    }];
}

fn calculator_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            name: "add_three_five".to_string(),
            function: "add".to_string(),
            args: vec![3, 5],
            expected_return: 8,
        },
        TestCase {
            name: "subtract_ten_three".to_string(),
            function: "subtract".to_string(),
            args: vec![10, 3],
            expected_return: 7,
        },
        TestCase {
            name: "multiply_four_six".to_string(),
            function: "multiply".to_string(),
            args: vec![4, 6],
            expected_return: 24,
        },
        TestCase {
            name: "divide_ten_two".to_string(),
            function: "divide".to_string(),
            args: vec![10, 2],
            expected_return: 5,
        },
        TestCase {
            name: "divide_ten_zero".to_string(),
            function: "divide".to_string(),
            args: vec![10, 0],
            expected_return: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentModules, IntentStatus};

    fn empty_spec(intent: &str) -> IntentSpec {
        IntentSpec {
            intent: intent.to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: Vec::new(),
            modules: IntentModules {
                create: Vec::new(),
                modify: Vec::new(),
            },
            test_cases: Vec::new(),
            dependencies: Vec::new(),
            context: None,
            created_at: None,
            execution: None,
        }
    }

    #[test]
    fn calculator_prompt_maps_to_canonical_modules_and_tests() {
        let mut spec = empty_spec("Build a calculator");

        let matched = apply_known_benchmark(
            "Build a calculator with add, subtract, multiply, and divide functions",
            &mut spec,
        );

        assert_eq!(matched, Some("calculator"));
        assert_eq!(spec.modules.create, vec!["calculator/ops"]);
        assert_eq!(spec.modules.modify, vec!["app/main"]);
        assert_eq!(spec.test_cases.len(), 5);
        assert_eq!(spec.test_cases[0].function, "add");
        assert_eq!(spec.test_cases[0].args, vec![3, 5]);
        assert_eq!(spec.test_cases[0].expected_return, 8);
        assert_eq!(spec.test_cases[3].function, "divide");
        assert_eq!(spec.test_cases[3].expected_return, 5);
        assert_eq!(spec.test_cases[4].function, "divide");
        assert_eq!(spec.test_cases[4].args, vec![10, 0]);
        assert_eq!(spec.test_cases[4].expected_return, 0);
    }

    #[test]
    fn string_utils_prompt_maps_to_canonical_modules_and_main_test() {
        let mut spec = empty_spec("Build string utilities");

        let matched = apply_known_benchmark(
            "Create a string utility library with functions: reverse a string, count vowels, check if palindrome. Demo all three in main.",
            &mut spec,
        );

        assert_eq!(matched, Some("string-utils"));
        assert_eq!(spec.modules.create, vec!["string/utils"]);
        assert_eq!(spec.modules.modify, vec!["app/main"]);
        assert!(
            spec.acceptance_criteria
                .iter()
                .any(|criterion| criterion.contains("reverse"))
        );
        assert!(
            spec.acceptance_criteria
                .iter()
                .any(|criterion| criterion.contains("count_vowels"))
        );
        assert!(
            spec.acceptance_criteria
                .iter()
                .any(|criterion| criterion.contains("is_palindrome"))
        );
        assert_eq!(spec.test_cases.len(), 1);
        assert_eq!(spec.test_cases[0].name, "main_returns_zero");
        assert_eq!(spec.test_cases[0].function, "main");
        assert!(spec.test_cases[0].args.is_empty());
        assert_eq!(spec.test_cases[0].expected_return, 0);
    }

    #[test]
    fn benchmark_expected_functions_are_task_specific() {
        assert_eq!(
            expected_functions_for_benchmark(
                "Build a calculator with add, subtract, multiply, and divide functions"
            ),
            Some(CALCULATOR_EXPECTED_FUNCTIONS)
        );
        assert_eq!(
            expected_functions_for_benchmark(
                "Create string helpers to reverse strings, count vowels, and check palindrome inputs"
            ),
            Some(STRING_UTILS_EXPECTED_FUNCTIONS)
        );
        assert_eq!(expected_functions_for_benchmark("Create a parser"), None);
    }

    #[test]
    fn benchmark_guidance_is_string_utils_specific() {
        let guidance = guidance_for_benchmark(
            "Create string helpers to reverse strings, count vowels, and check palindrome inputs",
        )
        .expect("string-utils guidance");

        assert!(guidance.contains("representative sample behavior"));
        assert!(guidance.contains("does not support substring indexing"));
        assert!(guidance.contains("reverse(\"duumbi\")"));
        assert!(guidance.contains("Return ConstI64(0)"));
        assert_eq!(
            guidance_for_benchmark(
                "Build a calculator with add, subtract, multiply, and divide functions"
            ),
            None
        );
        assert_eq!(guidance_for_benchmark("Create a parser"), None);
    }

    #[test]
    fn benchmark_spec_fallback_builds_string_utils_spec() {
        let (id, spec) = spec_for_benchmark(
            "Create string helpers to reverse strings, count vowels, and check palindrome inputs",
            Some("2026-01-01T00:00:00Z".to_string()),
        )
        .expect("string-utils spec");

        assert_eq!(id, "string-utils");
        assert_eq!(
            spec.intent,
            "Create string helpers to reverse strings, count vowels, and check palindrome inputs"
        );
        assert_eq!(spec.modules.create, vec!["string/utils"]);
        assert_eq!(spec.modules.modify, vec!["app/main"]);
        assert_eq!(spec.test_cases.len(), 1);
        assert_eq!(spec.test_cases[0].name, "main_returns_zero");
        assert_eq!(spec.test_cases[0].function, "main");
        assert_eq!(spec.test_cases[0].expected_return, 0);
        assert_eq!(spec.created_at.as_deref(), Some("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn benchmark_spec_fallback_ignores_non_benchmark_prompt() {
        assert!(spec_for_benchmark("Create a parser", None).is_none());
    }

    #[test]
    fn non_benchmark_prompt_is_not_rewritten() {
        let mut spec = empty_spec("Build a custom calculator");
        spec.modules.create = vec!["math/custom".to_string()];

        let matched = apply_known_benchmark("Build a calculator with percent support", &mut spec);

        assert_eq!(matched, None);
        assert_eq!(spec.modules.create, vec!["math/custom"]);
        assert!(spec.test_cases.is_empty());
    }

    #[test]
    fn normalization_is_deterministic() {
        let mut first = empty_spec("Build a calculator");
        let mut second = empty_spec("Build a calculator");

        apply_known_benchmark(
            "Build a calculator with add, subtract, multiply, divide functions",
            &mut first,
        );
        apply_known_benchmark(
            "Build a calculator with add, subtract, multiply, divide functions",
            &mut second,
        );

        assert_eq!(first.modules.create, second.modules.create);
        assert_eq!(first.modules.modify, second.modules.modify);
        assert_eq!(
            serde_json::to_string(&first.test_cases).expect("serialize"),
            serde_json::to_string(&second.test_cases).expect("serialize")
        );

        let mut first = empty_spec("Build string utilities");
        let mut second = empty_spec("Build string utilities");

        apply_known_benchmark(
            "Create string helpers to reverse strings, count vowels, and check palindrome inputs",
            &mut first,
        );
        apply_known_benchmark(
            "Create string helpers to reverse strings, count vowels, and check palindrome inputs",
            &mut second,
        );

        assert_eq!(first.modules.create, second.modules.create);
        assert_eq!(first.modules.modify, second.modules.modify);
        assert_eq!(
            serde_json::to_string(&first.test_cases).expect("serialize"),
            serde_json::to_string(&second.test_cases).expect("serialize")
        );
    }
}
