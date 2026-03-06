//! Workspace initialization command.
//!
//! Creates the `.duumbi/` directory structure with default configuration,
//! schema, a skeleton `main.jsonld` program, and the standard library modules
//! in the M5 cache layout (`.duumbi/cache/@duumbi/<name>@<version>/`).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::manifest::ModuleManifest;

/// Embedded `stdlib/math.jsonld` — abs, max, min for i64.
const STDLIB_MATH: &str = include_str!("../../stdlib/math.jsonld");

/// Embedded `stdlib/io.jsonld` — print wrappers for i64, f64, bool.
const STDLIB_IO: &str = include_str!("../../stdlib/io.jsonld");

/// Stdlib module versions pinned at init time.
const STDLIB_VERSION: &str = "1.0.0";

/// Skeleton `main.jsonld` — a simple `add(3, 5)` program.
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
              "duumbi:value": 3,
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Const",
              "@id": "duumbi:main/main/entry/1",
              "duumbi:value": 5,
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Add",
              "@id": "duumbi:main/main/entry/2",
              "duumbi:left": { "@id": "duumbi:main/main/entry/0" },
              "duumbi:right": { "@id": "duumbi:main/main/entry/1" },
              "duumbi:resultType": "i64"
            },
            {
              "@type": "duumbi:Print",
              "@id": "duumbi:main/main/entry/3",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
            },
            {
              "@type": "duumbi:Return",
              "@id": "duumbi:main/main/entry/4",
              "duumbi:operand": { "@id": "duumbi:main/main/entry/2" }
            }
          ]
        }
      ]
    }
  ]
}
"#;

/// Default `config.toml` template (M5 format with scope-based stdlib deps).
const DEFAULT_CONFIG: &str = r#"[compiler]
version = "0.1"

[build]
output_dir = "build"

# Standard library modules (created in .duumbi/cache/@duumbi/ by duumbi init).
# The cache directory is excluded from version control (.gitignore).
[dependencies]
"@duumbi/stdlib-math" = "1.0.0"
"@duumbi/stdlib-io" = "1.0.0"

# Uncomment and configure to enable AI commands (duumbi add, duumbi undo).
# [llm]
# provider = "anthropic"      # or "openai"
# model = "claude-sonnet-4-6" # or "gpt-4o"
# api_key_env = "ANTHROPIC_API_KEY"  # name of env var holding the API key
"#;

/// `.gitignore` template — excludes the auto-generated cache and build dirs.
const GITIGNORE: &str = "\
# duumbi generated — do not commit
.duumbi/cache/
.duumbi/build/
.duumbi/history/
.duumbi/telemetry/
";

/// Initializes a new duumbi workspace at the given base path.
///
/// Creates `.duumbi/` with subdirectories for config, graph, schema, build,
/// telemetry, and intents. Stdlib modules are written to the M5 cache layout:
/// `.duumbi/cache/@duumbi/stdlib-math@1.0.0/` and
/// `.duumbi/cache/@duumbi/stdlib-io@1.0.0/`.
/// Fails if `.duumbi/` already exists.
pub fn run_init(base: &Path) -> Result<()> {
    let duumbi_dir = base.join(".duumbi");

    if duumbi_dir.exists() {
        anyhow::bail!("Workspace already exists at '{}'", duumbi_dir.display());
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
            "Mathematical utility functions (abs, max, min) for i64",
            vec!["abs".to_string(), "max".to_string(), "min".to_string()],
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
            "I/O utility functions (print wrappers for i64, f64, bool)",
            vec![
                "print_i64".to_string(),
                "print_f64".to_string(),
                "print_bool".to_string(),
            ],
        ),
    )
    .context("Failed to write stdlib io module")?;

    // Write config (includes stdlib deps by default)
    fs::write(duumbi_dir.join("config.toml"), DEFAULT_CONFIG)
        .context("Failed to write config.toml")?;

    // Write skeleton main.jsonld
    fs::write(duumbi_dir.join("graph").join("main.jsonld"), SKELETON_MAIN)
        .context("Failed to write main.jsonld")?;

    // Write .gitignore alongside .duumbi/ in the workspace root
    let gitignore = base.join(".gitignore");
    if !gitignore.exists() {
        fs::write(&gitignore, GITIGNORE).context("Failed to write .gitignore")?;
    }

    eprintln!("Project initialized at {}", duumbi_dir.display());
    Ok(())
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
    fn init_fails_if_already_exists() {
        let tmp = TempDir::new().expect("invariant: temp dir must be created");
        run_init(tmp.path()).expect("invariant: first init should succeed");
        let result = run_init(tmp.path());
        assert!(result.is_err());
    }
}
