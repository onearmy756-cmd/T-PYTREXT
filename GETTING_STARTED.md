# 🚀 Getting Started with T-PYTREXT

> Build your first app in under 5 minutes.

---

## Prerequisites

| Tool | Version | How to Install |
|------|---------|---------------|
| Python | ≥ 3.10 | [python.org](https://python.org) |
| Git | Any | [git-scm.com](https://git-scm.com) |
| Rust (optional) | ≥ 1.75 | [rustup.rs](https://rustup.rs) — needed for native speed |
| Elixir (optional) | ≥ 1.16 | [elixir-lang.org](https://elixir-lang.org) — for real-time clustering |

---

## Step 1: Clone & Install

```bash
git clone https://github.com/onearmy756-cmd/T-PYTREXT.git
cd T-PYTREXT
pip install -r requirements.txt
```

---

## Step 2: Verify Installation

```bash
python -c "from pytrex import PyTreXApp; print('✅ PyTreXT is ready!')"
```

**Expected output:** `✅ PyTreXT is ready!`

---

## Step 3: Create Your First App

```bash
pytrex init MyFirstApp
cd MyFirstApp
```

This creates:
- `main.py` — Your app code
- `frontend/index.html` — Desktop UI
- `tauri.conf.json` — Tauri configuration
- `requirements.txt` — Dependencies

---

## Step 4: Write Your First Event

Open `main.py` and add your first event:

```python
from pytrex import PyTreXApp, event
import json

class MyApp(PyTreXApp):
    def __init__(self):
        super().__init__(name="My First PyTreXT App")

    @event("salamu")
    def greet(self, data):
        """Respond to greeting"""
        return json.dumps({
            "message": f"Hujambo! Nilipokea: {data}",
            "status": "success"
        })

if __name__ == "__main__":
    app = MyApp()
    app.run()
```

---

## Step 5: Run Your App

```bash
pytrex dev
```

Your app is now running! The dev server watches for file changes and auto-reloads.

---

## Step 6: Test Your App

```bash
pytrex test
```

---

## Next Steps

- 🏦 **Run the demos**: `python demos/smart_bank/main.py`
- 📚 **Read API Reference**: See [API_REFERENCE.md](API_REFERENCE.md)
- 🏗️ **Read Architecture**: See [ARCHITECTURE.md](ARCHITECTURE.md)
- 🚀 **Deploy to production**: `pytrex deploy -t docker`

---

## Common Patterns

### Adding Blockchain

```python
from pytrex import BlockchainBridge

bc = BlockchainBridge()
bc.add_block("transaction_1")
bc.add_block("transaction_2")
print(bc.verify_chain())
```

### Adding AI Agent

```python
from pytrex import HermesAgent

agent = HermesAgent()
result = agent.chat("What is PyTreXT?")
print(result["reply"])
```

### Adding Encryption

```python
from pytrex import EncryptionManager

enc = EncryptionManager(password="my-secret")
encrypted = enc.encrypt("Sensitive Data")
decrypted = enc.decrypt(encrypted)
```

### Adding Search

```python
from pytrex import WebSearchEngine

search = WebSearchEngine()
results = search.search("Tanzania technology")
```

---

## Need Help?

- 📖 [API Reference](API_REFERENCE.md)
- 🏗️ [Architecture Guide](ARCHITECTURE.md)
- 🐛 [GitHub Issues](https://github.com/onearmy756-cmd/T-PYTREXT/issues)
- 💬 Run `pytrex dashboard` to see all your projects
