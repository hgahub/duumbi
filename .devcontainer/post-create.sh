#!/usr/bin/env bash
set -euo pipefail

# Ensure Cargo and workspace target directories are writable for the devcontainer user.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
USER_HOME="${HOME:-/home/vscode}"
DEV_USER="$(id -un)"
DEV_GROUP="$(id -gn)"
mkdir -p "$USER_HOME/.cargo/registry" "$USER_HOME/.cargo/git" "$WORKSPACE_DIR/target"
if command -v sudo >/dev/null 2>&1; then
    if ! sudo chown -R "$DEV_USER:$DEV_GROUP" "$USER_HOME/.cargo" "$WORKSPACE_DIR/target"; then
        echo "warning: failed to set ownership for Cargo cache or target directory" >&2
    fi
    sudo chmod -R u+rwX,g+rwX "$USER_HOME/.cargo" "$WORKSPACE_DIR/target"
fi

# Install required Rust components
rustup component add rustfmt clippy

# Configure npm global installs to use a user-writable prefix.
# This avoids EACCES errors caused by root-owned global module paths.
if command -v npm >/dev/null 2>&1; then
    mkdir -p "$USER_HOME/.local/bin" "$USER_HOME/.local/lib"
    npm config set prefix "$USER_HOME/.local"

    ensure_path_export() {
        local rc_file="$1"
        local export_line='export PATH="$HOME/.local/bin:$PATH"'

        touch "$rc_file"
        if ! grep -qxF "$export_line" "$rc_file"; then
            echo "$export_line" >> "$rc_file"
        fi
    }

    ensure_path_export "$USER_HOME/.bashrc"
    ensure_path_export "$USER_HOME/.profile"
fi

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
