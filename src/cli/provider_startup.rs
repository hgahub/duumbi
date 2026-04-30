//! Startup provider auto-configuration from environment variables.
//!
//! This keeps first-run interactive startup ergonomic: when a known provider
//! API key is already present in the environment, Duumbi can verify it and add
//! the corresponding user-level provider config before the TUI starts.

use std::collections::HashSet;

use crate::agents::model_access::{ModelAccessStore, credential_fingerprint_for_secret};
use crate::config::{
    DuumbiConfig, EffectiveConfig, KeyStorage, ProviderConfig, ProviderConfigSource, ProviderKind,
    ProviderRole,
};

/// A provider credential discovered in the process environment.
#[derive(Debug, Clone)]
pub struct EnvProviderSetup {
    /// Provider kind tied to the environment variable.
    pub provider: ProviderKind,
    /// Environment variable containing the API key.
    pub env_var: &'static str,
    /// API key value read from the environment.
    pub key: String,
    #[cfg(test)]
    base_url: Option<String>,
}

/// Outcome for one provider startup setup attempt.
#[derive(Debug, Clone)]
pub struct EnvProviderSetupResult {
    /// Provider kind.
    pub provider: ProviderKind,
    /// Environment variable used for this provider.
    pub env_var: &'static str,
    /// Whether setup succeeded.
    pub success: bool,
    /// User-facing summary.
    pub message: String,
}

/// Summary of startup auto-configuration.
#[derive(Debug, Clone, Default)]
pub struct EnvProviderSetupReport {
    /// Per-provider setup results.
    pub results: Vec<EnvProviderSetupResult>,
}

impl EnvProviderSetupReport {
    /// Returns true if at least one provider was configured successfully.
    #[must_use]
    pub fn any_success(&self) -> bool {
        self.results.iter().any(|result| result.success)
    }
}

/// Returns providers with non-empty env vars that are absent from effective config.
#[must_use]
pub fn discover_env_provider_setups(effective: &EffectiveConfig) -> Vec<EnvProviderSetup> {
    discover_env_provider_setups_from(effective, env_provider_candidates())
}

fn discover_env_provider_setups_from(
    effective: &EffectiveConfig,
    candidates: &[EnvProviderCandidate],
) -> Vec<EnvProviderSetup> {
    let configured = configured_provider_kinds(&effective.config);
    candidates
        .iter()
        .filter(|candidate| !configured.contains(&candidate.provider))
        .filter_map(|candidate| {
            let key = std::env::var(candidate.env_var).ok()?;
            let key = key.trim().to_string();
            if key.is_empty() {
                return None;
            }
            Some(EnvProviderSetup {
                provider: candidate.provider.clone(),
                env_var: candidate.env_var,
                key,
                #[cfg(test)]
                base_url: candidate.base_url.clone(),
            })
        })
        .collect()
}

/// Probes and persists discovered env providers into the user config.
///
/// Successful providers are saved as environment-backed provider entries. The
/// secret itself is not copied into `credentials.toml`.
pub async fn configure_env_providers(
    effective: &EffectiveConfig,
    setups: Vec<EnvProviderSetup>,
) -> EnvProviderSetupReport {
    if setups.is_empty() {
        return EnvProviderSetupReport::default();
    }

    let mut user_config = effective.user_config.clone();
    if !can_write_user_provider_config(effective) {
        return EnvProviderSetupReport {
            results: setups
                .into_iter()
                .map(|setup| EnvProviderSetupResult {
                    provider: setup.provider,
                    env_var: setup.env_var,
                    success: false,
                    message: format!(
                        "Provider auto-configuration skipped because active providers come from {} config.",
                        provider_source_label(effective.provider_source)
                    ),
                })
                .collect(),
        };
    }

    let mut report = EnvProviderSetupReport::default();
    let mut changed = false;
    let mut pending_metadata = Vec::new();

    for setup in setups {
        let provider_config = provider_config_for_setup(&setup, &user_config);
        match super::app::probe_provider_config_with_key(
            provider_config.clone(),
            setup.key.clone(),
            false,
        )
        .await
        {
            Ok(probe_report) => {
                let fingerprint =
                    credential_fingerprint_for_secret(&setup.provider, &setup.key, false);
                upsert_provider(&mut user_config, provider_config);
                changed = true;
                pending_metadata.push(PendingModelAccessMetadata {
                    result_index: report.results.len(),
                    fingerprint,
                    probe_report: probe_report.clone(),
                });
                report.results.push(EnvProviderSetupResult {
                    provider: setup.provider,
                    env_var: setup.env_var,
                    success: true,
                    message: probe_report.success_message(),
                });
            }
            Err(message) => {
                report.results.push(EnvProviderSetupResult {
                    provider: setup.provider,
                    env_var: setup.env_var,
                    success: false,
                    message,
                });
            }
        }
    }

    if changed && let Err(e) = crate::config::save_user_config(&user_config) {
        for result in &mut report.results {
            if result.success {
                result.success = false;
                result.message = format!("Provider config save failed: {e}");
            }
        }
        return report;
    }

    for pending in pending_metadata {
        if let Err(e) = ModelAccessStore::record_report(&pending.fingerprint, &pending.probe_report)
            && let Some(result) = report.results.get_mut(pending.result_index)
        {
            result.message = format!("{} Model access metadata save failed: {e}", result.message);
        }
    }

    report
}

#[derive(Debug, Clone)]
struct PendingModelAccessMetadata {
    result_index: usize,
    fingerprint: String,
    probe_report: crate::agents::model_access::ProviderProbeReport,
}

#[derive(Debug, Clone)]
struct EnvProviderCandidate {
    provider: ProviderKind,
    env_var: &'static str,
    #[cfg(test)]
    base_url: Option<String>,
}

fn env_provider_candidates() -> &'static [EnvProviderCandidate] {
    &[
        EnvProviderCandidate {
            provider: ProviderKind::Anthropic,
            env_var: "ANTHROPIC_API_KEY",
            #[cfg(test)]
            base_url: None,
        },
        EnvProviderCandidate {
            provider: ProviderKind::OpenAI,
            env_var: "OPENAI_API_KEY",
            #[cfg(test)]
            base_url: None,
        },
        EnvProviderCandidate {
            provider: ProviderKind::Grok,
            env_var: "XAI_API_KEY",
            #[cfg(test)]
            base_url: None,
        },
        EnvProviderCandidate {
            provider: ProviderKind::OpenRouter,
            env_var: "OPENROUTER_API_KEY",
            #[cfg(test)]
            base_url: None,
        },
        EnvProviderCandidate {
            provider: ProviderKind::MiniMax,
            env_var: "MINIMAX_API_KEY",
            #[cfg(test)]
            base_url: None,
        },
    ]
}

fn configured_provider_kinds(config: &DuumbiConfig) -> HashSet<ProviderKind> {
    config
        .effective_providers()
        .into_iter()
        .map(|provider| provider.provider)
        .collect()
}

fn can_write_user_provider_config(effective: &EffectiveConfig) -> bool {
    !effective.user_config.providers.is_empty()
        || matches!(effective.provider_source, ProviderConfigSource::None)
}

fn provider_source_label(source: ProviderConfigSource) -> &'static str {
    match source {
        ProviderConfigSource::None => "empty",
        ProviderConfigSource::System | ProviderConfigSource::LegacySystem => "system",
        ProviderConfigSource::User | ProviderConfigSource::LegacyUser => "user",
        ProviderConfigSource::Workspace | ProviderConfigSource::LegacyWorkspace => "workspace",
    }
}

fn provider_config_for_setup(
    setup: &EnvProviderSetup,
    user_config: &DuumbiConfig,
) -> ProviderConfig {
    let role = user_config
        .providers
        .iter()
        .find(|provider| provider.provider == setup.provider)
        .map(|provider| provider.role.clone())
        .unwrap_or_else(|| {
            if user_config.providers.is_empty() {
                ProviderRole::Primary
            } else {
                ProviderRole::Fallback
            }
        });

    ProviderConfig {
        provider: setup.provider.clone(),
        role,
        model: None,
        api_key_env: setup.env_var.to_string(),
        #[cfg(test)]
        base_url: setup.base_url.clone(),
        #[cfg(not(test))]
        base_url: None,
        timeout_secs: None,
        key_storage: Some(KeyStorage::Env),
        auth_token_env: None,
    }
}

fn upsert_provider(config: &mut DuumbiConfig, provider: ProviderConfig) {
    if let Some(index) = config
        .providers
        .iter()
        .position(|existing| existing.provider == provider.provider)
    {
        config.providers[index] = provider;
    } else {
        config.providers.push(provider);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::model_access::ModelAccessStore;
    use crate::config::{ProviderConfigSource, merge_config_layers};
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = &self.previous {
                    std::env::set_var(self.key, previous);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        env_lock().lock().unwrap_or_else(|err| err.into_inner())
    }

    fn effective_with(
        user_config: DuumbiConfig,
        workspace_config: DuumbiConfig,
    ) -> EffectiveConfig {
        merge_config_layers(DuumbiConfig::default(), user_config, workspace_config)
    }

    fn test_provider(kind: ProviderKind, env_var: &str) -> ProviderConfig {
        ProviderConfig {
            provider: kind,
            role: ProviderRole::Primary,
            model: None,
            api_key_env: env_var.to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        }
    }

    fn test_candidate(provider: ProviderKind, env_var: &'static str) -> EnvProviderCandidate {
        EnvProviderCandidate {
            provider,
            env_var,
            base_url: None,
        }
    }

    #[test]
    fn discovery_maps_all_known_env_vars() {
        let _lock = lock_env();
        let guards = [
            EnvGuard::set("ANTHROPIC_API_KEY", "anthropic-key"),
            EnvGuard::set("OPENAI_API_KEY", " openai-key \n"),
            EnvGuard::set("XAI_API_KEY", "xai-key"),
            EnvGuard::set("OPENROUTER_API_KEY", "openrouter-key"),
            EnvGuard::set("MINIMAX_API_KEY", "minimax-key"),
        ];
        let effective = effective_with(DuumbiConfig::default(), DuumbiConfig::default());

        let setups = discover_env_provider_setups(&effective);

        drop(guards);
        assert_eq!(setups.len(), 5);
        assert!(
            setups
                .iter()
                .any(|setup| setup.provider == ProviderKind::Anthropic)
        );
        assert!(
            setups
                .iter()
                .any(|setup| setup.provider == ProviderKind::OpenAI)
        );
        let openai = setups
            .iter()
            .find(|setup| setup.provider == ProviderKind::OpenAI)
            .expect("openai setup must be discovered");
        assert_eq!(openai.key, "openai-key");
        assert!(
            setups
                .iter()
                .any(|setup| setup.provider == ProviderKind::Grok)
        );
        assert!(
            setups
                .iter()
                .any(|setup| setup.provider == ProviderKind::OpenRouter)
        );
        assert!(
            setups
                .iter()
                .any(|setup| setup.provider == ProviderKind::MiniMax)
        );
    }

    #[test]
    fn discovery_ignores_empty_values_and_existing_provider_kind() {
        let _lock = lock_env();
        let _openai = EnvGuard::set("OPENAI_API_KEY", "   ");
        let _minimax = EnvGuard::set("MINIMAX_API_KEY", "minimax-key");
        let mut user = DuumbiConfig::default();
        user.providers
            .push(test_provider(ProviderKind::MiniMax, "MINIMAX_API_KEY"));
        let effective = effective_with(user, DuumbiConfig::default());
        let candidates = [
            test_candidate(ProviderKind::OpenAI, "OPENAI_API_KEY"),
            test_candidate(ProviderKind::MiniMax, "MINIMAX_API_KEY"),
        ];

        let setups = discover_env_provider_setups_from(&effective, &candidates);

        assert!(setups.is_empty());
    }

    #[test]
    fn configure_success_saves_env_backed_config_without_secret_file() {
        let _lock = lock_env();
        let temp = TempDir::new().expect("invariant: temp dir");
        let _home = EnvGuard::set("HOME", temp.path().to_str().expect("utf8 temp path"));
        let setup = EnvProviderSetup {
            provider: ProviderKind::MiniMax,
            env_var: "MINIMAX_API_KEY",
            key: "secret-minimax-key".to_string(),
            base_url: Some("duumbi-test://ok".to_string()),
        };
        let effective = effective_with(DuumbiConfig::default(), DuumbiConfig::default());

        let report = run_async_test(configure_env_providers(&effective, vec![setup]));

        assert!(report.any_success());
        let saved = crate::config::load_user_config().expect("user config must be saved");
        assert_eq!(saved.providers.len(), 1);
        assert_eq!(saved.providers[0].provider, ProviderKind::MiniMax);
        assert_eq!(saved.providers[0].api_key_env, "MINIMAX_API_KEY");
        assert_eq!(saved.providers[0].key_storage, Some(KeyStorage::Env));
        let credentials = temp.path().join(".duumbi/credentials.toml");
        assert!(!credentials.exists());

        let fingerprint =
            credential_fingerprint_for_secret(&ProviderKind::MiniMax, "secret-minimax-key", false);
        let accessible = ModelAccessStore::accessible_models_from_home(
            temp.path(),
            &ProviderKind::MiniMax,
            &fingerprint,
        );
        assert!(accessible.contains("MiniMax-M2.7"));
    }

    #[test]
    fn configure_failure_does_not_save_provider() {
        let _lock = lock_env();
        let temp = TempDir::new().expect("invariant: temp dir");
        let _home = EnvGuard::set("HOME", temp.path().to_str().expect("utf8 temp path"));
        let setup = EnvProviderSetup {
            provider: ProviderKind::OpenAI,
            env_var: "OPENAI_API_KEY",
            key: "bad-key".to_string(),
            base_url: Some("duumbi-test://unauthorized".to_string()),
        };
        let effective = effective_with(DuumbiConfig::default(), DuumbiConfig::default());

        let report = run_async_test(configure_env_providers(&effective, vec![setup]));

        assert!(!report.any_success());
        assert_eq!(report.results.len(), 1);
        assert!(!crate::config::user_config_path().exists());
    }

    #[test]
    fn configure_skips_when_lower_layer_providers_are_active() {
        let _lock = lock_env();
        let temp = TempDir::new().expect("invariant: temp dir");
        let _home = EnvGuard::set("HOME", temp.path().to_str().expect("utf8 temp path"));
        let mut workspace = DuumbiConfig::default();
        workspace
            .providers
            .push(test_provider(ProviderKind::Anthropic, "ANTHROPIC_API_KEY"));
        let effective = effective_with(DuumbiConfig::default(), workspace);
        assert_eq!(effective.provider_source, ProviderConfigSource::Workspace);
        let setup = EnvProviderSetup {
            provider: ProviderKind::MiniMax,
            env_var: "MINIMAX_API_KEY",
            key: "secret-minimax-key".to_string(),
            base_url: Some("duumbi-test://ok".to_string()),
        };

        let report = run_async_test(configure_env_providers(&effective, vec![setup]));

        assert!(!report.any_success());
        assert_eq!(report.results.len(), 1);
        assert!(report.results[0].message.contains("workspace config"));
        assert!(!crate::config::user_config_path().exists());
    }

    fn run_async_test<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("invariant: tokio test runtime")
            .block_on(future)
    }
}
