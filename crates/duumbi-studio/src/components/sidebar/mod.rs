//! Sidebar components — Phase 15 redesign.
//!
//! Single tree view (no tabs) with intent tree and module tree.
//! Intents expand to show C4 children. Modules expand to functions.
//! The sidebar push-navigates the canvas and is resizable via JS.

use leptos::prelude::*;

use crate::state::{ActivePanel, StudioState};

/// Left sidebar with intent tree and module explorer.
///
/// Rendered inside the Phase 15 layout shell. Visibility is controlled
/// by `sidebar_collapsed` state. Resizable via JS drag handle.
#[component]
pub fn Sidebar() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let sidebar_class = move || {
        if state.sidebar_collapsed.get() {
            "sidebar"
        } else {
            "sidebar open"
        }
    };

    view! {
        <div class=sidebar_class id="sidebar">
            <div class="sidebar-inner" id="sidebarInner">
                <div class="sidebar-resize" id="sidebarResize"></div>

                // Header with title and pin/close
                <div class="sidebar-header">
                    <span class="sidebar-title">"Explorer"</span>
                    <div class="sidebar-actions">
                        <div class="sidebar-pin" id="pinBtn" title="Pin sidebar">
                            <svg viewBox="0 0 14 14">
                                <path d="M5 1.5l4 2.5v3l1.5 1.5v1.5h-3.5V13l-1 1-1-1V10H1.5V8.5L3 7V4z"
                                    fill="none" stroke="currentColor" stroke-width="1.5"/>
                            </svg>
                        </div>
                        <div class="sidebar-close"
                            on:click=move |_| state.sidebar_collapsed.set(true)>
                            {"\u{2715}"}
                        </div>
                    </div>
                </div>

                // Scrollable content
                <div class="sidebar-scroll">
                    // Intents page (controlled by JS page switching)
                    <div class="sb-page active" id="page-intents">
                        // Workspace intents section
                        <div class="sidebar-section">
                            <div class="section-header">
                                <div class="section-label">"Workspace"</div>
                                <div class="section-create" title="Create intent" id="sidebarCreateBtn">
                                    <svg viewBox="0 0 12 12">
                                        <line x1="6" y1="2" x2="6" y2="10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                                        <line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                                    </svg>
                                </div>
                            </div>

                            // Intent tree — rendered from state, JS handles expand/collapse
                            {move || {
                                let intents = state.intents.get();
                                if intents.is_empty() {
                                    view! {
                                        <div class="tree-item" style="color:#5a5855">
                                            "No intents yet"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            {intents.into_iter().map(|intent| {
                                                let slug = intent.slug.clone();
                                                let slug_id = format!("intent-{}", slug.replace(' ', "-"));
                                                let children_id = format!("children-{}", slug.replace(' ', "-"));
                                                view! {
                                                    <div class="tree-intent" id=slug_id.clone()>
                                                        <svg class="intent-chevron" viewBox="0 0 10 10">
                                                            <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.3"
                                                                fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                                                        </svg>
                                                        <svg class="tree-icon" viewBox="0 0 12 12" style="opacity:.8">
                                                            <circle cx="6" cy="6" r="5" stroke="currentColor" fill="none"/>
                                                            <circle cx="6" cy="6" r="2" stroke="currentColor" fill="none"/>
                                                        </svg>
                                                        <span>{slug}</span>
                                                    </div>
                                                    <div class="tree-children" id=children_id>
                                                        <div class="tree-child">
                                                            <span class="child-dot" style="background:#6fd8b2"></span>
                                                            "Context"
                                                            <span class="tree-badge tb-fn" style="margin-left:auto">"C4"</span>
                                                        </div>
                                                        <div class="tree-child">
                                                            <span class="child-dot" style="background:#9ac4ef"></span>
                                                            "Container"
                                                            <span class="tree-badge tb-mod" style="margin-left:auto">"C4"</span>
                                                        </div>
                                                        <div class="tree-child">
                                                            <span class="child-dot" style="background:#e07830"></span>
                                                            "Component"
                                                            <span class="tree-badge" style="margin-left:auto;background:#352618;color:#e07830">"C4"</span>
                                                        </div>
                                                        <div class="tree-child">
                                                            <span class="child-dot" style="background:#c25a1a"></span>
                                                            "Code"
                                                            <span class="tree-badge" style="margin-left:auto;background:#351a1a;color:#f09090">"C4"</span>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>

                        // Modules section
                        <div class="sidebar-section">
                            <div class="section-label">"Modules"</div>
                            <ModuleTree />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Module tree showing workspace module structure.
///
/// SSR renders the initial module list from `InitialData`.
/// Click navigates to Graph panel at that module.
#[component]
fn ModuleTree() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let modules = use_context::<crate::state::InitialData>()
        .map(|d| d.modules.clone())
        .unwrap_or_default();

    view! {
        <div>
            {modules.into_iter().map(|module_id| {
                let mid = module_id.clone();
                let mid_click = module_id.clone();
                view! {
                    <div class="tree-item"
                        on:click=move |_| {
                            state.selected_module.set(Some(mid_click.clone()));
                            state.active_panel.set(ActivePanel::Graph);
                        }>
                        <svg class="tree-icon" viewBox="0 0 13 13">
                            <rect x="1" y="3" width="11" height="8" rx="1.5"
                                fill="none" stroke="currentColor" stroke-width="1.3"/>
                            <path d="M1 5h11" stroke="currentColor" stroke-width="1.3"/>
                        </svg>
                        <span>{mid}</span>
                        <span class="tree-badge tb-mod">"mod"</span>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
