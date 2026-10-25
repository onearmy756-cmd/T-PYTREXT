#!/usr/bin/env bash
# Endesha OpenHands (upstream) Agent Canvas — UI halisi ya OpenHands.
#
#   bash scripts/openhands-up.sh          # anza (au anzisha upya)
#   bash scripts/openhands-up.sh stop     # simamisha
#   bash scripts/openhands-up.sh logs     # logi
#
# Hurudisha: http://<host>/canvas/   (port ya ndani 12001 -> 8000 ya container)
#
# Image: ghcr.io/openhands/agent-canvas (OpenHands v1.56.0, all-in-one:
# agent-server + automation + frontend).
set -euo pipefail

NAME=openhands-canvas
IMAGE=${OPENHANDS_IMAGE:-ghcr.io/openhands/agent-canvas:latest}
PORT=${OPENHANDS_PORT:-12001}
STATE=${OPENHANDS_STATE:-/workspace/oh-state}
PROJECTS=${OPENHANDS_PROJECTS:-/workspace/oh-projects}

# Docker socket ni root:docker — endesha kwa kikundi cha docker.
DOCKER="docker"
if ! docker ps >/dev/null 2>&1; then
  if sg docker -c "docker ps" >/dev/null 2>&1; then
    DOCKER="sg docker -c"
  else
    echo "docker haipatikani. Anza daemon: sudo dockerd >/tmp/dockerd.log 2>&1 &" >&2
    exit 1
  fi
fi

run() { if [ "$DOCKER" = "sg docker -c" ]; then sg docker -c "$*"; else $*; fi; }

case "${1:-up}" in
  stop)
    run "docker rm -f $NAME" || true
    echo "imesimamishwa: $NAME"
    exit 0
    ;;
  logs)
    run "docker logs --tail 60 -f $NAME"
    exit 0
    ;;
esac

mkdir -p "$STATE" "$PROJECTS"

run "docker rm -f $NAME" >/dev/null 2>&1 || true

# -p 12001:8000 (bila 127.0.0.1) ili proxy ya umma iweze kuifikia.
run "docker run -d --name $NAME --restart unless-stopped \
  -p ${PORT}:8000 \
  -e AGENT_CANVAS_ALLOW_LAN_SESSION_KEY=true \
  -e AGENT_CANVAS_DISABLE_TELEMETRY=1 \
  -v ${STATE}:/home/openhands/.openhands \
  -v ${PROJECTS}:/projects \
  $IMAGE"

echo "inaanzisha..."
READY=0
for i in $(seq 1 40); do
  if curl -fsS -m 3 "http://127.0.0.1:${PORT}/health" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 3
done

if [ "$READY" != 1 ]; then
  echo "haikuanza kwa wakati. Angalia: bash scripts/openhands-up.sh logs" >&2
  exit 1
fi

# Weka rangi ya MBILINYI IDE (Obsidian Ember) kwenye UI ya OpenHands.
THEME="$(cd "$(dirname "$0")" && pwd)/openhands-theme/mbilinyi-theme.css"
if [ -f "$THEME" ]; then
  run "docker cp '$THEME' $NAME:/opt/agent-canvas/frontend/mbilinyi-theme.css"
  TMPIDX=$(mktemp)
  run "docker exec $NAME cat /opt/agent-canvas/frontend/index.html" > "$TMPIDX"
  if ! grep -q mbilinyi-theme.css "$TMPIDX"; then
    python3 - "$TMPIDX" <<'PY'
import sys
p = sys.argv[1]
h = open(p).read()
h = h.replace(
    "</head>",
    '<link rel="stylesheet" href="/canvas/mbilinyi-theme.css"></head>',
    1,
)
open(p, "w").write(h)
PY
    run "docker cp '$TMPIDX' $NAME:/opt/agent-canvas/frontend/index.html"
  fi
  rm -f "$TMPIDX"
  echo "rangi ya MBILINYI (Obsidian Ember) imewekwa kwenye UI."
fi

echo "OpenHands iko tayari: http://127.0.0.1:${PORT}/canvas/"
echo "Weka LLM API key kwenye UI (Settings) kabla ya kuanza mazungumzo."
exit 0
