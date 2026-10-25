#!/bin/sh
# docker-entrypoint.sh — MTECHOS container: AGENT HALISI + server kuu.
#
# Mchakato mmoja hautoshi: mfumo una sehemu mbili zinazoendesha pamoja kwenye
# machine moja ya Fly:
#
#   1. fundi-server  → AGENT HALISI (tool registry 60) kwenye loopback :8787.
#      Hii ndiyo inayotumiwa na `chat.rs::try_fundi_agent` (inajaribiwa KABLA
#      ya LLM) — ndiyo maana chat inarudi `agent_used: true`.
#      Zana zake zinaita API ya MTECH OS kwa `FUNDI_DEPLOY_URL`.
#
#   2. fundi-deploy  → server kuu (Axum, :8080) — Web UI + API zote.
#      Anaendesha kwenye FOREGROUND ili Fly aone mchakato halisi (health check
#      ya /health inapima hii). Akianguka, container inaisha — sio "zombie".
#
# LLM YA AGENT (muhimu): fundi-server ina provider lookup yake (`fundi-config`).
# Bila key inaanguka kwenye StubProvider ("no LLM backend configured for model
# 'gpt-4o'"). Tunaitumia KEY ILE ILE ya mfumo (`FUNDI_AI_KEY` — Fly secret,
# haipo kwenye repo) kwa njia ya kawaida ya OpenAI-compatible:
#
#   FUNDI_MODEL      default "openai:gpt-oss:120b"
#                    (prefix `openai:` ni lazima — `parse_model_string` inagawanya
#                     kwenye ':' ya kwanza; model ya Ollama yenye ':' yenyewe,
#                     mf. `gpt-oss:120b`, inahitaji prefix hii.)
#   FUNDI_BASE_URL   default "https://ollama.com/v1"  (OpenAiProvider inaongeza
#                    `/chat/completions`, hivyo /v1 ni lazima iwepo.)
#   OPENAI_API_KEY   = FUNDI_AI_KEY ikiwa haijawekwa moja kwa moja.

set -eu

MAIN_PORT="${PORT:-8080}"
AGENT_PORT="${FUNDI_AGENT_PORT:-8787}"

# /data mara nyingi ni volume (Fly) inayomilikiwa na root, hivyo mtumiaji `fundi`
# hawezi kuandika SQLite. (FUNDI_DB tayari ikiwekwa, tunaheshimu.)
# Bila hii, UI hutoa "Unable to connect to server" — server huanguka tu.
if [ -z "${FUNDI_DB:-}" ]; then
    if _probe="$(mktemp -d /data/.probe.XXXXXX 2>/dev/null)"; then
        rmdir "$_probe" 2>/dev/null || true
    else
        echo "[entrypoint] ONYO: /data haiandikiki — tunatumia /tmp/fundi.db"
        FUNDI_DB="sqlite:///tmp/fundi.db?mode=rwc"
        export FUNDI_DB
    fi
fi

if [ "${FUNDI_AGENT_ENABLED:-1}" != "0" ]; then
    # Key ya agent: OPENAI_API_KEY ikiwa imewekwa, la sivyo key ile ile ya mfumo.
    if [ -z "${OPENAI_API_KEY:-}" ] && [ -n "${FUNDI_AI_KEY:-}" ]; then
        OPENAI_API_KEY="${FUNDI_AI_KEY}"
        export OPENAI_API_KEY
    fi

    AGENT_MODEL="${FUNDI_MODEL:-openai:gpt-oss:120b}"
    AGENT_BASE_URL="${FUNDI_BASE_URL:-https://ollama.com/v1}"

    if [ -z "${OPENAI_API_KEY:-}" ]; then
        echo "[entrypoint] ONYO: hakuna FUNDI_AI_KEY/OPENAI_API_KEY — agent atakwama kwenye StubProvider"
    fi

    echo "[entrypoint] Inaanzisha AGENT HALISI (fundi-server) kwenye 127.0.0.1:${AGENT_PORT} (model ${AGENT_MODEL})"
    FUNDI_SERVER_ADDR="127.0.0.1:${AGENT_PORT}" \
    FUNDI_DEPLOY_URL="${FUNDI_DEPLOY_URL:-http://127.0.0.1:${MAIN_PORT}}" \
    FUNDI_MODEL="${AGENT_MODEL}" \
    FUNDI_BASE_URL="${AGENT_BASE_URL}" \
        /app/fundi-server &
    AGENT_PID=$!

    # Subiri agent ijibu (max ~10s) — kama haiji, server kuu INAENDELEA
    # (mfumo unafanya kazi bila agent; chat inashuka kwenye LLM + typed tools).
    i=0
    while [ "$i" -lt 20 ]; do
        if curl -sf -m 2 "http://127.0.0.1:${AGENT_PORT}/health" >/dev/null 2>&1; then
            echo "[entrypoint] Agent tayari (pid ${AGENT_PID})"
            break
        fi
        if ! kill -0 "$AGENT_PID" 2>/dev/null; then
            echo "[entrypoint] ONYO: agent imekwama kuanza — tunaendelea bila yeye"
            break
        fi
        i=$((i + 1))
        sleep 0.5
    done
else
    echo "[entrypoint] FUNDI_AGENT_ENABLED=0 — tunaanza bila agent"
fi

# ── 3. Protocol ya agent-server: RUST PEKEE (hakuna Python runtime) ──
# Canvas ni UI PEKEE; kila kitendo huita protocol ya agent-server kwenye
# `/api/*` + `/sockets/*`. Hizi SASA zinahudumiwa na `server/agent/src/
# agent_native/` (Rust) — hakuna venv, `uv`, wala `openhands-agent-server`.
#
# SHARTI YA USALAMA: Canvas huipata key ndani ya `/ide` HTML
# (`window.__AGENT_CANVAS_SESSION_API_KEY__`) — inatoka kwa token IMARA ya
# fundi-deploy (`agent_session_stable`, `<FUNDI_DATA>/agent_token`). Protocol
# ya Rust huithibitisha kwa `X-Session-API-Key`, hivyo token LAZIMA iwepo
# KABLA fundi-deploy ianze (imeundwa hapa chini ikiwa haipo).
DATA_DIR="${FUNDI_DATA:-/app/data}"
mkdir -p "$DATA_DIR" 2>/dev/null || true
# `FUNDI_DATA` (/app/data) ni image layer isiyodumu kwenye DEPLOY; volume
# halisi ni `/data`. Weka token /data inapowezekana ili idumu.
PERSIST_DIR="$DATA_DIR"
if [ -d /data ] && [ -w /data ]; then
    PERSIST_DIR="/data"
    mkdir -p /data/agent-server 2>/dev/null || true
fi
# Rust husoma token kwa `FUNDI_DATA/agent_token`. Weka kwenye PERSIST_DIR na
# symlink kutoka FUNDI_DATA ili `agent_session_stable` isome ile ile.
TOKEN_FILE="${DATA_DIR}/agent_token"
SHARED_TOKEN="${PERSIST_DIR}/agent_token"
if [ ! -s "$SHARED_TOKEN" ]; then
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$SHARED_TOKEN" 2>/dev/null || true
    chmod 600 "$SHARED_TOKEN" 2>/dev/null || true
fi
if [ -s "$SHARED_TOKEN" ] && [ "$SHARED_TOKEN" != "$TOKEN_FILE" ]; then
    ln -sf "$SHARED_TOKEN" "$TOKEN_FILE" 2>/dev/null || true
fi
# Tengeneza token IMARA (hex 64, sawa na `agent_session` ya Rust) ikiwa haipo.
if [ ! -s "$TOKEN_FILE" ]; then
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$TOKEN_FILE" 2>/dev/null || true
    chmod 600 "$TOKEN_FILE" 2>/dev/null || true
fi
# Hifadhi ya data ya protocol (conversations/bash events/workspace) kwa Rust.
# Hifadhi kwenye PERSIST_DIR ili idumu redeploy; Rust hurudi kwenye hifadhi yake
# ya ndani (FUNDI_DATA) ikiwa hazipo.
export FUNDI_NATIVE_STORE="${FUNDI_NATIVE_STORE:-${PERSIST_DIR}/agent-server/state.json}"
export FUNDI_WORKSPACE="${FUNDI_WORKSPACE:-${PERSIST_DIR}/agent-server/workspace}"
mkdir -p "$FUNDI_WORKSPACE" 2>/dev/null || true

echo "[entrypoint] Inaanzisha server kuu kwenye 0.0.0.0:${MAIN_PORT}"

# LLM ya CLOUD (Ollama Cloud / OpenAI-compatible): server kuu husoma key kwa
# `FUNDI_AI_KEY`. Kwenye Fly tunaweka key moja tu (`FUNDI_AI_KEY`) — iweke pia
# `OPENAI_API_KEY` ili iwe rahisi; hapa tunaihamisha kinyume pia.
if [ -z "${FUNDI_AI_KEY:-}" ] && [ -n "${OPENAI_API_KEY:-}" ]; then
    FUNDI_AI_KEY="${OPENAI_API_KEY}"
    export FUNDI_AI_KEY
fi
export FUNDI_CLOUD_URL="${FUNDI_CLOUD_URL:-https://ollama.com/v1}"

# $@ inaruhusu kubadilisha amri bila kujenga image upya (mf. debug shell).
if [ "$#" -gt 0 ]; then
    exec "$@"
fi
exec /app/fundi-deploy
