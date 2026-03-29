//! Left icon rail navigation (44px vertical strip).
//!
//! Provides quick-access buttons for toggling the sidebar explorer
//! and opening the command palette.

use leptos::prelude::*;

use crate::state::StudioState;

/// Left icon rail with navigation buttons.
#[component]
pub fn IconRail() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let toggle_sidebar = move |_| {
        state.sidebar_collapsed.update(|v| *v = !*v);
    };

    let open_search = move |_| {
        state.search_visible.set(true);
    };

    view! {
        <div class="icon-rail visible">
            <button class="rail-btn" title="Explorer" on:click=toggle_sidebar>
                <svg viewBox="0 0 18 18">
                    <rect x="2" y="2" width="5.5" height="5.5" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.5"/>
                    <rect x="10.5" y="2" width="5.5" height="5.5" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.5"/>
                    <rect x="2" y="10.5" width="5.5" height="5.5" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.5"/>
                    <rect x="10.5" y="10.5" width="5.5" height="5.5" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.5"/>
                </svg>
            </button>
            <button class="rail-btn" title="Search (Cmd+K)" on:click=open_search>
                <svg viewBox="0 0 18 18">
                    <circle cx="8" cy="8" r="5" fill="none" stroke="currentColor" stroke-width="1.5"/>
                    <line x1="11.8" y1="11.8" x2="16" y2="16" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
            </button>
        </div>
    }
}
