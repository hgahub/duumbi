//! Progress indicator helpers.
//!
//! Wraps [`indicatif`] spinners and progress bars with consistent styling
//! for the duumbi CLI. All spinners use `finish_and_clear()` before any
//! streaming text output to avoid interleaving.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

/// Creates a styled spinner with the given message.
///
/// The spinner auto-ticks every 80ms. Call `finish_and_clear()` or
/// `finish_with_message()` when done.
#[must_use]
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("invariant: valid spinner template")
            .tick_strings(&[
                "\u{280b}", "\u{2819}", "\u{2838}", "\u{2830}", "\u{2824}", "\u{2826}", "\u{2807}",
                "\u{280f}", "\u{2713}",
            ]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Creates a spinner and immediately finishes it with the message visible.
/// Useful for one-shot status updates that should leave a trace.
#[allow(dead_code)]
pub fn spinner_done(message: &str) {
    let pb = spinner(message);
    pb.finish_with_message(message.to_string());
}

// ---------------------------------------------------------------------------
// Progress bar
// ---------------------------------------------------------------------------

/// Creates a styled progress bar for N items (e.g. tasks).
#[must_use]
#[allow(dead_code)] // API for future intent multi-progress
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("{msg} [{bar:30.cyan/dim}] {pos}/{len}")
            .expect("invariant: valid progress bar template")
            .progress_chars("\u{2588}\u{2592}\u{2591}"),
    );
    pb.set_message(message.to_string());
    pb
}

// ---------------------------------------------------------------------------
// Multi-progress
// ---------------------------------------------------------------------------

/// Creates a [`MultiProgress`] container for grouping multiple progress bars.
#[must_use]
#[allow(dead_code)] // API for future intent multi-progress
pub fn multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Adds a task-level spinner to a multi-progress container.
#[must_use]
#[allow(dead_code)] // API for future intent multi-progress
pub fn add_task_spinner(mp: &MultiProgress, message: &str) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("invariant: valid task spinner template")
            .tick_strings(&[
                "\u{280b}", "\u{2819}", "\u{2838}", "\u{2830}", "\u{2824}", "\u{2826}", "\u{2807}",
                "\u{280f}", "\u{2713}",
            ]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_creates_and_finishes() {
        let sp = spinner("Testing...");
        sp.finish_and_clear();
    }

    #[test]
    fn progress_bar_tracks_items() {
        let pb = progress_bar(5, "Tasks");
        pb.inc(1);
        assert_eq!(pb.position(), 1);
        pb.finish_and_clear();
    }

    #[test]
    fn multi_progress_adds_tasks() {
        let mp = multi_progress();
        let sp = add_task_spinner(&mp, "Task 1");
        sp.finish_and_clear();
    }
}
