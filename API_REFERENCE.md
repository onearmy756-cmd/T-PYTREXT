# 📖 T-PYTREXT API Reference

> Complete API documentation for 220+ classes and 39 Rust functions.

---

## Core Classes

### `PyTreXApp(name)`
The main application class. All PyTreXT apps extend this.

```python
from pytrex import PyTreXApp, event

class MyApp(PyTreXApp):
    def __init__(self):
        super().__init__(name="My App")

app = MyApp()
app.run()  # Starts Tauri desktop + Elixir engine
```

**Attributes:**
- `app.name` — Application name
- `app.blockchain` — `BlockchainBridge` instance
- `app.encryption` — `EncryptionManager` instance
- `app.bus` — `EventBus` instance
- `app.network` — Network client
- `app.rag` — `RAGEngine` instance
- `app.orm` — `ORMEngine` instance
- `app.auth` — `AuthManager` instance
- `app.state` — `StateMachine` instance
- `app.cache` — `CacheManager` instance
- `app.api` — `APIServer` instance
- `app.scheduler` — `CronScheduler` instance
- `app.audit` — `AuditTrail` instance

**Methods:**
- `app.run()` — Start the full application
- `app.open_window(label, title, url)` — Open additional Tauri window

---

### `@event(name)`
Decorator to register event handlers.

```python
@event("my_event")
def handler(data):
    return f"Processed: {data}"

# Call: execute_python_event("my_event", "test")
```

---

## Blockchain

### `BlockchainBridge()`
SHA-256 blockchain engine.

```python
bc = BlockchainBridge()

# Create blocks
result = bc.add_block("transaction_data")
# → {"status": "ok", "block": {"index": 1, "hash": "abc...", ...}}

# Verify chain
result = bc.verify_chain()
# → {"status": "ok", "valid": True, "blocks": 1}

# Get status
status = bc.get_status()
```

---

## Encryption

### `EncryptionManager(password)`
AES-256 encryption + SHA-256 hashing.

```python
enc = EncryptionManager(default_password="my-key")

# Encrypt/Decrypt
ciphertext = enc.encrypt("sensitive data")
plaintext = enc.decrypt(ciphertext)

# Hash
hash_val = enc.hash("data")  # SHA-256

# Generate secret
secret = enc.generate_secret(32)  # 32-char random string
```

---

## AI Agents

### `HermesAgent(name)`
Function-calling AI agent.

```python
agent = HermesAgent(name="MyAgent")

# Register custom functions
agent.register_function(
    "get_weather", my_weather_func,
    "Get current weather",
    {"city": {"type": "string"}}
)

# Chat with agent
result = agent.chat("What's the weather in Dar es Salaam?")
# → {"reply": "...", "function_calls": [...], "iterations": 1}

# Built-in functions
print(agent.list_functions())
# → blockchain_status, search_web, get_time, encrypt_text, decrypt_text

# Integrate with search
agent.integrate_search()
agent.integrate_mcp(mcp_client)
```

### `LangChainAgent(model_name, temperature)`
LangChain-based AI agent with chains, tools, and memory.

```python
agent = LangChainAgent(model_name="gpt-4")

# Add tools
agent.add_tool("search", my_search_func, "Search the web")

# Run chains
result = agent.run("What is PyTreXT?", chain_type="conversation")
result = agent.run("Calculate 2+2", chain_type="react")

# RAG queries
result = agent.rag_query("query", documents=["doc1", "doc2"], top_k=3)

# Memory management
agent.add_message("user", "Hello")
agent.clear_memory()
```

---

## Search

### `SearchEngine()`
Multi-engine web search (SearXNG + DuckDuckGo).

```python
search = SearchEngine(default_engine="duckduckgo")

# Search
results = search.search("query", engine="all", max_results=10)
# → List[SearchResult]

# DuckDuckGo
results = search.duckduckgo_search("query", max_results=5)
answer = search.duckduckgo_instant_answer("What is Python?")
news = search.duckduckgo_news("Tanzania", max_results=5)

# SearXNG
results = search.searxng_search("query", instance="https://searx.be")

# Summary for AI
summary = search.web_search_summary("query")
```

---

## Human-in-the-Loop

### `HumanInTheLoop(timeout)`
Approval workflows for AI actions.

```python
hitl = HumanInTheLoop(default_timeout=300)

# Request approval
action_id = hitl.request_approval(
    "transfer_funds",
    {"amount": 10000, "to": "ACC123"},
    timeout=600
)

# Approve / Reject
hitl.approve(action_id)
hitl.reject(action_id, "Amount too high")

# Query
pending = hitl.get_pending()
action = hitl.get_action(action_id)
history = hitl.get_history()

# Hooks
hitl.on_approve("transfer_funds", my_callback)
```

---

## MCP Protocol

### `MCPClient(server_url)`
Model Context Protocol client.

```python
client = MCPClient(server_url="http://localhost:8000/mcp")

# Connect
client.connect()

# List tools
tools = client.list_tools()

# Invoke tool
result = client.invoke_tool("blockchain_create_block", {"data": "test"})

# Resources & Prompts
resources = client.list_resources()
prompts = client.list_prompts()
content = client.read_resource("pytrex://blockchain/chain")

# Server info
info = client.server_info()
```

---

## Testing

### `TestRunner(app)`
Automated test framework.

```python
from pytrex import TestRunner, test_app, quick_test

# Full test
runner = TestRunner(app=my_app)
suite = runner.run_all()
# → TestSuite(passed=35, failed=0, total=38)

# Quick test
results = quick_test()

# Test specific module
runner.run_all(modules=["Blockchain", "Encryption"])

# Export
runner.export_json("results.json")
```

---

## Project Management

### `ProjectManager()`
Discover and manage PyTreXT projects.

```python
pm = ProjectManager()

# Scan for projects
projects = pm.scan(max_depth=3)

# List all
pm.list_projects()
pm.projects_table()

# Get project
proj = pm.get_project("SmartBank")

# Run
pm.run("SmartBank")
pm.run("SmartBank", dev_mode=False)

# Test
pm.test("SmartBank")
pm.test()  # All projects

# Build
pm.build("SmartBank", target="docker")

# Watch mode
pm.watch("SmartBank")

# Dashboard
pm.serve_dashboard(port=8080)

# Export
pm.export_json("projects.json")
```

---

## Production Deployment

### `ProductionBuilder(project_path, config)`
Build and deploy for production.

```python
from pytrex import ProductionBuilder, ProductionConfig, deploy, quick_build

# One-command deploy
deploy(".", target="docker")

# Custom
config = ProductionConfig(app_name="myapp", port=8080)
builder = ProductionBuilder(project_path=".", config=config)

# Build
builder.build_docker()
builder.build_standalone()

# Deploy
builder.deploy_cloud("aws")
builder.deploy_cloud("gcp")

# Configs
builder.generate_nginx_config("myapp.com")
builder.generate_systemd_service()
builder.generate_kubernetes_manifests()
builder.generate_github_actions()
builder.generate_env_files()
builder.generate_security_hardening()

# All-in-one
builder.deploy_all(target="docker")
```

---

## Rust Core Functions (via PyO3)

When `my_framework` is built (`maturin develop`):

| Function | Description |
|----------|-------------|
| `my_framework.fanya_app()` | Launch Tauri v2 desktop window |
| `my_framework.fanya_block_ya_blockchain(data)` | Create SHA-256 block |
| `my_framework.hakiki_blockchain(chain)` | Verify chain integrity |
| `my_framework.kuandaa_database_salama(path, key)` | Init encrypted SQLite |
| `my_framework.fanya_muamala_salama(acc, type, amt)` | ACID transaction |
| `my_framework.aes_encrypt(data, pwd)` | AES-256 encrypt |
| `my_framework.aes_decrypt(data, pwd)` | AES-256 decrypt |
| `my_framework.hash_data(data, algo)` | SHA-256/512 hash |
| `my_framework.anzisha_axum_server(port)` | Start Axum HTTP server |
| `my_framework.candle_pakia_model(path)` | Load Candle ML model |
| `my_framework.candle_embed(text)` | Generate embeddings |
| `my_framework.burn_fundisha(data, epochs)` | Train Burn model |
| `my_framework.mcp_client_anzisha(url)` | Connect MCP client |
| ...and 26 more | |

---

## Event System

```python
from pytrex.core import REGISTERED_EVENTS, execute_python_event

# Register
@event("my_event")
def handler(data):
    return json.dumps({"result": data})

# Execute
result = execute_python_event("my_event", "payload")

# Async
from pytrex.core import execute_python_event_async
future = execute_python_event_async("my_event", "payload")

# EventBus (pub/sub)
app.bus.on("user_login", callback)
app.bus.emit("user_login", {"user": "Juma"})
```

---

## Additional Services (Partial List)

| Category | Classes |
|----------|---------|
| **Auth** | AuthManager, OAuth2Integration, SSOManager, LDAPManager |
| **Database** | ORMEngine, ORMQuery, DatabaseMigrations |
| **AI/ML** | NeuralNetwork, RAGEngine, LLMIntegration, VectorDatabase, EmbeddingEngine |
| **Media** | ImageProcessor, QRCodeManager, BarcodeScanner, OCREngine |
| **Network** | WebSocketServer, APIServer, GraphQLServer, WebRTCVideoCall |
| **Enterprise** | AuditTrail, MultiTenantManager, WorkflowEngine, PermissionsEngine |
| **DevOps** | CICDPipeline, DependencyChecker, PerformanceMonitor, BackupManager |
| **Industry** | HealthcareHL7, FinancePortfolio, InventorySCM, HRPayroll, CRMPipeline |

---

> **Full list: 220+ classes available via `dir(pytrex)`**
