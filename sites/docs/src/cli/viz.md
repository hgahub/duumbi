# duumbi viz

Launch the web-based graph visualizer. Opens a local server with a
Cytoscape.js graph view that live-syncs with the workspace via WebSocket.

## Usage

```bash
duumbi viz
duumbi viz --port 9000   # Custom port
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | `8420` | HTTP server port |

## Features

- **Live sync**: Graph updates automatically when `.duumbi/graph/main.jsonld` changes
- **Node details**: Click a node to see its `@type`, `@id`, and all fields
- **Zoom/pan**: Standard Cytoscape.js controls

## Access

Open [http://localhost:8420](http://localhost:8420) in your browser after running `duumbi viz`.
