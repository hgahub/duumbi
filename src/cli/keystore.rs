//! Secure API key storage using `~/.duumbi/credentials.toml` (0600 perms).
//!
//! Avoids OS keychain dialogs — many CLI tools (AWS CLI, GitHub CLI) use this
//! approach. Keys are stored in plaintext but the file is owner-readable only.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn credentials_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".duumbi").join("credentials.toml")
}

fn load_all() -> HashMap<String, String> {
    let path = credentials_path();
    let Ok(content) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    toml::from_str::<HashMap<String, String>>(&content).unwrap_or_default()
}

fn save_all(map: &HashMap<String, String>) -> Result<(), String> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create ~/.duumbi: {e}"))?;
    }
    let content =
        toml::to_string(map).map_err(|e| format!("failed to serialize credentials: {e}"))?;
    fs::write(&path, &content).map_err(|e| format!("failed to write credentials file: {e}"))?;

    // Set 0600 permissions on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)
            .map_err(|e| format!("failed to set file permissions: {e}"))?;
    }

    Ok(())
}

/// Stores an API key in `~/.duumbi/credentials.toml`.
pub fn store_api_key(env_var_name: &str, key: &str) -> Result<(), String> {
    let mut map = load_all();
    map.insert(env_var_name.to_string(), key.to_string());
    save_all(&map)
}

/// Loads an API key from `~/.duumbi/credentials.toml`.
pub fn load_api_key(env_var_name: &str) -> Option<String> {
    load_all().remove(env_var_name)
}

/// Deletes an API key from `~/.duumbi/credentials.toml`.
#[allow(dead_code)]
pub fn delete_api_key(env_var_name: &str) -> Result<(), String> {
    let mut map = load_all();
    map.remove(env_var_name);
    save_all(&map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_path_ends_with_credentials_toml() {
        let path = credentials_path();
        assert_eq!(path.file_name().unwrap(), "credentials.toml");
    }
}
