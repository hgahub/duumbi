//! Build panel — compile and run with output display.
//!
//! One-click build and run. Shows compilation output, errors with
//! node IDs, and binary execution stdout/stderr.

use leptos::prelude::*;

use crate::state::{BuildStatus, StudioState};

/// Build panel: compile and run the workspace.
///
/// "Build" triggers `trigger_build()` server function.
/// "Run" executes the compiled binary and captures output.
/// Error node IDs are displayed (future: clickable to navigate to Graph).
#[component]
pub fn BuildPanel() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let build_output = RwSignal::new(String::new());
    let run_output = RwSignal::new(String::new());

    let on_build = move |_| {
        state.build_status.set(BuildStatus::Building);
        // JS calls /api/build and updates the output
    };

    let status_text = move || {
        match state.build_status.get() {
            BuildStatus::Idle => "Ready to build",
            BuildStatus::Building => "Building...",
            BuildStatus::Success => "Build succeeded",
            BuildStatus::Failed(ref msg) => return msg.clone(),
        }
        .to_string()
    };

    let status_class = move || match state.build_status.get() {
        BuildStatus::Idle => "tb-mod",
        BuildStatus::Building => "tb-mod",
        BuildStatus::Success => "tb-fn",
        BuildStatus::Failed(_) => "tb-err",
    };

    view! {
        <div class="workspace-view active" style="display:flex;flex-direction:column">
            // Toolbar
            <div class="md-panel-header" style="flex-shrink:0">
                <button class="cip-btn cip-btn-create" on:click=on_build>
                    "Build"
                </button>
                <button class="cip-btn cip-btn-cancel" id="runBtn">
                    "Run"
                </button>
                <span class=move || format!("tree-badge {}", status_class())
                    style="margin-left:auto">
                    {status_text}
                </span>
            </div>

            // Build output
            <div class="md-panel" style="flex:1">
                <div class="md-panel-header">
                    <div class="md-panel-tab active">"Build Output"</div>
                </div>
                <div class="md-content">
                    <pre>
                        <code>{move || {
                            let out = build_output.get();
                            if out.is_empty() {
                                "Press Build to compile the workspace.".to_string()
                            } else {
                                out
                            }
                        }}</code>
                    </pre>
                </div>
            </div>

            // Run output
            <div class="md-panel" style="flex:1">
                <div class="md-panel-header">
                    <div class="md-panel-tab active">"Run Output"</div>
                </div>
                <div class="md-content">
                    <pre>
                        <code>{move || {
                            let out = run_output.get();
                            if out.is_empty() {
                                "Build first, then press Run.".to_string()
                            } else {
                                out
                            }
                        }}</code>
                    </pre>
                </div>
            </div>
        </div>
    }
}
