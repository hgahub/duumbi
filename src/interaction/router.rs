//! Lightweight request-shape routing helpers for mode handoff suggestions.

use crate::interaction::InteractionMode;

/// Coarse shape of a natural-language request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestShape {
    /// Question or explanation request; best handled by Query mode.
    Question,
    /// Bounded change request; best handled by Agent mode.
    Mutation,
    /// Larger feature/spec request; best handled by Intent mode.
    Intent,
    /// No confident classification.
    Unknown,
}

impl RequestShape {
    /// Preferred interaction mode for this request shape, if known.
    #[must_use]
    pub const fn preferred_mode(self) -> Option<InteractionMode> {
        match self {
            Self::Question => Some(InteractionMode::Query),
            Self::Mutation => Some(InteractionMode::Agent),
            Self::Intent => Some(InteractionMode::Intent),
            Self::Unknown => None,
        }
    }
}

/// Classifies a request using intentionally simple, deterministic heuristics.
#[must_use]
pub fn classify_request(input: &str) -> RequestShape {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return RequestShape::Unknown;
    }
    let lower = trimmed.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or("");

    if lower.ends_with('?')
        || matches!(
            first,
            "what" | "why" | "where" | "when" | "who" | "how" | "explain" | "compare" | "show"
        )
    {
        return RequestShape::Question;
    }

    if lower.contains("build a ")
        || lower.contains("create an application")
        || lower.contains("create a new application")
        || lower.contains("with tests")
        || lower.contains("acceptance criteria")
        || matches!(first, "plan")
    {
        return RequestShape::Intent;
    }

    if matches!(
        first,
        "add" | "fix" | "change" | "modify" | "update" | "refactor" | "remove" | "implement"
    ) {
        return RequestShape::Mutation;
    }

    RequestShape::Unknown
}

/// Returns true when the input looks better suited for Query mode.
#[must_use]
pub fn is_question_like(input: &str) -> bool {
    classify_request(input) == RequestShape::Question
}

/// Returns true when the input looks like it asks for a graph mutation.
#[must_use]
pub fn is_mutation_like(input: &str) -> bool {
    matches!(
        classify_request(input),
        RequestShape::Mutation | RequestShape::Intent
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_question_shapes() {
        assert_eq!(
            classify_request("What modules exist?"),
            RequestShape::Question
        );
        assert_eq!(
            classify_request("explain this graph"),
            RequestShape::Question
        );
    }

    #[test]
    fn classifies_mutation_shapes() {
        assert_eq!(
            classify_request("add a power function"),
            RequestShape::Mutation
        );
        assert_eq!(classify_request("fix E001"), RequestShape::Mutation);
    }

    #[test]
    fn classifies_intent_shapes() {
        assert_eq!(
            classify_request("plan a calculator module"),
            RequestShape::Intent
        );
        assert_eq!(
            classify_request("build a string utility library with tests"),
            RequestShape::Intent
        );
    }
}
