#!/usr/bin/env bash
# engines-setup.sh — sanidi engines zote za MTECHOS (nguvu moja).
#
# Huandaa:
#   1. OpenHands Agent Canvas UI  — build ya SPA (`engines/mbilinyi` → /ide)
#   2. VisualClaw gateway          — venv + install (`engines/visualclaw` → :8800)
#   3. Magma server                — habari TU (inahitaji GPU/torch; si kwa CPU)
#
# Baada ya hii: `bash scripts/ide-start.sh` huzindua kila kitu.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "== 1/3 OpenHands Agent Canvas UI =="
if [ -f "$ROOT/engines/mbilinyi/build/index.html" ]; then
  echo "  · tayari imejengwa"
elif command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/engines/mbilinyi" \
    && VITE_BASE_PATH=/ide npm ci >/tmp/canvas-build.log 2>&1 \
    && VITE_BASE_PATH=/ide npm run build >>/tmp/canvas-build.log 2>&1) \
    && echo "  ✓ imejengwa" || echo "  ! imeshindwa (angalia /tmp/canvas-build.log)"
else
  echo "  ! npm haipo — ruka"
fi

echo "== 2/3 VisualClaw =="
VC="$ROOT/engines/visualclaw"
# VisualClaw imevendoriwa kama marejeo ya MAARIFA (SKILL.md + docs), si runtime
# ya Python. MTECHOS huchukua skills zake moja kwa moja kwenye prompt
# (server/agent/src/unified_engines.rs) — hakuna venv inayohitajika.
if [ -d "$VC/visualclaw/skills_seed" ]; then
  echo "  ✓ skills_seed ipo — MTECHOS huchukua SKILL.md moja kwa moja (hakuna venv)"
else
  echo "  ! skills_seed haipo"
fi

echo "== 3/3 Magma =="
# Magma imevendoriwa kama marejeo TU (docs/assets). Hakuna runtime ya Python.
echo "  · Magma ni marejeo ya docs/assets; hakuna server ya Python inayohitajika."

echo
echo "Tayari. Endesha: bash scripts/ide-start.sh"
