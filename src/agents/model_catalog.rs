//! Versioned internal LLM model catalog and deterministic model routing.
//!
//! Users configure providers and credentials. Duumbi owns concrete model
//! selection so releases can update model IDs and routing policy without
//! exposing model choice as user workflow.

use crate::agents::analyzer::{Complexity, Risk, TaskProfile, TaskType};
use crate::agents::template::AgentRole;
use crate::config::{ProviderConfig, ProviderKind, ResolvedProviderConfig};

const RETIRED_GROK_CODE_FAST_1: &str = "grok-code-fast-1";

/// Static metadata for a model that Duumbi may select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalogEntry {
    /// Provider that serves the model.
    pub provider: ProviderKind,
    /// Provider-specific model identifier.
    pub model: &'static str,
    /// Relative output quality score, 0-100.
    pub quality: u8,
    /// Relative latency score, 0-100.
    pub speed: u8,
    /// Relative affordability score, 0-100.
    pub cost_efficiency: u8,
    /// Whether this model is preferred for reasoning-heavy tasks.
    pub reasoning: bool,
    /// Whether this model is preferred for coding/graph mutation tasks.
    pub coding: bool,
}

/// Context used to select a model for an agent call.
#[derive(Debug, Clone, Default)]
pub struct ModelSelectionContext {
    /// Agent role that will make the call.
    pub agent_role: Option<AgentRole>,
    /// Deterministic task profile when available.
    pub task_profile: Option<TaskProfile>,
    /// Approximate prompt tokens for context-size decisions.
    pub prompt_tokens: Option<usize>,
    /// Remaining token budget for the current intent/session.
    pub budget_remaining_tokens: Option<usize>,
    /// Tool calls are required for graph mutation.
    pub requires_tools: bool,
    /// Models confirmed accessible for the configured credential.
    ///
    /// When this list is non-empty, routing is restricted to these models.
    pub accessible_models: Vec<String>,
    /// Models known to be denied for the configured credential.
    pub denied_models: Vec<String>,
}

/// Returns all models embedded in this Duumbi release.
#[must_use]
pub fn catalog() -> &'static [ModelCatalogEntry] {
    &[
        ModelCatalogEntry {
            provider: ProviderKind::Anthropic,
            model: "claude-opus-4-6",
            quality: 98,
            speed: 45,
            cost_efficiency: 35,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::Anthropic,
            model: "claude-sonnet-4-6",
            quality: 92,
            speed: 75,
            cost_efficiency: 72,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::Anthropic,
            model: "claude-haiku-4-5",
            quality: 76,
            speed: 95,
            cost_efficiency: 96,
            reasoning: false,
            coding: false,
        },
        ModelCatalogEntry {
            provider: ProviderKind::OpenAI,
            model: "gpt-5.5",
            quality: 98,
            speed: 50,
            cost_efficiency: 40,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::OpenAI,
            model: "gpt-5.4",
            quality: 92,
            speed: 76,
            cost_efficiency: 72,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::OpenAI,
            model: "gpt-5.4-mini",
            quality: 78,
            speed: 96,
            cost_efficiency: 96,
            reasoning: false,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::Grok,
            model: "grok-4.20-reasoning",
            quality: 95,
            speed: 60,
            cost_efficiency: 60,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::Grok,
            model: "grok-4.20-non-reasoning",
            quality: 88,
            speed: 80,
            cost_efficiency: 78,
            reasoning: false,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::Grok,
            model: "grok-4-1-fast-reasoning",
            quality: 90,
            speed: 85,
            cost_efficiency: 82,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::Grok,
            model: "grok-4-1-fast-non-reasoning",
            quality: 84,
            speed: 94,
            cost_efficiency: 90,
            reasoning: false,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::MiniMax,
            model: "MiniMax-M2.7",
            quality: 92,
            speed: 70,
            cost_efficiency: 76,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::MiniMax,
            model: "MiniMax-M2.7-highspeed",
            quality: 88,
            speed: 90,
            cost_efficiency: 84,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::MiniMax,
            model: "MiniMax-M2.5",
            quality: 89,
            speed: 72,
            cost_efficiency: 82,
            reasoning: true,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::MiniMax,
            model: "MiniMax-M2.5-highspeed",
            quality: 84,
            speed: 92,
            cost_efficiency: 90,
            reasoning: false,
            coding: true,
        },
        ModelCatalogEntry {
            provider: ProviderKind::OpenRouter,
            model: "openrouter/auto",
            quality: 86,
            speed: 76,
            cost_efficiency: 76,
            reasoning: true,
            coding: true,
        },
    ]
}

/// Returns true when a model identifier is intentionally retired by this release.
#[must_use]
pub(crate) fn is_retired_model(provider: &ProviderKind, model: &str) -> bool {
    matches!(provider, ProviderKind::Grok) && model == RETIRED_GROK_CODE_FAST_1
}

/// Selects the best model for the given provider and call context.
#[must_use]
pub fn select_model(
    provider: &ProviderKind,
    context: &ModelSelectionContext,
) -> Option<&'static ModelCatalogEntry> {
    catalog()
        .iter()
        .filter(|entry| &entry.provider == provider)
        .filter(|entry| model_is_allowed(entry, context))
        .max_by_key(|entry| score_entry(entry, context))
}

/// Resolves a user provider config into a concrete runtime provider config.
#[must_use]
pub fn resolve_provider_config(
    config: &ProviderConfig,
    context: &ModelSelectionContext,
) -> Option<ResolvedProviderConfig> {
    let selected = config
        .model
        .as_deref()
        .filter(|model| !is_retired_model(&config.provider, model))
        .map(str::to_string)
        .or_else(|| select_model(&config.provider, context).map(|entry| entry.model.to_string()))?;

    Some(ResolvedProviderConfig {
        provider: config.provider.clone(),
        model: selected,
        api_key_env: config.api_key_env.clone(),
        base_url: config.base_url.clone(),
        timeout_secs: config.timeout_secs,
        auth_token_env: config.auth_token_env.clone(),
    })
}

fn model_is_allowed(entry: &ModelCatalogEntry, context: &ModelSelectionContext) -> bool {
    if !context.accessible_models.is_empty() {
        return context
            .accessible_models
            .iter()
            .any(|model| model == entry.model);
    }
    !context
        .denied_models
        .iter()
        .any(|model| model == entry.model)
}

fn score_entry(entry: &ModelCatalogEntry, context: &ModelSelectionContext) -> i32 {
    let mut score = i32::from(entry.quality) * 4
        + i32::from(entry.speed) * 2
        + i32::from(entry.cost_efficiency);

    if context.requires_tools && entry.coding {
        score += 20;
    }

    if matches!(
        context.agent_role,
        Some(AgentRole::Planner | AgentRole::Reviewer | AgentRole::Repair)
    ) && entry.reasoning
    {
        score += 18;
    }

    if matches!(
        context.agent_role,
        Some(AgentRole::Coder | AgentRole::Repair)
    ) && entry.coding
    {
        score += 20;
    }

    if let Some(profile) = &context.task_profile {
        if matches!(profile.complexity, Complexity::Complex) && entry.reasoning {
            score += 18;
        }
        if matches!(profile.risk, Risk::High) && entry.reasoning {
            score += 16;
        }
        if matches!(profile.risk, Risk::High) && entry.quality >= 95 {
            score += 120;
        }
        if matches!(profile.task_type, TaskType::Fix | TaskType::Refactor) && entry.coding {
            score += 10;
        }
    }

    if context
        .budget_remaining_tokens
        .is_some_and(|remaining| remaining < 8_000)
    {
        score += i32::from(entry.cost_efficiency) * 2;
    }

    if context.prompt_tokens.is_some_and(|tokens| tokens > 80_000) {
        score += i32::from(entry.quality);
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::analyzer::{Complexity, Risk, Scope, TaskProfile};

    #[test]
    fn catalog_has_entry_for_each_provider() {
        for provider in [
            ProviderKind::Anthropic,
            ProviderKind::OpenAI,
            ProviderKind::Grok,
            ProviderKind::OpenRouter,
            ProviderKind::MiniMax,
        ] {
            assert!(catalog().iter().any(|entry| entry.provider == provider));
        }
    }

    #[test]
    fn legacy_model_overrides_catalog_selection() {
        let config = ProviderConfig {
            provider: ProviderKind::Anthropic,
            role: crate::config::ProviderRole::Primary,
            model: Some("legacy-model".to_string()),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        };

        let resolved = resolve_provider_config(&config, &ModelSelectionContext::default());

        assert_eq!(resolved.expect("model must resolve").model, "legacy-model");
    }

    #[test]
    fn grok_code_fast_is_not_in_catalog() {
        assert!(
            !catalog()
                .iter()
                .any(|entry| entry.provider == ProviderKind::Grok
                    && entry.model == RETIRED_GROK_CODE_FAST_1)
        );
    }

    #[test]
    fn retired_grok_legacy_model_falls_back_to_catalog_selection() {
        let config = ProviderConfig {
            provider: ProviderKind::Grok,
            role: crate::config::ProviderRole::Primary,
            model: Some(RETIRED_GROK_CODE_FAST_1.to_string()),
            api_key_env: "XAI_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        };

        let resolved = resolve_provider_config(&config, &ModelSelectionContext::default())
            .expect("model must resolve");

        assert_ne!(resolved.model, RETIRED_GROK_CODE_FAST_1);
        assert!(
            catalog()
                .iter()
                .any(|entry| entry.provider == ProviderKind::Grok && entry.model == resolved.model)
        );
    }

    #[test]
    fn high_risk_repair_prefers_reasoning_model() {
        let context = ModelSelectionContext {
            agent_role: Some(AgentRole::Repair),
            task_profile: Some(TaskProfile {
                complexity: Complexity::Complex,
                task_type: TaskType::Fix,
                scope: Scope::MultiModule,
                risk: Risk::High,
            }),
            requires_tools: true,
            ..ModelSelectionContext::default()
        };

        let selected = select_model(&ProviderKind::Anthropic, &context);

        assert_eq!(
            selected.expect("model must resolve").model,
            "claude-opus-4-6"
        );
    }

    #[test]
    fn tight_budget_prefers_mini_model() {
        let context = ModelSelectionContext {
            budget_remaining_tokens: Some(1_000),
            ..ModelSelectionContext::default()
        };

        let selected = select_model(&ProviderKind::OpenAI, &context);

        assert_eq!(selected.expect("model must resolve").model, "gpt-5.4-mini");
    }

    #[test]
    fn denied_models_are_excluded_from_selection() {
        let context = ModelSelectionContext {
            denied_models: vec!["MiniMax-M2.7-highspeed".to_string()],
            ..ModelSelectionContext::default()
        };

        let selected = select_model(&ProviderKind::MiniMax, &context);

        assert_ne!(
            selected.expect("model must resolve").model,
            "MiniMax-M2.7-highspeed"
        );
    }

    #[test]
    fn accessible_models_restrict_selection() {
        let context = ModelSelectionContext {
            accessible_models: vec!["MiniMax-M2.5".to_string()],
            ..ModelSelectionContext::default()
        };

        let selected = select_model(&ProviderKind::MiniMax, &context);

        assert_eq!(selected.expect("model must resolve").model, "MiniMax-M2.5");
    }

    #[test]
    fn empty_allowed_set_after_access_filter_returns_none() {
        let context = ModelSelectionContext {
            accessible_models: vec!["unknown-model".to_string()],
            denied_models: vec!["MiniMax-M2.7".to_string()],
            ..ModelSelectionContext::default()
        };

        assert!(select_model(&ProviderKind::MiniMax, &context).is_none());
    }
}
