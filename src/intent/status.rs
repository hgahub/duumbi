//! Intent status display and history archival.
//!
//! Provides `duumbi intent status [name]` functionality and archives
//! completed intents to `.duumbi/intents/history/`.

use std::fs;
use std::path::Path;

use super::spec::{ExecutionMeta, IntentSpec, IntentStatus};
use super::{IntentError, history_dir, intent_path, list_intents, load_intent, save_intent};

// ---------------------------------------------------------------------------
// Status display
// ---------------------------------------------------------------------------

/// Prints a list summary of all active intents (no name given).
pub fn print_status_list(workspace: &Path) -> Result<(), IntentError> {
    let slugs = list_intents(workspace)?;
    if slugs.is_empty() {
        eprintln!("No active intents.");
        return Ok(());
    }
    eprintln!("{:<20} {:<12} Intent", "Name", "Status");
    eprintln!("{}", "-".repeat(60));
    for slug in &slugs {
        match load_intent(workspace, slug) {
            Ok(spec) => {
                let intent_preview: String = spec.intent.chars().take(40).collect();
                eprintln!(
                    "{:<20} {:<12} {}",
                    slug,
                    spec.status.to_string(),
                    intent_preview
                );
            }
            Err(e) => eprintln!("{:<20} error        {e}", slug),
        }
    }
    Ok(())
}

/// Prints detailed status for a single named intent.
pub fn print_status_detail(workspace: &Path, slug: &str) -> Result<(), IntentError> {
    let spec = load_intent(workspace, slug)?;

    eprintln!();
    eprintln!("Intent: {slug} ({})", spec.status);
    if let Some(ref created) = spec.created_at {
        eprintln!("Created: {created}");
    }
    eprintln!();
    eprintln!("Description: {}", spec.intent);

    if !spec.acceptance_criteria.is_empty() {
        eprintln!();
        eprintln!("Acceptance Criteria:");
        for (i, c) in spec.acceptance_criteria.iter().enumerate() {
            eprintln!("  {}. {c}", i + 1);
        }
    }

    if let Some(ref exec) = spec.execution {
        eprintln!();
        eprintln!("Execution Result:");
        eprintln!("  Completed at: {}", exec.completed_at);
        eprintln!("  Tasks completed: {}", exec.tasks_completed);
        eprintln!("  Tests: {}/{} passed", exec.tests_passed, exec.tests_total);
    }

    eprintln!();
    Ok(())
}

// ---------------------------------------------------------------------------
// History archival
// ---------------------------------------------------------------------------

/// Archives a completed intent to `.duumbi/intents/history/<slug>.yaml`.
///
/// Appends `execution` metadata to the archived copy and removes the active file.
pub fn archive_intent(
    workspace: &Path,
    slug: &str,
    meta: ExecutionMeta,
) -> Result<(), IntentError> {
    let mut spec: IntentSpec = load_intent(workspace, slug)?;
    spec.status = IntentStatus::Completed;
    spec.execution = Some(meta);

    let hist_dir = history_dir(workspace);
    fs::create_dir_all(&hist_dir).map_err(|source| IntentError::Io {
        path: hist_dir.display().to_string(),
        source,
    })?;

    // Write to history
    let hist_path = hist_dir.join(format!("{slug}.yaml"));
    let contents = serde_yaml::to_string(&spec).map_err(IntentError::Serialize)?;
    fs::write(&hist_path, contents).map_err(|source| IntentError::Io {
        path: hist_path.display().to_string(),
        source,
    })?;

    // Remove active file
    let active_path = intent_path(workspace, slug);
    if active_path.exists() {
        fs::remove_file(&active_path).map_err(|source| IntentError::Io {
            path: active_path.display().to_string(),
            source,
        })?;
    }

    Ok(())
}

/// Marks an intent as `Failed` and saves it in place (not archived).
pub fn mark_failed(workspace: &Path, slug: &str) -> Result<(), IntentError> {
    let mut spec = load_intent(workspace, slug)?;
    spec.status = IntentStatus::Failed;
    save_intent(workspace, slug, &spec)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::spec::{IntentModules, IntentStatus};

    fn minimal_spec(intent: &str) -> IntentSpec {
        IntentSpec {
            intent: intent.to_string(),
            version: 1,
            status: IntentStatus::Pending,
            acceptance_criteria: vec![],
            modules: IntentModules::default(),
            test_cases: vec![],
            dependencies: vec![],
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            execution: None,
        }
    }

    #[test]
    fn archive_moves_to_history() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        save_intent(tmp.path(), "my-intent", &minimal_spec("Build something")).expect("save");

        let meta = ExecutionMeta {
            completed_at: "2026-01-02T00:00:00Z".to_string(),
            tasks_completed: 2,
            tests_passed: 3,
            tests_total: 3,
        };
        archive_intent(tmp.path(), "my-intent", meta).expect("archive");

        // Active file must be gone
        assert!(!intent_path(tmp.path(), "my-intent").exists());
        // History file must exist
        assert!(history_dir(tmp.path()).join("my-intent.yaml").exists());
    }

    #[test]
    fn archive_sets_completed_status() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        save_intent(tmp.path(), "intent-a", &minimal_spec("Test")).expect("save");

        let meta = ExecutionMeta {
            completed_at: "2026-01-01T00:00:00Z".to_string(),
            tasks_completed: 1,
            tests_passed: 1,
            tests_total: 1,
        };
        archive_intent(tmp.path(), "intent-a", meta).expect("archive");

        let hist_path = history_dir(tmp.path()).join("intent-a.yaml");
        let contents = fs::read_to_string(&hist_path).expect("read");
        let spec: IntentSpec = serde_yaml::from_str(&contents).expect("parse");
        assert_eq!(spec.status, IntentStatus::Completed);
        assert!(spec.execution.is_some());
    }

    #[test]
    fn mark_failed_updates_status() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        save_intent(tmp.path(), "failing", &minimal_spec("Fail me")).expect("save");
        mark_failed(tmp.path(), "failing").expect("mark failed");

        let spec = load_intent(tmp.path(), "failing").expect("load");
        assert_eq!(spec.status, IntentStatus::Failed);
    }
}
