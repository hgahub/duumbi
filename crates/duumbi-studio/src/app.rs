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
use crate::components::sidebar::Sidebar;
use crate::components::toast::ToastContainer;
use crate::state::StudioState;
use crate::theme::ThemeToggle;

/// Root application component.
///
/// Renders the full Studio layout: sidebar, main graph area with breadcrumb,
/// inspector panel, and bottom chat panel.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let state = StudioState::new();
    provide_context(state.clone());

    // Load initial workspace status
    #[cfg(feature = "hydrate")]
    {
        use crate::server_fns::get_workspace_status;
        let state_clone = state.clone();
        leptos::task::spawn_local(async move {
            if let Ok(status) = get_workspace_status().await {
                state_clone.workspace_name.set(status.name);
            }
        });
    }

    let theme_class = move || state.theme.get().css_class();

    view! {
        <Html attr:class=theme_class />
        <Title text="DUUMBI Studio" />
        <Link rel="stylesheet" href="/studio.css" />

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
                    <button class="header-btn search-btn" title="Search (Ctrl+K)">
                        "Search"
                    </button>
                    <button class="header-btn shortcuts-btn" title="Keyboard shortcuts (?)">
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
        </div>
    }
}
