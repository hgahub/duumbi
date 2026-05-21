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

**Requirements:**

- Rust stable 1.80+ through `rustup`.
- macOS: Xcode Command Line Tools or an equivalent C compiler/linker.
- Linux: `build-essential` or an equivalent C compiler/linker.
- Windows native: Windows 10 version 1903+ on `x86_64-pc-windows-msvc`, the stable MSVC Rust toolchain, Visual Studio Build Tools or equivalent MSVC C++ tools, Windows SDK, and a usable linker/C compiler environment.

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

## Query First

Use DUUMBI's interactive mode to ask read-only questions before requesting graph
changes. Query mode inspects the workspace and can explain what exists, where
behavior lives, and when a request should hand off to Agent or Intent.

Query requires an available LLM provider. Configure one in `.duumbi/config.toml`;
DUUMBI selects concrete models internally per task and agent.

```toml
[[providers]]
provider = "anthropic"           # or "openai"
role = "primary"
api_key_env = "ANTHROPIC_API_KEY"
```

Then start the interactive TUI:

```bash
export ANTHROPIC_API_KEY=sk-ant-...

duumbi
```

Inside the TUI, Query is the default read-only mode. Use slash commands for
one-shot questions from any mode:

```text
/query "what functions exist?"
/ask "where does main behavior live?"
```

Query answers include metadata such as sources, confidence, model, and suggested
handoff when a write-capable Agent or Intent request is more appropriate.

---

## AI Mutation

After you understand the workspace, use Agent mode or `duumbi add` for bounded
write-capable graph changes:

```bash
duumbi add "change the constant 3 to 7"
duumbi add --yes "multiply instead of add"
duumbi undo   # restore the previous graph
```

Supported providers: **Anthropic**, **OpenAI**, **xAI Grok**, **OpenRouter**, and **MiniMax**.

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
| Windows | x86_64-pc-windows-msvc | Native target; MSVC tools required |

Native Windows builds use the MSVC Rust target and do not require WSL2. The current Windows support boundary does not cover ARM64 Windows, MinGW, Cygwin, GNU Windows toolchains, installers, packaging, or release signing.

---

## Community

- Read the [Code of Conduct](CODE_OF_CONDUCT.md) before opening issues or pull requests.
- Use the provided issue and PR templates.
