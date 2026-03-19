//! Token budget estimation and context fitting.
//!
//! Provides a [`TokenEstimator`] trait with a simple character-based
//! implementation ([`CharEstimator`]: char_count / 3.5 + 10% margin).
//! The [`fit_to_budget`] function drops lowest-priority fragments until
//! the context fits within the token limit.

use crate::context::collector::ContextNodes;

/// Default maximum context tokens.
pub const DEFAULT_MAX_TOKENS: usize = 8192;

/// Trait for estimating token count from text.
pub trait TokenEstimator {
    /// Estimates the number of tokens in the given text.
    fn estimate(&self, text: &str) -> usize;
}

/// Simple character-based token estimator.
///
/// Uses the heuristic: `char_count / 3.5` with a 10% safety margin.
/// This is intentionally conservative — avoids depending on tiktoken-rs.
pub struct CharEstimator;

impl TokenEstimator for CharEstimator {
    fn estimate(&self, text: &str) -> usize {
        let raw = (text.len() as f64 / 3.5).ceil() as usize;
        // Add 10% safety margin
        raw + raw / 10
    }
}

/// Returns the default maximum context tokens.
#[must_use]
pub fn default_max_tokens() -> usize {
    DEFAULT_MAX_TOKENS
}

/// Fits collected context nodes within a token budget.
///
/// Returns a single formatted string with fragments in priority order,
/// dropping lowest-priority (highest priority number) fragments first
/// until the result fits within `max_tokens`.
#[must_use]
pub fn fit_to_budget(
    nodes: &ContextNodes,
    max_tokens: usize,
    estimator: &dyn TokenEstimator,
) -> String {
    if nodes.fragments.is_empty() {
        return String::new();
    }

    // Try including all fragments first
    let all_text = format_fragments(&nodes.fragments);
    if estimator.estimate(&all_text) <= max_tokens {
        return all_text;
    }

    // Drop fragments from the end (lowest priority = highest number)
    // Fragments are already sorted by priority (ascending)
    let mut included = nodes.fragments.clone();
    while !included.is_empty() {
        let text = format_fragments(&included);
        if estimator.estimate(&text) <= max_tokens {
            return text;
        }
        // Remove last (lowest priority) fragment
        included.pop();
    }

    String::new()
}

/// Formats fragments into a single context string.
fn format_fragments(fragments: &[crate::context::collector::ContextFragment]) -> String {
    fragments
        .iter()
        .map(|f| f.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::collector::ContextFragment;

    #[test]
    fn char_estimator_basic() {
        let est = CharEstimator;
        // 35 chars / 3.5 = 10, +10% = 11
        assert_eq!(est.estimate("12345678901234567890123456789012345"), 11);
    }

    #[test]
    fn char_estimator_empty() {
        let est = CharEstimator;
        assert_eq!(est.estimate(""), 0);
    }

    #[test]
    fn fit_to_budget_all_fit() {
        let nodes = ContextNodes {
            fragments: vec![ContextFragment {
                text: "short text".to_string(),
                priority: 1,
                source_module: "main".to_string(),
            }],
        };
        let result = fit_to_budget(&nodes, 1000, &CharEstimator);
        assert_eq!(result, "short text");
    }

    #[test]
    fn fit_to_budget_drops_low_priority() {
        let long_text = "x".repeat(10000); // ~3143 tokens
        let nodes = ContextNodes {
            fragments: vec![
                ContextFragment {
                    text: "important".to_string(),
                    priority: 1,
                    source_module: "main".to_string(),
                },
                ContextFragment {
                    text: long_text,
                    priority: 5,
                    source_module: "other".to_string(),
                },
            ],
        };
        let result = fit_to_budget(&nodes, 100, &CharEstimator);
        assert!(result.contains("important"));
        assert!(!result.contains("xxxx"));
    }

    #[test]
    fn fit_to_budget_empty() {
        let nodes = ContextNodes {
            fragments: Vec::new(),
        };
        let result = fit_to_budget(&nodes, 1000, &CharEstimator);
        assert!(result.is_empty());
    }

    #[test]
    fn default_max_tokens_value() {
        assert_eq!(default_max_tokens(), 8192);
    }
}
