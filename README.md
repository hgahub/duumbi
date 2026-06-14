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

### Developer preview release

DUUMBI `v0.4.0-preview` is distributed as prebuilt GitHub Release archives for:

| Platform | Target | Status |
|----------|--------|--------|
| macOS Apple Silicon | `aarch64-apple-darwin` | Required preview target |
| macOS Intel | `x86_64-apple-darwin` | Required preview target |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Required preview target |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Extra preview target |

Download the archive for your platform from the
[`v0.4.0-preview` release](https://github.com/hgahub/duumbi/releases/tag/v0.4.0-preview)
and verify it with the published checksum file:

For the archive install, Rust is not required to install DUUMBI itself. The
smoke test below runs `duumbi build` and `duumbi run`, so it still needs the
native build/link tools used when compiling DUUMBI programs: Xcode Command Line
Tools or an equivalent C compiler/linker on macOS, and `build-essential` or an
equivalent C compiler/linker on Linux. Linux systems also need the runtime
linker dependencies used by the DUUMBI C runtime, including libcurl.

```bash
DUUMBI_VERSION=v0.4.0-preview
DUUMBI_TARGET=<target>

curl -LO "https://github.com/hgahub/duumbi/releases/download/${DUUMBI_VERSION}/duumbi-${DUUMBI_VERSION}-${DUUMBI_TARGET}.tar.gz"
curl -LO "https://github.com/hgahub/duumbi/releases/download/${DUUMBI_VERSION}/checksums.txt"
shasum -a 256 --ignore-missing -c checksums.txt

tar xzf "duumbi-${DUUMBI_VERSION}-${DUUMBI_TARGET}.tar.gz"
export PATH="$PWD/duumbi-${DUUMBI_VERSION}-${DUUMBI_TARGET}:$PATH"

duumbi --version
duumbi init smoke
cd smoke
duumbi build
duumbi run
```

Keep the extracted release directory together. The CLI expects the packaged
`runtime/` tree beside the executable when linking native DUUMBI programs.

### Build from source

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

For a runnable reference program that combines HTTP, SQLite, and JSON, see
[`examples/flagship-http-sqlite-json/`](examples/flagship-http-sqlite-json/)
and the [examples guide](docs/examples.md).

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

- Join the [DUUMBI Discord](https://discord.gg/FJ92HrZKu7) for community discussion, help, showcases, development coordination, and announcements.
- Read the [Code of Conduct](CODE_OF_CONDUCT.md) before opening issues or pull requests.
- Use the provided issue and PR templates.
