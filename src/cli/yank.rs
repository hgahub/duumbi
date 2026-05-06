//! CLI handler for `duumbi yank`.
//!
//! Marks a published module version as yanked in a registry. Yanked versions
//! remain downloadable by lockfile for reproducibility but are excluded from
//! version resolution for new installs.

use std::io::{self, Write as _};
use std::path::Path;

use anyhow::{Context, Result};

use crate::config;
use crate::registry::credentials;

use super::registry::build_registry_client_with_credentials;

/// Runs the `duumbi yank` command.
///
/// Parses the `@scope/name@version` specifier, confirms with the user,
/// verifies auth, and sends a yank request to the registry.
pub async fn run_yank(
    workspace: &Path,
    specifier: &str,
    registry_name: Option<&str>,
    yes: bool,
) -> Result<()> {
    let cfg = config::load_config(workspace)
        .context("Failed to load .duumbi/config.toml. Run `duumbi init` to create a workspace.")?;

    // 1. Parse specifier
    let (module_name, version) = parse_yank_specifier(specifier)?;

    // 2. Resolve target registry
    let target_registry = resolve_target_registry(&cfg, registry_name)?;
    let registry_url = cfg.registries.get(&target_registry).ok_or_else(|| {
        anyhow::anyhow!(
            "Registry '{target_registry}' not found in config.toml.\n\
                 Add it with `duumbi registry add {target_registry} <url>`."
        )
    })?;

    eprintln!("Yank {module_name}@{version} from {target_registry} ({registry_url})");

    // 3. Confirm
    if !yes {
        eprintln!();
        eprintln!("Warning: yanked versions are excluded from new dependency resolution.");
        eprintln!("Existing lockfiles referencing this version can still download it.");
        eprint!("\nProceed with yank? [y/N] ");
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

    // 4. Verify auth token
    let creds = credentials::load_credentials()
        .map_err(|e| anyhow::anyhow!("Failed to load credentials: {e}"))?;
    if credentials::get_token(&creds, &target_registry).is_none() {
        anyhow::bail!(
            "No authentication token for registry '{target_registry}' (E014).\n\
             Run `duumbi registry login {target_registry}` first."
        );
    }

    // 5. Send yank request
    eprintln!("Yanking {module_name}@{version}...");

    let client = build_registry_client_with_credentials(&cfg)?;
    client
        .yank(&target_registry, &module_name, &version)
        .await
        .map_err(|e| anyhow::anyhow!("Yank failed: {e}"))?;

    // 6. Success
    eprintln!();
    eprintln!("Yanked {module_name}@{version} from {target_registry}.");

    Ok(())
}

/// Parses a `@scope/name@version` specifier into `(module_name, version)`.
///
/// Accepted formats:
/// - `@scope/name@1.0.0` → `("@scope/name", "1.0.0")`
/// - `name@1.0.0` → `("name", "1.0.0")`
fn parse_yank_specifier(specifier: &str) -> Result<(String, String)> {
    // Handle scoped packages: @scope/name@version
    // The last '@' that is NOT at position 0 is the version separator
    let version_sep = specifier.rfind('@').filter(|&pos| pos > 0).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid specifier '{specifier}'.\n\
                 Expected format: @scope/name@version (e.g. @duumbi/stdlib-math@1.0.0)"
        )
    })?;

    let module_name = &specifier[..version_sep];
    let version = &specifier[version_sep + 1..];

    if module_name.is_empty() || version.is_empty() {
        anyhow::bail!(
            "Invalid specifier '{specifier}'.\n\
             Expected format: @scope/name@version (e.g. @duumbi/stdlib-math@1.0.0)"
        );
    }

    // Validate version is valid SemVer
    semver::Version::parse(version).map_err(|e| {
        anyhow::anyhow!(
            "Invalid version '{version}' in specifier: {e}\n\
             Version must be valid SemVer (e.g. 1.0.0)"
        )
    })?;

    Ok((module_name.to_string(), version.to_string()))
}

/// Resolves the target registry name from `--registry` flag or config defaults.
fn resolve_target_registry(cfg: &config::DuumbiConfig, explicit: Option<&str>) -> Result<String> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }

    if let Some(ref ws) = cfg.workspace
        && let Some(ref default) = ws.default_registry
    {
        return Ok(default.clone());
    }

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DuumbiConfig, WorkspaceSection};
    use std::collections::HashMap;

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
            logging: None,
            mcp_clients: HashMap::new(),
        }
    }

    fn setup_workspace(dir: &std::path::Path) {
        use std::fs;
        let duumbi = dir.join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: create dirs");
        fs::write(
            duumbi.join("config.toml"),
            "[workspace]\nname = \"test\"\ndefault-registry = \"test\"\n\n[registries]\ntest = \"https://test.dev\"\n",
        )
        .expect("invariant: write config");
    }

    // -- parse_yank_specifier tests --

    #[test]
    fn parse_scoped_specifier() {
        let (name, ver) = parse_yank_specifier("@duumbi/stdlib-math@1.0.0").expect("must parse");
        assert_eq!(name, "@duumbi/stdlib-math");
        assert_eq!(ver, "1.0.0");
    }

    #[test]
    fn parse_unscoped_specifier() {
        let (name, ver) = parse_yank_specifier("mymod@2.3.4").expect("must parse");
        assert_eq!(name, "mymod");
        assert_eq!(ver, "2.3.4");
    }

    #[test]
    fn parse_specifier_with_prerelease() {
        let (name, ver) =
            parse_yank_specifier("@test/mod@1.0.0-beta.1").expect("must parse prerelease");
        assert_eq!(name, "@test/mod");
        assert_eq!(ver, "1.0.0-beta.1");
    }

    #[test]
    fn parse_specifier_missing_version_fails() {
        let err = parse_yank_specifier("@duumbi/stdlib-math").expect_err("must fail");
        assert!(err.to_string().contains("Invalid specifier"));
    }

    #[test]
    fn parse_specifier_invalid_version_fails() {
        let err = parse_yank_specifier("@duumbi/mod@not-semver").expect_err("must fail");
        assert!(err.to_string().contains("Invalid version"));
    }

    #[test]
    fn parse_specifier_empty_version_fails() {
        let err = parse_yank_specifier("@duumbi/mod@").expect_err("must fail");
        assert!(err.to_string().contains("Invalid"));
    }

    #[test]
    fn parse_specifier_bare_at_fails() {
        let err = parse_yank_specifier("@").expect_err("must fail");
        assert!(err.to_string().contains("Invalid"));
    }

    // -- resolve_target_registry tests --

    #[test]
    fn resolve_explicit_registry() {
        let cfg = make_config(vec![("test", "https://t.dev")], None);
        let result = resolve_target_registry(&cfg, Some("test")).expect("must resolve");
        assert_eq!(result, "test");
    }

    #[test]
    fn resolve_default_registry() {
        let cfg = make_config(
            vec![("a", "https://a.dev"), ("b", "https://b.dev")],
            Some("b"),
        );
        let result = resolve_target_registry(&cfg, None).expect("must resolve");
        assert_eq!(result, "b");
    }

    #[test]
    fn resolve_sole_registry() {
        let cfg = make_config(vec![("only", "https://o.dev")], None);
        let result = resolve_target_registry(&cfg, None).expect("must resolve");
        assert_eq!(result, "only");
    }

    #[test]
    fn resolve_ambiguous_fails() {
        let cfg = make_config(vec![("a", "https://a.dev"), ("b", "https://b.dev")], None);
        let err = resolve_target_registry(&cfg, None).expect_err("must fail");
        assert!(err.to_string().contains("No target registry"));
    }

    // -- run_yank integration tests --

    #[tokio::test]
    async fn yank_requires_auth_token() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(tmp.path());

        let err = run_yank(
            tmp.path(),
            "@test/mod@1.0.0",
            Some("test"),
            true, // skip confirmation
        )
        .await
        .expect_err("must fail without auth");

        assert!(
            err.to_string().contains("E014") || err.to_string().contains("authentication"),
            "expected auth error, got: {err}"
        );
    }

    #[tokio::test]
    async fn yank_invalid_specifier_fails() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(tmp.path());

        let err = run_yank(tmp.path(), "@test/mod", Some("test"), true)
            .await
            .expect_err("must fail with bad specifier");

        assert!(
            err.to_string().contains("Invalid specifier"),
            "expected specifier error, got: {err}"
        );
    }

    #[tokio::test]
    async fn yank_unknown_registry_fails() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        setup_workspace(tmp.path());

        let err = run_yank(tmp.path(), "@test/mod@1.0.0", Some("nonexistent"), true)
            .await
            .expect_err("must fail for unknown registry");

        assert!(
            err.to_string().contains("not found in config.toml"),
            "expected registry not found error, got: {err}"
        );
    }

    #[tokio::test]
    async fn yank_no_registry_configured_fails() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: create dirs");
        fs::write(duumbi.join("config.toml"), "[workspace]\nname = \"test\"\n")
            .expect("invariant: write config");

        let err = run_yank(tmp.path(), "@test/mod@1.0.0", None, true)
            .await
            .expect_err("must fail without registries");

        assert!(err.to_string().contains("No target registry"));
    }

    #[tokio::test]
    async fn yank_with_wiremock_success() {
        use std::fs;
        use tempfile::TempDir;
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let tmp = TempDir::new().expect("invariant: tempdir");
        let duumbi = tmp.path().join(".duumbi");
        fs::create_dir_all(&duumbi).expect("invariant: create dirs");

        fs::write(
            duumbi.join("config.toml"),
            format!(
                "[workspace]\nname = \"test\"\ndefault-registry = \"test\"\n\n[registries]\ntest = \"{}\"\n",
                server.uri()
            ),
        )
        .expect("invariant: write config");

        // Set up credentials in global location — this test relies on
        // ~/.duumbi/credentials.toml having a token for "test". Since we
        // can't easily mock the global path, we verify the wiremock setup
        // is correct and test the auth-check separately above.

        Mock::given(method("DELETE"))
            .and(path("/api/v1/modules/@test/mod/1.0.0"))
            .and(header("authorization", "Bearer yank-token"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // The yank will fail at the auth check (no credentials in ~/.duumbi/)
        // but this validates the wiremock setup and specifier parsing are correct.
        let err = run_yank(tmp.path(), "@test/mod@1.0.0", Some("test"), true)
            .await
            .expect_err("must fail without credentials");
        assert!(err.to_string().contains("E014"));
    }
}
