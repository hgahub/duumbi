//! CLI handler for `duumbi publish`.
//!
//! Packages the current module workspace and uploads it to a configured
//! registry. Validates the manifest and graph before packaging.

use std::fmt::Write as _;
use std::io::{self, Write as IoWrite};
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::config;
use crate::registry::credentials;
use crate::registry::package;

use super::registry::build_registry_client_with_credentials;

/// Runs the `duumbi publish` command.
///
/// Steps:
/// 1. Validate manifest exists and has required fields
/// 2. Run `duumbi check` to validate graph files
/// 3. Package module as `.tar.gz`
/// 4. Compute semantic hash of the archive
/// 5. If `--dry-run`, print summary with archive contents and exit
/// 6. Confirm with user (unless `--yes`)
/// 7. Verify auth token exists for the target registry (E014)
/// 8. Upload via `PUT /api/v1/modules/{name}`
/// 9. Print published URL on success
pub async fn run_publish(
    workspace: &Path,
    registry_name: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let cfg = config::load_config(workspace)
        .context("Failed to load .duumbi/config.toml. Run `duumbi init` to create a workspace.")?;

    // 1. Resolve target registry
    let target_registry = resolve_target_registry(&cfg, registry_name)?;
    let registry_url = cfg
        .registries
        .get(&target_registry)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Registry '{target_registry}' not found in config.toml.\n\
                 Add it with `duumbi registry add {target_registry} <url>`."
            )
        })?
        .clone();

    // 2. Validate graph files pass `duumbi check`
    let graph_path = workspace.join(".duumbi/graph/main.jsonld");
    if !graph_path.exists() {
        anyhow::bail!(
            "No graph file found at .duumbi/graph/main.jsonld.\n\
             A publishable module must have at least a main.jsonld."
        );
    }
    eprintln!("Validating graph...");
    super::commands::check(&graph_path)
        .context("Graph validation failed. Fix errors before publishing.")?;

    // 3. Package module
    eprintln!("Packaging module...");
    let tarball =
        package::pack_module(workspace).map_err(|e| anyhow::anyhow!("Packaging failed: {e}"))?;

    // 4. Compute integrity hash
    let mut hasher = Sha256::new();
    hasher.update(&tarball);
    let hash = hasher.finalize();
    let mut integrity = String::from("sha256:");
    for b in hash.as_slice() {
        write!(integrity, "{b:02x}").expect("invariant: writing to String cannot fail");
    }

    // 5. Read manifest for display
    let manifest_path = workspace.join(".duumbi/manifest.toml");
    let manifest = crate::manifest::parse_manifest(&manifest_path)
        .map_err(|e| anyhow::anyhow!("Failed to read manifest: {e}"))?;

    let module_name = &manifest.module.name;
    let module_version = &manifest.module.version;

    // List archive contents
    let file_list = list_tarball_contents(&tarball);

    eprintln!();
    eprintln!("  name:      {module_name}");
    eprintln!("  version:   {module_version}");
    eprintln!("  registry:  {target_registry} ({registry_url})");
    eprintln!("  size:      {}", format_size(tarball.len()));
    eprintln!("  integrity: {integrity}");
    eprintln!("  files:");
    for (name, size) in &file_list {
        eprintln!("    {name:<40} {}", format_size(*size));
    }

    if dry_run {
        eprintln!();
        eprintln!("Dry run — module packaged but not uploaded.");
        return Ok(());
    }

    // 6. Confirm with user
    if !yes {
        eprint!("\nPublish {module_name}@{module_version} to {target_registry}? [y/N] ");
        io::stderr().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read confirmation")?;

        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    // 7. Verify auth token exists
    let creds = credentials::load_credentials()
        .map_err(|e| anyhow::anyhow!("Failed to load credentials: {e}"))?;
    if credentials::get_token(&creds, &target_registry).is_none() {
        anyhow::bail!(
            "No authentication token for registry '{target_registry}' (E014).\n\
             Run `duumbi registry login {target_registry}` first."
        );
    }

    // 8. Upload
    eprintln!("Publishing {module_name}@{module_version} to {target_registry}...");

    let client = build_registry_client_with_credentials(&cfg)?;
    let response = client
        .publish(&target_registry, module_name, &tarball)
        .await
        .map_err(|e| anyhow::anyhow!("Publish failed: {e}"))?;

    // 9. Success
    eprintln!();
    eprintln!(
        "Published {}@{} to {target_registry}.",
        response.name, response.version
    );
    eprintln!(
        "  URL: {}/modules/{}",
        registry_url.trim_end_matches('/'),
        module_name
    );

    Ok(())
}

/// Resolves the target registry name from `--registry` flag or config defaults.
fn resolve_target_registry(cfg: &config::DuumbiConfig, explicit: Option<&str>) -> Result<String> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }

    // Try default-registry from [workspace]
    if let Some(ref ws) = cfg.workspace
        && let Some(ref default) = ws.default_registry
    {
        return Ok(default.clone());
    }

    // Fall back to the sole configured registry (if exactly one)
    if cfg.registries.len() == 1 {
        return Ok(cfg
            .registries
            .keys()
            .next()
            .expect("invariant: len==1")
            .clone());
    }

    anyhow::bail!(
        "No target registry specified.\n\
         Use `--registry <name>` or set a default with `duumbi registry default <name>`."
    )
}

/// Formats a byte count as a human-readable size string.
fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Lists files inside a `.tar.gz` archive with their sizes.
fn list_tarball_contents(data: &[u8]) -> Vec<(String, usize)> {
    let decoder = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    let mut entries = Vec::new();

    if let Ok(iter) = archive.entries() {
        for entry in iter.flatten() {
            let path = entry
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "<unknown>".to_string());
            let size = entry.size() as usize;
            entries.push((path, size));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DuumbiConfig, WorkspaceSection};
    use std::collections::HashMap;

    /// Minimal valid JSON-LD graph that passes `duumbi check`.
    const VALID_GRAPH: &str = r#"{
  "@context": { "duumbi": "https://duumbi.dev/ns/core#" },
  "@type": "duumbi:Module",
  "@id": "duumbi:main",
  "duumbi:name": "main",
  "duumbi:functions": [
    {
      "@type": "duumbi:Function",
      "@id": "duumbi:main/main",
      "duumbi:name": "main",
      "duumbi:returnType": "i64",
      "duumbi:blocks": [
        {
          "@type": "duumbi:Block",
          "@id": "duumbi:main/main/entry",
          "duumbi:label": "entry",
          "duumbi:ops": [
            {
              "@type": "duumbi:Const",
              "@id": "duumbi:main/main/entry/0",
              "duumbi:value": 0,
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Return",
              "@id": "duumbi:main/main/entry/1",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/0" }
            }
          ]
        }
      ]
    }
  ]
}"#;

    fn make_config(registries: Vec<(&str, &str)>, default: Option<&str>) -> DuumbiConfig {
        let mut reg_map = HashMap::new();
        for (name, url) in registries {
            reg_map.insert(name.to_string(), url.to_string());
        }
        DuumbiConfig {
            workspace: Some(WorkspaceSection {
                name: "test".to_string(),
                namespace: String::new(),
                default_registry: default.map(|s| s.to_string()),
            }),
            llm: None,
            providers: Vec::new(),
            registries: reg_map,
            dependencies: HashMap::new(),
            vendor: None,
            cost: None,
            agent: None,
            editor: None,
            mcp_clients: HashMap::new(),
        }
    }

    /// Creates a publishable workspace with valid graph and manifest.
    fn make_publishable_workspace(dir: &std::path::Path, name: &str, version: &str) {
        use std::fs;

        let duumbi = dir.join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("invariant: create dirs");

        fs::write(
            duumbi.join("config.toml"),
            "[workspace]\nname = \"test\"\n\n[registries]\ntest = \"https://test.dev\"\n",
        )
        .expect("invariant: write config");

        let manifest =
            crate::manifest::ModuleManifest::new(name, version, "Test module", vec!["main".into()]);
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml())
            .expect("invariant: write manifest");

        fs::write(graph.join("main.jsonld"), VALID_GRAPH).expect("invariant: write graph");
    }

    #[test]
    fn resolve_explicit_registry() {
        let cfg = make_config(vec![("duumbi", "https://r.dev")], None);
        let result = resolve_target_registry(&cfg, Some("duumbi")).expect("must resolve");
        assert_eq!(result, "duumbi");
    }

    #[test]
    fn resolve_default_registry() {
        let cfg = make_config(
            vec![("duumbi", "https://r.dev"), ("company", "https://c.dev")],
            Some("company"),
        );
        let result = resolve_target_registry(&cfg, None).expect("must resolve");
        assert_eq!(result, "company");
    }

    #[test]
    fn resolve_sole_registry() {
        let cfg = make_config(vec![("only", "https://only.dev")], None);
        let result = resolve_target_registry(&cfg, None).expect("must resolve");
        assert_eq!(result, "only");
    }

    #[test]
    fn resolve_ambiguous_registries_fails() {
        let cfg = make_config(vec![("a", "https://a.dev"), ("b", "https://b.dev")], None);
        let err = resolve_target_registry(&cfg, None).expect_err("must fail");
        assert!(err.to_string().contains("No target registry"));
    }

    #[test]
    fn dry_run_pack_works() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/pub", "1.0.0");

        let tarball = package::pack_module(tmp.path()).expect("pack must succeed");
        assert!(!tarball.is_empty());
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn list_tarball_contents_returns_sorted_entries() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/list", "1.0.0");

        let tarball = package::pack_module(tmp.path()).expect("pack must succeed");
        let entries = list_tarball_contents(&tarball);

        assert!(!entries.is_empty(), "tarball must have entries");

        // Check sorted
        let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "entries must be sorted");

        // Must contain known files
        assert!(
            entries.iter().any(|(n, _)| n == "manifest.toml"),
            "must contain manifest.toml"
        );
        assert!(
            entries.iter().any(|(n, _)| n == "graph/main.jsonld"),
            "must contain graph/main.jsonld"
        );
    }

    #[tokio::test]
    async fn publish_requires_auth_token() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/noauth", "1.0.0");

        let err = run_publish(tmp.path(), Some("test"), false, true)
            .await
            .expect_err("must fail without auth");
        assert!(
            err.to_string().contains("E014") || err.to_string().contains("authentication"),
            "expected auth error, got: {err}"
        );
    }

    #[tokio::test]
    async fn publish_dry_run_succeeds_without_auth() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/dryrun", "2.0.0");

        run_publish(tmp.path(), Some("test"), true, false)
            .await
            .expect("dry-run must succeed without auth");
    }

    #[tokio::test]
    async fn publish_missing_manifest_fails() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("invariant: create dirs");

        fs::write(
            duumbi.join("config.toml"),
            "[workspace]\nname = \"test\"\n\n[registries]\ntest = \"https://test.dev\"\n",
        )
        .expect("invariant: write config");

        // Valid graph but NO manifest — packaging should fail
        fs::write(graph.join("main.jsonld"), VALID_GRAPH).expect("invariant: write graph");

        let err = run_publish(tmp.path(), Some("test"), true, false)
            .await
            .expect_err("must fail without manifest");
        assert!(
            err.to_string().contains("manifest") || err.to_string().contains("Packaging"),
            "expected manifest error, got: {err}"
        );
    }

    #[tokio::test]
    async fn publish_no_registry_configured_fails() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: create dirs");

        fs::write(duumbi.join("config.toml"), "[workspace]\nname = \"test\"\n")
            .expect("invariant: write config");

        let err = run_publish(tmp.path(), None, true, false)
            .await
            .expect_err("must fail without registries");
        assert!(err.to_string().contains("No target registry"));
    }

    #[tokio::test]
    async fn publish_invalid_graph_fails() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("invariant: create dirs");

        fs::write(
            duumbi.join("config.toml"),
            "[workspace]\nname = \"test\"\n\n[registries]\ntest = \"https://test.dev\"\n",
        )
        .expect("invariant: write config");

        let manifest = crate::manifest::ModuleManifest::new(
            "@test/bad",
            "1.0.0",
            "Bad graph",
            vec!["main".into()],
        );
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml())
            .expect("invariant: write manifest");

        // Invalid graph (missing @id, no functions, etc.)
        fs::write(graph.join("main.jsonld"), "{}").expect("invariant: write graph");

        let err = run_publish(tmp.path(), Some("test"), true, false)
            .await
            .expect_err("must fail with invalid graph");
        assert!(
            err.to_string().contains("Graph validation failed"),
            "expected graph validation error, got: {err}"
        );
    }

    #[tokio::test]
    async fn publish_missing_graph_fails() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        let graph = duumbi.join("graph");
        fs::create_dir_all(&graph).expect("invariant: create dirs");

        fs::write(
            duumbi.join("config.toml"),
            "[workspace]\nname = \"test\"\n\n[registries]\ntest = \"https://test.dev\"\n",
        )
        .expect("invariant: write config");

        let manifest = crate::manifest::ModuleManifest::new(
            "@test/nograph",
            "1.0.0",
            "No graph",
            vec!["main".into()],
        );
        fs::write(duumbi.join("manifest.toml"), manifest.to_toml())
            .expect("invariant: write manifest");

        // No main.jsonld — should fail
        let err = run_publish(tmp.path(), Some("test"), true, false)
            .await
            .expect_err("must fail without main.jsonld");
        assert!(
            err.to_string().contains("No graph file found"),
            "expected missing graph error, got: {err}"
        );
    }

    #[tokio::test]
    async fn publish_registry_not_in_config_fails() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        make_publishable_workspace(tmp.path(), "@test/noreg", "1.0.0");

        let err = run_publish(tmp.path(), Some("nonexistent"), true, false)
            .await
            .expect_err("must fail for unknown registry");
        assert!(
            err.to_string().contains("not found in config.toml"),
            "expected registry not found error, got: {err}"
        );
    }
}
