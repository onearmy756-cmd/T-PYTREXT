# MBILINYITECH IDE — Canvas Extension App

The MTECHOS IDE, packaged as an App for **OpenHands Agent Canvas**. It runs
inside the Canvas UI as a routed page and talks to the MTECHOS Rust agent
(`/api/ide/*`, `/api/akili/*`, `/api/agent/*`) and to the owning Agent Server.

## What it contributes

One page, `ide`, at route `/mbilinyitech-ide` (nav label `MBILINYITECH IDE`).

The page ships two surfaces so no MTECHOS IDE feature is dropped:

- **Native app panel** (bundled into `extension.js`):
  - **Monaco editor** (VS Code engine, `zed-one-dark` theme, offline bundled — no CDN)
  - **MONOCODE AI** — Cursor/Devin-style coding surface:
    - ghost-text **inline autocomplete** via `/api/ide/complete` (FIM), with a
      ✨ AI on/off toggle in the toolbar
    - **Cmd/Ctrl+K inline edit** (or the ⌘K Edit button) via `/api/ide/edit`
  - **Autopilot** (Website / Mobile / IoT / Gaming) via `/api/ide/agent/run`
  - file tree + file open/save via `/api/ide/file`, `/api/ide/tree`
  - terminal via `/api/ide/terminal`
  - agent chat via `/api/agent/chat`, health via `/api/agent/health`
  - provider URL / model / key settings (stored in `localStorage`)
- **Full IDE** button: embeds the complete MTECHOS canvas (`/ide`, swarm of 50
  agents, LSP, DAP, E2E, emulators, AKILI, providers, cyber ops) in an
  iframe, so every existing feature stays reachable.

## Build

```bash
npm install
npm run build          # -> dist/extension.js (one self-contained ESM file)
npm run validate       # static Canvas App validator
npm test               # unit tests (real app code + host test double)
cp dist/extension.js extension.js   # sync the checked-in entrypoint
```

The Vite build produces exactly one file: Monaco CSS is inlined and the two
Monaco worker-bootstrap dynamic imports are rewritten, because the Canvas host
loads the entrypoint as a Blob ESM module and rejects runtime dynamic imports.

## Install in Agent Canvas

Customize → Apps → **Add app**:

- **source**: this directory (backend-local), or the repository URL
- **repo_path**: `platforms/mbilinyitech-ide` (remote)

Installation leaves the app disabled. Enabling it runs trusted same-realm code.
Then open **MBILINYITECH IDE** in the sidebar.

## Configuration

The MTECHOS API base defaults to `http://127.0.0.1:8080`. Set it at runtime
either through the page's ⚙️ Settings or by defining before Canvas loads:

```html
<script>window.__MTECHOS__ = { apiBase: "http://127.0.0.1:8080" };</script>
```

## Limitations (Canvas Extensions API v1)

- Pages are the only contribution type in host API 1.
- Enabled app code is trusted same-realm JavaScript; there is no sandbox.
