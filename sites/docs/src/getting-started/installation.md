# Installation

## Prerequisites

- Rust toolchain (`rustup` recommended): [rustup.rs](https://rustup.rs)
- A C compiler for linking: `cc` on PATH (Xcode CLT on macOS, `build-essential` on Debian/Ubuntu)

## Install from crates.io

```bash
cargo install duumbi
```

## Build from source

```bash
git clone https://github.com/hgahub/duumbi
cd duumbi
cargo build --release
# Binary: target/release/duumbi
```

## Verify installation

```bash
duumbi --version
```

## Supported platforms

| Platform | Architecture | Support |
|----------|-------------|---------|
| macOS | aarch64 (Apple Silicon) | Primary |
| macOS | x86_64 | CI-tested |
| Linux | x86_64 | CI-tested |

Windows is not supported in the current release.
