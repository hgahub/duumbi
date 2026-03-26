//! Command palette overlay (Cmd+K / Ctrl+K).
//!
//! Provides quick access to intents, graph nodes, providers, settings,
//! and other commands. Replaces the previous search overlay and shortcuts
//! overlay with a unified command interface.

use leptos::prelude::*;

use crate::state::StudioState;

/// Command palette overlay activated by Cmd+K.
///
/// Renders a search input with grouped results: Intents, Nodes, Commands.
/// The actual keyboard shortcut binding and filtering logic lives in
/// `studio.js` for SSR compatibility.
#[component]
pub fn CommandPalette() -> impl IntoView {
    let state = expect_context::<StudioState>();

    let close = move |_| {
        state.search_visible.set(false);
    };

    view! {
        <div class="cmd-backdrop" class:open=move || state.search_visible.get()
            on:click=close>
            <div class="cmd-palette" on:click=move |e| e.stop_propagation()>
                <div class="cmd-input-row">
                    <svg viewBox="0 0 16 16">
                        <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/>
                        <line x1="10.2" y1="10.2" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                    </svg>
                    <input
                        class="cmd-input"
                        id="cmdInput"
                        type="text"
                        placeholder="Search intents, nodes, commands..."
                        autocomplete="off"
                    />
                </div>
                <div class="cmd-results" id="cmdResults">
                    <div class="cmd-group" data-group="intents">
                        <div class="cmd-group-label">"INTENTS"</div>
                        // Populated by JS from state.intents
                    </div>
                    <div class="cmd-group" data-group="commands">
                        <div class="cmd-group-label">"COMMANDS"</div>
                        <div class="cmd-item" data-filter="new intent create">
                            <svg viewBox="0 0 14 14">
                                <line x1="7" y1="3" x2="7" y2="11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                                <line x1="3" y1="7" x2="11" y2="7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                            </svg>
                            <span>"New Intent"</span>
                            <span class="cmd-item-hint">"+"</span>
                        </div>
                        <div class="cmd-item" data-filter="build compile">
                            <svg viewBox="0 0 14 14">
                                <path d="M3 11L7 3L11 11H3Z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
                            </svg>
                            <span>"Build Project"</span>
                            <span class="cmd-item-hint">{"\u{2318}B"}</span>
                        </div>
                        <div class="cmd-item" data-filter="theme toggle dark light">
                            <svg viewBox="0 0 14 14">
                                <circle cx="7" cy="7" r="4" fill="none" stroke="currentColor" stroke-width="1.3"/>
                            </svg>
                            <span>"Toggle Theme"</span>
                        </div>
                        <div class="cmd-item" data-filter="provider settings llm model">
                            <svg viewBox="0 0 14 14">
                                <circle cx="7" cy="7" r="3" fill="none" stroke="currentColor" stroke-width="1.3"/>
                                <path d="M7 1v2M7 11v2M1 7h2M11 7h2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                            </svg>
                            <span>"Configure Providers"</span>
                        </div>
                        <div class="cmd-item" data-filter="registry search module">
                            <svg viewBox="0 0 14 14">
                                <rect x="2" y="2" width="10" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/>
                            </svg>
                            <span>"Registry Search"</span>
                        </div>
                    </div>
                </div>
                <div class="cmd-footer">
                    <span>{"\u{2191}\u{2193} navigate"}</span>
                    <span>{"\u{21B5} select"}</span>
                    <span>"esc close"</span>
                </div>
            </div>
        </div>
    }
}
