//! Centralized terminal color palette.
//!
//! All CLI color usage goes through these helper functions so that:
//! - Colors are consistent across the entire CLI
//! - `NO_COLOR` / `CLICOLOR` are respected via `owo-colors` + `anstream`
//! - Non-TTY output is automatically stripped of ANSI codes

use owo_colors::OwoColorize;

// ---------------------------------------------------------------------------
// Semantic color helpers — return colored `String` values
// ---------------------------------------------------------------------------

/// Renders text in red (errors, failures).
#[must_use]
pub fn error(text: &str) -> String {
    format!("{}", text.red().bold())
}

/// Renders text in green (success, pass).
#[must_use]
pub fn success(text: &str) -> String {
    format!("{}", text.green().bold())
}

/// Renders text in yellow (warnings).
#[must_use]
pub fn warning(text: &str) -> String {
    format!("{}", text.yellow())
}

/// Renders text in cyan (informational highlights).
#[must_use]
pub fn info(text: &str) -> String {
    format!("{}", text.cyan())
}

/// Renders text in dim/grey (secondary information).
#[must_use]
pub fn dim(text: &str) -> String {
    format!("{}", text.dimmed())
}

/// Renders text in bold (emphasis).
#[must_use]
pub fn bold(text: &str) -> String {
    format!("{}", text.bold())
}

/// Renders a slash command name in bold cyan.
#[must_use]
pub fn command(text: &str) -> String {
    format!("{}", text.cyan().bold())
}

/// Renders an error code (e.g. "E001") in bold red.
#[must_use]
pub fn error_code(text: &str) -> String {
    format!("{}", text.red().bold())
}

/// Renders a node ID in blue.
#[must_use]
pub fn node_id(text: &str) -> String {
    format!("{}", text.blue())
}

/// Renders a check mark in green.
#[must_use]
pub fn check_mark() -> String {
    success("\u{2713}")
}

/// Renders a cross mark in red.
#[must_use]
pub fn cross_mark() -> String {
    error("\u{2717}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_functions_return_non_empty() {
        assert!(!error("test").is_empty());
        assert!(!success("test").is_empty());
        assert!(!warning("test").is_empty());
        assert!(!info("test").is_empty());
        assert!(!dim("test").is_empty());
        assert!(!bold("test").is_empty());
        assert!(!command("/build").is_empty());
        assert!(!error_code("E001").is_empty());
        assert!(!node_id("duumbi:main").is_empty());
        assert!(!check_mark().is_empty());
        assert!(!cross_mark().is_empty());
    }

    #[test]
    fn check_and_cross_contain_unicode() {
        // The raw strings contain the Unicode characters (possibly wrapped in ANSI)
        let cm = check_mark();
        let xm = cross_mark();
        // On non-TTY the ANSI codes might be stripped, but the char itself is present
        assert!(cm.contains('\u{2713}') || cm.contains("✓"));
        assert!(xm.contains('\u{2717}') || xm.contains("✗"));
    }
}
