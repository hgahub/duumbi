//! File-based CRUD storage for knowledge nodes.
//!
//! Persists [`KnowledgeNode`] entries as JSON files under
//! `.duumbi/knowledge/{type}/` directories. Supports loading all nodes,
//! querying by type or tag, and saving/updating individual nodes.

use std::fs;
use std::path::{Path, PathBuf};

use crate::knowledge::KnowledgeError;
use crate::knowledge::types::{
    DecisionRecord, KnowledgeNode, PatternRecord, SuccessRecord, TYPE_DECISION, TYPE_FAILURE,
    TYPE_PATTERN, TYPE_SUCCESS,
};

/// File-based knowledge store rooted at `.duumbi/knowledge/`.
pub struct KnowledgeStore {
    /// Root directory (e.g. `/project/.duumbi/knowledge/`).
    root: PathBuf,
}

impl KnowledgeStore {
    /// Creates a new store for the given workspace root.
    ///
    /// The knowledge directory is at `{workspace}/.duumbi/knowledge/`.
    /// Creates the directory tree if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub fn new(workspace: &Path) -> Result<Self, KnowledgeError> {
        let root = workspace.join(".duumbi").join("knowledge");
        fs::create_dir_all(root.join("success"))
            .map_err(|e| KnowledgeError::Io(format!("creating knowledge/success dir: {e}")))?;
        fs::create_dir_all(root.join("failure"))
            .map_err(|e| KnowledgeError::Io(format!("creating knowledge/failure dir: {e}")))?;
        fs::create_dir_all(root.join("decision"))
            .map_err(|e| KnowledgeError::Io(format!("creating knowledge/decision dir: {e}")))?;
        fs::create_dir_all(root.join("pattern"))
            .map_err(|e| KnowledgeError::Io(format!("creating knowledge/pattern dir: {e}")))?;
        Ok(Self { root })
    }

    /// Opens the knowledge store without creating any directories.
    #[must_use]
    pub fn open_existing(workspace: &Path) -> Self {
        let root = workspace.join(".duumbi").join("knowledge");
        Self { root }
    }

    /// Saves a knowledge node to disk. Overwrites if the same `@id` exists.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file write fails.
    pub fn save_node(&self, node: &KnowledgeNode) -> Result<(), KnowledgeError> {
        let (subdir, filename) = node_path_parts(node);
        let dir = self.root.join(subdir);
        let path = dir.join(filename);
        let json = serde_json::to_string_pretty(node)
            .map_err(|e| KnowledgeError::Serialize(e.to_string()))?;
        fs::write(&path, json)
            .map_err(|e| KnowledgeError::Io(format!("writing {}: {e}", path.display())))?;
        Ok(())
    }

    /// Loads all knowledge nodes from disk.
    ///
    /// Skips files that fail to deserialize (logs nothing — silent degradation).
    #[must_use]
    pub fn load_all(&self) -> Vec<KnowledgeNode> {
        let mut nodes = Vec::new();
        for subdir in &["success", "failure", "decision", "pattern"] {
            let dir = self.root.join(subdir);
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "json")
                        && let Ok(content) = fs::read_to_string(&path)
                        && let Ok(node) = serde_json::from_str::<KnowledgeNode>(&content)
                    {
                        nodes.push(node);
                    }
                }
            }
        }
        nodes
    }

    /// Loads all nodes of a specific `@type`.
    #[must_use]
    pub fn query_by_type(&self, node_type: &str) -> Vec<KnowledgeNode> {
        let subdir = match node_type {
            TYPE_SUCCESS => "success",
            TYPE_FAILURE => "failure",
            TYPE_DECISION => "decision",
            TYPE_PATTERN => "pattern",
            _ => return Vec::new(),
        };
        let dir = self.root.join(subdir);
        let mut nodes = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json")
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(node) = serde_json::from_str::<KnowledgeNode>(&content)
                {
                    nodes.push(node);
                }
            }
        }
        nodes
    }

    /// Queries nodes matching a tag (checks tags on Decision and Pattern records).
    #[must_use]
    pub fn query_by_tag(&self, tag: &str) -> Vec<KnowledgeNode> {
        self.load_all()
            .into_iter()
            .filter(|node| match node {
                KnowledgeNode::Decision(d) => d.tags.iter().any(|t| t == tag),
                KnowledgeNode::Pattern(p) => p.tags.iter().any(|t| t == tag),
                KnowledgeNode::Success(_) | KnowledgeNode::Failure(_) => false,
            })
            .collect()
    }

    /// Removes a node by its `@id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be removed.
    pub fn remove_node(&self, id: &str) -> Result<bool, KnowledgeError> {
        // Search all subdirectories
        for subdir in &["success", "decision", "pattern"] {
            let dir = self.root.join(subdir);
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "json")
                        && let Ok(content) = fs::read_to_string(&path)
                        && let Ok(node) = serde_json::from_str::<KnowledgeNode>(&content)
                        && node.id() == id
                    {
                        fs::remove_file(&path).map_err(|e| {
                            KnowledgeError::Io(format!("removing {}: {e}", path.display()))
                        })?;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Returns the total count of nodes per type.
    #[must_use]
    pub fn stats(&self) -> KnowledgeStats {
        KnowledgeStats {
            successes: count_json_files(&self.root.join("success")),
            decisions: count_json_files(&self.root.join("decision")),
            patterns: count_json_files(&self.root.join("pattern")),
        }
    }
}

/// Summary statistics for the knowledge store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeStats {
    /// Number of success records.
    pub successes: usize,
    /// Number of decision records.
    pub decisions: usize,
    /// Number of pattern records.
    pub patterns: usize,
}

impl KnowledgeStats {
    /// Total number of knowledge nodes.
    #[must_use]
    pub fn total(&self) -> usize {
        self.successes + self.decisions + self.patterns
    }
}

/// Counts `.json` files in a directory.
fn count_json_files(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .count()
        })
        .unwrap_or(0)
}

/// Derives the subdirectory and filename for a knowledge node.
fn node_path_parts(node: &KnowledgeNode) -> (&'static str, String) {
    let (subdir, id) = match node {
        KnowledgeNode::Success(r) => ("success", &r.id),
        KnowledgeNode::Failure(r) => ("failure", &r.id),
        KnowledgeNode::Decision(r) => ("decision", &r.id),
        KnowledgeNode::Pattern(r) => ("pattern", &r.id),
    };
    // Sanitize id for filename: replace non-alphanumeric with '_'
    let safe_name: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    (subdir, format!("{safe_name}.json"))
}

/// Loads a single success record by scanning the success directory.
///
/// Returns `None` if not found.
#[must_use]
#[allow(dead_code)] // Used by CLI commands
pub fn load_success_records(workspace: &Path) -> Vec<SuccessRecord> {
    let dir = workspace.join(".duumbi/knowledge/success");
    let mut records = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(KnowledgeNode::Success(r)) =
                    serde_json::from_str::<KnowledgeNode>(&content)
            {
                records.push(r);
            }
        }
    }
    records
}

/// Loads all decision records from the knowledge store.
#[must_use]
#[allow(dead_code)] // Used by CLI commands
pub fn load_decision_records(workspace: &Path) -> Vec<DecisionRecord> {
    let dir = workspace.join(".duumbi/knowledge/decision");
    let mut records = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(KnowledgeNode::Decision(r)) =
                    serde_json::from_str::<KnowledgeNode>(&content)
            {
                records.push(r);
            }
        }
    }
    records
}

/// Loads all pattern records from the knowledge store.
#[must_use]
#[allow(dead_code)] // Used by CLI commands
pub fn load_pattern_records(workspace: &Path) -> Vec<PatternRecord> {
    let dir = workspace.join(".duumbi/knowledge/pattern");
    let mut records = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(KnowledgeNode::Pattern(r)) =
                    serde_json::from_str::<KnowledgeNode>(&content)
            {
                records.push(r);
            }
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::types::SuccessRecord;
    use tempfile::TempDir;

    #[test]
    fn store_save_and_load_roundtrip() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");

        let record = SuccessRecord::new("add multiply", "AddFunction");
        let node = KnowledgeNode::Success(record.clone());
        store.save_node(&node).expect("save");

        let all = store.load_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id(), node.id());
    }

    #[test]
    fn store_query_by_type() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");

        store
            .save_node(&KnowledgeNode::Success(SuccessRecord::new("r1", "t1")))
            .expect("save");
        store
            .save_node(&KnowledgeNode::Decision(
                crate::knowledge::types::DecisionRecord::new("d1"),
            ))
            .expect("save");

        let successes = store.query_by_type(TYPE_SUCCESS);
        assert_eq!(successes.len(), 1);
        let decisions = store.query_by_type(TYPE_DECISION);
        assert_eq!(decisions.len(), 1);
        let patterns = store.query_by_type(TYPE_PATTERN);
        assert!(patterns.is_empty());
    }

    #[test]
    fn store_query_by_tag() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");

        let mut d = crate::knowledge::types::DecisionRecord::new("test decision");
        d.tags = vec!["architecture".to_string()];
        store.save_node(&KnowledgeNode::Decision(d)).expect("save");

        let found = store.query_by_tag("architecture");
        assert_eq!(found.len(), 1);
        let not_found = store.query_by_tag("nonexistent");
        assert!(not_found.is_empty());
    }

    #[test]
    fn store_remove_node() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");

        let node = KnowledgeNode::Success(SuccessRecord::new("r", "t"));
        let id = node.id().to_string();
        store.save_node(&node).expect("save");
        assert_eq!(store.load_all().len(), 1);

        let removed = store.remove_node(&id).expect("remove");
        assert!(removed);
        assert!(store.load_all().is_empty());
    }

    #[test]
    fn store_remove_nonexistent_returns_false() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");
        let removed = store.remove_node("nonexistent").expect("remove");
        assert!(!removed);
    }

    #[test]
    fn store_stats() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");

        store
            .save_node(&KnowledgeNode::Success(SuccessRecord::new("r1", "t1")))
            .expect("save");
        store
            .save_node(&KnowledgeNode::Success(SuccessRecord::new("r2", "t2")))
            .expect("save");
        store
            .save_node(&KnowledgeNode::Decision(
                crate::knowledge::types::DecisionRecord::new("d1"),
            ))
            .expect("save");

        let stats = store.stats();
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.decisions, 1);
        assert_eq!(stats.patterns, 0);
        assert_eq!(stats.total(), 3);
    }

    #[test]
    fn store_overwrites_existing_node() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");

        let mut r = SuccessRecord::new("original", "t");
        let id = r.id.clone();
        store
            .save_node(&KnowledgeNode::Success(r.clone()))
            .expect("save");

        r.request = "updated".to_string();
        store
            .save_node(&KnowledgeNode::Success(r))
            .expect("save overwrite");

        let all = store.load_all();
        // Same id means same file, so still 1
        assert_eq!(all.len(), 1);
        if let KnowledgeNode::Success(s) = &all[0] {
            assert_eq!(s.id, id);
            assert_eq!(s.request, "updated");
        } else {
            panic!("expected Success node");
        }
    }

    #[test]
    fn empty_store_returns_empty() {
        let tmp = TempDir::new().expect("temp dir");
        let store = KnowledgeStore::new(tmp.path()).expect("create store");
        assert!(store.load_all().is_empty());
        assert_eq!(store.stats().total(), 0);
    }
}
