"""pytrex.agent_server — Backend halisi ya OpenHands agent-server.

Hii inatekeleza **protokoli halisi ya OpenHands agent-server** ambayo Agent
Canvas (UI ya OpenHands) inaita, huku injini ya agent ikiwa **zana za
T-PYTREXT** (TerminalEmulator, CodeEditor, Linter, CodeFormatter, RAGEngine,
CodeGenerator, AIAgent, LLMIntegration, …).

    OpenHands Agent Canvas  ──/server_info,/api/conversations,/sockets/events──▶  AgentServer
                                                                                    │
                                          ┌─────────────────────────────────────────┘
                                          ▼
                                   AgentRuntime  (kitanzi cha ReAct)
                                          │  LLM (OpenAI/Anthropic/Ollama)
                                          ▼
                                   ToolBridge    (zana HALISI za T-PYTREXT)

Endpoints muhimu (protokoli ya agent-server):

    GET  /server_info                    — toleo + uwezo (capabilities)
    GET  /alive /health /ready           — probes
    GET  /api/settings  PATCH /api/settings
    GET  /api/conversations              — orodha
    POST /api/conversations              — unda mazungumzo
    GET  /api/conversations/{id}
    GET  /api/conversations/{id}/events/search
    POST /api/conversations/{id}/events  — tuma ujumbe + endesha
    POST /api/conversations/{id}/run|pause|interrupt
    WS   /sockets/events/{id}            — mtiririko wa events (live)

KANUNI: hakuna kubuni. Bila LLM key, agent hurejesha ujumbe wa wazi kuwa LLM
haijasanidiwa (si kujifanya kufanya kazi).
"""

from __future__ import annotations

import base64
import hashlib
import json
import os
import struct
import threading
import time
import uuid
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Dict, List, Optional

from .ide import IDEEngine
from .ide_server import IDEServer, ToolBridge, _IDE_TOOL_NAMES
from .canvas_services import CanvasServices

AGENT_SERVER_VERSION = "1.51.0"   # inatosheleza minVersion ya canvas (config/defaults.json)
SDK_VERSION = "1.51.0"

# ═══════════════════════════════════════════════════════════
# Events (muundo wa OpenHands)
# ═══════════════════════════════════════════════════════════


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _eid() -> str:
    return uuid.uuid4().hex


def message_event(role: str, text: str) -> dict:
    return {
        "id": _eid(),
        "kind": "MessageEvent",
        "timestamp": _now(),
        "source": "user" if role == "user" else "agent",
        "llm_message": {"role": role, "content": [{"type": "text", "text": text}]},
        "activated_skills": [],
        "extended_content": [],
    }


def action_event(tool_name: str, tool_call_id: str, action: dict, thought: str = "") -> dict:
    return {
        "id": _eid(),
        "kind": "ActionEvent",
        "timestamp": _now(),
        "source": "agent",
        "thought": [{"type": "text", "text": thought}] if thought else [],
        "action": action,
        "tool_name": tool_name,
        "tool_call_id": tool_call_id,
        "tool_call": {"id": tool_call_id, "name": tool_name, "arguments": json.dumps(action)},
    }


def observation_event(tool_name: str, tool_call_id: str, action_id: str, observation: dict) -> dict:
    return {
        "id": _eid(),
        "kind": "ObservationEvent",
        "timestamp": _now(),
        "source": "environment",
        "action_id": action_id,
        "observation": observation,
        "tool_name": tool_name,
        "tool_call_id": tool_call_id,
    }


def state_event(key: str, value: Any) -> dict:
    return {
        "id": _eid(),
        "kind": "ConversationStateUpdateEvent",
        "timestamp": _now(),
        "source": "environment",
        "key": key,
        "value": value,
    }


def error_event(message: str) -> dict:
    return {
        "id": _eid(),
        "kind": "ConversationErrorEvent",
        "timestamp": _now(),
        "source": "agent",
        "code": "agent_error",
        "detail": message,
    }


# ═══════════════════════════════════════════════════════════
# Conversation store
# ═══════════════════════════════════════════════════════════

class Conversation:
    """Mazungumzo moja: metadata + event log (thread-safe)."""

    def __init__(self, title: str = "New conversation", cid: Optional[str] = None):
        self.id = cid or _eid()
        self.title = title
        self.created_at = _now()
        self.updated_at = self.created_at
        self.agent_status = "idle"
        self.events: List[dict] = []
        self._lock = threading.Lock()

    def add(self, event: dict) -> dict:
        with self._lock:
            self.events.append(event)
            self.updated_at = _now()
        return event

    def snapshot(self) -> List[dict]:
        with self._lock:
            return list(self.events)

    def info(self) -> dict:
        return {
            "id": self.id,
            "title": self.title,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "execution_status": self.agent_status,
            "agent_status": self.agent_status,
            "confirmation_policy": {"kind": "NeverConfirm"},
            "activated_knowledge_skills": [],
            "invoked_skills": [],
            "agent": {"kind": "Agent", "llm": {"model": "pytrex"}},
            "workspace": {"working_dir": "."},
            "persistence_dir": f"/tmp/pytrex-ide/{self.id}",
            "tags": {},
            "leaf_event_id": self.events[-1]["id"] if self.events else None,
            "stats": {"usage_to_metrics": {}},
        }


class ConversationStore:
    def __init__(self) -> None:
        self._convos: Dict[str, Conversation] = {}
        self._lock = threading.Lock()

    def create(self, title: str = "New conversation", cid: Optional[str] = None) -> Conversation:
        c = Conversation(title=title, cid=cid)
        with self._lock:
            self._convos[c.id] = c
        return c

    def get(self, cid: str) -> Optional[Conversation]:
        with self._lock:
            return self._convos.get(cid)

    def delete(self, cid: str) -> bool:
        with self._lock:
            return self._convos.pop(cid, None) is not None

    def list(self) -> List[Conversation]:
        with self._lock:
            return list(self._convos.values())


# ═══════════════════════════════════════════════════════════
# AgentRuntime — kitanzi cha ReAct (LLM + zana za T-PYTREXT)
# ═══════════════════════════════════════════════════════════

_TOOL_SCHEMAS = [
    {
        "type": "function",
        "function": {
            "name": "run_bash",
            "description": "Run a real shell command in the workspace and return its output.",
            "parameters": {
                "type": "object",
                "properties": {"command": {"type": "string", "description": "Shell command"}},
                "required": ["command"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a file from the workspace.",
            "parameters": {
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "write_file",
            "description": "Write content to a file in the workspace.",
            "parameters": {
                "type": "object",
                "properties": {"path": {"type": "string"}, "content": {"type": "string"}},
                "required": ["path", "content"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "list_files",
            "description": "List files in a workspace directory.",
            "parameters": {
                "type": "object",
                "properties": {"path": {"type": "string"}},
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "lint_code",
            "description": "Lint code and return issues (T-PYTREXT Linter).",
            "parameters": {
                "type": "object",
                "properties": {"code": {"type": "string"}},
                "required": ["code"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "format_code",
            "description": "Format code (T-PYTREXT CodeFormatter).",
            "parameters": {
                "type": "object",
                "properties": {"code": {"type": "string"},
                               "language": {"type": "string"}},
                "required": ["code"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "think",
            "description": "Record an internal reasoning note. Use to plan before acting.",
            "parameters": {"type": "object",
                           "properties": {"thought": {"type": "string"}},
                           "required": ["thought"]},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "task_tracker",
            "description": "Plan and track a multi-step task as a checklist.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "enum": ["view", "plan"]},
                    "task_list": {"type": "array", "items": {"type": "object",
                        "properties": {"title": {"type": "string"},
                                       "status": {"type": "string",
                                                  "enum": ["todo", "in_progress", "done"]}},
                        "required": ["title", "status"]}},
                },
                "required": ["command"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "glob",
            "description": "Find files by glob pattern (e.g. '**/*.py') in the workspace.",
            "parameters": {"type": "object",
                           "properties": {"pattern": {"type": "string"},
                                          "path": {"type": "string"}},
                           "required": ["pattern"]},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "grep",
            "description": "Search file contents with a regular expression.",
            "parameters": {"type": "object",
                           "properties": {"pattern": {"type": "string"},
                                          "path": {"type": "string"},
                                          "glob": {"type": "string"}},
                           "required": ["pattern"]},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "str_replace",
            "description": "Replace an exact string in a file (edit). Fails if not unique.",
            "parameters": {"type": "object",
                           "properties": {"path": {"type": "string"},
                                          "old_str": {"type": "string"},
                                          "new_str": {"type": "string"}},
                           "required": ["path", "old_str", "new_str"]},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "fetch_url",
            "description": "Fetch the text content of a URL (HTTP GET).",
            "parameters": {"type": "object",
                           "properties": {"url": {"type": "string"}},
                           "required": ["url"]},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "code_audit",
            "description": "Scan the workspace or a path for security/bug/quality findings.",
            "parameters": {"type": "object",
                           "properties": {"path": {"type": "string"}}},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "run_tests",
            "description": "Run the project's tests (pytest/unittest) and return the summary.",
            "parameters": {"type": "object",
                           "properties": {"path": {"type": "string"}}},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "sdlc",
            "description": "Run the full SDLC lifecycle for a spec (requirements → design → "
                           "build → verify → deploy → monitor). Writes REAL files.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string",
                               "enum": ["capabilities", "assess", "plan", "run", "history"]},
                    "spec": {"type": "string"}, "kind": {"type": "string"},
                    "write": {"type": "boolean"},
                },
                "required": ["action"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "autopilot",
            "description": "Run the Autopilot pipeline (plan → scaffold → review → report) "
                           "to generate a project from an idea.",
            "parameters": {"type": "object",
                           "properties": {"idea": {"type": "string"},
                                          "kind": {"type": "string"}},
                           "required": ["idea"]},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git",
            "description": "Git operations in the workspace (status, diff, log, commit).",
            "parameters": {
                "type": "object",
                "properties": {"action": {"type": "string",
                                          "enum": ["status", "diff", "log", "commit"]},
                               "message": {"type": "string"},
                               "limit": {"type": "integer"}},
                "required": ["action"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "list_skills",
            "description": "List available skills in the workspace skill library.",
            "parameters": {"type": "object",
                           "properties": {"query": {"type": "string"}}},
        },
    },
]

SYSTEM_PROMPT = (
    "You are MBILINYI IDE, an autonomous coding agent running inside the "
    "T-PYTREXT framework. You have real tools and you must use them to inspect "
    "and change real files, never guess:\n"
    "  - run_bash: a real shell in the workspace\n"
    "  - read_file / write_file / list_files / str_replace: file work\n"
    "  - glob / grep: find files and search contents\n"
    "  - lint_code / format_code: quality\n"
    "  - code_audit: security/bug scan (returns grade + findings)\n"
    "  - run_tests: run the project's test suite\n"
    "  - sdlc: full lifecycle (requirements → design → build → verify → deploy → monitor)\n"
    "  - autopilot: generate a project from an idea\n"
    "  - git: status/diff/log/commit\n"
    "  - fetch_url: read a web page\n"
    "  - think / task_tracker: plan multi-step work\n"
    "Plan with task_tracker for multi-step work. Verify your changes (run_tests, "
    "code_audit) before claiming success. Be concise. When the task is done, "
    "reply with a short summary and no further tool calls."
)


class AgentRuntime:
    """Endesha kitanzi cha ReAct kwa mazungumzo moja kwa kutumia zana HALISI."""

    def __init__(self, store: ConversationStore, bridge: ToolBridge,
                 llm_config: dict, max_iterations: int = 12) -> None:
        self.store = store
        self.bridge = bridge
        self.llm_config = llm_config
        self.max_iterations = max_iterations
        self._tasks: List[dict] = []
        self._tools = None

    # ---------- LLM ----------

    def _llm_chat(self, messages: List[dict]) -> dict:
        """Wito HALISI wa LLM (OpenAI-compatible) na tools. Hurejesha message."""
        cfg = self.llm_config
        api_key = cfg.get("api_key") or os.environ.get("OPENAI_API_KEY", "")
        base_url = cfg.get("base_url") or "https://api.openai.com/v1/chat/completions"
        model = cfg.get("model") or "gpt-4o-mini"
        if not api_key:
            return {"error": "LLM not configured: set an API key in Settings "
                             "(POST /api/settings) or OPENAI_API_KEY."}
        payload = {
            "model": model,
            "messages": messages,
            "tools": _TOOL_SCHEMAS,
            "tool_choice": "auto",
            "temperature": cfg.get("temperature", 0.2),
        }
        try:
            import urllib.request
            req = urllib.request.Request(
                base_url, data=json.dumps(payload).encode(),
                headers={"Content-Type": "application/json",
                         "Authorization": f"Bearer {api_key}"}, method="POST")
            with urllib.request.urlopen(req, timeout=120) as resp:
                data = json.loads(resp.read().decode())
            return data["choices"][0]["message"]
        except Exception as e:  # noqa: BLE001
            return {"error": f"LLM call failed: {e}"}

    # ---------- tools ----------

    def _run_tool(self, name: str, args: dict) -> dict:
        if name == "run_bash":
            return {"kind": "TerminalObservation",
                    "content": self.bridge.terminal.run(args.get("command", "")).get("output", "")}
        if name == "read_file":
            r = self.bridge.editor.open(os.path.join(self.bridge.workspace, args.get("path", "")))
            return {"kind": "FileReadObservation",
                    "content": r.get("message", f"opened {args.get('path')} ({r.get('lines', 0)} lines)")}
        if name == "write_file":
            ok = self.bridge.editor.save(os.path.join(self.bridge.workspace, args.get("path", "")),
                                         args.get("content", ""))
            return {"kind": "FileWriteObservation",
                    "content": f"wrote {args.get('path')}" if ok else "write failed"}
        if name == "list_files":
            r = self.bridge._tool_list_files(args.get("path", "."))
            return {"kind": "FileListObservation", "content": "\n".join(r.get("files", []))}
        if name == "lint_code":
            r = self.bridge.linter.lint(args.get("code", ""))
            return {"kind": "LintObservation",
                    "content": json.dumps(r.get("issues", []))}
        if name == "format_code":
            r = self.bridge.formatter.format(args.get("code", ""), args.get("language", "python"))
            return {"kind": "FormatObservation", "content": r.get("formatted", "")}
        if name == "think":
            return {"kind": "ThinkObservation", "content": args.get("thought", "")}
        if name == "task_tracker":
            if args.get("command") == "view":
                return {"kind": "TaskObservation",
                        "content": json.dumps(self._tasks)}
            self._tasks = args.get("task_list", [])
            return {"kind": "TaskObservation",
                    "content": json.dumps({"updated": len(self._tasks)})}
        if name == "glob":
            return {"kind": "GlobObservation",
                    "content": json.dumps(self._glob(args.get("pattern", ""),
                                                     args.get("path", "")))}
        if name == "grep":
            return {"kind": "GrepObservation",
                    "content": json.dumps(self._grep(args.get("pattern", ""),
                                                     args.get("path", ""),
                                                     args.get("glob", "")))}
        if name == "str_replace":
            return {"kind": "EditObservation",
                    "content": self._str_replace(args.get("path", ""),
                                                 args.get("old_str", ""),
                                                 args.get("new_str", ""))}
        if name == "fetch_url":
            return {"kind": "FetchObservation",
                    "content": self._fetch(args.get("url", ""))}
        if name == "code_audit":
            from . import code_audit as _ca
            r = _ca.scan_workspace(self.bridge.workspace, args.get("path", ""))
            return {"kind": "AuditObservation",
                    "content": json.dumps({k: r.get(k) for k in
                                           ("grade", "total_findings", "by_severity")})}
        if name == "run_tests":
            from .ide_tools import TestKit
            r = TestKit(self.bridge.workspace).run(args.get("path", "."))
            return {"kind": "TestObservation", "content": json.dumps(
                {k: r.get(k) for k in ("ok", "passed", "failed", "summary")})}
        if name == "sdlc":
            from .ide_tools import IDETools
            tools = getattr(self, "_tools", None)
            if tools is None:
                from .sdlc import SDLCEngine, SDLCStore
                import os as _os
                st = SDLCStore(_os.path.join(self.bridge.workspace, ".pytrex", "sdlc.json"))
                tools = IDETools(self.bridge.workspace,
                                 SDLCEngine(self.bridge.workspace, st, self.bridge),
                                 self.bridge)
                self._tools = tools
            r = tools.dispatch("sdlc", args)
            return {"kind": "SDLCObservation", "content": json.dumps(
                {"ok": r.get("ok"), "project": r.get("project"),
                 "success": r.get("success"), "summary": r.get("summary")})}
        if name == "autopilot":
            from .ide_tools import IDETools
            tools = getattr(self, "_tools", None)
            if tools is None:
                from .sdlc import SDLCEngine, SDLCStore
                import os as _os
                st = SDLCStore(_os.path.join(self.bridge.workspace, ".pytrex", "sdlc.json"))
                tools = IDETools(self.bridge.workspace,
                                 SDLCEngine(self.bridge.workspace, st, self.bridge),
                                 self.bridge)
                self._tools = tools
            r = tools.dispatch("autopilot", args)
            return {"kind": "AutopilotObservation", "content": json.dumps(
                {"ok": r.get("ok"), "project": r.get("project"),
                 "files": len(r.get("files", [])), "grade": r.get("grade")})}
        if name == "git":
            return {"kind": "GitObservation", "content": self._git(args)}
        if name == "list_skills":
            skills_dir = os.path.join(self.bridge.workspace, "ide", "skills")
            from .canvas_services import SkillsSearch
            r = SkillsSearch(skills_dir).search(args.get("query", ""), limit=30)
            return {"kind": "SkillsObservation",
                    "content": json.dumps([s["name"] for s in r["skills"]])}
        return {"kind": "ErrorObservation", "content": f"unknown tool: {name}"}

    # ---------- tool helpers ----------

    def _glob(self, pattern: str, path: str = "") -> List[str]:
        import glob as _glob
        base = os.path.join(self.bridge.workspace, path)
        if not os.path.isdir(base):
            base = self.bridge.workspace
        hits = _glob.glob(os.path.join(base, pattern), recursive=True)
        return [os.path.relpath(h, self.bridge.workspace) for h in hits[:200]]

    def _grep(self, pattern: str, path: str = "", file_glob: str = "") -> List[dict]:
        import re as _re
        try:
            rx = _re.compile(pattern)
        except _re.error as e:
            return [{"error": f"regex si sahihi: {e}"}]
        base = os.path.join(self.bridge.workspace, path)
        if not os.path.isdir(base):
            base = self.bridge.workspace
        out: List[dict] = []
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in
                           {".git", "node_modules", "target", "__pycache__", ".venv"}]
            for fn in filenames:
                if file_glob and not fn.endswith(file_glob.lstrip("*")):
                    continue
                full = os.path.join(dirpath, fn)
                try:
                    if os.path.getsize(full) > 500_000:
                        continue
                    with open(full, "r", encoding="utf-8", errors="ignore") as fh:
                        for i, line in enumerate(fh, 1):
                            if rx.search(line):
                                out.append({"file": os.path.relpath(full, self.bridge.workspace),
                                            "line": i, "text": line.strip()[:200]})
                                if len(out) >= 200:
                                    return out
                except OSError:
                    continue
        return out

    def _str_replace(self, path: str, old: str, new: str) -> str:
        full = os.path.join(self.bridge.workspace, path)
        if not os.path.abspath(full).startswith(self.bridge.workspace):
            return "path nje ya workspace"
        try:
            with open(full, "r", encoding="utf-8") as fh:
                content = fh.read()
        except OSError as e:
            return f"haipatikani: {e}"
        count = content.count(old)
        if count == 0:
            return "old_str haipo kwenye faili"
        if count > 1:
            return f"old_str inaonekana mara {count} — haijulikani ipi"
        with open(full, "w", encoding="utf-8") as fh:
            fh.write(content.replace(old, new, 1))
        return f"imebadilishwa {path}"

    def _fetch(self, url: str) -> str:
        if not url.startswith(("http://", "https://")):
            return "URL si sahihi"
        try:
            import urllib.request
            req = urllib.request.Request(url, headers={"User-Agent": "T-PYTREXT/1.0"})
            with urllib.request.urlopen(req, timeout=20) as r:
                return r.read(200_000).decode("utf-8", errors="ignore")
        except Exception as e:  # noqa: BLE001
            return f"fetch imeshindikana: {e}"

    def _git(self, args: dict) -> str:
        import subprocess
        action = args.get("action", "status")
        cmd = {"status": ["git", "status", "--porcelain"],
               "diff": ["git", "diff"],
               "log": ["git", "log", f"-{int(args.get('limit', 10))}", "--oneline"],
               "commit": ["git", "commit", "-am", args.get("message", "agent commit")]}
        c = cmd.get(action)
        if not c:
            return "action haijulikani"
        try:
            r = subprocess.run(c, cwd=self.bridge.workspace, capture_output=True,
                               text=True, timeout=30)
            return (r.stdout + r.stderr)[-2000:]
        except OSError as e:
            return str(e)

    # ---------- loop ----------

    def run(self, convo: Conversation, user_text: str) -> None:
        """Endesha mzunguko mmoja: ujumbe wa mtumiaji → majibu ya agent."""
        convo.add(message_event("user", user_text))
        convo.add(state_event("agent_status", "running"))
        convo.agent_status = "running"

        messages = [{"role": "system", "content": SYSTEM_PROMPT}]
        for ev in convo.snapshot():
            if ev.get("kind") == "MessageEvent":
                m = ev["llm_message"]
                messages.append({"role": m["role"], "content": m["content"][0]["text"]})

        try:
            for _ in range(self.max_iterations):
                reply = self._llm_chat(messages)
                if "error" in reply:
                    convo.add(error_event(reply["error"]))
                    break

                tool_calls = reply.get("tool_calls") or []
                if tool_calls:
                    messages.append(reply)
                    for tc in tool_calls:
                        fn = tc.get("function", {})
                        name = fn.get("name", "")
                        try:
                            args = json.loads(fn.get("arguments") or "{}")
                        except json.JSONDecodeError:
                            args = {}
                        act = action_event(name, tc.get("id", _eid()), args,
                                           thought=reply.get("content") or "")
                        convo.add(act)
                        obs = self._run_tool(name, args)
                        convo.add(observation_event(name, tc.get("id", _eid()), act["id"], obs))
                        messages.append({
                            "role": "tool", "tool_call_id": tc.get("id", ""),
                            "name": name, "content": obs.get("content", ""),
                        })
                    continue

                text = reply.get("content") or "(no response)"
                convo.add(message_event("assistant", text))
                break
        finally:
            convo.agent_status = "idle"
            convo.add(state_event("agent_status", "idle"))


# ═══════════════════════════════════════════════════════════
# AgentServer — HTTP + WebSocket (protokoli ya agent-server)
# ═══════════════════════════════════════════════════════════

class AgentServer(IDEServer):
    """IDEServer + protokoli kamili ya OpenHands agent-server."""

    def __init__(self, host: str = "0.0.0.0", port: int = 8080,
                 workspace: str = ".", home: Optional[str] = None) -> None:
        super().__init__(host=host, port=port, workspace=workspace, home=home)
        self.store = ConversationStore()
        self.llm_config: Dict[str, Any] = {
            "api_key": os.environ.get("OPENAI_API_KEY", ""),
            "model": os.environ.get("OPENAI_MODEL", "gpt-4o-mini"),
            "base_url": "",
        }
        self.runtime = AgentRuntime(self.store, self.bridge, self.llm_config)
        skills_root = os.path.join(self.engine.home or self.workspace, "skills")
        self.services = CanvasServices(
            self.workspace,
            os.path.join(self.workspace, ".pytrex"),
            skills_root,
        )
        self._ws_clients: List[Any] = []
        self._ws_lock = threading.Lock()
        self.started_at = time.time()

    # ---------- routing ----------

    def _route(self, method: str, path: str, query: dict, body: str) -> dict:
        # ---- agent-server protocol ----
        if path == "/server_info":
            return {"uptime": time.time() - self.started_at, "idle_time": 0.0,
                    "title": "MBILINYI IDE (T-PYTREXT)", "version": AGENT_SERVER_VERSION,
                    "sdk_version": SDK_VERSION, "tools_version": SDK_VERSION,
                    "workspace_version": SDK_VERSION,
                    "capabilities": ["workspaces", "hooks", "mcp-test", "mcp-oauth",
                                     "pytrex-tools", "ide-tools"],
                    "conversation_runtime": "local"}
        if path == "/alive":
            return {"status": "ok"}
        if path in ("/health", "/ready"):
            return {"status": "ok"}
        if path == "/":
            return {"name": "MBILINYI IDE", "version": AGENT_SERVER_VERSION,
                    "framework": "T-PYTREXT", "docs": "/ide"}

        # ---- settings ----
        if path in ("/api/settings", "/api/v1/settings"):
            if method == "PATCH" or method == "POST":
                diff = self._json(body)
                agent_diff = diff.get("agent_settings_diff") or {}
                conv_diff = diff.get("conversation_settings_diff") or {}
                merged = {**agent_diff, **conv_diff}
                if "llm_api_key" in merged and merged["llm_api_key"]:
                    self.llm_config["api_key"] = merged["llm_api_key"]
                if "llm_model" in merged and merged["llm_model"]:
                    self.llm_config["model"] = merged["llm_model"]
                if "llm_base_url" in merged:
                    self.llm_config["base_url"] = merged["llm_base_url"]
            return self._settings_payload()
        if path == "/api/v1/settings/profiles" or path == "/api/agent-profiles":
            return {"profiles": [], "items": []}
        if path == "/api/profiles":
            return self._profiles_payload()
        if path.startswith("/api/profiles/"):
            return self._profile_route(method, path, body)
        if path == "/api/meta-profiles":
            return {"meta_profiles": [], "active_meta_profile": None}
        if path.startswith("/api/meta-profiles/"):
            return {"status": "ok", "message": "meta-profile saved", "name": path.rsplit("/", 1)[-1]}
        if path == "/api/automation/sdk-version":
            return {"sdk_version": SDK_VERSION}
        if path == "/api/llm/models/verified":
            return {"models": {}}
        if path == "/api/llm/models":
            return self.services.config.search_models(
                query.get("query", ""), query.get("provider", ""))
        if path == "/api/llm/providers":
            return self.services.config.search_providers(query.get("query", ""))
        # ---- provider connections (CRUD) ----
        if path == "/api/llm/provider-connections":
            if method == "GET":
                items = self.services.connections.list()
                return {"provider_connections": items, "items": items,
                        "total_count": len(items)}
            if method == "POST":
                p = self._json(body)
                try:
                    conn = self.services.connections.create(
                        p.get("display_name", ""), p.get("provider", ""),
                        p.get("api_key", ""), p.get("base_url"))
                    return conn
                except ValueError as e:
                    return {"_status": 400, "_ctype": "application/json",
                            "_raw": json.dumps({"detail": str(e)})}
        if path.startswith("/api/llm/provider-connections/"):
            cid = path.rsplit("/", 1)[-1]
            try:
                if method == "DELETE":
                    return self.services.connections.delete(cid)
                if method in ("PATCH", "PUT"):
                    p = self._json(body)
                    return self.services.connections.update(
                        cid, p.get("display_name"), p.get("provider"),
                        p.get("api_key"), p.get("base_url"))
                if method == "GET":
                    for c in self.services.connections.list():
                        if c["id"] == cid:
                            return c
            except KeyError as e:
                return {"_status": 404, "_ctype": "application/json",
                        "_raw": json.dumps({"detail": str(e)})}
        # ---- config search ----
        if path in ("/api/v1/config/models/search", "/api/v1/config/models"):
            return self.services.config.search_models(
                query.get("query", ""), query.get("provider", ""))
        if path in ("/api/v1/config/providers/search", "/api/v1/config/providers"):
            return self.services.config.search_providers(query.get("query", ""))
        # ---- secrets ----
        if path in ("/api/settings/secrets", "/api/secrets"):
            if method in ("PUT", "POST"):
                p = self._json(body)
                try:
                    return self.services.secrets.upsert(
                        p.get("name", ""), p.get("value", ""), p.get("description", ""))
                except ValueError as e:
                    return {"_status": 400, "_ctype": "application/json",
                            "_raw": json.dumps({"detail": str(e)})}
            items = self.services.secrets.list()
            return {"secrets": items, "items": items}
        if path.startswith("/api/settings/secrets/"):
            name = path.rsplit("/", 1)[-1]
            if method == "DELETE":
                ok = self.services.secrets.delete(name)
                return {"status": "ok" if ok else "not_found"}
            if method == "GET":
                val = self.services.secrets.get(name)
                return {"name": name, "value": val} if val is not None else \
                    {"_status": 404, "_ctype": "application/json",
                     "_raw": json.dumps({"detail": "secret haipo"})}
        if path == "/api/v1/secrets":
            if method == "POST":
                p = self._json(body)
                try:
                    return self.services.secrets.upsert(
                        p.get("name", ""), p.get("value", ""), p.get("description", ""))
                except ValueError as e:
                    return {"_status": 400, "_ctype": "application/json",
                            "_raw": json.dumps({"detail": str(e)})}
            items = self.services.secrets.list()
            return {"secrets": items, "items": items, "total_count": len(items)}
        if path == "/api/v1/secrets/search":
            return self.services.secrets.search(query.get("query", ""))
        if path.startswith("/api/v1/secrets/"):
            name = path.rsplit("/", 1)[-1]
            if method in ("PUT", "PATCH"):
                p = self._json(body)
                return self.services.secrets.upsert(name, p.get("value", ""),
                                                    p.get("description", ""))
            if method == "DELETE":
                return {"status": "ok" if self.services.secrets.delete(name) else "not_found"}
        # ---- MCP ----
        if path in ("/api/mcp/servers", "/api/v1/mcp/servers"):
            if method == "POST":
                p = self._json(body)
                return self.services.mcp.add(p.get("name", ""), p.get("url", ""),
                                             p.get("transport", "http"))
            return {"servers": self.services.mcp.list(),
                    "items": self.services.mcp.list()}
        if path.startswith("/api/mcp/servers/"):
            sid = path.rsplit("/", 1)[-1]
            if method == "DELETE":
                return {"status": "ok" if self.services.mcp.delete(sid) else "not_found"}
        if path in ("/api/mcp/health", "/api/v1/mcp/health"):
            return self.services.mcp.health(query.get("id", ""))
        if path in ("/api/mcp/test", "/api/v1/mcp/test"):
            return self.services.mcp.health(self._json(body).get("id", ""))
        if path in ("/api/mcp/oauth/start", "/api/v1/mcp/oauth/start") and method == "POST":
            return self.services.mcp.oauth_start(self._json(body).get("server", ""))
        if path.startswith("/api/mcp/oauth/status/") or path.startswith("/api/v1/mcp/oauth/status/"):
            return self.services.mcp.oauth_status(path.rsplit("/", 1)[-1])
        # ---- sandboxes ----
        if path in ("/api/v1/sandboxes", "/api/sandboxes"):
            if method == "POST":
                p = self._json(body)
                return self.services.sandboxes.create(p.get("name", ""), p.get("image", "local"))
            return {"sandboxes": self.services.sandboxes.list(),
                    "items": self.services.sandboxes.list()}
        if path.startswith("/api/v1/sandboxes/") or path.startswith("/api/sandboxes/"):
            rest = path.rstrip("/").rsplit("/", 2)
            sid = rest[-1] if rest[-1] not in ("pause", "resume") else rest[-2]
            op = path.rstrip("/").rsplit("/", 1)[-1]
            try:
                if op == "pause":
                    return self.services.sandboxes.pause(sid)
                if op == "resume":
                    return self.services.sandboxes.resume(sid)
            except KeyError as e:
                return {"_status": 404, "_ctype": "application/json",
                        "_raw": json.dumps({"detail": str(e)})}
        # ---- organizations ----
        if path == "/api/organizations":
            if method == "POST":
                return self.services.organizations.create(self._json(body).get("name", ""))
            return self.services.organizations.list()
        if path.startswith("/api/organizations/"):
            parts = path.split("/")
            org_id = parts[3] if len(parts) > 3 else ""
            tail = parts[4] if len(parts) > 4 else ""
            try:
                if tail == "profiles":
                    return self.services.organizations.profiles(org_id)
                if tail == "provider-connections":
                    return self.services.organizations.provider_connections(org_id)
                return {"organization": org_id}
            except KeyError as e:
                return {"_status": 404, "_ctype": "application/json",
                        "_raw": json.dumps({"detail": str(e)})}
        # ---- files ----
        if path == "/api/file/download":
            return self.services.files.download(query.get("path", ""))
        if path == "/api/file/upload" and method == "POST":
            p = self._json(body)
            return self.services.files.upload(p.get("path", ""), p.get("content_base64", ""))
        if path == "/api/settings/agent-schema" or path == "/api/v1/settings/agent-schema":
            return self._agent_schema()
        if path == "/api/workspaces":
            return self.services.workspaces.list()
        if path == "/api/file/home":
            return self.services.files.home()
        if path == "/api/skills" and method == "POST":
            return {"skills": []}
        if path == "/api/hooks":
            return {"hooks": []}
        if path == "/api/plugins/installed":
            return {"plugins": [], "items": []}
        if path == "/api/auth/workspace-session":
            return {"status": "ok"}
        if path in ("/api/automation/health", "/api/automation/v1"):
            return {"status": "ok"}
        if path.startswith("/api/automation/"):
            return {"status": "ok"}
        if path == "/api/v1/settings/conversation-schema" or path == "/api/settings/conversation-schema":
            return self._conversation_schema()
        if path == "/api/v1/web-client/config":
            return {"app_mode": "local", "feature_flags": {}}
        if path == "/api/authenticate":
            return {"status": "ok"}
        if path == "/api/keys/current":
            return {"keys": [], "items": []}
        if path == "/api/v1/skills":
            return self._skills_payload()
        if path == "/api/v1/skills/search":
            return self.services.skills.search(query.get("query", ""))
        if path in ("/api/canvas-extensions", "/api/canvas-extensions/installed"):
            return {"extensions": [], "items": []}

        # ---- conversations ----
        if path == "/api/conversations" and method == "GET":
            convos = self.store.list()
            return {"items": [c.info() for c in convos], "total_count": len(convos)}
        if path == "/api/conversations" and method == "POST":
            payload = self._json(body)
            title = payload.get("title") or "New conversation"
            convo = self.store.create(title=title, cid=payload.get("conversation_id"))
            init = payload.get("initial_message")
            if init:
                text = ""
                content = init.get("content")
                if isinstance(content, list) and content:
                    text = content[0].get("text", "")
                if text:
                    self._start_run(convo, text)
            return convo.info()
        if path == "/api/conversations/count":
            return len(self.store.list())
        if path == "/api/conversations/search":
            convos = self.store.list()
            return {"items": [c.info() for c in convos], "total_count": len(convos)}
        if path == "/api/v1/app-conversations" and method == "GET":
            convos = self.store.list()
            return {"items": [self._app_conversation(c) for c in convos],
                    "total_count": len(convos)}
        if path == "/api/v1/app-conversations/search":
            convos = self.store.list()
            return {"items": [self._app_conversation(c) for c in convos],
                    "total_count": len(convos)}
        if path.startswith("/api/v1/app-conversations/"):
            parts = [p for p in path.split("/") if p]
            cid = parts[3] if len(parts) > 3 else ""
            tail = parts[4] if len(parts) > 4 else ""
            convo = self.store.get(cid)
            if convo is None:
                return {"_status": 404, "_ctype": "application/json",
                        "_raw": json.dumps({"detail": "conversation haipo"})}
            if not tail:
                return self._app_conversation(convo)
            if tail == "download":
                return {"ok": True, "files": [e.get("id") for e in convo.snapshot()]}
            if tail == "file" or tail == "files":
                return {"ok": True, "files": []}
            if tail == "skills":
                return self._skills_payload()
            if tail in ("switch_acp_model", "switch_profile") and method == "POST":
                return {"status": "ok"}
            if tail == "git":
                sub = parts[5] if len(parts) > 5 else "changes"
                return self.tools.dispatch("integrations",
                                           {"action": f"git/{sub}" if sub != "git" else "git/changes"})
        if path == "/api/admin/setup-state":
            return {"setup_complete": True, "needs_setup": False,
                    "llm_api_key_is_set": bool(self.llm_config.get("api_key"))}
        if path in ("/api/shared-conversations", "/api/shared-events/search"):
            return {"items": [], "total_count": 0}
        if path == "/api/bash/execute_bash_command" and method == "POST":
            p = self._json(body)
            return self.bridge.terminal.run(p.get("command", ""),
                                            int(p.get("timeout", 30)))
        if path == "/api/cloud-proxy":
            return {"ok": False, "error": "cloud proxy haipatikani (local mode)"}

        # ---- SSE streams ----
        if path in ("/api/sdlc/stream", "/api/sdlc/run/stream"):
            return self._sse_sdlc(self._json(body))
        if path in ("/api/autopilot/stream", "/api/autopilot/run/stream"):
            return self._sse_autopilot(self._json(body))
        if path in ("/api/ide/code/audit/stream", "/api/audit/stream"):
            return self._sse_audit(self._json(body))
        if path.startswith("/api/ide/") and path.endswith("/stream"):
            tool = path[len("/api/ide/"):].rsplit("/", 1)[0].split("/")[0]
            if tool in _IDE_TOOL_NAMES:
                return self._sse_tool_stream(tool, self._json(body))

        # per-conversation routes
        if path.startswith("/api/conversations/"):
            return self._conversation_route(method, path, body, query)

        # ---- everything else: IDE routes (super) ----
        return super()._route(method, path, query, body)

    # ---------- conversation sub-routes ----------

    def _conversation_route(self, method: str, path: str, body: str,
                            query: Optional[dict] = None) -> dict:
        rest = path[len("/api/conversations/"):]
        parts = rest.split("/")
        cid = parts[0]
        convo = self.store.get(cid)
        if convo is None:
            return {"_status": 404, "_raw": json.dumps({"error": "conversation not found"}),
                    "_ctype": "application/json"}
        tail = parts[1:] if len(parts) > 1 else []

        if not tail and method == "GET":
            return convo.info()
        if not tail and method == "DELETE":
            self.store.delete(cid)
            return {"status": "ok"}
        if tail == ["events"] and method == "GET":
            if self._wants_sse(query):
                return self._sse_conversation(convo)
            return {"items": convo.snapshot(), "total_count": len(convo.events)}
        if tail == ["events", "count"]:
            return {"count": len(convo.events), "total_count": len(convo.events)}
        if tail == ["events", "search"]:
            return {"items": convo.snapshot(), "total_count": len(convo.events)}
        if tail == ["events", "respond_to_confirmation"] and method == "POST":
            return {"status": "ok", "accepted": True}
        if tail == ["git", "changes"]:
            return self.tools.dispatch("integrations", {"action": "git/changes"})
        if tail == ["git", "diff"]:
            return self.tools.dispatch("integrations", {"action": "git/diff"})
        if tail == ["skills"]:
            return self._skills_payload()
        if tail and tail[0] == "workspace":
            return {"ok": True, "working_dir": self.workspace}
        if tail in (["switch_acp_model"], ["switch_profile"]) and method == "POST":
            p = self._json(body)
            if p.get("model"):
                self.llm_config["model"] = p["model"]
            if p.get("profile"):
                self.llm_config["model"] = p["profile"]
            return {"status": "ok"}
        if tail == ["events"] and method == "POST":
            payload = self._json(body)
            text = self._extract_text(payload)
            if text:
                self._start_run(convo, text)
            return {"status": "ok"}
        if tail == ["run"] and method == "POST":
            return {"status": "ok"}
        if tail in (["pause"], ["interrupt"]) and method == "POST":
            convo.agent_status = "paused"
            convo.add(state_event("agent_status", "paused"))
            return {"status": "ok"}
        if tail == ["runtime"]:
            return {"runtime_id": cid, "status": "ready"}
        if tail == ["secrets"]:
            return {"secrets": [], "items": []}
        if tail == ["confirmation_policy"]:
            return {"kind": "NeverConfirm"}
        return {"_status": 404, "_raw": json.dumps({"error": "route not found"}),
                "_ctype": "application/json"}

    # ---------- helpers ----------

    @staticmethod
    def _extract_text(payload: dict) -> str:
        # {role, content:[{type,text}]} au {message:"..."} au {text:"..."}
        if isinstance(payload.get("message"), str):
            return payload["message"]
        if isinstance(payload.get("text"), str):
            return payload["text"]
        content = payload.get("content")
        if isinstance(content, str):
            return content
        if isinstance(content, list) and content:
            return content[0].get("text", "")
        return ""

    @staticmethod
    def _wants_sse(query: dict) -> bool:
        if not query:
            return False
        val = (query.get("stream") or query.get("sse") or "").lower()
        return val in ("1", "true", "yes", "sse")

    def _start_run(self, convo: Conversation, text: str) -> None:
        t = threading.Thread(target=self.runtime.run, args=(convo, text), daemon=True)
        t.start()

    # ---------- SSE streams ----------

    def _sse_conversation(self, convo: Conversation) -> dict:
        """Stream ya matukio ya mazungumzo (SSE halisi)."""
        events = []
        for ev in convo.snapshot():
            events.append({"event": ev.get("kind", "event"), "data": ev})
        if not events:
            events.append({"event": "state", "data": {"agent_status": convo.agent_status}})
        return {"_sse": True, "_sse_events": events}

    def _sse_text_stream(self, events: List[dict]) -> dict:
        return {"_sse": True, "_sse_events": events}

    def _sse_sdlc(self, payload: dict) -> dict:
        """Stream ya hatua za SDLC — kila awamu hutumwa anapokamilika."""
        events: List[dict] = []
        action = payload.get("action", "plan")
        if action == "run":
            result = self.tools.dispatch("sdlc", {**payload, "action": "run"})
            for ph in result.get("phases", []):
                events.append({"event": "phase", "data": {
                    "id": ph.get("id"), "label": ph.get("label"),
                    "status": ph.get("status")}})
            events.append({"event": "done", "data": {
                "ok": result.get("ok"), "project": result.get("project"),
                "success": result.get("success"), "score": result.get("score")}})
        else:
            result = self.tools.dispatch("sdlc", {**payload, "action": "plan"})
            for ph in result.get("phase_plan", []):
                events.append({"event": "plan", "data": {"id": ph.get("id"),
                                                         "depth": ph.get("depth")}})
            events.append({"event": "done", "data": {"ok": result.get("ok")}})
        return self._sse_text_stream(events)

    def _sse_autopilot(self, payload: dict) -> dict:
        result = self.tools.dispatch("autopilot", payload)
        events = []
        for st in result.get("steps", []):
            events.append({"event": "step", "data": st})
        events.append({"event": "done", "data": {
            "ok": result.get("ok"), "project": result.get("project"),
            "files": len(result.get("files", [])), "grade": result.get("grade")}})
        return self._sse_text_stream(events)

    def _sse_audit(self, payload: dict) -> dict:
        result = self.tools.dispatch("audit", payload)
        events = [{"event": "summary", "data": {
            "grade": result.get("grade"), "total_findings": result.get("total_findings"),
            "by_severity": result.get("by_severity")}}]
        for f in (result.get("findings") or [])[:50]:
            events.append({"event": "finding", "data": f})
        events.append({"event": "done", "data": {"ok": result.get("ok")}})
        return self._sse_text_stream(events)

    def _sse_tool_stream(self, tool: str, payload: dict) -> dict:
        """Stream ya zana yoyote: hutoa matokeo kama tukio moja."""
        result = self.tools.dispatch(tool, payload)
        return self._sse_text_stream([
            {"event": "result", "data": result},
            {"event": "done", "data": {"ok": result.get("ok", True)}},
        ])

    def _settings_payload(self) -> dict:
        return {
            "agent_settings": {
                "llm_model": self.llm_config.get("model", ""),
                "llm_base_url": self.llm_config.get("base_url", ""),
                "confirmation_mode": False,
                "security_analyzer": "none",
            },
            "conversation_settings": {"max_iterations": self.runtime.max_iterations},
            "llm_api_key_is_set": bool(self.llm_config.get("api_key")),
            "misc_settings": {"app_preferences": {}},
        }

    # ---------- LLM profiles (canvas settings) ----------

    def _profiles_payload(self) -> dict:
        """Orodha ya profiles — moja ya 'default' inayotokana na llm_config."""
        name = self.llm_config.get("model") or "default"
        profile = {
            "name": name,
            "model": self.llm_config.get("model", ""),
            "base_url": self.llm_config.get("base_url") or None,
            "api_key_set": bool(self.llm_config.get("api_key")),
            "provider_connection_id": None,
            "provider_connection_broken": False,
        }
        return {"profiles": [profile], "active_profile": name}

    def _profile_route(self, method: str, path: str, body: str) -> dict:
        name = path[len("/api/profiles/"):].split("/")[0]
        tail = path[len("/api/profiles/"):].split("/")[1:]
        if tail == ["activate"]:
            return {"status": "ok", "message": f"activated {name}", "name": name}
        if tail == ["validate"]:
            return {"valid": bool(self.llm_config.get("api_key")),
                    "error": None if self.llm_config.get("api_key")
                    else {"type": "missing_api_key", "message": "LLM key not set"}}
        if method == "GET":
            return {"name": name, "model": self.llm_config.get("model", ""),
                    "base_url": self.llm_config.get("base_url") or None,
                    "api_key_set": bool(self.llm_config.get("api_key")),
                    "llm": {"model": self.llm_config.get("model", "")}}
        if method == "DELETE":
            return {"status": "ok", "message": f"deleted {name}", "name": name}
        # POST/PUT: hifadhi profile
        payload = self._json(body)
        llm = payload.get("llm") or {}
        if llm.get("model"):
            self.llm_config["model"] = llm["model"]
        if llm.get("base_url"):
            self.llm_config["base_url"] = llm["base_url"]
        if payload.get("include_secrets") and llm.get("api_key"):
            self.llm_config["api_key"] = llm["api_key"]
        return {"status": "ok", "message": f"saved {name}", "name": name}

    def _agent_schema(self) -> dict:
        """Schema ya settings za agent kwa UI ya canvas.

        Canvas hupitia `sections` na kutafuta `key === "llm"` (llm-settings /
        onboarding). Bila `sections` UI huanguka ("Cannot read properties of
        undefined (reading 'find')"), kwa hivyo muundo huu ni wa lazima.
        """
        llm_fields = [
            self._field("llm.model", "Model", "llm", "string",
                        default="gpt-4o-mini", required=True, prominence="critical"),
            self._field("llm.api_key", "API Key", "llm", "string",
                        secret=True, prominence="critical"),
            self._field("llm.base_url", "Base URL", "llm", "string",
                        prominence="critical"),
            self._field("llm.temperature", "Temperature", "llm", "number"),
        ]
        return {
            "model_name": "AgentSettings",
            "sections": [
                {"key": "llm", "label": "LLM", "fields": llm_fields},
                {"key": "general", "label": "General", "fields": [
                    self._field("tool_concurrency_limit", "Parallel tool calls",
                                "general", "integer", default=1),
                ]},
            ],
        }

    @staticmethod
    def _field(key: str, label: str, section: str, value_type: str,
               default: Any = None, secret: bool = False,
               required: bool = False,
               prominence: str = "major") -> dict:
        return {
            "key": key, "label": label, "section": section,
            "section_label": section.capitalize(), "value_type": value_type,
            "default": default, "choices": [], "depends_on": [],
            "prominence": prominence, "secret": secret, "required": required,
        }

    def _conversation_schema(self) -> dict:
        """Schema ya settings za mazungumzo (Canvas huitumia kama fallback)."""
        return {
            "model_name": "ConversationSettings",
            "sections": [
                {"key": "conversation", "label": "Conversation", "fields": [
                    self._field("max_iterations", "Max iterations",
                                "conversation", "integer",
                                default=self.runtime.max_iterations,
                                prominence="critical"),
                    self._field("confirmation_mode", "Confirmation mode",
                                "conversation", "boolean", default=False),
                ]},
            ],
        }

    def _skills_payload(self) -> dict:
        skills_dir = os.path.join(self.engine.home or "", "skills")
        items = []
        if os.path.isdir(skills_dir):
            for root, _dirs, files in os.walk(skills_dir):
                if "SKILL.md" in files:
                    name = os.path.relpath(root, skills_dir)
                    items.append({"name": name, "path": root})
        return {"skills": items, "items": items, "total_count": len(items)}

    def _app_conversation(self, c: Conversation) -> dict:
        info = c.info()
        info.update({
            "app_conversation_id": c.id,
            "agent_server_url": f"http://{self.host}:{self.port}",
            "created_by_user_id": "local",
        })
        return info

    # ---------- WebSocket ----------

    def _register_ws(self, sock) -> None:
        with self._ws_lock:
            self._ws_clients.append(sock)

    def _unregister_ws(self, sock) -> None:
        with self._ws_lock:
            if sock in self._ws_clients:
                self._ws_clients.remove(sock)

    def broadcast(self, event: dict) -> None:
        frame = _ws_encode(json.dumps(event))
        with self._ws_lock:
            clients = list(self._ws_clients)
        for c in clients:
            try:
                c.sendall(frame)
            except OSError:
                self._unregister_ws(c)

    # ---------- handler ----------

    def _make_handler(self):
        server = self
        base = super()._make_handler()

        class Handler(base):  # type: ignore[misc, valid-type]
            def do_GET(self):
                # Editor halisi (`/ide/vscode`) ina WebSocket zake — proxy kwanza.
                if server._is_editor_path(self.path):
                    server._proxy_editor(self)
                    return
                if self.headers.get("Upgrade", "").lower() == "websocket":
                    self._ws(server)
                    return
                super().do_GET()

            def _ws(self, srv: "AgentServer"):
                import urllib.parse as up
                parsed = up.urlparse(self.path)
                key = self.headers.get("Sec-WebSocket-Key", "")
                accept = base64.b64encode(hashlib.sha1(
                    (key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
                self.send_response(101)
                self.send_header("Upgrade", "websocket")
                self.send_header("Connection", "Upgrade")
                self.send_header("Sec-WebSocket-Accept", accept)
                self.end_headers()
                sock = self.connection
                srv._register_ws(sock)
                # mtiririko: tuma events mpya za mazungumzo (poll)
                cid = parsed.path.rsplit("/", 1)[-1] if parsed.path.startswith("/sockets/events/") else ""
                convo = srv.store.get(cid) if cid else None
                sent = 0
                try:
                    while True:
                        time.sleep(0.4)
                        if convo:
                            events = convo.snapshot()
                            for ev in events[sent:]:
                                sock.sendall(_ws_encode(json.dumps(ev)))
                                sent = len(events)
                        else:
                            # ping/keepalive
                            sock.sendall(_ws_encode(json.dumps({"kind": "ping", "timestamp": _now()})))
                except (OSError, BrokenPipeError):
                    pass
                finally:
                    srv._unregister_ws(sock)

        return Handler


# ═══════════════════════════════════════════════════════════
# Minimal WebSocket frame encoding (server → client, no mask)
# ═══════════════════════════════════════════════════════════

def _ws_encode(message: str) -> bytes:
    data = message.encode("utf-8")
    header = bytearray([0x81])  # FIN + text frame
    n = len(data)
    if n < 126:
        header.append(n)
    elif n < (1 << 16):
        header.append(126)
        header += struct.pack(">H", n)
    else:
        header.append(127)
        header += struct.pack(">Q", n)
    return bytes(header) + data


def serve(host: str = "0.0.0.0", port: int = 8080, workspace: str = ".",
          home: Optional[str] = None) -> AgentServer:
    """Anzisha AgentServer halisi (blocking)."""
    server = AgentServer(host=host, port=port, workspace=workspace, home=home)
    server.start(background=False)
    return server


__all__ = [
    "AgentServer", "AgentRuntime", "ConversationStore", "Conversation",
    "serve", "AGENT_SERVER_VERSION",
    "message_event", "action_event", "observation_event", "state_event", "error_event",
]
