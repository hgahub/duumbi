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

    // Build a shared exports hint from all non-main test_case functions.
    // Used in both CreateModule and AddFunction tasks so the LLM is reminded
    // to populate duumbi:exports at every mutation step.
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

    // Phase 1: Create new modules
    for module_name in &spec.modules.create {
        let criteria_summary = spec
            .acceptance_criteria
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");

        let algo_hint = algorithmic_hint(&spec.intent, &spec.acceptance_criteria);

        let description = if criteria_summary.is_empty() {
            format!(
                "Create module '{module_name}' as described in the intent.{exports_hint}{algo_hint}"
            )
        } else {
            format!(
                "Create module '{module_name}'. Requirements: {criteria_summary}{exports_hint}{algo_hint}"
            )
        };

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

        let criteria = spec.acceptance_criteria.join("; ");
        let description = format!(
            "Modify module '{}' to satisfy the acceptance criteria: {}{exports_hint}",
            module_name, criteria,
        );

        tasks.push(Task {
            id,
            kind: TaskKind::AddFunction {
                module_name: module_name.clone(),
                description: format!("{criteria}{exports_hint}"),
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

    // Sanitize intent title for safe interpolation into prompt strings.
    // Remove control chars, escape quotes/backslashes to prevent prompt corruption.
    let intent_title: String = spec
        .intent
        .chars()
        .take(50)
        .filter(|c| !c.is_control())
        .map(|c| if c == '"' || c == '\\' { ' ' } else { c })
        .collect();

    let main_desc = if test_summary.is_empty() {
        format!(
            "Update the main function to demonstrate the implementation. Modules: {}",
            all_modules.join(", ")
        )
    } else {
        format!(
            "Update the main function to call and demonstrate: {test_summary}. \
             The binary must exit with the result of the first call.\n\n\
             OUTPUT FORMAT: The program MUST produce human-readable formatted output.\n\
             1. Print a header: ConstString(\"=== {intent_title} ===\") → PrintString\n\
             2. For EACH test case, print a labeled result line:\n\
                ConstString(\"func(a, b) = \") → StringFromI64(call_result) → StringConcat → PrintString\n\
             Use this exact pattern for each line:\n\
               op/0: ConstString(\"func(args) = \")\n\
               op/1: [Call the function → get i64 result]\n\
               op/2: StringFromI64(op/1)\n\
               op/3: StringConcat(op/0, op/2)\n\
               op/4: PrintString(op/3)\n\
             This makes the output readable like: \"gcd(48, 18) = 6\"",
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
// Algorithmic hints
// ---------------------------------------------------------------------------

/// Generates implementation hints based on the intent description and criteria.
///
/// Analyses keywords to suggest appropriate patterns (recursion, branching,
/// Euclidean algorithm, etc.) so the LLM knows HOW to implement, not just WHAT.
fn algorithmic_hint(intent: &str, criteria: &[String]) -> String {
    let combined = format!(
        "{} {}",
        intent.to_lowercase(),
        criteria.join(" ").to_lowercase()
    );

    let mut hints = Vec::new();

    // Recursive patterns
    if combined.contains("fibonacci") || combined.contains("fib(") {
        hints.push(
            "ALGORITHM: Use recursion. Base cases: fib(0)=0, fib(1)=1. \
             Recursive case: fib(n)=fib(n-1)+fib(n-2). \
             Implementation: entry block loads n, compares with 0 (Branch to base_zero), \
             then compares with 1 (Branch to base_one), else falls through to recurse block. \
             Each base block returns a Const. Recurse block: Sub(n,1)→Call fib, Sub(n,2)→Call fib, Add results→Return.",
        );
    } else if combined.contains("tribonacci") {
        hints.push(
            "ALGORITHM: Use recursion. Base cases: t(0)=0, t(1)=0, t(2)=1. \
             Recursive: t(n)=t(n-1)+t(n-2)+t(n-3). Use Compare+Branch for base cases.",
        );
    } else if combined.contains("factorial") {
        hints.push(
            "ALGORITHM: Use recursion. Base case: n<=1 returns 1. \
             Recursive: n * factorial(n-1). Entry block: Load n, Const 1, Compare le, \
             Branch to base/recurse. Base block: Const 1, Return. \
             Recurse block: Sub(n,1)→Call factorial→Mul(n, result)→Return.",
        );
    }

    // GCD / Euclidean
    if combined.contains("gcd") || combined.contains("greatest common divisor") {
        hints.push(
            "ALGORITHM: Euclidean algorithm via recursion. gcd(a,b): if b==0 return a, \
             else return gcd(b, a%b). Use Modulo op for a%b. \
             Entry block: Load a, Load b, Const 0, Compare(b, 0, eq), Branch to base/recurse. \
             Base block: Load a, Return. Recurse block: Load a, Load b, Modulo(a,b), \
             Load b→Call gcd(b, a%b)→Return.",
        );
    }

    // LCM
    if combined.contains("lcm") || combined.contains("least common multiple") {
        hints.push("ALGORITHM: lcm(a,b) = |a*b| / gcd(a,b). Implement gcd first, then use it.");
    }

    // Prime check
    if combined.contains("prime") || combined.contains("is_prime") {
        hints.push(
            "ALGORITHM: Trial division. Check if n<=1 (return 0). Check if n<=3 (return 1). \
             Check n%2==0 (return 0). Use recursive helper: try_divide(n, d) where d starts at 3. \
             If d*d > n return 1. If n%d==0 return 0. Else try_divide(n, d+2).",
        );
    }

    // Collatz
    if combined.contains("collatz") {
        hints.push(
            "ALGORITHM: Recursive step counter. collatz(n): if n==1 return 0. \
             If n%2==0, return 1 + collatz(n/2). Else return 1 + collatz(3*n+1). \
             Use Modulo(n,2) for even check, Branch for each case.",
        );
    }

    // Digital root
    if combined.contains("digital root") || combined.contains("digit sum") {
        hints.push(
            "ALGORITHM: For digit sum: use Modulo(n,10) to get last digit, Div(n,10) to remove it. \
             Recurse until n==0. For digital root: recurse digit_sum until result < 10.",
        );
    }

    // Absolute value
    if combined.contains("absolute") || combined.contains("abs(") {
        hints.push("ALGORITHM: Compare n >= 0. If true, return n. If false, return Sub(0, n).");
    }

    // Clamp
    if combined.contains("clamp") {
        hints.push(
            "ALGORITHM: clamp(x, lo, hi): Compare(x, lo, lt)→return lo. \
             Compare(x, hi, gt)→return hi. Else return x. Use 3 blocks with Branch.",
        );
    }

    // Power of 2
    if combined.contains("power of 2")
        || combined.contains("2^")
        || combined.contains("2 to the power")
    {
        hints.push(
            "ALGORITHM: Recursive. pow2(0)=1. pow2(n)=2*pow2(n-1). \
             Or use Mul(2, Call pow2(n-1)).",
        );
    }

    // General branching hint if no specific algorithm matched but criteria suggest conditions
    if hints.is_empty()
        && (combined.contains("if ")
            || combined.contains("when ")
            || combined.contains("returns 1") && combined.contains("returns 0"))
    {
        hints.push(
            "PATTERN: Use Compare+Branch for conditional logic. Entry block evaluates the condition, \
             branches to separate blocks for true/false outcomes. Each outcome block ends with Return.",
        );
    }

    if hints.is_empty() {
        String::new()
    } else {
        format!(" {}", hints.join(" "))
    }
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
            context: None,
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
    fn create_task_includes_exports_hint() {
        let spec = sample_spec();
        let tasks = decompose(&spec);
        let create_task = tasks
            .iter()
            .find(|t| matches!(&t.kind, TaskKind::CreateModule { .. }))
            .expect("must have create task");
        // The exports hint must list the function from test_cases (sorted, no "main")
        assert!(
            create_task.description.contains(
                "IMPORTANT: the duumbi:exports array MUST include ALL of these functions: [add]"
            ),
            "create task must include sorted exports hint; got: {}",
            create_task.description
        );
    }

    #[test]
    fn add_function_task_includes_exports_hint() {
        use crate::intent::spec::{IntentModules, TestCase};
        // Spec with a non-main module in `modify` so Phase 2 generates an AddFunction task.
        let spec = IntentSpec {
            intent: "Test".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec!["double(x) returns x * 2".to_string()],
            modules: IntentModules {
                create: vec![],
                modify: vec!["math/ops".to_string()],
            },
            test_cases: vec![TestCase {
                name: "t1".to_string(),
                function: "double".to_string(),
                args: vec![7],
                expected_return: 14,
            }],
            dependencies: vec![],
            context: None,
            created_at: None,
            execution: None,
        };
        let tasks = decompose(&spec);
        let add_task = tasks
            .iter()
            .find(|t| matches!(&t.kind, TaskKind::AddFunction { .. }))
            .expect("must have add_function task");
        assert!(
            add_task.description.contains(
                "IMPORTANT: the duumbi:exports array MUST include ALL of these functions: [double]"
            ),
            "add_function task must include exports hint; got: {}",
            add_task.description
        );
    }

    #[test]
    fn exports_hint_excludes_main_function() {
        use crate::intent::spec::{IntentModules, TestCase};
        let spec = IntentSpec {
            intent: "Test".to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec![],
            modules: IntentModules {
                create: vec!["some/module".to_string()],
                modify: vec![],
            },
            test_cases: vec![
                TestCase {
                    name: "t1".to_string(),
                    function: "main".to_string(),
                    args: vec![],
                    expected_return: 0,
                },
                TestCase {
                    name: "t2".to_string(),
                    function: "helper".to_string(),
                    args: vec![],
                    expected_return: 1,
                },
            ],
            dependencies: vec![],
            context: None,
            created_at: None,
            execution: None,
        };
        let tasks = decompose(&spec);
        let create_task = tasks
            .iter()
            .find(|t| matches!(&t.kind, TaskKind::CreateModule { .. }))
            .expect("must have create task");
        assert!(
            create_task.description.contains("[helper]"),
            "exports hint must list helper but not main; got: {}",
            create_task.description
        );
        assert!(
            !create_task.description.contains("main"),
            "exports hint must not include 'main'; got: {}",
            create_task.description
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
            context: None,
            created_at: None,
            execution: None,
        };
        let tasks = decompose(&spec);
        assert_eq!(tasks.len(), 1, "only ModifyMain for empty spec");
        assert!(matches!(tasks[0].kind, TaskKind::ModifyMain { .. }));
    }
}
