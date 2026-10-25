

# ⚡ Fundi Agent `v0.1`

**The self-evolving AI agent. One binary. Every platform.**

Rust rewrite of [Fundi Agent](https://github.com/Lumio-Research/hermes-agent-rs) by [Lumio Research](https://github.com/Lumio-Research).

`110,000+ lines of Rust` · `1,428 tests` · `17 crates` · `~16MB binary`

**[English](./README.md)** · **[中文](./README_ZH.md)** · **[日本語](./README_JA.md)** · **[한국어](./README_KO.md)**



---

> **v0.1 Status:** Core agent loop, 10 LLM providers, 30 tool backends, 17 platform adapters, memory system, and CLI/TUI are production-ready.

## Project Status

This repository is the Lumio Research Rust port of Fundi Agent.

The `main` branch includes OpenAI-compatible function tool payload normalization
for strict providers such as DeepSeek, Moonshot/Kimi, SiliconFlow, and other
OpenAI-compatible endpoints. Users on the `v0.1.1` release binaries should build
from `main` or use a release newer than `v0.1.1` for that fix.

## Why Fundi?

🚀 **Zero dependencies** — Single static binary. No Python, no pip, no Docker. Copy it to a Raspberry Pi, a $3 VPS, or an air-gapped server and run it.

🧠 **Self-evolution engine** — Multi-armed bandit model selection, long-task planning, and prompt/memory shaping. The agent gets better the more you use it.

🔌 **17 platforms, 30+ tools, 8 memory backends** — Telegram, Discord, Slack, WhatsApp, Signal, Matrix, and 11 more. File ops, browser, code execution, vision, voice, web search, Home Assistant, and beyond.

⚡ **True concurrency** — Rust's tokio runtime dispatches tool calls across OS threads. A 30-second browser scrape doesn't block a 50ms file read. No GIL.

## 🧠 Akili Agent V4.0 (Production)

**Human-like 12-layer thinking** integrated into `fundi-agent`:

| Layer | Name |
|-------|------|
| 1 | Attention |
| 2 | Perception |
| 3 | System 2 (Slow Thinking) |
| 4 | System 1 (Fast Thinking) |
| 5 | Subconscious |
| 6 | Intuition |
| 7 | Imagination |
| 8 | Emotional Intelligence |
| 9 | Social Cognition |
| 10 | Values & Ethics |
| 11 | Meaning & Purpose |
| 12 | Self-Identity |

**Location:** `crates/fundi-agent/src/akili_v4.rs` + `akili_integration.rs`

**Production API:**

```rust
use fundi_agent::{akili_decide, akili_gate, requires_hitl, AkiliAgentV4};

// Full decision (JSON)
let decision = akili_decide("Nataka deploy Ubuntu kwa PCs 10");

// Ethics + meaning gate before dangerous tools
let (allowed, reason, score) = akili_gate("deploy pxe", "user request");
if !allowed { /* block */ }

// HITL check
if requires_hitl("format disk") { /* ask user */ }

// Stateful agent
let mut akili = AkiliAgentV4::new();
let resp = akili.live("Kompyuta inaenda polepole");
```

**Features (production):**
- Ethics gate blocks dangerous actions
- HITL required for deploy / format / delete / remediate
- System 1 (fast patterns) + System 2 (deliberate)
- Intuition + emotion + social + meaning alignment
- Tracing logs (`target: akili_v4`)
- Stable JSON for API / dashboard / agent_loop

## Quick Start

```bash
# Install (auto-detects OS and CPU)
cargo install --path server/agent/crates/fundi-cli

# Set your API key
echo "ANTHROPIC_API_KEY=sk-..." >> ~/.fundi/.env

# Run interactive session
fundi
```

That's it. You're in an interactive session with tool access, memory, and streaming.

## What Can It Do?

**Chat with any LLM** — switch models mid-conversation:

```
fundi
> /model gpt-4o
> Summarize this repo and find security issues
```

**One-shot tasks** from the command line:

```bash
fundi chat --query "Refactor auth.rs to use the new error types"
```

**Start the full server** — API + platform gateway + cron, all in one:

```bash
fundi serve                          # API + gateway + cron
fundi serve --no-gateway             # API only (for dashboard UI)
fundi serve --no-gateway --no-cron   # API only (minimal)
```

**Multi-platform gateway** — connect Telegram, Discord, Slack, and more simultaneously:

```bash
fundi gateway start
```

**Run anywhere** — Docker, SSH, or remote sandboxes:

```yaml
# ~/.fundi/config.yaml
terminal:
  backend: docker
  image: ubuntu:24.04
```

**MCP + ACP** — connect external tool servers or expose Fundi as one:

```yaml
mcp:
  servers:
    - name: my-tools
      command: npx my-mcp-server
```

**Voice mode** — VAD + STT + TTS pipeline for hands-free interaction.

## Architecture

```
fundi-agent-rs/
├── crates/
│   ├── fundi-cli              # Binary entry point, TUI, slash commands
│   ├── fundi-agent            # Agent loop, LLM providers, memory plugins + Akili V4
│   ├── fundi-core             # Shared types, traits, error hierarchy
│   ├── fundi-intelligence     # Model routing, prompt building, self-evolution
│   ├── fundi-config           # Config loading, YAML/env merging
│   ├── fundi-tools            # 30+ tool backends, approval engine
│   ├── fundi-gateway          # 17 platform adapters, session management
│   ├── fundi-server           # HTTP/WebSocket API server
│   ├── fundi-runtime          # Unified runtime builder (server + gateway + cron)
│   ├── fundi-environments     # Terminal: Local/Docker/SSH/Daytona/Modal/Singularity
│   ├── fundi-mcp              # Model Context Protocol client/server
│   ├── fundi-acp              # Agent Communication Protocol
│   ├── fundi-skills           # Skill management and hub
│   ├── fundi-cron             # Cron scheduling
│   ├── fundi-bus              # In-process message bus (agent service abstraction)
│   ├── fundi-eval             # SWE-bench, Terminal-Bench, YC Bench
│   └── fundi-telemetry        # OpenTelemetry + Prometheus
├── apps/
│   ├── dashboard/              # Web management UI (React + Vite + Tailwind)
│   ├── web-app/                # Web app + Electron desktop shell (macOS/Windows/Linux)
│   └── mobile-app/             # Mobile client (Flutter — iOS/Android)
└── scripts/                    # Install, CI, and convenience scripts
```

**Key traits:** `LlmProvider` (10 providers) · `ToolHandler` (30+ backends) · `PlatformAdapter` (17 platforms) · `TerminalBackend` (6 backends) · `MemoryProvider` (8 plugins) · `AgentService` (local/remote transparent)

**Tool call parsers:** Fundi, Anthropic, OpenAI, Qwen, Llama, DeepSeek, Auto

## CLI Commands

```
fundi                    # Interactive REPL (default)
fundi chat               # Single-shot query
fundi serve              # Start API server + gateway + cron
fundi gateway            # Platform gateway only (no API)
fundi config             # Configuration management
fundi model              # Model / provider management
fundi auth               # Authentication (login/logout/status)
fundi tools              # Tool management
fundi mcp                # MCP server management
fundi skills             # Skill management
fundi plugins            # Plugin management
fundi sessions           # Session management
fundi memory             # Memory provider management
fundi cron               # Cron job management
fundi setup              # Interactive setup wizard
fundi doctor             # Health check
fundi status             # Running status
fundi version            # Version info
```

## Install

**One-liner** (auto-detects OS and CPU):

```bash
cargo install --path server/agent/crates/fundi-cli
```

**From source:**

```bash
cargo install --path server/agent/crates/fundi-cli --locked
```

**Manual download:** grab the binary for your platform from [Releases](https://github.com/Lumio-Research/hermes-agent-rs/releases).

**Docker:**

```bash
docker run --rm -it -v ~/.fundi:/root/.fundi ghcr.io/lumio-research/fundi-agent-rs
```

## Development

**Prerequisites:** Rust 1.75+, Node.js 20+ (for dashboard/client apps)

```bash
# Build the CLI
cargo build -p fundi-cli

# Run directly (skips cargo lock contention with rust-analyzer)
scripts/fundi

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all --check
```

**Dashboard UI** (web management panel):

```bash
cd apps/dashboard
npm install
npm run dev          # Vite dev server with HMR, proxies /api to fundi serve
```

**Client app** (Web + Desktop shared project):

```bash
cd apps/web-app
pnpm install
pnpm dev:web
```

**Electron desktop shell** (Linux / macOS / Windows):

```bash
cd apps/web-app
pnpm dev:desktop
```

Package artifacts:

```bash
pnpm dist
```

Parity checklist is tracked in `apps/web-app/electron/TAURI_PARITY.md`.

**Mobile app** (React Native + Expo — iOS / Android):

```bash
cd apps/mobile-app
pnpm install
pnpm ios            # iOS simulator
pnpm android        # Android emulator
pnpm start          # Expo dev server (Expo Go)
```

Shares types and API client with `apps/web-app`. Features:
- Session list + new chat
- Streaming chat via WebSocket
- Settings (API URL, token) persisted with AsyncStorage

**Protocol SDK** (IDE / external client integration):

```bash
cd sdk/typescript
pnpm install
pnpm build
pnpm test            # protocol contract tests
FUNDI_E2E_BASE=http://127.0.0.1:8787 pnpm test:e2e
FUNDI_SCHEMA_BASE=http://127.0.0.1:8787 pnpm sync:protocol
```

**One-command local protocol e2e** (auto start/stop `fundi-server`):

```bash
bash scripts/ci/protocol-e2e-local.sh
```

## Contributing

Contributions welcome. Before submitting a PR:

1. Run the full test suite:
   ```bash
   cargo test --workspace                                    # 1,428 tests
   cargo clippy --workspace --all-targets -- -D warnings     # Lint (warnings fail CI)
   cargo fmt --all --check                                   # Format
   ```

2. Run smoke tests:
   ```bash
   bash scripts/ci/smoke.sh          # Release binary smoke check
   bash scripts/ci/keypath-e2e.sh    # Core end-to-end paths
   bash scripts/ci/protocol-sdk-check.sh   # Protocol SDK contracts + optional e2e smoke
   bash scripts/ci/protocol-e2e-local.sh   # Local server + protocol SDK e2e smoke
   bash scripts/ci/electron-check.sh       # Electron shell build check
   bash scripts/ci/mobile-check.sh         # React Native type check + tests
   bash scripts/ci/protocol-matrix-check.sh # Rust↔TS↔Flutter protocol fixtures compatibility
   ```

3. If you changed `apps/dashboard`:
   ```bash
   cd apps/dashboard && npm run build
   ```

See [AGENTS.md](AGENTS.md) for architecture details and coding conventions.
For multi-client CI/CD operations, see [CI_CD_RUNBOOK.md](CI_CD_RUNBOOK.md).

## License

MIT — see [LICENSE](LICENSE).

Based on [Fundi Agent](https://github.com/Lumio-Research/hermes-agent-rs) by [Lumio Research](https://github.com/Lumio-Research).
