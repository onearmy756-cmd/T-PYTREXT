# IDE + Tauri — status ya ukamilishaji

## Imefanywa ✅

| Kipengele | Maelezo |
|-----------|----------|
| Monaco + Zed UI | ide.html + CSS + core JS (gzip fallback) |
| PLAN quick buttons | Website / Mobile / IoT / Gaming |
| Terminal / Search wiring | `runCmd` / `runSearch` + aliases |
| Providers + logos (18) | ide-providers.js |
| API key flow | save → `/api/ai/custom` + `/api/ai/key` |
| Per-provider keys (local) | ide-finish.js → localStorage + headers |
| Extensions UI | tool-groups |
| MCP UI + list tools | localStorage servers + `/api/ide/tool-calls` |
| ACP UI | health/config ping |
| Tauri v2 scaffold | ide-tauri/ |
| CI workflow | `.github/workflows/ide-tauri.yml` |
| Loader dual-path | plain chunks **au** gzip parts |

## Bado / partial ⚠️

| Kipengele | Hali |
|-----------|------|
| `ide-js-0..11.js` plain chunks | **Bado hazijapakiwa zote** — loader inatumia **gzip fallback** (`ide-gz-p0..3`) ambayo **inafanya kazi** |
| MCP protocol JSON-RPC full | UI + tool-calls tu, si MCP server binary |
| ACP protocol full | health/config tu |
| Tauri icons committed | CI inazizalisha; local: `npx tauri icon` |
| Agent sidecar bundled | spawn optional via `MTECHOS_AGENT_BIN` |
| Monaco offline | bado CDN |
| Native Anthropic headers | OpenAI-compatible path |

## Jinsi ya kuendesha sasa

```bash
# Agent
cd server/agent && cargo run --release   # port 8080

# Browser
open http://127.0.0.1:8080/ide.html

# Desktop
cd ide-tauri && npm i && npm run dev
```

Console inapaswa kuonyesha: `[IDE] loaded 28xxx` (kutoka gzip au plain).
