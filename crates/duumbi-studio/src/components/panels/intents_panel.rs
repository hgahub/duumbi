//! Intents panel — unified create, review, and execute.
//!
//! Left side: intent list with status indicators.
//! Right side: selected intent detail or create form.
//! Buttons wired to `create_intent()` and `execute_intent()` server fns.

use leptos::prelude::*;

use crate::server_fns::{create_intent, execute_intent, get_intents};
use crate::state::StudioState;

/// Intents panel: the first stop in the development cycle.
///
/// Displays the intent list and detail view in a master-detail layout.
/// Create, review, and execute intents without leaving this panel.
#[component]
pub fn IntentsPanel() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let selected_intent = RwSignal::new(Option::<String>::None);
    let status_message = RwSignal::new(String::new());

    let create_action = Action::new(move |desc: &String| {
        let desc = desc.clone();
        async move { create_intent(desc).await }
    });

    let execute_action = Action::new(move |slug: &String| {
        let slug = slug.clone();
        async move { execute_intent(slug).await }
    });

    let refresh_intents = Action::new(move |_: &()| async move { get_intents().await });

    // React to create completion.
    Effect::new(move || {
        if let Some(result) = create_action.value().get() {
            match result {
                Ok(intent) => {
                    status_message.set(format!("Intent '{}' created.", intent.slug));
                    selected_intent.set(Some(intent.slug.clone()));
                    refresh_intents.dispatch(());
                }
                Err(e) => status_message.set(format!("Create failed: {e}")),
            }
        }
    });

    // React to execute completion.
    Effect::new(move || {
        if let Some(result) = execute_action.value().get() {
            match result {
                Ok(msg) => {
                    status_message.set(msg);
                    refresh_intents.dispatch(());
                }
                Err(e) => status_message.set(format!("Execute failed: {e}")),
            }
        }
    });

    // Update state intents when refresh completes.
    Effect::new(move || {
        if let Some(Ok(intents)) = refresh_intents.value().get() {
            state.intents.set(intents);
        }
    });

    let on_create = move |_| {
        // Read the textarea value from the DOM (browser-only).
        #[cfg(feature = "hydrate")]
        let desc = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("intentDescription"))
            .and_then(|el| {
                use wasm_bindgen::JsCast;
                el.dyn_into::<web_sys::HtmlTextAreaElement>().ok()
            })
            .map(|ta| ta.value())
            .unwrap_or_default();

        #[cfg(not(feature = "hydrate"))]
        let desc = String::new();

        if !desc.trim().is_empty() {
            status_message.set("Creating intent...".to_string());
            create_action.dispatch(desc);
        }
    };

    let on_execute = move |_| {
        if let Some(slug) = selected_intent.get() {
            status_message.set(format!("Executing '{slug}'..."));
            execute_action.dispatch(slug);
        }
    };

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
                                        let slug = intent.slug.clone();
                                        let slug_click = slug.clone();
                                        let is_selected = move || selected_intent.get().as_deref() == Some(&slug);
                                        let status_class = match intent.status.as_str() {
                                            "Completed" => "tb-fn",
                                            "Failed" => "tb-err",
                                            "InProgress" => "tb-mod",
                                            _ => "tb-mod",
                                        };
                                        view! {
                                            <div class="tree-intent"
                                                class:selected=is_selected
                                                on:click=move |_| selected_intent.set(Some(slug_click.clone()))
                                                style="cursor:pointer">
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
                            <button class="cip-btn cip-btn-create" on:click=on_create>
                                "Create & Plan"
                            </button>
                            <button class="cip-btn cip-btn-cancel" on:click=on_execute>
                                "Execute"
                            </button>
                        </div>
                        // Status message
                        {move || {
                            let msg = status_message.get();
                            if msg.is_empty() {
                                view! { <span></span> }.into_any()
                            } else {
                                view! {
                                    <p style="margin-top:12px;color:#908c82;font-size:13px">{msg}</p>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}
