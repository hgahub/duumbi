//! Theme management for the Studio UI.
//!
//! Provides a theme toggle component and CSS class management.
//! Theme preference is persisted via localStorage on the client side.

use leptos::prelude::*;

use crate::state::{StudioState, Theme};

/// Theme toggle button component.
///
/// Renders a sun/moon icon button that toggles between dark and light themes.
/// Also responds to `Ctrl+Shift+T` keyboard shortcut.
#[component]
pub fn ThemeToggle() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let toggle = move |_| {
        state.theme.update(|t| *t = t.toggle());
    };

    let label = move || match state.theme.get() {
        Theme::Dark => "Light mode",
        Theme::Light => "Dark mode",
    };

    let icon = move || match state.theme.get() {
        Theme::Dark => "\u{2600}",   // sun
        Theme::Light => "\u{1F319}", // crescent moon
    };

    view! {
        <button
            class="theme-toggle"
            on:click=toggle
            title=label
            aria-label=label
        >
            {icon}
        </button>
    }
}
