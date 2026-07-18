# PyTreX Framework

**PyTreX** (Python + Tauri v2 + Rust + Elixir) is a revolutionary, high-performance, Full-Stack AI & Real-Time Desktop-First framework. It empowers developers to build ultra-fast, cross-platform desktop applications using **only Python**, while leveraging the native speed of **Rust**, the lightweight UI of **Tauri v2**, the deep learning power of **PyTorch**, and the massive concurrency of **Elixir (BEAM VM)**.

---

## Key Features

- **Blazing Fast Desktop UI:** Powered by **Tauri v2** & Rust (Wry). No more heavy, memory-hungry Electron apps.
- **On-Device AI Engine:** Native integration with **PyTorch** for real-time computer vision, NLP, and local predictions.
- **Distributed Concurrency:** Powered by **Elixir**, allowing your apps to handle millions of connections and real-time syncing across clusters with ease.
- **Single-Language Developer Experience:** Write simple, high-level Python code, and let PyTreX handle the complex bindings under the hood.
- **Encrypted Database (SQLx):** Auto-migrating SQLite database with AES-256 encryption (SQLCipher) for military-grade data security.
- **Blockchain Engine (SHA-256):** Built-in distributed ledger to prevent data tampering and fraud in financial applications.
- **Container Engine:** Built-in Linux namespace isolation (Docker-like) for secure app deployment.
- **AI Fine-Tuning Studio:** Train and fine-tune AI models locally with live progress broadcasting via Elixir.

---

## Architecture

```
    ┌──────────────────────────────────────────────────────────────────────────────┐
    │                         TAURI v2 HTML5/CSS/JS FRONTEND                       │
    │  (Desktop UI - Inatumia RAM ndogo, Webview Native Engine)                    │
    └──────────────────────────────────────┬───────────────────────────────────────┘
                                           │
                                           │ (Tauri invoke / IPC Channel)
                                           ▼
    ┌──────────────────────────────────────────────────────────────────────────────┐
    │                             RUST CORE BRIDGE (PyO3)                          │
    │  - Inapokea amri za UI na kuzitafsiri kwenda Python GIL                     │
    │  - Inasimamia usalama na ulinzi wa Memory (Memory Safety & Zero-Crashes)    │
    │  - SQLx Encrypted Database (AES-256)                                        │
    │  - Blockchain Engine (SHA-256)                                              │
    └──────────────────────────────────────┬───────────────────────────────────────┘
                                           │
                 ┌─────────────────────────┴─────────────────────────┐
                 │ (Python GIL Binding)                              │ (Linux System Calls)
                 ▼                                                   ▼
    ┌──────────────────────────────────────────┐       ┌───────────────────────────┐
    │        PYTHON CORE MANAGEMENT            │       │  INTERNAL CONTAINER ENGINE│
    │  - Inasimamia @event decorators          │       │  - Linux Namespaces       │
    │  - Logic kuu ya mradi (main.py)          │       │  - chroot / unshare       │
    │  - AI Fine-Tuning Studio                 │       │  - Isolate App Ecosystem  │
    └────────────────────┬─────────────────────┘       └───────────────────────────┘
                         │
          ┌──────────────┴──────────────┐
          │ (Native Memory)             │ (Local IPC / Async Sockets)
          ▼                             ▼
    ┌──────────────────────────┐  ┌────────────────────────────────────────────────┐
    │     PYTORCH AI ENGINE    │  │          ELIXIR CONCURRENCY ENGINE             │
    │  - Local AI Processing   │  │  - BEAM Virtual Machine (Erlang)               │
    │  - Computer Vision / NLP │  │  - Real-time Sockets & Concurrency Broadcast   │
    │  - On-Device Predictions │  │  - Low Latency Clustering (Matawi ya Mfumo)    │
    └──────────────────────────┘  └────────────────────────────────────────────────┘
```

---

## Quick Start

### Prerequisites

- Python >= 3.10
- Rust (via [rustup](https://rustup.rs))
- Node.js >= 18
- Elixir >= 1.16 with Erlang/OTP >= 26 (optional — for real-time clustering)
- Maturin (`pip install maturin`)

### Installation

#### Method 1: One-Command Install (Recommended)

**Windows (PowerShell):**
```powershell
git clone https://github.com/PyTreX/pytrex-framework.git
cd pytrex-framework
.\install.ps1
```

**Linux / macOS:**
```bash
git clone https://github.com/PyTreX/pytrex-framework.git
cd pytrex-framework
chmod +x install.sh
./install.sh
```

This will:
1. Create a Python virtual environment (`.venv`)
2. Install all Python dependencies (`requirements.txt`)
3. Build the Rust native core (`my_framework` via Maturin/PyO3)
4. Install CLI tools (`maturin`, `pytest`)

#### Method 2: Manual Install

```bash
git clone https://github.com/PyTreX/pytrex-framework.git
cd pytrex-framework

# 1. Create virtual environment
python -m venv .venv
source .venv/bin/activate    # Linux/macOS
# .\.venv\Scripts\Activate.ps1   # Windows

# 2. Install Python dependencies
pip install -r requirements.txt
pip install maturin pytest pytest-asyncio

# 3. Build Rust core
# Windows (Python 3.14+):
$env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY='1'
python -m maturin develop

# Linux/macOS:
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 python -m maturin develop

# 4. Verify installation
python -c "import pytrex; print('PyTreX ready!')"
```

#### Method 3: Python-only (No Rust)

If you don't have Rust installed, PyTreX still works in Python-only mode:

```bash
pip install -r requirements.txt
pip install maturin
python -c "from pytrex.core import PyTreXApp; app=PyTreXApp(); print('OK')"
```

> Note: Without Rust, encryption, blockchain, QR generation, and image resize will use Python fallbacks.

### Verify Installation

```bash
# Run the full test suite (481 tests)
python -m pytest tests/test_features.py -v

# Check all modules
python -c "
from pytrex import PyTreXApp
app = PyTreXApp()
print(f'Modules: {len(dir(app))} loaded')
print(f'Neural Network: {app.neural_net is not None}')
print(f'Blockchain: {app.blockchain is not None}')
print(f'Database: {app.db is not None}')
print('PyTreX is ready!')
"
```

### Initialize a New Project

```bash
pytrex init MySmartApp
cd MySmartApp
```

### Write Your App (`main.py`)

```python
from pytrex import PyTreXApp, event
import json

class App(PyTreXApp):
    def __init__(self):
        super().__init__(name="My Smart App")

    @event("ping")
    def ping(self, data):
        return json.dumps({"status": "pong", "received": data})

if __name__ == "__main__":
    app = App()
    app.run()
```

### Run in Development Mode

```bash
pytrex dev
```

### Build for Production

```bash
# Standalone Desktop App
pytrex build local

# Mesh Network (connect office computers without internet)
pytrex build mesh

# Serverless (Docker for Cloud Run / Lambda)
pytrex build serverless

# VPS (Cloud Server)
pytrex build vps
```

---

## Demo Applications

### Smart Bank System (`demos/smart_bank/`)
Banking POS with:
- **PyTorch AI** signature verification
- **Blockchain Engine** (SHA-256) for tamper-proof transaction records
- **SQLx** encrypted database with ACID-compliant transactions
- **Elixir** real-time transaction sync across bank network

### Smart Retail POS (`demos/smart_retail/`)
Retail point-of-sale with:
- **YOLO AI** camera-based product detection
- **Elixir** inventory sync across all branches in real-time

### AI Fine-Tuning Studio
Train AI models locally with Swahili dataset:
- See `demos/my_dataset.json` for sample Swahili training data
- Use `pytrex/finetune.py` for fine-tuning with live progress via Elixir

---

## Project Structure

```
pytrex/
├── pyproject.toml              # Python package config + Maturin build
├── Cargo.toml                  # Rust dependencies (Tauri v2, PyO3, SQLx, sha2)
├── tauri.conf.json             # Tauri v2 configuration
├── build.rs                    # Tauri build script
├── src/
│   └── lib.rs                  # Rust core: PyO3 + Tauri + SQLx + Blockchain + Container
├── pytrex/                     # Python package
│   ├── __init__.py             # Public exports
│   ├── core.py                 # Event registry, app lifecycle, Elixir client, blockchain cache
│   ├── cli.py                  # CLI tool (init, dev, build, containerize)
│   └── finetune.py             # AI Fine-Tuning Studio (PyTorch training)
├── pytrex_engine/              # Elixir OTP application
│   ├── mix.exs
│   ├── config/config.exs
│   └── lib/
│       ├── pytrex_engine.ex            # Application supervisor
│       ├── websocket_server.ex         # WebSocket server (Plug/Cowboy)
│       ├── websocket_handler.ex        # WebSocket message handler
│       ├── task_dispatcher.ex          # Async task processing
│       └── cluster_manager.ex          # BEAM cluster management
├── frontend/
│   └── index.html              # Default frontend template
├── demos/                      # Demo applications
│   ├── smart_bank/             # Banking POS (Blockchain + AI + SQLx)
│   ├── smart_retail/           # Retail POS (YOLO + Elixir)
│   └── my_dataset.json         # Swahili AI training dataset
├── README.md
├── LICENSE
└── .gitignore
```

---

## Rust Core Functions (via PyO3)

| Function | Description |
|----------|-------------|
| `fanya_app()` | Opens Tauri v2 desktop window |
| `anzisha_container(root_path)` | Isolates app in Linux container (namespaces) |
| `kuandaa_database_salama(db_path, key)` | Creates encrypted SQLite database (AES-256) |
| `fanya_muamala_salama(acc_no, type, amount)` | Executes ACID-compliant bank transaction |
| `fanya_block_ya_blockchain(data)` | Creates SHA-256 blockchain block |
| `hakiki_blockchain(chain_json)` | Audits blockchain for tampering |

---

## Contributing

PyTreX is an open-source project. If you are passionate about Python, Rust, Elixir, or AI engineering, feel free to fork this repository, open issues, and submit pull requests!

## License

This project is licensed under the MIT License.
