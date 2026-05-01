//! LLM provider management for the interactive REPL.
//!
//! Implements list, add, remove, and set operations on the `[[providers]]`
//! section of `.duumbi/config.toml`, returning [`OutputLine`] slices that
//! the REPL pushes directly to its output buffer.

use crate::config::{DuumbiConfig, ProviderConfig, ProviderKind, ProviderRole};

use super::mode::{OutputLine, OutputStyle};

// ---------------------------------------------------------------------------
// list_providers
// ---------------------------------------------------------------------------

/// Returns a formatted table of all active providers from the config.
///
/// Uses [`DuumbiConfig::effective_providers`] so that the legacy `[llm]`
/// section is handled transparently alongside `[[providers]]` entries.
#[must_use]
pub fn list_providers(config: &DuumbiConfig) -> Vec<OutputLine> {
    let providers = config.effective_providers();
    if providers.is_empty() {
        return vec![OutputLine::new(
            "No providers configured.",
            OutputStyle::Dim,
        )];
    }

    let mut table = comfy_table::Table::new();
    table
        .set_header(vec!["#", "Provider", "Role", "Auth", "Key Env", "Ready?"])
        .load_preset(comfy_table::presets::UTF8_FULL_CONDENSED);

    for (i, p) in providers.iter().enumerate() {
        let (auth_type, ready) = if let Some(ref token_env) = p.auth_token_env {
            let token_ok = credential_available(token_env);
            let key_ok = credential_available(&p.api_key_env);
            ("bearer", if token_ok || key_ok { "yes" } else { "no" })
        } else {
            let key_ok = credential_available(&p.api_key_env);
            ("api-key", if key_ok { "yes" } else { "no" })
        };
        table.add_row(vec![
            (i + 1).to_string(),
            p.provider.to_string(),
            format!("{:?}", p.role).to_lowercase(),
            auth_type.to_string(),
            p.auth_token_env
                .as_deref()
                .unwrap_or(&p.api_key_env)
                .to_string(),
            ready.to_string(),
        ]);
    }

    table
        .to_string()
        .lines()
        .map(|l| OutputLine::new(l.to_string(), OutputStyle::Normal))
        .collect()
}

fn credential_available(env_var_name: &str) -> bool {
    std::env::var(env_var_name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || crate::credentials::load_api_key(env_var_name)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// add_provider
// ---------------------------------------------------------------------------

/// Parses `args` and appends a new provider to `config.providers`.
///
/// Expected syntax: `<type> <api_key_env> [--role fallback] [--base-url URL] [--auth-token-env ENV]`
///
/// `<type>` must be one of: `anthropic`, `openai`, `grok`, `openrouter`, `minimax`.
#[must_use]
pub fn add_provider(config: &mut DuumbiConfig, args: &str) -> Vec<OutputLine> {
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if tokens.len() < 2 {
        return vec![OutputLine::new(
            "Usage: /provider add <type> <api_key_env> [--role fallback] [--base-url URL] [--auth-token-env ENV]",
            OutputStyle::Dim,
        )];
    }

    let kind = match parse_provider_kind(tokens[0]) {
        Ok(k) => k,
        Err(msg) => return vec![OutputLine::new(msg, OutputStyle::Error)],
    };

    let api_key_env = tokens[1].to_string();

    // Parse optional flags from the remaining tokens.
    let mut role = ProviderRole::Primary;
    let mut base_url: Option<String> = None;
    let mut auth_token_env: Option<String> = None;
    let mut i = 2usize;
    while i < tokens.len() {
        match tokens[i] {
            "--role" => {
                i += 1;
                if i >= tokens.len() {
                    return vec![OutputLine::new(
                        "--role requires a value (primary or fallback)",
                        OutputStyle::Error,
                    )];
                }
                role = match tokens[i] {
                    "primary" => ProviderRole::Primary,
                    "fallback" => ProviderRole::Fallback,
                    other => {
                        return vec![OutputLine::new(
                            format!("Unknown role '{other}'. Use: primary, fallback"),
                            OutputStyle::Error,
                        )];
                    }
                };
            }
            "--base-url" => {
                i += 1;
                if i >= tokens.len() {
                    return vec![OutputLine::new(
                        "--base-url requires a value",
                        OutputStyle::Error,
                    )];
                }
                base_url = Some(tokens[i].to_string());
            }
            "--auth-token-env" => {
                i += 1;
                if i >= tokens.len() {
                    return vec![OutputLine::new(
                        "--auth-token-env requires a value (env var name for Bearer token)",
                        OutputStyle::Error,
                    )];
                }
                auth_token_env = Some(tokens[i].to_string());
            }
            other => {
                return vec![OutputLine::new(
                    format!("Unknown flag '{other}'"),
                    OutputStyle::Error,
                )];
            }
        }
        i += 1;
    }

    let auth_info = if auth_token_env.is_some() {
        " (subscription/Bearer)"
    } else {
        ""
    };

    config.providers.push(ProviderConfig {
        provider: kind,
        role,
        model: None,
        api_key_env: api_key_env.clone(),
        base_url,
        timeout_secs: None,
        key_storage: None,
        auth_token_env,
    });

    vec![OutputLine::new(
        format!(
            "Provider added: {}{auth_info} (api_key_env: {api_key_env})",
            tokens[0]
        ),
        OutputStyle::Success,
    )]
}

// ---------------------------------------------------------------------------
// remove_provider
// ---------------------------------------------------------------------------

/// Removes the provider identified by a 1-based index number or provider type.
#[must_use]
pub fn remove_provider(config: &mut DuumbiConfig, selector: &str) -> Vec<OutputLine> {
    if config.providers.is_empty() {
        return vec![OutputLine::new(
            "No providers configured.",
            OutputStyle::Dim,
        )];
    }

    // Try 1-based numeric index first.
    if let Ok(n) = selector.parse::<usize>() {
        if n == 0 || n > config.providers.len() {
            return vec![OutputLine::new(
                format!("Index {n} out of range (1–{}).", config.providers.len()),
                OutputStyle::Error,
            )];
        }
        let removed = config.providers.remove(n - 1);
        return vec![OutputLine::new(
            format!("Removed provider #{n}: {}", removed.provider),
            OutputStyle::Success,
        )];
    }

    // Fall back to provider-kind match (first occurrence).
    if let Some(pos) = config
        .providers
        .iter()
        .position(|p| p.provider.to_string() == selector)
    {
        let removed = config.providers.remove(pos);
        return vec![OutputLine::new(
            format!("Removed provider: {}", removed.provider),
            OutputStyle::Success,
        )];
    }

    vec![OutputLine::new(
        format!("No provider found matching '{selector}'."),
        OutputStyle::Error,
    )]
}

// ---------------------------------------------------------------------------
// set_provider_field
// ---------------------------------------------------------------------------

/// Updates a single field on the provider at a 1-based index.
///
/// Syntax: `<index> <field> <value>`
///
/// Valid fields: `api_key_env`, `role`, `base_url`, `auth_token_env`.
#[must_use]
pub fn set_provider_field(config: &mut DuumbiConfig, args: &str) -> Vec<OutputLine> {
    let mut parts = args.splitn(3, ' ');
    let idx_str = parts.next().unwrap_or("").trim();
    let field = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("").trim();

    if idx_str.is_empty() || field.is_empty() || value.is_empty() {
        return vec![OutputLine::new(
            "Usage: /provider set <index> <field> <value>",
            OutputStyle::Dim,
        )];
    }

    let idx: usize = match idx_str.parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            return vec![OutputLine::new(
                format!("'{idx_str}' is not a valid index."),
                OutputStyle::Error,
            )];
        }
    };

    if idx == 0 || idx > config.providers.len() {
        return vec![OutputLine::new(
            format!("Index {idx} out of range (1–{}).", config.providers.len()),
            OutputStyle::Error,
        )];
    }

    let provider = &mut config.providers[idx - 1];

    match field {
        "api_key_env" => {
            provider.api_key_env = value.to_string();
            vec![OutputLine::new(
                format!("Provider #{idx}: api_key_env set to '{value}'"),
                OutputStyle::Success,
            )]
        }
        "role" => match value {
            "primary" => {
                provider.role = ProviderRole::Primary;
                vec![OutputLine::new(
                    format!("Provider #{idx}: role set to 'primary'"),
                    OutputStyle::Success,
                )]
            }
            "fallback" => {
                provider.role = ProviderRole::Fallback;
                vec![OutputLine::new(
                    format!("Provider #{idx}: role set to 'fallback'"),
                    OutputStyle::Success,
                )]
            }
            other => vec![OutputLine::new(
                format!("Unknown role '{other}'. Use: primary, fallback"),
                OutputStyle::Error,
            )],
        },
        "base_url" => {
            provider.base_url = Some(value.to_string());
            vec![OutputLine::new(
                format!("Provider #{idx}: base_url set to '{value}'"),
                OutputStyle::Success,
            )]
        }
        "auth_token_env" => {
            if value == "none" || value == "null" || value.is_empty() {
                provider.auth_token_env = None;
                vec![OutputLine::new(
                    format!("Provider #{idx}: auth_token_env cleared (using API key)"),
                    OutputStyle::Success,
                )]
            } else {
                provider.auth_token_env = Some(value.to_string());
                vec![OutputLine::new(
                    format!(
                        "Provider #{idx}: auth_token_env set to '{value}' (subscription/Bearer)"
                    ),
                    OutputStyle::Success,
                )]
            }
        }
        other => vec![OutputLine::new(
            format!(
                "Unknown field '{other}'. Valid fields: api_key_env, role, base_url, auth_token_env"
            ),
            OutputStyle::Error,
        )],
    }
}

// ---------------------------------------------------------------------------
// CLI output helper
// ---------------------------------------------------------------------------

/// Prints a list of [`OutputLine`]s to stderr with style-appropriate formatting.
///
/// Used by the `duumbi provider` CLI subcommand to render the same output that
/// the REPL would display in its TUI buffer.
pub fn print_output_lines(lines: &[OutputLine]) {
    use owo_colors::OwoColorize as _;

    for line in lines {
        match line.style {
            OutputStyle::Error => eprintln!("{}", line.text.red()),
            OutputStyle::Success => eprintln!("{}", line.text.green()),
            OutputStyle::Dim => eprintln!("{}", line.text.dimmed()),
            _ => eprintln!("{}", line.text),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parses a provider kind string into a [`ProviderKind`].
fn parse_provider_kind(s: &str) -> Result<ProviderKind, String> {
    match s {
        "anthropic" => Ok(ProviderKind::Anthropic),
        "openai" => Ok(ProviderKind::OpenAI),
        "grok" => Ok(ProviderKind::Grok),
        "openrouter" => Ok(ProviderKind::OpenRouter),
        "minimax" => Ok(ProviderKind::MiniMax),
        other => Err(format!(
            "Unknown provider type '{other}'. Use: anthropic, openai, grok, openrouter, minimax"
        )),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DuumbiConfig;

    fn empty_config() -> DuumbiConfig {
        DuumbiConfig::default()
    }

    fn config_with_one_provider() -> DuumbiConfig {
        let mut cfg = empty_config();
        cfg.providers.push(ProviderConfig {
            provider: ProviderKind::Anthropic,
            role: ProviderRole::Primary,
            model: None,
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        });
        cfg
    }

    #[test]
    fn list_no_providers_returns_dim_message() {
        let lines = list_providers(&empty_config());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].style, OutputStyle::Dim);
        assert!(lines[0].text.contains("No providers"));
    }

    #[test]
    fn add_then_list_shows_entry() {
        let mut cfg = empty_config();
        let add_lines = add_provider(&mut cfg, "anthropic ANTHROPIC_API_KEY");
        assert_eq!(add_lines[0].style, OutputStyle::Success);
        assert_eq!(cfg.providers.len(), 1);
        assert!(cfg.providers[0].model.is_none());

        let list_lines = list_providers(&cfg);
        let combined = list_lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(combined.contains("anthropic"));
        assert!(!combined.contains("Model"));
    }

    #[test]
    fn remove_by_index_removes_entry() {
        let mut cfg = config_with_one_provider();
        let lines = remove_provider(&mut cfg, "1");
        assert_eq!(lines[0].style, OutputStyle::Success);
        assert!(cfg.providers.is_empty());
    }

    #[test]
    fn remove_by_provider_type_removes_entry() {
        let mut cfg = config_with_one_provider();
        let lines = remove_provider(&mut cfg, "anthropic");
        assert_eq!(lines[0].style, OutputStyle::Success);
        assert!(cfg.providers.is_empty());
    }

    #[test]
    fn remove_out_of_range_returns_error() {
        let mut cfg = config_with_one_provider();
        let lines = remove_provider(&mut cfg, "99");
        assert_eq!(lines[0].style, OutputStyle::Error);
        assert_eq!(cfg.providers.len(), 1);
    }

    #[test]
    fn remove_unknown_provider_returns_error() {
        let mut cfg = config_with_one_provider();
        let lines = remove_provider(&mut cfg, "gpt-4o");
        assert_eq!(lines[0].style, OutputStyle::Error);
        assert_eq!(cfg.providers.len(), 1);
    }

    #[test]
    fn set_model_field_returns_error() {
        let mut cfg = config_with_one_provider();
        let lines = set_provider_field(&mut cfg, "1 model claude-opus-4-5");
        assert_eq!(lines[0].style, OutputStyle::Error);
        assert!(cfg.providers[0].model.is_none());
    }

    #[test]
    fn set_role_to_fallback() {
        let mut cfg = config_with_one_provider();
        let lines = set_provider_field(&mut cfg, "1 role fallback");
        assert_eq!(lines[0].style, OutputStyle::Success);
        assert_eq!(cfg.providers[0].role, ProviderRole::Fallback);
    }

    #[test]
    fn set_unknown_field_returns_error() {
        let mut cfg = config_with_one_provider();
        let lines = set_provider_field(&mut cfg, "1 timeout_secs 30");
        assert_eq!(lines[0].style, OutputStyle::Error);
    }

    #[test]
    fn parse_provider_kind_all_variants() {
        assert_eq!(
            parse_provider_kind("anthropic"),
            Ok(ProviderKind::Anthropic)
        );
        assert_eq!(parse_provider_kind("openai"), Ok(ProviderKind::OpenAI));
        assert_eq!(parse_provider_kind("grok"), Ok(ProviderKind::Grok));
        assert_eq!(
            parse_provider_kind("openrouter"),
            Ok(ProviderKind::OpenRouter)
        );
        assert_eq!(parse_provider_kind("minimax"), Ok(ProviderKind::MiniMax));
        assert!(parse_provider_kind("unknown").is_err());
    }

    #[test]
    fn add_with_role_fallback_flag() {
        let mut cfg = empty_config();
        let _ = add_provider(&mut cfg, "grok XAI_API_KEY --role fallback");
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.providers[0].role, ProviderRole::Fallback);
    }

    #[test]
    fn add_with_base_url_flag() {
        let mut cfg = empty_config();
        let _ = add_provider(
            &mut cfg,
            "openai OPENAI_API_KEY --base-url https://api.openai.com",
        );
        assert_eq!(
            cfg.providers[0].base_url.as_deref(),
            Some("https://api.openai.com")
        );
    }

    #[test]
    fn add_too_few_args_returns_usage() {
        let mut cfg = empty_config();
        let lines = add_provider(&mut cfg, "anthropic");
        assert!(lines[0].text.contains("Usage"));
        assert!(cfg.providers.is_empty());
    }

    #[test]
    fn add_unknown_provider_type_returns_error() {
        let mut cfg = empty_config();
        let lines = add_provider(&mut cfg, "mistral mistral-large MISTRAL_API_KEY");
        assert_eq!(lines[0].style, OutputStyle::Error);
        assert!(cfg.providers.is_empty());
    }
}
