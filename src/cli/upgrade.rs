//! Workspace upgrade command.
//!
//! Migrates Phase 4-5 workspaces to Phase 7 format:
//! stdlib relocation, config v2 with registries, lockfile v1 regeneration,
//! and `.gitignore` updates.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{self, WorkspaceSection};
use crate::deps;

/// Gitignore line that must be present after upgrade.
const CACHE_GITIGNORE_LINE: &str = ".duumbi/cache/";

/// Runs the workspace upgrade migration.
///
/// Migration steps:
/// 1. Detect old format (no `[registries]` section, stdlib in old path)
/// 2. Create backup of config.toml
/// 3. Move `.duumbi/stdlib/` → `.duumbi/cache/@duumbi/stdlib-*@1.0.0/`
/// 4. Update config.toml with `[registries]` and `[workspace]` sections
/// 5. Regenerate lockfile (v0 → v1)
/// 6. Ensure `.gitignore` has `.duumbi/cache/`
///
/// Non-destructive: creates backups before modifying. Idempotent.
pub fn run_upgrade(workspace: &Path) -> Result<()> {
    let duumbi_dir = workspace.join(".duumbi");
    if !duumbi_dir.exists() {
        anyhow::bail!("No .duumbi/ directory found — not a duumbi workspace");
    }

    let mut changes = Vec::new();

    // Step 1: Migrate stdlib from old path if present
    let old_stdlib = duumbi_dir.join("stdlib");
    if old_stdlib.exists() {
        migrate_stdlib(&duumbi_dir, &old_stdlib)?;
        changes.push("Migrated .duumbi/stdlib/ → .duumbi/cache/@duumbi/");
    }

    // Step 2: Update config.toml
    let config_path = duumbi_dir.join("config.toml");
    if config_path.exists() {
        let upgraded = upgrade_config(workspace)?;
        if upgraded {
            changes.push("Updated config.toml with [registries] and [workspace] sections");
        }
    }

    // Step 3: Regenerate lockfile
    let config = config::load_config(workspace).unwrap_or_default();
    if !config.dependencies.is_empty() {
        match deps::generate_lockfile(workspace, &config) {
            Ok(_) => changes.push("Regenerated deps.lock (v1 format)"),
            Err(e) => eprintln!("  Warning: could not regenerate lockfile: {e}"),
        }
    }

    // Step 4: Ensure .gitignore has cache exclusion
    let gitignore_path = workspace.join(".gitignore");
    if ensure_gitignore_cache(&gitignore_path)? {
        changes.push("Added .duumbi/cache/ to .gitignore");
    }

    if changes.is_empty() {
        eprintln!("Workspace is already up to date — no migration needed.");
    } else {
        for change in &changes {
            eprintln!("  ✓ {change}");
        }
        eprintln!("Upgrade complete ({} changes).", changes.len());
    }

    Ok(())
}

/// Migrates stdlib modules from `.duumbi/stdlib/<name>/` to cache layout.
///
/// Moves `math/` and `io/` directories into the scoped cache format.
fn migrate_stdlib(duumbi_dir: &Path, old_stdlib: &Path) -> Result<()> {
    let modules = [
        ("math", "@duumbi", "stdlib-math", "1.0.0"),
        ("io", "@duumbi", "stdlib-io", "1.0.0"),
    ];

    for (old_name, scope, cache_name, version) in &modules {
        let old_mod_dir = old_stdlib.join(old_name);
        if !old_mod_dir.exists() {
            continue;
        }

        let cache_entry = duumbi_dir
            .join("cache")
            .join(scope)
            .join(format!("{cache_name}@{version}"));

        // Skip if already migrated
        if cache_entry.exists() {
            continue;
        }

        let cache_graph = cache_entry.join("graph");
        fs::create_dir_all(&cache_graph)
            .with_context(|| format!("Failed to create cache dir for {scope}/{cache_name}"))?;

        // Copy graph files from old location
        let old_graph = old_mod_dir.join(".duumbi").join("graph");
        if old_graph.exists() {
            copy_dir_files(&old_graph, &cache_graph)?;
        }
    }

    // Remove old stdlib directory after successful migration
    fs::remove_dir_all(old_stdlib).context("Failed to remove old .duumbi/stdlib/ directory")?;

    Ok(())
}

/// Upgrades config.toml to M7 format.
///
/// Adds `[registries]` and `[workspace]` sections if missing.
/// Creates a backup at `config.toml.bak` before modifying.
/// Returns `true` if changes were made.
fn upgrade_config(workspace: &Path) -> Result<bool> {
    let mut config = config::load_config(workspace).unwrap_or_default();
    let mut changed = false;

    // Add registries if missing
    if config.registries.is_empty() {
        config.registries.insert(
            "duumbi".to_string(),
            "https://registry.duumbi.dev".to_string(),
        );
        changed = true;
    }

    // Add workspace section if missing
    if config.workspace.is_none() {
        config.workspace = Some(WorkspaceSection {
            name: String::new(),
            namespace: String::new(),
            default_registry: Some("duumbi".to_string()),
        });
        changed = true;
    } else if let Some(ref mut ws) = config.workspace
        && ws.default_registry.is_none()
    {
        ws.default_registry = Some("duumbi".to_string());
        changed = true;
    }

    if changed {
        // Backup before modifying
        let config_path = workspace.join(".duumbi").join("config.toml");
        let backup_path = workspace.join(".duumbi").join("config.toml.bak");
        if config_path.exists() {
            fs::copy(&config_path, &backup_path).context("Failed to backup config.toml")?;
        }
        config::save_config(workspace, &config).map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    Ok(changed)
}

/// Ensures `.gitignore` contains the cache exclusion line.
///
/// Returns `true` if the line was added.
fn ensure_gitignore_cache(gitignore_path: &Path) -> Result<bool> {
    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path).context("Failed to read .gitignore")?;
        if content.contains(CACHE_GITIGNORE_LINE) {
            return Ok(false);
        }
        // Append the line
        let mut new_content = content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(CACHE_GITIGNORE_LINE);
        new_content.push('\n');
        fs::write(gitignore_path, new_content).context("Failed to update .gitignore")?;
    } else {
        fs::write(
            gitignore_path,
            format!("# duumbi generated\n{CACHE_GITIGNORE_LINE}\n"),
        )
        .context("Failed to create .gitignore")?;
    }
    Ok(true)
}

/// Copies all files (non-recursive) from `src` to `dst`.
fn copy_dir_files(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src).with_context(|| format!("Failed to read {}", src.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), dst.join(entry.file_name()))
                .with_context(|| format!("Failed to copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_old_workspace(dir: &Path) {
        let d = dir.join(".duumbi");
        fs::create_dir_all(d.join("graph")).expect("create graph");
        fs::create_dir_all(d.join("schema")).expect("create schema");

        // Old config without registries
        fs::write(
            d.join("config.toml"),
            r#"[dependencies]
"@duumbi/stdlib-math" = "1.0.0"
"#,
        )
        .expect("write old config");

        // Old stdlib structure
        let stdlib_math = d.join("stdlib/math/.duumbi/graph");
        fs::create_dir_all(&stdlib_math).expect("create stdlib math");
        fs::write(
            stdlib_math.join("math.jsonld"),
            r#"{"@type": "duumbi:Module", "duumbi:name": "math"}"#,
        )
        .expect("write math");

        // Skeleton main
        fs::write(
            d.join("graph/main.jsonld"),
            r#"{"@type": "duumbi:Module", "duumbi:name": "main", "duumbi:functions": []}"#,
        )
        .expect("write main");
    }

    #[test]
    fn upgrade_migrates_stdlib_to_cache() {
        let tmp = TempDir::new().expect("tempdir");
        make_old_workspace(tmp.path());

        run_upgrade(tmp.path()).expect("upgrade must succeed");

        // Old stdlib should be removed
        assert!(
            !tmp.path().join(".duumbi/stdlib").exists(),
            "old stdlib must be removed"
        );

        // New cache location should exist
        assert!(
            tmp.path()
                .join(".duumbi/cache/@duumbi/stdlib-math@1.0.0/graph/math.jsonld")
                .exists(),
            "stdlib-math must be in cache"
        );
    }

    #[test]
    fn upgrade_adds_registries_to_config() {
        let tmp = TempDir::new().expect("tempdir");
        make_old_workspace(tmp.path());

        run_upgrade(tmp.path()).expect("upgrade must succeed");

        let config = config::load_config(tmp.path()).expect("config must parse");
        assert!(
            config.registries.contains_key("duumbi"),
            "must have duumbi registry"
        );
        let ws = config.workspace.expect("workspace section");
        assert_eq!(ws.default_registry.as_deref(), Some("duumbi"));
    }

    #[test]
    fn upgrade_creates_config_backup() {
        let tmp = TempDir::new().expect("tempdir");
        make_old_workspace(tmp.path());

        run_upgrade(tmp.path()).expect("upgrade must succeed");

        assert!(
            tmp.path().join(".duumbi/config.toml.bak").exists(),
            "backup must be created"
        );
    }

    #[test]
    fn upgrade_adds_cache_to_gitignore() {
        let tmp = TempDir::new().expect("tempdir");
        make_old_workspace(tmp.path());

        // Create a .gitignore without cache line
        fs::write(tmp.path().join(".gitignore"), "*.o\n").expect("write gitignore");

        run_upgrade(tmp.path()).expect("upgrade must succeed");

        let content = fs::read_to_string(tmp.path().join(".gitignore")).expect("read gitignore");
        assert!(content.contains(".duumbi/cache/"), "must add cache line");
        assert!(content.contains("*.o"), "must preserve existing content");
    }

    #[test]
    fn upgrade_is_idempotent() {
        let tmp = TempDir::new().expect("tempdir");
        make_old_workspace(tmp.path());

        run_upgrade(tmp.path()).expect("first upgrade");
        run_upgrade(tmp.path()).expect("second upgrade must also succeed");

        // Should still have valid config
        let config = config::load_config(tmp.path()).expect("config must parse");
        assert!(config.registries.contains_key("duumbi"));
    }

    #[test]
    fn upgrade_fails_without_duumbi_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let result = run_upgrade(tmp.path());
        assert!(result.is_err(), "must fail without .duumbi/");
    }
}
