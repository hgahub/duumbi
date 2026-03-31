#!/usr/bin/env bash
set -euo pipefail

# Install required Rust components
rustup component add rustfmt clippy

# Install pre-commit hooks if pre-commit is available
if command -v pre-commit >/dev/null 2>&1; then
    pre-commit install --install-hooks --hook-type pre-commit --hook-type commit-msg
fi

# Install mdbook if not already available
if ! command -v mdbook >/dev/null 2>&1; then
    cargo install mdbook
fi
