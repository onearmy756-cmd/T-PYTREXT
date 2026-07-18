# Migrating from Electron to PyTreX Framework

## Overview

PyTreX is a full-stack desktop framework that replaces Electron with a Rust + Tauri v2 core, offering:
- **10x smaller** bundle size (no Chromium bundled)
- **Native performance** via Rust + PyO3
- **Built-in encrypted database** (SQLx + AES-256)
- **Blockchain ledger** for audit trails
- **AI integration** (PyTorch)
- **Elixir concurrency** (BEAM VM)

## Migration Steps

### 1. Project Structure

| Electron | PyTreX |
|----------|--------|
| `main.js` (Node.js) | `main.py` (Python) |
| `renderer/` (HTML/JS) | `frontend/` (HTML/JS) |
| `preload.js` | Handled by Rust core |
| `package.json` | `pyproject.toml` |
| `electron-builder` | `pytrex build` |

### 2. IPC Replacement

**Electron:**
```js
// main.js
ipcMain.handle('get-data', (event, ...args) => { ... });

// renderer
const { ipcRenderer } = require('electron');
const result = await ipcRenderer.invoke('get-data', data);
```

**PyTreX:**
```python
# main.py
from pytrex import PyTreXApp, event

class MyApp(PyTreXApp):
    @event("get_data")
    def get_data(self, data):
        return json.dumps({"status": "success", "data": ...})

// renderer (frontend/index.html)
const result = await window.__TAURI__.invoke('tauri_to_python', {
    event_name: 'get_data', data: JSON.stringify(payload)
});
```

### 3. Database

**Electron (SQLite):**
```js
const Database = require('better-sqlite3');
const db = new Database('app.db');
db.prepare('CREATE TABLE ...').run();
```

**PyTreX (Encrypted SQLx):**
```python
import my_framework
my_framework.kuandaa_database_salama("app.db", "your_aes_key")
result = my_framework.fanya_muamala_salama("acc123", "deposit", 500.0)
```

### 4. Auto-Updater

**Electron:**
```js
const { autoUpdater } = require('electron-updater');
autoUpdater.checkForUpdates();
```

**PyTreX:**
```python
# Built into Tauri v2 — configure in tauri.conf.json:
# "plugins": { "updater": { "active": true, "endpoints": ["https://..."] } }
```

### 5. Quick Start

```bash
pytrex init my_app
cd my_app
pytrex dev
```
