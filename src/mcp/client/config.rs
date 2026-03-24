//! Configuration types for external MCP client connections.
//!
//! Defines [`McpClientConfig`], which describes how DUUMBI connects to an
//! external MCP server. One entry corresponds to one `[mcp-clients.<name>]`
//! table in `config.toml`.

use serde::{Deserialize, Serialize};

fn default_trusted() -> bool {
    true
}

/// Configuration for a single external MCP server.
///
/// Each entry in the `[mcp-clients]` section of `config.toml` deserializes
/// into one of these.
///
/// ```toml
/// [mcp-clients.my-server]
/// url = "http://localhost:3000/sse"
/// description = "Local tool server"
/// trusted = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct McpClientConfig {
    /// URL of the external MCP server (e.g., SSE endpoint).
    pub url: String,

    /// Human-readable description of what this server provides.
    #[serde(default)]
    pub description: String,

    /// Whether this server is trusted for agent access.
    ///
    /// Trusted servers may be called by DUUMBI agents without additional
    /// confirmation. Defaults to `true` for ergonomics; set to `false` for
    /// servers whose tools should require explicit approval.
    #[serde(default = "default_trusted")]
    pub trusted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DuumbiConfig, load_config};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, contents: &str) {
        let duumbi = dir.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: temp dir must be writable");
        fs::write(duumbi.join("config.toml"), contents)
            .expect("invariant: config must be writable");
    }

    // -------------------------------------------------------------------------
    // McpClientConfig serde roundtrip
    // -------------------------------------------------------------------------

    #[test]
    fn mcp_client_config_serde_roundtrip() {
        let original = McpClientConfig {
            url: "http://localhost:3000/sse".to_string(),
            description: "Local tool server".to_string(),
            trusted: true,
        };

        let json = serde_json::to_string(&original).expect("serialization must succeed");
        let parsed: McpClientConfig =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(parsed.url, original.url);
        assert_eq!(parsed.description, original.description);
        assert_eq!(parsed.trusted, original.trusted);
    }

    #[test]
    fn mcp_client_config_default_trusted_is_true() {
        let json = r#"{"url": "http://example.com"}"#;
        let cfg: McpClientConfig = serde_json::from_str(json).expect("minimal config must parse");

        assert!(cfg.trusted, "trusted must default to true");
        assert_eq!(
            cfg.description, "",
            "description must default to empty string"
        );
    }

    #[test]
    fn mcp_client_config_default_description_is_empty() {
        let json = r#"{"url": "http://example.com", "trusted": false}"#;
        let cfg: McpClientConfig = serde_json::from_str(json).expect("config must parse");

        assert_eq!(cfg.description, "");
        assert!(!cfg.trusted);
    }

    #[test]
    fn mcp_client_config_explicit_untrusted() {
        let original = McpClientConfig {
            url: "https://external.example.com/mcp".to_string(),
            description: "Third-party server".to_string(),
            trusted: false,
        };

        let toml_str = toml::to_string(&original).expect("must serialize to TOML");
        let parsed: McpClientConfig = toml::from_str(&toml_str).expect("must parse TOML");

        assert_eq!(parsed.url, original.url);
        assert_eq!(parsed.description, original.description);
        assert!(!parsed.trusted);
    }

    // -------------------------------------------------------------------------
    // DuumbiConfig integration: [mcp-clients] section in config.toml
    // -------------------------------------------------------------------------

    #[test]
    fn duumbi_config_parses_mcp_clients_section() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[mcp-clients.my-server]
url = "http://localhost:3000/sse"
description = "Local tool server"
trusted = true
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        assert_eq!(cfg.mcp_clients.len(), 1);

        let server = cfg
            .mcp_clients
            .get("my-server")
            .expect("my-server must be present");
        assert_eq!(server.url, "http://localhost:3000/sse");
        assert_eq!(server.description, "Local tool server");
        assert!(server.trusted);
    }

    #[test]
    fn duumbi_config_parses_multiple_mcp_clients() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        write_config(
            &tmp,
            r#"
[mcp-clients.server-a]
url = "http://localhost:3001/sse"
description = "Server A"

[mcp-clients.server-b]
url = "http://localhost:3002/sse"
description = "Server B"
trusted = false
"#,
        );

        let cfg = load_config(tmp.path()).expect("config must parse");
        assert_eq!(cfg.mcp_clients.len(), 2);

        let a = cfg
            .mcp_clients
            .get("server-a")
            .expect("server-a must be present");
        assert!(a.trusted, "defaults to trusted");

        let b = cfg
            .mcp_clients
            .get("server-b")
            .expect("server-b must be present");
        assert!(!b.trusted);
    }

    #[test]
    fn duumbi_config_without_mcp_clients_parses_fine() {
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

        let cfg = load_config(tmp.path()).expect("config without mcp-clients must parse");
        assert!(
            cfg.mcp_clients.is_empty(),
            "mcp_clients must default to empty map"
        );
    }

    #[test]
    fn duumbi_config_empty_config_has_empty_mcp_clients() {
        // Verify backward compat: an entirely empty config.toml is still valid
        // and mcp_clients defaults to an empty HashMap.
        let config = DuumbiConfig::default();
        assert!(config.mcp_clients.is_empty());
    }

    #[test]
    fn mcp_clients_roundtrip_via_duumbi_config() {
        let mut mcp_clients = HashMap::new();
        mcp_clients.insert(
            "test-server".to_string(),
            McpClientConfig {
                url: "http://localhost:9999/sse".to_string(),
                description: "Test".to_string(),
                trusted: true,
            },
        );

        let config = DuumbiConfig {
            mcp_clients,
            ..Default::default()
        };

        let toml_str = toml::to_string_pretty(&config).expect("must serialize");
        let parsed: DuumbiConfig = toml::from_str(&toml_str).expect("must parse");

        assert_eq!(parsed.mcp_clients.len(), 1);
        let server = parsed
            .mcp_clients
            .get("test-server")
            .expect("server must be present");
        assert_eq!(server.url, "http://localhost:9999/sse");
    }
}
