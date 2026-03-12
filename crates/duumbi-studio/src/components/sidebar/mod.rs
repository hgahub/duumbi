//! Sidebar components.
//!
//! Contains the file explorer, intent panel, and registry search panel.

use leptos::prelude::*;

use crate::state::{SidebarTab, StudioState, ToastKind};

/// Left sidebar component.
///
/// Shows tabbed navigation: Explorer, Intents, Registry.
#[component]
pub fn Sidebar() -> impl IntoView {
    let state = expect_context::<StudioState>();

    view! {
        <aside class="studio-sidebar">
            <div class="sidebar-tabs">
                <button
                    class=move || if state.sidebar_tab.get() == SidebarTab::Explorer { "sidebar-tab active" } else { "sidebar-tab" }
                    on:click=move |_| state.sidebar_tab.set(SidebarTab::Explorer)
                >
                    "Explorer"
                </button>
                <button
                    class=move || if state.sidebar_tab.get() == SidebarTab::Intents { "sidebar-tab active" } else { "sidebar-tab" }
                    on:click=move |_| state.sidebar_tab.set(SidebarTab::Intents)
                >
                    "Intents"
                </button>
                <button
                    class=move || if state.sidebar_tab.get() == SidebarTab::Registry { "sidebar-tab active" } else { "sidebar-tab" }
                    on:click=move |_| state.sidebar_tab.set(SidebarTab::Registry)
                >
                    "Registry"
                </button>
            </div>
            <div class="sidebar-content">
                {move || match state.sidebar_tab.get() {
                    SidebarTab::Explorer => view! {
                        <div class="sidebar-section">
                            <h3 class="section-title">"Explorer"</h3>
                            <ModuleTree />
                        </div>
                    }.into_any(),
                    SidebarTab::Intents => view! {
                        <div class="sidebar-section">
                            <h3 class="section-title">"Intents"</h3>
                            <IntentList />
                        </div>
                    }.into_any(),
                    SidebarTab::Registry => view! {
                        <div class="sidebar-section">
                            <RegistryPanel />
                        </div>
                    }.into_any(),
                }}
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
                        <span class="tree-arrow" inner_html="<svg viewBox='0 0 16 16'><polyline points='6 4 10 8 6 12'/></svg>"></span>
                        <span class="tree-icon" style="color:#5b9bd5">"\u{2B22}"</span>
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

/// Registry search and installed dependencies panel.
#[component]
fn RegistryPanel() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let search_input = NodeRef::<leptos::html::Input>::new();

    // Load installed deps on mount
    let deps_resource = Resource::new(
        || (),
        |_| async {
            crate::server_fns::get_installed_deps()
                .await
                .unwrap_or_default()
        },
    );

    // Update state when deps load
    Effect::new(move || {
        if let Some(deps) = deps_resource.get() {
            state.installed_deps.set(deps);
        }
    });

    let do_search = move || {
        let input = search_input.get().expect("invariant: search input mounted");
        let query = input.value();
        if query.trim().is_empty() {
            state.registry_results.set(Vec::new());
            return;
        }

        state.registry_loading.set(true);
        let state_clone = state;
        leptos::task::spawn_local(async move {
            match crate::server_fns::search_registry(query).await {
                Ok(results) => {
                    state_clone.registry_results.set(results);
                }
                Err(e) => {
                    super::toast::push_toast(
                        &state_clone,
                        ToastKind::Error,
                        format!("Search failed: {e}"),
                    );
                }
            }
            state_clone.registry_loading.set(false);
        });
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            do_search();
        }
    };

    let on_click_search = move |_: web_sys::MouseEvent| {
        do_search();
    };

    view! {
        <div class="registry-panel">
            // Search section
            <h3 class="section-title">"Search Registry"</h3>
            <div class="registry-search">
                <input
                    type="text"
                    class="registry-search-input"
                    placeholder="Search modules..."
                    node_ref=search_input
                    on:keydown=on_keydown
                />
                <button
                    class="registry-search-btn"
                    on:click=on_click_search
                    disabled=move || state.registry_loading.get()
                >
                    {move || if state.registry_loading.get() { "..." } else { "\u{1F50D}" }}
                </button>
            </div>

            // Search results
            {move || {
                let results = state.registry_results.get();
                if results.is_empty() {
                    view! { <div></div> }.into_any()
                } else {
                    view! {
                        <ul class="registry-results">
                            {results.into_iter().map(|hit| {
                                let name = hit.name.clone();
                                let install_name = hit.name.clone();
                                let state_for_install = state;
                                view! {
                                    <li class="registry-result-item">
                                        <div class="registry-result-info">
                                            <span class="registry-result-name">{name}</span>
                                            <span class="registry-result-version">{hit.latest_version}</span>
                                        </div>
                                        {hit.description.map(|desc| view! {
                                            <div class="registry-result-desc">{desc}</div>
                                        })}
                                        <button
                                            class="registry-install-btn"
                                            on:click=move |_| {
                                                let module = install_name.clone();
                                                let st = state_for_install;
                                                leptos::task::spawn_local(async move {
                                                    match crate::server_fns::install_module(module.clone()).await {
                                                        Ok(msg) => {
                                                            super::toast::push_toast(&st, ToastKind::Success, msg);
                                                            // Refresh installed deps
                                                            if let Ok(deps) = crate::server_fns::get_installed_deps().await {
                                                                st.installed_deps.set(deps);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            super::toast::push_toast(
                                                                &st,
                                                                ToastKind::Error,
                                                                format!("Install failed: {e}"),
                                                            );
                                                        }
                                                    }
                                                });
                                            }
                                        >
                                            "Install"
                                        </button>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    }.into_any()
                }
            }}

            // Installed dependencies
            <h3 class="section-title" style="margin-top: 16px">"Installed"</h3>
            {move || {
                let deps = state.installed_deps.get();
                if deps.is_empty() {
                    view! { <p class="registry-empty">"No dependencies"</p> }.into_any()
                } else {
                    view! {
                        <ul class="registry-deps-list">
                            {deps.into_iter().map(|dep| {
                                view! {
                                    <li class="registry-dep-item">
                                        <span class="registry-dep-name">{dep.name}</span>
                                        <span class="registry-dep-version">{dep.version}</span>
                                        <span class="registry-dep-source">{dep.source}</span>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    }.into_any()
                }
            }}
        </div>
    }
}
