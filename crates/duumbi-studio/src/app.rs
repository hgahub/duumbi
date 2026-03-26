//! Root application component.
//!
//! Sets up the Phase 15 layout: icon rail + sidebar + 3-panel canvas + footer.
//! Provides `StudioState` context to all child components.

use leptos::prelude::*;
use leptos_meta::*;

use crate::components::command_palette::CommandPalette;
use crate::components::footer::Footer;
use crate::components::icon_rail::IconRail;
use crate::components::panels::build_panel::BuildPanel;
use crate::components::panels::graph_panel::GraphPanel;
use crate::components::panels::intents_panel::IntentsPanel;
use crate::components::sidebar::Sidebar;
use crate::components::toast::ToastContainer;
use crate::state::{ActivePanel, InitialData, StudioState};

/// Root application component.
///
/// Renders the Studio with icon rail, sidebar, main canvas (3 panels),
/// footer navigation, command palette, and toast notifications.
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
            <Link
                rel="stylesheet"
                href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&family=Sora:wght@400;500;600&display=swap"
            />
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
        </head>
        <body class=theme_class>
        <div class="studio-root">
            // Header bar
            <header>
                <div class="workspace">
                    <span class="workspace-name">{move || state.workspace_name.get()}</span>
                    <svg class="workspace-chevron" viewBox="0 0 12 12" fill="none">
                        <path d="M3 4.5L6 7.5L9 4.5" stroke="#e8e4d9" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                </div>
                <div class="header-right">
                    <div class="header-search"
                        on:click=move |_| state.search_visible.set(true)
                        title="Search">
                        <svg viewBox="0 0 16 16">
                            <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/>
                            <line x1="10.2" y1="10.2" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                        </svg>
                        <span class="search-hotkey">{"\u{2318}K"}</span>
                    </div>
                </div>
            </header>

            // Left icon rail
            <IconRail />

            // Sidebar (intent tree + module tree)
            <Sidebar />

            // Main canvas — switches between 3 panels
            <div class="canvas" id="canvas">
                {move || match state.active_panel.get() {
                    ActivePanel::Intents => view! { <IntentsPanel /> }.into_any(),
                    ActivePanel::Graph => view! { <GraphPanel /> }.into_any(),
                    ActivePanel::Build => view! { <BuildPanel /> }.into_any(),
                }}
            </div>

            // Bottom footer with panel navigation
            <Footer />

            // Command palette (Cmd+K)
            <CommandPalette />

            // Toast notifications
            <ToastContainer />
        </div>
        <script src="/studio.js"></script>
        </body>
    }
}
