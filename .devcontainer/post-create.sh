#!/usr/bin/env bash
set -euo pipefail

# Ensure Cargo and workspace target directories are writable for the vscode user.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
mkdir -p /home/vscode/.cargo/registry /home/vscode/.cargo/git "$WORKSPACE_DIR/target"
if command -v sudo >/dev/null 2>&1; then
    VSCODE_GROUP="$(id -gn vscode)"
    if ! sudo chown -R "vscode:${VSCODE_GROUP}" /home/vscode/.cargo "$WORKSPACE_DIR/target"; then
        echo "warning: failed to set ownership for Cargo cache or target directory" >&2
    fi
    sudo chmod -R u+rwX,g+rwX /home/vscode/.cargo "$WORKSPACE_DIR/target"
fi

# Install required Rust components
rustup component add rustfmt clippy

# Install pre-commit hooks if pre-commit is available
if command -v pre-commit >/dev/null 2>&1; then
    pre-commit install --hook-type pre-commit --hook-type commit-msg
    if command -v python3.12 >/dev/null 2>&1; then
        pre-commit install-hooks
    else
        echo "warning: python3.12 not found; skipping pre-commit hook environment install"
    fi
fi

# Install mdbook if not already available
if ! command -v mdbook >/dev/null 2>&1; then
    cargo install mdbook
fi
