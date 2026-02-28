# Workspace Layout

After `duumbi init myapp`, the workspace structure is:

```
myapp/
└── .duumbi/
    ├── config.toml          # LLM provider, model, API key env reference
    ├── schema/
    │   └── core.schema.json # JSON Schema for Op node validation
    ├── graph/
    │   └── main.jsonld      # Source of truth for program logic
    ├── build/
    │   ├── output.o         # Cranelift-compiled object file
    │   ├── duumbi_runtime.o # Compiled C runtime shim
    │   └── output           # Final linked executable
    ├── telemetry/
    │   └── traces.jsonl     # traceId → nodeId mapping (runtime)
    └── history/
        └── 000001.jsonld    # Undo snapshots (LIFO stack)
```

## config.toml

```toml
[llm]
provider = "anthropic"        # or "openai"
model = "claude-sonnet-4-6"
api_key_env = "ANTHROPIC_API_KEY"
```

Set the corresponding environment variable before using `duumbi add`:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

## graph/main.jsonld

The source of truth. Modified by `duumbi add`, restored by `duumbi undo`.
Never edit manually unless you understand the JSON-LD schema.

See [JSON-LD Format](../jsonld/overview.md) for the full format reference.
