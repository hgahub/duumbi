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

            let _state2 = state;
            #[cfg(feature = "hydrate")]
            let state2 = _state2;
            #[cfg(feature = "hydrate")]
            leptos::task::spawn_local(async move {
                use crate::server_fns::send_chat_message;
                match send_chat_message(text).await {
                    Ok(response) => {
                        state2.chat_messages.update(|msgs| {
                            msgs.push(ChatMessage {
                                role: ChatRole::Assistant,
                                content: response.text,
                            });
                        });
                        if !response.changed_node_ids.is_empty() {
                            state2.highlighted_nodes.set(response.changed_node_ids);
                        }
                        // Reload graph data after mutation
                        if let Ok(data) = crate::server_fns::get_graph_context().await {
                            state2.graph_data.set(data);
                        }
                    }
                    Err(e) => {
                        state2.chat_messages.update(|msgs| {
                            msgs.push(ChatMessage {
                                role: ChatRole::System,
                                content: format!("Error: {e}"),
                            });
                        });
                    }
                }
                state2.chat_streaming.set(false);
            });
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
