//! Session persistence for REPL workflow state.
//!
//! Manages session state across CLI restarts. Each session tracks
//! conversation turns, LLM usage statistics, and provides atomic
//! save/load/archive operations.
//!
//! # Storage layout
//!
//! ```text
//! .duumbi/
//!   session/
//!     current.json           # Active session state
//!     history/
//!       {timestamp}.json     # Archived past sessions
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Errors from session operations.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// File I/O error.
    #[error("session I/O error: {0}")]
    Io(String),

    /// Serialization/deserialization error.
    #[error("session serialization error: {0}")]
    Serialize(String),
}

/// A single conversation turn persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistentTurn {
    /// The user's request text.
    pub request: String,

    /// Summary of what was done (e.g. describe_changes output).
    pub summary: String,

    /// When this turn occurred.
    pub timestamp: DateTime<Utc>,

    /// Classified task type.
    pub task_type: String,
}

/// LLM usage statistics tracked per session.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UsageStats {
    /// Total LLM API calls made.
    pub llm_calls: u32,

    /// Estimated total tokens sent.
    pub estimated_tokens_sent: usize,

    /// Estimated total tokens received.
    pub estimated_tokens_received: usize,

    /// Number of successful mutations.
    pub successes: u32,

    /// Number of failed mutations.
    pub failures: u32,

    /// Total retry attempts across all mutations.
    pub retries: u32,

    /// Per-provider call counts (e.g. {"anthropic": 5, "openai": 3}).
    #[serde(default)]
    pub provider_breakdown: std::collections::HashMap<String, u32>,
}

impl UsageStats {
    /// Records a successful LLM call.
    pub fn record_success(&mut self, provider: &str) {
        self.llm_calls += 1;
        self.successes += 1;
        *self
            .provider_breakdown
            .entry(provider.to_string())
            .or_insert(0) += 1;
    }

    /// Records a failed LLM call.
    pub fn record_failure(&mut self, provider: &str) {
        self.llm_calls += 1;
        self.failures += 1;
        *self
            .provider_breakdown
            .entry(provider.to_string())
            .or_insert(0) += 1;
    }

    /// Records a retry attempt.
    pub fn record_retry(&mut self) {
        self.retries += 1;
    }
}

/// Persistent session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Unique session identifier.
    pub session_id: String,

    /// When this session started.
    pub started_at: DateTime<Utc>,

    /// Conversation turns.
    pub turns: Vec<PersistentTurn>,

    /// Cumulative usage statistics.
    pub usage_stats: UsageStats,
}

impl SessionState {
    /// Creates a new empty session.
    #[must_use]
    fn new() -> Self {
        let now = Utc::now();
        Self {
            session_id: format!("session-{}", now.timestamp_millis()),
            started_at: now,
            turns: Vec::new(),
            usage_stats: UsageStats::default(),
        }
    }
}

/// Manages session persistence for a workspace.
pub struct SessionManager {
    /// Path to `.duumbi/session/`.
    session_dir: PathBuf,

    /// Current session state.
    state: SessionState,
}

impl SessionManager {
    /// Loads an existing session or creates a new one.
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation or file I/O fails.
    pub fn load_or_create(workspace: &Path) -> Result<Self, SessionError> {
        let session_dir = workspace.join(".duumbi/session");
        fs::create_dir_all(&session_dir)
            .map_err(|e| SessionError::Io(format!("creating session dir: {e}")))?;
        fs::create_dir_all(session_dir.join("history"))
            .map_err(|e| SessionError::Io(format!("creating session/history dir: {e}")))?;

        let current_path = session_dir.join("current.json");
        let state = if current_path.exists() {
            match fs::read_to_string(&current_path) {
                Ok(content) => {
                    serde_json::from_str::<SessionState>(&content).unwrap_or_else(|_| {
                        // Corrupted file — start fresh
                        SessionState::new()
                    })
                }
                Err(_) => SessionState::new(),
            }
        } else {
            SessionState::new()
        };

        Ok(Self { session_dir, state })
    }

    /// Saves the current session state atomically (tmp + rename).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file write fails.
    pub fn save(&self) -> Result<(), SessionError> {
        let current_path = self.session_dir.join("current.json");
        let tmp_path = self.session_dir.join("current.json.tmp");

        let json = serde_json::to_string_pretty(&self.state)
            .map_err(|e| SessionError::Serialize(e.to_string()))?;

        fs::write(&tmp_path, &json).map_err(|e| SessionError::Io(format!("writing tmp: {e}")))?;

        fs::rename(&tmp_path, &current_path)
            .map_err(|e| SessionError::Io(format!("renaming tmp to current: {e}")))?;

        Ok(())
    }

    /// Archives the current session and starts a new one.
    ///
    /// Moves `current.json` to `history/{timestamp}.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if save or rename fails.
    pub fn archive(&mut self) -> Result<(), SessionError> {
        if self.state.turns.is_empty() {
            // Nothing to archive
            self.state = SessionState::new();
            return Ok(());
        }

        // Save current state first
        self.save()?;

        let current_path = self.session_dir.join("current.json");
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let archive_path = self
            .session_dir
            .join("history")
            .join(format!("{timestamp}.json"));

        if current_path.exists() {
            fs::rename(&current_path, &archive_path)
                .map_err(|e| SessionError::Io(format!("archiving session: {e}")))?;
        }

        self.state = SessionState::new();
        Ok(())
    }

    /// Returns `true` if there is an existing session with turns.
    #[must_use]
    pub fn has_pending_session(&self) -> bool {
        !self.state.turns.is_empty()
    }

    /// Adds a conversation turn to the session.
    pub fn add_turn(&mut self, request: &str, summary: &str, task_type: &str) {
        self.state.turns.push(PersistentTurn {
            request: request.to_string(),
            summary: summary.to_string(),
            timestamp: Utc::now(),
            task_type: task_type.to_string(),
        });
    }

    /// Returns a reference to all turns in this session.
    #[must_use]
    pub fn turns(&self) -> &[PersistentTurn] {
        &self.state.turns
    }

    /// Returns a mutable reference to usage stats.
    pub fn usage_stats_mut(&mut self) -> &mut UsageStats {
        &mut self.state.usage_stats
    }

    /// Returns a reference to usage stats.
    #[must_use]
    pub fn usage_stats(&self) -> &UsageStats {
        &self.state.usage_stats
    }

    /// Returns the session ID.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.state.session_id
    }

    /// Returns when the session started.
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.state.started_at
    }

    /// Formats usage stats for display (e.g. `/status` command).
    #[must_use]
    pub fn format_stats(&self) -> String {
        let s = &self.state.usage_stats;
        let mut lines = vec![
            format!("Session: {}", self.state.session_id),
            format!(
                "Started: {}",
                self.state.started_at.format("%Y-%m-%d %H:%M:%S UTC")
            ),
            format!("Turns: {}", self.state.turns.len()),
            format!("LLM calls: {}", s.llm_calls),
            format!(
                "Successes: {} | Failures: {} | Retries: {}",
                s.successes, s.failures, s.retries
            ),
        ];
        if !s.provider_breakdown.is_empty() {
            let providers: Vec<String> = s
                .provider_breakdown
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
            lines.push(format!("Providers: {}", providers.join(", ")));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn session_create_and_save() {
        let tmp = TempDir::new().expect("temp dir");
        let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
        assert!(!mgr.has_pending_session());

        mgr.add_turn("add multiply", "added function", "AddFunction");
        assert!(mgr.has_pending_session());

        mgr.save().expect("save");

        // Reload
        let mgr2 = SessionManager::load_or_create(tmp.path()).expect("load");
        assert!(mgr2.has_pending_session());
        assert_eq!(mgr2.turns().len(), 1);
        assert_eq!(mgr2.turns()[0].request, "add multiply");
    }

    #[test]
    fn session_archive() {
        let tmp = TempDir::new().expect("temp dir");
        let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
        mgr.add_turn("test", "done", "AddFunction");
        mgr.archive().expect("archive");

        assert!(!mgr.has_pending_session());

        // Check archive file exists
        let history_dir = tmp.path().join(".duumbi/session/history");
        let entries: Vec<_> = fs::read_dir(&history_dir)
            .expect("read dir")
            .flatten()
            .collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn session_archive_empty_does_nothing() {
        let tmp = TempDir::new().expect("temp dir");
        let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
        mgr.archive().expect("archive empty");

        let history_dir = tmp.path().join(".duumbi/session/history");
        let entries: Vec<_> = fs::read_dir(&history_dir)
            .expect("read dir")
            .flatten()
            .collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn session_corrupted_recovery() {
        let tmp = TempDir::new().expect("temp dir");
        let session_dir = tmp.path().join(".duumbi/session");
        fs::create_dir_all(&session_dir).expect("mkdir");
        fs::write(session_dir.join("current.json"), "not valid json").expect("write corrupt");

        let mgr = SessionManager::load_or_create(tmp.path()).expect("recover");
        assert!(!mgr.has_pending_session());
    }

    #[test]
    fn usage_stats_tracking() {
        let mut stats = UsageStats::default();
        stats.record_success("anthropic");
        stats.record_success("anthropic");
        stats.record_failure("openai");
        stats.record_retry();

        assert_eq!(stats.llm_calls, 3);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.retries, 1);
        assert_eq!(stats.provider_breakdown.get("anthropic"), Some(&2));
        assert_eq!(stats.provider_breakdown.get("openai"), Some(&1));
    }

    #[test]
    fn format_stats_output() {
        let tmp = TempDir::new().expect("temp dir");
        let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
        mgr.add_turn("test", "done", "AddFunction");
        mgr.usage_stats_mut().record_success("anthropic");

        let output = mgr.format_stats();
        assert!(output.contains("Session:"));
        assert!(output.contains("Turns: 1"));
        assert!(output.contains("LLM calls: 1"));
        assert!(output.contains("anthropic: 1"));
    }

    #[test]
    fn persistent_turn_serialization() {
        let turn = PersistentTurn {
            request: "add func".to_string(),
            summary: "added".to_string(),
            timestamp: Utc::now(),
            task_type: "AddFunction".to_string(),
        };
        let json = serde_json::to_string(&turn).expect("serialize");
        let turn2: PersistentTurn = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(turn, turn2);
    }
}
