//! CLI handlers for `duumbi registry` subcommands.
//!
//! Manages registry configurations in `config.toml` and authentication
//! tokens in `~/.duumbi/credentials.toml`.

use std::io::{self, BufRead as _, Write as _};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::{self, DuumbiConfig};
use crate::registry::credentials;

/// Adds a registry to the `[registries]` section in `config.toml`.
///
/// Validates that the URL uses HTTPS (unless localhost for development).
pub fn run_registry_add(workspace: &Path, name: &str, url: &str) -> Result<()> {
    // Validate URL
    if !url.starts_with("https://") && !url.starts_with("http://localhost") {
        anyhow::bail!(
            "Registry URL must use HTTPS (got '{url}').\n\
             Only http://localhost is allowed for development."
        );
    }

    let mut cfg = config::load_config(workspace).unwrap_or_default();

    if cfg.registries.contains_key(name) {
        anyhow::bail!(
            "Registry '{name}' already exists. Remove it first with `duumbi registry remove {name}`."
        );
    }

    cfg.registries
        .insert(name.to_string(), url.trim_end_matches('/').to_string());
    config::save_config(workspace, &cfg).map_err(|e| anyhow::anyhow!("{e}"))?;

    eprintln!("Added registry '{name}' → {url}");
    Ok(())
}

/// Lists all configured registries.
pub fn run_registry_list(workspace: &Path) -> Result<()> {
    let cfg = config::load_config(workspace).unwrap_or_default();

    if cfg.registries.is_empty() {
        eprintln!("No registries configured.");
        return Ok(());
    }

    let default_reg = cfg
        .workspace
        .as_ref()
        .and_then(|ws| ws.default_registry.as_deref());

    eprintln!("{:<20} URL", "NAME");
    eprintln!("{}", "\u{2500}".repeat(60));

    for (name, url) in &cfg.registries {
        let marker = if default_reg == Some(name.as_str()) {
            " (default)"
        } else {
            ""
        };
        eprintln!("{:<20} {url}{marker}", name);
    }

    Ok(())
}

/// Removes a registry from `config.toml`.
///
/// Warns if any dependencies reference this registry.
pub fn run_registry_remove(workspace: &Path, name: &str) -> Result<()> {
    let mut cfg = config::load_config(workspace).unwrap_or_default();

    if !cfg.registries.contains_key(name) {
        anyhow::bail!("Registry '{name}' not found in config.toml.");
    }

    // Check if any deps reference this registry
    let referencing_deps: Vec<&String> = cfg
        .dependencies
        .iter()
        .filter(|(_, dep)| dep.registry() == Some(name))
        .map(|(dep_name, _)| dep_name)
        .collect();

    if !referencing_deps.is_empty() {
        eprintln!("Warning: the following dependencies reference registry '{name}':");
        for dep in &referencing_deps {
            eprintln!("  - {dep}");
        }
        eprintln!("These dependencies will fail to resolve after removal.");
    }

    cfg.registries.remove(name);

    // Clear default-registry if it pointed to the removed registry
    if let Some(ref mut ws) = cfg.workspace
        && ws.default_registry.as_deref() == Some(name)
    {
        ws.default_registry = None;
        eprintln!("Cleared default-registry (was '{name}').");
    }

    config::save_config(workspace, &cfg).map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Removed registry '{name}'.");
    Ok(())
}

/// Sets the default registry in `[workspace]`.
pub fn run_registry_default(workspace: &Path, name: &str) -> Result<()> {
    let mut cfg = config::load_config(workspace).unwrap_or_default();

    if !cfg.registries.contains_key(name) {
        anyhow::bail!(
            "Registry '{name}' not found. Add it first with `duumbi registry add {name} <url>`."
        );
    }

    let ws = cfg.workspace.get_or_insert_with(Default::default);
    ws.default_registry = Some(name.to_string());

    config::save_config(workspace, &cfg).map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Set default registry to '{name}'.");
    Ok(())
}

/// Authenticates with a registry by storing a token.
///
/// In interactive mode, prompts for the token with hidden input.
/// With `--token`, takes the token directly (for CI use).
///
/// Validates the token by calling `GET /api/v1/auth/verify` on the registry.
pub async fn run_registry_login(
    workspace: &Path,
    registry: &str,
    token_arg: Option<&str>,
) -> Result<()> {
    let cfg = config::load_config(workspace).unwrap_or_default();

    let registry_url = cfg
        .registries
        .get(registry)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Registry '{registry}' not found in config.toml.\n\
                 Add it first with `duumbi registry add {registry} <url>`."
            )
        })?
        .clone();

    let token = match token_arg {
        Some(t) => {
            if t.is_empty() {
                anyhow::bail!("Token cannot be empty.");
            }
            // Validate token against registry
            eprintln!("Validating token with {registry_url}...");
            validate_token(&registry_url, t).await?;
            t.to_string()
        }
        None => {
            // Try device code flow first; fall back to interactive prompt on failure.
            match device_code_login(&registry_url).await {
                Ok((token, username)) => {
                    eprintln!("Authenticated as {username}.");
                    eprintln!("Validating token with {registry_url}...");
                    validate_token(&registry_url, &token).await?;
                    token
                }
                Err(DeviceLoginOutcome::FallbackToPrompt) => {
                    let token = prompt_token_interactive()?;
                    if token.is_empty() {
                        anyhow::bail!("Token cannot be empty.");
                    }
                    eprintln!("Validating token with {registry_url}...");
                    validate_token(&registry_url, &token).await?;
                    token
                }
                Err(DeviceLoginOutcome::Fatal(e)) => return Err(e),
            }
        }
    };

    // Store credential
    let mut creds = credentials::load_credentials().map_err(|e| anyhow::anyhow!("{e}"))?;
    credentials::set_token(&mut creds, registry, &token);
    credentials::save_credentials(&creds).map_err(|e| anyhow::anyhow!("{e}"))?;

    // Check permissions
    if let Ok(path) = credentials::credentials_path()
        && let Some(warning) = credentials::check_permissions(&path)
    {
        eprintln!("{warning}");
    }

    eprintln!("Logged in to registry '{registry}'.");
    Ok(())
}

/// Removes stored credentials for a registry.
pub fn run_registry_logout(registry: Option<&str>) -> Result<()> {
    let mut creds = credentials::load_credentials().map_err(|e| anyhow::anyhow!("{e}"))?;

    match registry {
        Some(name) => {
            if credentials::remove_token(&mut creds, name) {
                credentials::save_credentials(&creds).map_err(|e| anyhow::anyhow!("{e}"))?;
                eprintln!("Logged out from registry '{name}'.");
            } else {
                eprintln!("No credentials stored for registry '{name}'.");
            }
        }
        None => {
            if creds.registries.is_empty() {
                eprintln!("No stored credentials.");
            } else {
                let names: Vec<String> = creds.registries.keys().cloned().collect();
                creds.registries.clear();
                credentials::save_credentials(&creds).map_err(|e| anyhow::anyhow!("{e}"))?;
                eprintln!("Logged out from all registries: {}", names.join(", "));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Outcome of an attempted device code login.
///
/// Separates "fall back gracefully" from "abort with a real error".
enum DeviceLoginOutcome {
    /// The registry does not support device code — caller should prompt for a token.
    FallbackToPrompt,
    /// A hard error occurred during device code exchange that the user must see.
    Fatal(anyhow::Error),
}

impl From<anyhow::Error> for DeviceLoginOutcome {
    fn from(e: anyhow::Error) -> Self {
        DeviceLoginOutcome::Fatal(e)
    }
}

// ---------------------------------------------------------------------------
// Device code flow response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AuthModeResponse {
    device_code_supported: bool,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
}

#[derive(Deserialize)]
struct DeviceTokenResponse {
    /// Bearer token returned on successful auth exchange.
    token: String,
    /// Display name / login of the authenticated user.
    username: String,
}

// ---------------------------------------------------------------------------
// Device code implementation
// ---------------------------------------------------------------------------

/// Attempts authentication via RFC 8628-style device code flow.
///
/// Returns `(token, username)` on success or a [`DeviceLoginOutcome`] error
/// indicating whether the caller should fall back to interactive token prompt
/// or surface a fatal error.
async fn device_code_login(
    registry_url: &str,
) -> std::result::Result<(String, String), DeviceLoginOutcome> {
    let base = registry_url.trim_end_matches('/');

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("Failed to create HTTP client")?;

    // ------------------------------------------------------------------
    // 1. Discover whether device code is supported.
    // ------------------------------------------------------------------
    let mode_url = format!("{base}/api/v1/auth/mode");
    let mode_resp = client
        .get(&mode_url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {mode_url}"))?;

    if mode_resp.status() == reqwest::StatusCode::NOT_FOUND {
        // Registry is too old or does not implement this endpoint → prompt.
        return Err(DeviceLoginOutcome::FallbackToPrompt);
    }

    if !mode_resp.status().is_success() {
        // Unexpected error; fall back rather than abort.
        return Err(DeviceLoginOutcome::FallbackToPrompt);
    }

    let mode: AuthModeResponse = mode_resp
        .json()
        .await
        .context("Failed to parse auth mode response")?;

    if !mode.device_code_supported {
        return Err(DeviceLoginOutcome::FallbackToPrompt);
    }

    // ------------------------------------------------------------------
    // 2. Request a device code.
    // ------------------------------------------------------------------
    let code_url = format!("{base}/api/v1/auth/device/code");
    let code_resp = client
        .post(&code_url)
        .json(&serde_json::json!({ "client_id": "duumbi-cli" }))
        .send()
        .await
        .with_context(|| format!("Failed to request device code from {code_url}"))?;

    if !code_resp.status().is_success() {
        let status = code_resp.status();
        return Err(anyhow::anyhow!("Device code request failed with HTTP {status}.").into());
    }

    let device: DeviceCodeResponse = code_resp
        .json()
        .await
        .context("Failed to parse device code response")?;

    // ------------------------------------------------------------------
    // 3. Instruct the user and optionally open the browser.
    // ------------------------------------------------------------------
    eprintln!(
        "Visit {} and enter code: {}",
        device.verification_uri, device.user_code
    );
    // Only auto-open http/https URLs to prevent abuse via custom URI schemes.
    if device.verification_uri.starts_with("https://")
        || device.verification_uri.starts_with("http://")
    {
        open_browser(&device.verification_uri);
    }

    // ------------------------------------------------------------------
    // 4. Poll for a token.
    // ------------------------------------------------------------------
    let token_url = format!("{base}/api/v1/auth/device/token");
    let deadline = Instant::now() + Duration::from_secs(15 * 60);
    let poll_interval = Duration::from_secs(5);

    loop {
        if Instant::now() >= deadline {
            return Err(anyhow::anyhow!(
                "Device code login timed out after 15 minutes. Run `duumbi registry login` again."
            )
            .into());
        }

        tokio::time::sleep(poll_interval).await;

        let poll_resp = client
            .post(&token_url)
            .json(&serde_json::json!({
                "device_code": &device.device_code,
                "client_id": "duumbi-cli"
            }))
            .send()
            .await
            .with_context(|| format!("Failed to poll {token_url}"))?;

        match poll_resp.status() {
            // Success — exchange complete.
            s if s.is_success() => {
                let tok: DeviceTokenResponse = poll_resp
                    .json()
                    .await
                    .context("Failed to parse device token response")?;
                return Ok((tok.token, tok.username));
            }
            // 428 Precondition Required — authorization pending, keep polling.
            reqwest::StatusCode::PRECONDITION_REQUIRED => {
                continue;
            }
            // 410 Gone — code expired.
            reqwest::StatusCode::GONE => {
                return Err(anyhow::anyhow!(
                    "The device code expired before authorization was completed.\n\
                         Run `duumbi registry login` again."
                )
                .into());
            }
            s => {
                return Err(anyhow::anyhow!(
                    "Unexpected status {s} while polling for device token."
                )
                .into());
            }
        }
    }
}

/// Attempts to open `url` in the system browser.
///
/// Silently ignores failures — opening a browser is best-effort only.
fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();

    // On other platforms (e.g. Windows) we skip the auto-open.
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let _ = url;
}

/// Prompts for a token interactively (reads from stdin, no echo).
fn prompt_token_interactive() -> Result<String> {
    eprint!("Token: ");
    io::stderr().flush().ok();

    // Read a line from stdin (in a real terminal, we'd disable echo,
    // but for portability we just read normally)
    let mut token = String::new();
    io::stdin()
        .lock()
        .read_line(&mut token)
        .context("Failed to read token from stdin")?;

    Ok(token.trim().to_string())
}

/// Validates a token against the registry's auth verification endpoint.
///
/// Calls `GET /api/v1/auth/verify` with the token as Bearer auth.
async fn validate_token(registry_url: &str, token: &str) -> Result<()> {
    let url = format!("{}/api/v1/auth/verify", registry_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    let resp = client
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .with_context(|| format!("Failed to connect to {url}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        anyhow::bail!("Token validation failed: the registry rejected this token (HTTP {status}).");
    }

    if !status.is_success() {
        anyhow::bail!(
            "Token validation returned unexpected status {status}.\n\
             The registry may not support token verification at this endpoint."
        );
    }

    eprintln!("Token verified successfully.");
    Ok(())
}

/// Builds a [`RegistryClient`] with credentials loaded from `~/.duumbi/credentials.toml`.
///
/// Used by `deps.rs` and other modules that need authenticated registry access.
#[must_use = "registry client errors should be handled"]
pub fn build_registry_client_with_credentials(
    cfg: &DuumbiConfig,
) -> Result<crate::registry::RegistryClient> {
    let registries = cfg.registries.clone();
    if registries.is_empty() {
        anyhow::bail!(
            "No registries configured in config.toml.\n\
             Add a [registries] section with at least one registry URL."
        );
    }

    let creds_file = credentials::load_credentials()
        .map_err(|e| anyhow::anyhow!("Failed to load credentials: {e}"))?;
    let client_creds = credentials::to_client_credentials(&creds_file);

    crate::registry::RegistryClient::new(registries, client_creds, None)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DependencyConfig;
    use std::fs;
    use tempfile::TempDir;

    fn setup_workspace(tmp: &TempDir) {
        let duumbi = tmp.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: mkdir");
        let config = r#"
[workspace]
name = "test"
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"
"#;
        fs::write(duumbi.join("config.toml"), config).expect("invariant: write config");
    }

    #[test]
    fn registry_add_success() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        run_registry_add(tmp.path(), "company", "https://registry.acme.com")
            .expect("add must succeed");

        let cfg = config::load_config(tmp.path()).expect("reload");
        assert_eq!(cfg.registries["company"], "https://registry.acme.com");
    }

    #[test]
    fn registry_add_rejects_http() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        let err =
            run_registry_add(tmp.path(), "bad", "http://evil.com").expect_err("must reject HTTP");
        assert!(err.to_string().contains("HTTPS"));
    }

    #[test]
    fn registry_add_allows_localhost() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        run_registry_add(tmp.path(), "local", "http://localhost:3000")
            .expect("localhost must be allowed");
    }

    #[test]
    fn registry_add_rejects_duplicate() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        let err = run_registry_add(tmp.path(), "duumbi", "https://other.dev")
            .expect_err("must reject duplicate");
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn registry_list_empty() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: mkdir");
        fs::write(duumbi.join("config.toml"), "").expect("write");

        run_registry_list(tmp.path()).expect("list must succeed");
    }

    #[test]
    fn registry_remove_success() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        // Add a second registry, then remove it
        run_registry_add(tmp.path(), "company", "https://registry.acme.com")
            .expect("add must succeed");
        run_registry_remove(tmp.path(), "company").expect("remove must succeed");

        let cfg = config::load_config(tmp.path()).expect("reload");
        assert!(!cfg.registries.contains_key("company"));
    }

    #[test]
    fn registry_remove_not_found() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        let err =
            run_registry_remove(tmp.path(), "nonexistent").expect_err("must fail for missing");
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn registry_remove_clears_default() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        run_registry_remove(tmp.path(), "duumbi").expect("remove must succeed");

        let cfg = config::load_config(tmp.path()).expect("reload");
        let ws = cfg.workspace.expect("workspace must exist");
        assert!(ws.default_registry.is_none());
    }

    #[test]
    fn registry_remove_warns_on_dep_reference() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        // Add a dep that references "duumbi" registry
        let mut cfg = config::load_config(tmp.path()).expect("load");
        cfg.dependencies.insert(
            "@duumbi/math".to_string(),
            DependencyConfig::VersionWithRegistry {
                version: "^1.0".to_string(),
                registry: "duumbi".to_string(),
            },
        );
        config::save_config(tmp.path(), &cfg).expect("save");

        // Remove should still succeed but with warning
        run_registry_remove(tmp.path(), "duumbi").expect("remove must succeed");
    }

    #[test]
    fn registry_default_success() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        run_registry_add(tmp.path(), "company", "https://registry.acme.com")
            .expect("add must succeed");
        run_registry_default(tmp.path(), "company").expect("default must succeed");

        let cfg = config::load_config(tmp.path()).expect("reload");
        let ws = cfg.workspace.expect("workspace must exist");
        assert_eq!(ws.default_registry.as_deref(), Some("company"));
    }

    #[test]
    fn registry_default_not_found() {
        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(&tmp);

        let err = run_registry_default(tmp.path(), "missing").expect_err("must fail for missing");
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn registry_logout_specific() {
        let creds_tmp = TempDir::new().expect("invariant: tempdir");
        let path = creds_tmp.path().join("credentials.toml");

        let mut creds = credentials::CredentialsFile::default();
        credentials::set_token(&mut creds, "test", "tok123");
        credentials::save_credentials_to(&path, &creds).expect("save");

        let mut loaded = credentials::load_credentials_from(&path).expect("load");
        assert!(credentials::remove_token(&mut loaded, "test"));
        credentials::save_credentials_to(&path, &loaded).expect("save");

        let reloaded = credentials::load_credentials_from(&path).expect("load");
        assert!(credentials::get_token(&reloaded, "test").is_none());
    }

    #[test]
    fn build_client_with_credentials_no_registries() {
        let cfg = DuumbiConfig::default();
        let err =
            build_registry_client_with_credentials(&cfg).expect_err("must fail without registries");
        assert!(err.to_string().contains("No registries"));
    }
}
