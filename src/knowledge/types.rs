//! Knowledge node types for the DUUMBI learning system.
//!
//! Defines three knowledge record types: [`SuccessRecord`], [`DecisionRecord`],
//! and [`PatternRecord`]. Each serializes to/from JSON with a `@type` tag
//! following JSON-LD conventions.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Schema version for knowledge node format. Bump when the format changes.
pub const SCHEMA_VERSION: u32 = 1;

/// Monotonic counter to guarantee unique IDs even within the same millisecond.
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// JSON-LD `@type` value for success records.
pub const TYPE_SUCCESS: &str = "duumbi:Success";
/// JSON-LD `@type` value for decision records.
pub const TYPE_DECISION: &str = "duumbi:Decision";
/// JSON-LD `@type` value for pattern records.
pub const TYPE_PATTERN: &str = "duumbi:Pattern";

/// A record of a successful LLM mutation, used for few-shot learning.
///
/// Logged after every successful `mutate_streaming()` call. Fields capture
/// enough context to score relevance against future requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SuccessRecord {
    /// JSON-LD type tag.
    /// Populated by default function; skipped when serialized inside [`KnowledgeNode`] enum
    /// (the enum's `serde(tag)` handles `@type`).
    #[serde(rename = "@type", default = "default_success_type", skip_serializing)]
    pub node_type: String,

    /// Unique identifier (format: `duumbi:success/<timestamp>`).
    #[serde(rename = "@id")]
    pub id: String,

    /// The original user request that led to this mutation.
    pub request: String,

    /// Classified task type (e.g. "AddFunction", "ModifyMain").
    pub task_type: String,

    /// Number of patch operations applied.
    pub ops_count: usize,

    /// Op types used (e.g. ["add_function", "replace_block"]).
    #[serde(default)]
    pub op_kinds: Vec<String>,

    /// Error codes encountered before success (retry context).
    #[serde(default)]
    pub error_codes: Vec<String>,

    /// Number of retries before success (0 = first attempt succeeded).
    pub retry_count: u32,

    /// Target module name (e.g. "main", "calculator/ops").
    #[serde(default)]
    pub module: String,

    /// Function names created or modified.
    #[serde(default)]
    pub functions: Vec<String>,

    /// When this mutation succeeded.
    pub timestamp: DateTime<Utc>,

    /// Schema version for forward compatibility.
    pub schema_version: u32,
}

fn default_success_type() -> String {
    TYPE_SUCCESS.to_string()
}

impl SuccessRecord {
    /// Creates a new success record with the current timestamp.
    #[must_use]
    pub fn new(request: impl Into<String>, task_type: impl Into<String>) -> Self {
        let now = Utc::now();
        let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!("duumbi:success/{}-{seq}", now.timestamp_millis());
        Self {
            node_type: TYPE_SUCCESS.to_string(),
            id,
            request: request.into(),
            task_type: task_type.into(),
            ops_count: 0,
            op_kinds: Vec::new(),
            error_codes: Vec::new(),
            retry_count: 0,
            module: String::new(),
            functions: Vec::new(),
            timestamp: now,
            schema_version: SCHEMA_VERSION,
        }
    }
}

/// A record of an architectural or design decision.
///
/// Stored when the user or system makes a significant choice that should
/// inform future mutations (e.g. "use separate modules for math ops").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    /// JSON-LD type tag.
    /// See [`SuccessRecord::node_type`].
    #[serde(rename = "@type", default = "default_decision_type", skip_serializing)]
    pub node_type: String,

    /// Unique identifier (format: `duumbi:decision/<timestamp>`).
    #[serde(rename = "@id")]
    pub id: String,

    /// The decision statement.
    pub decision: String,

    /// Why this decision was made.
    #[serde(default)]
    pub rationale: String,

    /// Alternatives that were considered.
    #[serde(default)]
    pub alternatives: Vec<String>,

    /// Tags for categorization (e.g. ["architecture", "modules"]).
    #[serde(default)]
    pub tags: Vec<String>,

    /// When this decision was recorded.
    pub timestamp: DateTime<Utc>,

    /// Schema version for forward compatibility.
    pub schema_version: u32,
}

fn default_decision_type() -> String {
    TYPE_DECISION.to_string()
}

impl DecisionRecord {
    /// Creates a new decision record with the current timestamp.
    #[must_use]
    pub fn new(decision: impl Into<String>) -> Self {
        let now = Utc::now();
        let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!("duumbi:decision/{}-{seq}", now.timestamp_millis());
        Self {
            node_type: TYPE_DECISION.to_string(),
            id,
            decision: decision.into(),
            rationale: String::new(),
            alternatives: Vec::new(),
            tags: Vec::new(),
            timestamp: now,
            schema_version: SCHEMA_VERSION,
        }
    }
}

/// A record of a recurring pattern observed across mutations.
///
/// Patterns emerge from multiple successes and help the system anticipate
/// common task shapes (e.g. "add function + wire into main").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatternRecord {
    /// JSON-LD type tag.
    /// See [`SuccessRecord::node_type`].
    #[serde(rename = "@type", default = "default_pattern_type", skip_serializing)]
    pub node_type: String,

    /// Unique identifier (format: `duumbi:pattern/<name>`).
    #[serde(rename = "@id")]
    pub id: String,

    /// Pattern name (e.g. "add-function-and-wire").
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// A concrete example of this pattern in action.
    #[serde(default)]
    pub example: String,

    /// How many times this pattern has been observed.
    pub frequency: u32,

    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,

    /// When this pattern was last observed.
    pub timestamp: DateTime<Utc>,

    /// Schema version for forward compatibility.
    pub schema_version: u32,
}

fn default_pattern_type() -> String {
    TYPE_PATTERN.to_string()
}

impl PatternRecord {
    /// Creates a new pattern record with frequency 1.
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!("duumbi:pattern/{name}-{seq}");
        Self {
            node_type: TYPE_PATTERN.to_string(),
            id,
            name,
            description: description.into(),
            example: String::new(),
            frequency: 1,
            tags: Vec::new(),
            timestamp: Utc::now(),
            schema_version: SCHEMA_VERSION,
        }
    }
}

/// Enum wrapping all knowledge node types for polymorphic storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "@type")]
pub enum KnowledgeNode {
    /// A successful mutation record.
    #[serde(rename = "duumbi:Success")]
    Success(SuccessRecord),
    /// A design decision record.
    #[serde(rename = "duumbi:Decision")]
    Decision(DecisionRecord),
    /// A recurring pattern record.
    #[serde(rename = "duumbi:Pattern")]
    Pattern(PatternRecord),
}

impl KnowledgeNode {
    /// Returns the `@id` of this node.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            KnowledgeNode::Success(r) => &r.id,
            KnowledgeNode::Decision(r) => &r.id,
            KnowledgeNode::Pattern(r) => &r.id,
        }
    }

    /// Returns the `@type` string of this node.
    #[must_use]
    pub fn node_type(&self) -> &str {
        match self {
            KnowledgeNode::Success(_) => TYPE_SUCCESS,
            KnowledgeNode::Decision(_) => TYPE_DECISION,
            KnowledgeNode::Pattern(_) => TYPE_PATTERN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_record_new_sets_defaults() {
        let r = SuccessRecord::new("add multiply function", "AddFunction");
        assert_eq!(r.node_type, TYPE_SUCCESS);
        assert!(r.id.starts_with("duumbi:success/"));
        assert_eq!(r.request, "add multiply function");
        assert_eq!(r.task_type, "AddFunction");
        assert_eq!(r.ops_count, 0);
        assert_eq!(r.retry_count, 0);
        assert_eq!(r.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn success_record_json_roundtrip() {
        let r = SuccessRecord::new("test", "CreateModule");
        let json = serde_json::to_string(&r).expect("must serialize");
        let r2: SuccessRecord = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(r.request, r2.request);
        assert_eq!(r.node_type, r2.node_type);
    }

    #[test]
    fn decision_record_new_sets_defaults() {
        let d = DecisionRecord::new("use separate modules for math");
        assert_eq!(d.node_type, TYPE_DECISION);
        assert!(d.id.starts_with("duumbi:decision/"));
        assert_eq!(d.decision, "use separate modules for math");
        assert!(d.rationale.is_empty());
    }

    #[test]
    fn pattern_record_new_sets_defaults() {
        let p = PatternRecord::new("add-and-wire", "Add function then wire into main");
        assert_eq!(p.node_type, TYPE_PATTERN);
        assert!(p.id.starts_with("duumbi:pattern/add-and-wire-"));
        assert_eq!(p.frequency, 1);
    }

    #[test]
    fn knowledge_node_enum_roundtrip() {
        let node = KnowledgeNode::Success(SuccessRecord::new("test", "AddFunction"));
        let json = serde_json::to_string(&node).expect("must serialize");
        let node2: KnowledgeNode = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(node.id(), node2.id());
        assert_eq!(node.node_type(), TYPE_SUCCESS);
    }

    #[test]
    fn knowledge_node_id_and_type() {
        let s = KnowledgeNode::Success(SuccessRecord::new("r", "t"));
        assert!(s.id().starts_with("duumbi:success/"));
        assert_eq!(s.node_type(), TYPE_SUCCESS);

        let d = KnowledgeNode::Decision(DecisionRecord::new("d"));
        assert!(d.id().starts_with("duumbi:decision/"));
        assert_eq!(d.node_type(), TYPE_DECISION);

        let p = KnowledgeNode::Pattern(PatternRecord::new("n", "d"));
        assert!(p.id().starts_with("duumbi:pattern/n-"));
        assert_eq!(p.node_type(), TYPE_PATTERN);
    }
}
