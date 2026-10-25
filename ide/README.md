# MBILINYI IDE

**MBILINYI IDE** ni mfumo halisi wa IDE ulioingizwa ndani ya **T-PYTREXT**:
OpenHands Agent Canvas ni **interface**, na kila zana ya T-PYTREXT + zana za IDE
zinafanya kazi pamoja kuongeza **kasi, nguvu na uwezo**.

```
        ┌──────────────────────────────────────────────────────────────┐
        │  OpenHands Agent Canvas  (UI — React, hudumiwa kwenye /ide)   │
        └───────────────────────────────┬──────────────────────────────┘
                                        │  protokoli ya OpenHands agent-server
                                        │  /server_info  /api/conversations
                                        │  /sockets/events/{id}  /api/settings
        ┌───────────────────────────────▼──────────────────────────────┐
        │  AgentServer  (pytrex/agent_server.py)                        │
        │  ├─ ConversationStore   — mazungumzo + event log              │
        │  ├─ AgentRuntime        — kitanzi cha ReAct (LLM + tools)     │
        │  ├─ IDEEngine           — canvas/desktop/backend probes       │
        │  └─ ToolBridge          — zana HALISI za T-PYTREXT            │
        │       TerminalEmulator · CodeEditor · Linter · CodeFormatter  │
        │       TypeChecker · TestFramework · SecurityScanner · REPL    │
        │       HotReloader · DebugProfiler · RAGEngine · CodeGenerator │
        │       CICDPipeline · VersionControl · Notebook · AIAgent      │
        │       LLMIntegration · ProcessManager · HealthChecker · …     │
        └──────────────────────────────────────────────────────────────┘
```

## Kile kilichomo

| Sehemu | Njia | Ni nini |
|---|---|---|
| Canvas UI | `ide/openhands/` | OpenHands Agent Canvas (React/TS, MIT) — interface |
| Desktop shell | `ide/desktop/` | Tauri v2 shell (dirisha la desktop) |
| Extension app | `ide/extension-app/` | Canvas Extension App (Monaco) |
| Backend (Rust) | `ide/backend/` | Agent ya Rust (crates za `fundi-*`) — API za IDE |
| Skills | `ide/skills/` | Skills 139 za canvas |

## Python modules (T-PYTREXT architecture)

| Module | Inatoa |
|---|---|
| `pytrex.ide` | `IDEEngine`, `IDEToolRegistry` (zana 17), `MBILINYIIDE` |
| `pytrex.ide_server` | `IDEServer`, `ToolBridge` (zana zote za T-PYTREXT) |
| `pytrex.agent_server` | `AgentServer` (protokoli ya OpenHands agent-server), `AgentRuntime` (ReAct) |

## Tumia

```bash
# Hali + plan
python -m pytrex.cli ide

# Zana zote
python -m pytrex.cli ide --tools

# Endesha mfumo HALISI (OpenHands interface + zana zote)
python -m pytrex.cli ide --serve -p 8080

# Kisha fungua: http://localhost:8080/ide
```

Bila LLM key, agent **hurejesha kosa la wazi** ("LLM not configured") — hakuna
kujifanya kufanya kazi. Weka key kupitia `PATCH /api/settings`
(`agent_settings_diff.llm_api_key`) au `OPENAI_API_KEY`.

## Protokoli (agent-server)

```
GET  /server_info                     → toleo + capabilities
GET  /alive /health /ready            → probes
GET  /api/settings                    → settings (+ llm_api_key_is_set)
PATCH /api/settings                   → weka llm_api_key / llm_model
GET  /api/conversations               → orodha
POST /api/conversations               → unda mazungumzo
GET  /api/conversations/{id}          → info
GET  /api/conversations/{id}/events   → event log
POST /api/conversations/{id}/events   → tuma ujumbe + endesha agent
WS   /sockets/events/{id}             → mtiririko wa events (live)
```

## Tests

```bash
python -m pytest tests/test_ide.py -q      # 34 tests
```
