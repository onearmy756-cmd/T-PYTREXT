"""pytrex.ide_server — MBILINYI IDE server halisi (OpenHands interface).

Hii ni server HALISI (si demo): inahudumia **OpenHands Agent Canvas** kama
interface, na kila zana ya T-PYTREXT + zana za IDE zinafanya kazi pamoja
kuongeza kasi, nguvu na uwezo.

Architecture (ile ile ya T-PYTREXT — Python, `@event`, `PyTreXApp`):

    ┌──────────────────────────────────────────────────────────┐
    │  OpenHands Agent Canvas (UI) — hudumiwa kwenye `/ide`     │
    └───────────────────────┬──────────────────────────────────┘
                            │  /api/*  (JSON)
    ┌───────────────────────▼──────────────────────────────────┐
    │  IDEServer (Python, ThreadingHTTPServer)                 │
    │  ├─ IDEEngine        — canvas/desktop/backend probes      │
    │  ├─ IDEToolRegistry  — 17 zana za IDE (LSP/DAP/build…)    │
    │  └─ ToolBridge       — zana HALISI za T-PYTREXT           │
    │       TerminalEmulator, CodeEditor, Linter, CodeFormatter,│
    │       TypeChecker, TestFramework, SecurityScanner, REPL,  │
    │       HotReloader, DebugProfiler, MemoryAnalyzer,         │
    │       FileWatcher, CodeGenerator, RAGEngine, SearchEngine,│
    │       CICDPipeline, VersionControl, Notebook, AIAgent,    │
    │       LLMIntegration, ProcessManager, HealthChecker…      │
    └──────────────────────────────────────────────────────────┘

Kila endpoint huitisha zana HALISI (si stub). Kama zana haipatikani, hurejesha
`ok: false` wazi — hakuna kubuni.
"""

from __future__ import annotations

import json
import os
import select
import shutil
import socket
import subprocess
import tempfile
import threading
import time
import traceback
import urllib.parse
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable, Dict, List, Optional, Tuple

from .core import (
    TerminalEmulator, CodeEditor, Linter, CodeFormatter, TypeChecker,
    TestFramework, SecurityScanner, REPL, HotReloader, DebugProfiler,
    MemoryAnalyzer, FileWatcher, CodeGenerator, RAGEngine,
    CICDPipeline, VersionControl, Notebook, AIAgent, LLMIntegration,
    ProcessManager, HealthChecker, AuditTrail, PerformanceMonitor,
    StateMachine, EventBus, FeatureFlags, ConfigManager,
)
from .search_engine import SearchEngine as WebSearchEngine
from .ide import IDEEngine, IDEToolRegistry, ide_home
from .editor import EditorServer, DEFAULT_EDITOR_PORT

IDE_SERVER_VERSION = "1.0.0"


class _StopProxy(Exception):
    """Ishara ya kumaliza proxy loop (socket moja imefungwa)."""


# ═══════════════════════════════════════════════════════════
# ToolBridge — zana HALISI za T-PYTREXT zilizounganishwa
# ═══════════════════════════════════════════════════════════

class ToolBridge:
    """Inaunganisha zana zote za T-PYTREXT zinazohusiana na IDE.

    Kila zana ni instance HALISI; `run(name, **kwargs)` huirudisha kwa JSON.
    """

    def __init__(self, workspace: str = ".") -> None:
        self.workspace = os.path.abspath(workspace)
        self.terminal = TerminalEmulator(cwd=self.workspace)
        self.editor = CodeEditor()
        self.linter = Linter()
        self.formatter = CodeFormatter()
        self.type_checker = TypeChecker()
        self.tests = TestFramework()
        self.security = SecurityScanner()
        self.repl = REPL()
        self.hot_reload = HotReloader()
        self.profiler = DebugProfiler()
        self.memory = MemoryAnalyzer()
        self.watcher = FileWatcher()
        self.codegen = CodeGenerator()
        self.rag = RAGEngine()
        self.search = WebSearchEngine()
        self.cicd = CICDPipeline()
        self.vcs = VersionControl()
        self.notebook = Notebook()
        self.agent = AIAgent(name="MBILINYI-IDE-Agent")
        self.llm = LLMIntegration()
        self.processes = ProcessManager()
        self.health = HealthChecker()
        self.audit = AuditTrail()
        self.perf = PerformanceMonitor()
        self.state = StateMachine()
        self.bus = EventBus()
        self.flags = FeatureFlags()
        self.config = ConfigManager()
        self.codegen.set_llm(self.llm)
        self.rag.set_llm(self.llm)
        self._register_agent_tools()
        self._register_health_checks()

    # ---------- internal ----------

    def _register_agent_tools(self) -> None:
        """Zana za IDE zinazopatikana kwa AIAgent (kasi + uwezo)."""
        self.agent.register_tool("read_file", self._tool_read_file, "Read a file from workspace")
        self.agent.register_tool("write_file", self._tool_write_file, "Write a file in workspace")
        self.agent.register_tool("list_files", self._tool_list_files, "List workspace files")
        self.agent.register_tool("run_shell", self._tool_run_shell, "Run a shell command")
        self.agent.register_tool("lint", lambda code: self.linter.lint(code), "Lint code")
        self.agent.register_tool("format", lambda code: self.formatter.format(code), "Format code")
        self.agent.register_tool("search", lambda q: self.search.web_search_summary(q), "Search the web")

    def _register_health_checks(self) -> None:
        self.health.register_check("workspace", lambda: os.path.isdir(self.workspace))
        self.health.register_check("terminal", lambda: self.terminal is not None)

    def _tool_read_file(self, path: str) -> dict:
        return self.editor.open(os.path.join(self.workspace, path))

    def _tool_write_file(self, path: str, content: str = "") -> dict:
        ok = self.editor.save(os.path.join(self.workspace, path), content)
        return {"status": "ok" if ok else "error", "path": path}

    def _tool_list_files(self, path: str = ".") -> dict:
        base = os.path.join(self.workspace, path)
        if not os.path.isdir(base):
            return {"status": "error", "message": "not a directory"}
        return {"status": "ok", "files": sorted(os.listdir(base))}

    def _tool_run_shell(self, command: str) -> dict:
        return self.terminal.run(command)

    # ---------- dispatch ----------

    def catalog(self) -> dict:
        return {
            "pytrex_tools": [
                "terminal", "editor", "linter", "formatter", "type_checker",
                "tests", "security", "repl", "hot_reload", "profiler",
                "memory", "watcher", "codegen", "rag", "search", "cicd",
                "vcs", "notebook", "agent", "llm", "processes", "health",
                "audit", "perf", "state", "bus", "flags", "config",
            ],
            "ide_tools": IDEToolRegistry().catalog(),
        }

    def run(self, tool: str, **kwargs) -> dict:
        """Tekeleza zana kwa jina. Hurejesha dict (kamwe si stub)."""
        try:
            if tool == "terminal":
                return self.terminal.run(kwargs.get("command", ""),
                                         timeout=int(kwargs.get("timeout", 30)))
            if tool == "editor":
                action = kwargs.get("action", "open")
                if action == "open":
                    return self.editor.open(kwargs.get("path", ""),
                                            kwargs.get("language", ""))
                if action == "save":
                    ok = self.editor.save(kwargs.get("path", ""), kwargs.get("content", ""))
                    return {"status": "ok" if ok else "error"}
                if action == "close":
                    return {"status": "ok" if self.editor.close(kwargs.get("path", "")) else "error"}
                return {"status": "error", "message": f"unknown editor action: {action}"}
            if tool == "linter":
                return self.linter.lint(kwargs.get("code", ""))
            if tool == "formatter":
                return self.formatter.format(kwargs.get("code", ""),
                                             kwargs.get("language", "python"))
            if tool == "type_checker":
                return self.type_checker.check_type(kwargs.get("value"),
                                                    kwargs.get("expected_type", "any"))
            if tool == "security":
                return self.security.scan(kwargs.get("input_text", ""),
                                          kwargs.get("context", "general"))
            if tool == "repl":
                return self.repl.eval(kwargs.get("code", ""))
            if tool == "hot_reload":
                if kwargs.get("action") == "check":
                    return {"status": "ok", "changed": self.hot_reload.check()}
                return {"status": "ok", "watching": self.hot_reload.watched_files}
            if tool == "profiler":
                return {"status": "ok", "profiles": self.profiler.profiles}
            if tool == "memory":
                return {"status": "ok", "memory": self.memory.analyze()} \
                    if hasattr(self.memory, "analyze") else {"status": "ok", "memory": []}
            if tool == "watcher":
                if kwargs.get("action") == "watch":
                    ok = self.watcher.watch(kwargs.get("path", ""), lambda *a: None)
                    return {"status": "ok" if ok else "error"}
                return {"status": "ok", "watched": self.watcher.watched_paths}
            if tool == "codegen":
                return self.codegen.generate(kwargs.get("description", ""),
                                             kwargs.get("language", "python"))
            if tool == "rag":
                if kwargs.get("action") == "add":
                    return self.rag.add_document(kwargs.get("doc_id", ""), kwargs.get("text", ""))
                return self.rag.query(kwargs.get("question", ""), int(kwargs.get("top_k", 5)))
            if tool == "search":
                return self.search.web_search_summary(kwargs.get("query", ""))
            if tool == "cicd":
                if kwargs.get("action") == "create":
                    ok = self.cicd.create(kwargs.get("name", "pipeline"), kwargs.get("stages", []))
                    return {"status": "ok" if ok else "error"}
                if kwargs.get("action") == "run":
                    return self.cicd.run(kwargs.get("name", ""), kwargs.get("environment", "dev"))
                return {"status": "ok", "pipelines": self.cicd.pipelines}
            if tool == "vcs":
                if kwargs.get("action") == "commit":
                    return self.vcs.commit(kwargs.get("resource_id", ""), kwargs.get("data"),
                                           kwargs.get("message", ""))
                if kwargs.get("action") == "history":
                    return {"status": "ok", "history": self.vcs.history(kwargs.get("resource_id", ""))}
                return {"status": "ok", "resources": self.vcs.resource_count()}
            if tool == "notebook":
                if kwargs.get("action") == "add":
                    idx = self.notebook.add_cell(kwargs.get("code", ""))
                    return {"status": "ok", "index": idx}
                if kwargs.get("action") == "run":
                    return self.notebook.run_cell(int(kwargs.get("index", 0)))
                if kwargs.get("action") == "run_all":
                    return {"status": "ok", "results": self.notebook.run_all()}
                return {"status": "ok", "cells": self.notebook.cell_count()}
            if tool == "agent":
                return self.agent.run(kwargs.get("goal", ""))
            if tool == "llm":
                if kwargs.get("action") == "configure":
                    return {"status": "ok", "configured": self.llm.configure(
                        kwargs.get("api_key", ""), kwargs.get("model", ""),
                        kwargs.get("base_url", ""))}
                return self.llm.chat(kwargs.get("message", ""))
            if tool == "processes":
                if kwargs.get("action") == "start":
                    ok = self.processes.start(kwargs.get("name", ""), kwargs.get("command", ""),
                                              kwargs.get("cwd", self.workspace))
                    return {"status": "ok" if ok else "error"}
                if kwargs.get("action") == "stop":
                    return {"status": "ok" if self.processes.stop(kwargs.get("name", "")) else "error"}
                return {"status": "ok", "processes": self.processes.list()}
            if tool == "health":
                return self.health.run()
            if tool == "audit":
                return {"status": "ok", "count": self.audit.count()}
            if tool == "flags":
                return {"status": "ok", "flags": self.flags.flags()}
            return {"status": "error", "ok": False, "message": f"zana '{tool}' haitambuliki"}
        except Exception as e:  # noqa: BLE001 — zana yoyote isivunje server
            return {"status": "error", "ok": False, "message": str(e),
                    "traceback": traceback.format_exc()[-800:]}


# ═══════════════════════════════════════════════════════════
# IDEServer — HTTP server halisi
# ═══════════════════════════════════════════════════════════

class IDEServer:
    """Server halisi ya MBILINYI IDE.

    Inahudumia:
      * `/ide` (+ assets)   — OpenHands Agent Canvas (UI)
      * `/api/ide/*`        — zana za IDE + zana za T-PYTREXT
      * `/api/status`       — hali ya kila kitu (probe halisi)
    """

    def __init__(
        self,
        host: str = "0.0.0.0",
        port: int = 8080,
        workspace: str = ".",
        home: Optional[str] = None,
    ) -> None:
        self.host = host
        self.port = port
        self.workspace = os.path.abspath(workspace)
        self.engine = IDEEngine(home=home)
        self.bridge = ToolBridge(self.workspace)
        self._httpd: Optional[ThreadingHTTPServer] = None
        self._thread: Optional[threading.Thread] = None
        self.started_at = time.time()
        self.requests = 0
        # Token ya kikao inayoingizwa kwenye canvas ili iunganishe backend hii
        # moja kwa moja. Server hii haitekelezi auth ya lazima (local), kwa
        # hivyo token ni kwa ajili ya kuunganisha UI pekee.
        self.session_api_key = os.environ.get("MTS_IDE_SESSION_KEY") or uuid.uuid4().hex
        # Editor halisi (OpenVSCode Server) nyuma ya `/ide/vscode`.
        self.editor = EditorServer(
            workspace=self.workspace,
            port=int(os.environ.get("MTS_EDITOR_PORT", DEFAULT_EDITOR_PORT)),
            token=self.session_api_key,
        )
        self.editor_base_path = self.editor.base_path
        self.editor_upstream = ("127.0.0.1", self.editor.port)
        self._compress_cache: Dict[Tuple[str, int], bytes] = {}
        self._compress_cache_bytes = 0
        self._cache_lock = threading.Lock()

    # ---------- routing ----------

    def _route(self, method: str, path: str, query: dict, body: str) -> dict:
        self.requests += 1

        # ---- status ----
        if path == "/api/status" or path == "/health":
            return {"ok": True, "data": {
                "version": IDE_SERVER_VERSION,
                "uptime": round(time.time() - self.started_at, 1),
                "requests": self.requests,
                "workspace": self.workspace,
                "ide": self.engine.status(),
            }}

        # ---- canvas (UI) ----
        if path in ("/ide", "/ide/", "/ide.html"):
            return self._serve_canvas_index()
        if path.startswith("/ide/"):
            return self._serve_canvas_asset(path[len("/ide/"):])

        # ---- IDE tools ----
        if path == "/api/ide/tools":
            return {"ok": True, "data": self.engine.tools.catalog()}

        # ---- editor halisi (OpenVSCode Server) ----
        if path in ("/api/vscode/status", "/api/ide/editor/status"):
            return self.editor.status()
        if path == "/api/vscode/url":
            base_url = query.get("base_url") or f"http://{self.host}:{self.port}"
            return {"url": self.editor.url(base_url)}
        if path == "/api/ide/editor/start" and method == "POST":
            return self.editor.start()
        if path == "/api/ide/editor/stop" and method == "POST":
            return self.editor.stop()
        if path == "/api/ide/plan":
            return {"ok": True, "data": self.engine.plan()}

        if path == "/api/ide/bridge":
            return {"ok": True, "data": self.bridge.catalog()}

        # ---- real tools dispatch ----
        if path == "/api/ide/run" and method == "POST":
            payload = self._json(body)
            tool = payload.pop("tool", "")
            if not tool:
                return {"ok": False, "error": "tool required"}
            result = self.bridge.run(tool, **payload)
            self.bridge.audit.log("ide", f"tool:{tool}", path)
            return {"ok": result.get("status") != "error", "data": result}

        # ---- editor shortcuts (kasi) ----
        if path == "/api/ide/file" and method == "GET":
            p = os.path.join(self.workspace, query.get("path", ""))
            return {"ok": True, "data": self.bridge.editor.open(p)}
        if path == "/api/ide/tree":
            return {"ok": True, "data": self._tree(query.get("path", "."))}
        if path == "/api/ide/lint" and method == "POST":
            return {"ok": True, "data": self.bridge.linter.lint(self._json(body).get("code", ""))}
        if path == "/api/ide/format" and method == "POST":
            p = self._json(body)
            return {"ok": True, "data": self.bridge.formatter.format(
                p.get("code", ""), p.get("language", "python"))}
        if path == "/api/ide/terminal" and method == "POST":
            p = self._json(body)
            return {"ok": True, "data": self.bridge.terminal.run(
                p.get("command", ""), int(p.get("timeout", 30)))}
        if path == "/api/ide/search" and method == "POST":
            return {"ok": True, "data": self.bridge.search.web_search_summary(self._json(body).get("query", ""))}

        # ---- lifecycle ----
        if path == "/api/ide/build_canvas" and method == "POST":
            return {"ok": True, "data": self.engine.build_canvas()}

        return {"_status": 404, "_ctype": "application/json",
                "_raw": json.dumps({"ok": False, "error": f"route haipo: {method} {path}"})}

    # ---------- helpers ----------

    @staticmethod
    def _json(body: str) -> dict:
        try:
            data = json.loads(body or "{}")
            return data if isinstance(data, dict) else {}
        except json.JSONDecodeError:
            return {}

    def _tree(self, rel: str = ".") -> dict:
        base = os.path.normpath(os.path.join(self.workspace, rel))
        if not base.startswith(self.workspace):
            return {"error": "nje ya workspace"}
        if not os.path.isdir(base):
            return {"error": "si folda"}
        entries = []
        for name in sorted(os.listdir(base))[:500]:
            full = os.path.join(base, name)
            entries.append({
                "name": name,
                "type": "dir" if os.path.isdir(full) else "file",
                "size": os.path.getsize(full) if os.path.isfile(full) else 0,
            })
        return {"root": base, "entries": entries}

    def _canvas_build_dir(self) -> Optional[str]:
        if not self.engine.home:
            return None
        build = os.path.join(self.engine.home, "openhands", "build")
        return build if os.path.isfile(os.path.join(build, "index.html")) else None

    def _serve_canvas_index(self) -> dict:
        build = self._canvas_build_dir()
        if not build:
            html = ("<!doctype html><meta charset=utf-8><title>MBILINYI IDE</title>"
                    "<body style='font-family:system-ui;background:#0b0d10;color:#e6e6e6;padding:2rem'>"
                    "<h1>MBILINYI IDE</h1><p>OpenHands Agent Canvas haijajengwa bado.</p>"
                    "<p>Endesha: <code>POST /api/ide/build_canvas</code> au "
                    "<code>pytrex ide --start</code></p></body>")
            return {"_raw": html, "_status": 200, "_ctype": "text/html; charset=utf-8"}
        with open(os.path.join(build, "index.html"), "r", encoding="utf-8") as f:
            html = f.read()
        # Ingiza runtime config ili Canvas iunganishe backend hii MOYAMOJA
        # (bila key, Canvas hutuonyesha skrini ya kusanidi backend).
        injection = self._runtime_injection()
        if injection:
            html = html.replace("</head>", injection + "</head>", 1)
        return {"_raw": html, "_status": 200, "_ctype": "text/html; charset=utf-8"}

    def _runtime_injection(self) -> str:
        """Tengeneza <script> ya config (kama static-server.mjs ya canvas)."""
        key = self.session_api_key
        base_path = "/ide"
        parts = []
        if key:
            k = json.dumps(key)
            parts.append(f"window.__AGENT_CANVAS_SESSION_API_KEY__={k};")
            parts.append(
                "try{var _k='mbilinyi-agent-server-config',"
                f"_c=JSON.parse(localStorage.getItem(_k)||'{{}}');"
                f"if(_c.sessionApiKey!=={k}){{_c.sessionApiKey={k};"
                "localStorage.setItem(_k,JSON.stringify(_c));}}catch(e){}")
        parts.append(f"window.__AGENT_CANVAS_BASE_PATH__={json.dumps(base_path)};")
        parts.append(f"window.__AGENT_CANVAS_VSCODE_BASE_PATH__={json.dumps(base_path + '/vscode')};")
        parts.append("window.__AGENT_CANVAS_DO_NOT_TRACK__=true;")
        return "<script>(function(){" + "".join(parts) + "})();</script>"

    def _serve_canvas_asset(self, rel: str) -> dict:
        build = self._canvas_build_dir()
        if not build:
            return {"_status": 404, "_raw": "canvas not built", "_ctype": "text/plain"}
        if ".." in rel.split("/"):
            return {"_status": 400, "_raw": "bad path", "_ctype": "text/plain"}
        full = os.path.join(build, rel)
        if not os.path.isfile(full):
            # SPA fallback: routes za canvas (/ide/conversations, /ide/settings,
            # n.k.) hazina faili — zirudishe index.html ili router ichukue.
            if not os.path.splitext(rel)[1]:
                return self._serve_canvas_index()
            return {"_status": 404, "_raw": "not found", "_ctype": "text/plain"}
        ctype = _content_type(full)
        if ctype.startswith("text/") or ctype.endswith(("javascript", "json", "svg+xml")):
            with open(full, "r", encoding="utf-8", errors="replace") as f:
                return {"_raw": f.read(), "_status": 200, "_ctype": ctype}
        with open(full, "rb") as f:
            return {"_raw": f.read(), "_status": 200, "_ctype": ctype}

    # ---------- editor proxy ----------

    def _is_editor_path(self, path: str) -> bool:
        base = self.editor_base_path
        p = path.split("?", 1)[0]
        return p == base or p.startswith(base + "/")

    def _proxy_editor(self, handler) -> None:
        """Proxy halisi (HTTP + WebSocket) kwa OpenVSCode Server.

        Tunaunganisha socket mbili (kivinjari ↔ editor) na kupitisha bytes
        mbichi pande zote mbili. Hii inaunga mkono HTTP ya kawaida pamoja na
        `Upgrade: websocket` (ambayo editor inahitaji kwa terminal, LSP, n.k.).
        """
        import socket as _socket

        # Proxy inashikilia socket yenyewe — zuia keep-alive ya handler.
        handler.close_connection = True

        if not self.editor.running:
            body = b"editor not running"
            handler.send_response(503)
            handler.send_header("Content-Type", "text/plain")
            handler.send_header("Content-Length", str(len(body)))
            handler.end_headers()
            handler.wfile.write(body)
            return

        # Jenga ombi la awali ili kulihamishia kwa editor.
        method = handler.command
        req_path = handler.path
        # Ondoa `tkn` kabla ya kupeleka: editor huitumia kama token ya
        # kuunganisha na hulazimika kurudisha 302 inayoiondoa (kisha kivinjari
        # hukwama kwenye 403 iliyohifadhiwa). Tunaiweka kama cookie hapa chini,
        # ambayo ndiyo njia editor inayotumia kuthibitisha maombi yanayofuata.
        if "?" in req_path:
            _p, _, _q = req_path.partition("?")
            _kept_q = "&".join(kv for kv in _q.split("&")
                               if kv and not kv.startswith("tkn="))
            req_path = f"{_p}?{_kept_q}" if _kept_q else _p
        if os.environ.get("MTS_IDE_ACCESS_LOG"):
            up = handler.headers.get("Upgrade", "")
            print(f"[editor] {method} {req_path} upgrade={up}", flush=True)
        try:
            length = int(handler.headers.get("Content-Length", 0) or 0)
        except ValueError:
            length = 0
        body = handler.rfile.read(length) if length else b""

        try:
            conn = _socket.create_connection(self.editor_upstream, timeout=10.0)
        except OSError as e:  # noqa: BLE001
            payload = f"editor unreachable: {e}".encode()
            handler.send_response(502)
            handler.send_header("Content-Type", "text/plain")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)
            return

        # WebSocket/upgrade huhitaji pipe mbichi ya pande mbili. HTTP ya
        # kawaida hutumia http.client: pipe mbichi hurudisha
        # `Transfer-Encoding: chunked` + `Connection: keep-alive` (hop-by-hop)
        # kwa mteja wa HTTP/2, na LB hukata stream hiyo kwa JS/CSS za workbench
        # (ERR_HTTP2_PROTOCOL_ERROR).
        if not handler.headers.get("Upgrade"):
            self._relay_http(handler, method, req_path, body)
            return

        try:
            conn = _socket.create_connection(self.editor_upstream, timeout=10.0)
        except OSError as e:  # noqa: BLE001
            payload = f"editor unreachable: {e}".encode()
            handler.send_response(502)
            handler.send_header("Content-Type", "text/plain")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)
            return

        # Tuma ombi la mwanzo (headers zote). Tunahifadhi Host halisi — editor
        # hutumia Host kujenga `remoteAuthority` ambayo kivinjari hutumia
        # kuunganisha WebSocket. Kuibadilisha kwake ni kumwelekeza kivinjari
        # kwenye `127.0.0.1:8001` isiyofikika.
        lines = [f"{method} {req_path} HTTP/1.1"]
        if not any(k.lower() == "host" for k in handler.headers.keys()):
            lines.append(f"Host: 127.0.0.1:{self.editor.port}")
        # Badilisha `vscode-tkn` YOYOTE: kivinjari huweka cookie ya kikao
        # kilichopita, na editor husoma ya kwanza pekee — cookie mbili
        # hupelekea 403 "Forbidden" isiyoeleweka.
        cookie_hdr = handler.headers.get("Cookie", "")
        kept = [c.strip() for c in cookie_hdr.split(";")
                if c.strip() and not c.strip().startswith("vscode-tkn=")]
        if self.editor.token:
            kept.append(f"vscode-tkn={self.editor.token}")
        if kept:
            lines.append("Cookie: " + "; ".join(kept))
        for k, v in handler.headers.items():
            if k.lower() == "cookie":
                continue  # imeunganishwa juu
            lines.append(f"{k}: {v}")
        # USIINGIZE X-Forwarded-* mara ya pili: kama zipo tayari (kutoka LB),
        # kurudia hupelekea editor kuunganisha thamani kwa koma → remoteAuthority
        # inakuwa "host, host:443" → CSP mbovu na WebSocket URL isiyo sahihi.
        _have = {k.lower() for k in handler.headers.keys()}
        if "x-forwarded-host" not in _have:
            lines.append(f"X-Forwarded-Host: {handler.headers.get('Host', '')}")
        if "x-forwarded-proto" not in _have:
            lines.append("X-Forwarded-Proto: https")
        if os.environ.get("MTS_IDE_ACCESS_LOG"):
            print(f"[editor] fwd={lines[1:]}", flush=True)
        raw = ("\r\n".join(lines) + "\r\n\r\n").encode("latin-1") + body
        try:
            conn.sendall(raw)
        except OSError:
            conn.close()
            return

        client = handler.connection
        try:
            conn.setblocking(False)
            client.setblocking(False)
            first = True
            while True:
                r, _w, x = select.select([conn, client], [], [conn, client], 60.0)
                if x:
                    break
                if not r:
                    break
                for src in r:
                    dst = client if src is conn else conn
                    try:
                        data = src.recv(65536)
                    except (BlockingIOError, InterruptedError):
                        continue
                    except OSError:
                        data = b""
                    if not data:
                        raise _StopProxy()
                    if first and src is conn and os.environ.get("MTS_IDE_ACCESS_LOG"):
                        print(f"[editor] resp={data.splitlines()[0]!r}", flush=True)
                    first = False
                    dst.sendall(data)
        except (_StopProxy, OSError):
            pass
        finally:
            try:
                conn.close()
            except OSError:
                pass
            try:
                client.close()
            except OSError:
                pass

    def _relay_http(self, handler, method: str, req_path: str, body: bytes) -> None:
        """Hamisha ombi la HTTP la kawaida kwenda editor kwa http.client."""
        import http.client as _http

        hop = {"connection", "keep-alive", "proxy-connection", "transfer-encoding",
               "upgrade", "te", "trailer", "proxy-authenticate",
               "proxy-authorization"}
        fwd: Dict[str, str] = {}
        for k, v in handler.headers.items():
            lk = k.lower()
            if lk in hop or lk == "cookie":
                continue
            # Tunaomba identity kwa upstream na kufunga gzip sisi wenyewe
            # (upstream haifanyi compression; workbench.js ni 13 MB).
            if lk == "accept-encoding":
                continue
            fwd[k] = v
        fwd["Host"] = handler.headers.get("Host", f"127.0.0.1:{self.editor.port}")
        cookie_hdr = handler.headers.get("Cookie", "")
        kept = [c.strip() for c in cookie_hdr.split(";")
                if c.strip() and not c.strip().startswith("vscode-tkn=")]
        if self.editor.token:
            kept.append(f"vscode-tkn={self.editor.token}")
        if kept:
            fwd["Cookie"] = "; ".join(kept)
        present = {k.lower() for k in fwd}
        if "x-forwarded-host" not in present:
            fwd["X-Forwarded-Host"] = handler.headers.get("Host", "")
        if "x-forwarded-proto" not in present:
            fwd["X-Forwarded-Proto"] = "https"

        conn = _http.HTTPConnection(self.editor_upstream[0],
                                    self.editor_upstream[1], timeout=30)
        try:
            conn.request(method, req_path, body=body or None, headers=fwd)
            resp = conn.getresponse()
            payload = resp.read()
            headers = [(k, v) for k, v in resp.getheaders()
                       if k.lower() not in hop and k.lower() not in ("server", "date")]
            payload, headers = self._maybe_compress(
                handler, method, req_path, payload, headers
            )
            handler.send_response(resp.status)
            for k, v in headers:
                handler.send_header(k, v)
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            if method != "HEAD":
                handler.wfile.write(payload)
        except (OSError, _http.HTTPException) as e:  # noqa: BLE001
            payload = f"editor unreachable: {e}".encode()
            handler.send_response(502)
            handler.send_header("Content-Type", "text/plain")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)
        finally:
            try:
                conn.close()
            except OSError:
                pass

    # ---------- compression ----------

    _COMPRESSIBLE = (".js", ".css", ".json", ".html", ".svg", ".map", ".txt",
                     ".md", ".xml", ".ttf")
    _CACHE_MAX_BYTES = 64 * 1024 * 1024

    def _maybe_compress(self, handler, method: str, req_path: str,
                        payload: bytes,
                        headers: List[Tuple[str, str]]) -> Tuple[bytes, List[Tuple[str, str]]]:
        """Gzip payload kubwa inayoweza kubanwa (workbench.js ni 13 MB).

        Upstream haifanyi compression, kwa hivyo kivinjari hupakua bundle
        kamili. Tunabanwa hapa na kuhifadhi matokeo kwenye cache — gharama ya
        gzip hulipwa mara moja tu kwa kila toleo la asset.
        """
        if method == "HEAD" or len(payload) < 1024:
            return payload, headers
        if handler.headers.get("Accept-Encoding", "").lower().find("gzip") < 0:
            return payload, headers
        if any(k.lower() == "content-encoding" for k, _ in headers):
            return payload, headers
        ctype = next((v for k, v in headers if k.lower() == "content-type"), "")
        path_only = req_path.split("?", 1)[0].lower()
        if not (path_only.endswith(self._COMPRESSIBLE)
                or any(t in ctype for t in ("javascript", "text/", "json",
                                            "svg", "xml"))):
            return payload, headers

        import gzip as _gzip

        key = (path_only, len(payload))
        with self._cache_lock:
            hit = self._compress_cache.get(key)
        if hit is None:
            hit = _gzip.compress(payload, 6)
            with self._cache_lock:
                if self._compress_cache_bytes + len(hit) <= self._CACHE_MAX_BYTES:
                    self._compress_cache[key] = hit
                    self._compress_cache_bytes += len(hit)
        if len(hit) >= len(payload):
            return payload, headers

        out = [(k, v) for k, v in headers
               if k.lower() not in ("content-length", "content-encoding")]
        out.append(("Content-Encoding", "gzip"))
        out.append(("Vary", "Accept-Encoding"))
        return hit, out


    # ---------- server ----------

    def _make_handler(self):
        server = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):  # tuliza
                pass

            def _dispatch(self, method: str):
                parsed = urllib.parse.urlparse(self.path)
                query = dict(urllib.parse.parse_qsl(parsed.query))
                # Editor halisi: `/ide/vscode` (HTTP + WebSocket) inaenda kwa proxy.
                if server._is_editor_path(parsed.path):
                    server._proxy_editor(self)
                    return
                length = int(self.headers.get("Content-Length", 0) or 0)
                body = self.rfile.read(length).decode("utf-8") if length else ""
                if os.environ.get("MTS_IDE_ACCESS_LOG"):
                    print(f"[ide] {method} {parsed.path}", flush=True)
                try:
                    result = server._route(method, parsed.path, query, body)
                except Exception as e:  # noqa: BLE001
                    result = {"ok": False, "error": str(e),
                              "traceback": traceback.format_exc()[-800:]}
                self._respond(result)

            def _respond(self, result: dict):
                if "_raw" in result:
                    payload = result["_raw"]
                    if isinstance(payload, str):
                        payload = payload.encode("utf-8")
                    self.send_response(result.get("_status", 200))
                    self.send_header("Content-Type", result.get("_ctype", "text/plain"))
                    self.send_header("Content-Length", str(len(payload)))
                    self.end_headers()
                    self.wfile.write(payload)
                    return
                payload = json.dumps(result, default=str).encode("utf-8")
                status = result.pop("_status", None) or (200 if result.get("ok", True) else 400)
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            def do_GET(self):
                self._dispatch("GET")

            def do_POST(self):
                self._dispatch("POST")

            def do_PATCH(self):
                self._dispatch("PATCH")

            def do_PUT(self):
                self._dispatch("PUT")

            def do_DELETE(self):
                self._dispatch("DELETE")

        return Handler

    def start(self, background: bool = True) -> dict:
        if self._httpd is not None:
            return {"status": "ok", "message": "tayari inaendesha", "port": self.port}
        editor_result = None
        if self.editor.enabled and os.environ.get("MTS_EDITOR_AUTOSTART", "1") != "0":
            editor_result = self.editor.start()
        self._httpd = ThreadingHTTPServer((self.host, self.port), self._make_handler())
        if background:
            self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)
            self._thread.start()
        else:
            self._httpd.serve_forever()
        return {
            "status": "ok",
            "url": f"http://{self.host}:{self.port}/ide",
            "api": f"http://{self.host}:{self.port}/api/status",
            "canvas_built": bool(self._canvas_build_dir()),
            "editor": editor_result or self.editor.status(),
        }

    def stop(self) -> dict:
        if self._httpd is None:
            return {"status": "ok", "message": "haijaendesha"}
        self._httpd.shutdown()
        self._httpd = None
        self.editor.stop()
        return {"status": "ok", "message": "imesimamishwa"}


# ═══════════════════════════════════════════════════════════
# helpers
# ═══════════════════════════════════════════════════════════

_CTYPES = {
    ".html": "text/html; charset=utf-8",
    ".js": "application/javascript",
    ".mjs": "application/javascript",
    ".css": "text/css; charset=utf-8",
    ".json": "application/json",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".gif": "image/gif",
    ".ico": "image/x-icon",
    ".woff": "font/woff",
    ".woff2": "font/woff2",
    ".map": "application/json",
    ".txt": "text/plain; charset=utf-8",
    ".webmanifest": "application/manifest+json",
}


def _content_type(path: str) -> str:
    return _CTYPES.get(os.path.splitext(path)[1].lower(), "application/octet-stream")


def serve(host: str = "0.0.0.0", port: int = 8080, workspace: str = ".",
          home: Optional[str] = None) -> IDEServer:
    """Anzisha server halisi ya MBILINYI IDE (blocking)."""
    server = IDEServer(host=host, port=port, workspace=workspace, home=home)
    server.start(background=False)
    return server


__all__ = ["IDEServer", "ToolBridge", "serve", "IDE_SERVER_VERSION"]
