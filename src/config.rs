//! Configuration loader for `.duumbi/config.toml`.
//!
//! Reads LLM provider settings, dependency declarations, registry endpoints,
//! and vendor configuration. The actual API key is **never** stored — only
//! the name of the env var.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur when loading or validating the config file.
#[allow(dead_code)] // Used in Issue #29/#30 when provider implementations call load_config
#[derive(Debug, Error)]
pub enum ConfigError {
    /// No `.duumbi/config.toml` found at the given path.
    #[error("Config file not found: {0}")]
    NotFound(String),

    /// I/O error while reading the config file.
    #[error("Failed to read config file '{path}': {source}")]
    Io {
        /// Path that was attempted.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// TOML parse error.
    #[error("Failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),

    /// A config field value is invalid.
    #[error("Config field '{field}' is invalid: {reason}")]
    Invalid {
        /// Field name.
        field: String,
        /// Reason the value is invalid.
        reason: String,
    },
}

/// LLM provider selection (legacy `[llm]` section format).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// Anthropic's Claude API (tool_use format).
    Anthropic,
    /// OpenAI's API (function calling format).
    OpenAI,
}

impl fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmProvider::Anthropic => f.write_str("anthropic"),
            LlmProvider::OpenAI => f.write_str("openai"),
        }
    }
}

/// LLM configuration block from `[llm]` in `config.toml` (legacy format).
///
/// Kept for backward compatibility. New configs should use `[[providers]]`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    /// LLM provider to use (`"anthropic"` or `"openai"`).
    pub provider: LlmProvider,

    /// Model name, e.g. `"claude-sonnet-4-6"` or `"gpt-4o"`.
    pub model: String,

    /// Name of the environment variable that holds the API key.
    ///
    /// Example: `"ANTHROPIC_API_KEY"` or `"OPENAI_API_KEY"`.
    /// The key itself is never stored in config.
    pub api_key_env: String,
}

// ---------------------------------------------------------------------------
// Phase 9B: Multi-provider configuration
// ---------------------------------------------------------------------------

/// Provider kind for the `[[providers]]` config section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    /// Anthropic Claude API.
    Anthropic,
    /// OpenAI API.
    #[serde(rename = "openai")]
    OpenAI,
    /// xAI Grok API (OpenAI-compatible).
    Grok,
    /// OpenRouter API (OpenAI-compatible).
    #[serde(rename = "openrouter")]
    OpenRouter,
    /// MiniMax API (OpenAI-compatible).
    #[serde(rename = "minimax")]
    MiniMax,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderKind::Anthropic => f.write_str("anthropic"),
            ProviderKind::OpenAI => f.write_str("openai"),
            ProviderKind::Grok => f.write_str("grok"),
            ProviderKind::OpenRouter => f.write_str("openrouter"),
            ProviderKind::MiniMax => f.write_str("minimax"),
        }
    }
}

/// Role of a provider in the provider chain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderRole {
    /// Primary provider — tried first.
    #[default]
    Primary,
    /// Fallback provider — tried when the primary fails with a transient error.
    Fallback,
}

/// Storage method for API keys.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyStorage {
    /// Stored in `~/.duumbi/credentials.toml` (0600 perms).
    File,
    /// Read from environment variable (default behavior).
    Env,
}

/// Configuration for a single LLM provider entry in `[[providers]]`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Provider type.
    pub provider: ProviderKind,

    /// Role in the provider chain (primary or fallback).
    #[serde(default)]
    pub role: ProviderRole,

    /// Legacy model name from older configs.
    ///
    /// New configs omit this field. Runtime code resolves the concrete model
    /// from Duumbi's versioned internal model catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Name of the environment variable holding the API key.
    pub api_key_env: String,

    /// Optional custom endpoint override (e.g. self-hosted API).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Optional per-provider timeout in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,

    /// How the API key is stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_storage: Option<KeyStorage>,

    /// Optional env var name for subscription/auth token (Bearer token).
    ///
    /// When set and the env var exists, this token is used for authentication
    /// instead of the API key. Enables subscription-based access
    /// (e.g. Claude Max OAuth token via `ANTHROPIC_AUTH_TOKEN`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token_env: Option<String>,
}

/// Runtime provider config with a concrete model selected by Duumbi.
#[derive(Debug, Clone)]
pub struct ResolvedProviderConfig {
    /// Provider type.
    pub provider: ProviderKind,
    /// Concrete internal model identifier.
    pub model: String,
    /// API key environment variable name.
    pub api_key_env: String,
    /// Optional custom endpoint override.
    pub base_url: Option<String>,
    /// Optional timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Optional bearer-token environment variable name.
    pub auth_token_env: Option<String>,
}

/// A dependency declared in the `[dependencies]` section of `config.toml`.
///
/// Three syntactic forms are supported:
///
/// ```toml
/// # Version-pinned (scope-based, M5+): bare version string or SemVer range
/// "@duumbi/stdlib-math" = "^1.0"
///
/// # Version with explicit registry selection
/// "@company/auth-core" = { version = "^3.0", registry = "company" }
///
/// # Local path dependency (all phases): value is a table with `path`
/// mylib = { path = "../mylib" }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DependencyConfig {
    /// Version with explicit registry — table with `version` and `registry` keys.
    VersionWithRegistry {
        /// SemVer version requirement (e.g. `"^1.0"`, `"~2.1"`, `">=3.0.0"`).
        version: String,
        /// Registry name key from the `[registries]` table.
        registry: String,
    },
    /// Local path dependency — resolved relative to the workspace root.
    Path {
        /// Relative or absolute path to the dependency workspace.
        path: String,
    },
    /// Cache/registry-resolved dependency — value is a SemVer version requirement string.
    ///
    /// Uses the default registry. Resolved from `.duumbi/cache/@scope/name@version/`.
    Version(String),
}

#[allow(dead_code)] // Methods called progressively as pipeline is integrated
impl DependencyConfig {
    /// Returns the version string regardless of variant.
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Version(v) | Self::VersionWithRegistry { version: v, .. } => Some(v.as_str()),
            Self::Path { .. } => None,
        }
    }

    /// Returns the path string if this is a `Path` variant.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Path { path } => Some(path.as_str()),
            _ => None,
        }
    }

    /// Returns the explicit registry name, if specified.
    pub fn registry(&self) -> Option<&str> {
        match self {
            Self::VersionWithRegistry { registry, .. } => Some(registry.as_str()),
            _ => None,
        }
    }

    /// Validates the version string as a SemVer requirement.
    ///
    /// Returns `Ok(())` if the version parses as a valid `semver::VersionReq`,
    /// or an error with the parse failure details.
    pub fn validate_version(&self) -> Result<(), ConfigError> {
        if let Some(v) = self.version() {
            semver::VersionReq::parse(v).map_err(|e| ConfigError::Invalid {
                field: "version".to_string(),
                reason: format!("Invalid SemVer range '{v}': {e}"),
            })?;
        }
        Ok(())
    }
}

/// Optional `[workspace]` section for namespace and identity configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorkspaceSection {
    /// Short workspace name used in module IDs and log output.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Local module namespace prefix (used by the resolver).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    /// Default registry name for dependencies without an explicit `registry` key.
    ///
    /// Must match a key in the `[registries]` table. If omitted, falls back to
    /// the first entry in `[registries]` or local-only resolution.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "default-registry"
    )]
    pub default_registry: Option<String>,
}

/// Vendoring strategy for the optional `[vendor]` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VendorStrategy {
    /// No vendoring — resolve all deps from the local cache (default).
    #[default]
    None,
    /// Vendor every dependency into `.duumbi/vendor/`.
    All,
    /// Vendor only modules matching the `include` patterns.
    Selective,
}

// ---------------------------------------------------------------------------
// Phase 12: Cost control configuration
// ---------------------------------------------------------------------------

fn default_budget_per_intent() -> usize {
    50_000
}

fn default_budget_per_session() -> usize {
    200_000
}

fn default_max_parallel() -> usize {
    3
}

fn default_circuit_breaker() -> u32 {
    5
}

fn default_alert_threshold() -> u8 {
    80
}

/// Cost control settings for the dynamic agent system.
///
/// All fields are optional in `config.toml`; omitted keys fall back to the
/// defaults listed below. An entirely absent `[cost]` section is also valid.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CostSection {
    /// Maximum total tokens allowed for one intent execution.
    #[serde(default = "default_budget_per_intent")]
    pub budget_per_intent: usize,

    /// Maximum total tokens allowed across an entire CLI session.
    #[serde(default = "default_budget_per_session")]
    pub budget_per_session: usize,

    /// Maximum number of LLM agent calls that may run concurrently.
    #[serde(default = "default_max_parallel")]
    pub max_parallel_agents: usize,

    /// Number of consecutive failures before the circuit breaker opens.
    #[serde(default = "default_circuit_breaker")]
    pub circuit_breaker_failures: u32,

    /// Alert when this percentage of the intent budget has been consumed.
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold_pct: u8,
}

impl Default for CostSection {
    fn default() -> Self {
        Self {
            budget_per_intent: default_budget_per_intent(),
            budget_per_session: default_budget_per_session(),
            max_parallel_agents: default_max_parallel(),
            circuit_breaker_failures: default_circuit_breaker(),
            alert_threshold_pct: default_alert_threshold(),
        }
    }
}

/// Optional `[vendor]` section in `config.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VendorSection {
    /// Vendoring strategy applied during `duumbi deps vendor`.
    #[serde(default)]
    pub strategy: VendorStrategy,
    /// Glob patterns for selective vendoring (only used when `strategy = "selective"`).
    ///
    /// Example: `["@company/*"]` vendors all modules in the `@company` scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
}

/// Top-level duumbi workspace configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DuumbiConfig {
    /// Workspace identity settings (name, namespace, default-registry).
    pub workspace: Option<WorkspaceSection>,

    /// External MCP server configurations for agent tool access.
    ///
    /// Keys are short server names used in logs and error messages.
    /// Values describe the connection URL and trust level.
    ///
    /// ```toml
    /// [mcp-clients.my-server]
    /// url = "http://localhost:3000/sse"
    /// description = "Local tool server"
    /// trusted = true
    /// ```
    #[serde(
        default,
        rename = "mcp-clients",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub mcp_clients: HashMap<String, crate::mcp::client::config::McpClientConfig>,

    /// LLM provider settings (legacy `[llm]` section — use `[[providers]]` instead).
    ///
    /// Omitting this section is allowed; the CLI will return a clear error
    /// when an AI command is invoked without LLM config.
    pub llm: Option<LlmConfig>,

    /// Multi-provider configuration (Phase 9B).
    ///
    /// ```toml
    /// [[providers]]
    /// provider = "anthropic"
    /// role = "primary"
    /// api_key_env = "ANTHROPIC_API_KEY"
    ///
    /// [[providers]]
    /// provider = "grok"
    /// role = "fallback"
    /// api_key_env = "XAI_API_KEY"
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ProviderConfig>,

    /// Named registry endpoints.
    ///
    /// Keys are short names used in `DependencyConfig::VersionWithRegistry`,
    /// values are HTTPS base URLs.
    ///
    /// ```toml
    /// [registries]
    /// duumbi = "https://registry.duumbi.dev"
    /// company = "https://registry.acme.com"
    /// ```
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub registries: HashMap<String, String>,

    /// Dependencies declared for this workspace.
    ///
    /// Keys are either scoped module names (`@scope/name`) or plain dep names.
    /// Values are a bare version string, `{ version, registry }` table, or `{ path }` table.
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyConfig>,

    /// Vendor configuration.
    pub vendor: Option<VendorSection>,

    /// Cost control settings for the dynamic agent system (Phase 12).
    ///
    /// All sub-fields have safe defaults; omitting the `[cost]` section is valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostSection>,
}

impl DuumbiConfig {
    /// Returns the effective provider list.
    ///
    /// If `[[providers]]` is non-empty, returns it directly.
    /// Otherwise, if the legacy `[llm]` section is present, converts it
    /// to a single primary provider entry for backward compatibility.
    /// Returns an empty vec if neither is configured.
    #[must_use]
    pub fn effective_providers(&self) -> Vec<ProviderConfig> {
        if !self.providers.is_empty() {
            return self.providers.clone();
        }

        // Legacy fallback: convert [llm] to a single provider
        if let Some(ref llm) = self.llm {
            let kind = match llm.provider {
                LlmProvider::Anthropic => ProviderKind::Anthropic,
                LlmProvider::OpenAI => ProviderKind::OpenAI,
            };
            return vec![ProviderConfig {
                provider: kind,
                role: ProviderRole::Primary,
                model: Some(llm.model.clone()),
                api_key_env: llm.api_key_env.clone(),
                base_url: None,
                timeout_secs: None,
                key_storage: None,
                auth_token_env: None,
            }];
        }

        Vec::new()
    }
}

/// Source layer that provides the active provider configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderConfigSource {
    /// No provider or legacy LLM configuration exists in any loaded layer.
    None,
    /// Providers came from `/etc/duumbi/config.toml`.
    System,
    /// Providers came from `~/.duumbi/config.toml`.
    User,
    /// Providers came from `<workspace>/.duumbi/config.toml`.
    Workspace,
    /// Legacy `[llm]` came from `/etc/duumbi/config.toml`.
    LegacySystem,
    /// Legacy `[llm]` came from `~/.duumbi/config.toml`.
    LegacyUser,
    /// Legacy `[llm]` came from `<workspace>/.duumbi/config.toml`.
    LegacyWorkspace,
}

/// Configuration assembled from system, user, and workspace layers.
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    /// Merged config used by runtime clients.
    pub config: DuumbiConfig,
    /// System-level config loaded from `/etc/duumbi/config.toml`, or default.
    pub system_config: DuumbiConfig,
    /// User-level config loaded from `~/.duumbi/config.toml`, or default.
    pub user_config: DuumbiConfig,
    /// Workspace config loaded from `<workspace>/.duumbi/config.toml`, or default.
    pub workspace_config: DuumbiConfig,
    /// Layer that supplied the active provider settings.
    pub provider_source: ProviderConfigSource,
}

/// Returns the user-level config path, `~/.duumbi/config.toml`.
#[must_use]
pub fn user_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".duumbi").join("config.toml")
}

/// Loads `~/.duumbi/config.toml`.
pub fn load_user_config() -> Result<DuumbiConfig, ConfigError> {
    load_config_file(&user_config_path())
}

/// Saves `~/.duumbi/config.toml`, creating `~/.duumbi/` if needed.
#[must_use = "save errors should be handled"]
pub fn save_user_config(config: &DuumbiConfig) -> Result<(), ConfigError> {
    save_config_file(&user_config_path(), config)
}

/// Loads system, user, and workspace config layers and returns the effective runtime config.
pub fn load_effective_config(workspace_root: &Path) -> Result<EffectiveConfig, ConfigError> {
    let system_config = match load_config_file(Path::new("/etc/duumbi/config.toml")) {
        Ok(config) => config,
        Err(ConfigError::NotFound(_)) => DuumbiConfig::default(),
        Err(err) => return Err(err),
    };
    let user_config = match load_user_config() {
        Ok(config) => config,
        Err(ConfigError::NotFound(_)) => DuumbiConfig::default(),
        Err(err) => return Err(err),
    };
    let workspace_config = match load_config(workspace_root) {
        Ok(config) => config,
        Err(ConfigError::NotFound(_)) => DuumbiConfig::default(),
        Err(err) => return Err(err),
    };

    Ok(merge_config_layers(
        system_config,
        user_config,
        workspace_config,
    ))
}

/// Merges system, user, and workspace config layers into one runtime config.
pub fn merge_config_layers(
    system_config: DuumbiConfig,
    user_config: DuumbiConfig,
    workspace_config: DuumbiConfig,
) -> EffectiveConfig {
    let mut config = system_config.clone();
    merge_non_provider_fields(&mut config, &user_config);
    merge_non_provider_fields(&mut config, &workspace_config);

    let provider_source = if !user_config.providers.is_empty() {
        config.providers = user_config.providers.clone();
        config.llm = user_config.llm.clone();
        ProviderConfigSource::User
    } else if user_config.llm.is_some() {
        config.providers.clear();
        config.llm = user_config.llm.clone();
        ProviderConfigSource::LegacyUser
    } else if !workspace_config.providers.is_empty() {
        config.providers = workspace_config.providers.clone();
        config.llm = workspace_config.llm.clone();
        ProviderConfigSource::Workspace
    } else if workspace_config.llm.is_some() {
        config.providers.clear();
        config.llm = workspace_config.llm.clone();
        ProviderConfigSource::LegacyWorkspace
    } else if !system_config.providers.is_empty() {
        config.providers = system_config.providers.clone();
        config.llm = system_config.llm.clone();
        ProviderConfigSource::System
    } else if system_config.llm.is_some() {
        config.providers.clear();
        config.llm = system_config.llm.clone();
        ProviderConfigSource::LegacySystem
    } else {
        config.providers.clear();
        config.llm = None;
        ProviderConfigSource::None
    };

    EffectiveConfig {
        config,
        system_config,
        user_config,
        workspace_config,
        provider_source,
    }
}

fn merge_non_provider_fields(base: &mut DuumbiConfig, overlay: &DuumbiConfig) {
    if overlay.workspace.is_some() {
        base.workspace = overlay.workspace.clone();
    }
    if !overlay.mcp_clients.is_empty() {
        base.mcp_clients = overlay.mcp_clients.clone();
    }
    if !overlay.registries.is_empty() {
        base.registries = overlay.registries.clone();
    }
    if !overlay.dependencies.is_empty() {
        base.dependencies = overlay.dependencies.clone();
    }
    if overlay.vendor.is_some() {
        base.vendor = overlay.vendor.clone();
    }
    if overlay.cost.is_some() {
        base.cost = overlay.cost.clone();
    }
}

/// Saves a [`DuumbiConfig`] to `<workspace_root>/.duumbi/config.toml`.
///
/// Creates the `.duumbi/` directory if it does not exist.
/// Overwrites any existing `config.toml`.
#[must_use = "save errors should be handled"]
pub fn save_config(workspace_root: &Path, config: &DuumbiConfig) -> Result<(), ConfigError> {
    save_config_file(&workspace_root.join(".duumbi").join("config.toml"), config)
}

fn save_config_file(path: &Path, config: &DuumbiConfig) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let contents = toml::to_string_pretty(config).map_err(|e| ConfigError::Invalid {
        field: "config".to_string(),
        reason: e.to_string(),
    })?;
    fs::write(path, contents).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}

/// Loads and validates configuration from `<workspace_root>/.duumbi/config.toml`.
///
/// Returns `Ok(DuumbiConfig)` if the file exists and parses successfully.
/// Returns `Err(ConfigError::NotFound)` if there is no `.duumbi/config.toml`.
#[allow(dead_code)] // Called in Issue #31 orchestrator and #32 duumbi-add CLI
pub fn load_config(workspace_root: &Path) -> Result<DuumbiConfig, ConfigError> {
    let path = workspace_root.join(".duumbi").join("config.toml");
    load_config_file(&path)
}

fn load_config_file(path: &Path) -> Result<DuumbiConfig, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::NotFound(path.display().to_string()));
    }

    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;

    let config: DuumbiConfig = toml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, contents: &str) {
        let duumbi = dir.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: temp dir must be writable");
        fs::write(duumbi.join("config.toml"), contents)
            .expect("invariant: config must be writable");
    }

    #[test]
    fn load_config_anthropic() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let llm = cfg.llm.expect("llm section must be present");
        assert_eq!(llm.provider, LlmProvider::Anthropic);
        assert_eq!(llm.model, "claude-sonnet-4-6");
        assert_eq!(llm.api_key_env, "ANTHROPIC_API_KEY");
    }

    #[test]
    fn load_config_openai() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[llm]
provider = "openai"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let llm = cfg.llm.expect("llm section must be present");
        assert_eq!(llm.provider, LlmProvider::OpenAI);
        assert_eq!(llm.model, "gpt-4o");
    }

    fn test_provider(kind: ProviderKind, model: &str) -> ProviderConfig {
        ProviderConfig {
            provider: kind,
            role: ProviderRole::Primary,
            model: Some(model.to_string()),
            api_key_env: "TEST_API_KEY".to_string(),
            base_url: None,
            timeout_secs: None,
            key_storage: None,
            auth_token_env: None,
        }
    }

    #[test]
    fn effective_config_uses_user_provider_without_workspace_provider() {
        let mut user = DuumbiConfig::default();
        user.providers
            .push(test_provider(ProviderKind::Anthropic, "user-model"));

        let effective = merge_config_layers(DuumbiConfig::default(), user, DuumbiConfig::default());

        assert_eq!(effective.provider_source, ProviderConfigSource::User);
        assert_eq!(
            effective.config.providers[0].model.as_deref(),
            Some("user-model")
        );
    }

    #[test]
    fn effective_config_user_provider_overrides_workspace_provider() {
        let mut user = DuumbiConfig::default();
        user.providers
            .push(test_provider(ProviderKind::Anthropic, "user-model"));
        let mut workspace = DuumbiConfig::default();
        workspace
            .providers
            .push(test_provider(ProviderKind::Anthropic, "workspace-model"));

        let effective = merge_config_layers(DuumbiConfig::default(), user, workspace);

        assert_eq!(effective.provider_source, ProviderConfigSource::User);
        assert_eq!(
            effective.config.providers[0].model.as_deref(),
            Some("user-model")
        );
    }

    #[test]
    fn effective_config_user_provider_overrides_workspace_legacy() {
        let mut user = DuumbiConfig::default();
        user.providers
            .push(test_provider(ProviderKind::Anthropic, "user-model"));
        let workspace = DuumbiConfig {
            llm: Some(LlmConfig {
                provider: LlmProvider::Anthropic,
                model: "workspace-legacy-model".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
            }),
            ..DuumbiConfig::default()
        };

        let effective = merge_config_layers(DuumbiConfig::default(), user, workspace);

        assert_eq!(effective.provider_source, ProviderConfigSource::User);
        assert_eq!(
            effective.config.effective_providers()[0].model.as_deref(),
            Some("user-model")
        );
    }

    #[test]
    fn effective_config_user_legacy_overrides_system_provider() {
        let mut system = DuumbiConfig::default();
        system
            .providers
            .push(test_provider(ProviderKind::Anthropic, "system-model"));
        let user = DuumbiConfig {
            llm: Some(LlmConfig {
                provider: LlmProvider::Anthropic,
                model: "user-legacy-model".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
            }),
            ..DuumbiConfig::default()
        };

        let effective = merge_config_layers(system, user, DuumbiConfig::default());

        assert_eq!(effective.provider_source, ProviderConfigSource::LegacyUser);
        assert_eq!(
            effective.config.effective_providers()[0].model.as_deref(),
            Some("user-legacy-model")
        );
    }

    #[test]
    fn effective_config_falls_back_to_user_legacy_llm() {
        let user = DuumbiConfig {
            llm: Some(LlmConfig {
                provider: LlmProvider::Anthropic,
                model: "claude-test".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
            }),
            ..DuumbiConfig::default()
        };

        let effective = merge_config_layers(DuumbiConfig::default(), user, DuumbiConfig::default());

        assert_eq!(effective.provider_source, ProviderConfigSource::LegacyUser);
        assert_eq!(
            effective.config.effective_providers()[0].model.as_deref(),
            Some("claude-test")
        );
    }

    #[test]
    fn load_config_no_llm_section_is_valid() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[compiler]
version = "0.1"

[build]
output_dir = "build"
"#,
        );

        let cfg = load_config(tmp.path()).expect("config without llm must parse");
        assert!(cfg.llm.is_none());
    }

    #[test]
    fn load_config_missing_file_returns_not_found() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        // No .duumbi directory created
        let err = load_config(tmp.path()).expect_err("must error on missing config");
        assert!(matches!(err, ConfigError::NotFound(_)));
    }

    #[test]
    fn load_config_invalid_toml_returns_parse_error() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(&tmp, "this is not valid toml [[[");
        let err = load_config(tmp.path()).expect_err("must error on invalid TOML");
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn load_config_unknown_provider_returns_parse_error() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[llm]
provider = "cohere"
model = "command"
api_key_env = "COHERE_KEY"
"#,
        );
        let err = load_config(tmp.path()).expect_err("unknown provider must fail");
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn llm_provider_display() {
        assert_eq!(LlmProvider::Anthropic.to_string(), "anthropic");
        assert_eq!(LlmProvider::OpenAI.to_string(), "openai");
    }

    // -------------------------------------------------------------------------
    // Config v2 tests (#156)
    // -------------------------------------------------------------------------

    #[test]
    fn config_v2_registries_and_default_registry() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[workspace]
name = "myapp"
namespace = "myapp"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"
company = "https://registry.acme.com"
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let ws = cfg.workspace.expect("workspace section must exist");
        assert_eq!(ws.default_registry.as_deref(), Some("duumbi"));
        assert_eq!(cfg.registries.len(), 2);
        assert_eq!(cfg.registries["duumbi"], "https://registry.duumbi.dev");
        assert_eq!(cfg.registries["company"], "https://registry.acme.com");
    }

    #[test]
    fn config_v2_version_with_registry_dep() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[dependencies]
"@company/auth" = { version = "^3.0", registry = "company" }
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let dep = &cfg.dependencies["@company/auth"];
        assert_eq!(dep.version(), Some("^3.0"));
        assert_eq!(dep.registry(), Some("company"));
    }

    #[test]
    fn config_v2_bare_version_dep() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[dependencies]
"@duumbi/stdlib-math" = "^1.0"
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let dep = &cfg.dependencies["@duumbi/stdlib-math"];
        assert_eq!(dep.version(), Some("^1.0"));
        assert!(dep.registry().is_none());
    }

    #[test]
    fn config_v2_path_dep_unchanged() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[dependencies]
"local-utils" = { path = "../shared/utils" }
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let dep = &cfg.dependencies["local-utils"];
        assert_eq!(dep.path(), Some("../shared/utils"));
        assert!(dep.version().is_none());
    }

    #[test]
    fn config_v2_mixed_deps() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[dependencies]
"@duumbi/stdlib-math" = "^1.0"
"@company/auth-core" = { version = "^3.0", registry = "company" }
"local-utils" = { path = "../shared/utils" }
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        assert_eq!(cfg.dependencies.len(), 3);

        assert!(matches!(
            cfg.dependencies["@duumbi/stdlib-math"],
            DependencyConfig::Version(_)
        ));
        assert!(matches!(
            cfg.dependencies["@company/auth-core"],
            DependencyConfig::VersionWithRegistry { .. }
        ));
        assert!(matches!(
            cfg.dependencies["local-utils"],
            DependencyConfig::Path { .. }
        ));
    }

    #[test]
    fn config_v2_vendor_selective_with_include() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[vendor]
strategy = "selective"
include = ["@company/*"]
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let vendor = cfg.vendor.expect("vendor section must exist");
        assert_eq!(vendor.strategy, VendorStrategy::Selective);
        assert_eq!(vendor.include, vec!["@company/*"]);
    }

    #[test]
    fn config_v2_backward_compat_no_registries() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"

[dependencies]
"@duumbi/stdlib-math" = "1.0.0"
"#,
        );

        let cfg = load_config(tmp.path()).expect("v1 config must still parse");
        assert!(cfg.registries.is_empty());
        assert!(cfg.workspace.is_none());
        assert_eq!(cfg.dependencies.len(), 1);
    }

    #[test]
    fn semver_range_valid_patterns() {
        for range in ["^1.0", "~2.1", ">=3.0.0", "=1.2.3", "*", "1.0.0"] {
            let dep = DependencyConfig::Version(range.to_string());
            dep.validate_version()
                .unwrap_or_else(|_| panic!("'{range}' must be a valid SemVer range"));
        }
    }

    #[test]
    fn semver_range_invalid_returns_error() {
        let dep = DependencyConfig::Version("not-a-version".to_string());
        let err = dep
            .validate_version()
            .expect_err("invalid range must error");
        assert!(matches!(err, ConfigError::Invalid { .. }));
    }

    #[test]
    fn semver_range_version_with_registry_validates() {
        let dep = DependencyConfig::VersionWithRegistry {
            version: "^3.0".to_string(),
            registry: "company".to_string(),
        };
        dep.validate_version().expect("^3.0 must be valid");
    }

    #[test]
    fn semver_range_edge_cases_valid() {
        // Additional valid SemVer patterns not covered by semver_range_valid_patterns
        for range in ["0.0.0", ">=1.0.0, <2.0.0", "1.2.3-alpha.1"] {
            let dep = DependencyConfig::Version(range.to_string());
            dep.validate_version()
                .unwrap_or_else(|_| panic!("'{range}' must be a valid SemVer range"));
        }
    }

    #[test]
    fn path_dep_validate_version_always_ok() {
        let dep = DependencyConfig::Path {
            path: "../some/path".to_string(),
        };
        dep.validate_version()
            .expect("path dep has no version to validate — must always be Ok");
    }

    #[test]
    fn dependency_config_accessor_methods() {
        let version = DependencyConfig::Version("^1.0".to_string());
        assert_eq!(version.version(), Some("^1.0"));
        assert!(version.path().is_none());
        assert!(version.registry().is_none());

        let with_reg = DependencyConfig::VersionWithRegistry {
            version: "~2.0".to_string(),
            registry: "company".to_string(),
        };
        assert_eq!(with_reg.version(), Some("~2.0"));
        assert!(with_reg.path().is_none());
        assert_eq!(with_reg.registry(), Some("company"));

        let path = DependencyConfig::Path {
            path: "../lib".to_string(),
        };
        assert!(path.version().is_none());
        assert_eq!(path.path(), Some("../lib"));
        assert!(path.registry().is_none());
    }

    #[test]
    fn config_empty_dependencies_map() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[dependencies]
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        assert!(cfg.dependencies.is_empty());
    }

    #[test]
    fn config_all_sections_populated() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[workspace]
name = "full-app"
namespace = "fullapp"
default-registry = "duumbi"

[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"

[registries]
duumbi = "https://registry.duumbi.dev"
private = "https://registry.example.com"

[dependencies]
"@duumbi/stdlib-math" = "^1.0"
"@private/auth" = { version = "^2.0", registry = "private" }
"local-utils" = { path = "../utils" }

[vendor]
strategy = "all"
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        assert!(cfg.workspace.is_some());
        assert!(cfg.llm.is_some());
        assert_eq!(cfg.registries.len(), 2);
        assert_eq!(cfg.dependencies.len(), 3);
        let vendor = cfg.vendor.expect("vendor section");
        assert_eq!(vendor.strategy, VendorStrategy::All);
    }

    #[test]
    fn vendor_strategy_none_is_default() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[vendor]
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        let vendor = cfg.vendor.expect("vendor section");
        assert_eq!(vendor.strategy, VendorStrategy::None);
        assert!(vendor.include.is_empty());
    }

    #[test]
    fn save_config_creates_duumbi_dir() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        // No .duumbi dir exists yet
        assert!(!tmp.path().join(".duumbi").exists());

        let cfg = DuumbiConfig::default();
        save_config(tmp.path(), &cfg).expect("save must create dir");

        assert!(tmp.path().join(".duumbi").exists());
        assert!(tmp.path().join(".duumbi/config.toml").exists());
    }

    #[test]
    fn config_workspace_name_roundtrip() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        let cfg = DuumbiConfig {
            workspace: Some(WorkspaceSection {
                name: "my-app".to_string(),
                namespace: "myapp".to_string(),
                default_registry: Some("duumbi".to_string()),
            }),
            ..Default::default()
        };

        save_config(tmp.path(), &cfg).expect("save must succeed");
        let loaded = load_config(tmp.path()).expect("load must succeed");

        let ws = loaded.workspace.expect("workspace section");
        assert_eq!(ws.name, "my-app");
        assert_eq!(ws.namespace, "myapp");
        assert_eq!(ws.default_registry.as_deref(), Some("duumbi"));
    }

    #[test]
    fn config_empty_workspace_name_omitted_in_toml() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        let cfg = DuumbiConfig {
            workspace: Some(WorkspaceSection {
                name: String::new(),
                namespace: String::new(),
                default_registry: None,
            }),
            ..Default::default()
        };

        save_config(tmp.path(), &cfg).expect("save must succeed");
        let contents = fs::read_to_string(tmp.path().join(".duumbi/config.toml")).expect("read");
        // Empty strings should be skipped by skip_serializing_if
        assert!(
            !contents.contains("name = \"\""),
            "empty name should not appear"
        );
    }

    #[test]
    fn config_v2_roundtrip_save_load() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        let mut cfg = DuumbiConfig::default();
        cfg.registries.insert(
            "duumbi".to_string(),
            "https://registry.duumbi.dev".to_string(),
        );
        cfg.dependencies.insert(
            "@company/auth".to_string(),
            DependencyConfig::VersionWithRegistry {
                version: "^3.0".to_string(),
                registry: "company".to_string(),
            },
        );
        cfg.vendor = Some(VendorSection {
            strategy: VendorStrategy::Selective,
            include: vec!["@company/*".to_string()],
        });

        save_config(tmp.path(), &cfg).expect("save must succeed");
        let loaded = load_config(tmp.path()).expect("reload must succeed");

        assert_eq!(loaded.registries["duumbi"], "https://registry.duumbi.dev");
        assert_eq!(loaded.dependencies["@company/auth"].version(), Some("^3.0"));
        assert_eq!(
            loaded.dependencies["@company/auth"].registry(),
            Some("company")
        );
        let vendor = loaded.vendor.expect("vendor section");
        assert_eq!(vendor.strategy, VendorStrategy::Selective);
        assert_eq!(vendor.include, vec!["@company/*"]);
    }
}
