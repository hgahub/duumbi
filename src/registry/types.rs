//! Response types for the registry HTTP API.
//!
//! These DTOs mirror the JSON responses from registry endpoints. All fields
//! use `serde` for automatic (de)serialization.

use serde::{Deserialize, Serialize};

/// Metadata for a single published version of a module.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionInfo {
    /// Semantic version string, e.g. `"1.2.0"`.
    pub version: String,
    /// SHA-256 integrity hash of the `.tar.gz` archive (`"sha256:<hex>"`).
    pub integrity: String,
    /// Whether this version has been yanked.
    #[serde(default)]
    pub yanked: bool,
    /// ISO-8601 publication timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

/// Full module metadata returned by `GET /api/v1/modules/@scope/name`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModuleInfo {
    /// Scoped module name, e.g. `"@duumbi/stdlib-math"`.
    pub name: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// All published versions (newest first by convention).
    pub versions: Vec<VersionInfo>,
}

/// A single search result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchHit {
    /// Scoped module name.
    pub name: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Latest non-yanked version.
    pub latest_version: String,
}

/// Response from `GET /api/v1/search?q=...`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    /// Matching modules.
    pub results: Vec<SearchHit>,
    /// Total number of matches (for pagination).
    pub total: u64,
}

/// Response from `PUT /api/v1/modules/@scope/name` (publish).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublishResponse {
    /// Scoped module name.
    pub name: String,
    /// Published version.
    pub version: String,
}
