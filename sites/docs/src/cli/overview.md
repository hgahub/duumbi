# CLI Overview

DUUMBI is operated entirely through the `duumbi` command-line tool.

## Commands

| Command | Description |
|---------|-------------|
| [`duumbi init <name>`](init.md) | Create a new workspace |
| [`duumbi build`](build.md) | Compile the graph to a native binary |
| [`duumbi run [-- args]`](run.md) | Build (if needed) and run the binary |
| [`duumbi check`](check.md) | Validate the graph without compiling |
| [`duumbi describe`](describe.md) | Print pseudocode representation |
| [`duumbi add "<request>"`](add.md) | AI-driven graph mutation |
| [`duumbi undo`](undo.md) | Restore the previous graph snapshot |
| [`duumbi viz`](viz.md) | Launch the web visualizer |

## Global flags

```
--help       Print help
--version    Print version
```

## Error output

Structured errors are written to **stdout** as JSONL:

```json
{"level":"error","code":"E001","message":"...","nodeId":"duumbi:main/main/entry/2"}
```

Human-readable summaries go to **stderr**. Never mix the two streams.

See [Error Codes](../jsonld/errors.md) for the full list.
