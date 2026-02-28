# Quick Start

## 1. Create a workspace

```bash
duumbi init myapp
cd myapp
```

This creates a `.duumbi/` directory with a starter graph (`main.jsonld`) that
implements `add(3, 5)` and prints the result.

## 2. Build and run

```bash
duumbi build
duumbi run
# Output: 8
```

## 3. Add a function with AI

```bash
duumbi add "add a fibonacci function that works for i64"
# Confirms the change, then writes the updated graph
duumbi build && duumbi run
```

## 4. Inspect the graph

```bash
duumbi describe
# Prints pseudocode representation of the current graph
```

## 5. Visualize

```bash
duumbi viz
# Opens http://localhost:8420 — Cytoscape.js graph view
```

## 6. Undo a change

```bash
duumbi undo
# Restores the previous snapshot from .duumbi/history/
```
