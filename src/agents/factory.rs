//! Provider factory — creates [`LlmProvider`] instances from config.
//!
//! [`create_provider`] builds a single provider; [`create_provider_chain`]
//! builds a chain with fallback support.

use crate::agents::fallback::ProviderChain;
use crate::agents::{AgentError, LlmProvider};
use crate::config::{ProviderConfig, ProviderKind};

/// Creates a single [`LlmProvider`] from a [`ProviderConfig`].
///
/// Reads the API key from the configured environment variable.
///
/// # Errors
///
/// Returns an error if the API key env var is not set.
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LlmProvider>, AgentError> {
    let api_key = config.resolve_api_key().map_err(|e| {
        AgentError::Parse(format!("Cannot create {} provider: {e}", config.provider))
    })?;

    let provider: Box<dyn LlmProvider> = match config.provider {
        ProviderKind::Anthropic => Box::new(super::anthropic::AnthropicClient::new(
            &config.model,
            api_key,
        )),
        ProviderKind::OpenAI => {
            let mut client = super::openai::OpenAiClient::new(&config.model, api_key);
            if let Some(ref url) = config.base_url {
                client = super::openai::OpenAiClient::with_base_url(
                    &config.model,
                    config
                        .resolve_api_key()
                        .map_err(|e| AgentError::Parse(format!("Cannot resolve API key: {e}")))?,
                    url,
                );
            }
            Box::new(client)
        }
        ProviderKind::Grok => Box::new(super::grok::GrokClient::new(&config.model, api_key)),
        ProviderKind::OpenRouter => Box::new(super::openrouter::OpenRouterClient::new(
            &config.model,
            api_key,
        )),
    };

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
    if configs.is_empty() {
        return Err(AgentError::Parse(
            "No provider configurations provided".to_string(),
        ));
    }

    if configs.len() == 1 {
        return create_provider(&configs[0]);
    }

    // Sort: primaries first, then fallbacks (stable order within each group)
    let mut sorted: Vec<&ProviderConfig> = configs.iter().collect();
    sorted.sort_by_key(|c| match c.role {
        crate::config::ProviderRole::Primary => 0,
        crate::config::ProviderRole::Fallback => 1,
    });

    let mut providers = Vec::with_capacity(sorted.len());
    for config in &sorted {
        providers.push(create_provider(config)?);
    }

    Ok(Box::new(ProviderChain::new(providers)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderConfig, ProviderKind, ProviderRole};

    fn make_config(kind: ProviderKind, role: ProviderRole) -> ProviderConfig {
        ProviderConfig {
            provider: kind,
            role,
            model: "test-model".to_string(),
            api_key_env: "DUUMBI_TEST_FACTORY_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
        }
    }

    #[test]
    fn create_provider_anthropic() {
        // SAFETY: test-only env var
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_KEY", "sk-test") };
        let config = make_config(ProviderKind::Anthropic, ProviderRole::Primary);
        let provider = create_provider(&config).expect("must create");
        assert_eq!(provider.name(), "anthropic");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_KEY") };
    }

    #[test]
    fn create_provider_grok() {
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_KEY", "sk-test") };
        let config = make_config(ProviderKind::Grok, ProviderRole::Primary);
        let provider = create_provider(&config).expect("must create");
        assert_eq!(provider.name(), "grok");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_KEY") };
    }

    #[test]
    fn create_provider_openrouter() {
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_KEY", "sk-test") };
        let config = make_config(ProviderKind::OpenRouter, ProviderRole::Primary);
        let provider = create_provider(&config).expect("must create");
        assert_eq!(provider.name(), "openrouter");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_KEY") };
    }

    #[test]
    fn create_provider_missing_key_returns_error() {
        let config = ProviderConfig {
            provider: ProviderKind::Anthropic,
            role: ProviderRole::Primary,
            model: "test".to_string(),
            api_key_env: "DUUMBI_DEFINITELY_NOT_SET_FACTORY".to_string(),
            base_url: None,
            timeout_secs: None,
        };
        match create_provider(&config) {
            Err(AgentError::Parse(_)) => {} // expected
            Err(other) => panic!("Expected Parse error, got: {other}"),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn create_chain_single_provider() {
        unsafe { std::env::set_var("DUUMBI_TEST_FACTORY_KEY", "sk-test") };
        let configs = vec![make_config(ProviderKind::Anthropic, ProviderRole::Primary)];
        let provider = create_provider_chain(&configs).expect("must create");
        assert_eq!(provider.name(), "anthropic");
        unsafe { std::env::remove_var("DUUMBI_TEST_FACTORY_KEY") };
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
            model: "test-model".to_string(),
            api_key_env: "DUUMBI_TEST_CHAIN_MULTI_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
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
}
