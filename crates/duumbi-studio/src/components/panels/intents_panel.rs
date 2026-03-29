//! Intents panel — unified create, review, and execute.
//!
//! Left side: intent list with status indicators.
//! Right side: selected intent detail (description, acceptance criteria,
//! task plan with progress, test cases, action buttons).

use leptos::prelude::*;

use crate::state::StudioState;

/// Intents panel: the first stop in the development cycle.
///
/// Displays the intent list and detail view in a master-detail layout.
/// Create, review, and execute intents without leaving this panel.
#[component]
pub fn IntentsPanel() -> impl IntoView {
    let state = expect_context::<StudioState>();

    view! {
        <div class="workspace-view active" style="display:flex">
            // Left: intent list
            <div class="md-panel" style="max-width:280px">
                <div class="md-panel-header">
                    <div class="md-panel-tab active">"Intents"</div>
                </div>
                <div class="md-content">
                    {move || {
                        let intents = state.intents.get();
                        if intents.is_empty() {
                            view! {
                                <p style="color:#5a5855;text-align:center;padding-top:40px">
                                    "No intents yet. Create one to get started."
                                </p>
                            }.into_any()
                        } else {
                            view! {
                                <div class="intent-list">
                                    {intents.into_iter().map(|intent| {
                                        let status_class = match intent.status.as_str() {
                                            "Completed" => "tb-fn",
                                            "Failed" => "tb-err",
                                            _ => "tb-mod",
                                        };
                                        view! {
                                            <div class="tree-intent">
                                                <span>{intent.slug.clone()}</span>
                                                <span class=format!("tree-badge {status_class}")>
                                                    {intent.status}
                                                </span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Right: detail view / create form
            <div class="md-panel" style="flex:1">
                <div class="md-panel-header">
                    <div class="md-panel-tab active">"Detail"</div>
                </div>
                <div class="md-content">
                    <h1>"Create Intent"</h1>
                    <p>"Describe what you want to build in natural language. The system will generate a structured plan, execute it, and validate the result."</p>
                    <div style="margin-top:16px">
                        <textarea
                            class="cip-textarea"
                            id="intentDescription"
                            placeholder="Build a calculator with add, subtract, multiply, divide..."
                            rows="4"
                        ></textarea>
                        <div style="margin-top:12px;display:flex;gap:8px">
                            <button class="cip-btn cip-btn-create" id="createIntentBtn">
                                "Create & Plan"
                            </button>
                            <button class="cip-btn cip-btn-cancel" id="executeIntentBtn">
                                "Execute"
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
