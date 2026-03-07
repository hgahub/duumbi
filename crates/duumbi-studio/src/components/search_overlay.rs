//! Ctrl+K quick search overlay component.
//!
//! Provides fuzzy search over modules, functions, and blocks
//! from the current graph data. Navigates on selection.

use leptos::prelude::*;

use crate::state::{C4Level, StudioState};

/// Quick search overlay triggered by Ctrl+K.
///
/// Searches node ids, labels, and types. Navigating to a result
/// sets the appropriate C4 level and selection signal.
#[component]
pub fn SearchOverlay() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let (query, set_query) = signal(String::new());

    // Extract signals (all Copy) so Show children closure can be Fn
    let search_visible = state.search_visible;
    let graph_data = state.graph_data;
    let selected_module = state.selected_module;
    let selected_function = state.selected_function;
    let selected_block = state.selected_block;
    let selected_node = state.selected_node;
    let c4_level = state.c4_level;

    // Filtered results: match query against node id, label, type
    let results = Memo::new(move |_| {
        let q = query.get().to_lowercase();
        let data = graph_data.get();
        if q.is_empty() {
            return data.nodes.into_iter().take(8).collect::<Vec<_>>();
        }
        data.nodes
            .into_iter()
            .filter(|n| {
                n.id.to_lowercase().contains(&q)
                    || n.label.to_lowercase().contains(&q)
                    || n.node_type.to_lowercase().contains(&q)
            })
            .take(8)
            .collect::<Vec<_>>()
    });

    let is_empty = move || results.get().is_empty();

    view! {
        <Show when=move || search_visible.get()>
            <div class="search-overlay"
                on:click=move |_: leptos::ev::MouseEvent| {
                    search_visible.set(false);
                    set_query.set(String::new());
                }
            >
                <div class="search-panel"
                    on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                >
                    <input
                        type="text"
                        class="search-input"
                        placeholder="Search nodes, functions, modules..."
                        prop:value=move || query.get()
                        on:input=move |ev| {
                            let target = event_target::<web_sys::HtmlInputElement>(&ev);
                            set_query.set(target.value());
                        }
                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                            if ev.key() == "Escape" {
                                search_visible.set(false);
                                set_query.set(String::new());
                            }
                        }
                    />

                    <ul class="search-results">
                        {move || results.get().into_iter().map(|node| {
                            let nid = node.id.clone();
                            let ntype = node.node_type.clone();
                            let on_select = move |_: leptos::ev::MouseEvent| {
                                match ntype.as_str() {
                                    "module" => {
                                        selected_module.set(Some(nid.clone()));
                                        c4_level.set(C4Level::Container);
                                    }
                                    "function" => {
                                        selected_function.set(Some(nid.clone()));
                                        c4_level.set(C4Level::Component);
                                    }
                                    "block" => {
                                        selected_block.set(Some(nid.clone()));
                                        c4_level.set(C4Level::Code);
                                    }
                                    _ => {
                                        selected_node.set(Some(nid.clone()));
                                    }
                                }
                                search_visible.set(false);
                                set_query.set(String::new());
                            };
                            view! {
                                <li class="search-result" on:click=on_select>
                                    <span class=format!("result-type type-{}", node.node_type)>
                                        {node.node_type.clone()}
                                    </span>
                                    <span class="result-label">{node.label.clone()}</span>
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>

                    <Show when=is_empty>
                        <div class="search-empty">"No results"</div>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
