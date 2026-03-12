//! Toast notification component.
//!
//! Displays temporary notifications in the top-right corner.
//! Toasts auto-dismiss after 5 seconds via `set_timeout`.

use leptos::prelude::*;

use crate::state::{StudioState, ToastKind, ToastMessage};

/// Adds a toast notification to the queue.
///
/// The toast will be auto-dismissed after 5 seconds.
pub fn push_toast(state: &StudioState, kind: ToastKind, text: String) {
    let id = state.toast_counter.get_untracked() + 1;
    state.toast_counter.set(id);

    let toast = ToastMessage { id, kind, text };

    state.toasts.update(|t| t.push(toast));

    // Auto-dismiss after 5 seconds
    let toasts_signal = state.toasts;
    set_timeout(
        move || {
            toasts_signal.update(|t| t.retain(|m| m.id != id));
        },
        std::time::Duration::from_secs(5),
    );
}

/// Toast notification container.
///
/// Renders active toast messages. Toasts auto-dismiss after 5 seconds.
#[component]
pub fn ToastContainer() -> impl IntoView {
    let state = expect_context::<StudioState>();

    view! {
        <div class="toast-container">
            {move || state.toasts.get().into_iter().map(|toast| {
                let kind_class = match toast.kind {
                    ToastKind::Success => "toast-success",
                    ToastKind::Error => "toast-error",
                    ToastKind::Info => "toast-info",
                };
                let toast_id = toast.id;
                let toasts_signal = state.toasts;
                view! {
                    <div class=format!("toast-item {kind_class}")>
                        <span class="toast-text">{toast.text}</span>
                        <button
                            class="toast-close"
                            on:click=move |_| {
                                toasts_signal.update(|t| t.retain(|m| m.id != toast_id));
                            }
                        >
                            "\u{2715}"
                        </button>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
