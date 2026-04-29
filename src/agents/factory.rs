//! Provider factory — creates [`LlmProvider`] instances from config.
//!
//! [`create_provider`] builds a single provider; [`create_provider_chain`]
//! builds a chain with fallback support.

use crate::agents::fallback::ProviderChain;
use crate::agents::model_access::ModelAccessStore;
use crate::agents::model_catalog::{self, ModelSelectionContext};
use crate::agents::{AgentError, LlmProvider};
use crate::config::{ProviderConfig, ProviderKind, ResolvedProviderConfig};

/// Creates a single [`LlmProvider`] from a [`ProviderConfig`].
///
/// Reads the API key from the configured environment variable.
///
/// # Errors
///
/// Returns an error if the API key env var is not set.
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LlmProvider>, AgentError> {
    create_provider_for_context(config, &ModelSelectionContext::default())
}

/// Creates a single [`LlmProvider`] using model routing context.
///
/// # Errors
///
/// Returns an error if the API key env var is not set.
pub fn create_provider_for_context(
    config: &ProviderConfig,
    context: &ModelSelectionContext,
) -> Result<Box<dyn LlmProvider>, AgentError> {
    let resolved = model_catalog::resolve_provider_config(config, context).ok_or_else(|| {
        AgentError::Parse(format!(
            "Cannot create {} provider: no allowed Duumbi model is available for this credential",
            config.provider
        ))
    })?;
    create_resolved_provider(&resolved)
}

/// Creates a provider using global model-access metadata when available.
///
/// Known-accessible models are preferred; known-denied models are excluded from
/// default routing for the active credential.
///
/// # Errors
///
/// Returns an error if provider construction fails or credentials are missing.
pub fn create_provider_for_global_access(
    config: &ProviderConfig,
) -> Result<Box<dyn LlmProvider>, AgentError> {
    create_provider_for_global_access_context(config, &ModelSelectionContext::default())
}

/// Creates a provider using global model-access metadata and call context.
///
/// # Errors
///
/// Returns an error if provider construction fails or credentials are missing.
pub fn create_provider_for_global_access_context(
    config: &ProviderConfig,
    context: &ModelSelectionContext,
) -> Result<Box<dyn LlmProvider>, AgentError> {
    let mut context = context.clone();
    if let Some(fingerprint) = crate::agents::model_access::credential_fingerprint_from_env(config)
    {
        context.accessible_models =
            ModelAccessStore::accessible_models(&config.provider, &fingerprint)
                .into_iter()
                .collect();
        context.denied_models = ModelAccessStore::denied_models(&config.provider, &fingerprint)
            .into_iter()
            .collect();
    }
    create_provider_for_context(config, &context)
}

fn create_resolved_provider(
    config: &ResolvedProviderConfig,
) -> Result<Box<dyn LlmProvider>, AgentError> {
    // Resolve credential: prefer auth_token_env (subscription) over api_key_env.
    let (api_key, use_auth_token) = if let Some(ref token_env) = config.auth_token_env {
        if let Ok(token) = std::env::var(token_env) {
            (token, true)
        } else {
            let key = resolve_api_key(config)?;
            (key, false)
        }
    } else {
        let key = resolve_api_key(config)?;
        (key, false)
    };

    create_provider_with_api_key(config, api_key, use_auth_token)
}

fn resolve_api_key(config: &ResolvedProviderConfig) -> Result<String, AgentError> {
    std::env::var(&config.api_key_env).map_err(|_| {
        AgentError::Parse(format!(
            "Cannot create {} provider: Config field 'api_key_env' is invalid: Environment variable '{}' is not set",
            config.provider, config.api_key_env
        ))
    })
}

/// Creates a single [`LlmProvider`] from explicit credential material.
///
/// This is used for connection probes before a key is persisted.
///
/// # Errors
///
/// Returns an error if provider construction fails.
pub fn create_provider_with_api_key(
    config: &ResolvedProviderConfig,
    api_key: impl Into<String>,
    use_auth_token: bool,
) -> Result<Box<dyn LlmProvider>, AgentError> {
    let api_key = api_key.into();
    let provider: Box<dyn LlmProvider> = match config.provider {
        ProviderKind::Anthropic => Box::new(
            super::anthropic::AnthropicClient::new(&config.model, &api_key)
                .with_bearer_auth(use_auth_token),
        ),
        ProviderKind::OpenAI => {
            // Reuse the already-resolved api_key to avoid a redundant env var read.
            let client = if let Some(ref url) = config.base_url {
                super::openai::OpenAiClient::with_base_url(&config.model, api_key, url)
            } else {
                super::openai::OpenAiClient::new(&config.model, api_key)
            };
            Box::new(client)
        }
        ProviderKind::Grok => {
            // Honor base_url override if configured (e.g. a custom xAI proxy).
            if let Some(ref url) = config.base_url {
                Box::new(
                    super::openai::OpenAiClient::with_base_url(&config.model, api_key, url)
                        .with_provider_name("grok"),
                )
            } else {
                Box::new(super::grok::GrokClient::new(&config.model, api_key))
            }
        }
        ProviderKind::OpenRouter => {
            if config.base_url.is_some() {
                tracing::warn!(
                    "base_url is not supported for the OpenRouter provider \
                     (required attribution headers would be lost); \
                     base_url is ignored"
                );
            }
            Box::new(super::openrouter::OpenRouterClient::new(
                &config.model,
                api_key,
            ))
        }
        ProviderKind::MiniMax => {
            if let Some(ref url) = config.base_url {
                Box::new(
                    super::openai::OpenAiClient::with_base_url(&config.model, api_key, url)
                        .with_provider_name("minimax"),
                )
            } else {
                Box::new(super::minimax::MiniMaxClient::new(&config.model, api_key))
            }
        }
    };

    if config.timeout_secs.is_some() {
        tracing::warn!(
            provider = %config.provider,
            "timeout_secs is not yet implemented for this provider and will be ignored"
        );
    }

    Ok(provider)
}

/// Creates a provider or provider chain from a list of configs.
///
/// If only one config is provided, returns that provider directly.
/// If multiple are provided, wraps them in a [`ProviderChain`] with
/// fallback support (primary first, then fallbacks in order).
///
/// # Errors
///
/// Returns an error if any API key env var is not set.
pub fn create_provider_chain(
    configs: &[ProviderConfig],
) -> Result<Box<dyn LlmProvider>, AgentError> {
    create_provider_chain_for_context(configs, &ModelSelectionContext::default())
}

/// Creates a provider chain using model routing context.
///
/// # Errors
///
/// Returns an error if any API key env var is not set.
pub fn create_provider_chain_for_context(
    configs: &[ProviderConfig],
    context: &ModelSelectionContext,
) -> Result<Box<dyn LlmProvider>, AgentError> {
    if configs.is_empty() {
        return Err(AgentError::Parse(
            "No provider configurations provided".to_string(),
        ));
    }

    if configs.len() == 1 {
        return create_provider_for_context(&configs[0], context);
    }

    let sorted = sorted_provider_configs(configs);
    let mut providers = Vec::with_capacity(sorted.len());
    for config in &sorted {
        providers.push(create_provider_for_context(config, context)?);
    }

    Ok(Box::new(ProviderChain::new(providers)))
}

/// Creates a provider chain using global model-access metadata.
///
/// # Errors
///
/// Returns an error if no providers are configured or provider construction fails.
pub fn create_provider_chain_for_global_access(
    configs: &[ProviderConfig],
) -> Result<Box<dyn LlmProvider>, AgentError> {
    if configs.is_empty() {
        return Err(AgentError::Parse(
            "No LLM providers configured. Run `duumbi provider add`.".to_string(),
        ));
    }

    if configs.len() == 1 {
        return create_provider_for_global_access(&configs[0]);
    }

    let sorted = sorted_provider_configs(configs);
    let mut providers = Vec::with_capacity(sorted.len());
    for config in &sorted {
        providers.push(create_provider_for_global_access(config)?);
    }
    Ok(Box::new(super::fallback::ProviderChain::new(providers)))
}

fn sorted_provider_configs(configs: &[ProviderConfig]) -> Vec<&ProviderConfig> {
    let mut sorted: Vec<&ProviderConfig> = configs.iter().collect();
    sorted.sort_by_key(|config| match config.role {
        crate::config::ProviderRole::Primary => 0,
        crate::config::ProviderRole::Fallback => 1,
    });
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderConfig, ProviderKind, ProviderRole};

    fn make_config_with_env(
        kind: ProviderKind,
        role: ProviderRole,
        env_var: &str,
    ) -> ProviderConfig {
        ProviderConfig {
            provider: kind,
            role,
            model: Some("test-model".to_string()),
            api_key_env: env_var.to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        }
    }

    #[test]
    fn create_provider_anthropic() {
        // Use a unique env var per test to avoid races with parallel test cleanup.
        // SAFETY: test-only env var, unique name prevents collision
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_ANTHROPIC", "sk-test") };
        let config = make_config_with_env(
            ProviderKind::Anthropic,
            ProviderRole::Primary,
            "DUUMBI_TEST_FACTORY_ANTHROPIC",
        );
        let provider = create_provider(&config).expect("must create");
        assert_eq!(provider.name(), "anthropic");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_ANTHROPIC") };
    }

    #[test]
    fn create_provider_grok() {
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_GROK", "sk-test") };
        let config = make_config_with_env(
            ProviderKind::Grok,
            ProviderRole::Primary,
            "DUUMBI_TEST_FACTORY_GROK",
        );
        let provider = create_provider(&config).expect("must create");
        assert_eq!(provider.name(), "grok");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_GROK") };
    }

    #[test]
    fn create_provider_openrouter() {
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_OPENROUTER", "sk-test") };
        let config = make_config_with_env(
            ProviderKind::OpenRouter,
            ProviderRole::Primary,
            "DUUMBI_TEST_FACTORY_OPENROUTER",
        );
        let provider = create_provider(&config).expect("must create");
        assert_eq!(provider.name(), "openrouter");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_OPENROUTER") };
    }

    #[test]
    fn create_provider_missing_key_returns_error() {
        let config = ProviderConfig {
            provider: ProviderKind::Anthropic,
            role: ProviderRole::Primary,
            model: Some("test".to_string()),
            api_key_env: "DUUMBI_DEFINITELY_NOT_SET_FACTORY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        };
        match create_provider(&config) {
            Err(AgentError::Parse(_)) => {} // expected
            Err(other) => panic!("Expected Parse error, got: {other}"),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn create_chain_single_provider() {
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_SINGLE", "sk-test") };
        let configs = vec![make_config_with_env(
            ProviderKind::Anthropic,
            ProviderRole::Primary,
            "DUUMBI_TEST_FACTORY_SINGLE",
        )];
        let provider = create_provider_chain(&configs).expect("must create");
        assert_eq!(provider.name(), "anthropic");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_SINGLE") };
    }

    #[test]
    fn create_chain_empty_returns_error() {
        match create_provider_chain(&[]) {
            Err(AgentError::Parse(_)) => {} // expected
            Err(other) => panic!("Expected Parse error, got: {other}"),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn create_chain_multi_provider() {
        // Use a unique env var to avoid race with parallel test cleanup
        unsafe { std::env::set_var("DUUMBI_TEST_CHAIN_MULTI_KEY", "sk-test") };
        let make = |kind, role| ProviderConfig {
            provider: kind,
            role,
            model: Some("test-model".to_string()),
            api_key_env: "DUUMBI_TEST_CHAIN_MULTI_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        };
        let configs = vec![
            make(ProviderKind::Anthropic, ProviderRole::Primary),
            make(ProviderKind::Grok, ProviderRole::Fallback),
        ];
        let provider = create_provider_chain(&configs).expect("must create");
        // Chain's name is the primary's name
        assert_eq!(provider.name(), "anthropic");
        unsafe { std::env::remove_var("DUUMBI_TEST_CHAIN_MULTI_KEY") };
    }

    #[test]
    fn create_global_access_chain_keeps_primary_before_fallback() {
        unsafe { std::env::set_var("DUUMBI_TEST_GLOBAL_CHAIN_KEY", "sk-test") };
        let make = |kind, role| ProviderConfig {
            provider: kind,
            role,
            model: Some("test-model".to_string()),
            api_key_env: "DUUMBI_TEST_GLOBAL_CHAIN_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        };
        let configs = vec![
            make(ProviderKind::Anthropic, ProviderRole::Fallback),
            make(ProviderKind::Grok, ProviderRole::Primary),
        ];
        let provider = create_provider_chain_for_global_access(&configs).expect("must create");

        assert_eq!(provider.name(), "grok");
        unsafe { std::env::remove_var("DUUMBI_TEST_GLOBAL_CHAIN_KEY") };
    }
}
