//! Bottom footer with 3-panel navigation (Intents / Graph / Build).
//!
//! Replaces the previous 5-item navigation with a streamlined 3-panel
//! layout that mirrors the development cycle: describe → visualize → compile.

use leptos::prelude::*;

use crate::state::{ActivePanel, StudioState};

/// Bottom footer bar with panel toggle buttons and user info.
#[component]
pub fn Footer() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let panel_class = move |panel: ActivePanel| {
        if state.active_panel.get() == panel {
            "footer-item active"
        } else {
            "footer-item"
        }
    };

    view! {
        <div class="footer">
            <div class="footer-avatar" title="User menu">
                <span class="avatar-initials">"D"</span>
                <span class="avatar-status"></span>
            </div>

            <div class="footer-center">
                <div class=move || panel_class(ActivePanel::Intents)
                    on:click=move |_| state.active_panel.set(ActivePanel::Intents)>
                    <svg viewBox="0 0 26 26">
                        <circle cx="13" cy="13" r="10" fill="none" stroke="currentColor" stroke-width="1.3"/>
                        <circle cx="13" cy="13" r="5" fill="none" stroke="currentColor" stroke-width="1.3"/>
                        <circle cx="13" cy="13" r="1.5" fill="currentColor"/>
                    </svg>
                    <span class="footer-label">"Intents"</span>
                </div>

                <div class=move || panel_class(ActivePanel::Graph)
                    on:click=move |_| state.active_panel.set(ActivePanel::Graph)>
                    <svg viewBox="0 0 26 26">
                        <rect x="3" y="3" width="8" height="8" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/>
                        <rect x="15" y="3" width="8" height="8" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/>
                        <rect x="9" y="15" width="8" height="8" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/>
                        <line x1="7" y1="11" x2="13" y2="15" stroke="currentColor" stroke-width="1.3"/>
                        <line x1="19" y1="11" x2="13" y2="15" stroke="currentColor" stroke-width="1.3"/>
                    </svg>
                    <span class="footer-label">"Graph"</span>
                </div>

                <div class=move || panel_class(ActivePanel::Build)
                    on:click=move |_| state.active_panel.set(ActivePanel::Build)>
                    <svg viewBox="0 0 26 26">
                        <path d="M4 20L13 4L22 20H4Z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
                        <line x1="13" y1="11" x2="13" y2="15" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                        <circle cx="13" cy="17.5" r="0.8" fill="currentColor"/>
                    </svg>
                    <span class="footer-label">"Build"</span>
                </div>
            </div>
        </div>
    }
}
