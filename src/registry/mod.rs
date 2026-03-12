//! Registry HTTP client for fetching, publishing, and searching modules.
//!
//! Provides [`RegistryClient`] which handles all HTTP communication with
//! duumbi registries (public and private). Supports retry with exponential
//! backoff on transient failures (429, 5xx).

#[allow(dead_code)]
pub mod client;
pub mod credentials;
#[allow(dead_code)]
pub mod package;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_error_display_messages() {
        let err = RegistryError::AuthFailed {
            registry: "duumbi".to_string(),
            reason: "token expired".to_string(),
        };
        assert!(err.to_string().contains("duumbi"));
        assert!(err.to_string().contains("token expired"));

        let err = RegistryError::VersionNotFound {
            module: "@test/mod".to_string(),
            version: "^2.0".to_string(),
            registry: "main".to_string(),
        };
        assert!(err.to_string().contains("@test/mod"));
        assert!(err.to_string().contains("^2.0"));

        let err = RegistryError::IntegrityMismatch {
            module: "@test/mod".to_string(),
            expected: "sha256:aaa".to_string(),
            actual: "sha256:bbb".to_string(),
        };
        assert!(err.to_string().contains("sha256:aaa"));
        assert!(err.to_string().contains("sha256:bbb"));

        let err = RegistryError::RetriesExhausted {
            url: "https://registry.duumbi.dev/api/v1/modules/test".to_string(),
            max_retries: 3,
        };
        assert!(err.to_string().contains("3"));

        let err = RegistryError::ApiError {
            status: 500,
            body: "Internal Server Error".to_string(),
        };
        assert!(err.to_string().contains("500"));

        let err = RegistryError::Parse("bad json".to_string());
        assert!(err.to_string().contains("bad json"));
    }

    #[test]
    fn api_error_and_parse_map_to_e013() {
        let err = RegistryError::ApiError {
            status: 500,
            body: "error".to_string(),
        };
        assert_eq!(err.error_code(), "E013");

        let err = RegistryError::Parse("bad".to_string());
        assert_eq!(err.error_code(), "E013");
    }

    #[test]
    fn retries_exhausted_maps_to_e013() {
        let err = RegistryError::RetriesExhausted {
            url: "https://registry.duumbi.dev".to_string(),
            max_retries: 3,
        };
        assert_eq!(err.error_code(), "E013");
    }

    #[test]
    fn unpack_error_maps_to_e013() {
        let err = RegistryError::Unpack {
            module: "test".to_string(),
            source: std::io::Error::other("test"),
        };
        assert_eq!(err.error_code(), "E013");
    }
}
