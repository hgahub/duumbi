//! Chat panel — Phase 15 redesign.
//!
//! Textarea-based chat connected to WebSocket for streaming LLM responses.
//! Model selector reads from configured providers. Streaming chunks
//! append to the current assistant message in real-time via JS.

use leptos::prelude::*;

use crate::state::{ChatMessage, ChatRole, StudioState};

/// Chat panel component for the Graph panel split view.
///
/// Contains a scrollable message list, a model selector dropdown,
/// and a textarea input. The actual WebSocket connection is managed
/// by `studio.js` (StudioWS module) — this component provides the DOM
/// structure that JS populates with streaming chunks.
#[component]
pub fn ChatPanel() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let (input_text, set_input_text) = signal(String::new());

    let do_send = {
        move || {
            let text = input_text.get();
            if text.trim().is_empty() {
                return;
            }

            // Add user message immediately
            state.chat_messages.update(|msgs| {
                msgs.push(ChatMessage {
                    role: ChatRole::User,
                    content: text.clone(),
                });
            });
            set_input_text.set(String::new());
            state.chat_streaming.set(true);

            // JS StudioWS.send() handles WebSocket communication.
            // The streaming response is rendered directly in the DOM by JS.
        }
    };
    let do_send = std::rc::Rc::new(do_send);

    let on_send = {
        let do_send = do_send.clone();
        move |_| do_send()
    };

    let on_keydown = {
        let do_send = do_send.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" && !ev.shift_key() {
                ev.prevent_default();
                do_send();
            }
        }
    };

    view! {
        // Chat header with model selector
        <div class="chat-panel-header">
            <div class="chat-panel-title">
                <svg viewBox="0 0 14 14">
                    <path d="M2 3h10v7H8l-3 2v-2H2z" fill="none" stroke="currentColor" stroke-width="1.4"
                        stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                "Chat"
            </div>
        </div>

        // Message list
        <div class="chat-messages" id="chatMessages">
            {move || state.chat_messages.get().into_iter().map(|msg| {
                let class = match msg.role {
                    ChatRole::User => "chat-msg user",
                    ChatRole::Assistant => "chat-msg ai",
                    ChatRole::System => "chat-msg ai",
                };
                view! {
                    <div class=class>
                        <span>{msg.content}</span>
                    </div>
                }
            }).collect_view()}
        </div>

        // Model selector area
        <div class="chat-model-area">
            <div class="chat-model" id="modelSelector">
                <span id="modelLabel">
                    {move || {
                        let p = state.selected_provider.get();
                        if p.is_empty() { "claude-sonnet-4-6".to_string() } else { p }
                    }}
                </span>
                <svg class="cm-chevron" viewBox="0 0 8 8">
                    <path d="M1 3L4 6L7 3" stroke="currentColor" stroke-width="1.3" fill="none"
                        stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
            </div>
            // Model dropdown populated by JS
            <div class="model-dropdown" id="modelDropdown"></div>
        </div>

        // Input area
        <div class="chat-input-area">
            <div class="chat-input-wrap">
                <textarea
                    class="chat-input"
                    id="chatInput"
                    rows="1"
                    placeholder="Describe what to build..."
                    prop:value=move || input_text.get()
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
                        set_input_text.set(target.value());
                    }
                    on:keydown=on_keydown
                ></textarea>
                <button class="chat-send" on:click=move |_| on_send(())>
                    <svg viewBox="0 0 14 14">
                        <path d="M1 7h12M8 2l5 5-5 5" stroke="currentColor" stroke-width="1.5"
                            fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                </button>
            </div>
        </div>
    }
}
