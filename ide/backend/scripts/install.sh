#!/usr/bin/env bash
# Sakinisha `fundi` CLI kutoka chanzo (fundi-deploy / server/agent).
# Hakuna release binaries bado - jitengeneze kwenye workspace hii.
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

SRC="${1:-}"
if [[ -z "${SRC}" ]]; then
  HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  if [[ -f "${HERE}/../Cargo.toml" ]]; then SRC="${HERE}/.."; fi
fi

if [[ -z "${SRC}" || ! -f "${SRC}/Cargo.toml" ]]; then
  cat >&2 <<'MSG'
ERROR: haiwezi kupata chanzo cha fundi-deploy (server/agent/Cargo.toml).

Tumia moja kati ya hizi:
  ./install.sh /path/to/fundi-deploy/server/agent
  cargo install --path /path/to/fundi-deploy/server/agent/crates/fundi-cli
MSG
  exit 1
fi

echo "Building fundi kutoka ${SRC} ..."
mkdir -p "${INSTALL_DIR}"
cargo install --path "${SRC}/crates/fundi-cli" --locked --root "$(dirname "${INSTALL_DIR}")"

echo "Imekamilika: $(dirname "${INSTALL_DIR}")/bin/fundi"
echo "Hakikisha ${INSTALL_DIR} iko kwenye PATH."
