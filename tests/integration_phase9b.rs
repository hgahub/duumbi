//! Phase 9B integration tests — multi-LLM providers.
//!
//! Tests config parsing, factory creation, fallback chain behavior,
//! and provider prompt tuning. No live API calls.

use duumbi::agents::AgentError;
use duumbi::agents::fallback::ProviderChain;
use duumbi::agents::prompts::provider_prompt_suffix;
use duumbi::config::{
    DuumbiConfig, LlmConfig, LlmProvider as ConfigLlmProvider, ProviderConfig, ProviderKind,
    ProviderRole,
};

// ---------------------------------------------------------------------------
// AgentError::is_transient
// ---------------------------------------------------------------------------

#[test]
fn agent_error_http_is_transient() {
    // reqwest errors are transient (network issues)
    let err = AgentError::Timeout(30);
    assert!(err.is_transient());
}

#[test]
fn agent_error_rate_limited_is_transient() {
    let err = AgentError::RateLimited {
        retry_after: Some(60),
    };
    assert!(err.is_transient());
}

#[test]
fn agent_error_server_500_is_transient() {
    let err = AgentError::ApiError {
        status: 500,
        body: "internal server error".to_string(),
    };
    assert!(err.is_transient());
}

#[test]
fn agent_error_429_is_transient() {
    let err = AgentError::ApiError {
        status: 429,
        body: "too many requests".to_string(),
    };
    assert!(err.is_transient());
}

#[test]
fn agent_error_401_is_not_transient() {
    let err = AgentError::ApiError {
        status: 401,
        body: "unauthorized".to_string(),
    };
    assert!(!err.is_transient());
}

#[test]
fn agent_error_403_is_not_transient() {
    let err = AgentError::ApiError {
        status: 403,
        body: "forbidden".to_string(),
    };
    assert!(!err.is_transient());
}

#[test]
fn agent_error_400_is_not_transient() {
    let err = AgentError::ApiError {
        status: 400,
        body: "bad request".to_string(),
    };
    assert!(!err.is_transient());
}

#[test]
fn agent_error_parse_is_not_transient() {
    let err = AgentError::Parse("invalid json".to_string());
    assert!(!err.is_transient());
}

#[test]
fn agent_error_no_tool_calls_is_not_transient() {
    let err = AgentError::NoToolCalls;
    assert!(!err.is_transient());
}

// ---------------------------------------------------------------------------
// Config: [[providers]] parsing
// ---------------------------------------------------------------------------

#[test]
fn config_providers_section_parses() {
    let toml_str = r#"
[[providers]]
provider = "anthropic"
role = "primary"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"

[[providers]]
provider = "grok"
role = "fallback"
model = "grok-3"
api_key_env = "XAI_API_KEY"
"#;

    let cfg: DuumbiConfig = toml::from_str(toml_str).expect("must parse");
    assert_eq!(cfg.providers.len(), 2);
    assert_eq!(cfg.providers[0].provider, ProviderKind::Anthropic);
    assert_eq!(cfg.providers[0].role, ProviderRole::Primary);
    assert_eq!(cfg.providers[0].model, "claude-sonnet-4-6");
    assert_eq!(cfg.providers[1].provider, ProviderKind::Grok);
    assert_eq!(cfg.providers[1].role, ProviderRole::Fallback);
}

#[test]
fn config_providers_openrouter() {
    let toml_str = r#"
[[providers]]
provider = "openrouter"
model = "meta-llama/llama-3.1-405b-instruct"
api_key_env = "OPENROUTER_API_KEY"
"#;

    let cfg: DuumbiConfig = toml::from_str(toml_str).expect("must parse");
    assert_eq!(cfg.providers[0].provider, ProviderKind::OpenRouter);
    assert_eq!(cfg.providers[0].role, ProviderRole::Primary); // default
}

#[test]
fn config_providers_with_base_url_and_timeout() {
    let toml_str = r#"
[[providers]]
provider = "openai"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"
base_url = "https://custom.endpoint/v1/chat/completions"
timeout_secs = 60
"#;

    let cfg: DuumbiConfig = toml::from_str(toml_str).expect("must parse");
    assert_eq!(
        cfg.providers[0].base_url.as_deref(),
        Some("https://custom.endpoint/v1/chat/completions")
    );
    assert_eq!(cfg.providers[0].timeout_secs, Some(60));
}

#[test]
fn config_empty_providers_is_valid() {
    let toml_str = r#"
[workspace]
name = "test"
"#;

    let cfg: DuumbiConfig = toml::from_str(toml_str).expect("must parse");
    assert!(cfg.providers.is_empty());
}

// ---------------------------------------------------------------------------
// Config: effective_providers backward compat
// ---------------------------------------------------------------------------

#[test]
fn effective_providers_from_llm_section() {
    let cfg = DuumbiConfig {
        llm: Some(LlmConfig {
            provider: ConfigLlmProvider::Anthropic,
            model: "claude-sonnet-4-6".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
        }),
        ..Default::default()
    };

    let providers = cfg.effective_providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].provider, ProviderKind::Anthropic);
    assert_eq!(providers[0].role, ProviderRole::Primary);
    assert_eq!(providers[0].model, "claude-sonnet-4-6");
}

#[test]
fn effective_providers_prefers_providers_over_llm() {
    let cfg = DuumbiConfig {
        llm: Some(LlmConfig {
            provider: ConfigLlmProvider::Anthropic,
            model: "old-model".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
        }),
        providers: vec![ProviderConfig {
            provider: ProviderKind::Grok,
            role: ProviderRole::Primary,
            model: "grok-3".to_string(),
            api_key_env: "XAI_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
        }],
        ..Default::default()
    };

    let providers = cfg.effective_providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].provider, ProviderKind::Grok);
}

#[test]
fn effective_providers_empty_when_no_config() {
    let cfg = DuumbiConfig::default();
    assert!(cfg.effective_providers().is_empty());
}

// ---------------------------------------------------------------------------
// Provider prompt tuning
// ---------------------------------------------------------------------------

#[test]
fn prompt_suffix_anthropic_empty() {
    assert!(provider_prompt_suffix("anthropic").is_empty());
}

#[test]
fn prompt_suffix_openai_has_reminder() {
    let suffix = provider_prompt_suffix("openai");
    assert!(!suffix.is_empty());
    assert!(suffix.contains("function calling"));
}

#[test]
fn prompt_suffix_grok_has_tool_reminder() {
    let suffix = provider_prompt_suffix("grok");
    assert!(suffix.contains("ONLY with tool calls"));
}

#[test]
fn prompt_suffix_openrouter_has_reminder() {
    let suffix = provider_prompt_suffix("openrouter");
    assert!(!suffix.is_empty());
}

#[test]
fn prompt_suffix_unknown_empty() {
    assert!(provider_prompt_suffix("future-provider").is_empty());
}

// ---------------------------------------------------------------------------
// ProviderChain construction
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "at least one provider")]
fn chain_requires_at_least_one_provider() {
    let _ = ProviderChain::new(vec![]);
}

// ---------------------------------------------------------------------------
// ProviderKind display
// ---------------------------------------------------------------------------

#[test]
fn provider_kind_display() {
    assert_eq!(ProviderKind::Anthropic.to_string(), "anthropic");
    assert_eq!(ProviderKind::OpenAI.to_string(), "openai");
    assert_eq!(ProviderKind::Grok.to_string(), "grok");
    assert_eq!(ProviderKind::OpenRouter.to_string(), "openrouter");
}

// ---------------------------------------------------------------------------
// ProviderRole default
// ---------------------------------------------------------------------------

#[test]
fn provider_role_default_is_primary() {
    let role = ProviderRole::default();
    assert_eq!(role, ProviderRole::Primary);
}

// ---------------------------------------------------------------------------
// Config roundtrip with providers
// ---------------------------------------------------------------------------

#[test]
fn config_providers_roundtrip_toml() {
    let cfg = DuumbiConfig {
        providers: vec![
            ProviderConfig {
                provider: ProviderKind::Anthropic,
                role: ProviderRole::Primary,
                model: "claude-sonnet-4-6".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                base_url: None,
                timeout_secs: None,
            },
            ProviderConfig {
                provider: ProviderKind::Grok,
                role: ProviderRole::Fallback,
                model: "grok-3".to_string(),
                api_key_env: "XAI_API_KEY".to_string(),
                base_url: None,
                timeout_secs: Some(30),
            },
        ],
        ..Default::default()
    };

    let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
    let loaded: DuumbiConfig = toml::from_str(&toml_str).expect("deserialize");
    assert_eq!(loaded.providers.len(), 2);
    assert_eq!(loaded.providers[0].provider, ProviderKind::Anthropic);
    assert_eq!(loaded.providers[1].provider, ProviderKind::Grok);
    assert_eq!(loaded.providers[1].timeout_secs, Some(30));
}
