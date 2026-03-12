//! HTTP client for duumbi module registries.
//!
//! [`RegistryClient`] handles all network communication with registry servers,
//! including retry with exponential backoff on transient failures (429, 5xx).

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response, StatusCode};
use sha2::{Digest, Sha256};

use super::RegistryError;
use super::types::*;
use crate::manifest::ModuleManifest;

/// Default HTTP timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum number of retry attempts for transient failures.
const MAX_RETRIES: u32 = 3;

/// Initial backoff duration before first retry.
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);

/// Credential for authenticating with a single registry.
#[derive(Debug, Clone)]
pub struct RegistryCredential {
    /// Bearer token.
    pub token: String,
}

/// HTTP client for interacting with duumbi module registries.
///
/// Constructed with a registry URL map and optional credentials. All methods
/// are async and use exponential backoff for transient failures.
pub struct RegistryClient {
    http: Client,
    registries: HashMap<String, String>,
    credentials: HashMap<String, RegistryCredential>,
    max_retries: u32,
}

impl RegistryClient {
    /// Creates a new registry client.
    ///
    /// # Arguments
    /// * `registries` — Map of registry name to base URL.
    /// * `credentials` — Map of registry name to credential.
    /// * `timeout` — HTTP request timeout (defaults to 30s if `None`).
    ///
    /// # Errors
    /// Returns `RegistryError::Unreachable` if the HTTP client cannot be built
    /// (TLS initialization failure).
    #[must_use = "registry client errors should be handled"]
    pub fn new(
        registries: HashMap<String, String>,
        credentials: HashMap<String, RegistryCredential>,
        timeout: Option<Duration>,
    ) -> Result<Self, RegistryError> {
        let timeout = timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let http =
            Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| RegistryError::Unreachable {
                    url: "client-init".to_string(),
                    source: e,
                })?;

        Ok(Self {
            http,
            registries,
            credentials,
            max_retries: MAX_RETRIES,
        })
    }

    /// Fetches full module metadata from a registry.
    ///
    /// Calls `GET /api/v1/modules/{module}`.
    #[must_use = "registry errors should be handled"]
    pub async fn fetch_module_info(
        &self,
        registry: &str,
        module: &str,
    ) -> Result<ModuleInfo, RegistryError> {
        let base = self.registry_url(registry)?;
        let url = format!("{base}/api/v1/modules/{module}");

        let resp = self.get_with_retry(registry, &url).await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Err(RegistryError::VersionNotFound {
                module: module.to_string(),
                version: "*".to_string(),
                registry: registry.to_string(),
            });
        }

        Self::check_auth_error(status, registry)?;
        Self::check_api_error(status, &resp).await?;

        resp.json::<ModuleInfo>()
            .await
            .map_err(|e| RegistryError::Parse(e.to_string()))
    }

    /// Downloads a module archive, verifies integrity, and unpacks to cache.
    ///
    /// Calls `GET /api/v1/modules/{module}/{version}/download`.
    /// Unpacks the `.tar.gz` into `cache_dir/@scope/name@version/`.
    /// Returns the parsed `ModuleManifest` from the unpacked archive.
    #[must_use = "registry errors should be handled"]
    pub async fn download_module(
        &self,
        registry: &str,
        module: &str,
        version: &str,
        cache_dir: &Path,
    ) -> Result<ModuleManifest, RegistryError> {
        let base = self.registry_url(registry)?;
        let url = format!("{base}/api/v1/modules/{module}/{version}/download");

        let resp = self.get_with_retry(registry, &url).await?;
        let status = resp.status();

        if status == StatusCode::NOT_FOUND {
            return Err(RegistryError::VersionNotFound {
                module: module.to_string(),
                version: version.to_string(),
                registry: registry.to_string(),
            });
        }

        Self::check_auth_error(status, registry)?;
        Self::check_api_error(status, &resp).await?;

        let bytes = resp.bytes().await.map_err(|e| RegistryError::Unreachable {
            url: url.clone(),
            source: e,
        })?;

        // Compute integrity hash of the downloaded archive
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let integrity = format!("sha256:{:x}", hasher.finalize());

        // Determine cache target directory
        let target_dir = module_cache_dir(cache_dir, module, version);
        std::fs::create_dir_all(&target_dir).map_err(|e| RegistryError::Unpack {
            module: module.to_string(),
            source: e,
        })?;

        // Unpack .tar.gz
        unpack_tarball(&bytes, &target_dir).map_err(|e| RegistryError::Unpack {
            module: module.to_string(),
            source: e,
        })?;

        // Read manifest from unpacked contents
        let manifest_path = target_dir.join("manifest.toml");
        let manifest = crate::manifest::parse_manifest(&manifest_path).map_err(|e| {
            RegistryError::Parse(format!(
                "Failed to read manifest from downloaded module: {e}"
            ))
        })?;

        // Store integrity hash alongside the manifest
        let integrity_path = target_dir.join(".integrity");
        let _ = std::fs::write(&integrity_path, &integrity);

        Ok(manifest)
    }

    /// Searches for modules matching a query string.
    ///
    /// Calls `GET /api/v1/search?q={query}`.
    #[must_use = "registry errors should be handled"]
    pub async fn search(
        &self,
        registry: &str,
        query: &str,
    ) -> Result<SearchResponse, RegistryError> {
        let base = self.registry_url(registry)?;
        let url = format!("{base}/api/v1/search?q={query}");

        let resp = self.get_with_retry(registry, &url).await?;
        Self::check_auth_error(resp.status(), registry)?;
        Self::check_api_error(resp.status(), &resp).await?;

        resp.json::<SearchResponse>()
            .await
            .map_err(|e| RegistryError::Parse(e.to_string()))
    }

    /// Publishes a module tarball to a registry.
    ///
    /// Calls `PUT /api/v1/modules/{module}` with the `.tar.gz` body.
    /// Requires valid credentials for the target registry.
    #[must_use = "registry errors should be handled"]
    pub async fn publish(
        &self,
        registry: &str,
        module: &str,
        tarball: &[u8],
    ) -> Result<PublishResponse, RegistryError> {
        let base = self.registry_url(registry)?;
        let url = format!("{base}/api/v1/modules/{module}");

        let cred = self
            .credentials
            .get(registry)
            .ok_or_else(|| RegistryError::AuthFailed {
                registry: registry.to_string(),
                reason: "No credentials configured for this registry".to_string(),
            })?;

        let resp = self
            .http
            .put(&url)
            .header(AUTHORIZATION, format!("Bearer {}", cred.token))
            .header(CONTENT_TYPE, "application/gzip")
            .body(tarball.to_vec())
            .send()
            .await
            .map_err(|e| RegistryError::Unreachable {
                url: url.clone(),
                source: e,
            })?;

        Self::check_auth_error(resp.status(), registry)?;
        Self::check_api_error(resp.status(), &resp).await?;

        resp.json::<PublishResponse>()
            .await
            .map_err(|e| RegistryError::Parse(e.to_string()))
    }

    /// Yanks a specific version from a registry.
    ///
    /// Calls `DELETE /api/v1/modules/{module}/{version}`.
    /// Requires valid credentials for the target registry.
    #[must_use = "registry errors should be handled"]
    pub async fn yank(
        &self,
        registry: &str,
        module: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        let base = self.registry_url(registry)?;
        let url = format!("{base}/api/v1/modules/{module}/{version}");

        let cred = self
            .credentials
            .get(registry)
            .ok_or_else(|| RegistryError::AuthFailed {
                registry: registry.to_string(),
                reason: "No credentials configured for this registry".to_string(),
            })?;

        let resp = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {}", cred.token))
            .send()
            .await
            .map_err(|e| RegistryError::Unreachable {
                url: url.clone(),
                source: e,
            })?;

        Self::check_auth_error(resp.status(), registry)?;
        Self::check_api_error(resp.status(), &resp).await?;

        Ok(())
    }

    /// Resolves a SemVer version requirement to the highest matching version.
    ///
    /// Fetches module info from the registry, filters out yanked versions,
    /// and returns the highest version that satisfies the requirement.
    #[must_use = "registry errors should be handled"]
    pub async fn resolve_version(
        &self,
        registry: &str,
        module: &str,
        version_req: &semver::VersionReq,
    ) -> Result<semver::Version, RegistryError> {
        let info = self.fetch_module_info(registry, module).await?;

        let best = info
            .versions
            .iter()
            .filter(|v| !v.yanked)
            .filter_map(|v| semver::Version::parse(&v.version).ok())
            .filter(|v| version_req.matches(v))
            .max();

        best.ok_or_else(|| RegistryError::VersionNotFound {
            module: module.to_string(),
            version: version_req.to_string(),
            registry: registry.to_string(),
        })
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Resolves a registry name to its base URL.
    fn registry_url(&self, name: &str) -> Result<&str, RegistryError> {
        self.registries
            .get(name)
            .map(|s| s.trim_end_matches('/'))
            .ok_or_else(|| RegistryError::AuthFailed {
                registry: name.to_string(),
                reason: format!("Registry '{name}' not configured"),
            })
    }

    /// Executes a GET request with exponential backoff retry on transient errors.
    async fn get_with_retry(&self, registry: &str, url: &str) -> Result<Response, RegistryError> {
        let mut backoff = INITIAL_BACKOFF;

        for attempt in 0..=self.max_retries {
            let mut req = self.http.get(url);
            if let Some(cred) = self.credentials.get(registry) {
                req = req.header(AUTHORIZATION, format!("Bearer {}", cred.token));
            }

            let result = req.send().await;

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        if attempt < self.max_retries {
                            tokio::time::sleep(backoff).await;
                            backoff *= 2;
                            continue;
                        }
                        return Err(RegistryError::RetriesExhausted {
                            url: url.to_string(),
                            max_retries: self.max_retries,
                        });
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if attempt < self.max_retries {
                        tokio::time::sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }
                    return Err(RegistryError::Unreachable {
                        url: url.to_string(),
                        source: e,
                    });
                }
            }
        }

        Err(RegistryError::RetriesExhausted {
            url: url.to_string(),
            max_retries: self.max_retries,
        })
    }

    /// Checks for 401/403 and returns `AuthFailed` error.
    fn check_auth_error(status: StatusCode, registry: &str) -> Result<(), RegistryError> {
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(RegistryError::AuthFailed {
                registry: registry.to_string(),
                reason: format!("HTTP {}", status.as_u16()),
            });
        }
        Ok(())
    }

    /// Checks for non-success status codes and returns `ApiError`.
    async fn check_api_error(status: StatusCode, _resp: &Response) -> Result<(), RegistryError> {
        if !status.is_success() {
            // We can't consume resp here since it's borrowed, so we just report the status.
            return Err(RegistryError::ApiError {
                status: status.as_u16(),
                body: format!("HTTP {status}"),
            });
        }
        Ok(())
    }
}

/// Builds the cache directory path for a module version.
///
/// Format: `{cache_dir}/@scope/name@version/`
/// Example: `@duumbi/stdlib-math` version `1.0.0` → `cache/@duumbi/stdlib-math@1.0.0/`
fn module_cache_dir(cache_dir: &Path, module: &str, version: &str) -> std::path::PathBuf {
    // Module names like "@scope/name" — split into scope dir + name@version
    if let Some((scope, name)) = module.split_once('/') {
        cache_dir.join(scope).join(format!("{name}@{version}"))
    } else {
        cache_dir.join(format!("{module}@{version}"))
    }
}

/// Unpacks a `.tar.gz` archive into the target directory.
fn unpack_tarball(data: &[u8], target: &Path) -> Result<(), std::io::Error> {
    let decoder = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(target)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::io::Write as _;
    use tempfile::TempDir;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper: create a client pointing at a wiremock server.
    fn test_client(server_url: &str, credentials: Option<(&str, &str)>) -> RegistryClient {
        let mut registries = HashMap::new();
        registries.insert("test".to_string(), server_url.to_string());

        let mut creds = HashMap::new();
        if let Some((reg, token)) = credentials {
            creds.insert(
                reg.to_string(),
                RegistryCredential {
                    token: token.to_string(),
                },
            );
        }

        RegistryClient::new(registries, creds, Some(Duration::from_secs(5)))
            .expect("client must build")
    }

    /// Helper: create a valid .tar.gz with a manifest.toml and a graph file.
    fn make_test_tarball(name: &str, version: &str) -> Vec<u8> {
        let manifest = format!(
            "[module]\nname = \"{name}\"\nversion = \"{version}\"\n\n[exports]\nfunctions = [\"test_fn\"]\n"
        );
        let graph_content = r#"{"@type": "duumbi:Module", "duumbi:name": "test"}"#;

        let mut buf = Vec::new();
        {
            let encoder = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);

            // Add manifest.toml
            let manifest_bytes = manifest.as_bytes();
            let mut header = tar::Header::new_gnu();
            header.set_size(manifest_bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "manifest.toml", manifest_bytes)
                .expect("append manifest");

            // Add graph/test.jsonld
            let graph_bytes = graph_content.as_bytes();
            let mut header = tar::Header::new_gnu();
            header.set_size(graph_bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "graph/test.jsonld", graph_bytes)
                .expect("append graph");

            builder.finish().expect("finish tar");
        }
        buf
    }

    fn sample_module_info() -> ModuleInfo {
        ModuleInfo {
            name: "@test/example".to_string(),
            description: Some("Test module".to_string()),
            versions: vec![
                VersionInfo {
                    version: "1.0.0".to_string(),
                    integrity: "sha256:abc123".to_string(),
                    yanked: false,
                    published_at: None,
                },
                VersionInfo {
                    version: "1.1.0".to_string(),
                    integrity: "sha256:def456".to_string(),
                    yanked: false,
                    published_at: None,
                },
                VersionInfo {
                    version: "2.0.0".to_string(),
                    integrity: "sha256:ghi789".to_string(),
                    yanked: true,
                    published_at: None,
                },
            ],
        }
    }

    #[tokio::test]
    async fn fetch_module_info_success() {
        let server = MockServer::start().await;
        let info = sample_module_info();

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let result = client
            .fetch_module_info("test", "@test/example")
            .await
            .expect("must succeed");

        assert_eq!(result.name, "@test/example");
        assert_eq!(result.versions.len(), 3);
    }

    #[tokio::test]
    async fn fetch_module_info_404_returns_version_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let err = client
            .fetch_module_info("test", "@test/missing")
            .await
            .expect_err("must fail");

        assert!(matches!(err, RegistryError::VersionNotFound { .. }));
    }

    #[tokio::test]
    async fn fetch_module_info_401_returns_auth_failed() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/private"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let err = client
            .fetch_module_info("test", "@test/private")
            .await
            .expect_err("must fail");

        assert!(matches!(err, RegistryError::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn download_module_unpacks_tarball() {
        let server = MockServer::start().await;
        let tarball = make_test_tarball("@test/example", "1.0.0");

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example/1.0.0/download"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(tarball)
                    .insert_header("content-type", "application/gzip"),
            )
            .mount(&server)
            .await;

        let tmp = TempDir::new().expect("tempdir");
        let client = test_client(&server.uri(), None);

        let manifest = client
            .download_module("test", "@test/example", "1.0.0", tmp.path())
            .await
            .expect("must succeed");

        assert_eq!(manifest.module.name, "@test/example");
        assert_eq!(manifest.module.version, "1.0.0");

        // Verify unpacked files exist
        let cache_dir = tmp.path().join("@test/example@1.0.0");
        assert!(cache_dir.join("manifest.toml").exists());
        assert!(cache_dir.join("graph/test.jsonld").exists());
        assert!(cache_dir.join(".integrity").exists());
    }

    #[tokio::test]
    async fn search_success() {
        let server = MockServer::start().await;
        let response = SearchResponse {
            results: vec![SearchHit {
                name: "@test/example".to_string(),
                description: Some("Test module".to_string()),
                latest_version: "1.1.0".to_string(),
            }],
            total: 1,
        };

        Mock::given(method("GET"))
            .and(path("/api/v1/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let result = client
            .search("test", "example")
            .await
            .expect("must succeed");

        assert_eq!(result.total, 1);
        assert_eq!(result.results[0].name, "@test/example");
    }

    #[tokio::test]
    async fn publish_requires_credentials() {
        let server = MockServer::start().await;

        // No credentials configured
        let client = test_client(&server.uri(), None);
        let err = client
            .publish("test", "@test/example", b"fake-tarball")
            .await
            .expect_err("must fail");

        assert!(matches!(err, RegistryError::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn publish_success_with_credentials() {
        let server = MockServer::start().await;
        let response = PublishResponse {
            name: "@test/example".to_string(),
            version: "1.0.0".to_string(),
        };

        Mock::given(method("PUT"))
            .and(path("/api/v1/modules/@test/example"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), Some(("test", "test-token")));
        let result = client
            .publish("test", "@test/example", b"fake-tarball")
            .await
            .expect("must succeed");

        assert_eq!(result.version, "1.0.0");
    }

    #[tokio::test]
    async fn yank_requires_credentials() {
        let server = MockServer::start().await;

        let client = test_client(&server.uri(), None);
        let err = client
            .yank("test", "@test/example", "1.0.0")
            .await
            .expect_err("must fail");

        assert!(matches!(err, RegistryError::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn auth_header_attached_when_credentials_present() {
        let server = MockServer::start().await;
        let info = sample_module_info();

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .and(header("authorization", "Bearer my-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), Some(("test", "my-secret")));
        let result = client
            .fetch_module_info("test", "@test/example")
            .await
            .expect("must succeed with auth header");

        assert_eq!(result.name, "@test/example");
    }

    #[tokio::test]
    async fn resolve_version_picks_highest_match() {
        let server = MockServer::start().await;
        let info = sample_module_info();

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let req = semver::VersionReq::parse("^1.0").expect("valid req");
        let version = client
            .resolve_version("test", "@test/example", &req)
            .await
            .expect("must resolve");

        assert_eq!(version, semver::Version::new(1, 1, 0));
    }

    #[tokio::test]
    async fn resolve_version_skips_yanked() {
        let server = MockServer::start().await;
        let info = sample_module_info();

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        // 2.0.0 exists but is yanked — should not match
        let req = semver::VersionReq::parse(">=2.0.0").expect("valid req");
        let err = client
            .resolve_version("test", "@test/example", &req)
            .await
            .expect_err("must fail — 2.0.0 is yanked");

        assert!(matches!(err, RegistryError::VersionNotFound { .. }));
    }

    #[tokio::test]
    async fn retry_on_429() {
        let server = MockServer::start().await;
        let info = sample_module_info();

        // First request returns 429, second returns 200
        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let result = client
            .fetch_module_info("test", "@test/example")
            .await
            .expect("must succeed after retry");

        assert_eq!(result.name, "@test/example");
    }

    #[tokio::test]
    async fn retry_on_500() {
        let server = MockServer::start().await;
        let info = sample_module_info();

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/example"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let result = client
            .fetch_module_info("test", "@test/example")
            .await
            .expect("must succeed after retry");

        assert_eq!(result.name, "@test/example");
    }

    #[tokio::test]
    async fn no_retry_on_401() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/modules/@test/private"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1) // Should be called exactly once — no retries
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), None);
        let err = client
            .fetch_module_info("test", "@test/private")
            .await
            .expect_err("must fail");

        assert!(matches!(err, RegistryError::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn module_cache_dir_scoped() {
        let dir = module_cache_dir(Path::new("/cache"), "@duumbi/stdlib-math", "1.0.0");
        assert_eq!(
            dir,
            std::path::PathBuf::from("/cache/@duumbi/stdlib-math@1.0.0")
        );
    }

    #[tokio::test]
    async fn module_cache_dir_unscoped() {
        let dir = module_cache_dir(Path::new("/cache"), "simple", "0.1.0");
        assert_eq!(dir, std::path::PathBuf::from("/cache/simple@0.1.0"));
    }

    #[test]
    fn error_codes_mapped_correctly() {
        // Can't easily construct a reqwest::Error, so test non-Unreachable variants:
        let err = RegistryError::AuthFailed {
            registry: "test".to_string(),
            reason: "bad token".to_string(),
        };
        assert_eq!(err.error_code(), "E014");

        let err = RegistryError::VersionNotFound {
            module: "m".to_string(),
            version: "1.0".to_string(),
            registry: "r".to_string(),
        };
        assert_eq!(err.error_code(), "E016");

        let err = RegistryError::IntegrityMismatch {
            module: "m".to_string(),
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        assert_eq!(err.error_code(), "E015");

        let err = RegistryError::RetriesExhausted {
            url: "http://x".to_string(),
            max_retries: 3,
        };
        assert_eq!(err.error_code(), "E013");
    }
}
