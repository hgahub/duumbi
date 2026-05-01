//! Centralized terminal color palette.
//!
//! Two coexisting APIs:
//!
//! - The top-level `owo-colors` helpers (`error`, `success`, `dim`, …) return
//!   colored `String` values for non-TUI command output.
//! - The [`tui`] submodule returns `ratatui::Style` values for the full-screen
//!   REPL, using the brand-aligned dark palette (rust + parchment + blue ink).
//!
//! Both honour `NO_COLOR`/`CLICOLOR` for graceful degradation on terminals
//! that do not support truecolor.

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
#[allow(dead_code)]
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
#[cfg(test)]
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
// TUI palette — ratatui::Style helpers for the full-screen REPL
// ---------------------------------------------------------------------------

/// Brand-aligned dark theme palette and semantic style helpers for the REPL.
///
/// The constants mirror the CSS variables used in the design mockup
/// (`duumbi-cli-brand-dark.html`). Every style helper returns a
/// [`ratatui::style::Style`] suitable for [`ratatui::text::Span::styled`].
///
/// Truecolor (24-bit) is preferred but the module gracefully falls back to
/// the 16 ANSI named colors when `NO_COLOR=1` is set or the terminal does
/// not advertise truecolor support via `COLORTERM`.
pub mod tui {
    use std::sync::OnceLock;

    use ratatui::style::{Color, Modifier, Style};

    // ---- Raw RGB palette (mirrors the mockup CSS vars) ---------------------

    /// Primary text colour (warm off-white).
    pub const PARCHMENT: Color = Color::Rgb(0xf5, 0xf2, 0xea);
    /// Global canvas background for the whole TUI (matches the mockup
    /// `--ink` canvas). Deliberately lightened slightly from pure black so
    /// the brand tone is perceptible on every modern terminal.
    pub const CANVAS_BG: Color = Color::Rgb(0x1a, 0x1a, 0x18);
    /// Slightly lifted surface used for inset cards.
    pub const PANEL_BG: Color = Color::Rgb(0x21, 0x21, 0x1e);
    /// Primary action / accent colour (rust).
    pub const RUST: Color = Color::Rgb(0xd0, 0x7a, 0x47);
    /// Softer rust used for badges and helper text.
    pub const RUST_SOFT: Color = Color::Rgb(0xc9, 0x6a, 0x3e);
    /// Emphasis blue used for the version badge and pulsing dot.
    pub const BLUE_INK: Color = Color::Rgb(0x7b, 0xa8, 0xe0);
    /// Slightly cooler blue used for the focused intent slug.
    pub const BLUE_MID: Color = Color::Rgb(0x6b, 0x9e, 0xd9);
    /// Dim hairline colour (parchment at ~7% alpha against the canvas).
    pub const HAIRLINE_DIM: Color = Color::Rgb(0x3a, 0x39, 0x35);
    /// Brighter hairline colour (parchment at ~22% alpha).
    pub const HAIRLINE: Color = Color::Rgb(0x6e, 0x6c, 0x65);
    /// Reserved for future JSON-LD key highlighting.
    pub const TOKEN_KEY: Color = Color::Rgb(0x7d, 0xcf, 0xff);
    /// Reserved for future JSON-LD string highlighting.
    pub const TOKEN_STRING: Color = Color::Rgb(0x9e, 0xce, 0x6a);
    /// Reserved for future JSON-LD URL highlighting.
    pub const TOKEN_URL: Color = Color::Rgb(0xe0, 0xaf, 0x68);
    /// Reserved for future JSON-LD type highlighting.
    pub const TOKEN_TYPE: Color = Color::Rgb(0xbb, 0x9a, 0xf7);

    /// Snapshot of the palette decision made once at startup.
    #[derive(Clone, Copy)]
    struct Palette {
        truecolor: bool,
    }

    static PALETTE: OnceLock<Palette> = OnceLock::new();

    fn palette() -> Palette {
        *PALETTE.get_or_init(|| Palette {
            truecolor: detect_truecolor(),
        })
    }

    fn detect_truecolor() -> bool {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        match std::env::var("COLORTERM") {
            Ok(v) => {
                let lower = v.to_lowercase();
                lower.contains("truecolor") || lower.contains("24bit")
            }
            Err(_) => {
                // Common modern terminals (kitty, wezterm, alacritty, vscode)
                // export `TERM_PROGRAM` even without COLORTERM.
                std::env::var("TERM_PROGRAM").is_ok()
            }
        }
    }

    /// Returns `true` when truecolor (24-bit) styling is enabled.
    #[must_use]
    pub fn truecolor_supported() -> bool {
        palette().truecolor
    }

    /// Maps a truecolor palette value to its closest 16-color fallback.
    pub(super) fn fallback(c: Color) -> Color {
        match c {
            RUST | RUST_SOFT => Color::Red,
            BLUE_INK | BLUE_MID | TOKEN_KEY => Color::Cyan,
            PARCHMENT => Color::White,
            HAIRLINE | HAIRLINE_DIM => Color::Gray,
            TOKEN_STRING => Color::Green,
            TOKEN_URL => Color::Yellow,
            TOKEN_TYPE => Color::Magenta,
            other => other,
        }
    }

    /// Returns the colour as-is when truecolor is supported, otherwise the
    /// 16-colour fallback.
    #[must_use]
    pub fn col(c: Color) -> Color {
        if truecolor_supported() {
            c
        } else {
            fallback(c)
        }
    }

    // ---- Semantic style helpers --------------------------------------------

    /// Bold parchment for the brand word ("duumbi").
    #[must_use]
    pub fn brand_word() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .add_modifier(Modifier::BOLD)
    }

    /// Blue-ink badge for the version number.
    #[must_use]
    pub fn version_badge() -> Style {
        Style::default().fg(col(BLUE_INK))
    }

    /// Soft rust used for short helper text (e.g. "type /help").
    #[must_use]
    pub fn helper() -> Style {
        Style::default().fg(col(RUST_SOFT))
    }

    /// Inset keycap (parchment foreground on a hairline-dim background).
    #[must_use]
    pub fn keycap() -> Style {
        Style::default().fg(col(PARCHMENT)).bg(col(HAIRLINE_DIM))
    }

    /// Base canvas style used across the whole TUI.
    #[must_use]
    pub fn canvas() -> Style {
        Style::default().bg(col(CANVAS_BG))
    }

    /// Mode pill background (parchment text on blue-ink).
    #[must_use]
    #[allow(dead_code)] // used in Phase B mode strip
    pub fn pill_blue() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(BLUE_INK))
            .add_modifier(Modifier::BOLD)
    }

    /// Activity / empty-state pill background (parchment text on rust).
    #[must_use]
    #[allow(dead_code)] // used in Phase C activity button + empty-state card
    pub fn pill_rust() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(RUST))
            .add_modifier(Modifier::BOLD)
    }

    /// Bold rust chevron (`›`) used as the prompt prefix.
    #[must_use]
    pub fn chevron() -> Style {
        Style::default().fg(col(RUST)).add_modifier(Modifier::BOLD)
    }

    /// Rust focus border around the prompt input well.
    #[must_use]
    #[allow(dead_code)] // used in Phase D focus ring
    pub fn focus_border() -> Style {
        Style::default().fg(col(RUST))
    }

    /// Hairline separator colour (mid-tone).
    #[must_use]
    pub fn hairline() -> Style {
        Style::default().fg(col(HAIRLINE))
    }

    /// Subtle panel surface used for inset cards and overlays.
    #[must_use]
    pub fn panel_surface() -> Style {
        Style::default().bg(col(PANEL_BG))
    }

    /// Border colour for inset panels and cards.
    #[must_use]
    pub fn panel_border() -> Style {
        Style::default().fg(col(HAIRLINE_DIM))
    }

    /// Rust accent used for the left edge of the empty-state card.
    #[must_use]
    pub fn panel_accent() -> Style {
        Style::default()
            .fg(col(RUST))
            .bg(col(PANEL_BG))
            .add_modifier(Modifier::BOLD)
    }

    /// Outline pill used for mode badges and empty-state tags.
    #[must_use]
    pub fn pill_outline() -> Style {
        Style::default()
            .fg(col(RUST))
            .bg(col(Color::Rgb(0x22, 0x1d, 0x19)))
            .add_modifier(Modifier::BOLD)
    }

    /// Uppercase status-dock labels ("TIME", "WORKSPACE", …).
    #[must_use]
    pub fn label_caps() -> Style {
        Style::default()
            .fg(col(HAIRLINE))
            .add_modifier(Modifier::DIM)
    }

    /// Inline uppercase labels for the single-row footer.
    #[must_use]
    pub fn label_caps_inline() -> Style {
        // Use the brighter hairline without DIM so the labels remain
        // perceptible against the warm canvas bg (DIM pushed the contrast
        // ratio below WCAG AA on the #1a1a18 surface).
        Style::default().fg(col(HAIRLINE))
    }

    /// Bold rust for status-dock workspace name.
    #[must_use]
    pub fn workspace_value() -> Style {
        Style::default().fg(col(RUST)).add_modifier(Modifier::BOLD)
    }

    /// Parchment for status-dock value rows.
    #[must_use]
    pub fn dock_value() -> Style {
        Style::default().fg(col(PARCHMENT))
    }

    /// Muted value style for dense footer rows.
    #[must_use]
    pub fn dock_value_muted() -> Style {
        Style::default().fg(col(Color::Rgb(0xd9, 0xd4, 0xc8)))
    }

    /// Bold blue-mid for the focused intent slug.
    #[must_use]
    pub fn intent_slug() -> Style {
        Style::default()
            .fg(col(BLUE_MID))
            .add_modifier(Modifier::BOLD)
    }

    /// Placeholder text inside the prompt well.
    #[must_use]
    pub fn placeholder() -> Style {
        Style::default()
            .fg(col(HAIRLINE))
            .add_modifier(Modifier::DIM)
    }

    // ---- Output buffer styles (replaces the inline OutputStyle match) -----

    /// Style for normal output lines.
    #[must_use]
    pub fn out_normal() -> Style {
        Style::default().fg(col(PARCHMENT))
    }

    /// Style for error output lines.
    #[must_use]
    pub fn out_error() -> Style {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    }

    /// Style for success output lines.
    #[must_use]
    pub fn out_success() -> Style {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for dim/secondary output lines.
    #[must_use]
    pub fn out_dim() -> Style {
        Style::default().fg(col(HAIRLINE))
    }

    /// Style for AI streaming / assistant output.
    #[must_use]
    pub fn out_ai() -> Style {
        Style::default().fg(col(BLUE_INK))
    }

    /// Style for the command column of `/help` output.
    #[must_use]
    pub fn out_help_cmd() -> Style {
        Style::default().fg(col(RUST)).add_modifier(Modifier::BOLD)
    }

    /// Style for the description column of `/help` output.
    #[must_use]
    pub fn out_help_desc() -> Style {
        Style::default().fg(col(PARCHMENT))
    }

    // ---- Slash-menu styles -------------------------------------------------

    /// Highlighted slash-menu row.
    #[must_use]
    pub fn slash_selected() -> Style {
        Style::default().fg(col(RUST)).add_modifier(Modifier::BOLD)
    }

    /// Background for the highlighted slash-menu row.
    #[must_use]
    pub fn slash_selected_row() -> Style {
        Style::default().bg(col(Color::Rgb(0x2d, 0x25, 0x1f)))
    }

    /// Highlight for the matched slash-command prefix.
    #[must_use]
    pub fn slash_match() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(Color::Rgb(0x5a, 0x34, 0x24)))
            .add_modifier(Modifier::BOLD)
    }

    /// Slash command text.
    #[must_use]
    pub fn slash_command() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .add_modifier(Modifier::BOLD)
    }

    /// Slash-menu group label.
    #[must_use]
    pub fn slash_group() -> Style {
        Style::default().fg(col(RUST)).add_modifier(Modifier::BOLD)
    }

    /// User-submitted text inside a conversation block.
    #[must_use]
    pub fn conversation_user_text() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(PANEL_BG))
            .add_modifier(Modifier::BOLD)
    }

    /// Timestamp and action text inside a conversation user block.
    #[must_use]
    pub fn conversation_user_meta() -> Style {
        Style::default().fg(col(HAIRLINE)).bg(col(PANEL_BG))
    }

    /// User block surface while selected by mouse.
    #[must_use]
    pub fn conversation_user_selected_surface() -> Style {
        Style::default().bg(col(Color::Rgb(0x2a, 0x28, 0x23)))
    }

    /// User text inside the selected conversation block.
    #[must_use]
    pub fn conversation_user_selected_text() -> Style {
        conversation_user_selected_surface()
            .fg(col(PARCHMENT))
            .add_modifier(Modifier::BOLD)
    }

    /// Timestamp and muted controls inside the selected conversation block.
    #[must_use]
    pub fn conversation_user_selected_meta() -> Style {
        conversation_user_selected_surface().fg(col(Color::Rgb(0xb0, 0xaa, 0x9d)))
    }

    /// Active action trigger inside the selected conversation block.
    #[must_use]
    pub fn conversation_user_action() -> Style {
        conversation_user_selected_surface()
            .fg(col(RUST))
            .add_modifier(Modifier::BOLD)
    }

    /// Rust accent used for selected conversation blocks.
    #[must_use]
    pub fn conversation_user_selected_accent() -> Style {
        conversation_user_selected_surface()
            .fg(col(RUST))
            .add_modifier(Modifier::BOLD)
    }

    /// Popup row used for conversation block actions.
    #[must_use]
    pub fn conversation_action_menu() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(Color::Rgb(0x24, 0x23, 0x20)))
    }

    /// Selected popup row used for conversation block actions.
    #[must_use]
    pub fn conversation_action_menu_selected() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(Color::Rgb(0x3a, 0x2b, 0x22)))
            .add_modifier(Modifier::BOLD)
    }

    /// Popup border used for conversation block actions.
    #[must_use]
    pub fn conversation_action_menu_border() -> Style {
        Style::default().fg(col(RUST))
    }

    /// Highlight used for app-managed mouse text selection.
    #[must_use]
    pub fn conversation_text_selection() -> Style {
        Style::default()
            .fg(col(PARCHMENT))
            .bg(col(Color::Rgb(0x36, 0x45, 0x58)))
            .add_modifier(Modifier::BOLD)
    }

    /// Unselected slash-menu row.
    #[must_use]
    #[allow(dead_code)] // currently uses out_dim() — kept for future per-row styling
    pub fn slash_normal() -> Style {
        Style::default()
            .fg(col(HAIRLINE))
            .add_modifier(Modifier::DIM)
    }

    // ---- Mode strip --------------------------------------------------------

    /// Outlined blue pill for the active mode indicator.
    #[must_use]
    pub fn mode_pill() -> Style {
        Style::default()
            .fg(col(BLUE_INK))
            .bg(col(Color::Rgb(0x1d, 0x23, 0x2c)))
            .add_modifier(Modifier::BOLD)
    }

    /// Dim helper text on the mode strip ("Shift+Tab swap").
    #[must_use]
    pub fn mode_hint() -> Style {
        Style::default()
            .fg(col(HAIRLINE))
            .add_modifier(Modifier::DIM)
    }

    /// Pulsing dot beside the active mode label.
    #[must_use]
    #[allow(dead_code)] // used in Phase D pulsing animation
    pub fn mode_dot() -> Style {
        Style::default().fg(col(BLUE_INK))
    }
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

    #[test]
    fn tui_helpers_compile_and_return_styles() {
        // Smoke test: every public helper returns a usable Style.
        let _ = tui::brand_word();
        let _ = tui::version_badge();
        let _ = tui::helper();
        let _ = tui::keycap();
        let _ = tui::canvas();
        let _ = tui::pill_blue();
        let _ = tui::pill_rust();
        let _ = tui::chevron();
        let _ = tui::focus_border();
        let _ = tui::hairline();
        let _ = tui::panel_surface();
        let _ = tui::panel_border();
        let _ = tui::panel_accent();
        let _ = tui::pill_outline();
        let _ = tui::label_caps();
        let _ = tui::label_caps_inline();
        let _ = tui::workspace_value();
        let _ = tui::dock_value();
        let _ = tui::dock_value_muted();
        let _ = tui::intent_slug();
        let _ = tui::placeholder();
        let _ = tui::out_normal();
        let _ = tui::out_error();
        let _ = tui::out_success();
        let _ = tui::out_dim();
        let _ = tui::out_ai();
        let _ = tui::out_help_cmd();
        let _ = tui::out_help_desc();
        let _ = tui::slash_selected();
        let _ = tui::slash_selected_row();
        let _ = tui::slash_match();
        let _ = tui::slash_command();
        let _ = tui::slash_group();
        let _ = tui::conversation_user_text();
        let _ = tui::conversation_user_meta();
        let _ = tui::conversation_text_selection();
        let _ = tui::slash_normal();
        let _ = tui::mode_pill();
        let _ = tui::mode_pill();
        let _ = tui::mode_hint();
        let _ = tui::mode_dot();
    }

    #[test]
    fn fallback_maps_palette_to_named_colors() {
        use ratatui::style::Color;
        // These conversions are deterministic regardless of env state.
        assert_eq!(tui::fallback(tui::RUST), Color::Red);
        assert_eq!(tui::fallback(tui::RUST_SOFT), Color::Red);
        assert_eq!(tui::fallback(tui::BLUE_INK), Color::Cyan);
        assert_eq!(tui::fallback(tui::BLUE_MID), Color::Cyan);
        assert_eq!(tui::fallback(tui::PARCHMENT), Color::White);
        assert_eq!(tui::fallback(tui::HAIRLINE), Color::Gray);
        assert_eq!(tui::fallback(tui::HAIRLINE_DIM), Color::Gray);
        assert_eq!(tui::fallback(tui::TOKEN_STRING), Color::Green);
        assert_eq!(tui::fallback(tui::TOKEN_URL), Color::Yellow);
    }

    #[test]
    fn col_returns_palette_color_when_truecolor() {
        // We can't easily mock the OnceLock, but we can at least exercise
        // the function and verify it returns *some* colour without panicking.
        let _ = tui::col(tui::RUST);
        let _ = tui::col(tui::PARCHMENT);
    }
}
