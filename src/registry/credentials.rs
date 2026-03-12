//! Credential storage for registry authentication.
//!
//! Manages bearer tokens in `~/.duumbi/credentials.toml`. Each registry
//! has its own `[registries.<name>]` section with a `token` field.
//!
//! File permissions are set to `0o600` (owner-only) on Unix to prevent
//! accidental exposure of tokens.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::client::RegistryCredential;

/// Errors from credential storage operations.
#[derive(Debug, Error)]
pub enum CredentialError {
    /// Home directory could not be determined.
    #[error("Could not determine home directory")]
    NoHomeDir,

    /// I/O error reading or writing credentials file.
    #[error("Credential file I/O error at '{path}': {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// TOML parse error.
    #[error("Failed to parse credentials file: {0}")]
    Parse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("Failed to serialize credentials: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// On-disk format for `~/.duumbi/credentials.toml`.
///
/// ```toml
/// [registries.duumbi]
/// token = "duu_abc123..."
///
/// [registries.company]
/// token = "tok_xyz789..."
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CredentialsFile {
    /// Map of registry name to stored credential.
    #[serde(default)]
    pub registries: HashMap<String, StoredCredential>,
}

/// A single stored credential entry.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredCredential {
    /// Bearer token for this registry.
    pub token: String,
}

/// Returns the path to `~/.duumbi/credentials.toml`.
///
/// # Errors
/// Returns `CredentialError::NoHomeDir` if the home directory cannot be found.
#[must_use = "credential path errors should be handled"]
pub fn credentials_path() -> Result<PathBuf, CredentialError> {
    let home = dirs::home_dir().ok_or(CredentialError::NoHomeDir)?;
    Ok(home.join(".duumbi").join("credentials.toml"))
}

/// Loads credentials from `~/.duumbi/credentials.toml`.
///
/// Returns an empty [`CredentialsFile`] if the file does not exist.
#[must_use = "credential errors should be handled"]
pub fn load_credentials() -> Result<CredentialsFile, CredentialError> {
    let path = credentials_path()?;
    load_credentials_from(&path)
}

/// Loads credentials from a specific path (useful for testing).
#[must_use = "credential errors should be handled"]
pub fn load_credentials_from(path: &Path) -> Result<CredentialsFile, CredentialError> {
    if !path.exists() {
        return Ok(CredentialsFile::default());
    }

    let contents = fs::read_to_string(path).map_err(|source| CredentialError::Io {
        path: path.display().to_string(),
        source,
    })?;

    let creds: CredentialsFile = toml::from_str(&contents)?;
    Ok(creds)
}

/// Saves credentials to `~/.duumbi/credentials.toml`.
///
/// Creates the `~/.duumbi/` directory if needed. Sets file permissions to
/// `0o600` on Unix platforms.
#[must_use = "credential save errors should be handled"]
pub fn save_credentials(creds: &CredentialsFile) -> Result<(), CredentialError> {
    let path = credentials_path()?;
    save_credentials_to(&path, creds)
}

/// Saves credentials to a specific path (useful for testing).
#[must_use = "credential save errors should be handled"]
pub fn save_credentials_to(path: &Path, creds: &CredentialsFile) -> Result<(), CredentialError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| CredentialError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let contents = toml::to_string_pretty(creds)?;
    fs::write(path, &contents).map_err(|source| CredentialError::Io {
        path: path.display().to_string(),
        source,
    })?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms).map_err(|source| CredentialError::Io {
            path: path.display().to_string(),
            source,
        })?;
    }

    Ok(())
}

/// Looks up the token for a specific registry.
///
/// Returns `None` if the registry has no stored credential.
#[must_use]
pub fn get_token(creds: &CredentialsFile, registry: &str) -> Option<String> {
    creds.registries.get(registry).map(|c| c.token.clone())
}

/// Sets (or updates) the token for a registry.
pub fn set_token(creds: &mut CredentialsFile, registry: &str, token: &str) {
    creds.registries.insert(
        registry.to_string(),
        StoredCredential {
            token: token.to_string(),
        },
    );
}

/// Removes the token for a registry.
///
/// Returns `true` if the registry had a stored credential that was removed.
pub fn remove_token(creds: &mut CredentialsFile, registry: &str) -> bool {
    creds.registries.remove(registry).is_some()
}

/// Converts a [`CredentialsFile`] into the `HashMap<String, RegistryCredential>`
/// format expected by [`super::client::RegistryClient`].
#[must_use]
pub fn to_client_credentials(creds: &CredentialsFile) -> HashMap<String, RegistryCredential> {
    creds
        .registries
        .iter()
        .map(|(name, stored)| {
            (
                name.clone(),
                RegistryCredential {
                    token: stored.token.clone(),
                },
            )
        })
        .collect()
}

/// Checks file permissions and warns if the credentials file is too open.
///
/// Returns `Some(message)` if permissions are not restrictive enough.
#[cfg(unix)]
#[must_use]
pub fn check_permissions(path: &Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt as _;
    if let Ok(meta) = fs::metadata(path) {
        let mode = meta.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Some(format!(
                "Warning: credentials file has permissions {mode:04o}, expected 0600.\n\
                 Run: chmod 600 {}",
                path.display()
            ));
        }
    }
    None
}

/// Checks file permissions (no-op on non-Unix platforms).
#[cfg(not(unix))]
#[must_use]
pub fn check_permissions(_path: &Path) -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_nonexistent_returns_empty() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("nonexistent.toml");
        let creds = load_credentials_from(&path).expect("must succeed for missing file");
        assert!(creds.registries.is_empty());
    }

    #[test]
    fn roundtrip_save_load() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");

        let mut creds = CredentialsFile::default();
        set_token(&mut creds, "duumbi", "duu_abc123");
        set_token(&mut creds, "company", "tok_xyz789");

        save_credentials_to(&path, &creds).expect("save must succeed");
        let loaded = load_credentials_from(&path).expect("load must succeed");

        assert_eq!(get_token(&loaded, "duumbi"), Some("duu_abc123".to_string()));
        assert_eq!(
            get_token(&loaded, "company"),
            Some("tok_xyz789".to_string())
        );
        assert_eq!(get_token(&loaded, "missing"), None);
    }

    #[test]
    fn remove_token_works() {
        let mut creds = CredentialsFile::default();
        set_token(&mut creds, "test", "tok_123");
        assert!(get_token(&creds, "test").is_some());

        let removed = remove_token(&mut creds, "test");
        assert!(removed);
        assert!(get_token(&creds, "test").is_none());

        let removed_again = remove_token(&mut creds, "test");
        assert!(!removed_again);
    }

    #[test]
    fn to_client_credentials_converts() {
        let mut creds = CredentialsFile::default();
        set_token(&mut creds, "reg1", "token1");
        set_token(&mut creds, "reg2", "token2");

        let client_creds = to_client_credentials(&creds);
        assert_eq!(client_creds.len(), 2);
        assert_eq!(client_creds["reg1"].token, "token1");
        assert_eq!(client_creds["reg2"].token, "token2");
    }

    #[cfg(unix)]
    #[test]
    fn permissions_set_to_0600() {
        use std::os::unix::fs::PermissionsExt as _;

        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");

        let creds = CredentialsFile::default();
        save_credentials_to(&path, &creds).expect("save must succeed");

        let meta = fs::metadata(&path).expect("must read metadata");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "credentials file must have 0600 permissions");
    }

    #[cfg(unix)]
    #[test]
    fn check_permissions_warns_on_open_file() {
        use std::os::unix::fs::PermissionsExt as _;

        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("creds.toml");
        fs::write(&path, "").expect("write");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod");

        let warning = check_permissions(&path);
        assert!(warning.is_some());
        assert!(warning.expect("must warn").contains("0644"));
    }

    #[test]
    fn token_with_special_characters_roundtrips() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");

        let mut creds = CredentialsFile::default();
        // Quotes, newline-escaped, unicode, equals signs
        set_token(&mut creds, "special", "tok=\"hello\"\nworld🔑=+/base64");
        save_credentials_to(&path, &creds).expect("save must succeed");

        let loaded = load_credentials_from(&path).expect("load must succeed");
        assert_eq!(
            get_token(&loaded, "special"),
            Some("tok=\"hello\"\nworld🔑=+/base64".to_string())
        );
    }

    #[test]
    fn empty_token_roundtrips() {
        let mut creds = CredentialsFile::default();
        set_token(&mut creds, "empty", "");
        assert_eq!(get_token(&creds, "empty"), Some(String::new()));
    }

    #[test]
    fn set_token_overwrites_existing() {
        let mut creds = CredentialsFile::default();
        set_token(&mut creds, "reg", "old_token");
        set_token(&mut creds, "reg", "new_token");
        assert_eq!(get_token(&creds, "reg"), Some("new_token".to_string()));
        assert_eq!(creds.registries.len(), 1);
    }

    #[test]
    fn empty_file_returns_empty_credentials() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");
        fs::write(&path, "").expect("write empty file");

        let creds = load_credentials_from(&path).expect("must parse empty file");
        assert!(creds.registries.is_empty());
    }

    #[test]
    fn corrupted_toml_returns_parse_error() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");
        fs::write(&path, "[registries.broken\ntoken = ").expect("write bad toml");

        let err = load_credentials_from(&path).expect_err("must fail on bad TOML");
        assert!(matches!(err, CredentialError::Parse(_)));
    }

    #[cfg(unix)]
    #[test]
    fn check_permissions_returns_none_for_nonexistent_file() {
        let result = check_permissions(std::path::Path::new("/nonexistent/credentials.toml"));
        assert!(result.is_none(), "nonexistent file should not warn");
    }

    #[cfg(unix)]
    #[test]
    fn check_permissions_returns_none_for_correct_perms() {
        use std::os::unix::fs::PermissionsExt as _;

        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("creds.toml");
        fs::write(&path, "").expect("write");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");

        let result = check_permissions(&path);
        assert!(result.is_none(), "0600 file should not warn");
    }

    #[test]
    fn to_client_credentials_empty_input() {
        let creds = CredentialsFile::default();
        let client_creds = to_client_credentials(&creds);
        assert!(client_creds.is_empty());
    }

    #[test]
    fn remove_nonexistent_registry_returns_false() {
        let mut creds = CredentialsFile::default();
        assert!(!remove_token(&mut creds, "nonexistent"));
    }

    #[test]
    fn credential_error_display_messages() {
        let err = CredentialError::NoHomeDir;
        assert!(err.to_string().contains("home directory"));

        let err = CredentialError::Io {
            path: "/test/path".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        assert!(err.to_string().contains("/test/path"));
    }

    #[test]
    fn many_registries_roundtrip() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");

        let mut creds = CredentialsFile::default();
        for i in 0..10 {
            set_token(&mut creds, &format!("reg_{i}"), &format!("tok_{i}"));
        }
        save_credentials_to(&path, &creds).expect("save must succeed");
        let loaded = load_credentials_from(&path).expect("load must succeed");

        assert_eq!(loaded.registries.len(), 10);
        for i in 0..10 {
            assert_eq!(
                get_token(&loaded, &format!("reg_{i}")),
                Some(format!("tok_{i}"))
            );
        }
    }

    #[test]
    fn credentials_path_returns_valid_path() {
        // This test verifies credentials_path() doesn't panic and returns a
        // reasonable path (may fail in CI if HOME is not set, which is fine).
        if let Ok(path) = credentials_path() {
            assert!(
                path.ends_with(".duumbi/credentials.toml"),
                "path must end with .duumbi/credentials.toml, got: {}",
                path.display()
            );
        }
    }

    #[test]
    fn parse_existing_credentials_toml() {
        let tmp = TempDir::new().expect("invariant: tempdir creation");
        let path = tmp.path().join("credentials.toml");

        fs::write(
            &path,
            r#"
[registries.duumbi]
token = "duu_test_token"

[registries.private]
token = "priv_abc"
"#,
        )
        .expect("write");

        let creds = load_credentials_from(&path).expect("must parse");
        assert_eq!(
            get_token(&creds, "duumbi"),
            Some("duu_test_token".to_string())
        );
        assert_eq!(get_token(&creds, "private"), Some("priv_abc".to_string()));
    }
}
