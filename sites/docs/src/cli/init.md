# duumbi init

Create a new DUUMBI workspace.

## Usage

```bash
duumbi init <name>
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Workspace directory name to create |

## What it creates

```
<name>/
└── .duumbi/
    ├── config.toml       # LLM configuration
    ├── schema/
    │   └── core.schema.json
    └── graph/
        └── main.jsonld   # Starter program: add(3, 5) → prints 8
```

## Example

```bash
duumbi init myapp
cd myapp
duumbi build && duumbi run
# Output: 8
```
