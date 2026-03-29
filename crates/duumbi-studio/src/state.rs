//! Studio application state.
//!
//! `StudioState` is provided as a Leptos context to all components.
//! It holds reactive signals for navigation, graph data, chat, and theme.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// C4 zoom level for the graph explorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum C4Level {
    /// C4 Context: software system + person + external systems.
    #[default]
    Context,
    /// C4 Container: native binary + runtime shim within system boundary.
    Container,
    /// C4 Component: active vs dead code functions + sub-components.
    Component,
    /// C4 Code: individual operations and data flow within a block.
    Code,
}

impl std::fmt::Display for C4Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Context => write!(f, "Context"),
            Self::Container => write!(f, "Container"),
            Self::Component => write!(f, "Component"),
            Self::Code => write!(f, "Code"),
        }
    }
}

/// Visual theme for the Studio UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    /// Dark theme (default) — GitHub dark-inspired palette.
    #[default]
    Dark,
    /// Light theme.
    Light,
}

impl Theme {
    /// Returns the CSS class name for the `<body>` element.
    #[must_use]
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Dark => "theme-dark",
            Self::Light => "theme-light",
        }
    }

    /// Toggles between Dark and Light.
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

/// Build status indicator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BuildStatus {
    /// No build in progress.
    #[default]
    Idle,
    /// Build is running.
    Building,
    /// Last build succeeded.
    Success,
    /// Last build failed with the given error message.
    Failed(String),
}

/// A single chat message in the Studio chat panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Who sent the message.
    pub role: ChatRole,
    /// Message content (may contain markdown).
    pub content: String,
}

/// The sender role for a chat message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRole {
    /// User input.
    User,
    /// AI response.
    Assistant,
    /// System notification (build result, error, etc.).
    System,
}

/// Summary of an intent for display in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSummary {
    /// Intent slug (filename without extension).
    pub slug: String,
    /// Human-readable description.
    pub description: String,
    /// Current status.
    pub status: String,
}

/// A node in the C4 graph view, serialized for the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier (e.g., module name, function id, block label, op id).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Node type: "module", "function", "block", or op type name.
    pub node_type: String,
    /// Optional badge text (e.g., function count).
    pub badge: Option<String>,
    /// Position assigned by layout algorithm.
    pub x: f64,
    /// Position assigned by layout algorithm.
    pub y: f64,
    /// Width for rendering.
    pub width: f64,
    /// Height for rendering.
    pub height: f64,
}

/// An edge in the C4 graph view, serialized for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique edge identifier.
    pub id: String,
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Edge label (e.g., "left", "right", "calls").
    pub label: String,
    /// Edge type for styling.
    pub edge_type: String,
}

/// Graph data for the current C4 level view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphData {
    /// Nodes at the current C4 level.
    pub nodes: Vec<GraphNode>,
    /// Edges at the current C4 level.
    pub edges: Vec<GraphEdge>,
}

/// A registry search result for display in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySearchHit {
    /// Scoped module name (e.g., `@duumbi/stdlib-math`).
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Latest published version.
    pub latest_version: String,
}

/// An installed dependency shown in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledDep {
    /// Dependency name.
    pub name: String,
    /// Version requirement or path.
    pub version: String,
    /// Source: "registry", "path", or "vendor".
    pub source: String,
}

/// Kind of toast notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToastKind {
    /// Success notification (green).
    Success,
    /// Error notification (red).
    Error,
    /// Informational notification (blue).
    Info,
}

/// A toast notification message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastMessage {
    /// Unique toast id (for removal).
    pub id: u32,
    /// Toast kind for styling.
    pub kind: ToastKind,
    /// Message text.
    pub text: String,
}

/// Which sidebar tab is active (legacy, kept for backward compat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SidebarTab {
    /// Module explorer.
    #[default]
    Explorer,
    /// Intent list.
    Intents,
    /// Registry search and installed deps.
    Registry,
}

/// Which main panel is active in the 3-panel layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActivePanel {
    /// Intent lifecycle: create, review, execute.
    #[default]
    Intents,
    /// C4 graph visualization + context-aware chat.
    Graph,
    /// Compile and run.
    Build,
}

/// Response from the chat server function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Human-readable summary of what the AI did.
    pub text: String,
    /// Node ids that were modified (for highlighting).
    pub changed_node_ids: Vec<String>,
}

/// Pre-loaded data for SSR initialization.
///
/// Loaded synchronously in `start_server` and provided as context so that
/// `App` can seed its signals with real data during SSR rendering.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct InitialData {
    /// Graph data for the Context level.
    pub graph: GraphData,
    /// Workspace name.
    pub workspace_name: String,
    /// Active intents.
    pub intents: Vec<IntentSummary>,
    /// Module list for the sidebar.
    pub modules: Vec<String>,
}

/// Central Studio state, provided as Leptos context.
#[derive(Clone, Copy)]
pub struct StudioState {
    /// Current C4 drill-down level.
    pub c4_level: RwSignal<C4Level>,
    /// Currently selected module (for Container+ views).
    pub selected_module: RwSignal<Option<String>>,
    /// Currently selected function (for Component+ views).
    pub selected_function: RwSignal<Option<String>>,
    /// Currently selected block (for Code view).
    pub selected_block: RwSignal<Option<String>>,
    /// Currently selected node id (for inspector).
    pub selected_node: RwSignal<Option<String>>,

    /// Graph data for the current view.
    pub graph_data: RwSignal<GraphData>,
    /// Node ids highlighted after AI mutation.
    pub highlighted_nodes: RwSignal<Vec<String>>,

    /// Chat message history.
    pub chat_messages: RwSignal<Vec<ChatMessage>>,
    /// Whether AI is currently streaming a response.
    pub chat_streaming: RwSignal<bool>,

    /// Workspace name.
    pub workspace_name: RwSignal<String>,
    /// Current build status.
    pub build_status: RwSignal<BuildStatus>,

    /// UI theme.
    pub theme: RwSignal<Theme>,

    /// Active and archived intents.
    pub intents: RwSignal<Vec<IntentSummary>>,

    /// Whether the sidebar is collapsed.
    pub sidebar_collapsed: RwSignal<bool>,

    /// Whether the keyboard shortcuts overlay is visible.
    pub shortcuts_visible: RwSignal<bool>,

    /// Whether the Ctrl+K search overlay is visible.
    pub search_visible: RwSignal<bool>,

    /// Active sidebar tab (legacy).
    pub sidebar_tab: RwSignal<SidebarTab>,

    /// Active main panel (Phase 15: 3-panel layout).
    pub active_panel: RwSignal<ActivePanel>,

    /// Selected LLM provider/model name for chat.
    pub selected_provider: RwSignal<String>,

    /// Whether WebSocket chat is connected.
    pub ws_connected: RwSignal<bool>,

    /// Registry search results.
    pub registry_results: RwSignal<Vec<RegistrySearchHit>>,

    /// Whether a registry search is in progress.
    pub registry_loading: RwSignal<bool>,

    /// Installed dependencies.
    pub installed_deps: RwSignal<Vec<InstalledDep>>,

    /// Toast notification queue.
    pub toasts: RwSignal<Vec<ToastMessage>>,

    /// Counter for unique toast ids.
    pub toast_counter: RwSignal<u32>,
}

impl StudioState {
    /// Creates a new `StudioState` seeded with pre-loaded data.
    ///
    /// Used during SSR to populate signals with real workspace data
    /// so the first render contains actual graph nodes, workspace name, etc.
    #[must_use]
    pub fn new_with_data(data: &InitialData) -> Self {
        Self {
            c4_level: RwSignal::new(C4Level::Context),
            selected_module: RwSignal::new(None),
            selected_function: RwSignal::new(None),
            selected_block: RwSignal::new(None),
            selected_node: RwSignal::new(None),
            graph_data: RwSignal::new(data.graph.clone()),
            highlighted_nodes: RwSignal::new(Vec::new()),
            chat_messages: RwSignal::new(Vec::new()),
            chat_streaming: RwSignal::new(false),
            workspace_name: RwSignal::new(data.workspace_name.clone()),
            build_status: RwSignal::new(BuildStatus::Idle),
            theme: RwSignal::new(Theme::Dark),
            intents: RwSignal::new(data.intents.clone()),
            sidebar_collapsed: RwSignal::new(false),
            shortcuts_visible: RwSignal::new(false),
            search_visible: RwSignal::new(false),
            sidebar_tab: RwSignal::new(SidebarTab::Explorer),
            active_panel: RwSignal::new(ActivePanel::Intents),
            selected_provider: RwSignal::new(String::new()),
            ws_connected: RwSignal::new(false),
            registry_results: RwSignal::new(Vec::new()),
            registry_loading: RwSignal::new(false),
            installed_deps: RwSignal::new(Vec::new()),
            toasts: RwSignal::new(Vec::new()),
            toast_counter: RwSignal::new(0),
        }
    }

    /// Creates a new `StudioState` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            c4_level: RwSignal::new(C4Level::Context),
            selected_module: RwSignal::new(None),
            selected_function: RwSignal::new(None),
            selected_block: RwSignal::new(None),
            selected_node: RwSignal::new(None),
            graph_data: RwSignal::new(GraphData::default()),
            highlighted_nodes: RwSignal::new(Vec::new()),
            chat_messages: RwSignal::new(Vec::new()),
            chat_streaming: RwSignal::new(false),
            workspace_name: RwSignal::new(String::new()),
            build_status: RwSignal::new(BuildStatus::Idle),
            theme: RwSignal::new(Theme::Dark),
            intents: RwSignal::new(Vec::new()),
            sidebar_collapsed: RwSignal::new(false),
            shortcuts_visible: RwSignal::new(false),
            search_visible: RwSignal::new(false),
            sidebar_tab: RwSignal::new(SidebarTab::Explorer),
            active_panel: RwSignal::new(ActivePanel::Intents),
            selected_provider: RwSignal::new(String::new()),
            ws_connected: RwSignal::new(false),
            registry_results: RwSignal::new(Vec::new()),
            registry_loading: RwSignal::new(false),
            installed_deps: RwSignal::new(Vec::new()),
            toasts: RwSignal::new(Vec::new()),
            toast_counter: RwSignal::new(0),
        }
    }
}

impl Default for StudioState {
    fn default() -> Self {
        Self::new()
    }
}
