//! Workspace initialization command.
//!
//! Creates the `.duumbi/` directory structure with default configuration,
//! schema, a skeleton `main.jsonld` program, and the standard library modules
//! in the M5 cache layout (`.duumbi/cache/@duumbi/<name>@<version>/`).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::manifest::ModuleManifest;

/// Embedded `stdlib/math.jsonld` — abs, max, min, sqrt, pow, mod, clamp, sign.
const STDLIB_MATH: &str = include_str!("../../stdlib/math.jsonld");

/// Embedded `stdlib/io.jsonld` — print wrappers for i64, f64, bool, string.
const STDLIB_IO: &str = include_str!("../../stdlib/io.jsonld");

/// Embedded `stdlib/lang.jsonld` — language utilities (assert_true).
const STDLIB_LANG: &str = include_str!("../../stdlib/lang.jsonld");

/// Embedded `stdlib/string.jsonld` — string utilities (length, contains, find, trim, to_upper, to_lower, replace).
const STDLIB_STRING: &str = include_str!("../../stdlib/string.jsonld");

/// Stdlib module versions pinned at init time.
const STDLIB_VERSION: &str = "1.0.0";

/// Skeleton `main.jsonld` — a minimal program that returns 0.
///
/// Users are expected to build on this blank slate using `duumbi add` or
/// by editing the graph directly. The old `add(3, 5)` sample is removed so
/// that new workspaces start empty.
const SKELETON_MAIN: &str = r#"{
  "@context": {
    "duumbi": "https://duumbi.dev/ns/core#"
  },
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
              "duumbi:value": 0
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
}
"#;

/// Maximum user-facing workspace name length accepted by init.
pub const MAX_WORKSPACE_NAME_CHARS: usize = 30;

/// Default `config.toml` template (M7 format with registries and scope-based deps).
const DEFAULT_CONFIG_TEMPLATE: &str = r#"[workspace]
name = {name}
namespace = {namespace}
default-registry = "duumbi"

[registries]
duumbi = "https://registry.duumbi.dev"

# Standard library modules (created in .duumbi/cache/@duumbi/ by duumbi init).
# The cache directory is excluded from version control (.gitignore).
[dependencies]
"@duumbi/stdlib-math" = "1.0.0"
"@duumbi/stdlib-io" = "1.0.0"
"@duumbi/stdlib-lang" = "1.0.0"
"@duumbi/stdlib-string" = "1.0.0"
"#;

/// `.gitignore` template — excludes the auto-generated cache and build dirs.
const GITIGNORE: &str = "\
# duumbi generated — do not commit
.duumbi/cache/
.duumbi/build/
.duumbi/history/
.duumbi/telemetry/
";

/// Result metadata for an initialized workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitSummary {
    /// Workspace root where `.duumbi/` was created.
    pub workspace_root: PathBuf,
    /// Validated display name written into config.
    pub workspace_name: String,
    /// Namespace slug written into config.
    pub namespace: String,
}

/// Options controlling workspace initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitOptions {
    /// User-facing workspace name.
    pub workspace_name: String,
    /// Namespace slug used by config and resolvers.
    pub namespace: String,
    /// Whether an existing `.duumbi/` directory may be deleted first.
    pub overwrite_existing: bool,
}

impl InitOptions {
    /// Builds validated init options from a workspace name.
    pub fn from_workspace_name(workspace_name: &str, overwrite_existing: bool) -> Result<Self> {
        let workspace_name = validate_workspace_name(workspace_name)?;
        let namespace = namespace_slug(&workspace_name);
        Ok(Self {
            workspace_name,
            namespace,
            overwrite_existing,
        })
    }
}

/// Returns the default workspace name for a path.
#[must_use]
pub fn default_workspace_name(base: &Path) -> String {
    let name = path_file_name(base).or_else(|| {
        base.canonicalize()
            .ok()
            .and_then(|canonical| path_file_name(&canonical))
    });
    name.unwrap_or_else(|| "workspace".to_string())
        .chars()
        .take(MAX_WORKSPACE_NAME_CHARS)
        .collect()
}

fn path_file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

/// Validates and normalizes a workspace display name.
pub fn validate_workspace_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Workspace name cannot be empty");
    }
    if trimmed.chars().count() > MAX_WORKSPACE_NAME_CHARS {
        anyhow::bail!(
            "Workspace name must be at most {} characters",
            MAX_WORKSPACE_NAME_CHARS
        );
    }
    Ok(trimmed.to_string())
}

/// Generates a portable namespace slug from a workspace name.
#[must_use]
pub fn namespace_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_separator = false;

    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(c.to_ascii_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }

    if slug.is_empty() {
        slug.push_str("workspace");
    }

    if !slug
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
    {
        slug.insert_str(0, "w-");
    }

    slug
}

/// Returns whether an existing `.duumbi/` directory contains any entries.
pub fn duumbi_dir_is_non_empty(base: &Path) -> Result<bool> {
    let duumbi_dir = base.join(".duumbi");
    if !duumbi_dir.exists() {
        return Ok(false);
    }
    Ok(fs::read_dir(&duumbi_dir)
        .with_context(|| format!("Failed to read '{}'", duumbi_dir.display()))?
        .next()
        .is_some())
}

/// Initializes a new duumbi workspace at the given base path.
///
/// Creates `.duumbi/` with subdirectories for config, graph, schema, build,
/// telemetry, and intents. Stdlib modules are written to the M5 cache layout:
/// `.duumbi/cache/@duumbi/stdlib-{math,io,lang,string}@1.0.0/`.
/// Fails if `.duumbi/` already exists and is non-empty.
pub fn run_init(base: &Path) -> Result<InitSummary> {
    let workspace_name = default_workspace_name(base);
    let options = InitOptions::from_workspace_name(&workspace_name, false)?;
    run_init_with_options(base, &options)
}

/// Initializes a new duumbi workspace using explicit options.
pub fn run_init_with_options(base: &Path, options: &InitOptions) -> Result<InitSummary> {
    let workspace_name = validate_workspace_name(&options.workspace_name)?;
    if options.namespace.trim().is_empty() {
        anyhow::bail!("Workspace namespace cannot be empty");
    }

    let duumbi_dir = base.join(".duumbi");

    if duumbi_dir.exists() {
        if options.overwrite_existing {
            fs::remove_dir_all(&duumbi_dir)
                .with_context(|| format!("Failed to remove '{}'", duumbi_dir.display()))?;
        } else if duumbi_dir_is_non_empty(base)? {
            anyhow::bail!("Workspace already exists at '{}'", duumbi_dir.display());
        }
    }

    // Core directory structure
    for subdir in &[
        "graph",
        "schema",
        "build",
        "telemetry",
        "intents",
        "history",
    ] {
        fs::create_dir_all(duumbi_dir.join(subdir))
            .with_context(|| format!("Failed to create .duumbi/{subdir}/"))?;
    }

    // Write stdlib math module to cache
    write_cache_module(
        &duumbi_dir,
        "@duumbi",
        "stdlib-math",
        STDLIB_VERSION,
        "math.jsonld",
        STDLIB_MATH,
        ModuleManifest::new(
            "@duumbi/stdlib-math",
            STDLIB_VERSION,
            "Mathematical utility functions (abs, max, min, sqrt, pow, mod, clamp, sign)",
            vec![
                "abs".to_string(),
                "max".to_string(),
                "min".to_string(),
                "sqrt".to_string(),
                "pow".to_string(),
                "mod".to_string(),
                "clamp".to_string(),
                "sign".to_string(),
            ],
        ),
    )
    .context("Failed to write stdlib math module")?;

    // Write stdlib io module to cache
    write_cache_module(
        &duumbi_dir,
        "@duumbi",
        "stdlib-io",
        STDLIB_VERSION,
        "io.jsonld",
        STDLIB_IO,
        ModuleManifest::new(
            "@duumbi/stdlib-io",
            STDLIB_VERSION,
            "I/O utility functions (print wrappers for i64, f64, bool, string)",
            vec![
                "print_i64".to_string(),
                "print_f64".to_string(),
                "print_bool".to_string(),
                "print_string".to_string(),
            ],
        ),
    )
    .context("Failed to write stdlib io module")?;

    // Write stdlib lang module to cache
    write_cache_module(
        &duumbi_dir,
        "@duumbi",
        "stdlib-lang",
        STDLIB_VERSION,
        "lang.jsonld",
        STDLIB_LANG,
        ModuleManifest::new(
            "@duumbi/stdlib-lang",
            STDLIB_VERSION,
            "Language utility functions (assert_true, i64_to_f64, f64_to_i64)",
            vec![
                "assert_true".to_string(),
                "i64_to_f64".to_string(),
                "f64_to_i64".to_string(),
            ],
        ),
    )
    .context("Failed to write stdlib lang module")?;

    // Write stdlib string module to cache
    write_cache_module(
        &duumbi_dir,
        "@duumbi",
        "stdlib-string",
        STDLIB_VERSION,
        "string.jsonld",
        STDLIB_STRING,
        ModuleManifest::new(
            "@duumbi/stdlib-string",
            STDLIB_VERSION,
            "String utility functions (length, contains, find, trim, to_upper, to_lower, replace)",
            vec![
                "length".to_string(),
                "contains".to_string(),
                "find".to_string(),
                "trim".to_string(),
                "to_upper".to_string(),
                "to_lower".to_string(),
                "replace".to_string(),
            ],
        ),
    )
    .context("Failed to write stdlib string module")?;

    // Write config (includes stdlib deps by default)
    let workspace_name_toml = toml::Value::String(workspace_name.clone()).to_string();
    let namespace_toml = toml::Value::String(options.namespace.clone()).to_string();
    fs::write(
        duumbi_dir.join("config.toml"),
        DEFAULT_CONFIG_TEMPLATE
            .replace("{name}", workspace_name_toml.trim())
            .replace("{namespace}", namespace_toml.trim()),
    )
    .context("Failed to write config.toml")?;

    // Write skeleton main.jsonld
    fs::write(duumbi_dir.join("graph").join("main.jsonld"), SKELETON_MAIN)
        .context("Failed to write main.jsonld")?;

    // Write .gitignore alongside .duumbi/ in the workspace root
    let gitignore = base.join(".gitignore");
    if !gitignore.exists() {
        fs::write(&gitignore, GITIGNORE).context("Failed to write .gitignore")?;
    }

    Ok(InitSummary {
        workspace_root: base.to_path_buf(),
        workspace_name,
        namespace: options.namespace.clone(),
    })
}

/// Writes a single stdlib module into the cache layer.
///
/// Creates: `.duumbi/cache/<scope>/<name>@<version>/graph/<jsonld_file>`
/// and:     `.duumbi/cache/<scope>/<name>@<version>/manifest.toml`
fn write_cache_module(
    duumbi_dir: &Path,
    scope: &str,
    name: &str,
    version: &str,
    jsonld_file: &str,
    jsonld_content: &str,
    manifest: ModuleManifest,
) -> Result<()> {
    let entry_dir = duumbi_dir
        .join("cache")
        .join(scope)
        .join(format!("{name}@{version}"));
    let graph_dir = entry_dir.join("graph");

    fs::create_dir_all(&graph_dir)
        .with_context(|| format!("Failed to create cache dir for {scope}/{name}"))?;

    fs::write(graph_dir.join(jsonld_file), jsonld_content)
        .with_context(|| format!("Failed to write {jsonld_file} for {scope}/{name}"))?;

    fs::write(entry_dir.join("manifest.toml"), manifest.to_toml())
        .with_context(|| format!("Failed to write manifest.toml for {scope}/{name}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_workspace_structure() {
        let tmp = TempDir::new().expect("invariant: temp dir must be created");
        run_init(tmp.path()).expect("invariant: init should succeed");

        let d = tmp.path().join(".duumbi");
        assert!(d.join("config.toml").exists());
        assert!(d.join("graph/main.jsonld").exists());
        assert!(d.join("schema").is_dir());
        assert!(d.join("build").is_dir());
        assert!(d.join("telemetry").is_dir());
        assert!(d.join("intents").is_dir());

        // M5 cache layout for stdlib
        assert!(
            d.join("cache/@duumbi/stdlib-math@1.0.0/graph/math.jsonld")
                .exists(),
            "stdlib-math jsonld must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-math@1.0.0/manifest.toml")
                .exists(),
            "stdlib-math manifest must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-io@1.0.0/graph/io.jsonld")
                .exists(),
            "stdlib-io jsonld must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-io@1.0.0/manifest.toml")
                .exists(),
            "stdlib-io manifest must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-lang@1.0.0/graph/lang.jsonld")
                .exists(),
            "stdlib-lang jsonld must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-lang@1.0.0/manifest.toml")
                .exists(),
            "stdlib-lang manifest must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-string@1.0.0/graph/string.jsonld")
                .exists(),
            "stdlib-string jsonld must exist"
        );
        assert!(
            d.join("cache/@duumbi/stdlib-string@1.0.0/manifest.toml")
                .exists(),
            "stdlib-string manifest must exist"
        );
    }

    #[test]
    fn init_writes_gitignore() {
        let tmp = TempDir::new().expect("tempdir");
        run_init(tmp.path()).expect("init must succeed");
        let gi = tmp.path().join(".gitignore");
        assert!(gi.exists(), ".gitignore must be created");
        let content = std::fs::read_to_string(&gi).expect("read .gitignore");
        assert!(
            content.contains(".duumbi/cache/"),
            ".gitignore must exclude cache"
        );
    }

    #[test]
    fn init_stdlib_manifests_are_valid() {
        let tmp = TempDir::new().expect("tempdir");
        run_init(tmp.path()).expect("init must succeed");

        let d = tmp.path().join(".duumbi");
        let math_manifest = crate::manifest::parse_manifest(
            &d.join("cache/@duumbi/stdlib-math@1.0.0/manifest.toml"),
        )
        .expect("math manifest must parse");
        assert_eq!(math_manifest.module.name, "@duumbi/stdlib-math");
        assert_eq!(math_manifest.module.version, "1.0.0");
        assert!(math_manifest.exports.functions.contains(&"abs".to_string()));
    }

    #[test]
    fn workspace_name_validation_trims_and_limits_length() {
        assert_eq!(
            validate_workspace_name("  My App  ").expect("valid"),
            "My App"
        );
        assert!(validate_workspace_name("").is_err());
        assert!(validate_workspace_name("   ").is_err());
        assert!(
            validate_workspace_name("a".repeat(MAX_WORKSPACE_NAME_CHARS + 1).as_str()).is_err()
        );
    }

    #[test]
    fn namespace_slug_is_portable() {
        assert_eq!(namespace_slug("My App"), "my-app");
        assert_eq!(namespace_slug("  My---App__v2  "), "my-app-v2");
        assert_eq!(namespace_slug("123 App"), "w-123-app");
        assert_eq!(namespace_slug("###"), "workspace");
    }

    #[test]
    fn default_workspace_name_for_dot_uses_current_directory_name() {
        let _lock = crate::cli::TEST_ENV_LOCK.lock().expect("env lock");
        let tmp = TempDir::new().expect("tempdir");
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(tmp.path()).expect("set current dir");

        let name = default_workspace_name(Path::new("."));

        std::env::set_current_dir(previous).expect("restore current dir");
        assert_eq!(
            name,
            tmp.path()
                .file_name()
                .and_then(|name| name.to_str())
                .expect("temp basename")
        );
    }

    #[test]
    fn init_config_writes_workspace_identity() {
        let tmp = TempDir::new().expect("tempdir");
        let options =
            InitOptions::from_workspace_name("Human Project", false).expect("valid options");
        let summary = run_init_with_options(tmp.path(), &options).expect("init must succeed");

        assert_eq!(summary.workspace_name, "Human Project");
        assert_eq!(summary.namespace, "human-project");
        let config = crate::config::load_config(tmp.path()).expect("config must parse");
        let ws = config.workspace.expect("workspace section must exist");
        assert_eq!(ws.name, "Human Project");
        assert_eq!(ws.namespace, "human-project");
    }

    #[test]
    fn init_config_uses_scope_based_deps() {
        let tmp = TempDir::new().expect("tempdir");
        run_init(tmp.path()).expect("init must succeed");

        let config = crate::config::load_config(tmp.path()).expect("config must parse");
        assert!(
            config.dependencies.contains_key("@duumbi/stdlib-math"),
            "config must have @duumbi/stdlib-math dep"
        );
        assert_eq!(
            config.dependencies["@duumbi/stdlib-math"].version(),
            Some("1.0.0")
        );
    }

    #[test]
    fn init_config_has_registry_section() {
        let tmp = TempDir::new().expect("tempdir");
        run_init(tmp.path()).expect("init must succeed");

        let config = crate::config::load_config(tmp.path()).expect("config must parse");
        assert_eq!(
            config.registries.get("duumbi").map(|s| s.as_str()),
            Some("https://registry.duumbi.dev"),
            "config must have duumbi registry"
        );
        let ws = config.workspace.expect("workspace section must exist");
        assert_eq!(
            ws.default_registry.as_deref(),
            Some("duumbi"),
            "default-registry must be duumbi"
        );
    }

    #[test]
    fn init_fails_if_already_exists() {
        let tmp = TempDir::new().expect("invariant: temp dir must be created");
        run_init(tmp.path()).expect("invariant: first init should succeed");
        let result = run_init(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn init_overwrite_replaces_only_duumbi_directory() {
        let tmp = TempDir::new().expect("invariant: temp dir must be created");
        run_init(tmp.path()).expect("invariant: first init should succeed");
        fs::write(tmp.path().join(".duumbi").join("old-marker"), "delete me").expect("write");
        fs::write(tmp.path().join("root-marker"), "keep me").expect("write");

        let options = InitOptions::from_workspace_name("Second App", true).expect("valid options");
        run_init_with_options(tmp.path(), &options).expect("overwrite init must succeed");

        assert!(!tmp.path().join(".duumbi").join("old-marker").exists());
        assert_eq!(
            fs::read_to_string(tmp.path().join("root-marker")).expect("read root marker"),
            "keep me"
        );
        let config = crate::config::load_config(tmp.path()).expect("config must parse");
        assert_eq!(config.workspace.expect("workspace").namespace, "second-app");
    }
}
