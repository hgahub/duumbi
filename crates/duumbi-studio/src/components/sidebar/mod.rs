//! Sidebar components.
//!
//! Contains the file explorer, intent panel, and config panel.

use leptos::prelude::*;

use crate::state::{C4Level, StudioState};

/// Left sidebar component.
///
/// Shows the file explorer (module tree), intent list, and config.
#[component]
pub fn Sidebar() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let is_collapsed = move || state.sidebar_collapsed.get();
    let toggle = move |_| state.sidebar_collapsed.update(|v| *v = !*v);

    view! {
        <aside class="studio-sidebar" class:collapsed=is_collapsed>
            <button class="sidebar-toggle" on:click=toggle title="Toggle sidebar">
                {move || if is_collapsed() { ">" } else { "<" }}
            </button>

            <div class="sidebar-content" style:display=move || if is_collapsed() { "none" } else { "block" }>
                // Explorer section
                <div class="sidebar-section">
                    <h3 class="section-title">"Explorer"</h3>
                    <ModuleTree />
                </div>

                // Intents section
                <div class="sidebar-section">
                    <h3 class="section-title">"Intents"</h3>
                    <IntentList />
                </div>
            </div>
        </aside>
    }
}

/// Module tree showing workspace structure.
#[component]
fn ModuleTree() -> impl IntoView {
    let state = expect_context::<StudioState>();

    // Use graph_data to show modules (from Context level data)
    let modules = move || {
        let data = state.graph_data.get();
        data.nodes
            .iter()
            .filter(|n| n.node_type == "module")
            .map(|n| n.id.clone())
            .collect::<Vec<_>>()
    };

    view! {
        <ul class="module-tree">
            {move || modules().into_iter().map(|module_id| {
                let mid = module_id.clone();
                let on_click = move |_| {
                    state.selected_module.set(Some(mid.clone()));
                    state.c4_level.set(C4Level::Container);
                };
                view! {
                    <li class="module-item" on:click=on_click>
                        <span class="module-icon">">"</span>
                        <span class="module-name">{module_id}</span>
                    </li>
                }
            }).collect::<Vec<_>>()}
        </ul>
    }
}

/// Intent list showing active and archived intents.
#[component]
fn IntentList() -> impl IntoView {
    let state = expect_context::<StudioState>();

    view! {
        <ul class="intent-list">
            {move || state.intents.get().into_iter().map(|intent| {
                let status_class = match intent.status.as_str() {
                    "Done" | "Completed" => "status-done",
                    "Running" | "InProgress" => "status-running",
                    "Failed" => "status-failed",
                    _ => "status-pending",
                };
                view! {
                    <li class="intent-item">
                        <span class=format!("intent-status {status_class}")>
                            {match intent.status.as_str() {
                                "Done" | "Completed" => "\u{2713}",
                                "Running" | "InProgress" => "\u{23F3}",
                                "Failed" => "\u{2717}",
                                _ => "\u{25CB}",
                            }}
                        </span>
                        <span class="intent-name">{intent.slug}</span>
                    </li>
                }
            }).collect::<Vec<_>>()}
        </ul>
    }
}
