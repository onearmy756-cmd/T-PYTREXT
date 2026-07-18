# 🚀 T-PYTREXT Framework

> **Full-Stack AI & Real-Time Desktop Framework** — Build Ultra-Fast, Production-Ready Apps with Python Alone.

<p align="center">
  <img src="https://img.shields.io/badge/Python-3.10+-blue?logo=python" alt="Python">
  <img src="https://img.shields.io/badge/Rust-1.75+-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Elixir-1.16+-purple?logo=elixir" alt="Elixir">
  <img src="https://img.shields.io/badge/Tauri-v2-green?logo=tauri" alt="Tauri">
  <img src="https://img.shields.io/badge/AI-PyTorch%20%7C%20Candle%20%7C%20Burn-red?logo=pytorch" alt="AI">
  <img src="https://img.shields.io/badge/Blockchain-SHA--256-gold" alt="Blockchain">
  <img src="https://img.shields.io/badge/License-MIT-brightgreen" alt="License">
  <img src="https://img.shields.io/badge/Tests-96%2F96%20PASSED-success" alt="Tests">
</p>

---

## ⚡ Why T-PYTREXT?

| Problem | PyTreXT Solution |
|---------|-----------------|
| Electron apps are slow & heavy (500MB RAM) | **Tauri v2 + Rust** — 10x lighter, 5x faster |
| Need 3-4 frameworks to build full app | **One framework** — Python + Rust + Elixir unified |
| Security is an afterthought | **Built-in**: AES-256, SHA-256 Blockchain, Encrypted DB |
| AI integration is complex | **Plug & play**: LangChain, Hermes Agent, PyTorch, Candle, Burn |
| Deployment takes days | **One command**: `pytrex deploy` → Docker/k8s/Cloud |
| No real-time capability | **Elixir BEAM** — millions of concurrent connections |

---

## 📊 By The Numbers

| Metric | Value |
|--------|-------|
| **Total Code** | 41,000+ lines (Python + Rust + Elixir) |
| **Classes** | 220+ |
| **CLI Commands** | 15 |
| **Demo Projects** | 8 (Bank, Retail, Chat, Crypto, Hospital, Exam, Real Estate, Voting) |
| **Tests** | 96/96 PASSED (100%) |
| **Rust PyO3 Functions** | 39 |
| **Production Configs** | 14 (Docker, k8s, Nginx, Cloud, Systemd) |
| **Events/Sec** | 55,506 ⚡ |
| **App Startup** | 117ms 🚀 |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│              TAURI v2 DESKTOP UI (HTML/CSS/JS)          │
│              Lightweight Webview — no Chromium           │
├─────────────────────────────────────────────────────────┤
│                   RUST CORE (PyO3)                       │
│  ┌──────────┬──────────┬──────────┬──────────────────┐  │
│  │ Axum HTTP│ SQLx DB  │ Blockchain│ AES-256 Crypto  │  │
│  │ Server   │ (Encrypt)│ (SHA-256) │ + Zlib Compress │  │
│  ├──────────┼──────────┼──────────┼──────────────────┤  │
│  │ Candle ML│ Burn DL  │ MCP Proto│ Container Engine │  │
│  └──────────┴──────────┴──────────┴──────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                PYTHON CORE (220+ Classes)                │
│  ┌──────────┬──────────┬──────────┬──────────────────┐  │
│  │PyTreXApp │ @event   │ LangChain│ Hermes Agent     │  │
│  ├──────────┼──────────┼──────────┼──────────────────┤  │
│  │RAG Engine│ Search   │ HITL     │ MCP Client       │  │
│  ├──────────┼──────────┼──────────┼──────────────────┤  │
│  │TestRunner│ Project  │ Production│ CLI (15 commands)│  │
│  │          │ Manager  │ Builder  │                  │  │
│  └──────────┴──────────┴──────────┴──────────────────┘  │
├─────────────────────────────────────────────────────────┤
│           ELIXIR BEAM VM (Real-Time Engine)              │
│  WebSocket Server • TaskDispatcher • ClusterManager      │
│  MCP Handler • Millions of concurrent connections        │
└─────────────────────────────────────────────────────────┘
```

---

## 🚀 Quick Start (60 Seconds)

```bash
# 1. Clone
git clone https://github.com/onearmy756-cmd/T-PYTREXT.git
cd T-PYTREXT

# 2. Install
pip install -r requirements.txt

# 3. Verify
python -c "from pytrex import PyTreXApp; print('✅ PyTreXT Ready!')"

# 4. Create your first app
pytrex init MyFirstApp
cd MyFirstApp

# 5. Run!
pytrex dev
```

---

## 📦 What Can You Build? (8 Live Demos)

| # | Demo | Key Features | Run |
|---|------|-------------|-----|
| 1 | 🏦 **Smart Bank** | Blockchain, AES-256, AI Signatures | `python demos/smart_bank/main.py` |
| 2 | 🛒 **Smart Retail** | YOLO AI, Real-time Inventory | `python demos/smart_retail/main.py` |
| 3 | 💬 **Chat System** | WebSocket, E2E Encryption, Groups | `python demos/chat_app/main.py` |
| 4 | 🤖 **Crypto Bot** | AI Trading, Blockchain, HITL | `python demos/crypto_bot/main.py` |
| 5 | 🏥 **Hospital System** | Encrypted EHR, AI Diagnosis | `python demos/hospital_system/main.py` |
| 6 | 📚 **AI Exam System** | RAG, Hermes AI, Auto-Grading | `python demos/ai_exam/main.py` |
| 7 | 🏠 **Real Estate** | AI Valuation, Contracts, Payments | `python demos/real_estate/main.py` |
| 8 | 🗳️ **Blockchain Voting** | Immutable Votes, Anti-Fraud, HITL | `python demos/voting_system/main.py` |

---

## ⚡ Performance (Benchmarked)

```
55,506 events/sec    ⚡  Event System
 1,898 blocks/sec    🔗  Blockchain
10,892 encrypt/sec   🔐  AES-256
359,274 hashes/sec   🔑  SHA-256
   117ms startup     🚀  App Ready
 7,543 async/sec     ⚡  8-Thread Parallel
```

---

## 🔧 CLI Commands (15 Total)

```bash
pytrex init <name>         # Create new project
pytrex dev                 # Dev mode with hot-reload
pytrex build <target>      # Build: local|mesh|serverless|vps|android|ios|web
pytrex containerize        # Linux container isolation

pytrex scan                # Discover all PyTreXT projects
pytrex projects            # List discovered projects
pytrex run <name>          # Run any project
pytrex test                # Test all modules (38 tests)
pytrex test-project <name> # Test specific project
pytrex build-project <name># Build project for production

pytrex deploy              # Deploy to production (Docker/k8s/Cloud)
pytrex deploy-quick        # Quick Docker deploy
pytrex dashboard           # Open project dashboard
pytrex export              # Export project info to JSON
pytrex watch <name>        # Watch mode (auto-restart)
```

---

## 🐍 Python API (Quick Examples)

```python
from pytrex import PyTreXApp, event

# Create app
app = PyTreXApp(name="My App")

# Register events
@event("ping")
def ping(data):
    return "pong!"

# Blockchain
from pytrex import BlockchainBridge
bc = BlockchainBridge()
bc.add_block("transaction_data")
bc.verify_chain()

# AI Agent
from pytrex import HermesAgent, LangChainAgent
hermes = HermesAgent()
hermes.chat("What is the capital of Tanzania?")

# Search
from pytrex import WebSearchEngine
search = WebSearchEngine()
search.search("PyTreX framework")

# Human-in-the-Loop
from pytrex import HumanInTheLoop
hitl = HumanInTheLoop()
aid = hitl.request_approval("deploy", {"env": "prod"})
hitl.approve(aid)

# Production Deploy
from pytrex import deploy
deploy(".", target="docker")
```

---

## 🌍 Deployment Targets

| Target | Command | Output |
|--------|---------|--------|
| 🐳 Docker | `pytrex deploy -t docker` | Docker image + compose |
| 🖥️ Standalone | `pytrex deploy -t standalone` | .exe / .app / binary |
| ☁️ AWS | `pytrex deploy -t aws` | ECS task + deploy script |
| ☁️ GCP | `pytrex deploy -t gcp` | Cloud Run + deploy script |
| ☁️ Azure | `pytrex deploy -t azure` | Container Instances |
| ☸️ Kubernetes | `pytrex deploy -t k8s` | Deployment + Service + HPA |
| 🐧 VPS | `pytrex deploy -t vps -d myapp.com` | Nginx + SSL + Systemd |
| 🪟 Windows | `pytrex deploy -t systemd` | Windows Service |

---

## 📂 Project Structure

```
T-PYTREXT/
├── pytrex/                  # Python Package (13 modules)
│   ├── core.py              # 11,700+ lines — PyTreXApp + 220 classes
│   ├── cli.py               # 15 CLI commands
│   ├── langchain_agent.py   # LangChain AI Agent
│   ├── search_engine.py     # SearXNG + DuckDuckGo
│   ├── human_in_loop.py     # HITL approval workflows
│   ├── hermes_agent.py      # Function-calling AI agent
│   ├── mcp_client.py        # MCP protocol client
│   ├── test_runner.py       # Test runner (38 tests, 12 modules)
│   ├── project_manager.py   # Project discovery & management
│   ├── production.py        # Production deployment builder
│   ├── finetune.py          # AI fine-tuning studio
│   └── templates/           # Project templates
├── src/lib.rs               # Rust core — 1,800+ lines, 39 PyO3 functions
├── pytrex_engine/           # Elixir OTP application
│   ├── lib/                 # 7 modules (WS, Tasks, Cluster, MCP)
│   └── config/
├── demos/                   # 8 Live Demo Projects
├── deploy/                  # 14 Production Config Files
├── frontend/                # UI templates + 3D Logo
├── tests/                   # Test suites (96 tests)
└── docs/                    # Documentation
```

---

## 🤝 For Future Generations

This framework is built to last. Here's how we ensure it:

- ✅ **MIT License** — Free forever, for everyone
- ✅ **96 Tests** — Every feature is verified
- ✅ **8 Demos** — Working examples for every use case
- ✅ **14 Deploy Configs** — Production-ready out of the box
- ✅ **Full Documentation** — README, API docs, Architecture, Getting Started
- ✅ **3 Languages** — Python (easy), Rust (fast), Elixir (scalable)
- ✅ **Modular Design** — Each module is independent and replaceable
- ✅ **Open Source** — Anyone can fork, improve, contribute

---

## 📚 Documentation

- **[GETTING_STARTED.md](GETTING_STARTED.md)** — Beginner's guide
- **[API_REFERENCE.md](API_REFERENCE.md)** — Full API documentation (220+ classes)
- **[ARCHITECTURE.md](ARCHITECTURE.md)** — System design & decisions
- **[ROADMAP.md](ROADMAP.md)** — Future plans
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — How to contribute

---

## 🧪 Running Tests

```bash
# All tests
pytrex test

# Specific module
pytrex test -m Blockchain -m Encryption

# Extended features
python -m pytest tests/test_extended_features.py -v

# Quick test
pytrex test --quick
```

---

## 👤 Author

**DR MBILINYI** — Creator of T-PYTREXT Framework

> "Build once, deploy everywhere. Python for the mind, Rust for the speed, Elixir for the scale."

---

## 📄 License

MIT License — Free for personal, commercial, educational, and government use.

See [LICENSE](LICENSE) for full details.

---

<p align="center">
  <b>⭐ Star this repo if you find it useful! ⭐</b><br>
  <i>Built with ❤️ in Tanzania 🇹🇿 | For Developers Worldwide 🌍</i>
</p>
