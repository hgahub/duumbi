//! Fallback provider chain for resilient LLM calls.
//!
//! [`ProviderChain`] wraps multiple [`LlmProvider`] instances and tries
//! each in order. Fallback triggers only on transient errors (network,
//! timeout, 5xx, 429). Auth and parse errors are returned immediately.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use crate::agents::{AgentError, LlmProvider};
use crate::patch::PatchOp;

/// A chain of LLM providers with automatic fallback on transient errors.
///
/// Providers are tried in order. If a provider fails with a transient error
/// (as determined by [`AgentError::is_transient`]), the next provider is
/// attempted. Non-transient errors are returned immediately.
pub struct ProviderChain {
    providers: Vec<Box<dyn LlmProvider>>,
    last_success_model_label: Mutex<Option<String>>,
}

impl ProviderChain {
    /// Creates a new provider chain from a list of providers.
    ///
    /// The first provider is the primary; subsequent providers are fallbacks.
    ///
    /// # Panics
    ///
    /// Panics if `providers` is empty.
    #[must_use]
    pub fn new(providers: Vec<Box<dyn LlmProvider>>) -> Self {
        assert!(
            !providers.is_empty(),
            "invariant: ProviderChain must have at least one provider"
        );
        Self {
            providers,
            last_success_model_label: Mutex::new(None),
        }
    }

    fn remember_success(&self, provider: &dyn LlmProvider) {
        if let Ok(mut last_success) = self.last_success_model_label.lock() {
            *last_success = Some(provider.model_label());
        }
    }
}

impl LlmProvider for ProviderChain {
    fn name(&self) -> &str {
        // Return the name of the first (primary) provider
        self.providers[0].name()
    }

    fn model_name(&self) -> Option<&str> {
        self.providers[0].model_name()
    }

    fn model_label(&self) -> String {
        self.last_success_model_label
            .lock()
            .ok()
            .and_then(|last_success| last_success.clone())
            .unwrap_or_else(|| self.providers[0].model_label())
    }

    fn call_with_tools<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let mut last_error: Option<AgentError> = None;

            for (i, provider) in self.providers.iter().enumerate() {
                match provider.call_with_tools(system_prompt, user_message).await {
                    Ok(ops) => {
                        self.remember_success(provider.as_ref());
                        return Ok(ops);
                    }
                    Err(e) => {
                        if !e.is_transient() || i + 1 == self.providers.len() {
                            return Err(e);
                        }
                        eprintln!(
                            "Provider '{}' failed (transient: {}), trying '{}'…",
                            provider.name(),
                            e,
                            self.providers[i + 1].name()
                        );
                        last_error = Some(e);
                    }
                }
            }

            // Should be unreachable due to the loop logic above,
            // but return the last error if somehow we get here.
            Err(last_error.expect("invariant: at least one provider must have been tried"))
        })
    }

    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let mut last_error: Option<AgentError> = None;

            for (i, provider) in self.providers.iter().enumerate() {
                match provider
                    .call_with_tools_streaming(system_prompt, user_message, on_text)
                    .await
                {
                    Ok(ops) => {
                        self.remember_success(provider.as_ref());
                        return Ok(ops);
                    }
                    Err(e) => {
                        if !e.is_transient() || i + 1 == self.providers.len() {
                            return Err(e);
                        }
                        eprintln!(
                            "Provider '{}' failed (transient: {}), trying '{}'…",
                            provider.name(),
                            e,
                            self.providers[i + 1].name()
                        );
                        last_error = Some(e);
                    }
                }
            }

            Err(last_error.expect("invariant: at least one provider must have been tried"))
        })
    }

    fn answer<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let mut last_error: Option<AgentError> = None;

            for (i, provider) in self.providers.iter().enumerate() {
                match provider.answer(system_prompt, user_message).await {
                    Ok(answer) => {
                        self.remember_success(provider.as_ref());
                        return Ok(answer);
                    }
                    Err(e) => {
                        if !e.is_transient() || i + 1 == self.providers.len() {
                            return Err(e);
                        }
                        eprintln!(
                            "Provider '{}' failed (transient: {}), trying '{}'...",
                            provider.name(),
                            e,
                            self.providers[i + 1].name()
                        );
                        last_error = Some(e);
                    }
                }
            }

            Err(last_error.expect("invariant: at least one provider must have been tried"))
        })
    }

    fn answer_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let mut last_error: Option<AgentError> = None;

            for (i, provider) in self.providers.iter().enumerate() {
                match provider
                    .answer_streaming(system_prompt, user_message, on_text)
                    .await
                {
                    Ok(answer) => {
                        self.remember_success(provider.as_ref());
                        return Ok(answer);
                    }
                    Err(e) => {
                        if !e.is_transient() || i + 1 == self.providers.len() {
                            return Err(e);
                        }
                        eprintln!(
                            "Provider '{}' failed (transient: {}), trying '{}'...",
                            provider.name(),
                            e,
                            self.providers[i + 1].name()
                        );
                        last_error = Some(e);
                    }
                }
            }

            Err(last_error.expect("invariant: at least one provider must have been tried"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        name: &'static str,
        model: &'static str,
        answer: Result<&'static str, AgentError>,
    }

    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn model_name(&self) -> Option<&str> {
            Some(self.model)
        }

        fn call_with_tools<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn call_with_tools_streaming<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
            _on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn answer<'a>(
            &'a self,
            _system_prompt: &'a str,
            _user_message: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
            Box::pin(async move {
                self.answer
                    .as_ref()
                    .map(|answer| (*answer).to_string())
                    .map_err(clone_agent_error)
            })
        }

        fn answer_streaming<'a>(
            &'a self,
            system_prompt: &'a str,
            user_message: &'a str,
            on_text: &'a (dyn Fn(&str) + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = Result<String, AgentError>> + Send + 'a>> {
            Box::pin(async move {
                let answer = self.answer(system_prompt, user_message).await?;
                on_text(&answer);
                Ok(answer)
            })
        }
    }

    fn clone_agent_error(error: &AgentError) -> AgentError {
        match error {
            AgentError::Timeout(seconds) => AgentError::Timeout(*seconds),
            AgentError::NoToolCalls => AgentError::NoToolCalls,
            other => AgentError::Parse(other.to_string()),
        }
    }

    #[test]
    #[should_panic(expected = "at least one provider")]
    fn chain_panics_on_empty() {
        let _ = ProviderChain::new(vec![]);
    }

    #[tokio::test]
    async fn answer_falls_back_after_transient_error() {
        let chain = ProviderChain::new(vec![
            Box::new(MockProvider {
                name: "primary",
                model: "primary-model",
                answer: Err(AgentError::Timeout(1)),
            }),
            Box::new(MockProvider {
                name: "fallback",
                model: "fallback-model",
                answer: Ok("grounded answer"),
            }),
        ]);

        let answer = chain.answer("system", "question").await.expect("answer");

        assert_eq!(answer, "grounded answer");
        assert_eq!(chain.model_label(), "fallback/fallback-model");
    }

    #[tokio::test]
    async fn answer_does_not_fallback_after_non_transient_error() {
        let chain = ProviderChain::new(vec![
            Box::new(MockProvider {
                name: "primary",
                model: "primary-model",
                answer: Err(AgentError::NoToolCalls),
            }),
            Box::new(MockProvider {
                name: "fallback",
                model: "fallback-model",
                answer: Ok("should not be used"),
            }),
        ]);

        let error = chain.answer("system", "question").await.expect_err("error");

        assert!(matches!(error, AgentError::NoToolCalls));
    }
}
