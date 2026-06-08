//! Deterministic provider model-catalog publisher primitives.
//!
//! This module builds publishable v1 catalog bytes from curated provider/model
//! metadata. Operational workflow evidence is returned separately so reruns do
//! not change the semantic catalog hash.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::agents::model_catalog::{
    CatalogProviderDocument, ModelCatalogDocumentEntry, ModelCatalogDocumentV1,
    ModelCatalogValidationError, ProviderDiscoveryState, ProviderDiscoveryStatus,
    accepted_v1_provider_metadata, catalog_sha256_hex, deterministic_catalog_bytes,
    validate_catalog_document_v1,
};

/// Curated input for deterministic v1 model-catalog generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPublisherInput {
    /// Semantic catalog content timestamp.
    pub content_timestamp: String,
    /// Source and curation provenance for reviewer inspection.
    pub source: String,
    /// Safe summary of generator status for the adopted catalog bytes.
    pub generator_status_summary: String,
    /// Concise user-facing change summary.
    pub change_summary: String,
    /// Per-provider discovery/fallback inputs.
    pub provider_discovery: Vec<CatalogPublisherProviderDiscovery>,
    /// Provider/model routing entries.
    pub models: Vec<ModelCatalogDocumentEntry>,
    /// Run-specific evidence that must not affect the adopted catalog bytes.
    #[serde(default)]
    pub run_evidence: CatalogPublisherRunEvidence,
}

/// Curated discovery input for one provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPublisherProviderDiscovery {
    /// Canonical v1 provider config key.
    pub provider_key: String,
    /// Discovery or fallback state from the generator input.
    pub state: CatalogPublisherDiscoveryState,
    /// User-facing warning required for fallback-backed metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Discovery state accepted by publisher input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogPublisherDiscoveryState {
    /// Provider metadata came from fresh discovery for this semantic catalog.
    FreshDiscovery,
    /// Provider metadata came from curated fallback input.
    CuratedFallback,
    /// Provider metadata came from previous-known-good fallback input.
    PreviousKnownGoodFallback,
    /// Provider metadata was manually curated without live discovery.
    ManuallyCurated,
    /// Provider discovery failed and no valid fallback metadata is available.
    Unavailable,
}

/// Run-specific publisher evidence excluded from deterministic catalog bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPublisherRunEvidence {
    /// Time this generator run executed, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at_unix_secs: Option<u64>,
    /// Workflow run URL or equivalent operational evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_url: Option<String>,
    /// Operational warnings from this run.
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Deterministic publisher output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedModelCatalog {
    /// Validated catalog document.
    pub document: ModelCatalogDocumentV1,
    /// Deterministic pretty JSON bytes for the document.
    pub catalog_bytes: Vec<u8>,
    /// SHA-256 hex digest of `catalog_bytes`.
    pub sha256: String,
    /// Standard checksum-file body for `model-catalog.v1.sha256`.
    pub sha256_file_bytes: Vec<u8>,
    /// Run-specific evidence kept outside the adopted catalog bytes.
    pub run_evidence: CatalogPublisherRunEvidence,
}

/// Errors from deterministic catalog publication.
#[derive(Debug, Error)]
pub enum CatalogPublisherError {
    /// Provider discovery input references OpenRouter, which is excluded from v1.
    #[error("OpenRouter is excluded from v1 catalog publication")]
    OpenRouterExcluded,
    /// Provider discovery input uses the legacy grok alias instead of xai.
    #[error("grok is compatibility-only; publisher input must use xai")]
    GrokIsLegacyAlias,
    /// Provider discovery input references an unsupported provider key.
    #[error("unsupported publisher provider key: {0}")]
    UnsupportedProvider(String),
    /// Provider discovery input appears more than once.
    #[error("duplicate publisher provider discovery input: {0}")]
    DuplicateProviderDiscovery(String),
    /// Required provider discovery input is missing.
    #[error("missing publisher provider discovery input: {0}")]
    MissingProviderDiscovery(String),
    /// Provider discovery failed without valid fallback metadata.
    #[error("provider discovery unavailable without valid fallback: {0}")]
    ProviderDiscoveryUnavailable(String),
    /// Accepted provider lacks at least one catalog model.
    #[error("missing model entries for provider: {0}")]
    MissingProviderModels(String),
    /// Catalog schema validation failed after generation.
    #[error("generated catalog validation failed")]
    Validation {
        /// Collected validation errors.
        errors: Vec<ModelCatalogValidationError>,
    },
    /// JSON serialization failed.
    #[error("catalog publisher JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Builds deterministic v1 catalog bytes and checksum from curated input.
///
/// # Errors
///
/// Returns an error when provider discovery input is incomplete or invalid, no
/// valid fallback exists, model coverage is incomplete, serialization fails, or
/// the generated v1 catalog does not pass schema validation.
pub fn publish_model_catalog_v1(
    input: &CatalogPublisherInput,
) -> Result<PublishedModelCatalog, CatalogPublisherError> {
    let discovery_status = build_discovery_status(input)?;
    let mut models = input.models.clone();
    models.sort_by(|left, right| {
        left.provider_key
            .cmp(&right.provider_key)
            .then_with(|| left.model_id.cmp(&right.model_id))
    });
    require_model_coverage(&models)?;

    let document = ModelCatalogDocumentV1 {
        schema_version: crate::agents::model_catalog::MODEL_CATALOG_V1_SCHEMA_VERSION.to_string(),
        content_timestamp: input.content_timestamp.clone(),
        source: input.source.clone(),
        generator_status_summary: input.generator_status_summary.clone(),
        providers: accepted_v1_provider_metadata()
            .iter()
            .map(|metadata| CatalogProviderDocument {
                display_name: metadata.display_name.to_string(),
                config_key: metadata.config_key.to_string(),
                api_key_env: metadata.api_key_env.to_string(),
                note: None,
            })
            .collect(),
        provider_discovery_status: discovery_status,
        models,
        change_summary: input.change_summary.clone(),
    };

    validate_catalog_document_v1(&document)
        .map_err(|errors| CatalogPublisherError::Validation { errors })?;
    let catalog_bytes = deterministic_catalog_bytes(&document)?;
    let sha256 = catalog_sha256_hex(&catalog_bytes);
    let sha256_file_bytes = format!("{sha256}  model-catalog.v1.json\n").into_bytes();

    Ok(PublishedModelCatalog {
        document,
        catalog_bytes,
        sha256,
        sha256_file_bytes,
        run_evidence: input.run_evidence.clone(),
    })
}

fn build_discovery_status(
    input: &CatalogPublisherInput,
) -> Result<Vec<ProviderDiscoveryStatus>, CatalogPublisherError> {
    let mut by_key = HashMap::new();
    for discovery in &input.provider_discovery {
        match discovery.provider_key.as_str() {
            "openrouter" => return Err(CatalogPublisherError::OpenRouterExcluded),
            "grok" => return Err(CatalogPublisherError::GrokIsLegacyAlias),
            key if accepted_v1_provider_metadata()
                .iter()
                .all(|provider| provider.config_key != key) =>
            {
                return Err(CatalogPublisherError::UnsupportedProvider(
                    discovery.provider_key.clone(),
                ));
            }
            _ => {}
        }
        if by_key
            .insert(discovery.provider_key.as_str(), discovery)
            .is_some()
        {
            return Err(CatalogPublisherError::DuplicateProviderDiscovery(
                discovery.provider_key.clone(),
            ));
        }
    }

    accepted_v1_provider_metadata()
        .iter()
        .map(|metadata| {
            let discovery = by_key.get(metadata.config_key).ok_or_else(|| {
                CatalogPublisherError::MissingProviderDiscovery(metadata.config_key.to_string())
            })?;
            let state = match discovery.state {
                CatalogPublisherDiscoveryState::FreshDiscovery => {
                    ProviderDiscoveryState::FreshDiscovery
                }
                CatalogPublisherDiscoveryState::CuratedFallback => {
                    ProviderDiscoveryState::CuratedFallback
                }
                CatalogPublisherDiscoveryState::PreviousKnownGoodFallback => {
                    ProviderDiscoveryState::PreviousKnownGoodFallback
                }
                CatalogPublisherDiscoveryState::ManuallyCurated => {
                    ProviderDiscoveryState::ManuallyCurated
                }
                CatalogPublisherDiscoveryState::Unavailable => {
                    return Err(CatalogPublisherError::ProviderDiscoveryUnavailable(
                        metadata.config_key.to_string(),
                    ));
                }
            };
            Ok(ProviderDiscoveryStatus {
                provider_key: metadata.config_key.to_string(),
                state,
                warning: discovery.warning.clone(),
            })
        })
        .collect()
}

fn require_model_coverage(
    models: &[ModelCatalogDocumentEntry],
) -> Result<(), CatalogPublisherError> {
    let providers_with_models = models
        .iter()
        .map(|model| model.provider_key.as_str())
        .collect::<HashSet<_>>();
    for metadata in accepted_v1_provider_metadata() {
        if !providers_with_models.contains(metadata.config_key) {
            return Err(CatalogPublisherError::MissingProviderModels(
                metadata.config_key.to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> CatalogPublisherInput {
        let body = match name {
            "valid" => {
                include_str!("../../tests/fixtures/model_catalog/publisher_valid.json")
            }
            "fallback" => {
                include_str!("../../tests/fixtures/model_catalog/publisher_fallback.json")
            }
            "unavailable" => {
                include_str!("../../tests/fixtures/model_catalog/publisher_unavailable.json")
            }
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(body).expect("publisher fixture parses")
    }

    #[test]
    fn publisher_generates_deterministic_catalog_and_checksum() {
        let input = fixture("valid");

        let first = publish_model_catalog_v1(&input).expect("publish succeeds");
        let second = publish_model_catalog_v1(&input).expect("publish succeeds");

        assert_eq!(first.catalog_bytes, second.catalog_bytes);
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(
            first.sha256_file_bytes,
            format!("{}  model-catalog.v1.json\n", first.sha256).into_bytes()
        );
        assert_eq!(first.document.providers.len(), 9);
        assert!(
            first
                .document
                .providers
                .iter()
                .any(|provider| provider.config_key == "xai")
        );
        assert!(
            !first
                .document
                .providers
                .iter()
                .any(|provider| provider.config_key == "openrouter")
        );
    }

    #[test]
    fn publisher_keeps_run_evidence_out_of_catalog_hash() {
        let mut first_input = fixture("valid");
        let mut second_input = first_input.clone();
        first_input.run_evidence.generated_at_unix_secs = Some(1_000);
        second_input.run_evidence.generated_at_unix_secs = Some(2_000);
        second_input
            .run_evidence
            .warnings
            .push("workflow retry happened".to_string());

        let first = publish_model_catalog_v1(&first_input).expect("publish succeeds");
        let second = publish_model_catalog_v1(&second_input).expect("publish succeeds");

        assert_eq!(first.catalog_bytes, second.catalog_bytes);
        assert_eq!(first.sha256, second.sha256);
        assert_ne!(first.run_evidence, second.run_evidence);
    }

    #[test]
    fn publisher_accepts_valid_fallback_with_user_warning() {
        let output = publish_model_catalog_v1(&fixture("fallback")).expect("publish succeeds");

        let qwen_status = output
            .document
            .provider_discovery_status
            .iter()
            .find(|status| status.provider_key == "qwen")
            .expect("qwen status exists");
        assert_eq!(
            qwen_status.state,
            ProviderDiscoveryState::PreviousKnownGoodFallback
        );
        assert!(
            qwen_status
                .warning
                .as_deref()
                .is_some_and(|warning| warning.contains("previous-known-good"))
        );
        assert!(validate_catalog_document_v1(&output.document).is_ok());
    }

    #[test]
    fn publisher_blocks_discovery_outage_without_valid_fallback() {
        let error =
            publish_model_catalog_v1(&fixture("unavailable")).expect_err("publish must fail");

        assert!(matches!(
            error,
            CatalogPublisherError::ProviderDiscoveryUnavailable(provider)
                if provider == "qwen"
        ));
    }

    #[test]
    fn publisher_rejects_openrouter_and_legacy_grok_inputs() {
        let mut input = fixture("valid");
        input
            .provider_discovery
            .push(CatalogPublisherProviderDiscovery {
                provider_key: "openrouter".to_string(),
                state: CatalogPublisherDiscoveryState::FreshDiscovery,
                warning: None,
            });
        assert!(matches!(
            publish_model_catalog_v1(&input),
            Err(CatalogPublisherError::OpenRouterExcluded)
        ));

        let mut input = fixture("valid");
        input.provider_discovery[0].provider_key = "grok".to_string();
        assert!(matches!(
            publish_model_catalog_v1(&input),
            Err(CatalogPublisherError::GrokIsLegacyAlias)
        ));
    }

    #[test]
    fn publisher_requires_model_coverage_for_every_provider() {
        let mut input = fixture("valid");
        input.models.retain(|model| model.provider_key != "gemini");

        let error = publish_model_catalog_v1(&input).expect_err("publish must fail");

        assert!(matches!(
            error,
            CatalogPublisherError::MissingProviderModels(provider)
                if provider == "gemini"
        ));
    }
}
