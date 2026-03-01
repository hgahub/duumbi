//! CLI handlers for the `duumbi deps` subcommand.
//!
//! Manages local path dependencies declared in `.duumbi/config.toml`.

use std::path::Path;

use anyhow::{Context, Result};

use crate::deps;

/// Lists all declared dependencies and their resolution status.
pub fn run_deps_list(workspace: &Path) -> Result<()> {
    let entries = deps::list_dependencies(workspace).context("Failed to list dependencies")?;

    if entries.is_empty() {
        eprintln!("No dependencies declared.");
        return Ok(());
    }

    for (name, dep_path, resolution) in &entries {
        match resolution {
            Ok(resolved) => eprintln!("  {name}: {dep_path} → {}", resolved.display()),
            Err(e) => eprintln!("  {name}: {dep_path} [ERROR: {e}]"),
        }
    }

    Ok(())
}

/// Adds a local path dependency to `config.toml`.
pub fn run_deps_add(workspace: &Path, name: &str, dep_path: &str) -> Result<()> {
    deps::add_dependency(workspace, name, dep_path)
        .with_context(|| format!("Failed to add dependency '{name}' at '{dep_path}'"))?;

    eprintln!("Added dependency '{name}' → {dep_path}");
    Ok(())
}

/// Removes a local path dependency from `config.toml`.
pub fn run_deps_remove(workspace: &Path, name: &str) -> Result<()> {
    let removed = deps::remove_dependency(workspace, name)
        .with_context(|| format!("Failed to remove dependency '{name}'"))?;

    if removed {
        eprintln!("Removed dependency '{name}'.");
    } else {
        eprintln!("Dependency '{name}' not found.");
    }

    Ok(())
}
