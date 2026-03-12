//! Registry HTTP client for fetching, publishing, and searching modules.
//!
//! Provides [`RegistryClient`] which handles all HTTP communication with
//! duumbi registries (public and private). Supports retry with exponential
//! backoff on transient failures (429, 5xx).

#[allow(dead_code)]
pub mod client;
#[allow(dead_code)]
pub mod types;

#[allow(unused_imports)]
pub use client::RegistryClient;
#[allow(unused_imports)]
pub use types::*;

use thiserror::Error;

use crate::errors::codes;

/// Errors produced by registry HTTP operations.
#[derive(Debug, Error)]
#[allow(dead_code)] // Progressively integrated as registry client pipeline is built
pub enum RegistryError {
    /// Registry server is unreachable (network error, DNS failure, timeout).
    #[error("Registry unreachable at '{url}': {source}")]
    Unreachable {
        /// Registry URL that failed.
        url: String,
        /// Underlying HTTP error.
        #[source]
        source: reqwest::Error,
    },

    /// Authentication failed (invalid, expired, or missing token).
    #[error("Authentication failed for registry '{registry}': {reason}")]
    AuthFailed {
        /// Registry name.
        registry: String,
        /// Human-readable reason.
        reason: String,
    },

    /// Requested version not found in the registry.
    #[error("Version '{version}' of '{module}' not found in registry '{registry}'")]
    VersionNotFound {
        /// Module name.
        module: String,
        /// Requested version or range.
        version: String,
        /// Registry name.
        registry: String,
    },

    /// Downloaded content integrity does not match expected hash.
    #[error("Integrity mismatch for '{module}': expected {expected}, got {actual}")]
    IntegrityMismatch {
        /// Module name.
        module: String,
        /// Expected integrity string.
        expected: String,
        /// Actual computed integrity string.
        actual: String,
    },

    /// Non-retryable HTTP error from the registry API.
    #[error("Registry API error (status {status}): {body}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Response body text.
        body: String,
    },

    /// I/O error during archive unpacking.
    #[error("Failed to unpack '{module}' to cache: {source}")]
    Unpack {
        /// Module name.
        module: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse a registry JSON response.
    #[error("Failed to parse registry response: {0}")]
    Parse(String),

    /// All retry attempts exhausted.
    #[error("Maximum retries ({max_retries}) exceeded for '{url}'")]
    RetriesExhausted {
        /// URL that was being retried.
        url: String,
        /// Number of retries attempted.
        max_retries: u32,
    },
}

impl RegistryError {
    /// Returns the structured error code for this error variant.
    #[must_use]
    #[allow(dead_code)]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Unreachable { .. } | Self::RetriesExhausted { .. } => {
                codes::E013_REGISTRY_UNREACHABLE
            }
            Self::AuthFailed { .. } => codes::E014_AUTH_FAILED,
            Self::IntegrityMismatch { .. } => codes::E015_INTEGRITY_MISMATCH,
            Self::VersionNotFound { .. } => codes::E016_VERSION_NOT_FOUND,
            Self::ApiError { .. } | Self::Parse(_) | Self::Unpack { .. } => {
                codes::E013_REGISTRY_UNREACHABLE
            }
        }
    }
}
