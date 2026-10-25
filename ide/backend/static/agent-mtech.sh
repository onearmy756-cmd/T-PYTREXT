#!/usr/bin/env bash
# =============================================================
# MTECH OS — AGENT INSTALLER (Linux)
# Mbilinyi Tech · Umiliki ni wako, leseni ni yako, faida ni yako
#
# MATUMIZI (kwenye kifaa cha Linux):
#   SERVER="http://192.168.1.10:8080" DEVICE="lab_user_201" bash agent-mtech.sh
#
# Script inafanya:
#   1. Kujisajili kwenye server (inapata TOKEN — inahifadhiwa /etc/mtech/agent.json)
#   2. Heartbeat kila sekunde 20
#   3. Poll kazi ZILIZOIDHINISHWA (HITL)
#   4. Kutekeleza kazi (usalama: ukaguzi wa ndani; bundle: apt)
#   5. Kuripoti maendeleo
# =============================================================
set -euo pipefail

SERVER="${SERVER:-http://127.0.0.1:8080}"
DEVICE="${DEVICE:-$(hostname)}"
INTERVAL="${INTERVAL:-20}"
STATE_DIR="/etc/mtech"
STATE_FILE="$STATE_DIR/agent.json"

mkdir -p "$STATE_DIR"

api() { # api METHOD PATH BODY
  local method="$1" path="$2" body="${3:-}"
  if [ -n "$body" ]; then
    curl -fsS -X "$method" -H 'Content-Type: application/json' -d "$body" "$SERVER$path"
  else
    curl -fsS -X "$method" "$SERVER$path"
  fi
}

# ---------- 1. REGISTER (au kurudi kwa token iliyopo) ----------
if [ ! -s "$STATE_FILE" ]; then
  echo "[MTECH OS] Inajisajili kwenye server: $SERVER (kifaa: $DEVICE)"
  REG=$(api POST /api/fleet/register "{\"device\":\"$DEVICE\",\"os_type\":\"ubuntu\",\"health\":\"nzuri\"}")
  echo "$REG" | grep -q '"ok":true' || { echo "[MTECH OS] Usajili umeshindikana: $REG"; exit 1; }
  echo "$REG" > "$STATE_FILE"
  chmod 600 "$STATE_FILE"
  echo "[MTECH OS] Imesajiliwa — token imehifadhiwa $STATE_FILE (root pekee)"
fi
AGENT_ID=$(sed -n 's/.*"agent_id":"\([^"]*\)".*/\1/p' "$STATE_FILE")
TOKEN=$(sed -n 's/.*"token":"\([^"]*\)".*/\1/p' "$STATE_FILE")
[ -n "$AGENT_ID" ] && [ -n "$TOKEN" ] || { echo "[MTECH OS] Token haijakamilika — futa $STATE_FILE ujisajili upya"; exit 1; }

get_health() {
  local cpu mem disk
  cpu=$(top -bn1 | awk '/Cpu\(s\)/{print $2+$4}' | head -1)
  mem=$(free -m | awk '/Mem:/{printf "%.1f", $7/1024}')
  disk=$(df -h / | awk 'NR==2{print $4}')
  echo "CPU ${cpu:-?}% · RAM ${mem:-?} GB bure · Disk ${disk:-?} bure"
}

security_check() {
  # Uchunguzi wa usalama wa ndani (bila zana za nje)
  local findings=""
  command -v ufw >/dev/null && [ "$(ufw status | grep -c 'Status: active')" = "0" ] && \
    findings='[{"kind":"problem","detail":"Firewall (ufw) imezimwa","severity":40}]'
  [ -z "$findings" ] && [ "$(awk -F: '$2!~/^(!\*|!)/ && $7!~/nologin|false/' /etc/passwd | wc -l)" -gt 30 ] && \
    findings='[{"kind":"problem","detail":"Watumiaji wengi wanaoingia","severity":20}]'
  echo "${findings:-[]}"
}

do_job() {
  local stage="$1" apps="$2"
  case "$stage" in
    secops:*)
      api POST /api/fleet/report "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"progress\":30,\"message\":\"Ukaguzi unaendelea\"}" >/dev/null || true
      F=$(security_check)
      api POST /api/secops/result "{\"account\":\"mteja1\",\"target\":\"$DEVICE\",\"mode\":\"security\",\"findings\":$F,\"sources\":\"mfumo\",\"note\":\"agent\"}" >/dev/null || true
      api POST /api/fleet/report "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"progress\":100,\"message\":\"Ukaguzi umekamilika\"}" >/dev/null || true
      ;;
    bundle:*|deploy*)
      api POST /api/fleet/report "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"progress\":20,\"message\":\"Usakinishaji unaendelea\"}" >/dev/null || true
      # Apps HALISI kutoka server (winget/apt) — kila amri inatekelezwa kwa mfululizo.
      if command -v apt-get >/dev/null; then
        DEBIAN_FRONTEND=noninteractive apt-get update -qq >/dev/null 2>&1 || true
      fi
      local cmds="$apps"
      if [ -z "$cmds" ]; then
        # server haikutuma apps (bundle isiyojulikana) — usakinishaji wa msingi salama
        cmds="DEBIAN_FRONTEND=noninteractive apt-get install -y -qq curl htop"
      fi
      local total i
      total=$(printf '%s\n' "$cmds" | grep -c .)
      i=0
      printf '%s\n' "$cmds" | while IFS= read -r c; do
        [ -z "$c" ] && continue
        i=$((i + 1))
        api POST /api/fleet/report "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"progress\":$((20 + i * 70 / total)),\"message\":\"App $i/$total: $c\"}" >/dev/null || true
        DEBIAN_FRONTEND=noninteractive sh -c "$c" >/dev/null 2>&1 || true
      done
      api POST /api/fleet/report "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"progress\":100,\"message\":\"Usakinishaji umekamilika\"}" >/dev/null || true
      ;;
    *)
      api POST /api/fleet/report "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"progress\":100,\"message\":\"Kazi haijatambulika — imeghairi\"}" >/dev/null || true
      ;;
  esac
}

# ---------- 2-5. LOOP KUU ----------
echo "[MTECH OS] Agent inaendesha — Ctrl+C kuzima"
while true; do
  api POST /api/fleet/heartbeat "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\",\"health\":\"$(get_health)\"}" >/dev/null 2>&1 || echo "[MTECH OS] heartbeat imeshindikana"
  P=$(api POST /api/fleet/poll "{\"agent_id\":\"$AGENT_ID\",\"token\":\"$TOKEN\"}" 2>/dev/null || true)
  if echo "$P" | grep -q '"job":{"id"'; then
    JID=$(echo "$P" | sed -n 's/.*"job":{"id":"\([^"]*\)".*/\1/p')
    STAGE=$(echo "$P" | sed -n 's/.*"stage":"\([^"]*\)".*/\1/p')
    # Apps halisi kutoka server: `"apps":["cmd1","cmd2"]` → kila moja kwenye mstari wake
    APPS=$(echo "$P" | sed -n 's/.*"apps":\[\([^]]*\)\].*/\1/p' | tr -d '"' | tr ',' '\n')
    echo "[MTECH OS] Kazi mpya: $JID ($STAGE)"
    do_job "$STAGE" "$APPS"
  fi
  sleep "$INTERVAL"
done
