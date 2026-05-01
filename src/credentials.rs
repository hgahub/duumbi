//! User-level credential file helpers.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn credentials_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".duumbi").join("credentials.toml"))
}

fn load_all() -> HashMap<String, String> {
    let Some(path) = credentials_path() else {
        return HashMap::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    toml::from_str::<HashMap<String, String>>(&content).unwrap_or_default()
}

/// Loads a credential from `~/.duumbi/credentials.toml`.
#[must_use]
pub fn load_api_key(env_var_name: &str) -> Option<String> {
    load_all().remove(env_var_name)
}
