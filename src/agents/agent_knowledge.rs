//! Agent-level knowledge store for strategies and failure patterns.
//!
//! Persists successful strategies and observed failure patterns to
//! `.duumbi/knowledge/strategies/` and `.duumbi/knowledge/failure-patterns/`
//! respectively.  Records are never deleted — deprecated entries are marked
//! with `deprecated = true` so the history remains auditable.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Storage paths
// ---------------------------------------------------------------------------

const STRATEGIES_DIR: &str = ".duumbi/knowledge/strategies";
const FAILURE_PATTERNS_DIR: &str = ".duumbi/knowledge/failure-patterns";

/// Deprecation threshold: fail rate must exceed this fraction.
const DEPRECATION_FAIL_RATE: f64 = 0.70;
/// Minimum number of attempts before deprecation is considered.
const DEPRECATION_MIN_ATTEMPTS: u32 = 10;

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

/// A successful strategy for a particular agent template.
///
/// Strategies are accumulated over many runs and rated by their success/fail
/// counts.  An entry whose fail rate exceeds 70% (with at least 10 attempts)
/// is marked `deprecated = true` by [`AgentKnowledgeStore::prune_deprecated`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Strategy {
    /// JSON-LD type tag.
    #[serde(rename = "@type")]
    pub node_type: String,
    /// Unique identifier.
    #[serde(rename = "@id")]
    pub id: String,
    /// The agent template this strategy belongs to.
    pub template_id: String,
    /// Human-readable description.
    pub description: String,
    /// Pattern in the task description that triggers this strategy.
    pub trigger_pattern: String,
    /// Concrete approach to apply.
    pub approach: String,
    /// Number of times this strategy led to success.
    pub success_count: u32,
    /// Number of times this strategy led to failure.
    pub fail_count: u32,
    /// Whether this strategy has been retired due to high failure rate.
    pub deprecated: bool,
    /// When this strategy was first recorded.
    pub timestamp: DateTime<Utc>,
}

impl Strategy {
    /// Create a new strategy with zero counts and `deprecated = false`.
    #[must_use]
    pub fn new(template_id: &str, description: &str, trigger: &str, approach: &str) -> Self {
        let id = format!("duumbi:strategy/{}", uuid_slug(template_id, description));
        Strategy {
            node_type: "duumbi:Strategy".to_string(),
            id,
            template_id: template_id.to_string(),
            description: description.to_string(),
            trigger_pattern: trigger.to_string(),
            approach: approach.to_string(),
            success_count: 0,
            fail_count: 0,
            deprecated: false,
            timestamp: Utc::now(),
        }
    }

    /// Returns `true` if this strategy should be deprecated.
    ///
    /// Criterion: at least [`DEPRECATION_MIN_ATTEMPTS`] total attempts and
    /// fail rate exceeds [`DEPRECATION_FAIL_RATE`].
    #[must_use]
    pub fn should_deprecate(&self) -> bool {
        let total = self.success_count + self.fail_count;
        if total < DEPRECATION_MIN_ATTEMPTS {
            return false;
        }
        (self.fail_count as f64 / total as f64) > DEPRECATION_FAIL_RATE
    }
}

// ---------------------------------------------------------------------------
// FailurePattern
// ---------------------------------------------------------------------------

/// A recorded failure pattern for learning.
///
/// When a coder agent produces patches that consistently fail validation with
/// the same error codes, that pattern is recorded here so future prompts can
/// include mitigations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailurePattern {
    /// JSON-LD type tag.
    #[serde(rename = "@type")]
    pub node_type: String,
    /// Unique identifier.
    #[serde(rename = "@id")]
    pub id: String,
    /// The agent template this pattern applies to.
    pub template_id: String,
    /// Short description of the failure pattern.
    pub pattern: String,
    /// DUUMBI error codes associated with this pattern (e.g. `["E001", "E003"]`).
    pub error_codes: Vec<String>,
    /// Suggested mitigation to inject into the prompt.
    pub mitigation: String,
    /// Times the mitigation successfully avoided this failure.
    pub success_count: u32,
    /// Times the mitigation did not prevent this failure.
    pub fail_count: u32,
    /// Whether this pattern has been retired.
    pub deprecated: bool,
    /// When this pattern was first recorded.
    pub timestamp: DateTime<Utc>,
}

impl FailurePattern {
    /// Create a new failure pattern with zero counts and `deprecated = false`.
    #[must_use]
    pub fn new(
        template_id: &str,
        pattern: &str,
        error_codes: Vec<String>,
        mitigation: &str,
    ) -> Self {
        let id = format!("duumbi:failure-pattern/{}", uuid_slug(template_id, pattern));
        FailurePattern {
            node_type: "duumbi:FailurePattern".to_string(),
            id,
            template_id: template_id.to_string(),
            pattern: pattern.to_string(),
            error_codes,
            mitigation: mitigation.to_string(),
            success_count: 0,
            fail_count: 0,
            deprecated: false,
            timestamp: Utc::now(),
        }
    }

    /// Returns `true` if this failure pattern should be deprecated.
    ///
    /// Same criterion as [`Strategy::should_deprecate`]: at least 10 attempts
    /// and fail rate exceeds 70 %.
    #[must_use]
    pub fn should_deprecate(&self) -> bool {
        let total = self.success_count + self.fail_count;
        if total < DEPRECATION_MIN_ATTEMPTS {
            return false;
        }
        (self.fail_count as f64 / total as f64) > DEPRECATION_FAIL_RATE
    }
}

// ---------------------------------------------------------------------------
// AgentKnowledgeStore
// ---------------------------------------------------------------------------

/// Agent knowledge store for strategies and failure patterns.
///
/// All methods are static — the store itself carries no state; files on disk
/// are the source of truth.
pub struct AgentKnowledgeStore;

impl AgentKnowledgeStore {
    /// Load all strategies from the workspace.
    ///
    /// Files that fail to parse are silently skipped.
    #[must_use]
    pub fn load_strategies(workspace: &Path) -> Vec<Strategy> {
        load_json_dir(workspace.join(STRATEGIES_DIR))
    }

    /// Load all failure patterns from the workspace.
    ///
    /// Files that fail to parse are silently skipped.
    #[must_use]
    pub fn load_failure_patterns(workspace: &Path) -> Vec<FailurePattern> {
        load_json_dir(workspace.join(FAILURE_PATTERNS_DIR))
    }

    /// Persist a strategy to disk.
    ///
    /// The file name is derived from the strategy `@id`.  An existing file
    /// with the same name is overwritten.
    pub fn save_strategy(workspace: &Path, strategy: &Strategy) -> std::io::Result<()> {
        let dir = workspace.join(STRATEGIES_DIR);
        std::fs::create_dir_all(&dir)?;
        let name = id_to_filename(&strategy.id);
        let path = dir.join(format!("{name}.json"));
        let json = serde_json::to_string_pretty(strategy).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Persist a failure pattern to disk.
    ///
    /// The file name is derived from the pattern `@id`.  An existing file
    /// with the same name is overwritten.
    pub fn save_failure_pattern(workspace: &Path, pattern: &FailurePattern) -> std::io::Result<()> {
        let dir = workspace.join(FAILURE_PATTERNS_DIR);
        std::fs::create_dir_all(&dir)?;
        let name = id_to_filename(&pattern.id);
        let path = dir.join(format!("{name}.json"));
        let json = serde_json::to_string_pretty(pattern).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Mark strategies and failure patterns with >70 % failure rate as deprecated.
    ///
    /// Reads all files, sets `deprecated = true` on qualifying records, and
    /// writes them back.  Never deletes files.  Returns the number of records
    /// that were newly deprecated.
    pub fn prune_deprecated(workspace: &Path) -> std::io::Result<usize> {
        let mut count = 0;
        count += prune_dir::<Strategy>(workspace.join(STRATEGIES_DIR))?;
        count += prune_dir::<FailurePattern>(workspace.join(FAILURE_PATTERNS_DIR))?;
        Ok(count)
    }

    /// Return non-deprecated strategies that belong to the given template.
    #[must_use]
    pub fn relevant_strategies<'a>(
        strategies: &'a [Strategy],
        template_id: &str,
    ) -> Vec<&'a Strategy> {
        strategies
            .iter()
            .filter(|s| s.template_id == template_id && !s.deprecated)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a JSON-LD `@id` string to a safe file name component.
fn id_to_filename(id: &str) -> String {
    id.replace(['/', ':', '@'], "_")
        .trim_matches('_')
        .to_string()
}

/// Build a short slug from two strings (used for generated `@id` values).
fn uuid_slug(a: &str, b: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    a.hash(&mut h);
    b.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Load all JSON files from a directory, deserialising each as `T`.
fn load_json_dir<T: for<'de> Deserialize<'de>>(dir: std::path::PathBuf) -> Vec<T> {
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| {
            let content = std::fs::read_to_string(e.path()).ok()?;
            serde_json::from_str(&content).ok()
        })
        .collect()
}

/// Mark qualifying records in a directory as deprecated and write back.
fn prune_dir<T>(dir: std::path::PathBuf) -> std::io::Result<usize>
where
    T: for<'de> Deserialize<'de> + Serialize + ShouldDeprecate,
{
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(0);
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut record: T = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !record.is_deprecated() && record.check_should_deprecate() {
            record.set_deprecated(true);
            let json = serde_json::to_string_pretty(&record).map_err(std::io::Error::other)?;
            std::fs::write(&path, json)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Sealed helper trait used only by [`prune_dir`].
trait ShouldDeprecate {
    fn check_should_deprecate(&self) -> bool;
    fn is_deprecated(&self) -> bool;
    fn set_deprecated(&mut self, v: bool);
}

impl ShouldDeprecate for Strategy {
    fn check_should_deprecate(&self) -> bool {
        self.should_deprecate()
    }
    fn is_deprecated(&self) -> bool {
        self.deprecated
    }
    fn set_deprecated(&mut self, v: bool) {
        self.deprecated = v;
    }
}

impl ShouldDeprecate for FailurePattern {
    fn check_should_deprecate(&self) -> bool {
        self.should_deprecate()
    }
    fn is_deprecated(&self) -> bool {
        self.deprecated
    }
    fn set_deprecated(&mut self, v: bool) {
        self.deprecated = v;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Strategy
    // -----------------------------------------------------------------------

    #[test]
    fn strategy_new_defaults() {
        let s = Strategy::new(
            "duumbi:template/coder",
            "emit minimal patch",
            "create",
            "use add_op",
        );
        assert_eq!(s.node_type, "duumbi:Strategy");
        assert_eq!(s.template_id, "duumbi:template/coder");
        assert!(!s.deprecated);
        assert_eq!(s.success_count, 0);
        assert_eq!(s.fail_count, 0);
    }

    #[test]
    fn strategy_should_not_deprecate_below_min_attempts() {
        let mut s = Strategy::new("t", "d", "p", "a");
        s.success_count = 0;
        s.fail_count = 9; // 9 < 10 minimum
        assert!(!s.should_deprecate());
    }

    #[test]
    fn strategy_should_not_deprecate_low_fail_rate() {
        let mut s = Strategy::new("t", "d", "p", "a");
        s.success_count = 5;
        s.fail_count = 5; // 50% — below threshold
        assert!(!s.should_deprecate());
    }

    #[test]
    fn strategy_should_deprecate_high_fail_rate() {
        let mut s = Strategy::new("t", "d", "p", "a");
        s.success_count = 2;
        s.fail_count = 8; // 80% > 70%, total=10
        assert!(s.should_deprecate());
    }

    #[test]
    fn strategy_should_deprecate_exactly_at_boundary() {
        let mut s = Strategy::new("t", "d", "p", "a");
        s.success_count = 3;
        s.fail_count = 7; // 70% — NOT greater than 70%
        assert!(!s.should_deprecate());
    }

    #[test]
    fn strategy_serialization_roundtrip() {
        let s = Strategy::new("tmpl", "desc", "trigger", "approach");
        let json = serde_json::to_string(&s).expect("ser");
        let back: Strategy = serde_json::from_str(&json).expect("deser");
        assert_eq!(s.id, back.id);
        assert_eq!(s.template_id, back.template_id);
        assert_eq!(s.description, back.description);
        assert_eq!(s.approach, back.approach);
        assert_eq!(s.deprecated, back.deprecated);
    }

    // -----------------------------------------------------------------------
    // FailurePattern
    // -----------------------------------------------------------------------

    #[test]
    fn failure_pattern_new_defaults() {
        let p = FailurePattern::new(
            "duumbi:template/coder",
            "missing return op",
            vec!["E003".to_string()],
            "always end block with return",
        );
        assert_eq!(p.node_type, "duumbi:FailurePattern");
        assert!(!p.deprecated);
        assert_eq!(p.error_codes, vec!["E003"]);
    }

    #[test]
    fn failure_pattern_should_deprecate() {
        let mut p = FailurePattern::new("t", "pat", vec![], "mit");
        p.success_count = 1;
        p.fail_count = 9; // 90%, total=10
        assert!(p.should_deprecate());
    }

    #[test]
    fn failure_pattern_should_not_deprecate_below_min() {
        let mut p = FailurePattern::new("t", "pat", vec![], "mit");
        p.success_count = 0;
        p.fail_count = 8; // 8 < 10
        assert!(!p.should_deprecate());
    }

    #[test]
    fn failure_pattern_serialization_roundtrip() {
        let p = FailurePattern::new("tmpl", "pat", vec!["E001".to_string()], "mitigation");
        let json = serde_json::to_string(&p).expect("ser");
        let back: FailurePattern = serde_json::from_str(&json).expect("deser");
        assert_eq!(p.id, back.id);
        assert_eq!(p.error_codes, back.error_codes);
        assert_eq!(p.mitigation, back.mitigation);
    }

    // -----------------------------------------------------------------------
    // AgentKnowledgeStore — save/load roundtrips
    // -----------------------------------------------------------------------

    #[test]
    fn save_and_load_strategy() {
        let tmp = TempDir::new().expect("tmp");
        let mut s = Strategy::new("tmpl", "desc", "trigger", "approach");
        s.success_count = 3;
        AgentKnowledgeStore::save_strategy(tmp.path(), &s).expect("save");
        let loaded = AgentKnowledgeStore::load_strategies(tmp.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, s.id);
        assert_eq!(loaded[0].success_count, 3);
    }

    #[test]
    fn save_and_load_failure_pattern() {
        let tmp = TempDir::new().expect("tmp");
        let p = FailurePattern::new("tmpl", "pat", vec!["E002".to_string()], "mit");
        AgentKnowledgeStore::save_failure_pattern(tmp.path(), &p).expect("save");
        let loaded = AgentKnowledgeStore::load_failure_patterns(tmp.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].error_codes, vec!["E002"]);
    }

    #[test]
    fn load_empty_when_dir_absent() {
        let tmp = TempDir::new().expect("tmp");
        let strategies = AgentKnowledgeStore::load_strategies(tmp.path());
        let patterns = AgentKnowledgeStore::load_failure_patterns(tmp.path());
        assert!(strategies.is_empty());
        assert!(patterns.is_empty());
    }

    // -----------------------------------------------------------------------
    // prune_deprecated
    // -----------------------------------------------------------------------

    #[test]
    fn prune_deprecated_marks_high_fail_rate() {
        let tmp = TempDir::new().expect("tmp");
        let mut s = Strategy::new("tmpl", "bad strategy", "pat", "approach");
        s.success_count = 1;
        s.fail_count = 9; // 90% — above threshold
        AgentKnowledgeStore::save_strategy(tmp.path(), &s).expect("save");

        let count = AgentKnowledgeStore::prune_deprecated(tmp.path()).expect("prune");
        assert_eq!(count, 1);

        let loaded = AgentKnowledgeStore::load_strategies(tmp.path());
        assert!(loaded[0].deprecated, "should be deprecated after prune");
    }

    #[test]
    fn prune_deprecated_does_not_mark_low_fail_rate() {
        let tmp = TempDir::new().expect("tmp");
        let mut s = Strategy::new("tmpl", "good strategy", "pat", "approach");
        s.success_count = 8;
        s.fail_count = 2; // 20%
        AgentKnowledgeStore::save_strategy(tmp.path(), &s).expect("save");

        let count = AgentKnowledgeStore::prune_deprecated(tmp.path()).expect("prune");
        assert_eq!(count, 0);

        let loaded = AgentKnowledgeStore::load_strategies(tmp.path());
        assert!(!loaded[0].deprecated);
    }

    #[test]
    fn prune_deprecated_does_not_re_mark_already_deprecated() {
        let tmp = TempDir::new().expect("tmp");
        let mut s = Strategy::new("tmpl", "old bad strategy", "pat", "approach");
        s.success_count = 1;
        s.fail_count = 9;
        s.deprecated = true; // already deprecated
        AgentKnowledgeStore::save_strategy(tmp.path(), &s).expect("save");

        let count = AgentKnowledgeStore::prune_deprecated(tmp.path()).expect("prune");
        assert_eq!(
            count, 0,
            "already-deprecated record must not be counted again"
        );
    }

    #[test]
    fn prune_deprecated_no_dir_returns_zero() {
        let tmp = TempDir::new().expect("tmp");
        let count = AgentKnowledgeStore::prune_deprecated(tmp.path()).expect("prune");
        assert_eq!(count, 0);
    }

    // -----------------------------------------------------------------------
    // relevant_strategies
    // -----------------------------------------------------------------------

    #[test]
    fn relevant_strategies_filters_by_template_and_not_deprecated() {
        let mut s1 = Strategy::new("tmpl/coder", "s1", "p", "a");
        s1.success_count = 5;
        let mut s2 = Strategy::new("tmpl/coder", "s2", "p", "a");
        s2.deprecated = true;
        let s3 = Strategy::new("tmpl/planner", "s3", "p", "a");

        let all = vec![s1.clone(), s2, s3];
        let relevant = AgentKnowledgeStore::relevant_strategies(&all, "tmpl/coder");
        assert_eq!(relevant.len(), 1);
        assert_eq!(relevant[0].id, s1.id);
    }

    #[test]
    fn relevant_strategies_empty_when_no_match() {
        let s = Strategy::new("tmpl/planner", "s", "p", "a");
        let all = [s];
        let relevant = AgentKnowledgeStore::relevant_strategies(&all, "tmpl/coder");
        assert!(relevant.is_empty());
    }
}
