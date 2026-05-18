# duumbi studio

Start the DUUMBI Studio — a browser-based developer cockpit for exploring and editing your semantic graph workspace.

```
duumbi studio [--port <PORT>]
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--port <PORT>` | `8421` | TCP port to listen on |

## Description

`duumbi studio` starts an SSR (Server-Side Rendering) web server powered by
[Leptos](https://leptos.dev) and Axum. Open your browser at
`http://localhost:8421` to access the Studio UI.

The Studio provides:

- **C4 Drill-Down Graph** — four zoom levels: Context (modules), Container
  (functions), Component (blocks), Code (operations). Double-click any node
  to drill down; use breadcrumbs to navigate back.
- **AI Chat Panel** — Query is the default read-only chat mode for asking what
  exists, where behavior lives, and what risk a change may carry. Query answers
  show metadata such as sources, confidence, model, and suggested handoff when
  available. Switch to Agent mode only when you want a bounded graph mutation;
  Agent changes are applied through the LLM pipeline and snapshotted for undo.
- **Quick Search** — press the Search button (or `Ctrl+K`) to fuzzy-search
  nodes by id, label, or type.
- **Sidebar** — File Explorer shows all modules; Intents panel shows active
  and archived intents.
- **Dark / Light Theme** — toggle with the ☀/🌙 button in the header.
- **Keyboard Shortcuts** — press `?` to see all shortcuts.

## Requirements

- A duumbi workspace must exist in the current directory (run `duumbi init`
  first).
- A provider must be available in the effective configuration for AI chat
  features, including Query answers and Agent mutations. Workspace `[[providers]]`
  configuration is recommended; user-level config and legacy `[llm]` config are
  also supported.

## Example

```sh
# In your workspace directory
duumbi studio

# Or on a custom port
duumbi studio --port 9000
```

Then open `http://localhost:8421` in your browser.

## Architecture

The Studio is implemented as a separate Rust crate (`crates/duumbi-studio`)
using Leptos 0.8 for SSR + WASM hydration. The main `duumbi` binary launches
the Studio server directly.

Key components:

| Component | Description |
|-----------|-------------|
| `GraphCanvas` | SVG-based graph with pan/zoom and click handlers |
| `Sugiyama layout` | Layered layout algorithm for DAG positioning |
| `Orthogonal edge routing` | L-shaped edge paths between nodes |
| `server_fns.rs` | Leptos server functions bridging UI to workspace data |
| `ChatPanel` | AI chat that calls `orchestrator::mutate` via server fn |
| `SearchOverlay` | Ctrl+K fuzzy search over graph nodes |

## Standalone Binary

The `duumbi-studio` crate also provides a standalone binary for direct use:

```sh
cargo run -p duumbi-studio --features ssr --bin studio -- \
  --workspace /path/to/workspace --port 8421
```

## See Also

- [`duumbi viz`](viz.md) — lightweight Phase 3 graph visualizer (Cytoscape.js)
- [`duumbi intent`](../intent/overview.md) — manage development intents
