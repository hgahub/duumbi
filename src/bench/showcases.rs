//! Embedded showcase intent specs.
//!
//! Each showcase is a YAML string compiled into the binary via `include_str!`.
//! The benchmark runner parses these at runtime into [`IntentSpec`] structs.

use crate::intent::spec::IntentSpec;

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

/// A showcase entry with its name and embedded YAML source.
#[derive(Debug, Clone)]
pub struct Showcase {
    /// Short identifier (e.g. `"calculator"`).
    pub name: &'static str,
    /// Raw YAML source.
    pub yaml: &'static str,
}

/// All available showcases, in canonical order.
pub const ALL_SHOWCASES: &[Showcase] = &[
    Showcase {
        name: "calculator",
        yaml: CALCULATOR_YAML,
    },
    Showcase {
        name: "fibonacci",
        yaml: FIBONACCI_YAML,
    },
    Showcase {
        name: "sorting",
        yaml: SORTING_YAML,
    },
    Showcase {
        name: "state_machine",
        yaml: STATE_MACHINE_YAML,
    },
    Showcase {
        name: "multi_module",
        yaml: MULTI_MODULE_YAML,
    },
    Showcase {
        name: "string_ops",
        yaml: STRING_OPS_YAML,
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
    match filter {
        None => ALL_SHOWCASES.iter().collect(),
        Some(names) => ALL_SHOWCASES
            .iter()
            .filter(|s| names.iter().any(|n| n == s.name))
            .collect(),
    }
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
}
