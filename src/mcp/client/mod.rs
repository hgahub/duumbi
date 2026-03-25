//! MCP client subsystem for DUUMBI.
//!
//! Manages connections to external MCP servers and provides tool discovery
//! so DUUMBI agents can call tools exposed by those servers.
//!
//! In the full implementation the manager would open SSE connections and
//! perform the JSON-RPC `tools/list` handshake automatically. For the current
//! phase the tool list is registered explicitly via [`McpClientManager::register_tools`].

pub mod config;

use std::collections::HashMap;

use config::McpClientConfig;

/// A tool discovered on an external MCP server.
#[allow(dead_code)] // Phase 12 G-3+: used when SSE connection layer is wired up
#[derive(Debug, Clone)]
pub struct ExternalTool {
    /// Server this tool belongs to.
    pub server_name: String,
    /// Tool name as reported by the server.
    pub tool_name: String,
    /// Human-readable description of the tool.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// Manages connections to external MCP servers.
///
/// Each configured server can be connected to, and its tools discovered and
/// proxied for use by DUUMBI agents. The manager is constructed from the
/// `[mcp-clients]` section of `config.toml` via [`McpClientManager::new`].
#[allow(dead_code)] // Phase 12 infrastructure — wired into agent execution later
pub struct McpClientManager {
    configs: HashMap<String, McpClientConfig>,
    /// Cached tool lists per server name.
    discovered_tools: HashMap<String, Vec<ExternalTool>>,
}

#[allow(dead_code)] // Phase 12 infrastructure — wired into agent execution later
impl McpClientManager {
    /// Create a new manager from a map of server configs.
    ///
    /// The `configs` map typically comes from [`crate::config::DuumbiConfig::mcp_clients`].
    #[must_use]
    pub fn new(configs: HashMap<String, McpClientConfig>) -> Self {
        Self {
            configs,
            discovered_tools: HashMap::new(),
        }
    }

    /// Return the names of all configured external MCP servers.
    #[must_use]
    pub fn server_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.configs.keys().map(String::as_str).collect();
        // Sort for deterministic output (useful in tests and CLI listings).
        names.sort_unstable();
        names
    }

    /// Return the configuration for the named server, or `None` if not configured.
    #[must_use]
    pub fn get_config(&self, name: &str) -> Option<&McpClientConfig> {
        self.configs.get(name)
    }

    /// Return `true` when the named server is both configured and marked as trusted.
    ///
    /// Untrusted servers require explicit user approval before their tools are
    /// invoked by agents.
    #[must_use]
    pub fn is_trusted(&self, name: &str) -> bool {
        self.configs.get(name).is_some_and(|c| c.trusted)
    }

    /// Register the tool list for a server.
    ///
    /// Overwrites any previously registered tool list for `server_name`.
    /// In the full implementation this is called automatically after a
    /// successful `tools/list` JSON-RPC response.
    pub fn register_tools(&mut self, server_name: &str, tools: Vec<ExternalTool>) {
        self.discovered_tools.insert(server_name.to_string(), tools);
    }

    /// Return all discovered tools across every server.
    #[must_use]
    pub fn all_tools(&self) -> Vec<&ExternalTool> {
        self.discovered_tools.values().flatten().collect()
    }

    /// Find the first tool whose `tool_name` matches across all servers.
    ///
    /// When multiple servers expose a tool with the same name, the one from the
    /// lexicographically first server name is returned (deterministic order).
    #[must_use]
    pub fn find_tool(&self, tool_name: &str) -> Option<&ExternalTool> {
        // Iterate in sorted server order for determinism.
        let mut server_names: Vec<&str> =
            self.discovered_tools.keys().map(String::as_str).collect();
        server_names.sort_unstable();

        for name in server_names {
            if let Some(tools) = self.discovered_tools.get(name)
                && let Some(tool) = tools.iter().find(|t| t.tool_name == tool_name)
            {
                return Some(tool);
            }
        }
        None
    }

    /// Return all tools registered for the named server.
    ///
    /// Returns an empty slice when the server has no registered tools or is not
    /// configured.
    #[must_use]
    pub fn tools_for_server(&self, server_name: &str) -> Vec<&ExternalTool> {
        self.discovered_tools
            .get(server_name)
            .map(|tools| tools.iter().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_config(url: &str, trusted: bool) -> McpClientConfig {
        McpClientConfig {
            url: url.to_string(),
            description: String::new(),
            trusted,
        }
    }

    fn make_tool(server: &str, name: &str) -> ExternalTool {
        ExternalTool {
            server_name: server.to_string(),
            tool_name: name.to_string(),
            description: format!("Tool {name} on {server}"),
            input_schema: json!({"type": "object", "properties": {}}),
        }
    }

    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    #[test]
    fn new_with_empty_configs() {
        let mgr = McpClientManager::new(HashMap::new());
        assert!(mgr.server_names().is_empty());
        assert!(mgr.all_tools().is_empty());
    }

    // -------------------------------------------------------------------------
    // server_names()
    // -------------------------------------------------------------------------

    #[test]
    fn server_names_returns_configured_names() {
        let mut configs = HashMap::new();
        configs.insert("alpha".to_string(), make_config("http://alpha/sse", true));
        configs.insert("beta".to_string(), make_config("http://beta/sse", true));

        let mgr = McpClientManager::new(configs);
        let names = mgr.server_names();
        assert_eq!(names.len(), 2);
        // server_names() sorts alphabetically
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn server_names_sorted_alphabetically() {
        let mut configs = HashMap::new();
        configs.insert("zzz".to_string(), make_config("http://z/sse", true));
        configs.insert("aaa".to_string(), make_config("http://a/sse", true));
        configs.insert("mmm".to_string(), make_config("http://m/sse", true));

        let mgr = McpClientManager::new(configs);
        assert_eq!(mgr.server_names(), vec!["aaa", "mmm", "zzz"]);
    }

    // -------------------------------------------------------------------------
    // get_config()
    // -------------------------------------------------------------------------

    #[test]
    fn get_config_returns_some_for_known_server() {
        let mut configs = HashMap::new();
        configs.insert(
            "my-server".to_string(),
            make_config("http://my-server/sse", false),
        );

        let mgr = McpClientManager::new(configs);
        let cfg = mgr.get_config("my-server").expect("must find server");
        assert_eq!(cfg.url, "http://my-server/sse");
    }

    #[test]
    fn get_config_returns_none_for_unknown_server() {
        let mgr = McpClientManager::new(HashMap::new());
        assert!(mgr.get_config("nonexistent").is_none());
    }

    // -------------------------------------------------------------------------
    // is_trusted()
    // -------------------------------------------------------------------------

    #[test]
    fn is_trusted_returns_true_when_server_is_trusted() {
        let mut configs = HashMap::new();
        configs.insert("trusted".to_string(), make_config("http://t/sse", true));

        let mgr = McpClientManager::new(configs);
        assert!(mgr.is_trusted("trusted"));
    }

    #[test]
    fn is_trusted_returns_false_when_server_is_untrusted() {
        let mut configs = HashMap::new();
        configs.insert("untrusted".to_string(), make_config("http://u/sse", false));

        let mgr = McpClientManager::new(configs);
        assert!(!mgr.is_trusted("untrusted"));
    }

    #[test]
    fn is_trusted_returns_false_for_unconfigured_server() {
        let mgr = McpClientManager::new(HashMap::new());
        assert!(!mgr.is_trusted("ghost"));
    }

    // -------------------------------------------------------------------------
    // register_tools() + all_tools()
    // -------------------------------------------------------------------------

    #[test]
    fn register_tools_then_all_tools_returns_them() {
        let mut configs = HashMap::new();
        configs.insert("srv".to_string(), make_config("http://srv/sse", true));
        let mut mgr = McpClientManager::new(configs);

        mgr.register_tools(
            "srv",
            vec![make_tool("srv", "echo"), make_tool("srv", "ping")],
        );

        let tools = mgr.all_tools();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn register_tools_overwrites_previous_list() {
        let mut configs = HashMap::new();
        configs.insert("srv".to_string(), make_config("http://srv/sse", true));
        let mut mgr = McpClientManager::new(configs);

        mgr.register_tools("srv", vec![make_tool("srv", "old")]);
        mgr.register_tools(
            "srv",
            vec![make_tool("srv", "new1"), make_tool("srv", "new2")],
        );

        assert_eq!(mgr.all_tools().len(), 2);
        assert!(mgr.find_tool("old").is_none());
        assert!(mgr.find_tool("new1").is_some());
    }

    #[test]
    fn all_tools_aggregates_across_servers() {
        let mut configs = HashMap::new();
        configs.insert("s1".to_string(), make_config("http://s1/sse", true));
        configs.insert("s2".to_string(), make_config("http://s2/sse", true));
        let mut mgr = McpClientManager::new(configs);

        mgr.register_tools("s1", vec![make_tool("s1", "tool-a")]);
        mgr.register_tools(
            "s2",
            vec![make_tool("s2", "tool-b"), make_tool("s2", "tool-c")],
        );

        assert_eq!(mgr.all_tools().len(), 3);
    }

    // -------------------------------------------------------------------------
    // find_tool()
    // -------------------------------------------------------------------------

    #[test]
    fn find_tool_finds_across_servers() {
        let mut configs = HashMap::new();
        configs.insert("srv".to_string(), make_config("http://srv/sse", true));
        let mut mgr = McpClientManager::new(configs);

        mgr.register_tools("srv", vec![make_tool("srv", "calculator")]);

        let found = mgr.find_tool("calculator").expect("tool must be found");
        assert_eq!(found.tool_name, "calculator");
        assert_eq!(found.server_name, "srv");
    }

    #[test]
    fn find_tool_returns_none_for_missing_tool() {
        let mgr = McpClientManager::new(HashMap::new());
        assert!(mgr.find_tool("nonexistent").is_none());
    }

    #[test]
    fn find_tool_is_deterministic_with_multiple_servers() {
        // When two servers expose the same tool name the result should be stable.
        let mut configs = HashMap::new();
        configs.insert("aaa".to_string(), make_config("http://aaa/sse", true));
        configs.insert("zzz".to_string(), make_config("http://zzz/sse", true));
        let mut mgr = McpClientManager::new(configs);

        mgr.register_tools("aaa", vec![make_tool("aaa", "shared")]);
        mgr.register_tools("zzz", vec![make_tool("zzz", "shared")]);

        // "aaa" < "zzz" lexicographically — expect the aaa server's tool.
        let found = mgr.find_tool("shared").expect("must find a tool");
        assert_eq!(found.server_name, "aaa");
    }

    // -------------------------------------------------------------------------
    // tools_for_server()
    // -------------------------------------------------------------------------

    #[test]
    fn tools_for_server_returns_only_that_servers_tools() {
        let mut configs = HashMap::new();
        configs.insert("s1".to_string(), make_config("http://s1/sse", true));
        configs.insert("s2".to_string(), make_config("http://s2/sse", true));
        let mut mgr = McpClientManager::new(configs);

        mgr.register_tools("s1", vec![make_tool("s1", "x"), make_tool("s1", "y")]);
        mgr.register_tools("s2", vec![make_tool("s2", "z")]);

        let s1_tools = mgr.tools_for_server("s1");
        assert_eq!(s1_tools.len(), 2);
        assert!(s1_tools.iter().all(|t| t.server_name == "s1"));

        let s2_tools = mgr.tools_for_server("s2");
        assert_eq!(s2_tools.len(), 1);
        assert_eq!(s2_tools[0].tool_name, "z");
    }

    #[test]
    fn tools_for_server_returns_empty_for_unconfigured_server() {
        let mgr = McpClientManager::new(HashMap::new());
        assert!(mgr.tools_for_server("ghost").is_empty());
    }

    #[test]
    fn tools_for_server_returns_empty_before_registration() {
        let mut configs = HashMap::new();
        configs.insert("srv".to_string(), make_config("http://srv/sse", true));
        let mgr = McpClientManager::new(configs);

        // Server is configured but no tools registered yet.
        assert!(mgr.tools_for_server("srv").is_empty());
    }
}
