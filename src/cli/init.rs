//! Workspace initialization command.
//!
//! Creates the `.duumbi/` directory structure with default configuration,
//! schema, and a skeleton `main.jsonld` program.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

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

/// Default `config.toml` template.
const DEFAULT_CONFIG: &str = r#"[compiler]
version = "0.1"

[build]
output_dir = "build"

# Uncomment and configure to enable AI commands (duumbi add, duumbi undo).
# [llm]
# provider = "anthropic"      # or "openai"
# model = "claude-sonnet-4-6" # or "gpt-4o"
# api_key_env = "ANTHROPIC_API_KEY"  # name of env var holding the API key
"#;

/// Initializes a new duumbi workspace at the given base path.
///
/// Creates `.duumbi/` with subdirectories for config, graph, schema, build,
/// and telemetry. Fails if `.duumbi/` already exists.
pub fn run_init(base: &Path) -> Result<()> {
    let duumbi_dir = base.join(".duumbi");

    if duumbi_dir.exists() {
        anyhow::bail!("Workspace already exists at '{}'", duumbi_dir.display());
    }

    // Create directory structure
    fs::create_dir_all(duumbi_dir.join("graph")).context("Failed to create .duumbi/graph/")?;
    fs::create_dir_all(duumbi_dir.join("schema")).context("Failed to create .duumbi/schema/")?;
    fs::create_dir_all(duumbi_dir.join("build")).context("Failed to create .duumbi/build/")?;
    fs::create_dir_all(duumbi_dir.join("telemetry"))
        .context("Failed to create .duumbi/telemetry/")?;

    // Write config
    fs::write(duumbi_dir.join("config.toml"), DEFAULT_CONFIG)
        .context("Failed to write config.toml")?;

    // Write skeleton main.jsonld
    fs::write(duumbi_dir.join("graph").join("main.jsonld"), SKELETON_MAIN)
        .context("Failed to write main.jsonld")?;

    eprintln!("Project initialized at {}", duumbi_dir.display());
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
    }

    #[test]
    fn init_fails_if_already_exists() {
        let tmp = TempDir::new().expect("invariant: temp dir must be created");
        run_init(tmp.path()).expect("invariant: first init should succeed");
        let result = run_init(tmp.path());
        assert!(result.is_err());
    }
}
