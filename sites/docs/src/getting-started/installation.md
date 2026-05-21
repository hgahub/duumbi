# Installation

## Prerequisites

- Rust toolchain (`rustup` recommended): [rustup.rs](https://rustup.rs)
- A C compiler and linker:
  - macOS: Xcode Command Line Tools or equivalent
  - Linux: `build-essential` or equivalent
  - Windows native: Windows 10 version 1903+ on `x86_64-pc-windows-msvc`, the stable MSVC Rust toolchain, Visual Studio Build Tools or equivalent MSVC C++ tools, Windows SDK, and a usable linker/C compiler environment

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
| Windows | x86_64-pc-windows-msvc | Native target; MSVC tools required |

Native Windows builds use the MSVC Rust target and do not require WSL2. The current Windows support boundary does not cover ARM64 Windows, MinGW, Cygwin, GNU Windows toolchains, installers, packaging, or release signing.
