//! Sidebar components.
//!
//! Contains the file explorer, intent panel, and config panel.

use leptos::prelude::*;

use crate::state::StudioState;

/// Left sidebar component.
///
/// Shows the file explorer (module tree), intent list, and config.
#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <aside class="studio-sidebar">
            <div class="sidebar-content">
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
///
/// SSR renders the initial module list from `InitialData`. The JS
/// `updateSidebarTree()` rebuilds this on every C4 navigation.
#[component]
fn ModuleTree() -> impl IntoView {
    let modules = use_context::<crate::state::InitialData>()
        .map(|d| d.modules.clone())
        .unwrap_or_default();

    view! {
        <ul class="module-tree">
            {modules.into_iter().map(|module_id| {
                let mid = module_id.clone();
                view! {
                    <li class="module-item">
                        <span class="tree-arrow">"\u{25B8}"</span>
                        <span class="module-name">{mid}</span>
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
