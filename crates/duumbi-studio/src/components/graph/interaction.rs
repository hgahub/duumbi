//! Graph interaction handlers.
//!
//! Provides pan, zoom, and click event handling for the SVG graph canvas.

use leptos::prelude::*;
use std::sync::{Arc, Mutex};

/// Mutable state for pan/zoom interactions.
#[derive(Clone)]
pub struct InteractionState {
    inner: Arc<Mutex<InteractionInner>>,
}

struct InteractionInner {
    /// Whether the user is currently dragging (panning).
    dragging: bool,
    /// Last mouse position during drag.
    last_x: f64,
    last_y: f64,
    /// Current translation offset.
    translate_x: f64,
    translate_y: f64,
    /// Current zoom scale.
    scale: f64,
}

impl InteractionState {
    /// Creates a new interaction state with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InteractionInner {
                dragging: false,
                last_x: 0.0,
                last_y: 0.0,
                translate_x: 0.0,
                translate_y: 0.0,
                scale: 1.0,
            })),
        }
    }

    fn update_transform(&self, set_transform: &WriteSignal<String>) {
        let inner = self
            .inner
            .lock()
            .expect("invariant: interaction lock poisoned");
        set_transform.set(format!(
            "translate({},{}) scale({})",
            inner.translate_x, inner.translate_y, inner.scale
        ));
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a wheel event handler for zooming.
pub fn on_wheel(
    state: InteractionState,
    set_transform: WriteSignal<String>,
) -> impl Fn(web_sys::WheelEvent) + 'static {
    move |ev: web_sys::WheelEvent| {
        ev.prevent_default();
        let delta = ev.delta_y();
        let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };

        {
            let mut inner = state
                .inner
                .lock()
                .expect("invariant: interaction lock poisoned");
            inner.scale *= zoom_factor;
            inner.scale = inner.scale.clamp(0.1, 5.0);
        }

        state.update_transform(&set_transform);
    }
}

/// Creates a mousedown event handler for starting pan.
pub fn on_mousedown(state: InteractionState) -> impl Fn(web_sys::MouseEvent) + 'static {
    move |ev: web_sys::MouseEvent| {
        let mut inner = state
            .inner
            .lock()
            .expect("invariant: interaction lock poisoned");
        inner.dragging = true;
        inner.last_x = ev.client_x() as f64;
        inner.last_y = ev.client_y() as f64;
    }
}

/// Creates a mousemove event handler for panning.
pub fn on_mousemove(
    state: InteractionState,
    set_transform: WriteSignal<String>,
) -> impl Fn(web_sys::MouseEvent) + 'static {
    move |ev: web_sys::MouseEvent| {
        let is_dragging = {
            let inner = state
                .inner
                .lock()
                .expect("invariant: interaction lock poisoned");
            inner.dragging
        };

        if is_dragging {
            let x = ev.client_x() as f64;
            let y = ev.client_y() as f64;

            {
                let mut inner = state
                    .inner
                    .lock()
                    .expect("invariant: interaction lock poisoned");
                inner.translate_x += x - inner.last_x;
                inner.translate_y += y - inner.last_y;
                inner.last_x = x;
                inner.last_y = y;
            }

            state.update_transform(&set_transform);
        }
    }
}

/// Creates a mouseup event handler for ending pan.
pub fn on_mouseup(state: InteractionState) -> impl Fn(web_sys::MouseEvent) + 'static {
    move |_ev: web_sys::MouseEvent| {
        let mut inner = state
            .inner
            .lock()
            .expect("invariant: interaction lock poisoned");
        inner.dragging = false;
    }
}
