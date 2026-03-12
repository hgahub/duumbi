//! CLI handlers for the `duumbi deps` subcommand.
//!
//! Manages dependencies declared in `.duumbi/config.toml` — both local path
//! dependencies and registry-resolved scoped modules.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{self, DependencyConfig};
use crate::deps;
use crate::registry::client::{RegistryClient, RegistryCredential};

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

/// Adds a dependency to `config.toml`.
///
/// Dispatches between local path deps and registry deps based on arguments:
/// - If `path` is provided → local path dependency (existing behavior)
/// - Otherwise → registry dependency, resolves version and downloads to cache
pub async fn run_deps_add(
    workspace: &Path,
    name: &str,
    path: Option<&str>,
    registry: Option<&str>,
) -> Result<()> {
    if let Some(dep_path) = path {
        // Local path dependency
        deps::add_dependency(workspace, name, dep_path)
            .with_context(|| format!("Failed to add dependency '{name}' at '{dep_path}'"))?;
        eprintln!("Added dependency '{name}' → {dep_path}");
        return Ok(());
    }

    // Registry dependency — parse name@version specifier
    let (module_name, version_spec) = parse_registry_specifier(name);

    let cfg = config::load_config(workspace).unwrap_or_default();

    // Determine which registry to use
    let registry_name = registry
        .map(|s| s.to_string())
        .or_else(|| {
            cfg.workspace
                .as_ref()
                .and_then(|ws| ws.default_registry.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No registry specified and no default-registry in config.\n\
                 Use --registry or set default-registry in [workspace]."
            )
        })?;

    // Build registry client
    let client = build_registry_client(&cfg, workspace)?;

    // Resolve version
    let version_req = match &version_spec {
        Some(spec) => semver::VersionReq::parse(spec)
            .with_context(|| format!("Invalid version specifier: '{spec}'"))?,
        None => semver::VersionReq::STAR,
    };

    eprintln!("Resolving {module_name} from registry '{registry_name}'…");

    let resolved = client
        .resolve_version(&registry_name, module_name, &version_req)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let version_str = resolved.to_string();
    eprintln!("  Found version {version_str}");

    // Download to cache
    let cache_dir = workspace.join(".duumbi").join("cache");
    eprintln!("Downloading {module_name}@{version_str}…");

    let manifest = client
        .download_module(&registry_name, module_name, &version_str, &cache_dir)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    eprintln!(
        "  Cached {} (exports: {})",
        manifest.module.name,
        manifest.exports.functions.join(", ")
    );

    // Update config.toml
    let mut config = config::load_config(workspace).unwrap_or_default();
    let dep_config = if registry.is_some() {
        DependencyConfig::VersionWithRegistry {
            version: format!("^{version_str}"),
            registry: registry_name.clone(),
        }
    } else {
        DependencyConfig::Version(format!("^{version_str}"))
    };

    config
        .dependencies
        .insert(module_name.to_string(), dep_config);
    config::save_config(workspace, &config).map_err(|e| anyhow::anyhow!("{e}"))?;

    eprintln!("Added '{module_name}' = \"^{version_str}\" to config.toml");
    Ok(())
}

/// Updates dependencies to their latest compatible versions from registries.
///
/// If `name` is provided, only that dependency is updated. Otherwise all
/// version-based dependencies are checked for updates.
pub async fn run_deps_update(workspace: &Path, name: Option<&str>) -> Result<()> {
    let cfg = config::load_config(workspace).unwrap_or_default();
    let client = build_registry_client(&cfg, workspace)?;
    let cache_dir = workspace.join(".duumbi").join("cache");

    let default_registry = cfg
        .workspace
        .as_ref()
        .and_then(|ws| ws.default_registry.clone());

    let mut updates = Vec::new();

    let deps_to_check: Vec<(String, DependencyConfig)> = match name {
        Some(n) => {
            let dep = cfg
                .dependencies
                .get(n)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Dependency '{n}' not found in config.toml"))?;
            vec![(n.to_string(), dep)]
        }
        None => cfg.dependencies.clone().into_iter().collect(),
    };

    for (dep_name, dep_config) in &deps_to_check {
        let (version_req_str, registry_name) = match dep_config {
            DependencyConfig::Version(v) => (v.as_str(), default_registry.as_deref()),
            DependencyConfig::VersionWithRegistry { version, registry } => {
                (version.as_str(), Some(registry.as_str()))
            }
            DependencyConfig::Path { .. } => {
                if name.is_some() {
                    eprintln!("  {dep_name}: local path dependency — skipped");
                }
                continue;
            }
        };

        let registry = registry_name.ok_or_else(|| {
            anyhow::anyhow!(
                "No registry for '{dep_name}': no default-registry configured.\n\
                 Set default-registry in [workspace] or use VersionWithRegistry."
            )
        })?;

        let version_req = semver::VersionReq::parse(version_req_str).with_context(|| {
            format!("Invalid version range for '{dep_name}': {version_req_str}")
        })?;

        match client
            .resolve_version(registry, dep_name, &version_req)
            .await
        {
            Ok(resolved) => {
                let version_str = resolved.to_string();
                // Check if we already have this version cached
                let cached = cache_dir
                    .join(dep_name.replace('/', "/"))
                    .parent()
                    .map(|scope| {
                        scope.join(format!(
                            "{}@{version_str}",
                            dep_name.split('/').last().unwrap_or(dep_name)
                        ))
                    });

                let already_cached = cached.as_ref().is_some_and(|p| p.exists());

                if !already_cached {
                    eprintln!("  {dep_name}: downloading {version_str}…");
                    client
                        .download_module(registry, dep_name, &version_str, &cache_dir)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }

                updates.push((dep_name.clone(), version_req_str.to_string(), version_str));
            }
            Err(e) => {
                eprintln!("  {dep_name}: could not resolve — {e}");
            }
        }
    }

    if updates.is_empty() {
        eprintln!("All dependencies are up to date.");
        return Ok(());
    }

    // Show version diff
    for (dep_name, old_req, new_ver) in &updates {
        eprintln!("  {dep_name}: {old_req} → {new_ver} (latest compatible)");
    }

    // Regenerate lockfile
    let config = config::load_config(workspace).unwrap_or_default();
    match deps::generate_lockfile(workspace, &config) {
        Ok(_) => eprintln!("Lockfile updated."),
        Err(e) => eprintln!("  Warning: could not regenerate lockfile: {e}"),
    }

    eprintln!("Updated {} dependencies.", updates.len());
    Ok(())
}

/// Searches for modules across configured registries.
///
/// If `registry` is specified, searches only that registry. Otherwise searches
/// all configured registries.
pub async fn run_search(workspace: &Path, query: &str, registry: Option<&str>) -> Result<()> {
    let cfg = config::load_config(workspace).unwrap_or_default();
    let client = build_registry_client(&cfg, workspace)?;

    let registries_to_search: Vec<String> = match registry {
        Some(r) => vec![r.to_string()],
        None => cfg.registries.keys().cloned().collect(),
    };

    let mut found_any = false;

    for reg_name in &registries_to_search {
        let result = client
            .search(reg_name, query)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        if result.results.is_empty() {
            continue;
        }

        found_any = true;

        if registries_to_search.len() > 1 {
            eprintln!("Registry: {reg_name}");
        }

        // Table header
        eprintln!("{:<40} {:<12} {}", "NAME", "VERSION", "DESCRIPTION");
        eprintln!("{}", "─".repeat(72));

        for hit in &result.results {
            let desc = hit.description.as_deref().unwrap_or("");
            let truncated = if desc.len() > 40 {
                format!("{}…", &desc[..39])
            } else {
                desc.to_string()
            };
            eprintln!("{:<40} {:<12} {truncated}", hit.name, hit.latest_version);
        }

        if result.total > result.results.len() as u64 {
            eprintln!(
                "\n  ({} of {} results shown)",
                result.results.len(),
                result.total
            );
        }

        eprintln!();
    }

    if !found_any {
        eprintln!("No modules found matching '{query}'.");
    }

    Ok(())
}

/// Verifies integrity of all dependencies against lockfile hashes.
///
/// Recomputes integrity hashes for each resolved dependency and compares
/// against the values recorded in `deps.lock`. Reports E015 on mismatch.
/// Returns `Ok(())` if all pass, or bails with an error if mismatches found.
pub fn run_deps_audit(workspace: &Path) -> Result<()> {
    let lock = deps::load_lockfile(workspace).context("Failed to read lockfile")?;

    if lock.dependencies.is_empty() {
        eprintln!("No dependencies to audit.");
        return Ok(());
    }

    let failures = deps::verify_lockfile(&lock).context("Failed to verify lockfile integrity")?;

    // Report results for each dependency
    let passed_count = lock.dependencies.len() - failures.len();
    let failure_names: std::collections::HashSet<&str> =
        failures.iter().map(|f| f.name.as_str()).collect();

    for entry in &lock.dependencies {
        let version = entry.version.as_deref().unwrap_or("?");
        if failure_names.contains(entry.name.as_str()) {
            eprintln!(
                "  \u{2717} {} v{version} — INTEGRITY MISMATCH (E015)",
                entry.name
            );
        } else if entry.integrity.is_some() {
            eprintln!("  \u{2713} {} v{version} — integrity OK", entry.name);
        } else {
            eprintln!(
                "  - {} v{version} — no integrity hash (v0 entry)",
                entry.name
            );
        }
    }

    if !failures.is_empty() {
        eprintln!();
        for f in &failures {
            eprintln!(
                "  E015: {}: expected {}, got {}",
                f.name, f.expected, f.actual
            );
        }
        anyhow::bail!(
            "Integrity audit failed: {}/{} dependencies have mismatches",
            failures.len(),
            lock.dependencies.len()
        );
    }

    eprintln!("\nAll {passed_count} dependencies passed integrity verification.");
    Ok(())
}

/// Displays the dependency tree as ASCII art.
///
/// Reads the lockfile for resolved entries and config for workspace identity.
/// The `_max_depth` parameter is reserved for future nested dependency support.
pub fn run_deps_tree(workspace: &Path, _max_depth: u32) -> Result<()> {
    let cfg = config::load_config(workspace).unwrap_or_default();
    let lock = deps::load_lockfile(workspace).context("Failed to read lockfile")?;

    // Workspace root name
    let ws_name = cfg
        .workspace
        .as_ref()
        .and_then(|ws| {
            if ws.name.is_empty() {
                None
            } else {
                Some(ws.name.as_str())
            }
        })
        .unwrap_or("(workspace)");

    eprintln!("{ws_name}");

    if lock.dependencies.is_empty() {
        eprintln!("  (no dependencies)");
        return Ok(());
    }

    let total = lock.dependencies.len();
    for (i, entry) in lock.dependencies.iter().enumerate() {
        let is_last = i == total - 1;
        let connector = if is_last { "└── " } else { "├── " };

        let version = entry.version.as_deref().unwrap_or("?");
        let source = format_source(entry);

        eprintln!("{connector}{} v{version} ({source})", entry.name);
    }

    Ok(())
}

/// Formats the source/provenance of a lock entry for display.
fn format_source(entry: &deps::LockEntry) -> String {
    if entry.vendored {
        return "vendored".to_string();
    }
    if let Some(ref src) = entry.source {
        return src.clone();
    }
    if let Some(ref path) = entry.effective_path() {
        return format!("path: {path}");
    }
    "unknown".to_string()
}

/// Removes a dependency from `config.toml`.
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

/// Vendors cached dependencies into `.duumbi/vendor/` for offline builds.
pub fn run_deps_vendor(workspace: &Path, all: bool, include: Option<&str>) -> Result<()> {
    let mode = if all {
        deps::VendorMode::All
    } else if let Some(pattern) = include {
        deps::VendorMode::Include(pattern.to_string())
    } else {
        deps::VendorMode::ConfigRules
    };

    let results =
        deps::vendor_dependencies(workspace, &mode).context("Failed to vendor dependencies")?;

    if results.is_empty() {
        eprintln!("No dependencies to vendor.");
    } else {
        for r in &results {
            eprintln!("  Vendored {} → {}", r.name, r.destination.display());
        }
        eprintln!("Vendored {} dependencies.", results.len());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parses a registry specifier like `@scope/name@^1.0` into `(name, optional_version)`.
///
/// Returns `("@scope/name", Some("^1.0"))` or `("@scope/name", None)`.
fn parse_registry_specifier(input: &str) -> (&str, Option<&str>) {
    // Find the second '@' — first one is the scope prefix
    if let Some(at_pos) = input[1..].find('@') {
        let split = at_pos + 1;
        (&input[..split], Some(&input[split + 1..]))
    } else {
        (input, None)
    }
}

/// Builds a [`RegistryClient`] from workspace config.
fn build_registry_client(cfg: &config::DuumbiConfig, _workspace: &Path) -> Result<RegistryClient> {
    let registries = cfg.registries.clone();
    if registries.is_empty() {
        anyhow::bail!(
            "No registries configured in config.toml.\n\
             Add a [registries] section with at least one registry URL."
        );
    }

    // TODO(#161): Load credentials from ~/.duumbi/credentials.toml
    let credentials: HashMap<String, RegistryCredential> = HashMap::new();

    RegistryClient::new(registries, credentials, None).map_err(|e| anyhow::anyhow!("{e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_specifier_with_version() {
        let (name, ver) = parse_registry_specifier("@duumbi/stdlib-math@^1.0");
        assert_eq!(name, "@duumbi/stdlib-math");
        assert_eq!(ver, Some("^1.0"));
    }

    #[test]
    fn parse_specifier_without_version() {
        let (name, ver) = parse_registry_specifier("@duumbi/stdlib-math");
        assert_eq!(name, "@duumbi/stdlib-math");
        assert_eq!(ver, None);
    }

    #[test]
    fn parse_specifier_exact_version() {
        let (name, ver) = parse_registry_specifier("@company/auth@2.1.0");
        assert_eq!(name, "@company/auth");
        assert_eq!(ver, Some("2.1.0"));
    }

    #[test]
    fn parse_specifier_tilde_version() {
        let (name, ver) = parse_registry_specifier("@scope/pkg@~3.2");
        assert_eq!(name, "@scope/pkg");
        assert_eq!(ver, Some("~3.2"));
    }
}
