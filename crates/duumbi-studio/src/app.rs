//! Root application component — Phase 15 design-aligned SSR shell.
//!
//! Renders HTML that structurally matches `design/duumbi_studio.html`.
//! All interactivity is via `studio.js` (onclick attrs → `window.__studio.*`).
//! Leptos `{move || ...}` is used only for SSR-time dynamic data injection
//! (workspace name, intents list).

use leptos::prelude::*;
use leptos_meta::*;

use crate::state::{InitialData, StudioState};

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let initial = use_context::<InitialData>().unwrap_or_default();
    let state = StudioState::new_with_data(&initial);
    provide_context(state);

    // Pre-render intent tree items for SSR.
    let intent_items = initial
        .intents
        .iter()
        .map(|intent| {
            let slug = intent.slug.clone();
            let slug_id = slug.replace(' ', "-").to_lowercase();
            let intent_id = format!("intent{}", capitalize_first(&slug_id));
            let children_id = format!("children-{slug_id}");
            let onclick_toggle = format!("window.__studio.toggleIntent('{slug_id}')");
            (intent.slug.clone(), intent_id, children_id, onclick_toggle)
        })
        .collect::<Vec<_>>();

    view! {
        <head>
            <Title text="DUUMBI Studio" />
            <Link rel="stylesheet" href="/studio.css" />
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
        </head>
        <body>

        // ── Header ──
        <header>
            <div class="workspace" onclick="window.__studio.openPopup('workspace')">
                <span class="workspace-name">{move || state.workspace_name.get()}</span>
                <svg class="workspace-chevron" viewBox="0 0 12 12" fill="none">
                    <path d="M3 4.5L6 7.5L9 4.5" stroke="#e8e4d9" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
            </div>
            <div class="header-right">
                <div class="header-toggle" id="headerToggle" onclick="window.__studio.toggleSidebarHeader()" title="Toggle sidebar">
                    <svg viewBox="0 0 16 16"><rect x="1" y="2" width="14" height="12" rx="2"/><line x1="5.5" y1="2" x2="5.5" y2="14"/></svg>
                </div>
                <div class="header-search" onclick="window.__studio.openSearch()" title="Search">
                    <svg viewBox="0 0 16 16"><circle cx="7" cy="7" r="4.5"/><line x1="10.2" y1="10.2" x2="14" y2="14"/></svg>
                    <span class="search-hotkey">{"\u{2318}K"}</span>
                </div>
            </div>
        </header>

        // ── Icon Rail ──
        <div class="icon-rail" id="iconRail">
            <button class="rail-btn" onclick="window.__studio.toggleExplorer()">
                <svg viewBox="0 0 18 18"><rect x="2" y="2" width="5.5" height="5.5" rx="1.2"/><rect x="10.5" y="2" width="5.5" height="5.5" rx="1.2"/><rect x="2" y="10.5" width="5.5" height="5.5" rx="1.2"/><rect x="10.5" y="10.5" width="5.5" height="5.5" rx="1.2"/></svg>
            </button>
        </div>

        // ── Sidebar ──
        <div class="sidebar" id="sidebar">
            <div class="sidebar-inner" id="sidebarInner">
                <div class="sidebar-resize" id="sidebarResize"></div>
                <div class="sidebar-header">
                    <span class="sidebar-title" id="sidebarTitle">"Explorer"</span>
                    <div class="sidebar-actions">
                        <div class="sidebar-pin" id="pinBtn" onclick="window.__studio.togglePin()" title="Pin sidebar">
                            <svg viewBox="0 0 14 14"><path d="M5 1.5l4 2.5v3l1.5 1.5v1.5h-3.5V13l-1 1-1-1V10H1.5V8.5L3 7V4z"/></svg>
                        </div>
                        <div class="sidebar-close" onclick="window.__studio.closeSidebar()">{"\u{2715}"}</div>
                    </div>
                </div>
                <div class="sidebar-scroll">
                    // Intents page
                    <div class="sb-page" id="page-intents">
                        <div class="sidebar-section">
                            <div class="section-header">
                                <div class="section-label">"Workspace"</div>
                                <div class="section-create" title="Create intent" onclick="window.__studio.openCreateIntent(event)">
                                    <svg viewBox="0 0 12 12"><line x1="6" y1="2" x2="6" y2="10"/><line x1="2" y1="6" x2="10" y2="6"/></svg>
                                </div>
                            </div>
                            // Dynamic intent tree from SSR data
                            {intent_items.into_iter().map(|(slug, intent_id, children_id, onclick)| {
                                view! {
                                    <div class="tree-intent" id=intent_id onclick=onclick.clone()>
                                        <svg class="intent-chevron" viewBox="0 0 10 10">
                                            <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                                        </svg>
                                        <svg class="tree-icon" viewBox="0 0 12 12" style="opacity:.8">
                                            <circle cx="6" cy="6" r="5" stroke="currentColor"/><circle cx="6" cy="6" r="2" stroke="currentColor"/>
                                        </svg>
                                        <span>{slug}</span>
                                    </div>
                                    <div class="tree-children" id=children_id>
                                        <div class="tree-child" onclick="window.__studio.selectC4('context')"><span class="child-dot" style="background:#6fd8b2"></span>"Context"<span class="tree-badge tb-fn" style="margin-left:auto">"C4"</span></div>
                                        <div class="tree-child" onclick="window.__studio.selectC4('container')"><span class="child-dot" style="background:#9ac4ef"></span>"Container"<span class="tree-badge tb-mod" style="margin-left:auto">"C4"</span></div>
                                        <div class="tree-child" onclick="window.__studio.selectC4('component')"><span class="child-dot" style="background:#e07830"></span>"Component"<span class="tree-badge" style="margin-left:auto;background:#352618;color:#e07830">"C4"</span></div>
                                        <div class="tree-child" onclick="window.__studio.selectC4('code')"><span class="child-dot" style="background:#c25a1a"></span>"Code"<span class="tree-badge" style="margin-left:auto;background:#351a1a;color:#f09090">"C4"</span></div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                    // Plans page (placeholder)
                    <div class="sb-page" id="page-plans">
                        <div class="sidebar-section"><div class="section-label">"Plans"</div>
                        <div style="padding:12px 8px;color:#4a4845;font-size:11px;font-family:'JetBrains Mono',monospace;text-align:center">"— coming soon —"</div></div>
                    </div>
                    // Build page (placeholder)
                    <div class="sb-page" id="page-build">
                        <div class="sidebar-section"><div class="section-label">"Build"</div>
                        <div style="padding:12px 8px;color:#4a4845;font-size:11px;font-family:'JetBrains Mono',monospace;text-align:center">"— coming soon —"</div></div>
                    </div>
                    // Agents page (placeholder)
                    <div class="sb-page" id="page-agents">
                        <div class="sidebar-section"><div class="section-label">"Agents"</div>
                        <div style="padding:12px 8px;color:#4a4845;font-size:11px;font-family:'JetBrains Mono',monospace;text-align:center">"— coming soon —"</div></div>
                    </div>
                    // Registry page (placeholder)
                    <div class="sb-page" id="page-registry">
                        <div class="sidebar-section"><div class="section-label">"Registry"</div>
                        <div style="padding:12px 8px;color:#4a4845;font-size:11px;font-family:'JetBrains Mono',monospace;text-align:center">"— coming soon —"</div></div>
                    </div>
                </div>
            </div>
        </div>

        // ── Canvas ──
        <div class="canvas" id="canvas">
            <div class="workspace-view" id="workspaceView">
                // MD Panel (intent documentation / graph visualization)
                <div class="md-panel" id="mdPanel">
                    <div class="md-panel-header">
                        <div class="md-panel-tab active">
                            <svg viewBox="0 0 12 12"><path d="M2 1h8a1 1 0 011 1v8a1 1 0 01-1 1H2a1 1 0 01-1-1V2a1 1 0 011-1z" stroke="currentColor" fill="none"/><path d="M3.5 4L5 6l-1.5 2M6.5 7h2" stroke="currentColor" fill="none" stroke-linecap="round"/></svg>
                            {move || {
                                let ws = state.workspace_name.get();
                                if ws.is_empty() { "workspace".to_string() } else { format!("{ws}.md") }
                            }}
                        </div>
                    </div>
                    <div class="md-content" id="mdContent">
                        <h1>"Welcome to DUUMBI Studio"</h1>
                        <p>"Select an intent from the sidebar or create a new one to get started."</p>
                        <p>"Use the footer navigation to switch between Intents, Plans, Build, Agents, and Registry views."</p>
                    </div>
                </div>

                // Split resize handle
                <div class="split-resize" id="splitResize"></div>

                // Chat Panel
                <div class="chat-panel" id="chatPanel">
                    <div class="chat-panel-header">
                        <div class="chat-panel-title">
                            <svg viewBox="0 0 14 14"><path d="M7 1a6 6 0 100 12 6 6 0 000-12z"/><circle cx="5" cy="6" r="1" fill="#c25a1a"/><circle cx="9" cy="6" r="1" fill="#c25a1a"/><path d="M5 9c.5.8 1.2 1 2 1s1.5-.2 2-1" stroke="#c25a1a"/></svg>
                            "AI Chat"
                        </div>
                        <div class="chat-mode-tabs" role="tablist" aria-label="Chat mode">
                            <button class="chat-mode-tab active" data-mode="query" onclick="window.__studio.setChatMode('query')" title="Read-only answers">"Query"</button>
                            <button class="chat-mode-tab" data-mode="agent" onclick="window.__studio.setChatMode('agent')" title="Apply graph changes">"Agent"</button>
                            <button class="chat-mode-tab" data-mode="intent" onclick="window.__studio.setChatMode('intent')" title="Use the intent workflow">"Intent"</button>
                        </div>
                    </div>
                    <div class="chat-messages" id="chatMessages">
                        <div class="chat-msg ai">
                            "Welcome! I can help you build, modify, and understand your semantic graph. Select an intent or ask me anything."
                            <div class="msg-meta">"DUUMBI AI"</div>
                        </div>
                    </div>
                    <div class="chat-input-area">
                        <div class="chat-input-wrap">
                            <textarea class="chat-input" id="chatInput" rows="1" placeholder="Ask about this graph..." onkeydown="window.__studio.handleChatKey(event)"></textarea>
                            <button class="chat-send" onclick="window.__studio.sendChat()">
                                <svg viewBox="0 0 14 14"><path d="M1 7h12M8 2l5 5-5 5"/></svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        // ── Footer ──
        <div class="footer">
            <div class="footer-avatar" id="avatarBtn" onclick="window.__studio.toggleUserMenu(event)">
                <span class="avatar-initials">"D"</span>
                <span class="avatar-status"></span>
            </div>
            <div class="footer-center">
                <div class="footer-item" data-fn="intents" onclick="window.__studio.toggleFunction('intents')">
                    <svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="6"/><circle cx="8" cy="8" r="2.5"/><line x1="8" y1="2" x2="8" y2="4"/><line x1="8" y1="12" x2="8" y2="14"/><line x1="2" y1="8" x2="4" y2="8"/><line x1="12" y1="8" x2="14" y2="8"/></svg>
                    <span class="footer-label">"Intents"</span>
                </div>
                <div class="footer-item" data-fn="plans" onclick="window.__studio.toggleFunction('plans')">
                    <svg viewBox="0 0 16 16"><rect x="3" y="2" width="10" height="12" rx="1.5"/><line x1="6" y1="5.5" x2="11" y2="5.5"/><line x1="6" y1="8" x2="11" y2="8"/><line x1="6" y1="10.5" x2="9" y2="10.5"/></svg>
                    <span class="footer-label">"Plans"</span>
                </div>
                <div class="footer-item" data-fn="build" onclick="window.__studio.toggleFunction('build')">
                    <svg viewBox="0 0 16 16"><polygon points="5,2 13,8 5,14"/></svg>
                    <span class="footer-label">"Build"</span>
                </div>
                <div class="footer-item" data-fn="agents" onclick="window.__studio.toggleFunction('agents')">
                    <svg viewBox="0 0 16 16"><circle cx="8" cy="5" r="3"/><path d="M3 14c0-2.8 2.2-5 5-5s5 2.2 5 5"/><circle cx="12.5" cy="3.5" r="1.5"/><line x1="12.5" y1="5" x2="12.5" y2="7"/><line x1="11" y1="6" x2="14" y2="6"/></svg>
                    <span class="footer-label">"Agents"</span>
                </div>
                <div class="footer-item" data-fn="registry" onclick="window.__studio.toggleFunction('registry')">
                    <svg viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="2"/><line x1="2" y1="6" x2="14" y2="6"/><line x1="2" y1="10" x2="14" y2="10"/><line x1="6" y1="6" x2="6" y2="14"/></svg>
                    <span class="footer-label">"Registry"</span>
                </div>
            </div>
        </div>

        // ── User Menu ──
        <div class="user-menu" id="userMenu">
            <div class="um-header">
                <div class="um-avatar"><span>"D"</span></div>
                <div class="um-info">
                    <div class="um-name">"Developer"</div>
                    <div class="um-email">"dev@duumbi.dev"</div>
                </div>
            </div>
            <div class="um-divider"></div>
            <div class="um-item" onclick="window.__studio.closeUserMenu()">
                <svg viewBox="0 0 16 16"><circle cx="8" cy="5" r="3" stroke="currentColor" fill="none" stroke-width="1.3"/><path d="M3 14c0-2.8 2.2-5 5-5s5 2.2 5 5" stroke="currentColor" fill="none" stroke-width="1.3"/></svg>
                "Profile"
            </div>
            <div class="um-item" id="settingsBtn" onclick="window.__studio.openSettings()">
                <svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="5.5" stroke="currentColor" fill="none" stroke-width="1.3"/><path d="M8 5.5v5M5.5 8h5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
                "Settings"
            </div>
            <div class="um-divider"></div>
            <div class="um-item" onclick="window.__studio.closeUserMenu()">
                <svg viewBox="0 0 16 16"><path d="M6 3h6a2 2 0 012 2v6a2 2 0 01-2 2H6" stroke="currentColor" fill="none" stroke-width="1.3" stroke-linecap="round"/><path d="M9 8H2m0 0l2.5-2.5M2 8l2.5 2.5" stroke="currentColor" fill="none" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
                "Sign out"
            </div>
        </div>

        // ── Settings Popup ──
        <div class="settings-backdrop" id="settingsBackdrop" onclick="window.__studio.closeSettings()">
            <div class="settings-popup" onclick="event.stopPropagation()">
                <div class="settings-header">
                    <span class="settings-title">"Settings"</span>
                    <div class="settings-close" onclick="window.__studio.closeSettings()">{"\u{2715}"}</div>
                </div>
                <div class="settings-body">
                    <div class="settings-sidebar">
                        <div class="settings-nav-item active" data-section="providers">"Providers"</div>
                    </div>
                    <div class="settings-main" id="settingsMain">
                        // Populated by JS openSettings()
                        <div style="padding:20px;color:#5a5855;font-family:'JetBrains Mono',monospace;font-size:11px">"Loading providers..."</div>
                    </div>
                </div>
                <div class="settings-footer">
                    <span class="settings-error" id="settingsError"></span>
                    <button class="cip-btn cip-btn-create" id="settingsSaveBtn" onclick="window.__studio.saveProviders()">"Save"</button>
                </div>
            </div>
        </div>

        // ── Create Intent Popup ──
        <div class="cip-backdrop" id="cipBackdrop" onclick="window.__studio.closeCreateIntent()">
            <div class="cip-popup" onclick="event.stopPropagation()" style="width:520px">
                <div class="cip-header">
                    <span class="cip-title">"New Intent"</span>
                    <div class="cip-close" onclick="window.__studio.closeCreateIntent()">{"\u{2715}"}</div>
                </div>
                <div class="cip-body">
                    <div class="cip-label">"Describe what you want to build"</div>
                    <textarea class="cip-textarea" id="cipIntent" rows="5" style="min-height:120px"
                        placeholder="Build a calculator with add, subtract, multiply, divide functions that work on i64 numbers"
                        oninput="window.__studio.validateCip()"></textarea>
                </div>
                <div class="cip-footer">
                    <button class="cip-btn cip-btn-cancel" onclick="window.__studio.closeCreateIntent()">"Cancel"</button>
                    <button class="cip-btn cip-btn-create" id="cipCreateBtn" disabled onclick="window.__studio.createNewIntent()">"Create"</button>
                </div>
            </div>
        </div>

        // ── Command Palette ──
        <div class="cmd-backdrop" id="cmdBackdrop" onclick="window.__studio.closeCmdIfOutside(event)">
            <div class="cmd-palette">
                <div class="cmd-input-row">
                    <svg viewBox="0 0 16 16"><circle cx="7" cy="7" r="4.5"/><line x1="10.2" y1="10.2" x2="14" y2="14"/></svg>
                    <input class="cmd-input" id="cmdInput" type="text" placeholder="Search nodes, intents, commands..." autocomplete="off" oninput="window.__studio.filterCmd()"/>
                </div>
                <div class="cmd-results" id="cmdResults">
                    <div class="cmd-group-label">"Intents"</div>
                    // Dynamic intent entries from SSR
                    {initial.intents.iter().map(|intent| {
                        let slug = intent.slug.clone();
                        let filter = format!("{} {}", slug, intent.description);
                        let onclick = format!("window.__studio.closeSearch();window.__studio.selectIntent('{}')", slug.replace(' ', "-").to_lowercase());
                        view! {
                            <div class="cmd-item" data-filter=filter onclick=onclick>
                                <svg viewBox="0 0 14 14"><circle cx="7" cy="7" r="5" fill="none"/><circle cx="7" cy="7" r="2" fill="none"/></svg>
                                {slug}
                                <span class="cmd-item-hint">"intent"</span>
                            </div>
                        }
                    }).collect_view()}
                    <div class="cmd-group-label">"Nodes"</div>
                    // Populated dynamically by JS after graph loads
                    <div class="cmd-group-label">"Commands"</div>
                    <div class="cmd-item" data-filter="new intent create" onclick="window.__studio.closeSearch();window.__studio.openCreateIntent()">
                        <svg viewBox="0 0 14 14"><line x1="7" y1="3" x2="7" y2="11"/><line x1="3" y1="7" x2="11" y2="7"/></svg>
                        "New Intent"
                    </div>
                    <div class="cmd-item" data-filter="build compile" onclick="window.__studio.closeSearch()">
                        <svg viewBox="0 0 14 14"><polygon points="4,2 12,7 4,12" fill="none"/></svg>
                        "Build Project"
                        <span class="cmd-item-hint">{"\u{2318}B"}</span>
                    </div>
                </div>
                <div class="cmd-footer">
                    <span><kbd>{"\u{2191}\u{2193}"}</kbd>" navigate"</span>
                    <span><kbd>{"\u{21B5}"}</kbd>" select"</span>
                    <span><kbd>"esc"</kbd>" close"</span>
                </div>
            </div>
        </div>

        // ── General Backdrop + Workspace Overlay ──
        <div class="backdrop" id="backdrop" onclick="window.__studio.closeAllPopups()"></div>
        <div class="overlay" id="popup-workspace">
            <div class="popup">
                <div class="popup-header">
                    <span class="popup-title">"Workspace"</span>
                    <div class="popup-close" onclick="window.__studio.closeAllPopups()">{"\u{2715}"}</div>
                </div>
                <div class="popup-body">"— workspace switcher coming soon —"</div>
            </div>
        </div>

        <script src="/studio.js"></script>
        </body>
    }
}

/// Capitalizes the first character of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
