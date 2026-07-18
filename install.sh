#!/bin/bash
# PyTreX Framework — One-Command Install (Linux/macOS)
# Run: chmod +x install.sh && ./install.sh

set -e

echo ""
echo "========================================"
echo "  PyTreX Framework Installer"
echo "  Python + Tauri v2 + Rust + Elixir"
echo "========================================"
echo ""

# 1. Check Python
echo "[1/6] Checking Python..."
if command -v python3 &> /dev/null; then
    PY=python3
    echo "  OK: $($PY --version)"
elif command -v python &> /dev/null; then
    PY=python
    echo "  OK: $($PY --version)"
else
    echo "  ERROR: Python not found. Install Python 3.10+"
    exit 1
fi

# 2. Create virtual environment
echo "[2/6] Creating virtual environment..."
if [ ! -d ".venv" ]; then
    $PY -m venv .venv
    echo "  Created .venv"
else
    echo "  .venv already exists"
fi

# 3. Install Python dependencies
echo "[3/6] Installing Python dependencies..."
./.venv/bin/pip install --upgrade pip
./.venv/bin/pip install -r requirements.txt
./.venv/bin/pip install maturin pytest pytest-asyncio
echo "  Python dependencies installed"

# 4. Check Rust
echo "[4/6] Checking Rust..."
if command -v rustc &> /dev/null; then
    echo "  OK: $(rustc --version)"
else
    echo "  WARNING: Rust not found. Install from https://rustup.rs"
    echo "  Rust is needed to build the native core (my_framework)"
fi

# 5. Build Rust core
echo "[5/6] Building Rust core (PyO3)..."
if command -v rustc &> /dev/null; then
    export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
    ./.venv/bin/python -m maturin develop
    echo "  Rust core built and installed"
else
    echo "  SKIPPED: Rust not installed (Python-only mode)"
fi

# 6. Check Elixir (optional)
echo "[6/6] Checking Elixir (optional)..."
if command -v elixir &> /dev/null; then
    echo "  OK: Elixir found"
else
    echo "  OPTIONAL: Elixir not found. Install from https://elixir-lang.org"
    echo "  Elixir is needed for real-time WebSocket clustering"
fi

# Done
echo ""
echo "========================================"
echo "  Installation Complete!"
echo "========================================"
echo ""
echo "Quick Start:"
echo "  source .venv/bin/activate"
echo "  python -c 'import pytrex; print(\"PyTreX ready!\")'"
echo ""
echo "Run tests:"
echo "  .venv/bin/python -m pytest tests/test_features.py -v"
echo ""
echo "Create a new project:"
echo "  pytrex init MySmartApp"
echo ""
