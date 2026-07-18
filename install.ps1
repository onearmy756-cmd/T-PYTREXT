# PyTreX Framework — One-Command Install (Windows PowerShell)
# Run: .\install.ps1

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  PyTreX Framework Installer" -ForegroundColor Cyan
Write-Host "  Python + Tauri v2 + Rust + Elixir" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Check Python
Write-Host "[1/6] Checking Python..." -ForegroundColor Yellow
try {
    $pyVersion = python --version 2>&1
    Write-Host "  OK: $pyVersion" -ForegroundColor Green
} catch {
    Write-Host "  ERROR: Python not found. Install Python 3.10+ from https://python.org" -ForegroundColor Red
    exit 1
}

# 2. Create virtual environment
Write-Host "[2/6] Creating virtual environment..." -ForegroundColor Yellow
if (-not (Test-Path ".venv")) {
    python -m venv .venv
    Write-Host "  Created .venv" -ForegroundColor Green
} else {
    Write-Host "  .venv already exists" -ForegroundColor Green
}

# 3. Install Python dependencies
Write-Host "[3/6] Installing Python dependencies..." -ForegroundColor Yellow
& .\.venv\Scripts\pip.exe install --upgrade pip
& .\.venv\Scripts\pip.exe install -r requirements.txt
& .\.venv\Scripts\pip.exe install maturin pytest pytest-asyncio
Write-Host "  Python dependencies installed" -ForegroundColor Green

# 4. Check Rust
Write-Host "[4/6] Checking Rust..." -ForegroundColor Yellow
try {
    $rustVersion = rustc --version 2>&1
    Write-Host "  OK: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "  WARNING: Rust not found. Install from https://rustup.rs" -ForegroundColor Yellow
    Write-Host "  Rust is needed to build the native core (my_framework)" -ForegroundColor Yellow
}

# 5. Build Rust core
Write-Host "[5/6] Building Rust core (PyO3)..." -ForegroundColor Yellow
if (Get-Command rustc -ErrorAction SilentlyContinue) {
    $env:PYO3_USE_ABI3_FORWARD_COMPATIBILITY = '1'
    & .\.venv\Scripts\python.exe -m maturin develop
    Write-Host "  Rust core built and installed" -ForegroundColor Green
} else {
    Write-Host "  SKIPPED: Rust not installed (Python-only mode)" -ForegroundColor Yellow
}

# 6. Check Elixir (optional)
Write-Host "[6/6] Checking Elixir (optional)..." -ForegroundColor Yellow
try {
    $elixirVersion = elixir --version 2>&1
    Write-Host "  OK: Elixir found" -ForegroundColor Green
} catch {
    Write-Host "  OPTIONAL: Elixir not found. Install from https://elixir-lang.org" -ForegroundColor Yellow
    Write-Host "  Elixir is needed for real-time WebSocket clustering" -ForegroundColor Yellow
}

# Done
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Installation Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Quick Start:" -ForegroundColor White
Write-Host "  .\.venv\Scripts\Activate.ps1" -ForegroundColor White
Write-Host "  python -c \"import pytrex; print('PyTreX ready!')\"" -ForegroundColor White
Write-Host ""
Write-Host "Run tests:" -ForegroundColor White
Write-Host "  .\.venv\Scripts\python.exe -m pytest tests\test_features.py -v" -ForegroundColor White
Write-Host ""
Write-Host "Create a new project:" -ForegroundColor White
Write-Host "  pytrex init MySmartApp" -ForegroundColor White
Write-Host ""
