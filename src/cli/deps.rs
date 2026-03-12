//! CLI handlers for the `duumbi deps` subcommand.
//!
//! Manages dependencies declared in `.duumbi/config.toml` — both local path
//! dependencies and registry-resolved scoped modules.

use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{self, DependencyConfig};
use crate::deps;
use crate::registry::client::RegistryClient;

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

    // Determine which registry to use (scope-level routing: #171)
    let registry_name = resolve_registry_for_module(registry, module_name, &cfg)?;

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
        let version_req_str = match dep_config {
            DependencyConfig::Version(v)
            | DependencyConfig::VersionWithRegistry { version: v, .. } => v.as_str(),
            DependencyConfig::Path { .. } => {
                if name.is_some() {
                    eprintln!("  {dep_name}: local path dependency — skipped");
                }
                continue;
            }
        };

        // Use scope-level routing (#171) for registry resolution
        let registry = match resolve_registry_for_module(dep_config.registry(), dep_name, &cfg) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  {dep_name}: could not determine registry — {e}");
                continue;
            }
        };

        let version_req = semver::VersionReq::parse(version_req_str).with_context(|| {
            format!("Invalid version range for '{dep_name}': {version_req_str}")
        })?;

        match client
            .resolve_version(&registry, dep_name, &version_req)
            .await
        {
            Ok(resolved) => {
                let version_str = resolved.to_string();
                // Check if we already have this version cached
                let cached = cache_dir.join(dep_name).parent().map(|scope| {
                    scope.join(format!(
                        "{}@{version_str}",
                        dep_name.split('/').next_back().unwrap_or(dep_name)
                    ))
                });

                let already_cached = cached.as_ref().is_some_and(|p| p.exists());

                if !already_cached {
                    eprintln!("  {dep_name}: downloading {version_str}…");
                    client
                        .download_module(&registry, dep_name, &version_str, &cache_dir)
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
        eprintln!("{:<40} {:<12} DESCRIPTION", "NAME", "VERSION");
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

/// Downloads and resolves all dependencies from registries into the cache.
///
/// Iterates over all version-based dependencies in `config.toml`, resolves
/// their versions via the appropriate registry, downloads any missing modules
/// to `.duumbi/cache/`, and generates/updates the lockfile.
///
/// With `--frozen`, aborts if the lockfile would change (for CI reproducibility).
pub async fn run_deps_install(workspace: &Path, frozen: bool) -> Result<()> {
    let cfg = config::load_config(workspace)
        .context("Failed to load config. Run `duumbi init` first.")?;

    // Load existing lockfile for --frozen comparison
    let existing_lock_str = if frozen {
        let lock_path = workspace.join(".duumbi").join("deps.lock");
        if lock_path.exists() {
            Some(std::fs::read_to_string(&lock_path).context("Failed to read existing deps.lock")?)
        } else {
            Some(String::new())
        }
    } else {
        None
    };

    // Collect version-based deps that need resolution
    let version_deps: Vec<(String, &DependencyConfig)> = cfg
        .dependencies
        .iter()
        .filter(|(_, dep)| dep.version().is_some())
        .map(|(name, dep)| (name.clone(), dep))
        .collect();

    if version_deps.is_empty() && cfg.dependencies.is_empty() {
        eprintln!("No dependencies declared in config.toml.");
        return Ok(());
    }

    let cache_dir = workspace.join(".duumbi").join("cache");
    let mut downloaded = 0u32;
    let mut cached = 0u32;
    let mut path_count = 0u32;

    // Download registry deps
    if !version_deps.is_empty() {
        let client = build_registry_client(&cfg, workspace)?;

        for (dep_name, dep_config) in &version_deps {
            let version_str = dep_config
                .version()
                .expect("invariant: filtered for version deps");

            let registry_name =
                match resolve_registry_for_module(dep_config.registry(), dep_name, &cfg) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("  {dep_name}: could not resolve registry — {e}");
                        continue;
                    }
                };

            let version_req = semver::VersionReq::parse(version_str).with_context(|| {
                format!("Invalid version range for '{dep_name}': {version_str}")
            })?;

            // Resolve best matching version
            let resolved = client
                .resolve_version(&registry_name, dep_name, &version_req)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to resolve '{dep_name}': {e}"))?;

            let resolved_str = resolved.to_string();

            // Check if already in cache
            let cache_entry = if let Some((scope, short_name)) = deps::parse_scoped_name(dep_name) {
                cache_dir
                    .join(scope)
                    .join(format!("{short_name}@{resolved_str}"))
            } else {
                cache_dir.join(format!("{dep_name}@{resolved_str}"))
            };

            if cache_entry.exists() {
                eprintln!("  {dep_name}@{resolved_str} — cached");
                cached += 1;
            } else {
                eprintln!("  {dep_name}@{resolved_str} — downloading from {registry_name}…");
                client
                    .download_module(&registry_name, dep_name, &resolved_str, &cache_dir)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to download '{dep_name}': {e}"))?;
                downloaded += 1;
            }
        }
    }

    // Count path deps
    for dep in cfg.dependencies.values() {
        if dep.path().is_some() {
            path_count += 1;
        }
    }

    // Generate/update lockfile
    let lock = deps::generate_lockfile(workspace, &cfg).context("Failed to generate lockfile")?;

    // --frozen check: compare new lockfile with existing
    if frozen {
        let new_lock_str = std::fs::read_to_string(workspace.join(".duumbi/deps.lock"))
            .context("Failed to read generated deps.lock")?;
        let existing = existing_lock_str.expect("invariant: loaded when frozen=true");

        if new_lock_str != existing {
            anyhow::bail!(
                "--frozen: lockfile would change ({} dependencies resolved).\n\
                 Run `duumbi deps install` without --frozen to update deps.lock.",
                lock.dependencies.len()
            );
        }
    }

    // Summary
    let total = downloaded + cached + path_count;
    eprintln!();
    eprintln!(
        "Installed {total} dependencies ({downloaded} downloaded, {cached} cached, {path_count} local)."
    );
    eprintln!("Lockfile: {} entries.", lock.dependencies.len());

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

/// Resolves which registry to use for a module, implementing scope-level routing (#171).
///
/// Priority order:
/// 1. Explicit `--registry` flag
/// 2. Scope-based auto-routing: `@scope/...` maps to registry named `scope`
///    (e.g. `@company/auth` → registry `company`)
/// 3. `@duumbi/*` always routes to registry `duumbi`
/// 4. Default registry from `[workspace] default-registry`
///
/// Returns an error if no matching registry is found.
fn resolve_registry_for_module(
    explicit: Option<&str>,
    module_name: &str,
    cfg: &config::DuumbiConfig,
) -> Result<String> {
    // 1. Explicit --registry flag takes highest priority
    if let Some(name) = explicit {
        if !cfg.registries.contains_key(name) {
            anyhow::bail!(
                "Registry '{name}' not found in config.toml.\n\
                 Add it with `duumbi registry add {name} <url>`."
            );
        }
        return Ok(name.to_string());
    }

    // 2. Scope-based auto-routing: @scope/name → registry "scope"
    if let Some(scope) = extract_scope(module_name) {
        // @duumbi/* always routes to "duumbi"
        let registry_name = if scope == "@duumbi" {
            "duumbi"
        } else {
            &scope[1..]
        };

        if cfg.registries.contains_key(registry_name) {
            return Ok(registry_name.to_string());
        }
        // Scope doesn't match any registry — fall through to default
    }

    // 3. Default registry from [workspace]
    if let Some(ref ws) = cfg.workspace
        && let Some(ref default) = ws.default_registry
        && cfg.registries.contains_key(default.as_str())
    {
        return Ok(default.clone());
    }

    anyhow::bail!(
        "No registry could be resolved for '{module_name}'.\n\
         Use --registry, configure a matching scope registry, or set default-registry in [workspace]."
    )
}

/// Extracts the scope from a scoped module name (e.g. `@scope/name` → `@scope`).
///
/// Returns `None` for unscoped names.
fn extract_scope(module_name: &str) -> Option<String> {
    if module_name.starts_with('@')
        && let Some(slash_pos) = module_name.find('/')
    {
        return Some(module_name[..slash_pos].to_string());
    }
    None
}

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

/// Builds a [`RegistryClient`] from workspace config with credentials
/// loaded from `~/.duumbi/credentials.toml`.
fn build_registry_client(cfg: &config::DuumbiConfig, _workspace: &Path) -> Result<RegistryClient> {
    super::registry::build_registry_client_with_credentials(cfg)
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

    // -----------------------------------------------------------------------
    // Scope-level routing tests (#171)
    // -----------------------------------------------------------------------

    fn make_cfg_with_registries(
        registries: &[(&str, &str)],
        default: Option<&str>,
    ) -> config::DuumbiConfig {
        let mut cfg = config::DuumbiConfig::default();
        for (name, url) in registries {
            cfg.registries.insert(name.to_string(), url.to_string());
        }
        if let Some(d) = default {
            let ws = cfg.workspace.get_or_insert_with(Default::default);
            ws.default_registry = Some(d.to_string());
        }
        cfg
    }

    #[test]
    fn routing_explicit_registry_wins() {
        let cfg = make_cfg_with_registries(
            &[
                ("duumbi", "https://r.duumbi.dev"),
                ("company", "https://r.acme.com"),
            ],
            Some("duumbi"),
        );
        let result = resolve_registry_for_module(Some("company"), "@duumbi/stdlib-math", &cfg);
        assert_eq!(result.expect("must resolve"), "company");
    }

    #[test]
    fn routing_explicit_registry_not_found() {
        let cfg = make_cfg_with_registries(&[("duumbi", "https://r.duumbi.dev")], None);
        let err = resolve_registry_for_module(Some("missing"), "@duumbi/math", &cfg);
        assert!(err.is_err());
    }

    #[test]
    fn routing_scope_based_duumbi() {
        let cfg = make_cfg_with_registries(&[("duumbi", "https://r.duumbi.dev")], None);
        let result = resolve_registry_for_module(None, "@duumbi/stdlib-math", &cfg);
        assert_eq!(result.expect("must resolve"), "duumbi");
    }

    #[test]
    fn routing_scope_based_company() {
        let cfg = make_cfg_with_registries(
            &[
                ("duumbi", "https://r.duumbi.dev"),
                ("company", "https://r.acme.com"),
            ],
            Some("duumbi"),
        );
        let result = resolve_registry_for_module(None, "@company/auth-core", &cfg);
        assert_eq!(result.expect("must resolve"), "company");
    }

    #[test]
    fn routing_scope_no_match_falls_to_default() {
        let cfg = make_cfg_with_registries(&[("duumbi", "https://r.duumbi.dev")], Some("duumbi"));
        let result = resolve_registry_for_module(None, "@unknown/pkg", &cfg);
        assert_eq!(result.expect("must fallback to default"), "duumbi");
    }

    #[test]
    fn routing_unscoped_uses_default() {
        let cfg = make_cfg_with_registries(&[("duumbi", "https://r.duumbi.dev")], Some("duumbi"));
        let result = resolve_registry_for_module(None, "simple-pkg", &cfg);
        assert_eq!(result.expect("must use default"), "duumbi");
    }

    #[test]
    fn routing_no_match_no_default_errors() {
        let cfg = make_cfg_with_registries(&[("duumbi", "https://r.duumbi.dev")], None);
        let err = resolve_registry_for_module(None, "@other/pkg", &cfg);
        assert!(err.is_err());
    }

    #[test]
    fn extract_scope_scoped() {
        assert_eq!(extract_scope("@duumbi/math"), Some("@duumbi".to_string()));
        assert_eq!(extract_scope("@company/auth"), Some("@company".to_string()));
    }

    #[test]
    fn extract_scope_unscoped() {
        assert_eq!(extract_scope("simple"), None);
        assert_eq!(extract_scope("no-scope"), None);
    }
}
