//! Intent review command — display and list intent specs.
//!
//! Used by `duumbi intent review [name]` and the `/intent review` REPL command.

use std::path::Path;

use super::spec::{IntentSpec, IntentStatus};
use super::{IntentError, list_intents, load_intent};

/// Prints a formatted summary of all active intents to stderr.
///
/// If no intents exist, prints a "no intents" message.
pub fn print_intent_list(workspace: &Path) -> Result<(), IntentError> {
    let slugs = list_intents(workspace)?;
    if slugs.is_empty() {
        eprintln!("No intents found. Use `duumbi intent create \"<description>\"` to create one.");
        return Ok(());
    }
    eprintln!("Active intents:");
    for slug in &slugs {
        match load_intent(workspace, slug) {
            Ok(spec) => {
                let date = spec.created_at.as_deref().unwrap_or("unknown date");
                let tc_count = spec.test_cases.len();
                eprintln!(
                    "  {slug} — {} [{}] ({} test{})",
                    spec.intent,
                    spec.status,
                    tc_count,
                    if tc_count == 1 { "" } else { "s" },
                );
                let _ = date; // suppress unused if not displayed
            }
            Err(e) => eprintln!("  {slug} — (error loading: {e})"),
        }
    }
    Ok(())
}

/// Prints a detailed view of a single intent spec to stderr.
pub fn print_intent_detail(workspace: &Path, slug: &str) -> Result<(), IntentError> {
    let spec = load_intent(workspace, slug)?;
    print_spec_detail(slug, &spec);
    Ok(())
}

/// Renders a detailed intent spec to stderr.
pub fn print_spec_detail(slug: &str, spec: &IntentSpec) {
    let status_icon = match spec.status {
        IntentStatus::Pending => "○",
        IntentStatus::InProgress => "◉",
        IntentStatus::Completed => "✓",
        IntentStatus::Failed => "✗",
    };

    eprintln!();
    eprintln!("Intent: {} ({})", slug, spec.intent);
    eprintln!("Status: {status_icon} {}", spec.status);
    if let Some(ref created) = spec.created_at {
        eprintln!("Created: {created}");
    }

    if !spec.acceptance_criteria.is_empty() {
        eprintln!();
        eprintln!("Acceptance Criteria:");
        for (i, criterion) in spec.acceptance_criteria.iter().enumerate() {
            eprintln!("  {}. {criterion}", i + 1);
        }
    }

    if !spec.modules.create.is_empty() || !spec.modules.modify.is_empty() {
        eprintln!();
        eprintln!("Modules:");
        for m in &spec.modules.create {
            eprintln!("  + {m} (create)");
        }
        for m in &spec.modules.modify {
            eprintln!("  ~ {m} (modify)");
        }
    }

    if !spec.test_cases.is_empty() {
        eprintln!();
        eprintln!("Test Cases:");
        for tc in &spec.test_cases {
            let args_str = tc
                .args
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "  {} — {}({}) → {}",
                tc.name, tc.function, args_str, tc.expected_return
            );
        }
    }

    if let Some(ref exec) = spec.execution {
        eprintln!();
        eprintln!("Execution:");
        eprintln!("  Completed: {}", exec.completed_at);
        eprintln!("  Tasks: {}/{}", exec.tasks_completed, exec.tasks_completed);
        eprintln!("  Tests: {}/{}", exec.tests_passed, exec.tests_total);
    }
    eprintln!();
}

/// Formats a detailed intent spec into a log buffer (REPL-safe).
///
/// Same content as [`print_spec_detail`] but appends to `log` instead of
/// writing to stderr.
pub fn format_spec_detail(slug: &str, spec: &IntentSpec, log: &mut Vec<String>) {
    let status_icon = match spec.status {
        IntentStatus::Pending => "○",
        IntentStatus::InProgress => "◉",
        IntentStatus::Completed => "✓",
        IntentStatus::Failed => "✗",
    };

    log.push(format!("Intent: {} ({})", slug, spec.intent));
    log.push(format!("Status: {status_icon} {}", spec.status));
    if let Some(ref created) = spec.created_at {
        log.push(format!("Created: {created}"));
    }

    if !spec.acceptance_criteria.is_empty() {
        log.push("Acceptance Criteria:".to_string());
        for (i, criterion) in spec.acceptance_criteria.iter().enumerate() {
            log.push(format!("  {}. {criterion}", i + 1));
        }
    }

    if !spec.modules.create.is_empty() || !spec.modules.modify.is_empty() {
        log.push("Modules:".to_string());
        for m in &spec.modules.create {
            log.push(format!("  + {m} (create)"));
        }
        for m in &spec.modules.modify {
            log.push(format!("  ~ {m} (modify)"));
        }
    }

    if !spec.test_cases.is_empty() {
        log.push("Test Cases:".to_string());
        for tc in &spec.test_cases {
            let args_str = tc
                .args
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            log.push(format!(
                "  {} — {}({}) → {}",
                tc.name, tc.function, args_str, tc.expected_return
            ));
        }
    }
}

/// Opens the intent YAML in `$EDITOR` (falls back to `vi`) and re-validates.
///
/// Returns `Ok(())` if the file was saved with valid YAML, or an error if
/// the editor fails or the resulting YAML is invalid.
pub fn edit_intent(workspace: &Path, slug: &str) -> Result<(), IntentError> {
    let path = super::intent_path(workspace, slug);

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|source| IntentError::Io {
            path: path.display().to_string(),
            source,
        })?;

    if !status.success() {
        eprintln!("Editor exited with {status}");
    }

    // Re-validate by loading
    load_intent(workspace, slug)?;
    eprintln!("Intent '{slug}' saved and validated.");
    Ok(())
}
