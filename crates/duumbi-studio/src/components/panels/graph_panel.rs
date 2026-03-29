//! Graph panel — C4 drill-down + context-aware chat split view.
//!
//! Left pane: C4 graph visualization (reuses existing `GraphCanvas`).
//! Right pane: Chat connected to WebSocket for streaming LLM mutations.
//! A resizable splitter divides the two panes.

use leptos::prelude::*;

use crate::components::chat::ChatPanel;
use crate::components::graph::GraphCanvas;
use crate::state::StudioState;

/// Graph panel: the visual core of the Studio.
///
/// Split view with C4 graph on the left and contextual chat on the right.
/// The chat sends the current C4 level and module to the WebSocket handler
/// for context-aware prompt enrichment.
#[component]
pub fn GraphPanel() -> impl IntoView {
    let _state = expect_context::<StudioState>();

    view! {
        <div class="workspace-view active" style="display:flex">
            // Left: Graph canvas
            <div class="md-panel" style="flex:1;min-width:300px">
                <div class="md-panel-header">
                    <div class="md-panel-tab active">
                        <svg viewBox="0 0 12 12" style="width:12px;height:12px">
                            <rect x="1" y="1" width="10" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/>
                        </svg>
                        "Graph"
                    </div>
                </div>
                <div style="flex:1;position:relative;overflow:hidden">
                    <GraphCanvas />
                </div>
            </div>

            // Resizable splitter
            <div class="split-resize" id="splitResize"></div>

            // Right: Chat panel
            <div class="chat-panel">
                <ChatPanel />
            </div>
        </div>
    }
}
