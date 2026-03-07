//! Node inspector panel component.
//!
//! Shows details of the currently selected graph node: type, id,
//! fields, edges, and metadata.

use leptos::prelude::*;

use crate::state::StudioState;

/// Right-side inspector panel showing selected node details.
#[component]
pub fn Inspector() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let selected_node_info = move || {
        let node_id = state.selected_node.get()?;
        let data = state.graph_data.get();
        data.nodes.into_iter().find(|n| n.id == node_id)
    };

    view! {
        <div class="inspector-panel">
            <h3 class="panel-title">"Inspector"</h3>

            {move || match selected_node_info() {
                Some(node) => view! {
                    <div class="inspector-content">
                        <div class="inspector-field">
                            <span class="field-label">"@type"</span>
                            <span class="field-value">{node.node_type.clone()}</span>
                        </div>
                        <div class="inspector-field">
                            <span class="field-label">"@id"</span>
                            <span class="field-value">{node.id.clone()}</span>
                        </div>
                        <div class="inspector-field">
                            <span class="field-label">"label"</span>
                            <span class="field-value">{node.label.clone()}</span>
                        </div>
                        {node.badge.clone().map(|b| view! {
                            <div class="inspector-field">
                                <span class="field-label">"info"</span>
                                <span class="field-value">{b}</span>
                            </div>
                        })}
                    </div>
                }.into_any(),
                None => view! {
                    <div class="inspector-empty">
                        <p>"Select a node to inspect"</p>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
