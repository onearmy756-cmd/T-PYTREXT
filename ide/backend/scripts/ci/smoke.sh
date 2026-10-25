#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY_PATH="${1:-$ROOT_DIR/target/release/fundi}"

if [[ ! -x "$BINARY_PATH" ]]; then
  echo "[smoke] binary not found or not executable: $BINARY_PATH"
  echo "[smoke] build with: cargo build --release -p fundi-cli"
  exit 1
fi

TMP_HOME="$(mktemp -d)"
trap 'rm -rf "$TMP_HOME"' EXIT

echo "[smoke] binary: $BINARY_PATH"
"$BINARY_PATH" --version >/dev/null
FUNDI_HOME="$TMP_HOME" "$BINARY_PATH" model >/dev/null
FUNDI_HOME="$TMP_HOME" "$BINARY_PATH" gateway status >/dev/null

echo "[smoke] ok"
