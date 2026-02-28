# DUUMBI

**DUUMBI** is an AI-first semantic graph compiler. Program logic lives in typed
JSON-LD graphs — not text files. AI generates and mutates the graph. The
toolchain validates every mutation before compilation. Syntax errors are
structurally impossible.

## Core idea

```
.jsonld graph  →  schema validation  →  Cranelift IR  →  native binary
                                              ↑
                                   AI proposes graph patches
```

The graph is the single source of truth. The compiler, validator, AI agent, and
web visualizer all operate on the same representation.

## Status

| Phase | Goal | Status |
|-------|------|--------|
| 0 | JSON-LD → native binary | ✅ Complete |
| 1 | Usable CLI (fibonacci, branching) | ✅ Complete |
| 2 | AI graph mutation (`duumbi add`) | ✅ Complete |
| 3 | Web visualizer (`duumbi viz`) | ✅ Complete |
| 4 | Interactive CLI & module system | 🔄 In progress |

## Quick install

```bash
cargo install duumbi
```

Then create your first workspace:

```bash
duumbi init myapp
cd myapp
duumbi build
duumbi run
```

## Getting help

- [Quick Start](getting-started/quickstart.md) — first program in 5 minutes
- [CLI Reference](cli/overview.md) — all commands
- [JSON-LD Format](jsonld/overview.md) — graph format reference
- [GitHub](https://github.com/hgahub/duumbi) — source code and issues
