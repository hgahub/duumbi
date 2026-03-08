//! Graph visualization components.
//!
//! SVG-based graph rendering with pan/zoom, node/edge rendering,
//! and C4 level-specific views.

pub mod c4_code;
pub mod c4_component;
pub mod c4_container;
pub mod c4_context;
pub mod interaction;
pub mod svg_edge;
pub mod svg_node;

use leptos::prelude::*;

use crate::layout;
use crate::state::{C4Level, StudioState};

/// Main graph canvas component.
///
/// Renders an SVG container with pan/zoom support. Delegates to
/// the appropriate C4 level view based on `StudioState.c4_level`.
#[component]
pub fn GraphCanvas() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let (view_box, set_view_box) = signal("0 0 800 600".to_string());
    let (transform, set_transform) = signal("translate(0,0) scale(1)".to_string());

    // Compute layout whenever graph data changes
    let layout_data = Memo::new(move |_| {
        let data = state.graph_data.get();
        let (nodes, bbox) = layout::compute_layout(&data);
        let edges = layout::edge_routing::route_edges(&data.edges, &nodes);

        // Update viewBox to fit content
        let vb = format!(
            "{} {} {} {}",
            bbox.min_x - 20.0,
            bbox.min_y - 20.0,
            bbox.width() + 40.0,
            bbox.height() + 40.0
        );
        set_view_box.set(vb);

        (nodes, edges)
    });

    // Pan/zoom state
    let interaction_state = interaction::InteractionState::new();
    let on_wheel = interaction::on_wheel(interaction_state.clone(), set_transform);
    let on_mousedown = interaction::on_mousedown(interaction_state.clone());
    let on_mousemove = interaction::on_mousemove(interaction_state.clone(), set_transform);
    let on_mouseup = interaction::on_mouseup(interaction_state.clone());

    view! {
        <div class="graph-canvas-container">
            <svg
                class="graph-canvas"
                viewBox=move || view_box.get()
                on:wheel=on_wheel
                on:mousedown=on_mousedown
                on:mousemove=on_mousemove
                on:mouseup=on_mouseup
            >
                // Arrow marker definition
                <defs>
                    <marker
                        id="arrowhead"
                        markerWidth="10"
                        markerHeight="7"
                        refX="10"
                        refY="3.5"
                        orient="auto"
                    >
                        <polygon points="0 0, 10 3.5, 0 7" class="edge-arrow" />
                    </marker>
                </defs>

                <g transform=move || transform.get()>
                    // Render edges first (below nodes)
                    {move || {
                        let (_, edges) = layout_data.get();
                        edges.into_iter().map(|edge| {
                            view! { <svg_edge::SvgEdge edge=edge /> }
                        }).collect::<Vec<_>>()
                    }}

                    // Render nodes
                    {move || {
                        let (nodes, _) = layout_data.get();
                        nodes.into_iter().map(|node| {
                            view! { <svg_node::SvgNode node=node /> }
                        }).collect::<Vec<_>>()
                    }}
                </g>
            </svg>

            // C4 level tabs
            <div class="c4-tabs">
                <C4Tab level=C4Level::Context label="Context" />
                <C4Tab level=C4Level::Container label="Container" />
                <C4Tab level=C4Level::Component label="Component" />
                <C4Tab level=C4Level::Code label="Code" />
            </div>
        </div>
    }
}

/// A single C4 level tab button.
#[component]
fn C4Tab(level: C4Level, label: &'static str) -> impl IntoView {
    let state = expect_context::<StudioState>();

    let is_active = move || state.c4_level.get() == level;

    let on_click = move |_| {
        state.c4_level.set(level);
    };

    view! {
        <button
            class="c4-tab"
            class:active=is_active
            on:click=on_click
        >
            {label}
        </button>
    }
}
