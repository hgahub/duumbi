//! Root application component.
//!
//! Sets up the layout shell with sidebar, graph canvas, inspector, and chat.
//! Provides `StudioState` context to all child components.

use leptos::prelude::*;
use leptos_meta::*;

use crate::components::breadcrumb::Breadcrumb;
use crate::components::chat::ChatPanel;
use crate::components::graph::GraphCanvas;
use crate::components::inspector::Inspector;
use crate::components::search_overlay::SearchOverlay;
use crate::components::shortcuts::ShortcutsOverlay;
use crate::components::sidebar::Sidebar;
use crate::components::toast::ToastContainer;
use crate::state::{InitialData, StudioState};
use crate::theme::ThemeToggle;

/// Root application component.
///
/// Renders the full Studio layout: sidebar, main graph area with breadcrumb,
/// inspector panel, and bottom chat panel.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // Seed state with pre-loaded data from SSR context (if available).
    let initial = use_context::<InitialData>().unwrap_or_default();
    let state = StudioState::new_with_data(&initial);
    provide_context(state);

    let theme_class = move || state.theme.get().css_class();

    view! {
        <head>
            <Title text="DUUMBI Studio" />
            <Link rel="stylesheet" href="/studio.css" />
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
        </head>
        <body class=theme_class>
        <div class="studio-root">
            // Header bar
            <header class="studio-header">
                <div class="header-left">
                    <span class="studio-logo">"DUUMBI Studio"</span>
                    <span class="studio-version">"v0.7.0"</span>
                    <span class="studio-workspace">{move || state.workspace_name.get()}</span>
                </div>
                <div class="header-right">
                    <ThemeToggle />
                    <button class="header-btn search-btn" title="Search (Ctrl+K)"
                        on:click=move |_| state.search_visible.update(|v| *v = !*v)>
                        "Search"
                    </button>
                    <button class="header-btn shortcuts-btn" title="Keyboard shortcuts (?)"
                        on:click=move |_| state.shortcuts_visible.update(|v| *v = !*v)>
                        "?"
                    </button>
                </div>
            </header>

            // Main content area
            <div class="studio-body">
                // Left sidebar
                <Sidebar />

                // Center: graph + breadcrumb
                <main class="studio-main">
                    <Breadcrumb />
                    <GraphCanvas />
                </main>

                // Right: inspector
                <aside class="studio-inspector">
                    <Inspector />
                </aside>
            </div>

            // Bottom chat panel
            <ChatPanel />

            // Toast notifications
            <ToastContainer />

            // Search overlay (Ctrl+K)
            <SearchOverlay />

            // Keyboard shortcuts overlay
            <ShortcutsOverlay />
        </div>
        <script src="/studio.js"></script>
        </body>
    }
}
