//! Shared interaction-mode model for CLI, Studio, and future integrations.
//!
//! Query mode is read-only. Agent and Intent are write-capable through their
//! existing explicit pipelines.

pub mod router;

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Top-level DUUMBI user interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionMode {
    /// Read-only explanation, inspection, and architectural conversation.
    #[default]
    Query,
    /// Direct graph mutation for bounded changes.
    Agent,
    /// Intent-spec creation, refinement, execution, and verification.
    Intent,
}

impl InteractionMode {
    /// Human-readable stable label for this mode.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Agent => "agent",
            Self::Intent => "intent",
        }
    }

    /// Returns the next mode in the Shift+Tab cycle.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Query => Self::Agent,
            Self::Agent => Self::Intent,
            Self::Intent => Self::Query,
        }
    }
}

/// Error returned when parsing an unknown interaction mode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown interaction mode: {0}")]
pub struct ParseInteractionModeError(String);

impl FromStr for InteractionMode {
    type Err = ParseInteractionModeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "query" | "ask" => Ok(Self::Query),
            "agent" | "add" => Ok(Self::Agent),
            "intent" => Ok(Self::Intent),
            other => Err(ParseInteractionModeError(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_query() {
        assert_eq!(InteractionMode::default(), InteractionMode::Query);
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(InteractionMode::Query.label(), "query");
        assert_eq!(InteractionMode::Agent.label(), "agent");
        assert_eq!(InteractionMode::Intent.label(), "intent");
    }

    #[test]
    fn cycle_order_is_query_agent_intent() {
        assert_eq!(InteractionMode::Query.next(), InteractionMode::Agent);
        assert_eq!(InteractionMode::Agent.next(), InteractionMode::Intent);
        assert_eq!(InteractionMode::Intent.next(), InteractionMode::Query);
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(
            "query".parse::<InteractionMode>(),
            Ok(InteractionMode::Query)
        );
        assert_eq!("ask".parse::<InteractionMode>(), Ok(InteractionMode::Query));
        assert_eq!(
            "agent".parse::<InteractionMode>(),
            Ok(InteractionMode::Agent)
        );
        assert_eq!(
            "intent".parse::<InteractionMode>(),
            Ok(InteractionMode::Intent)
        );
    }
}
