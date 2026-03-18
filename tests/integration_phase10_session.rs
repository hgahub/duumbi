//! Integration tests for Phase 10 Track E: Session & Workflow Persistence.
//!
//! Tests session save/load roundtrip, resume detection, archive, stats,
//! and corrupted file recovery.

use std::fs;

use duumbi::session::{SessionManager, UsageStats};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Save/load roundtrip
// ---------------------------------------------------------------------------

#[test]
fn session_save_load_roundtrip() {
    let tmp = TempDir::new().expect("temp dir");

    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
    mgr.add_turn("add multiply", "added function", "AddFunction");
    mgr.add_turn("fix error", "fixed E001", "FixError");
    mgr.save().expect("save");

    let mgr2 = SessionManager::load_or_create(tmp.path()).expect("load");
    assert_eq!(mgr2.turns().len(), 2);
    assert_eq!(mgr2.turns()[0].request, "add multiply");
    assert_eq!(mgr2.turns()[1].request, "fix error");
    assert_eq!(mgr2.turns()[0].task_type, "AddFunction");
}

// ---------------------------------------------------------------------------
// Resume detection
// ---------------------------------------------------------------------------

#[test]
fn session_resume_detection() {
    let tmp = TempDir::new().expect("temp dir");

    let mgr = SessionManager::load_or_create(tmp.path()).expect("create");
    assert!(!mgr.has_pending_session());

    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
    mgr.add_turn("test", "done", "AddFunction");
    mgr.save().expect("save");

    let mgr2 = SessionManager::load_or_create(tmp.path()).expect("load");
    assert!(mgr2.has_pending_session());
}

// ---------------------------------------------------------------------------
// Archive
// ---------------------------------------------------------------------------

#[test]
fn session_archive_moves_file() {
    let tmp = TempDir::new().expect("temp dir");

    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
    mgr.add_turn("request 1", "did stuff", "AddFunction");
    mgr.add_turn("request 2", "more stuff", "ModifyMain");
    mgr.archive().expect("archive");

    // After archive: no pending session
    assert!(!mgr.has_pending_session());

    // Archive file exists
    let history_dir = tmp.path().join(".duumbi/session/history");
    let entries: Vec<_> = fs::read_dir(&history_dir)
        .expect("read dir")
        .flatten()
        .collect();
    assert_eq!(entries.len(), 1);

    // current.json should not exist
    let current = tmp.path().join(".duumbi/session/current.json");
    assert!(!current.exists());
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

// ---------------------------------------------------------------------------
// Usage stats accumulation
// ---------------------------------------------------------------------------

#[test]
fn usage_stats_accumulation() {
    let mut stats = UsageStats::default();

    stats.record_success("anthropic");
    stats.record_success("anthropic");
    stats.record_failure("openai");
    stats.record_retry();
    stats.record_retry();

    assert_eq!(stats.llm_calls, 3);
    assert_eq!(stats.successes, 2);
    assert_eq!(stats.failures, 1);
    assert_eq!(stats.retries, 2);
    assert_eq!(stats.provider_breakdown["anthropic"], 2);
    assert_eq!(stats.provider_breakdown["openai"], 1);
}

#[test]
fn usage_stats_persist_with_session() {
    let tmp = TempDir::new().expect("temp dir");

    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");
    mgr.add_turn("test", "done", "AddFunction");
    mgr.usage_stats_mut().record_success("anthropic");
    mgr.usage_stats_mut().record_success("anthropic");
    mgr.usage_stats_mut().record_failure("openai");
    mgr.save().expect("save");

    let mgr2 = SessionManager::load_or_create(tmp.path()).expect("load");
    assert_eq!(mgr2.usage_stats().llm_calls, 3);
    assert_eq!(mgr2.usage_stats().successes, 2);
    assert_eq!(mgr2.usage_stats().failures, 1);
}

// ---------------------------------------------------------------------------
// Corrupted recovery
// ---------------------------------------------------------------------------

#[test]
fn session_corrupted_json_recovery() {
    let tmp = TempDir::new().expect("temp dir");
    let session_dir = tmp.path().join(".duumbi/session");
    fs::create_dir_all(&session_dir).expect("mkdir");
    fs::write(session_dir.join("current.json"), "{{{{not json").expect("write");

    let mgr = SessionManager::load_or_create(tmp.path()).expect("recover");
    assert!(!mgr.has_pending_session());
    assert_eq!(mgr.turns().len(), 0);
}

#[test]
fn session_truncated_file_recovery() {
    let tmp = TempDir::new().expect("temp dir");
    let session_dir = tmp.path().join(".duumbi/session");
    fs::create_dir_all(&session_dir).expect("mkdir");
    // Write partial JSON
    fs::write(
        session_dir.join("current.json"),
        r#"{"session_id":"test","started_at":"#,
    )
    .expect("write");

    let mgr = SessionManager::load_or_create(tmp.path()).expect("recover");
    assert!(!mgr.has_pending_session());
}

// ---------------------------------------------------------------------------
// Format stats output
// ---------------------------------------------------------------------------

#[test]
fn format_stats_includes_key_fields() {
    let tmp = TempDir::new().expect("temp dir");
    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");

    mgr.add_turn("request 1", "done 1", "AddFunction");
    mgr.add_turn("request 2", "done 2", "FixError");
    mgr.usage_stats_mut().record_success("anthropic");

    let output = mgr.format_stats();
    assert!(output.contains("Session:"));
    assert!(output.contains("Turns: 2"));
    assert!(output.contains("LLM calls: 1"));
    assert!(output.contains("Successes: 1"));
    assert!(output.contains("anthropic: 1"));
}

// ---------------------------------------------------------------------------
// Multiple archives
// ---------------------------------------------------------------------------

#[test]
fn session_multiple_archives() {
    let tmp = TempDir::new().expect("temp dir");
    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("create");

    // First session
    mgr.add_turn("session 1", "done", "AddFunction");
    mgr.archive().expect("archive 1");

    // Second session
    mgr.add_turn("session 2", "done", "FixError");
    mgr.archive().expect("archive 2");

    let history_dir = tmp.path().join(".duumbi/session/history");
    let entries: Vec<_> = fs::read_dir(&history_dir)
        .expect("read dir")
        .flatten()
        .collect();
    assert_eq!(entries.len(), 2);
}

// ---------------------------------------------------------------------------
// Session ID uniqueness
// ---------------------------------------------------------------------------

#[test]
fn session_ids_are_unique() {
    let tmp = TempDir::new().expect("temp dir");

    let mgr1 = SessionManager::load_or_create(tmp.path()).expect("create 1");
    let id1 = mgr1.session_id().to_string();

    // Archive and create new
    let mut mgr = SessionManager::load_or_create(tmp.path()).expect("load");
    mgr.add_turn("test", "done", "AddFunction");
    mgr.archive().expect("archive");

    // The new session should have a different ID
    let id2 = mgr.session_id().to_string();
    assert_ne!(id1, id2, "Session IDs should be unique");
}
