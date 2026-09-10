//! Application-specific verification integrated with intent execution and repair.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use super::spec::IntentSpec;
use super::verifier::TestReport;

/// Behavioral checks supplied by an execution caller instead of i64 test cases.
pub trait ExternalVerifier: Send + Sync {
    /// Adds stable execution inputs before the intent is saved and hashed.
    fn prepare_spec(&self, spec: &mut IntentSpec);
    /// Verifies the authored workspace and retains implementation-specific evidence.
    #[must_use]
    fn verify<'a>(
        &'a mut self,
        workspace: &'a Path,
    ) -> Pin<Box<dyn Future<Output = TestReport> + Send + 'a>>;
    /// Whether graph repair can address the latest verification failure.
    #[must_use]
    fn repairable(&self) -> bool;
    /// Human-readable verifier identity for preflight and execution logs.
    #[must_use]
    fn kind(&self) -> &str;
}
