//! Fallback provider chain for resilient LLM calls.
//!
//! [`ProviderChain`] wraps multiple [`LlmProvider`] instances and tries
//! each in order. Fallback triggers only on transient errors (network,
//! timeout, 5xx, 429). Auth and parse errors are returned immediately.

use std::future::Future;
use std::pin::Pin;

use crate::agents::{AgentError, LlmProvider};
use crate::patch::PatchOp;

/// A chain of LLM providers with automatic fallback on transient errors.
///
/// Providers are tried in order. If a provider fails with a transient error
/// (as determined by [`AgentError::is_transient`]), the next provider is
/// attempted. Non-transient errors are returned immediately.
pub struct ProviderChain {
    providers: Vec<Box<dyn LlmProvider>>,
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
        Self { providers }
    }
}

impl LlmProvider for ProviderChain {
    fn name(&self) -> &str {
        // Return the name of the first (primary) provider
        self.providers[0].name()
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
                    Ok(ops) => return Ok(ops),
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
                    Ok(ops) => return Ok(ops),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "at least one provider")]
    fn chain_panics_on_empty() {
        let _ = ProviderChain::new(vec![]);
    }
}
