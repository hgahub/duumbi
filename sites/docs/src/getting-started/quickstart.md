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

## 3. Ask read-only questions

Before asking DUUMBI to change the graph, use the interactive TUI's Query mode
to inspect the current workspace. Query is read-only: it can explain what exists,
where behavior lives, and when a request should hand off to Agent or Intent.

Query uses your configured LLM provider. The interactive TUI includes the
`/provider` setup flow, so if no provider is configured yet, start `duumbi` and
run `/provider` before asking Query questions.

After provider setup, start or return to the TUI:

```bash
duumbi
```

Inside the TUI, Query is the default mode. Use slash commands for one-shot
questions from any mode:

```text
/query "what functions exist?"
/ask "where does main behavior live?"
```

Answers include metadata such as sources, confidence, model, and suggested
handoff when a write-capable mode is more appropriate.

## 4. Add a function with AI

```bash
duumbi add "add a fibonacci function that works for i64"
# Confirms the change, then writes the updated graph
duumbi build && duumbi run
```

## 5. Inspect the graph

```bash
duumbi describe
# Prints pseudocode representation of the current graph
```

## 6. Undo a change

```bash
duumbi undo
# Restores the previous snapshot from .duumbi/history/
```
