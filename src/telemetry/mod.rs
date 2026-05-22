//! Local telemetry configuration and artifact path helpers.
//!
//! Phase 13 telemetry is opt-in. The defaults here keep normal builds and runs
//! uninstrumented while giving traced runs a deterministic local artifact
//! location when later build and runtime cycles wire the feature through.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Environment variable overriding the local telemetry artifact directory.
pub const TELEMETRY_DIR_ENV: &str = "DUUMBI_TELEMETRY_DIR";

const DEFAULT_ARTIFACT_DIR: &str = ".duumbi/telemetry";

/// Optional local telemetry settings from the `[telemetry]` config section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct TelemetrySection {
    /// Whether telemetry is enabled by config.
    ///
    /// Build and run surfaces still require an explicit traced mode before
    /// emitting telemetry. This field only records the user's local preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Local directory for telemetry artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_dir: Option<PathBuf>,

    /// Whether traced runs may capture argument or value snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_values: Option<bool>,
}

impl TelemetrySection {
    /// Returns whether telemetry is enabled after defaults.
    #[must_use]
    pub fn effective_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    /// Returns whether argument or value snapshots may be captured after defaults.
    #[must_use]
    pub fn effective_capture_values(&self) -> bool {
        self.capture_values.unwrap_or(false)
    }

    /// Returns the configured artifact directory before env overrides.
    #[must_use]
    pub fn configured_artifact_dir(&self) -> &Path {
        self.artifact_dir
            .as_deref()
            .unwrap_or_else(|| Path::new(DEFAULT_ARTIFACT_DIR))
    }

    /// Resolves the effective local artifact directory.
    ///
    /// `DUUMBI_TELEMETRY_DIR` wins when set to a non-empty value. Relative paths
    /// are interpreted relative to `workspace_root`.
    #[must_use]
    pub fn effective_artifact_dir(&self, workspace_root: &Path) -> PathBuf {
        let path = std::env::var_os(TELEMETRY_DIR_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.configured_artifact_dir().to_path_buf());

        if path.is_absolute() {
            path
        } else {
            workspace_root.join(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn telemetry_defaults_are_local_and_off() {
        let section = TelemetrySection::default();

        assert!(!section.effective_enabled());
        assert!(!section.effective_capture_values());
        assert_eq!(
            section.configured_artifact_dir(),
            Path::new(DEFAULT_ARTIFACT_DIR)
        );
    }

    #[test]
    fn telemetry_artifact_dir_resolves_relative_to_workspace() {
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("custom/telemetry")),
            ..TelemetrySection::default()
        };

        assert_eq!(
            section.effective_artifact_dir(workspace.path()),
            workspace.path().join("custom/telemetry")
        );
    }

    #[test]
    fn telemetry_env_override_wins() {
        let _guard = ENV_LOCK
            .lock()
            .expect("invariant: test env lock must not be poisoned");
        let workspace = TempDir::new().expect("invariant: temp dir creation must succeed");
        let override_dir = workspace.path().join("env-telemetry");
        // SAFETY: this test serializes environment mutation with ENV_LOCK and
        // removes the variable before releasing the lock.
        unsafe {
            std::env::set_var(TELEMETRY_DIR_ENV, &override_dir);
        }

        let section = TelemetrySection {
            artifact_dir: Some(PathBuf::from("config-telemetry")),
            ..TelemetrySection::default()
        };

        assert_eq!(
            section.effective_artifact_dir(workspace.path()),
            override_dir
        );
        // SAFETY: see the set_var safety note above; the same lock is held.
        unsafe {
            std::env::remove_var(TELEMETRY_DIR_ENV);
        }
    }
}
