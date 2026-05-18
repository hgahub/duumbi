# CLI Overview

DUUMBI is operated entirely through the `duumbi` command-line tool.

Run `duumbi` with no subcommand to open the interactive TUI. Query mode is the
default read-only mode for understanding a workspace before changing it. From
inside the TUI, use `/query <question>` or `/ask <question>` for one-shot
read-only questions from any mode.

Query examples:

```text
/query "what functions exist?"
/ask "where does main behavior live?"
```

Query answers may include metadata such as sources, confidence, model, and a
suggested handoff when Agent or Intent is the better write-capable path.

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
