# 🏗️ T-PYTREXT Architecture

> System design, design decisions, and how everything fits together.

---

## Overview

T-PYTREXT is a **polyglot framework** combining three languages for three purposes:

| Language | Purpose | Engine |
|----------|---------|--------|
| **Python** | Business Logic, AI/ML, User Code | CPython 3.10+ |
| **Rust** | Speed-Critical Operations, Security | Compiled via PyO3 (maturin) |
| **Elixir** | Real-Time Communication, Clustering | BEAM VM (Erlang/OTP) |

---

## Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│  LAYER 1: PRESENTATION                                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Tauri v2 (Rust/Wry)                              │  │
│  │  • Lightweight webview (not Chromium!)            │  │
│  │  • IPC bridge: invoke("tauri_to_python", data)    │  │
│  │  • 900×700 default, resizable, multi-window       │  │
│  └───────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│  LAYER 2: RUST CORE (PyO3)                              │
│  ┌──────────┬──────────┬──────────┬─────────────────┐  │
│  │ Blockchain│ Encryption│ Database │ Container       │  │
│  │ SHA-256  │ AES-256  │ SQLx     │ Linux Namespace │  │
│  ├──────────┼──────────┼──────────┼─────────────────┤  │
│  │ Axum HTTP│ Candle ML│ Burn DL  │ MCP Protocol    │  │
│  │ Server   │ Inference│ Training │ Server/Client   │  │
│  └──────────┴──────────┴──────────┴─────────────────┘  │
│  • 39 #[pyfunction] exports via maturin                 │
│  • Graceful fallback: Python-only mode if not compiled  │
├─────────────────────────────────────────────────────────┤
│  LAYER 3: PYTHON CORE                                   │
│  ┌───────────────────────────────────────────────────┐  │
│  │  PyTreXApp — Main Application Hub                 │  │
│  │  • @event decorator → REGISTERED_EVENTS dict      │  │
│  │  • 130+ service objects auto-initialized          │  │
│  │  • EventBus: in-app pub/sub                        │  │
│  │  • RateLimiter: sliding window, 100/min default    │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  Agents & AI                                       │  │
│  │  • LangChainAgent — chains, tools, RAG            │  │
│  │  • HermesAgent — function-calling, 5 built-ins    │  │
│  │  • SearchEngine — SearXNG + DuckDuckGo            │  │
│  │  • MCPClient — Model Context Protocol             │  │
│  │  • HumanInTheLoop — approval workflows            │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  Management & Deployment                           │  │
│  │  • TestRunner — 38 tests, 12 modules              │  │
│  │  • ProjectManager — scan, run, test, build        │  │
│  │  • ProductionBuilder — Docker/k8s/Cloud deploy    │  │
│  └───────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│  LAYER 4: ELIXIR BEAM VM                                │
│  ┌───────────────────────────────────────────────────┐  │
│  │  WebSocket Server (Plug/Cowboy)                   │  │
│  │  • Port 42351 — persistent connection              │  │
│  │  • JSON messages: {event, payload, broadcast}      │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  TaskDispatcher (async supervisor)                 │  │
│  │  • 18 event handlers                               │  │
│  │  • 30s timeout per task                            │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  ClusterManager (GenServer)                        │  │
│  │  • BEAM node connections                          │  │
│  │  • Cross-node :rpc.call broadcasting              │  │
│  │  • Cluster cookie authentication                  │  │
│  ├───────────────────────────────────────────────────┤  │
│  │  MCPHandler — Model Context Protocol              │  │
│  │  • 6 tools, 3 resources, 3 prompts               │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Data Flow

### 1. Frontend → Python (Tauri IPC)

```
User clicks button in HTML
  ↓
window.__TAURI__.invoke("tauri_to_python", {event_name, data})
  ↓
Rust #[tauri::command] tauri_to_python()
  ↓
Python::with_gil → pytrex.core.execute_python_event()
  ↓
REGISTERED_EVENTS[name](data) → returns JSON
  ↓
Back to frontend
```

### 2. Python → Elixir (WebSocket)

```
Python ElixirClient.emit("event", payload)
  ↓
WebSocket → localhost:42351/ws
  ↓
Elixir WebSocketHandler → JSON decode
  ↓
TaskDispatcher.process_task(event, payload)
  ↓
Optional: ClusterManager.broadcast() to other nodes
```

### 3. Python → Rust (PyO3 Direct)

```
Python: my_framework.fanya_block_ya_blockchain(data)
  ↓
PyO3: #[pyfunction] fn fanya_block_ya_blockchain()
  ↓
Rust: Sha256::digest() + Block::new()
  ↓
Returns JSON string to Python
```

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **PyO3 (not ctypes)** | Zero-copy memory sharing, native Python objects |
| **Tauri v2 (not Electron)** | 10x less RAM, no Chromium, Rust security |
| **Elixir BEAM (not Node.js)** | Millions of concurrent connections, fault-tolerant |
| **Single core.py file** | Simplicity — one file to understand the whole system |
| **Graceful fallback** | Works without Rust/Elixir — pure Python mode available |
| **Swahili function names** | Tanzania-first design, global-ready code |
| **Modular agents** | Each AI agent is independent and replaceable |
| **14 deploy configs** | Production-ready from day 1 |

---

## Security Architecture

```
┌─────────────────────────────────────┐
│  INPUT VALIDATION                   │
│  • InputValidator class              │
│  • Rate limiting (100/min/window)   │
├─────────────────────────────────────┤
│  ENCRYPTION LAYER                   │
│  • AES-256-GCM (Rust aes-gcm)       │
│  • SHA-256 hashing (Rust sha2)      │
│  • Key derivation via SHA-256       │
│  • Memory zeroization (zeroize)     │
├─────────────────────────────────────┤
│  DATABASE SECURITY                  │
│  • SQLx encrypted (PRAGMA key)      │
│  • AES-256 at rest                  │
│  • ACID-compliant transactions       │
├─────────────────────────────────────┤
│  BLOCKCHAIN INTEGRITY               │
│  • SHA-256 block hashing            │
│  • Immutable chain verification     │
│  • Tamper detection                 │
├─────────────────────────────────────┤
│  PRODUCTION HARDENING               │
│  • Nginx: SSL, rate limiting        │
│  • Security headers (XSS, HSTS)     │
│  • Non-root Docker user             │
│  • Systemd: NoNewPrivileges, etc.   │
└─────────────────────────────────────┘
```

---

## Performance Characteristics

| Operation | Speed | Engine |
|-----------|-------|--------|
| Event dispatch | 55,506/sec | Python |
| Blockchain create | 1,898/sec | Python fallback |
| AES-256 encrypt | 10,892/sec | Python fallback |
| SHA-256 hash | 359,274/sec | Python hashlib |
| App startup | 117ms | Python + Rust init |

> **With Rust core compiled (maturin develop): 10-100x faster for crypto/DB/blockchain operations.**
