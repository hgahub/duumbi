//! Atomic workspace rollback for failed agent team executions.
//!
//! [`WorkspaceSnapshot`] captures the state of all `.jsonld` files in a
//! workspace graph directory before a multi-agent team begins executing.
//! If any step fails the snapshot can be restored, writing back all
//! files that existed at snapshot time.  Files that were **created after**
//! the snapshot was taken are not removed by [`WorkspaceSnapshot::restore`];
//! callers are responsible for cleaning up any newly created files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// WorkspaceSnapshot
// ---------------------------------------------------------------------------

/// Snapshot of all `.jsonld` graph files in a workspace before team execution.
///
/// Created by [`WorkspaceSnapshot::capture`] and used to restore the workspace
/// to a known-good state via [`WorkspaceSnapshot::restore`].
#[derive(Debug)]
pub struct WorkspaceSnapshot {
    /// Map of absolute file path → file contents at snapshot time.
    files: HashMap<PathBuf, String>,
}

impl WorkspaceSnapshot {
    /// Capture the current state of all `.jsonld` files under `workspace`.
    ///
    /// Walks the entire directory tree rooted at `workspace` and reads every
    /// file whose name ends with `.jsonld`.  Subdirectories are traversed
    /// recursively.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if any file or directory cannot be read.
    pub fn capture(workspace: &Path) -> std::io::Result<Self> {
        let mut files = HashMap::new();
        collect_jsonld_files(workspace, &mut files)?;
        Ok(Self { files })
    }

    /// Restore all files captured in this snapshot, overwriting current state.
    ///
    /// Files that existed at snapshot time are written back with their original
    /// content.  Files that did not exist at snapshot time (created after the
    /// snapshot) are **not** removed — callers are responsible for cleaning up
    /// newly created files if needed.
    ///
    /// Parent directories are created if they no longer exist.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if any file cannot be written.
    pub fn restore(&self) -> std::io::Result<()> {
        for (path, content) in &self.files {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
        }
        Ok(())
    }

    /// Returns the map of captured file paths to their contents.
    #[must_use]
    pub fn files(&self) -> &HashMap<PathBuf, String> {
        &self.files
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Recursively collect all `.jsonld` files under `dir` into `out`.
fn collect_jsonld_files(dir: &Path, out: &mut HashMap<PathBuf, String>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_jsonld_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonld") {
            let content = fs::read_to_string(&path)?;
            out.insert(path, content);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).expect("write_file must succeed");
    }

    // -----------------------------------------------------------------------
    // capture
    // -----------------------------------------------------------------------

    #[test]
    fn capture_on_empty_directory_returns_empty_snapshot() {
        let tmp = TempDir::new().expect("tempdir");
        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");
        assert!(snapshot.files().is_empty());
    }

    #[test]
    fn capture_reads_jsonld_files() {
        let tmp = TempDir::new().expect("tempdir");
        write_file(tmp.path(), "main.jsonld", r#"{"@id":"main"}"#);
        write_file(tmp.path(), "ops.jsonld", r#"{"@id":"ops"}"#);
        write_file(tmp.path(), "readme.txt", "not jsonld");

        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");
        assert_eq!(snapshot.files().len(), 2, "only .jsonld files captured");
    }

    #[test]
    fn capture_ignores_non_jsonld_files() {
        let tmp = TempDir::new().expect("tempdir");
        write_file(tmp.path(), "config.toml", "[workspace]");
        write_file(tmp.path(), "output.o", "binary");

        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");
        assert!(snapshot.files().is_empty());
    }

    #[test]
    fn capture_traverses_subdirectories() {
        let tmp = TempDir::new().expect("tempdir");
        let sub = tmp.path().join("graph");
        fs::create_dir(&sub).expect("create subdir");
        write_file(&sub, "main.jsonld", r#"{"@id":"main"}"#);
        write_file(tmp.path(), "ops.jsonld", r#"{"@id":"ops"}"#);

        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");
        assert_eq!(snapshot.files().len(), 2);
    }

    // -----------------------------------------------------------------------
    // restore
    // -----------------------------------------------------------------------

    #[test]
    fn restore_roundtrip_preserves_file_contents() {
        let tmp = TempDir::new().expect("tempdir");
        write_file(tmp.path(), "main.jsonld", r#"{"@id":"original"}"#);

        // Capture original state.
        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");

        // Mutate the file.
        write_file(tmp.path(), "main.jsonld", r#"{"@id":"modified"}"#);

        // Restore.
        snapshot.restore().expect("restore");

        let content = fs::read_to_string(tmp.path().join("main.jsonld")).expect("read");
        assert_eq!(content, r#"{"@id":"original"}"#);
    }

    #[test]
    fn restore_overwrites_changed_files() {
        let tmp = TempDir::new().expect("tempdir");
        write_file(tmp.path(), "a.jsonld", "content_a_v1");
        write_file(tmp.path(), "b.jsonld", "content_b_v1");

        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");

        // Overwrite both files with new content.
        write_file(tmp.path(), "a.jsonld", "content_a_v2");
        write_file(tmp.path(), "b.jsonld", "content_b_v2");

        snapshot.restore().expect("restore");

        assert_eq!(
            fs::read_to_string(tmp.path().join("a.jsonld")).expect("read a"),
            "content_a_v1"
        );
        assert_eq!(
            fs::read_to_string(tmp.path().join("b.jsonld")).expect("read b"),
            "content_b_v1"
        );
    }

    #[test]
    fn files_returns_captured_paths() {
        let tmp = TempDir::new().expect("tempdir");
        write_file(tmp.path(), "x.jsonld", "{}");

        let snapshot = WorkspaceSnapshot::capture(tmp.path()).expect("capture");
        let files = snapshot.files();
        assert_eq!(files.len(), 1);
        let path = files.keys().next().expect("one path");
        assert!(path.ends_with("x.jsonld"));
    }

    #[test]
    fn capture_on_nonexistent_path_returns_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let nonexistent = tmp.path().join("does_not_exist");
        let snapshot = WorkspaceSnapshot::capture(&nonexistent).expect("capture nonexistent");
        assert!(snapshot.files().is_empty());
    }
}
