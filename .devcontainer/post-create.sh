#!/usr/bin/env bash
set -euo pipefail

# Ensure Cargo and target cache directories are writable for the vscode user.
mkdir -p /home/vscode/.cargo/registry /home/vscode/.cargo/git /home/vscode/.cache/duumbi-target
if command -v sudo >/dev/null 2>&1; then
    sudo chown -R vscode:rustlang /home/vscode/.cargo /home/vscode/.cache/duumbi-target || true
    sudo chmod -R u+rwX,g+rwX /home/vscode/.cargo /home/vscode/.cache/duumbi-target || true
fi

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
