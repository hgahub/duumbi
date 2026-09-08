//! Opt-in capturing decorator for `--capture-model-io`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use crate::agents::{AgentError, CapturePayloadStatus, CapturedProviderCall, LlmProvider};
use crate::intent::attempt::CapturedModelIo;
use crate::knowledge::learning::redact_secret_text;
use crate::patch::PatchOp;

/// Wraps a provider and records redacted current-attempt model I/O.
pub(crate) struct CapturingProvider<'a> {
    inner: &'a dyn LlmProvider,
    payloads: Mutex<Vec<CapturedModelIo>>,
}

impl<'a> CapturingProvider<'a> {
    /// Decorates `inner` for one isolated attempt.
    #[must_use]
    pub(crate) fn new(inner: &'a dyn LlmProvider) -> Self {
        Self {
            inner,
            payloads: Mutex::new(Vec::new()),
        }
    }

    /// Takes recorded, redacted payloads.
    #[must_use]
    pub(crate) fn take_payloads(&self) -> Vec<CapturedModelIo> {
        self.payloads
            .lock()
            .expect("invariant: capturing mutex")
            .drain(..)
            .collect()
    }

    fn record(&self, captured: &CapturedProviderCall) {
        if captured.payload_status != CapturePayloadStatus::Captured {
            return;
        }
        let mut payloads = self.payloads.lock().expect("invariant: capturing mutex");
        let index = payloads.len() / 2 + 1;
        payloads.push(CapturedModelIo {
            name: format!("request-{index}.txt"),
            body: redact_secret_text(&captured.request_prompt),
        });
        if let Some(raw) = captured.raw_response.as_deref() {
            payloads.push(CapturedModelIo {
                name: format!("response-{index}.txt"),
                body: redact_secret_text(raw),
            });
        }
    }
}

impl LlmProvider for CapturingProvider<'_> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model_name(&self) -> Option<&str> {
        self.inner.model_name()
    }

    fn model_label(&self) -> String {
        self.inner.model_label()
    }

    fn call_with_tools<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            Ok(self
                .call_with_tools_captured(system_prompt, user_message)
                .await?
                .ops)
        })
    }

    fn call_with_tools_streaming<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
        _on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        self.call_with_tools(system_prompt, user_message)
    }

    fn call_with_tools_captured<'a>(
        &'a self,
        system_prompt: &'a str,
        user_message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CapturedProviderCall, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let captured = self
                .inner
                .call_with_tools_captured(system_prompt, user_message)
                .await?;
            self.record(&captured);
            Ok(captured)
        })
    }
}
