//! Toast notification component.
//!
//! Displays temporary notifications in the top-right corner.

use leptos::prelude::*;

/// Toast notification container.
///
/// Renders active toast messages. Toasts auto-dismiss after 5 seconds.
#[component]
pub fn ToastContainer() -> impl IntoView {
    // Toast system will be implemented with a signal-based queue.
    // For now, render an empty container as the mount point.
    view! {
        <div class="toast-container"></div>
    }
}
