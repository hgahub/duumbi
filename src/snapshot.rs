//! Git-like snapshot history for `duumbi undo`.
//!
//! Snapshots are stored as sequentially numbered JSON-LD files in
//! `.duumbi/history/`. Each `save_snapshot` call writes the current graph
//! before a mutation; `restore_latest` reverts to the most recent snapshot.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Saves the current graph JSON-LD source to `.duumbi/history/` before mutation.
///
/// Files are named `{N:06}.jsonld` (zero-padded 6-digit sequence numbers).
/// The next available sequence number is used automatically.
pub fn save_snapshot(workspace_root: &Path, source: &str) -> Result<PathBuf> {
    let history_dir = workspace_root.join(".duumbi").join("history");
    fs::create_dir_all(&history_dir).context("Failed to create .duumbi/history/ directory")?;

    let next_n = next_sequence_number(&history_dir)?;
    let snapshot_path = history_dir.join(format!("{next_n:06}.jsonld"));

    fs::write(&snapshot_path, source)
        .with_context(|| format!("Failed to write snapshot to '{}'", snapshot_path.display()))?;

    tracing::debug!("Snapshot saved: {}", snapshot_path.display());
    Ok(snapshot_path)
}

/// Restores the most recent snapshot to `.duumbi/graph/main.jsonld`.
///
/// Returns `Ok(true)` if a snapshot was restored, `Ok(false)` if no snapshots exist.
pub fn restore_latest(workspace_root: &Path) -> Result<bool> {
    let history_dir = workspace_root.join(".duumbi").join("history");
    if !history_dir.exists() {
        return Ok(false);
    }

    let latest = latest_snapshot_path(&history_dir)?;
    let Some(snapshot_path) = latest else {
        return Ok(false);
    };

    let snapshot_content = fs::read_to_string(&snapshot_path)
        .with_context(|| format!("Failed to read snapshot '{}'", snapshot_path.display()))?;

    let graph_path = workspace_root
        .join(".duumbi")
        .join("graph")
        .join("main.jsonld");
    fs::write(&graph_path, &snapshot_content)
        .with_context(|| format!("Failed to restore to '{}'", graph_path.display()))?;

    // Remove the snapshot after restoring (pop the history stack)
    fs::remove_file(&snapshot_path)
        .with_context(|| format!("Failed to remove snapshot '{}'", snapshot_path.display()))?;

    eprintln!("Restored from snapshot: {}", snapshot_path.display());
    Ok(true)
}

/// Returns the number of available snapshots (undo depth).
pub fn snapshot_count(workspace_root: &Path) -> Result<usize> {
    let history_dir = workspace_root.join(".duumbi").join("history");
    if !history_dir.exists() {
        return Ok(0);
    }

    let count = fs::read_dir(&history_dir)
        .context("Failed to read .duumbi/history/")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonld"))
        .count();

    Ok(count)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the next sequence number for a new snapshot.
fn next_sequence_number(history_dir: &Path) -> Result<u32> {
    let max = fs::read_dir(history_dir)
        .context("Failed to read history directory")?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let stem = Path::new(&name)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            stem?.parse::<u32>().ok()
        })
        .max();

    Ok(max.map_or(1, |n| n + 1))
}

/// Returns the path of the latest (highest-numbered) snapshot, or `None`.
fn latest_snapshot_path(history_dir: &Path) -> Result<Option<std::path::PathBuf>> {
    let latest = fs::read_dir(history_dir)
        .context("Failed to read history directory")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonld"))
        .filter_map(|entry| {
            let path = entry.path();
            let n: Option<u32> = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse().ok());
            n.map(|n| (n, path))
        })
        .max_by_key(|(n, _)| *n)
        .map(|(_, path)| path);

    Ok(latest)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_workspace(tmp: &TempDir) {
        let graph_dir = tmp.path().join(".duumbi").join("graph");
        fs::create_dir_all(&graph_dir).expect("invariant: workspace dirs must be created");
        fs::write(
            graph_dir.join("main.jsonld"),
            r#"{"@type": "duumbi:Module"}"#,
        )
        .expect("invariant: graph file must be writable");
    }

    #[test]
    fn save_and_restore_single_snapshot() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        setup_workspace(&tmp);

        let original_content = r#"{"@type": "duumbi:Module", "version": 1}"#;
        save_snapshot(tmp.path(), original_content).expect("snapshot must save");

        assert_eq!(snapshot_count(tmp.path()).expect("must count"), 1);

        // Overwrite graph with new content
        let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");
        fs::write(&graph_path, r#"{"@type": "duumbi:Module", "version": 2}"#).expect("must write");

        // Restore should bring back version 1
        let restored = restore_latest(tmp.path()).expect("must restore");
        assert!(restored);

        let content = fs::read_to_string(&graph_path).expect("must read");
        assert_eq!(content, original_content);

        // After restore, snapshot count should be 0
        assert_eq!(snapshot_count(tmp.path()).expect("must count"), 0);
    }

    #[test]
    fn restore_without_snapshots_returns_false() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        setup_workspace(&tmp);

        let restored = restore_latest(tmp.path()).expect("must not error");
        assert!(!restored);
    }

    #[test]
    fn multiple_snapshots_restores_latest() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        setup_workspace(&tmp);

        save_snapshot(tmp.path(), "version 1").expect("snap 1");
        save_snapshot(tmp.path(), "version 2").expect("snap 2");
        save_snapshot(tmp.path(), "version 3").expect("snap 3");

        assert_eq!(snapshot_count(tmp.path()).expect("count"), 3);

        restore_latest(tmp.path()).expect("restore");

        // After restoring, the latest (3) was popped → 2 remain
        assert_eq!(snapshot_count(tmp.path()).expect("count"), 2);

        let graph_path = tmp.path().join(".duumbi").join("graph").join("main.jsonld");
        let content = fs::read_to_string(&graph_path).expect("read");
        assert_eq!(content, "version 3");
    }

    #[test]
    fn snapshot_count_without_history_dir_returns_zero() {
        let tmp = TempDir::new().expect("invariant: temp dir creation must succeed");
        assert_eq!(snapshot_count(tmp.path()).expect("count"), 0);
    }
}
