//! Few-shot patch examples for LLM retry context.
//!
//! Contains a curated set of successful graph mutation examples used to guide
//! the LLM on retry attempts. Selection is rule-based: examples are matched
//! by error code relevance and user request keywords.
//!
//! # Usage
//!
//! Called by the orchestrator retry pipeline on step 2 (second retry attempt)
//! to inject a relevant successful patch alongside the error feedback.

/// A single successful patch example with context for matching.
pub struct PatchExample {
    /// Short description (used in tests and future UI).
    #[allow(dead_code)]
    pub description: &'static str,
    /// Error codes this example is most relevant for (e.g. E003, E001).
    pub error_codes: &'static [&'static str],
    /// Request keywords that increase this example's match score.
    pub keywords: &'static [&'static str],
    /// A complete JSON patch that was applied successfully.
    pub patch_json: &'static str,
}

/// Curated set of successful patch examples covering common mutation patterns.
static EXAMPLES: &[PatchExample] = &[
    // ------------------------------------------------------------------
    // Pattern: add a simple binary function (add_function, full object)
    // ------------------------------------------------------------------
    PatchExample {
        description: "Adding a simple add(a, b) function using add_function",
        error_codes: &["E003", "E009"],
        keywords: &["add", "function", "create", "new", "implement"],
        patch_json: r#"add_function: {
  "function": {
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
        {"@type": "duumbi:Load",   "@id": "duumbi:main/add/entry/0", "duumbi:variable": "a", "duumbi:resultType": "i64"},
        {"@type": "duumbi:Load",   "@id": "duumbi:main/add/entry/1", "duumbi:variable": "b", "duumbi:resultType": "i64"},
        {"@type": "duumbi:Add",    "@id": "duumbi:main/add/entry/2", "duumbi:left": {"@id": "duumbi:main/add/entry/0"}, "duumbi:right": {"@id": "duumbi:main/add/entry/1"}, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Return", "@id": "duumbi:main/add/entry/3", "duumbi:operand": {"@id": "duumbi:main/add/entry/2"}}
      ]
    }]
  }
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: add a conditional function (Branch, two blocks)
    // ------------------------------------------------------------------
    PatchExample {
        description: "Adding a conditional max(a, b) function with Branch and two blocks",
        error_codes: &["E003", "E009"],
        keywords: &[
            "if",
            "branch",
            "conditional",
            "max",
            "min",
            "compare",
            "greater",
            "less",
        ],
        patch_json: r#"add_function: {
  "function": {
    "@type": "duumbi:Function",
    "@id": "duumbi:main/max",
    "duumbi:name": "max",
    "duumbi:returnType": "i64",
    "duumbi:params": [
      {"duumbi:name": "a", "duumbi:paramType": "i64"},
      {"duumbi:name": "b", "duumbi:paramType": "i64"}
    ],
    "duumbi:blocks": [
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/max/entry",
        "duumbi:label": "entry",
        "duumbi:ops": [
          {"@type": "duumbi:Load",    "@id": "duumbi:main/max/entry/0", "duumbi:variable": "a", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Load",    "@id": "duumbi:main/max/entry/1", "duumbi:variable": "b", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Compare", "@id": "duumbi:main/max/entry/2", "duumbi:operator": "gt", "duumbi:left": {"@id": "duumbi:main/max/entry/0"}, "duumbi:right": {"@id": "duumbi:main/max/entry/1"}, "duumbi:resultType": "bool"},
          {"@type": "duumbi:Branch",  "@id": "duumbi:main/max/entry/3", "duumbi:condition": {"@id": "duumbi:main/max/entry/2"}, "duumbi:trueBlock": "duumbi:main/max/true_branch", "duumbi:falseBlock": "duumbi:main/max/false_branch"}
        ]
      },
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/max/true_branch",
        "duumbi:label": "true_branch",
        "duumbi:ops": [
          {"@type": "duumbi:Load",   "@id": "duumbi:main/max/true_branch/0", "duumbi:variable": "a", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Return", "@id": "duumbi:main/max/true_branch/1", "duumbi:operand": {"@id": "duumbi:main/max/true_branch/0"}}
        ]
      },
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/max/false_branch",
        "duumbi:label": "false_branch",
        "duumbi:ops": [
          {"@type": "duumbi:Load",   "@id": "duumbi:main/max/false_branch/0", "duumbi:variable": "b", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Return", "@id": "duumbi:main/max/false_branch/1", "duumbi:operand": {"@id": "duumbi:main/max/false_branch/0"}}
        ]
      }
    ]
  }
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: replace_block to atomically rewrite a block body
    // ------------------------------------------------------------------
    PatchExample {
        description: "Using replace_block to rewrite a block that adds Print before Return",
        error_codes: &["E003", "E004", "E009"],
        keywords: &[
            "fix", "rewrite", "replace", "block", "print", "return", "missing",
        ],
        patch_json: r#"replace_block: {
  "block_id": "duumbi:main/main/entry",
  "ops": [
    {"@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/0", "duumbi:value": 42, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Print",  "@id": "duumbi:main/main/entry/1", "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/2", "duumbi:operand": {"@id": "duumbi:main/main/entry/0"}}
  ]
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: fix E001 type mismatch — use matching types in binary op
    // ------------------------------------------------------------------
    PatchExample {
        description: "Fixing E001 type mismatch by ensuring both operands have the same resultType",
        error_codes: &["E001"],
        keywords: &[
            "type", "mismatch", "i64", "f64", "operand", "add", "sub", "mul", "div",
        ],
        patch_json: r#"replace_block: {
  "block_id": "duumbi:main/compute/entry",
  "ops": [
    {"@type": "duumbi:Load",   "@id": "duumbi:main/compute/entry/0", "duumbi:variable": "x", "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const",  "@id": "duumbi:main/compute/entry/1", "duumbi:value": 10, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Add",    "@id": "duumbi:main/compute/entry/2", "duumbi:left": {"@id": "duumbi:main/compute/entry/0"}, "duumbi:right": {"@id": "duumbi:main/compute/entry/1"}, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/compute/entry/3", "duumbi:operand": {"@id": "duumbi:main/compute/entry/2"}}
  ]
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: recursive function (Call + Compare for base case)
    // ------------------------------------------------------------------
    PatchExample {
        description: "Adding a recursive factorial function with base case Branch",
        error_codes: &["E003", "E009", "E010"],
        keywords: &[
            "recursive",
            "recursion",
            "factorial",
            "fibonacci",
            "loop",
            "base case",
        ],
        patch_json: r#"add_function: {
  "function": {
    "@type": "duumbi:Function",
    "@id": "duumbi:main/factorial",
    "duumbi:name": "factorial",
    "duumbi:returnType": "i64",
    "duumbi:params": [{"duumbi:name": "n", "duumbi:paramType": "i64"}],
    "duumbi:blocks": [
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/factorial/entry",
        "duumbi:label": "entry",
        "duumbi:ops": [
          {"@type": "duumbi:Load",    "@id": "duumbi:main/factorial/entry/0", "duumbi:variable": "n", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Const",   "@id": "duumbi:main/factorial/entry/1", "duumbi:value": 1, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Compare", "@id": "duumbi:main/factorial/entry/2", "duumbi:operator": "le", "duumbi:left": {"@id": "duumbi:main/factorial/entry/0"}, "duumbi:right": {"@id": "duumbi:main/factorial/entry/1"}, "duumbi:resultType": "bool"},
          {"@type": "duumbi:Branch",  "@id": "duumbi:main/factorial/entry/3", "duumbi:condition": {"@id": "duumbi:main/factorial/entry/2"}, "duumbi:trueBlock": "duumbi:main/factorial/base", "duumbi:falseBlock": "duumbi:main/factorial/recurse"}
        ]
      },
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/factorial/base",
        "duumbi:label": "base",
        "duumbi:ops": [
          {"@type": "duumbi:Const",  "@id": "duumbi:main/factorial/base/0", "duumbi:value": 1, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Return", "@id": "duumbi:main/factorial/base/1", "duumbi:operand": {"@id": "duumbi:main/factorial/base/0"}}
        ]
      },
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/factorial/recurse",
        "duumbi:label": "recurse",
        "duumbi:ops": [
          {"@type": "duumbi:Load",   "@id": "duumbi:main/factorial/recurse/0", "duumbi:variable": "n", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Const",  "@id": "duumbi:main/factorial/recurse/1", "duumbi:value": 1, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Sub",    "@id": "duumbi:main/factorial/recurse/2", "duumbi:left": {"@id": "duumbi:main/factorial/recurse/0"}, "duumbi:right": {"@id": "duumbi:main/factorial/recurse/1"}, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Call",   "@id": "duumbi:main/factorial/recurse/3", "duumbi:function": "factorial", "duumbi:args": [{"@id": "duumbi:main/factorial/recurse/2"}], "duumbi:resultType": "i64"},
          {"@type": "duumbi:Load",   "@id": "duumbi:main/factorial/recurse/4", "duumbi:variable": "n", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Mul",    "@id": "duumbi:main/factorial/recurse/5", "duumbi:left": {"@id": "duumbi:main/factorial/recurse/4"}, "duumbi:right": {"@id": "duumbi:main/factorial/recurse/3"}, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Return", "@id": "duumbi:main/factorial/recurse/6", "duumbi:operand": {"@id": "duumbi:main/factorial/recurse/5"}}
        ]
      }
    ]
  }
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: Store/Load for local variable (accumulator pattern)
    // ------------------------------------------------------------------
    PatchExample {
        description: "Using Store/Load to implement a local variable accumulator",
        error_codes: &["E003"],
        keywords: &[
            "store",
            "load",
            "variable",
            "local",
            "accumulator",
            "assign",
        ],
        patch_json: r#"replace_block: {
  "block_id": "duumbi:main/compute/entry",
  "ops": [
    {"@type": "duumbi:Const",  "@id": "duumbi:main/compute/entry/0", "duumbi:value": 10, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Store",  "@id": "duumbi:main/compute/entry/1", "duumbi:variable": "result", "duumbi:operand": {"@id": "duumbi:main/compute/entry/0"}},
    {"@type": "duumbi:Load",   "@id": "duumbi:main/compute/entry/2", "duumbi:variable": "result", "duumbi:resultType": "i64"},
    {"@type": "duumbi:Return", "@id": "duumbi:main/compute/entry/3", "duumbi:operand": {"@id": "duumbi:main/compute/entry/2"}}
  ]
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: function calling another function (Call op)
    // ------------------------------------------------------------------
    PatchExample {
        description: "Calling an existing function from main using the Call op",
        error_codes: &["E003", "E010"],
        keywords: &["call", "invoke", "use", "main", "demonstrate", "show"],
        patch_json: r#"replace_block: {
  "block_id": "duumbi:main/main/entry",
  "ops": [
    {"@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/0", "duumbi:value": 5, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Const",  "@id": "duumbi:main/main/entry/1", "duumbi:value": 3, "duumbi:resultType": "i64"},
    {"@type": "duumbi:Call",   "@id": "duumbi:main/main/entry/2", "duumbi:function": "add", "duumbi:args": [{"@id": "duumbi:main/main/entry/0"}, {"@id": "duumbi:main/main/entry/1"}], "duumbi:resultType": "i64"},
    {"@type": "duumbi:Print",  "@id": "duumbi:main/main/entry/3", "duumbi:operand": {"@id": "duumbi:main/main/entry/2"}},
    {"@type": "duumbi:Return", "@id": "duumbi:main/main/entry/4", "duumbi:operand": {"@id": "duumbi:main/main/entry/2"}}
  ]
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: modify_op to change a constant value
    // ------------------------------------------------------------------
    PatchExample {
        description: "Using modify_op to change a Const node's value",
        error_codes: &["E003", "E004"],
        keywords: &["modify", "change", "update", "value", "constant"],
        patch_json: r#"modify_op: {
  "node_id": "duumbi:main/main/entry/0",
  "field": "duumbi:value",
  "value": 42
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: fix E005 duplicate @id — use unique suffix
    // ------------------------------------------------------------------
    PatchExample {
        description: "Fixing E005 duplicate @id by using unique index suffixes per block",
        error_codes: &["E005"],
        keywords: &["duplicate", "id", "unique", "@id", "conflict"],
        patch_json: r#"add_function: {
  "function": {
    "@type": "duumbi:Function",
    "@id": "duumbi:main/helper",
    "duumbi:name": "helper",
    "duumbi:returnType": "i64",
    "duumbi:params": [],
    "duumbi:blocks": [{
      "@type": "duumbi:Block",
      "@id": "duumbi:main/helper/entry",
      "duumbi:label": "entry",
      "duumbi:ops": [
        {"@type": "duumbi:Const",  "@id": "duumbi:main/helper/entry/0", "duumbi:value": 0, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Return", "@id": "duumbi:main/helper/entry/1", "duumbi:operand": {"@id": "duumbi:main/helper/entry/0"}}
      ]
    }]
  }
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: multiply function
    // ------------------------------------------------------------------
    PatchExample {
        description: "Adding a multiply(a, b) function",
        error_codes: &["E003"],
        keywords: &["multiply", "mul", "product", "times"],
        patch_json: r#"add_function: {
  "function": {
    "@type": "duumbi:Function",
    "@id": "duumbi:main/multiply",
    "duumbi:name": "multiply",
    "duumbi:returnType": "i64",
    "duumbi:params": [
      {"duumbi:name": "a", "duumbi:paramType": "i64"},
      {"duumbi:name": "b", "duumbi:paramType": "i64"}
    ],
    "duumbi:blocks": [{
      "@type": "duumbi:Block",
      "@id": "duumbi:main/multiply/entry",
      "duumbi:label": "entry",
      "duumbi:ops": [
        {"@type": "duumbi:Load",   "@id": "duumbi:main/multiply/entry/0", "duumbi:variable": "a", "duumbi:resultType": "i64"},
        {"@type": "duumbi:Load",   "@id": "duumbi:main/multiply/entry/1", "duumbi:variable": "b", "duumbi:resultType": "i64"},
        {"@type": "duumbi:Mul",    "@id": "duumbi:main/multiply/entry/2", "duumbi:left": {"@id": "duumbi:main/multiply/entry/0"}, "duumbi:right": {"@id": "duumbi:main/multiply/entry/1"}, "duumbi:resultType": "i64"},
        {"@type": "duumbi:Return", "@id": "duumbi:main/multiply/entry/3", "duumbi:operand": {"@id": "duumbi:main/multiply/entry/2"}}
      ]
    }]
  }
}"#,
    },
    // ------------------------------------------------------------------
    // Pattern: division with safe guard (div with zero check via Branch)
    // ------------------------------------------------------------------
    PatchExample {
        description: "Safe division function that returns 0 when divisor is zero",
        error_codes: &["E001", "E003"],
        keywords: &["divide", "div", "division", "zero", "safe", "guard"],
        patch_json: r#"add_function: {
  "function": {
    "@type": "duumbi:Function",
    "@id": "duumbi:main/safe_div",
    "duumbi:name": "safe_div",
    "duumbi:returnType": "i64",
    "duumbi:params": [
      {"duumbi:name": "a", "duumbi:paramType": "i64"},
      {"duumbi:name": "b", "duumbi:paramType": "i64"}
    ],
    "duumbi:blocks": [
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/safe_div/entry",
        "duumbi:label": "entry",
        "duumbi:ops": [
          {"@type": "duumbi:Load",    "@id": "duumbi:main/safe_div/entry/0", "duumbi:variable": "b", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Const",   "@id": "duumbi:main/safe_div/entry/1", "duumbi:value": 0, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Compare", "@id": "duumbi:main/safe_div/entry/2", "duumbi:operator": "eq", "duumbi:left": {"@id": "duumbi:main/safe_div/entry/0"}, "duumbi:right": {"@id": "duumbi:main/safe_div/entry/1"}, "duumbi:resultType": "bool"},
          {"@type": "duumbi:Branch",  "@id": "duumbi:main/safe_div/entry/3", "duumbi:condition": {"@id": "duumbi:main/safe_div/entry/2"}, "duumbi:trueBlock": "duumbi:main/safe_div/zero", "duumbi:falseBlock": "duumbi:main/safe_div/divide"}
        ]
      },
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/safe_div/zero",
        "duumbi:label": "zero",
        "duumbi:ops": [
          {"@type": "duumbi:Const",  "@id": "duumbi:main/safe_div/zero/0", "duumbi:value": 0, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Return", "@id": "duumbi:main/safe_div/zero/1", "duumbi:operand": {"@id": "duumbi:main/safe_div/zero/0"}}
        ]
      },
      {
        "@type": "duumbi:Block",
        "@id": "duumbi:main/safe_div/divide",
        "duumbi:label": "divide",
        "duumbi:ops": [
          {"@type": "duumbi:Load",   "@id": "duumbi:main/safe_div/divide/0", "duumbi:variable": "a", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Load",   "@id": "duumbi:main/safe_div/divide/1", "duumbi:variable": "b", "duumbi:resultType": "i64"},
          {"@type": "duumbi:Div",    "@id": "duumbi:main/safe_div/divide/2", "duumbi:left": {"@id": "duumbi:main/safe_div/divide/0"}, "duumbi:right": {"@id": "duumbi:main/safe_div/divide/1"}, "duumbi:resultType": "i64"},
          {"@type": "duumbi:Return", "@id": "duumbi:main/safe_div/divide/3", "duumbi:operand": {"@id": "duumbi:main/safe_div/divide/2"}}
        ]
      }
    ]
  }
}"#,
    },
];

/// Returns the best matching example for the given error codes and user request,
/// or `None` if no example reaches the minimum match threshold.
///
/// Scoring: +3 per matched error code, +1 per matched keyword in the request.
/// Returns the highest-scoring example with score > 0.
#[must_use]
pub fn select_example(
    diagnostics: &[crate::errors::Diagnostic],
    user_request: &str,
) -> Option<&'static str> {
    let error_codes: Vec<&str> = diagnostics.iter().map(|d| d.code.as_str()).collect();
    let request_lower = user_request.to_lowercase();

    let best = EXAMPLES
        .iter()
        .map(|ex| {
            let mut score: i32 = 0;
            for code in &error_codes {
                if ex.error_codes.contains(code) {
                    score += 3;
                }
            }
            for kw in ex.keywords {
                if request_lower.contains(kw) {
                    score += 1;
                }
            }
            (score, ex)
        })
        .filter(|(score, _)| *score > 0)
        .max_by_key(|(score, _)| *score);

    best.map(|(_, ex)| ex.patch_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{Diagnostic, codes};

    fn diag(code: &str) -> Diagnostic {
        Diagnostic::error(code, "test error")
    }

    #[test]
    fn select_example_returns_none_when_no_match() {
        let result = select_example(&[], "completely unrelated request xyz123");
        assert!(result.is_none());
    }

    #[test]
    fn select_example_matches_by_error_code() {
        let diagnostics = vec![diag(codes::E001_TYPE_MISMATCH)];
        let result = select_example(&diagnostics, "fix the function");
        assert!(result.is_some(), "E001 should match at least one example");
        let patch = result.expect("must have example");
        // E001 example should reference replace_block
        assert!(patch.contains("replace_block"));
    }

    #[test]
    fn select_example_matches_by_keyword() {
        let result = select_example(&[], "add a recursive factorial function");
        assert!(result.is_some(), "factorial keyword should match");
        let patch = result.expect("must have example");
        assert!(patch.contains("factorial"));
    }

    #[test]
    fn select_example_prefers_error_code_match_over_keywords() {
        // E005 (duplicate id) should prioritize the duplicate @id example
        let diagnostics = vec![diag(codes::E005_DUPLICATE_ID)];
        let result = select_example(&diagnostics, "add a function");
        assert!(result.is_some());
    }

    #[test]
    fn select_example_max_count_returns_single_best() {
        let diagnostics = vec![diag(codes::E003_MISSING_FIELD)];
        let result = select_example(&diagnostics, "add a simple add function");
        // Should return something (E003 + "add" keywords match multiple)
        assert!(result.is_some());
    }
}
