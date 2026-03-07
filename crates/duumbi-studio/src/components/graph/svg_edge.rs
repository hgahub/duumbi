//! SVG edge rendering component.
//!
//! Renders a graph edge as an SVG path with an arrowhead marker
//! and optional label.

use leptos::prelude::*;

use crate::layout::LayoutEdge;

/// Renders a single graph edge as an SVG path.
#[component]
pub fn SvgEdge(edge: LayoutEdge) -> impl IntoView {
    let css_class = format!("graph-edge edge-{}", edge.edge_type);
    let has_label = !edge.label.is_empty();

    view! {
        <g class=css_class>
            <path
                d=edge.path_data.clone()
                class="edge-path"
                marker-end="url(#arrowhead)"
            />
            {if has_label {
                Some(view! {
                    <text
                        x=edge.label_x
                        y=edge.label_y
                        text-anchor="middle"
                        class="edge-label"
                    >
                        {edge.label.clone()}
                    </text>
                })
            } else {
                None
            }}
        </g>
    }
}
