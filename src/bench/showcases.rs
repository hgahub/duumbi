//! Embedded showcase intent specs.
//!
//! Each showcase is a YAML string compiled into the binary via `include_str!`.
//! The benchmark runner parses these at runtime into [`IntentSpec`] structs.

use crate::intent::spec::IntentSpec;

/// Benchmark suite classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowcaseSuite {
    /// Existing six-showcase benchmark suite.
    Core,
    /// Scaled multi-function and multi-module benchmark suite.
    Scaled,
}

impl ShowcaseSuite {
    /// Returns the stable report string for the suite.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Scaled => "scaled",
        }
    }
}

/// Verification strategy expected for a benchmark showcase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowcaseVerification {
    /// Verify with existing i64 intent test cases.
    I64Tests,
    /// Requires broader process evidence beyond the current i64 verifier.
    ProcessEvidence {
        /// Evidence kind written to benchmark reports.
        evidence_kind: &'static str,
        /// Loopback route expected for process evidence.
        expected_route: &'static str,
        /// JSON fields expected in process evidence.
        expected_json_fields: &'static [&'static str],
        /// Current verification gap to report until automated process checks exist.
        verification_gap: &'static str,
    },
}

/// Calculator: add, sub, mul, div on i64.
pub const CALCULATOR_YAML: &str = include_str!("showcases/calculator.yaml");

/// Fibonacci: recursive fib(n).
pub const FIBONACCI_YAML: &str = include_str!("showcases/fibonacci.yaml");

/// Sorting (simplified): min_of_two(a, b) returns the smaller of two i64 values.
pub const SORTING_YAML: &str = include_str!("showcases/sorting.yaml");

/// State machine (simplified): clamp(x, lo, hi) clamps an i64 between lo and hi.
pub const STATE_MACHINE_YAML: &str = include_str!("showcases/state_machine.yaml");

/// Multi-module: double and square in a separate ops module.
pub const MULTI_MODULE_YAML: &str = include_str!("showcases/multi_module.yaml");

/// String operations: length_of_hello, hello_contains_ell.
pub const STRING_OPS_YAML: &str = include_str!("showcases/string_ops.yaml");

/// Scaled multi-function math pipeline.
pub const SCALED_MATH_PIPELINE_YAML: &str = include_str!("showcases/scaled_math_pipeline.yaml");

/// Scaled cross-module stats library.
pub const SCALED_CROSS_MODULE_STATS_YAML: &str =
    include_str!("showcases/scaled_cross_module_stats.yaml");

/// Scaled branch/recursion showcase.
pub const SCALED_BRANCH_RECURSION_YAML: &str =
    include_str!("showcases/scaled_branch_recursion.yaml");

/// Scaled string/array-style module showcase.
pub const SCALED_STRING_ARRAY_YAML: &str = include_str!("showcases/scaled_string_array.yaml");

/// Scaled HTTP + SQLite + JSON composition showcase.
pub const SCALED_HTTP_SQLITE_JSON_YAML: &str =
    include_str!("showcases/scaled_http_sqlite_json.yaml");

/// A showcase entry with its name and embedded YAML source.
#[derive(Debug, Clone, Copy)]
pub struct Showcase {
    /// Short identifier (e.g. `"calculator"`).
    pub name: &'static str,
    /// Raw YAML source.
    pub yaml: &'static str,
    /// Benchmark suite this showcase belongs to.
    pub suite: ShowcaseSuite,
    /// Whether the showcase is part of the low-budget smoke subset.
    pub smoke: bool,
    /// Feature tags used in scaled benchmark reports.
    pub tags: &'static [&'static str],
    /// Verification strategy for the showcase.
    pub verification: ShowcaseVerification,
}

/// All available showcases, in canonical order.
pub const ALL_SHOWCASES: &[Showcase] = &[
    Showcase {
        name: "calculator",
        yaml: CALCULATOR_YAML,
        suite: ShowcaseSuite::Core,
        smoke: true,
        tags: &["single_module", "multi_function", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "fibonacci",
        yaml: FIBONACCI_YAML,
        suite: ShowcaseSuite::Core,
        smoke: false,
        tags: &["recursion", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "sorting",
        yaml: SORTING_YAML,
        suite: ShowcaseSuite::Core,
        smoke: false,
        tags: &["branch", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "state_machine",
        yaml: STATE_MACHINE_YAML,
        suite: ShowcaseSuite::Core,
        smoke: false,
        tags: &["branch", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "multi_module",
        yaml: MULTI_MODULE_YAML,
        suite: ShowcaseSuite::Core,
        smoke: true,
        tags: &["multi_module", "cross_module", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "string_ops",
        yaml: STRING_OPS_YAML,
        suite: ShowcaseSuite::Core,
        smoke: false,
        tags: &["string", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
];

/// Scaled showcases for DUUMBI-689, in canonical order.
pub const SCALED_SHOWCASES: &[Showcase] = &[
    Showcase {
        name: "scaled_math_pipeline",
        yaml: SCALED_MATH_PIPELINE_YAML,
        suite: ShowcaseSuite::Scaled,
        smoke: true,
        tags: &["scaled", "multi_function", "single_module", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "scaled_cross_module_stats",
        yaml: SCALED_CROSS_MODULE_STATS_YAML,
        suite: ShowcaseSuite::Scaled,
        smoke: true,
        tags: &["scaled", "multi_module", "cross_module", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "scaled_branch_recursion",
        yaml: SCALED_BRANCH_RECURSION_YAML,
        suite: ShowcaseSuite::Scaled,
        smoke: false,
        tags: &["scaled", "branch", "recursion", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "scaled_string_array",
        yaml: SCALED_STRING_ARRAY_YAML,
        suite: ShowcaseSuite::Scaled,
        smoke: false,
        tags: &["scaled", "string", "array_like", "i64"],
        verification: ShowcaseVerification::I64Tests,
    },
    Showcase {
        name: "scaled_http_sqlite_json",
        yaml: SCALED_HTTP_SQLITE_JSON_YAML,
        suite: ShowcaseSuite::Scaled,
        smoke: true,
        tags: &["scaled", "http", "sqlite", "json", "process_evidence"],
        verification: ShowcaseVerification::ProcessEvidence {
            evidence_kind: "loopback_http_sqlite_json",
            expected_route: "/facts",
            expected_json_fields: &["service", "route", "count", "first_fact", "storage"],
            verification_gap: "current verifier does not check HTTP JSON payload semantics",
        },
    },
];

/// Parses a showcase YAML into an [`IntentSpec`].
///
/// Returns an error string if the YAML is malformed.
#[must_use = "check whether the parse succeeded"]
pub fn parse_showcase(showcase: &Showcase) -> Result<IntentSpec, String> {
    serde_yaml::from_str(showcase.yaml)
        .map_err(|e| format!("failed to parse showcase '{}': {e}", showcase.name))
}

/// Returns the subset of showcases matching the given filter names.
///
/// If `filter` is `None`, returns all showcases.
#[must_use]
pub fn filter_showcases(filter: Option<&[String]>) -> Vec<&'static Showcase> {
    filter_showcases_with_options(filter, None, false)
}

/// Returns showcases matching optional names, suite, and smoke selection.
///
/// The default with no suite and no explicit names remains the historical core
/// suite. Explicit names can target either core or scaled showcases.
#[must_use]
pub fn filter_showcases_with_options(
    filter: Option<&[String]>,
    suite: Option<ShowcaseSuite>,
    smoke: bool,
) -> Vec<&'static Showcase> {
    let candidates: Vec<&Showcase> = if filter.is_some() {
        ALL_SHOWCASES
            .iter()
            .chain(SCALED_SHOWCASES.iter())
            .collect()
    } else {
        match suite {
            None | Some(ShowcaseSuite::Core) => ALL_SHOWCASES.iter().collect(),
            Some(ShowcaseSuite::Scaled) => SCALED_SHOWCASES.iter().collect(),
        }
    };

    match filter {
        None => candidates,
        Some(names) => candidates
            .into_iter()
            .filter(|s| names.iter().any(|n| n == s.name))
            .collect(),
    }
    .into_iter()
    .filter(|s| !smoke || s.smoke)
    .filter(|s| suite.is_none_or(|expected| s.suite == expected))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_showcases_parse() {
        for showcase in ALL_SHOWCASES {
            let spec = parse_showcase(showcase)
                .unwrap_or_else(|e| panic!("showcase '{}' failed to parse: {e}", showcase.name));
            assert!(
                !spec.test_cases.is_empty(),
                "showcase '{}' has no test cases",
                showcase.name
            );
            assert!(
                !spec.intent.is_empty(),
                "showcase '{}' has empty intent",
                showcase.name
            );
        }
    }

    #[test]
    fn scaled_showcases_parse_and_include_required_features() {
        let mut has_cross_module = false;
        let mut has_http_sqlite_json = false;

        for showcase in SCALED_SHOWCASES {
            let spec = parse_showcase(showcase)
                .unwrap_or_else(|e| panic!("showcase '{}' failed to parse: {e}", showcase.name));
            assert!(
                !spec.intent.is_empty(),
                "showcase '{}' has empty intent",
                showcase.name
            );
            assert!(
                !showcase.tags.is_empty(),
                "showcase '{}' has no feature tags",
                showcase.name
            );
            has_cross_module |= showcase.tags.contains(&"cross_module");
            has_http_sqlite_json |= showcase.tags.contains(&"http")
                && showcase.tags.contains(&"sqlite")
                && showcase.tags.contains(&"json");

            if matches!(showcase.verification, ShowcaseVerification::I64Tests) {
                assert!(
                    !spec.test_cases.is_empty(),
                    "showcase '{}' has no verifier test cases",
                    showcase.name
                );
            }
        }

        assert!(
            has_cross_module,
            "scaled suite must include cross-module behavior"
        );
        assert!(
            has_http_sqlite_json,
            "scaled suite must include HTTP + SQLite + JSON behavior"
        );
    }

    #[test]
    fn filter_showcases_returns_subset() {
        let names = vec!["calculator".to_string(), "fibonacci".to_string()];
        let filtered = filter_showcases(Some(&names));
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "calculator");
        assert_eq!(filtered[1].name, "fibonacci");
    }

    #[test]
    fn filter_showcases_none_returns_all() {
        let all = filter_showcases(None);
        assert_eq!(all.len(), ALL_SHOWCASES.len());
    }

    #[test]
    fn scaled_smoke_filter_includes_process_evidence_case() {
        let filtered = filter_showcases_with_options(None, Some(ShowcaseSuite::Scaled), true);
        assert!(filtered.iter().any(|s| s.name == "scaled_math_pipeline"));
        assert!(
            filtered
                .iter()
                .any(|s| s.name == "scaled_cross_module_stats")
        );
        assert!(filtered.iter().any(|s| s.name == "scaled_http_sqlite_json"));
        assert!(filtered.iter().all(|s| s.smoke));
    }
}
