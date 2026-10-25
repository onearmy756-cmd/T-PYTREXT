#!/usr/bin/env bash
# Register ide_emulators + ide_lab modules + routes into server/agent/src/main.rs
set -euo pipefail
MAIN="${1:-server/agent/src/main.rs}"

if [[ ! -f "$MAIN" ]]; then
  echo "main.rs haipatikani: $MAIN" >&2
  exit 1
fi

add_mod() {
  local m="$1" anchor="$2"
  if grep -q "mod $m;" "$MAIN"; then
    echo "mod $m already present"
    return
  fi
  if grep -q "$anchor" "$MAIN"; then
    sed -i "s|$anchor|$anchor\nmod $m;|" "$MAIN"
    echo "added mod $m"
  else
    echo "anchor '$anchor' haipatikani — ongeza manually: mod $m;" >&2
  fi
}

add_mod ide_emulators 'mod ide_api;'
add_mod ide_lab 'mod ide_emulators;'
add_mod ide_swarm 'mod ide_lab;'
add_mod code_audit 'mod ide_swarm;'
add_mod ide_devices 'mod code_audit;'
add_mod ide_testkit 'mod ide_devices;'
add_mod ide_build 'mod ide_testkit;'
add_mod ide_engineer 'mod ide_build;'
add_mod ide_lsp 'mod ide_engineer;'
add_mod ide_dap 'mod ide_lsp;'
add_mod ide_e2e 'mod ide_dap;'
add_mod ide_pipeline 'mod ide_e2e;'

if grep -q 'ide_emulators::router' "$MAIN"; then
  echo "emulator router merge already present"
else
  sed -i 's|\.route("/api/ide/agent/status", get(ide_agent_status))|.route("/api/ide/agent/status", get(ide_agent_status))\n        .merge(crate::ide_emulators::router())|' "$MAIN"
  echo "added emulator router merge"
fi

if grep -q 'ide_lab::router' "$MAIN"; then
  echo "lab router merge already present"
else
  sed -i 's|\.merge(crate::ide_emulators::router())|.merge(crate::ide_emulators::router())\n        .merge(crate::ide_lab::router())|' "$MAIN"
  echo "added lab router merge"
fi

if grep -q 'code_audit::router' "$MAIN"; then
  echo "code_audit router merge already present"
else
  sed -i 's|\.merge(crate::ide_lab::router())|.merge(crate::ide_lab::router())\n        .merge(crate::code_audit::router())|' "$MAIN"
  echo "added code_audit router merge"
fi

if grep -q 'ide_swarm::router' "$MAIN"; then
  echo "swarm router merge already present"
else
  sed -i 's|\.merge(crate::code_audit::router())|.merge(crate::code_audit::router())\n        .merge(crate::ide_swarm::router())|' "$MAIN"
  echo "added swarm router merge"
fi

if grep -q 'ide_devices::router' "$MAIN"; then
  echo "devices router merge already present"
else
  sed -i 's|\.merge(crate::ide_swarm::router())|.merge(crate::ide_swarm::router())\n        .merge(crate::ide_devices::router())|' "$MAIN"
  echo "added devices router merge"
fi

for m in ide_testkit ide_build ide_engineer ide_lsp ide_dap ide_e2e ide_pipeline; do
  if grep -q "${m}::router" "$MAIN"; then
    echo "$m router merge already present"
  else
    sed -i "s|\.merge(crate::ide_devices::router())|.merge(crate::ide_devices::router())\n        .merge(crate::${m}::router())|" "$MAIN"
    echo "added $m router merge"
  fi
done

echo "OK — rebuild agent: cargo build -p fundi-deploy"
