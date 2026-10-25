#!/usr/bin/env bash
# Anzisha MTECHOS IDE (backend agent + static app)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "→ patch emulator routes"
bash scripts/patch_ide_emulators.sh || true

# AGENT CANVAS UI (`engines/mbilinyi`, @openhands/agent-canvas): bila build/,
# /ide hurudisha ukurasa wa maelekezo PEKEE (hakuna UI halisi). Hapa tunaijenga
# kwa base `/ide` ili assets ziwe /ide/assets/*. Weka SKIP_CANVAS_BUILD=1 kuruka.
if [ "${SKIP_CANVAS_BUILD:-0}" != "1" ] && [ ! -f "$ROOT/engines/mbilinyi/build/index.html" ]; then
  if command -v npm >/dev/null 2>&1; then
    echo "→ Agent Canvas UI haijajengwa — inajenga (npm ci + build, base /ide)…"
    if (cd "$ROOT/engines/mbilinyi" && VITE_BASE_PATH=/ide npm ci >/tmp/canvas-build.log 2>&1 \
        && VITE_BASE_PATH=/ide npm run build >>/tmp/canvas-build.log 2>&1); then
      echo "  ✓ Canvas UI imejengwa (engines/mbilinyi/build)"
    else
      echo "  ! build imeshindwa (angalia /tmp/canvas-build.log) — /ide itaonyesha maelekezo"
    fi
  else
    echo "  ! npm haipo — ruka build ya Canvas (angalia engines/README.md)"
  fi
elif [ -f "$ROOT/engines/mbilinyi/build/index.html" ]; then
  echo "→ Agent Canvas UI ipo (engines/mbilinyi/build)"
fi

# VISUALCLAW (engines/visualclaw): skill injection + memory + governance.
# MTECHOS ni Rust/Go/Elixir pekee — VisualClaw ni MARJEO YA SKILLS pekee
# (`visualclaw/skills_seed/*/*/SKILL.md` huingizwa kwenye prompt moja kwa moja
# na `unified_engines.rs`). Hakuna gateway/venv ya Python.

# FUNDI SERVER (OpenHands backend ya Rust): HTTP/WS API kwenye :8787 yenye
# tool calls HALISI (terminal, execute_code, R engine, memory, n.k.). Bila hii,
# daraja la agent (fundi-deploy → :8787) halifanyi kazi (IDE Agent panel haina tools).
# Inaanza nyuma (background); fundi-deploy inaendelea kama process kuu.
export FUNDI_DATA="${FUNDI_DATA:-$ROOT/data}"
export FUNDI_AGENT_ADDR="${FUNDI_AGENT_ADDR:-http://127.0.0.1:8787}"

# LLM ya backend: fundi-server hutumia ~/.fundi/config.yaml. Kama haipo na app
# ina key (data/ai_key), itengeneze kutoka env (FUNDI_CLOUD_URL/MODEL) ili
# backend isikose LLM (bila key → StubProvider → agent haina uwezo).
FUNDI_HOME_DIR="${FUNDI_HOME:-$HOME/.fundi}"
if [ ! -f "$FUNDI_HOME_DIR/config.yaml" ] && [ -f "$FUNDI_DATA/ai_key" ]; then
  mkdir -p "$FUNDI_HOME_DIR"
  _key="$(tr -d '\n' < "$FUNDI_DATA/ai_key")"
  _url="${FUNDI_CLOUD_URL:-https://ollama.com/v1}"
  _model="${FUNDI_CLOUD_MODEL:-gpt-oss:20b}"
  printf 'model: "ollama:%s"\nllm_providers:\n  ollama:\n    api_key: "%s"\n    base_url: "%s"\n    model: "%s"\n' \
    "$_model" "$_key" "$_url" "$_model" > "$FUNDI_HOME_DIR/config.yaml"
  echo "→ config ya backend imetengenezwa: $FUNDI_HOME_DIR/config.yaml"
fi

if ! curl -sf -m 2 "$FUNDI_AGENT_ADDR/health" >/dev/null 2>&1; then
  echo "→ fundi-server (backend agent) :8787"
  (cd server/agent && cargo run --release --bin fundi-server >/tmp/fundi-server.log 2>&1 &)
  for _ in $(seq 1 30); do
    curl -sf -m 2 "$FUNDI_AGENT_ADDR/health" >/dev/null 2>&1 && break
    sleep 1
  done
  curl -sf -m 2 "$FUNDI_AGENT_ADDR/health" >/dev/null 2>&1 \
    && echo "  ✓ backend ipo" || echo "  ! backend haijaanza (angalia /tmp/fundi-server.log)"
else
  echo "→ fundi-server tayari inaendesha"
fi

cd server/agent
echo "→ cargo run --release (fundi-deploy)"
export RUST_LOG="${RUST_LOG:-info}"
export PORT="${PORT:-8080}"
cargo run --release --bin fundi-deploy 2>/dev/null || cargo run --release
