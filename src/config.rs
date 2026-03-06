//! Configuration loader for `.duumbi/config.toml`.
//!
//! Reads LLM provider settings used by `duumbi add` and related AI commands.
//! The actual API key is **never** stored — only the name of the env var.

use std::fmt;
use std::fs;
use std::path::Path;

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

/// LLM provider selection.
#[allow(dead_code)] // Used in Issue #29/#30 provider implementations
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

/// LLM configuration block from `[llm]` in `config.toml`.
#[allow(dead_code)] // Used in Issue #31 orchestrator
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

impl LlmConfig {
    /// Resolves the API key by reading the configured environment variable.
    ///
    /// Returns an error if the env var is not set.
    #[allow(dead_code)] // Called in Issue #29/#30 provider implementations
    #[must_use = "must use the resolved API key or handle the error"]
    pub fn resolve_api_key(&self) -> Result<String, ConfigError> {
        std::env::var(&self.api_key_env).map_err(|_| ConfigError::Invalid {
            field: "api_key_env".to_string(),
            reason: format!("Environment variable '{}' is not set", self.api_key_env),
        })
    }
}

/// A dependency declared in the `[dependencies]` section of `config.toml`.
///
/// Two syntactic forms are supported:
///
/// ```toml
/// # Version-pinned (scope-based, M5+): value is a bare version string
/// "@duumbi/stdlib-math" = "1.0.0"
///
/// # Local path dependency (all phases): value is a table with `path`
/// mylib = { path = "../mylib" }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DependencyConfig {
    /// Cache/registry-resolved dependency — value is the version string.
    ///
    /// The module is resolved from `.duumbi/cache/@scope/name@version/`.
    Version(String),
    /// Local path dependency — resolved relative to the workspace root.
    Path {
        /// Relative or absolute path to the dependency workspace.
        path: String,
    },
}

#[allow(dead_code)] // Methods called progressively as pipeline is integrated
impl DependencyConfig {
    /// Returns the version string if this is a `Version` variant.
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Version(v) => Some(v.as_str()),
            Self::Path { .. } => None,
        }
    }

    /// Returns the path string if this is a `Path` variant.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Path { path } => Some(path.as_str()),
            Self::Version(_) => None,
        }
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

/// Optional `[vendor]` section in `config.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VendorSection {
    /// Vendoring strategy applied during `duumbi deps vendor`.
    #[serde(default)]
    pub strategy: VendorStrategy,
}

/// Top-level duumbi workspace configuration.
#[allow(dead_code)] // Used in Issue #31 orchestrator
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DuumbiConfig {
    /// Workspace identity settings (name, namespace).
    pub workspace: Option<WorkspaceSection>,

    /// LLM provider settings — required for `duumbi add` / AI commands.
    ///
    /// Omitting this section is allowed; the CLI will return a clear error
    /// when an AI command is invoked without LLM config.
    pub llm: Option<LlmConfig>,

    /// Dependencies declared for this workspace.
    ///
    /// Keys are either scoped module names (`@scope/name`) or plain dep names.
    /// Values are either a bare version string or a `{ path = "..." }` table.
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyConfig>,

    /// Vendor configuration.
    pub vendor: Option<VendorSection>,
}

/// Saves a [`DuumbiConfig`] to `<workspace_root>/.duumbi/config.toml`.
///
/// Creates the `.duumbi/` directory if it does not exist.
/// Overwrites any existing `config.toml`.
#[must_use = "save errors should be handled"]
pub fn save_config(workspace_root: &Path, config: &DuumbiConfig) -> Result<(), ConfigError> {
    let duumbi_dir = workspace_root.join(".duumbi");
    fs::create_dir_all(&duumbi_dir).map_err(|source| ConfigError::Io {
        path: duumbi_dir.display().to_string(),
        source,
    })?;
    let path = duumbi_dir.join("config.toml");
    let contents = toml::to_string_pretty(config).map_err(|e| ConfigError::Invalid {
        field: "config".to_string(),
        reason: e.to_string(),
    })?;
    fs::write(&path, contents).map_err(|source| ConfigError::Io {
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

    if !path.exists() {
        return Err(ConfigError::NotFound(path.display().to_string()));
    }

    let contents = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
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
    fn resolve_api_key_returns_value_when_set() {
        let llm = LlmConfig {
            provider: LlmProvider::Anthropic,
            model: "claude-sonnet-4-6".to_string(),
            api_key_env: "DUUMBI_TEST_KEY_ABC123".to_string(),
        };
        // SAFETY: test-only env mutation; var name is unique to this test.
        // Cargo's test harness runs these tests single-threaded by default.
        unsafe { std::env::set_var("DUUMBI_TEST_KEY_ABC123", "sk-test") };
        let key = llm
            .resolve_api_key()
            .expect("key must resolve when env var is set");
        assert_eq!(key, "sk-test");
        // SAFETY: same rationale — cleaning up what we set.
        unsafe { std::env::remove_var("DUUMBI_TEST_KEY_ABC123") };
    }

    #[test]
    fn resolve_api_key_errors_when_unset() {
        let llm = LlmConfig {
            provider: LlmProvider::Anthropic,
            model: "claude-sonnet-4-6".to_string(),
            api_key_env: "DUUMBI_DEFINITELY_NOT_SET_XYZ".to_string(),
        };
        let err = llm
            .resolve_api_key()
            .expect_err("must error when env var missing");
        assert!(matches!(err, ConfigError::Invalid { .. }));
    }

    #[test]
    fn llm_provider_display() {
        assert_eq!(LlmProvider::Anthropic.to_string(), "anthropic");
        assert_eq!(LlmProvider::OpenAI.to_string(), "openai");
    }
}
