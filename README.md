# DUUMBI

[![Coverage](https://codecov.io/gh/hgahub/duumbi/branch/main/graph/badge.svg)](https://codecov.io/gh/hgahub/duumbi)

> AI-first semantic graph compiler. Programs are stored as JSON-LD graphs — not text files. Describe a change in plain language; the toolchain validates, compiles, and links it to a native binary via Cranelift.

**[Website](https://www.duumbi.dev/) · [Docs](https://docs.duumbi.dev/) · [Registry](https://registry.duumbi.dev/)**

---

## How it works

Traditional compilers transform text → AST → machine code. DUUMBI skips the text entirely. Program logic lives as a typed semantic graph in JSON-LD format. An AI agent generates and mutates the graph; every mutation is validated before compilation. Syntax errors are structurally impossible.

```
.jsonld  →  Parser  →  Semantic Graph  →  Validator  →  Cranelift  →  binary
```

---

## Install

**Requirements:** Rust stable (1.80+), a C compiler on `$PATH` (`cc` / Xcode CLT / `gcc`).

```bash
git clone git@github.com:hgahub/duumbi.git
cd duumbi
cargo install --path .
```

---

## Quickstart

```bash
duumbi init myproject
cd myproject

duumbi build      # compile
duumbi run        # run the binary
duumbi check      # validate without compiling
duumbi describe   # human-readable pseudo-code
```

---

## AI Mutation

Configure your LLM provider in `.duumbi/config.toml`:

```toml
[llm]
provider = "anthropic"           # or "openai"
model = "claude-sonnet-4-6"      # or "gpt-4o"
api_key_env = "ANTHROPIC_API_KEY"
```

Then describe changes in plain language:

```bash
export ANTHROPIC_API_KEY=sk-ant-...

duumbi add "change the constant 3 to 7"
duumbi add --yes "multiply instead of add"
duumbi undo   # restore the previous graph
```

Supported providers: **Anthropic** (`claude-sonnet-4-6`, `claude-opus-4-6`) and **OpenAI** (`gpt-4o`, `gpt-4o-mini`).

Full reference → [docs.duumbi.dev](https://docs.duumbi.dev/)

---

## Build & test

```bash
cargo build
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt
```

---

## Supported platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | aarch64 (Apple Silicon) | Primary |
| macOS | x86_64 | CI-tested |
| Linux | x86_64 | CI-tested |
| Windows | — | Not supported |

---

## Community

- Read the [Code of Conduct](CODE_OF_CONDUCT.md) before opening issues or pull requests.
- Use the provided issue and PR templates.
