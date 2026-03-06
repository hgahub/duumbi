//! Intent-Driven Development system (Phase 5).
//!
//! An **intent** is a structured YAML specification that describes what a
//! user wants to build: acceptance criteria, affected modules, and test cases.
//! The intent system uses an LLM (via the existing agent infrastructure) to:
//!
//! 1. **Create** a structured spec from a natural language request.
//! 2. **Review** and optionally edit the spec before execution.
//! 3. **Execute** the spec: decompose → mutate graph → verify tests.
//! 4. **Report** the outcome and archive completed intents.
//!
//! # Directory layout
//!
//! ```text
//! .duumbi/intents/
//!   calculator.yaml          ← active intent specs
//!   history/
//!     calculator.yaml        ← archived after completion
//! ```

#![allow(dead_code)] // Progressively integrated as CLI commands are wired

pub mod coordinator;
pub mod create;
pub mod execute;
pub mod review;
pub mod spec;
pub mod status;
pub mod verifier;

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use spec::IntentSpec;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by the intent system.
#[derive(Debug, Error)]
pub enum IntentError {
    /// Intent file not found.
    #[error("Intent '{name}' not found in .duumbi/intents/")]
    NotFound {
        /// Intent slug name.
        name: String,
    },

    /// I/O error reading or writing an intent file.
    #[error("I/O error for '{path}': {source}")]
    Io {
        /// File path.
        path: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// YAML parse error.
    #[error("Failed to parse intent YAML at '{path}': {source}")]
    Parse {
        /// File path.
        path: String,
        /// Underlying error.
        #[source]
        source: serde_yaml::Error,
    },

    /// YAML serialization error.
    #[error("Failed to serialize intent YAML: {0}")]
    Serialize(#[from] serde_yaml::Error),
}

// ---------------------------------------------------------------------------
// File I/O helpers
// ---------------------------------------------------------------------------

/// Returns the `.duumbi/intents/` directory for a workspace.
pub fn intents_dir(workspace: &Path) -> PathBuf {
    workspace.join(".duumbi").join("intents")
}

/// Returns the `.duumbi/intents/history/` directory for a workspace.
pub fn history_dir(workspace: &Path) -> PathBuf {
    intents_dir(workspace).join("history")
}

/// Returns the path to an active intent YAML file.
pub fn intent_path(workspace: &Path, slug: &str) -> PathBuf {
    intents_dir(workspace).join(format!("{slug}.yaml"))
}

/// Loads an intent spec by slug from `.duumbi/intents/<slug>.yaml`.
#[must_use = "intent load errors should be handled"]
pub fn load_intent(workspace: &Path, slug: &str) -> Result<IntentSpec, IntentError> {
    let path = intent_path(workspace, slug);
    if !path.exists() {
        return Err(IntentError::NotFound {
            name: slug.to_string(),
        });
    }
    let contents = fs::read_to_string(&path).map_err(|source| IntentError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_yaml::from_str(&contents).map_err(|source| IntentError::Parse {
        path: path.display().to_string(),
        source,
    })
}

/// Saves an intent spec to `.duumbi/intents/<slug>.yaml`.
#[must_use = "intent save errors should be handled"]
pub fn save_intent(workspace: &Path, slug: &str, spec: &IntentSpec) -> Result<(), IntentError> {
    let dir = intents_dir(workspace);
    fs::create_dir_all(&dir).map_err(|source| IntentError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let path = intent_path(workspace, slug);
    let contents = serde_yaml::to_string(spec)?;
    fs::write(&path, contents).map_err(|source| IntentError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Lists all active intent slugs in `.duumbi/intents/` (excludes `history/`).
#[must_use = "intent list errors should be handled"]
pub fn list_intents(workspace: &Path) -> Result<Vec<String>, IntentError> {
    let dir = intents_dir(workspace);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut slugs = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|source| IntentError::Io {
        path: dir.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| IntentError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("yaml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            slugs.push(stem.to_string());
        }
    }
    slugs.sort();
    Ok(slugs)
}

/// Generates a URL-safe slug from a natural language intent string.
///
/// Lowercases, replaces non-alphanumeric chars with hyphens, trims hyphens,
/// and limits to 40 characters.
pub fn slugify(intent: &str) -> String {
    let raw: String = intent
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = raw.trim_matches('-');
    // Collapse consecutive hyphens
    let mut slug = String::new();
    let mut prev_hyphen = false;
    for c in trimmed.chars() {
        if c == '-' {
            if !prev_hyphen {
                slug.push('-');
            }
            prev_hyphen = true;
        } else {
            slug.push(c);
            prev_hyphen = false;
        }
    }
    slug.truncate(40);
    slug.trim_end_matches('-').to_string()
}

/// Makes a slug unique within `.duumbi/intents/` by appending a counter if needed.
pub fn unique_slug(workspace: &Path, base_slug: &str) -> String {
    let path = intent_path(workspace, base_slug);
    if !path.exists() {
        return base_slug.to_string();
    }
    for i in 2..=99 {
        let candidate = format!("{base_slug}-{i}");
        if !intent_path(workspace, &candidate).exists() {
            return candidate;
        }
    }
    format!("{base_slug}-{}", uuid_suffix())
}

fn uuid_suffix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Build a calculator"), "build-a-calculator");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("Hello, World! (test)"), "hello-world-test");
    }

    #[test]
    fn slugify_truncates_at_40() {
        let long = "a".repeat(100);
        assert_eq!(slugify(&long).len(), 40);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let spec = IntentSpec {
            intent: "Test intent".to_string(),
            version: 1,
            status: spec::IntentStatus::Pending,
            acceptance_criteria: vec!["criterion 1".to_string()],
            modules: spec::IntentModules {
                create: vec!["my/module".to_string()],
                modify: vec![],
            },
            test_cases: vec![],
            dependencies: vec![],
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            execution: None,
        };

        save_intent(tmp.path(), "test-intent", &spec).expect("must save");
        let loaded = load_intent(tmp.path(), "test-intent").expect("must load");
        assert_eq!(loaded.intent, "Test intent");
        assert_eq!(loaded.modules.create, vec!["my/module"]);
    }

    #[test]
    fn load_nonexistent_returns_not_found() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let err = load_intent(tmp.path(), "nonexistent").expect_err("must error");
        assert!(matches!(err, IntentError::NotFound { .. }));
    }

    #[test]
    fn list_intents_empty() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let slugs = list_intents(tmp.path()).expect("must list");
        assert!(slugs.is_empty());
    }

    #[test]
    fn list_intents_finds_files() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let spec = IntentSpec {
            intent: "Test".to_string(),
            version: 1,
            status: spec::IntentStatus::Pending,
            acceptance_criteria: vec![],
            modules: spec::IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            created_at: None,
            execution: None,
        };
        save_intent(tmp.path(), "alpha", &spec).expect("save");
        save_intent(tmp.path(), "beta", &spec).expect("save");

        let slugs = list_intents(tmp.path()).expect("list");
        assert_eq!(slugs, vec!["alpha", "beta"]);
    }

    #[test]
    fn unique_slug_no_conflict() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let slug = unique_slug(tmp.path(), "my-intent");
        assert_eq!(slug, "my-intent");
    }
}
