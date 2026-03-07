//! Keyboard shortcuts overlay component.
//!
//! Shows a `?` overlay listing all Studio keyboard shortcuts.
//! Mounted in the root App; toggled via the `?` header button or `?` key.

use leptos::prelude::*;

use crate::state::StudioState;

/// Renders the keyboard shortcuts help overlay.
///
/// Visible when `StudioState.shortcuts_visible` is true.
#[component]
pub fn ShortcutsOverlay() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let on_close = move |_| state.shortcuts_visible.set(false);

    view! {
        {move || state.shortcuts_visible.get().then(|| view! {
            <div class="shortcuts-overlay" on:click=on_close>
                <div class="shortcuts-panel" on:click=|ev| ev.stop_propagation()>
                    <h2>"Keyboard Shortcuts"</h2>
                    <div class="shortcut-row">
                        <span>"Open this help"</span>
                        <kbd class="shortcut-key">"?"</kbd>
                    </div>
                    <div class="shortcut-row">
                        <span>"Quick search"</span>
                        <kbd class="shortcut-key">"Ctrl+K"</kbd>
                    </div>
                    <div class="shortcut-row">
                        <span>"Trigger build"</span>
                        <kbd class="shortcut-key">"b"</kbd>
                    </div>
                    <div class="shortcut-row">
                        <span>"Focus chat input"</span>
                        <kbd class="shortcut-key">"/"</kbd>
                    </div>
                    <div class="shortcut-row">
                        <span>"Toggle theme"</span>
                        <kbd class="shortcut-key">"Ctrl+Shift+T"</kbd>
                    </div>
                    <div class="shortcut-row">
                        <span>"Close / dismiss"</span>
                        <kbd class="shortcut-key">"Esc"</kbd>
                    </div>
                    <div class="shortcut-row">
                        <span>"Double-click node"</span>
                        <span>"Drill down"</span>
                    </div>
                </div>
            </div>
        })}
    }
}
