//! Chat panel components.
//!
//! Provides the AI chat interface at the bottom of the Studio.

use leptos::prelude::*;

use crate::state::{ChatMessage, ChatRole, StudioState};

/// Bottom chat panel component.
///
/// Contains a scrollable message list and an input area.
#[component]
pub fn ChatPanel() -> impl IntoView {
    let state = expect_context::<StudioState>();
    let (input_text, set_input_text) = signal(String::new());

    let on_send = move |_| {
        let text = input_text.get();
        if text.trim().is_empty() {
            return;
        }

        // Add user message
        state.chat_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: ChatRole::User,
                content: text.clone(),
            });
        });

        set_input_text.set(String::new());

        // TODO: Send to server function for LLM processing
    };

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            on_send(());
        }
    };

    view! {
        <div class="chat-panel">
            <div class="chat-header">
                <span class="chat-title">"Chat"</span>
                <span class="chat-model">"claude-sonnet-4-6"</span>
            </div>

            <div class="chat-messages">
                {move || state.chat_messages.get().into_iter().map(|msg| {
                    let class = match msg.role {
                        ChatRole::User => "chat-msg user",
                        ChatRole::Assistant => "chat-msg assistant",
                        ChatRole::System => "chat-msg system",
                    };
                    let prefix = match msg.role {
                        ChatRole::User => "You: ",
                        ChatRole::Assistant => "AI: ",
                        ChatRole::System => "",
                    };
                    view! {
                        <div class=class>
                            <span class="msg-prefix">{prefix}</span>
                            <span class="msg-content">{msg.content}</span>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>

            <div class="chat-input-area">
                <input
                    type="text"
                    class="chat-input"
                    placeholder="Type a message or /command..."
                    prop:value=move || input_text.get()
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        set_input_text.set(target.value());
                    }
                    on:keydown=on_keydown
                />
                <button class="chat-send" on:click=move |_| on_send(())>
                    "Send"
                </button>
            </div>
        </div>
    }
}
