//! Versioned internal LLM model catalog and deterministic model routing.
//!
//! Users configure providers and credentials. Duumbi owns concrete model
//! selection so releases can update model IDs and routing policy without
//! exposing model choice as user workflow.

use std::collections::HashSet;

use crate::agents::analyzer::{Complexity, Risk, TaskProfile, TaskType};
use crate::agents::template::AgentRole;
use crate::config::{ProviderConfig, ProviderKind, ResolvedProviderConfig};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(crate) const RETIRED_GROK_CODE_FAST_1: &str = "grok-code-fast-1";

/// Schema version accepted for refreshed provider model catalogs.
pub const MODEL_CATALOG_V1_SCHEMA_VERSION: &str = "duumbi.model_catalog.v1";

/// Canonical provider metadata used by v1 catalog validation and setup UX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogProviderMetadata {
    /// User-facing provider name.
    pub display_name: &'static str,
    /// Canonical config key used in new catalog and provider setup paths.
    pub config_key: &'static str,
    /// Conventional environment variable that stores the provider API key.
    pub api_key_env: &'static str,
}

const ACCEPTED_V1_PROVIDERS: &[CatalogProviderMetadata] = &[
    CatalogProviderMetadata {
        display_name: "Anthropic",
        config_key: "anthropic",
        api_key_env: "ANTHROPIC_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "OpenAI",
        config_key: "openai",
        api_key_env: "OPENAI_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "xAI",
        config_key: "xai",
        api_key_env: "XAI_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "MiniMax",
        config_key: "minimax",
        api_key_env: "MINIMAX_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "DeepSeek",
        config_key: "deepseek",
        api_key_env: "DEEPSEEK_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "Alibaba Cloud Model Studio (Qwen)",
        config_key: "qwen",
        api_key_env: "DASHSCOPE_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "Moonshot AI (Kimi)",
        config_key: "moonshot",
        api_key_env: "MOONSHOT_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "Zhipu AI (GLM)",
        config_key: "zhipu",
        api_key_env: "ZHIPUAI_API_KEY",
    },
    CatalogProviderMetadata {
        display_name: "Google Gemini",
        config_key: "gemini",
        api_key_env: "GEMINI_API_KEY",
    },
];

/// Returns the accepted v1 direct-provider metadata.
#[must_use]
pub fn accepted_v1_provider_metadata() -> &'static [CatalogProviderMetadata] {
    ACCEPTED_V1_PROVIDERS
}

/// Returns canonical provider metadata for a v1 provider key.
#[must_use]
pub fn v1_provider_metadata_for_key(config_key: &str) -> Option<&'static CatalogProviderMetadata> {
    ACCEPTED_V1_PROVIDERS
        .iter()
        .find(|provider| provider.config_key == config_key)
}

/// Returns the canonical key for a legacy provider alias.
#[must_use]
pub fn legacy_provider_alias_canonical_key(config_key: &str) -> Option<&'static str> {
    match config_key {
        "grok" => Some("xai"),
        _ => None,
    }
}

/// A provider entry in a refreshed v1 model catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProviderDocument {
    /// User-facing provider name.
    pub display_name: String,
    /// Canonical config key.
    pub config_key: String,
    /// Conventional API key environment variable.
    pub api_key_env: String,
    /// Optional safe provider note for review surfaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Discovery status for a provider in a refreshed v1 catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDiscoveryState {
    /// Provider metadata came from fresh discovery for this semantic catalog.
    FreshDiscovery,
    /// Provider metadata came from curated fallback input.
    CuratedFallback,
    /// Provider metadata came from previous-known-good fallback input.
    PreviousKnownGoodFallback,
    /// Provider metadata was manually curated without live discovery.
    ManuallyCurated,
}

/// Provider discovery status carried in the adopted catalog bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDiscoveryStatus {
    /// Canonical provider config key.
    pub provider_key: String,
    /// Discovery or fallback state used for the semantic catalog content.
    pub state: ProviderDiscoveryState,
    /// User-facing warning shown when fallback-backed metadata is adopted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Lifecycle state for a model in a refreshed v1 catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLifecycle {
    /// Model is active and selectable by DUUMBI routing.
    Active,
    /// Model is still known but should be avoided for new default routing.
    Deprecated,
    /// Model is retained for compatibility evidence and must not be selected.
    Retired,
}

/// A model entry in a refreshed v1 catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogDocumentEntry {
    /// Canonical provider config key.
    pub provider_key: String,
    /// Provider-specific model identifier.
    pub model_id: String,
    /// Model lifecycle state.
    pub lifecycle: ModelLifecycle,
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

/// Versioned refreshed model catalog document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogDocumentV1 {
    /// Catalog schema version.
    pub schema_version: String,
    /// Semantic catalog content timestamp.
    pub content_timestamp: String,
    /// Source and curation provenance for reviewer inspection.
    pub source: String,
    /// Safe summary of generator status for the adopted semantic catalog.
    pub generator_status_summary: String,
    /// Accepted direct providers covered by this catalog.
    pub providers: Vec<CatalogProviderDocument>,
    /// Per-provider discovery or fallback status carried in the adopted catalog.
    pub provider_discovery_status: Vec<ProviderDiscoveryStatus>,
    /// Provider/model routing entries.
    pub models: Vec<ModelCatalogDocumentEntry>,
    /// Concise user-facing change summary.
    pub change_summary: String,
}

/// Validation error for a refreshed v1 model catalog.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelCatalogValidationError {
    /// The catalog schema version is unsupported.
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(String),
    /// A required text field is empty.
    #[error("required field is empty: {0}")]
    EmptyRequiredField(&'static str),
    /// The provider is not supported by the v1 direct-provider catalog.
    #[error("unsupported v1 provider: {0}")]
    UnsupportedProvider(String),
    /// An accepted v1 provider is absent from the catalog.
    #[error("missing accepted v1 provider: {0}")]
    MissingProvider(String),
    /// OpenRouter appeared in v1 catalog content.
    #[error("OpenRouter is excluded from the v1 provider catalog")]
    OpenRouterExcluded,
    /// Grok appeared as a canonical provider key.
    #[error("grok is a legacy alias; xai must be the canonical provider key")]
    GrokIsLegacyAlias,
    /// A provider appears more than once.
    #[error("duplicate provider key: {0}")]
    DuplicateProvider(String),
    /// Provider display name or API key env var does not match canonical metadata.
    #[error("provider metadata mismatch for key: {0}")]
    ProviderMetadataMismatch(String),
    /// Discovery status is missing for a provider.
    #[error("missing provider discovery status: {0}")]
    MissingProviderDiscoveryStatus(String),
    /// Discovery status references a provider that is not in the catalog.
    #[error("unknown discovery status provider: {0}")]
    UnknownDiscoveryStatusProvider(String),
    /// Discovery status appears more than once for a provider.
    #[error("duplicate provider discovery status: {0}")]
    DuplicateProviderDiscoveryStatus(String),
    /// Fallback-backed provider metadata lacks a user-facing warning.
    #[error("fallback metadata lacks a user-facing warning: {0}")]
    MissingFallbackWarning(String),
    /// A model references a provider absent from the provider list.
    #[error("model references unknown provider: {0}")]
    UnknownModelProvider(String),
    /// A provider/model pair appears more than once.
    #[error("duplicate model entry: {provider_key}/{model_id}")]
    DuplicateModel {
        /// Provider key for the duplicate model entry.
        provider_key: String,
        /// Model identifier for the duplicate model entry.
        model_id: String,
    },
    /// A model identifier is empty.
    #[error("empty model identifier for provider: {0}")]
    EmptyModelId(String),
    /// A routing score is outside the accepted 0-100 range.
    #[error("invalid routing score {field}={value} for {provider_key}/{model_id}")]
    InvalidRoutingScore {
        /// Score field name.
        field: &'static str,
        /// Provider key for the model.
        provider_key: String,
        /// Model identifier.
        model_id: String,
        /// Invalid score value.
        value: u8,
    },
}

/// Validates a refreshed v1 catalog document.
///
/// Returns all validation errors so publisher and client surfaces can report
/// complete safe diagnostics without adopting invalid catalog content.
pub fn validate_catalog_document_v1(
    document: &ModelCatalogDocumentV1,
) -> Result<(), Vec<ModelCatalogValidationError>> {
    let mut errors = Vec::new();

    if document.schema_version != MODEL_CATALOG_V1_SCHEMA_VERSION {
        errors.push(ModelCatalogValidationError::UnsupportedSchemaVersion(
            document.schema_version.clone(),
        ));
    }
    push_empty_field_errors(document, &mut errors);

    let provider_keys = validate_catalog_providers(document, &mut errors);
    validate_discovery_status(document, &provider_keys, &mut errors);
    validate_catalog_models(document, &provider_keys, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn push_empty_field_errors(
    document: &ModelCatalogDocumentV1,
    errors: &mut Vec<ModelCatalogValidationError>,
) {
    for (field, value) in [
        ("content_timestamp", document.content_timestamp.as_str()),
        ("source", document.source.as_str()),
        (
            "generator_status_summary",
            document.generator_status_summary.as_str(),
        ),
        ("change_summary", document.change_summary.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(ModelCatalogValidationError::EmptyRequiredField(field));
        }
    }
}

fn validate_catalog_providers(
    document: &ModelCatalogDocumentV1,
    errors: &mut Vec<ModelCatalogValidationError>,
) -> HashSet<String> {
    let mut provider_keys = HashSet::new();
    for provider in &document.providers {
        let key = provider.config_key.trim();
        if key == "openrouter" {
            errors.push(ModelCatalogValidationError::OpenRouterExcluded);
        }
        if key == "grok" {
            errors.push(ModelCatalogValidationError::GrokIsLegacyAlias);
        }
        let Some(metadata) = v1_provider_metadata_for_key(key) else {
            errors.push(ModelCatalogValidationError::UnsupportedProvider(
                provider.config_key.clone(),
            ));
            continue;
        };
        if !provider_keys.insert(key.to_string()) {
            errors.push(ModelCatalogValidationError::DuplicateProvider(
                key.to_string(),
            ));
        }
        if provider.display_name != metadata.display_name
            || provider.api_key_env != metadata.api_key_env
        {
            errors.push(ModelCatalogValidationError::ProviderMetadataMismatch(
                key.to_string(),
            ));
        }
    }
    for metadata in accepted_v1_provider_metadata() {
        if !provider_keys.contains(metadata.config_key) {
            errors.push(ModelCatalogValidationError::MissingProvider(
                metadata.config_key.to_string(),
            ));
        }
    }
    provider_keys
}

fn validate_discovery_status(
    document: &ModelCatalogDocumentV1,
    provider_keys: &HashSet<String>,
    errors: &mut Vec<ModelCatalogValidationError>,
) {
    let mut discovery_keys = HashSet::new();
    for status in &document.provider_discovery_status {
        if !provider_keys.contains(&status.provider_key) {
            errors.push(ModelCatalogValidationError::UnknownDiscoveryStatusProvider(
                status.provider_key.clone(),
            ));
        }
        if !discovery_keys.insert(status.provider_key.clone()) {
            errors.push(
                ModelCatalogValidationError::DuplicateProviderDiscoveryStatus(
                    status.provider_key.clone(),
                ),
            );
        }
        if matches!(
            status.state,
            ProviderDiscoveryState::CuratedFallback
                | ProviderDiscoveryState::PreviousKnownGoodFallback
        ) && status
            .warning
            .as_deref()
            .is_none_or(|warning| warning.trim().is_empty())
        {
            errors.push(ModelCatalogValidationError::MissingFallbackWarning(
                status.provider_key.clone(),
            ));
        }
    }

    for key in provider_keys {
        if !discovery_keys.contains(key) {
            errors.push(ModelCatalogValidationError::MissingProviderDiscoveryStatus(
                key.clone(),
            ));
        }
    }
}

fn validate_catalog_models(
    document: &ModelCatalogDocumentV1,
    provider_keys: &HashSet<String>,
    errors: &mut Vec<ModelCatalogValidationError>,
) {
    let mut model_keys = HashSet::new();
    for model in &document.models {
        if !provider_keys.contains(&model.provider_key) {
            errors.push(ModelCatalogValidationError::UnknownModelProvider(
                model.provider_key.clone(),
            ));
        }
        if model.model_id.trim().is_empty() {
            errors.push(ModelCatalogValidationError::EmptyModelId(
                model.provider_key.clone(),
            ));
        }
        if !model_keys.insert((model.provider_key.clone(), model.model_id.clone())) {
            errors.push(ModelCatalogValidationError::DuplicateModel {
                provider_key: model.provider_key.clone(),
                model_id: model.model_id.clone(),
            });
        }
        push_invalid_score_error(
            "quality",
            model.quality,
            &model.provider_key,
            &model.model_id,
            errors,
        );
        push_invalid_score_error(
            "speed",
            model.speed,
            &model.provider_key,
            &model.model_id,
            errors,
        );
        push_invalid_score_error(
            "cost_efficiency",
            model.cost_efficiency,
            &model.provider_key,
            &model.model_id,
            errors,
        );
    }
}

fn push_invalid_score_error(
    field: &'static str,
    value: u8,
    provider_key: &str,
    model_id: &str,
    errors: &mut Vec<ModelCatalogValidationError>,
) {
    if value > 100 {
        errors.push(ModelCatalogValidationError::InvalidRoutingScore {
            field,
            provider_key: provider_key.to_string(),
            model_id: model_id.to_string(),
            value,
        });
    }
}

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
    matches!(provider, &ProviderKind::Grok) && model == RETIRED_GROK_CODE_FAST_1
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

    fn valid_catalog_document() -> ModelCatalogDocumentV1 {
        ModelCatalogDocumentV1 {
            schema_version: MODEL_CATALOG_V1_SCHEMA_VERSION.to_string(),
            content_timestamp: "2026-06-08T00:00:00Z".to_string(),
            source: "fixture-input@abc123".to_string(),
            generator_status_summary: "all providers validated from curated fixture".to_string(),
            providers: accepted_v1_provider_metadata()
                .iter()
                .map(|provider| CatalogProviderDocument {
                    display_name: provider.display_name.to_string(),
                    config_key: provider.config_key.to_string(),
                    api_key_env: provider.api_key_env.to_string(),
                    note: None,
                })
                .collect(),
            provider_discovery_status: accepted_v1_provider_metadata()
                .iter()
                .map(|provider| ProviderDiscoveryStatus {
                    provider_key: provider.config_key.to_string(),
                    state: ProviderDiscoveryState::FreshDiscovery,
                    warning: None,
                })
                .collect(),
            models: accepted_v1_provider_metadata()
                .iter()
                .map(|provider| ModelCatalogDocumentEntry {
                    provider_key: provider.config_key.to_string(),
                    model_id: format!("{}-fixture-model", provider.config_key),
                    lifecycle: ModelLifecycle::Active,
                    quality: 80,
                    speed: 80,
                    cost_efficiency: 80,
                    reasoning: true,
                    coding: true,
                })
                .collect(),
            change_summary: "fixture catalog for validation".to_string(),
        }
    }

    #[test]
    fn accepted_v1_provider_metadata_matches_product_spec() {
        let providers = accepted_v1_provider_metadata();

        assert_eq!(providers.len(), 9);
        assert!(providers.iter().any(|provider| {
            provider.display_name == "xAI"
                && provider.config_key == "xai"
                && provider.api_key_env == "XAI_API_KEY"
        }));
        assert!(providers.iter().any(|provider| {
            provider.display_name == "Alibaba Cloud Model Studio (Qwen)"
                && provider.config_key == "qwen"
                && provider.api_key_env == "DASHSCOPE_API_KEY"
        }));
        assert!(
            !providers
                .iter()
                .any(|provider| provider.config_key == "openrouter")
        );
        assert_eq!(legacy_provider_alias_canonical_key("grok"), Some("xai"));
    }

    #[test]
    fn validate_catalog_document_accepts_valid_v1_catalog() {
        let document = valid_catalog_document();

        assert!(validate_catalog_document_v1(&document).is_ok());
    }

    #[test]
    fn validate_catalog_document_rejects_openrouter() {
        let mut document = valid_catalog_document();
        document.providers.push(CatalogProviderDocument {
            display_name: "OpenRouter".to_string(),
            config_key: "openrouter".to_string(),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            note: None,
        });

        let errors = validate_catalog_document_v1(&document).expect_err("catalog must fail");

        assert!(errors.contains(&ModelCatalogValidationError::OpenRouterExcluded));
    }

    #[test]
    fn validate_catalog_document_rejects_grok_as_canonical_key() {
        let mut document = valid_catalog_document();
        document.providers.push(CatalogProviderDocument {
            display_name: "xAI (Grok)".to_string(),
            config_key: "grok".to_string(),
            api_key_env: "XAI_API_KEY".to_string(),
            note: None,
        });

        let errors = validate_catalog_document_v1(&document).expect_err("catalog must fail");

        assert!(errors.contains(&ModelCatalogValidationError::GrokIsLegacyAlias));
    }

    #[test]
    fn validate_catalog_document_requires_every_accepted_provider() {
        let mut document = valid_catalog_document();
        document
            .providers
            .retain(|provider| provider.config_key != "gemini");

        let errors = validate_catalog_document_v1(&document).expect_err("catalog must fail");

        assert!(
            errors.contains(&ModelCatalogValidationError::MissingProvider(
                "gemini".to_string()
            ))
        );
    }

    #[test]
    fn validate_catalog_document_requires_discovery_status_for_each_provider() {
        let mut document = valid_catalog_document();
        document
            .provider_discovery_status
            .retain(|status| status.provider_key != "xai");

        let errors = validate_catalog_document_v1(&document).expect_err("catalog must fail");

        assert!(errors.contains(
            &ModelCatalogValidationError::MissingProviderDiscoveryStatus("xai".to_string())
        ));
    }

    #[test]
    fn validate_catalog_document_requires_warning_for_fallback_metadata() {
        let mut document = valid_catalog_document();
        let status = document
            .provider_discovery_status
            .iter_mut()
            .find(|status| status.provider_key == "minimax")
            .expect("invariant: minimax status exists");
        status.state = ProviderDiscoveryState::PreviousKnownGoodFallback;
        status.warning = None;

        let errors = validate_catalog_document_v1(&document).expect_err("catalog must fail");

        assert!(
            errors.contains(&ModelCatalogValidationError::MissingFallbackWarning(
                "minimax".to_string()
            ))
        );
    }

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
