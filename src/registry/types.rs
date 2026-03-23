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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_defaults_yanked_and_omits_missing_timestamp() {
        let version: VersionInfo =
            serde_json::from_str(r#"{"version":"1.2.0","integrity":"sha256:abc123"}"#)
                .expect("version info should deserialize");

        assert_eq!(version.version, "1.2.0");
        assert_eq!(version.integrity, "sha256:abc123");
        assert!(!version.yanked);
        assert_eq!(version.published_at, None);

        let serialized = serde_json::to_value(&version).expect("version info should serialize");
        assert_eq!(serialized["version"], "1.2.0");
        assert_eq!(serialized["integrity"], "sha256:abc123");
        assert_eq!(serialized["yanked"], false);
        assert!(serialized.get("published_at").is_none());
    }

    #[test]
    fn module_info_preserves_optional_description_and_versions() {
        let module: ModuleInfo = serde_json::from_str(
            r#"{
                "name":"@duumbi/stdlib-math",
                "description":"Math helpers",
                "versions":[
                    {
                        "version":"1.2.0",
                        "integrity":"sha256:abc123",
                        "published_at":"2026-03-01T12:00:00Z"
                    }
                ]
            }"#,
        )
        .expect("module info should deserialize");

        assert_eq!(module.name, "@duumbi/stdlib-math");
        assert_eq!(module.description.as_deref(), Some("Math helpers"));
        assert_eq!(module.versions.len(), 1);
        assert_eq!(module.versions[0].version, "1.2.0");
        assert!(!module.versions[0].yanked);
        assert_eq!(
            module.versions[0].published_at.as_deref(),
            Some("2026-03-01T12:00:00Z")
        );
    }

    #[test]
    fn search_response_roundtrips_nested_hits() {
        let response = SearchResponse {
            results: vec![
                SearchHit {
                    name: "@duumbi/stdlib-string".to_string(),
                    description: Some("String utilities".to_string()),
                    latest_version: "1.4.0".to_string(),
                },
                SearchHit {
                    name: "@duumbi/stdlib-math".to_string(),
                    description: None,
                    latest_version: "1.2.0".to_string(),
                },
            ],
            total: 2,
        };

        let json = serde_json::to_string(&response).expect("search response should serialize");
        let parsed: SearchResponse =
            serde_json::from_str(&json).expect("search response should deserialize");

        assert_eq!(parsed.total, 2);
        assert_eq!(parsed.results.len(), 2);
        assert_eq!(parsed.results[0].name, "@duumbi/stdlib-string");
        assert_eq!(
            parsed.results[0].description.as_deref(),
            Some("String utilities")
        );
        assert_eq!(parsed.results[1].name, "@duumbi/stdlib-math");
        assert_eq!(parsed.results[1].description, None);
        assert_eq!(parsed.results[1].latest_version, "1.2.0");
    }

    #[test]
    fn publish_response_serializes_expected_fields() {
        let response = PublishResponse {
            name: "@duumbi/stdlib-io".to_string(),
            version: "2.0.0".to_string(),
        };

        let serialized =
            serde_json::to_value(&response).expect("publish response should serialize");
        assert_eq!(serialized["name"], "@duumbi/stdlib-io");
        assert_eq!(serialized["version"], "2.0.0");
    }
}
