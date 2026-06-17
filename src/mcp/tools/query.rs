//! MCP Query mode tool.
//!
//! This module exposes DUUMBI Query mode through MCP while preserving the
//! read-only behavior of the existing Query engine. Normal operation uses the
//! configured provider chain. Tests call the provider-injected helper so CI
//! does not require live provider credentials or network access.

use std::future::Future;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::agents::LlmProvider;
use crate::agents::factory;
use crate::config;
use crate::query::{QueryEngine, QueryRequest};

/// Answers a read-only Query mode question through MCP.
pub fn query_ask(workspace: &Path, params: &Value) -> Result<Value, String> {
    let params = QueryAskParams::parse(params)?;
    let config = config::load_effective_config(workspace)
        .map_err(|error| format!("Provider configuration unavailable: {error}"))?;
    let providers = config.config.effective_providers();
    let provider = factory::create_available_provider_chain_for_global_access_context(
        &providers,
        &Default::default(),
    )
    .map_err(|error| format!("Provider unavailable for query_ask: {error}"))?;
    block_on_query(query_ask_with_provider(
        workspace,
        params,
        provider.as_ref(),
    ))
}

/// Executes Query mode with an injected provider.
pub async fn query_ask_with_provider(
    workspace: &Path,
    params: QueryAskParams,
    provider: &dyn LlmProvider,
) -> Result<Value, String> {
    let mut request = QueryRequest::new(workspace, params.question);
    request.visible_module = params.module;
    request.c4_level = params.c4_level;

    let answer = QueryEngine::new()
        .answer_streaming(provider, request, &|_| {})
        .await
        .map_err(|error| format!("Query failed: {error}"))?;
    let sources = if params.include_sources {
        serde_json::to_value(&answer.sources)
            .map_err(|error| format!("Query sources serialization failed: {error}"))?
    } else {
        Value::Array(Vec::new())
    };
    let suggested_handoff = serde_json::to_value(&answer.suggested_handoff)
        .map_err(|error| format!("Query handoff serialization failed: {error}"))?;

    Ok(serde_json::json!({
        "status": "success",
        "scope": "query_ask",
        "answer": answer.text,
        "model": answer.model,
        "confidence": answer.confidence,
        "sources": sources,
        "suggested_handoff": suggested_handoff,
        "evidence": [],
        "read_only": true,
    }))
}

/// Parameters for `query_ask`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryAskParams {
    /// User question to answer.
    pub question: String,
    /// Optional visible graph module name.
    #[serde(default)]
    pub module: Option<String>,
    /// Optional visible C4 level.
    #[serde(default)]
    pub c4_level: Option<String>,
    /// Whether source references should be returned.
    #[serde(default = "default_include_sources")]
    pub include_sources: bool,
}

impl QueryAskParams {
    fn parse(params: &Value) -> Result<Self, String> {
        let parsed: Self = serde_json::from_value(params.clone())
            .map_err(|error| format!("Invalid query_ask arguments: {error}"))?;
        if parsed.question.trim().is_empty() {
            return Err("Invalid query_ask arguments: question must not be empty".to_string());
        }
        Ok(Self {
            question: parsed.question.trim().to_string(),
            module: parsed.module.filter(|value| !value.trim().is_empty()),
            c4_level: parsed.c4_level.filter(|value| !value.trim().is_empty()),
            include_sources: parsed.include_sources,
        })
    }
}

fn default_include_sources() -> bool {
    true
}

fn block_on_query<F>(future: F) -> Result<Value, String>
where
    F: Future<Output = Result<Value, String>> + Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => std::thread::scope(|scope| {
            scope
                .spawn(move || handle.block_on(future))
                .join()
                .map_err(|_| "Query runtime thread panicked".to_string())?
        }),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Cannot create query runtime: {error}"))?
            .block_on(future),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentError;
    use crate::patch::PatchOp;
    use std::pin::Pin;
    use tempfile::TempDir;

    const VALID_GRAPH: &str = r#"{
        "@context": {"duumbi": "https://duumbi.dev/ns/core#"},
        "@type": "duumbi:Module",
        "@id": "duumbi:main",
        "duumbi:name": "main",
        "duumbi:functions": [{
            "@type": "duumbi:Function",
            "@id": "duumbi:main/main",
            "duumbi:name": "main",
            "duumbi:params": [],
            "duumbi:returnType": "i64",
            "duumbi:blocks": [{
                "@type": "duumbi:Block",
                "@id": "duumbi:main/main/entry",
                "duumbi:label": "entry",
                "duumbi:ops": [{
                    "@type": "duumbi:Return",
                    "@id": "duumbi:main/main/entry/0",
                    "duumbi:operand": {"@id": "duumbi:main/main/entry/1"}
                }, {
                    "@type": "duumbi:Const",
                    "@id": "duumbi:main/main/entry/1",
                    "duumbi:value": 0,
                    "duumbi:resultType": "i64"
                }]
            }]
        }]
    }"#;

    struct MockProvider;

    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn model_name(&self) -> Option<&str> {
            Some("query-test")
        }

        fn call_with_tools<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = std::result::Result<Vec<PatchOp>, AgentError>> + Send + 'a>>
        {
            Box::pin(async { Err(AgentError::NoToolCalls) })
        }

        fn call_with_tools_streaming<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
            _on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = std::result::Result<Vec<PatchOp>, AgentError>> + Send + 'a>>
        {
            Box::pin(async { Err(AgentError::NoToolCalls) })
        }

        fn answer_streaming<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
            on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = std::result::Result<String, AgentError>> + Send + 'a>>
        {
            on_text("The main module returns zero.");
            Box::pin(async { Ok("The main module returns zero.".to_string()) })
        }
    }

    fn workspace() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let graph_dir = dir.path().join(".duumbi/graph");
        std::fs::create_dir_all(&graph_dir).expect("create graph");
        std::fs::write(graph_dir.join("main.jsonld"), VALID_GRAPH).expect("write graph");
        std::fs::write(
            dir.path().join(".duumbi/config.toml"),
            "[workspace]\nname = \"test\"\n",
        )
        .expect("write config");
        dir
    }

    #[tokio::test]
    async fn query_ask_with_mock_provider_is_read_only() {
        let dir = workspace();
        let graph_path = dir.path().join(".duumbi/graph/main.jsonld");
        let config_path = dir.path().join(".duumbi/config.toml");
        let graph_before = std::fs::read(&graph_path).expect("read graph before");
        let config_before = std::fs::read(&config_path).expect("read config before");

        let answer = query_ask_with_provider(
            dir.path(),
            QueryAskParams::parse(&serde_json::json!({
                "question": "What does main return?",
                "include_sources": true,
            }))
            .expect("parse params"),
            &MockProvider,
        )
        .await
        .expect("query answer");

        assert_eq!(answer["status"], "success");
        assert_eq!(answer["model"], "mock/query-test");
        assert_eq!(answer["read_only"], true);
        assert!(
            answer["sources"]
                .as_array()
                .expect("sources")
                .iter()
                .any(|source| source["kind"] == "workspace_summary")
        );
        assert_eq!(
            std::fs::read(&graph_path).expect("read graph"),
            graph_before
        );
        assert_eq!(
            std::fs::read(&config_path).expect("read config"),
            config_before
        );
    }

    #[tokio::test]
    async fn mutation_shaped_query_returns_handoff_without_writes() {
        let dir = workspace();
        let graph_path = dir.path().join(".duumbi/graph/main.jsonld");
        let before = std::fs::read(&graph_path).expect("read graph before");

        let answer = query_ask_with_provider(
            dir.path(),
            QueryAskParams::parse(&serde_json::json!({
                "question": "Add a function that doubles an integer"
            }))
            .expect("parse params"),
            &MockProvider,
        )
        .await
        .expect("query answer");

        assert_eq!(answer["suggested_handoff"]["mode"], "agent");
        assert_eq!(std::fs::read(&graph_path).expect("read graph"), before);
    }
}
