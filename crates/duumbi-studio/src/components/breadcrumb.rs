//! Breadcrumb navigation component.
//!
//! Shows the current C4 drill-down path: workspace > module > function > block.
//! Each segment is clickable to navigate back to that level.

use leptos::prelude::*;

use crate::state::{C4Level, StudioState};

/// Breadcrumb navigation showing the current drill-down path.
#[component]
pub fn Breadcrumb() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let on_workspace = move |_| {
        state.c4_level.set(C4Level::Context);
        state.selected_module.set(None);
        state.selected_function.set(None);
        state.selected_block.set(None);
    };

    let on_module = move |_| {
        state.c4_level.set(C4Level::Container);
        state.selected_function.set(None);
        state.selected_block.set(None);
    };

    let on_function = move |_| {
        state.c4_level.set(C4Level::Component);
        state.selected_block.set(None);
    };

    view! {
        <nav class="breadcrumb">
            <button class="breadcrumb-item" on:click=on_workspace>
                {move || state.workspace_name.get()}
            </button>

            {move || state.selected_module.get().map(|m| view! {
                <span class="breadcrumb-sep">" > "</span>
                <button class="breadcrumb-item" on:click=on_module>
                    {m}
                </button>
            })}

            {move || state.selected_function.get().map(|f| view! {
                <span class="breadcrumb-sep">" > "</span>
                <button class="breadcrumb-item" on:click=on_function>
                    {f}
                </button>
            })}

            {move || state.selected_block.get().map(|b| view! {
                <span class="breadcrumb-sep">" > "</span>
                <span class="breadcrumb-item current">{b}</span>
            })}
        </nav>
    }
}
