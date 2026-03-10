//! SVG node rendering component.
//!
//! Renders a graph node as an SVG group with a shape (rect, circle, etc.),
//! label text, and optional badge.

use leptos::prelude::*;

use crate::layout::LayoutNode;
use crate::state::StudioState;

/// Renders a single graph node as an SVG element.
///
/// Node shape and color are determined by `node.node_type`.
/// Clicking selects the node; double-clicking drills down.
#[component]
pub fn SvgNode(node: LayoutNode) -> impl IntoView {
    let state = expect_context::<StudioState>();
    let node_id = node.id.clone();
    let node_id_click = node_id.clone();
    let node_id_dblclick = node_id.clone();
    let highlighted = state.highlighted_nodes;

    let is_selected = move || {
        state
            .selected_node
            .get()
            .as_ref()
            .is_some_and(|id| *id == node_id)
    };

    let is_highlighted = {
        let node_id = node_id_click.clone();
        move || highlighted.get().contains(&node_id)
    };

    let css_class = {
        let nt = node.node_type.clone();
        move || {
            let mut cls = format!("graph-node node-{}", nt);
            if is_selected() {
                cls.push_str(" selected");
            }
            if is_highlighted() {
                cls.push_str(" highlighted");
            }
            cls
        }
    };

    let on_click = move |_| {
        state.selected_node.set(Some(node_id_click.clone()));
    };

    let on_dblclick = {
        let node_type = node.node_type.clone();
        move |_| {
            use crate::state::C4Level;
            match node_type.as_str() {
                // C4 Context → Container drill-down (click on software system)
                "system" => {
                    state.selected_module.set(Some("app/main".to_string()));
                    state.c4_level.set(C4Level::Container);
                }
                // C4 Container → Component drill-down (click on binary/runtime)
                "container" => {
                    state.selected_module.set(Some("app/main".to_string()));
                    state.c4_level.set(C4Level::Component);
                }
                // C4 Component → Code drill-down (click on active function)
                "component" => {
                    // Extract function name from id: "component:main" → "main"
                    let fn_name = node_id_dblclick
                        .strip_prefix("component:")
                        .unwrap_or(&node_id_dblclick);
                    state.selected_function.set(Some(fn_name.to_string()));
                    state.c4_level.set(C4Level::Code);
                }
                // Legacy node types (still used at Code level)
                "module" => {
                    state.selected_module.set(Some(node_id_dblclick.clone()));
                    state.c4_level.set(C4Level::Container);
                }
                "function" => {
                    state.selected_function.set(Some(node_id_dblclick.clone()));
                    state.c4_level.set(C4Level::Component);
                }
                "block" => {
                    state.selected_block.set(Some(node_id_dblclick.clone()));
                    state.c4_level.set(C4Level::Code);
                }
                _ => {}
            }
        }
    };

    let x = node.x - node.width / 2.0;
    let y = node.y - node.height / 2.0;
    let rx = match node.node_type.as_str() {
        "person" => node.width / 2.0,                 // oval for person
        "system" | "container" | "component" => 12.0, // rounded rect
        "boundary" => 16.0,                           // large rounding
        "external" => 2.0,                            // sharp corners
        "component-dead" | "component-sub" => 8.0,    // slightly rounded
        "module" | "function" => 8.0,
        "block" => 4.0,
        "Const" | "ConstF64" | "ConstBool" => node.width / 2.0, // circle-ish
        _ => 6.0,
    };

    let has_badge = node.badge.is_some();
    // Label shifts up when badge is present so both fit in the box
    let label_y = if has_badge {
        node.y - 4.0
    } else {
        node.y + 4.0
    };

    let badge_view = node.badge.clone().map(|badge| {
        view! {
            <text
                x=node.x
                y=node.y + 14.0
                text-anchor="middle"
                class="node-badge"
            >
                {badge}
            </text>
        }
    });

    view! {
        <g
            class=css_class
            on:click=on_click
            on:dblclick=on_dblclick
            style="cursor: pointer"
        >
            <rect
                x=x
                y=y
                width=node.width
                height=node.height
                rx=rx
                ry=rx
                class="node-rect"
            />
            <text
                x=node.x
                y=label_y
                text-anchor="middle"
                class="node-label"
            >
                {node.label.clone()}
            </text>
            {badge_view}
        </g>
    }
}
