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
// C4Level, ChatMessage, ChatRole used inside #[cfg(feature = "hydrate")] blocks
#[allow(unused_imports)]
use crate::state::{C4Level, ChatMessage, ChatRole, StudioState};
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

    // Load initial workspace status and intents
    #[cfg(feature = "hydrate")]
    {
        use crate::server_fns::{get_intents, get_workspace_status};
        let state_clone = state.clone();
        leptos::task::spawn_local(async move {
            if let Ok(status) = get_workspace_status().await {
                state_clone.workspace_name.set(status.name);
            }
            if let Ok(intents) = get_intents().await {
                state_clone.intents.set(intents);
            }
        });
    }

    // Reactively load graph data whenever C4 level or selection changes.
    #[cfg(feature = "hydrate")]
    {
        use crate::server_fns::{
            get_block_ops, get_function_detail, get_graph_context, get_module_detail,
        };
        let state_for_effect = state.clone();
        Effect::new(move |_| {
            let level = state_for_effect.c4_level.get();
            let module = state_for_effect.selected_module.get();
            let function = state_for_effect.selected_function.get();
            let block = state_for_effect.selected_block.get();
            let state2 = state_for_effect.clone();

            leptos::task::spawn_local(async move {
                let result = match level {
                    C4Level::Context => get_graph_context().await,
                    C4Level::Container => {
                        if let Some(m) = module {
                            get_module_detail(m).await
                        } else {
                            get_graph_context().await
                        }
                    }
                    C4Level::Component => {
                        if let (Some(m), Some(f)) = (module, function) {
                            get_function_detail(m, f).await
                        } else {
                            return;
                        }
                    }
                    C4Level::Code => {
                        if let (Some(m), Some(f), Some(b)) = (module, function, block) {
                            get_block_ops(m, f, b).await
                        } else {
                            return;
                        }
                    }
                };

                match result {
                    Ok(data) => state2.graph_data.set(data),
                    Err(e) => {
                        state2.chat_messages.update(|msgs| {
                            msgs.push(ChatMessage {
                                role: ChatRole::System,
                                content: format!("Error loading graph: {e}"),
                            });
                        });
                    }
                }
            });
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
