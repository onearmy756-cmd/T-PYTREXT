# MBILINYITECH IDE — Tauri v2 (Desktop)

Shell ya desktop (**Tauri 2 + Rust**) inayotumia **UI ile ile ya canvas** kama cloud IDE:

- Anzisha: `static/boot-canvas.html` → agent (:8080) → **`/ide`** (MBILINYI Agent Canvas, React)
- Chat + quick actions + configurations zote ndani ya canvas (ide.html imetangazwa kufa)
- Backend: Rust agent (`server/agent`) — `/api/*` zote

**Hakuna mabadiliko ya UI logic** — Tauri inafunga dirisha tu na kuunganisha na agent.

## Mahitaji

- [Rust](https://rustup.rs/) (1.77+)
- [Node.js](https://nodejs.org/) 18+
- Tauri system deps: https://v2.tauri.app/start/prerequisites/
  - Linux: `webkit2gtk`, `libgtk-3`, n.k.
  - Windows: WebView2
  - macOS: Xcode CLT

## Setup

```bash
# 1) Agent backend (Rust) — terminal 1
cd server/agent
cargo build --release
# anzisha agent yako kama kawaida (port default 8080)
# mfano:
# ./target/release/fundi-agent
# au amri yako ya kawaida

# 2) Tauri shell — terminal 2
cd ide-tauri
npm install

# Icons (lazima kabla ya build ya kwanza):
npx tauri icon path/to/icon.png
# au weka icons chini ya src-tauri/icons/

npm run dev
```

## Env (optional)

| Env | Default | Maana |
|-----|---------|--------|
| `MTECHOS_API_BASE` | `http://127.0.0.1:8080` | URL ya agent HTTP |
| `MTECHOS_AGENT_BIN` | `server/agent/target/release/fundi-server` | Binary ya ku-spawn moja kwa moja (fundi-server = API :8080; fundi = CLI/TUI) |

## Production build

```bash
cd ide-tauri
npm run build
```

Artifacts: `ide-tauri/src-tauri/target/release/bundle/`

## Architecture

```
┌─────────────────────────────┐
│  Tauri 2 window (Rust)      │
│  loads boot-canvas.html → canvas (/ide)│
└──────────────┬──────────────┘
               │ fetch /api/*
               ▼
┌─────────────────────────────┐
│  server/agent (Rust Axum)   │
│  Monaco IDE APIs, AI, AKILI │
└─────────────────────────────┘
```

- **Dev**: `devUrl` → `http://127.0.0.1:8080` (agent inayendesha)
- **Build**: `frontendDist` → `server/agent/static` (ide.html + assets)
- Commands: `get_api_base`, `set_api_base`, `start_agent_if_needed`

## Notes

- Providers, Monaco, PLAN, AKILI, MCP UI — **zinabaki kama zilivyo**.
- Kama agent haipo, dirisha linafunguka; API calls zitashindwa hadi agent iwe up.
- Jina la binary linaweza kutofautiana — weka `MTECHOS_AGENT_BIN` sahihi.
