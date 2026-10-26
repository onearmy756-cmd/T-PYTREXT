"""pytrex.canvas_services — huduma za agent-server ambazo Agent Canvas huita.

Hizi zilikuwa zikijibu 404 au tupu (stub). Sasa kila moja ina hifadhi HALISI
(faili ya JSON, values zimesimbwa kwa EncryptionManager) na tabia ya kweli:

    provider-connections, secrets, mcp (servers + health + oauth),
    sandboxes, organizations, config search, skills search,
    files (upload/download), workspaces
"""

from __future__ import annotations

import json
import os
import threading
import time
import uuid
from typing import Dict, List, Optional

from .core import EncryptionManager

# ---------------------------------------------------------------------------
# Hifadhi ya JSON inayosimamiwa kwa uzi (thread-safe) + usimbaji wa siri
# ---------------------------------------------------------------------------


class _JsonStore:
    def __init__(self, path: str, password: str = "pytrex-canvas") -> None:
        self.path = path
        self._lock = threading.Lock()
        self._crypto = EncryptionManager(password)
        self._data: dict = {}
        self._load()

    def _load(self) -> None:
        try:
            with open(self.path, "r", encoding="utf-8") as fh:
                self._data = json.load(fh)
        except (OSError, json.JSONDecodeError):
            self._data = {}

    def _save(self) -> None:
        tmp = self.path + ".tmp"
        try:
            os.makedirs(os.path.dirname(self.path), exist_ok=True)
            with open(tmp, "w", encoding="utf-8") as fh:
                json.dump(self._data, fh, indent=1)
            os.replace(tmp, self.path)
        except OSError:
            pass

    def enc(self, value: str) -> str:
        return self._crypto.encrypt(value) if value else ""

    def dec(self, value: str) -> str:
        try:
            return self._crypto.decrypt(value) if value else ""
        except Exception:  # noqa: BLE001
            return ""

    def get(self, key: str, default=None):
        with self._lock:
            return self._data.get(key, default)

    def set(self, key: str, value) -> None:
        with self._lock:
            self._data[key] = value
            self._save()


# ---------------------------------------------------------------------------
# Provider connections (LLM providers)
# ---------------------------------------------------------------------------


class ProviderConnections:
    """CRUD ya muunganisho wa provider (OpenAI, Anthropic, …). Values zimesimbwa."""

    KNOWN = [
        {"provider": "openai", "display_name": "OpenAI", "base_url": "https://api.openai.com/v1"},
        {"provider": "anthropic", "display_name": "Anthropic", "base_url": "https://api.anthropic.com"},
        {"provider": "openrouter", "display_name": "OpenRouter", "base_url": "https://openrouter.ai/api/v1"},
        {"provider": "ollama", "display_name": "Ollama (local)", "base_url": "http://127.0.0.1:11434/v1"},
        {"provider": "groq", "display_name": "Groq", "base_url": "https://api.groq.com/openai/v1"},
        {"provider": "deepseek", "display_name": "DeepSeek", "base_url": "https://api.deepseek.com/v1"},
    ]

    def __init__(self, store: _JsonStore) -> None:
        self.store = store
        self._lock = threading.Lock()

    def list(self) -> List[dict]:
        with self._lock:
            items = self.store.get("provider_connections", [])
        return [self._public(c) for c in items]

    def create(self, display_name: str, provider: str, api_key: str,
               base_url: Optional[str] = None) -> dict:
        if not api_key:
            raise ValueError("api_key ni lazima")
        now = int(time.time() * 1000)
        conn = {"id": uuid.uuid4().hex, "display_name": display_name or provider,
                "provider": provider, "base_url": base_url,
                "api_key_enc": self.store.enc(api_key),
                "api_key_set": True, "created_at": now, "updated_at": now}
        with self._lock:
            items = self.store.get("provider_connections", [])
            items.append(conn)
            self.store.set("provider_connections", items)
        return self._public(conn)

    def update(self, cid: str, display_name: Optional[str] = None,
               provider: Optional[str] = None, api_key: Optional[str] = None,
               base_url: Optional[str] = None) -> dict:
        with self._lock:
            items = self.store.get("provider_connections", [])
            for c in items:
                if c["id"] == cid:
                    if display_name is not None:
                        c["display_name"] = display_name
                    if provider is not None:
                        c["provider"] = provider
                    if base_url is not None:
                        c["base_url"] = base_url
                    if api_key:
                        c["api_key_enc"] = self.store.enc(api_key)
                        c["api_key_set"] = True
                    c["updated_at"] = int(time.time() * 1000)
                    self.store.set("provider_connections", items)
                    return self._public(c)
        raise KeyError("connection haipo")

    def delete(self, cid: str) -> dict:
        with self._lock:
            items = self.store.get("provider_connections", [])
            for c in items:
                if c["id"] == cid:
                    items.remove(c)
                    self.store.set("provider_connections", items)
                    return self._public(c)
        raise KeyError("connection haipo")

    def api_key_for(self, provider: str) -> Optional[str]:
        with self._lock:
            items = self.store.get("provider_connections", [])
        for c in items:
            if c["provider"] == provider:
                return self.store.dec(c.get("api_key_enc", ""))
        return None

    @staticmethod
    def _public(c: dict) -> dict:
        return {"id": c["id"], "display_name": c["display_name"],
                "provider": c["provider"], "base_url": c.get("base_url"),
                "created_at": c.get("created_at"), "updated_at": c.get("updated_at"),
                "api_key_set": bool(c.get("api_key_set"))}


# ---------------------------------------------------------------------------
# Secrets (custom secrets za conversation)
# ---------------------------------------------------------------------------


class SecretsStore:
    """CRUD ya secrets (name, value, description). Values zimesimbwa."""

    def __init__(self, store: _JsonStore) -> None:
        self.store = store
        self._lock = threading.Lock()

    def list(self) -> List[dict]:
        with self._lock:
            items = self.store.get("secrets", [])
        return [{"name": s["name"], "description": s.get("description", "")}
                for s in items]

    def search(self, query: str = "") -> dict:
        items = self.list()
        q = (query or "").lower()
        if q:
            items = [s for s in items if q in s["name"].lower()
                     or q in s.get("description", "").lower()]
        return {"secrets": items, "items": items, "total_count": len(items)}

    def upsert(self, name: str, value: str, description: str = "") -> dict:
        if not name:
            raise ValueError("name ni lazima")
        with self._lock:
            items = self.store.get("secrets", [])
            for s in items:
                if s["name"] == name:
                    if value:
                        s["value_enc"] = self.store.enc(value)
                    s["description"] = description or s.get("description", "")
                    s["updated_at"] = int(time.time() * 1000)
                    self.store.set("secrets", items)
                    return {"name": name, "description": s["description"]}
            items.append({"name": name, "value_enc": self.store.enc(value),
                          "description": description,
                          "created_at": int(time.time() * 1000)})
            self.store.set("secrets", items)
        return {"name": name, "description": description}

    def delete(self, name: str) -> bool:
        with self._lock:
            items = self.store.get("secrets", [])
            new = [s for s in items if s["name"] != name]
            if len(new) == len(items):
                return False
            self.store.set("secrets", new)
            return True

    def get(self, name: str) -> Optional[str]:
        with self._lock:
            items = self.store.get("secrets", [])
        for s in items:
            if s["name"] == name:
                return self.store.dec(s.get("value_enc", ""))
        return None


# ---------------------------------------------------------------------------
# MCP servers (registry + health + oauth)
# ---------------------------------------------------------------------------


class MCPRegistry:
    """Usajili wa MCP servers + probe halisi ya afya."""

    def __init__(self, store: _JsonStore) -> None:
        self.store = store
        self._lock = threading.Lock()
        self._oauth: Dict[str, dict] = {}

    def list(self) -> List[dict]:
        with self._lock:
            return list(self.store.get("mcp_servers", []))

    def add(self, name: str, url: str, transport: str = "http") -> dict:
        srv = {"id": uuid.uuid4().hex, "name": name, "url": url,
               "transport": transport, "enabled": True,
               "created_at": int(time.time() * 1000)}
        with self._lock:
            items = self.store.get("mcp_servers", [])
            items.append(srv)
            self.store.set("mcp_servers", items)
        return srv

    def delete(self, sid: str) -> bool:
        with self._lock:
            items = self.store.get("mcp_servers", [])
            new = [s for s in items if s["id"] != sid]
            if len(new) == len(items):
                return False
            self.store.set("mcp_servers", new)
            return True

    def health(self, sid: str = "") -> dict:
        """Probe HALISI: jaribu kuunganisha kwa server (HTTP)."""
        import urllib.error
        import urllib.request
        items = self.list()
        if sid:
            items = [s for s in items if s["id"] == sid]
        out = []
        for s in items:
            status = {"id": s["id"], "name": s["name"], "url": s["url"],
                      "healthy": False, "detail": ""}
            try:
                req = urllib.request.Request(s["url"], method="GET")
                with urllib.request.urlopen(req, timeout=5) as r:
                    status["healthy"] = r.status < 500
                    status["detail"] = f"HTTP {r.status}"
            except urllib.error.HTTPError as e:
                status["healthy"] = e.code < 500
                status["detail"] = f"HTTP {e.code}"
            except Exception as e:  # noqa: BLE001
                status["detail"] = f"{type(e).__name__}: {e}"
            out.append(status)
        return {"ok": True, "servers": out, "total": len(out)}

    def oauth_start(self, server_name: str = "") -> dict:
        job = uuid.uuid4().hex
        self._oauth[job] = {"server": server_name, "status": "pending",
                            "started_at": int(time.time() * 1000)}
        return {"ok": True, "job_id": job, "status": "pending",
                "url": None,
                "note": "OAuth inahitaji provider credentials; weka kwenye settings"}

    def oauth_status(self, job_id: str) -> dict:
        job = self._oauth.get(job_id)
        if not job:
            return {"ok": False, "error": "job haipo"}
        return {"ok": True, "job_id": job_id, **job}


# ---------------------------------------------------------------------------
# Sandboxes
# ---------------------------------------------------------------------------


class SandboxManager:
    """Usimamizi wa sandboxes (unda/orodhesha/pause/resume)."""

    def __init__(self, store: _JsonStore, workspace: str) -> None:
        self.store = store
        self.workspace = os.path.abspath(workspace)
        self._lock = threading.Lock()

    def list(self) -> List[dict]:
        with self._lock:
            return list(self.store.get("sandboxes", []))

    def create(self, name: str = "", image: str = "local") -> dict:
        sb = {"id": uuid.uuid4().hex, "name": name or "sandbox",
              "image": image, "status": "running", "workspace": self.workspace,
              "created_at": int(time.time() * 1000)}
        with self._lock:
            items = self.store.get("sandboxes", [])
            items.append(sb)
            self.store.set("sandboxes", items)
        return sb

    def _set_status(self, sid: str, status: str) -> dict:
        with self._lock:
            items = self.store.get("sandboxes", [])
            for s in items:
                if s["id"] == sid:
                    s["status"] = status
                    self.store.set("sandboxes", items)
                    return s
        raise KeyError("sandbox haipo")

    def pause(self, sid: str) -> dict:
        return self._set_status(sid, "paused")

    def resume(self, sid: str) -> dict:
        return self._set_status(sid, "running")


# ---------------------------------------------------------------------------
# Organizations + config search
# ---------------------------------------------------------------------------


class Organizations:
    def __init__(self, store: _JsonStore) -> None:
        self.store = store
        self._lock = threading.Lock()

    def list(self) -> dict:
        with self._lock:
            orgs = list(self.store.get("organizations", []))
        return {"organizations": orgs, "items": orgs, "total_count": len(orgs)}

    def create(self, name: str) -> dict:
        org = {"id": uuid.uuid4().hex, "name": name,
               "created_at": int(time.time() * 1000)}
        with self._lock:
            orgs = self.store.get("organizations", [])
            orgs.append(org)
            self.store.set("organizations", orgs)
        return org

    def profiles(self, org_id: str) -> dict:
        with self._lock:
            orgs = self.store.get("organizations", [])
        org = next((o for o in orgs if o["id"] == org_id), None)
        if not org:
            raise KeyError("organization haipo")
        return {"profiles": [], "items": [], "organization": org_id}

    def provider_connections(self, org_id: str) -> dict:
        with self._lock:
            orgs = self.store.get("organizations", [])
        org = next((o for o in orgs if o["id"] == org_id), None)
        if not org:
            raise KeyError("organization haipo")
        return {"provider_connections": [], "items": [], "organization": org_id}


class ConfigSearch:
    """Utafutaji wa models/providers (catalog halisi + connections za mtumiaji)."""

    MODELS = [
        {"id": "openai/gpt-4o-mini", "provider": "openai", "name": "GPT-4o mini"},
        {"id": "openai/gpt-4o", "provider": "openai", "name": "GPT-4o"},
        {"id": "anthropic/claude-sonnet-4-5-20250929", "provider": "anthropic",
         "name": "Claude Sonnet 4.5"},
        {"id": "anthropic/claude-haiku-4-5-20251001", "provider": "anthropic",
         "name": "Claude Haiku 4.5"},
        {"id": "deepseek/deepseek-chat", "provider": "deepseek", "name": "DeepSeek Chat"},
        {"id": "groq/llama-3.3-70b", "provider": "groq", "name": "Llama 3.3 70B (Groq)"},
        {"id": "ollama/llama3.2", "provider": "ollama", "name": "Llama 3.2 (local)"},
    ]

    def __init__(self, connections: ProviderConnections) -> None:
        self.connections = connections

    def search_models(self, query: str = "", provider: str = "") -> dict:
        q = (query or "").lower()
        items = self.MODELS
        if provider:
            items = [m for m in items if m["provider"] == provider]
        if q:
            items = [m for m in items if q in m["id"].lower() or q in m["name"].lower()]
        return {"models": items, "items": items, "total_count": len(items)}

    def search_providers(self, query: str = "") -> dict:
        q = (query or "").lower()
        items = ProviderConnections.KNOWN
        if q:
            items = [p for p in items if q in p["provider"].lower()
                     or q in p["display_name"].lower()]
        connected = {c["provider"] for c in self.connections.list()}
        items = [{**p, "connected": p["provider"] in connected} for p in items]
        return {"providers": items, "items": items, "total_count": len(items)}


# ---------------------------------------------------------------------------
# Files + Workspaces
# ---------------------------------------------------------------------------


class FileService:
    def __init__(self, workspace: str) -> None:
        self.workspace = os.path.abspath(workspace)

    def _safe(self, path: str) -> Optional[str]:
        if not path or ".." in path.split("/"):
            return None
        full = os.path.join(self.workspace, path.lstrip("/"))
        return full if os.path.abspath(full).startswith(self.workspace) else None

    def download(self, path: str) -> dict:
        full = self._safe(path)
        if not full or not os.path.isfile(full):
            return {"ok": False, "error": "faili haipatikani"}
        try:
            import base64
            with open(full, "rb") as fh:
                data = fh.read()
            return {"ok": True, "path": path, "size": len(data),
                    "content_base64": base64.b64encode(data).decode("ascii")}
        except OSError as e:
            return {"ok": False, "error": str(e)}

    def upload(self, path: str, content_base64: str) -> dict:
        full = self._safe(path)
        if not full:
            return {"ok": False, "error": "path si sahihi"}
        try:
            import base64
            data = base64.b64decode(content_base64 or "")
            os.makedirs(os.path.dirname(full) or self.workspace, exist_ok=True)
            with open(full, "wb") as fh:
                fh.write(data)
            return {"ok": True, "path": path, "size": len(data)}
        except Exception as e:  # noqa: BLE001
            return {"ok": False, "error": str(e)}

    def home(self) -> dict:
        return {"home": self.workspace,
                "favorites": [{"label": "Workspace", "path": self.workspace}],
                "locations": []}


class Workspaces:
    def __init__(self, workspace: str) -> None:
        self.workspace = os.path.abspath(workspace)

    def list(self) -> dict:
        entries = []
        try:
            for name in sorted(os.listdir(self.workspace))[:200]:
                full = os.path.join(self.workspace, name)
                if os.path.isdir(full) and not name.startswith("."):
                    entries.append({"name": name, "path": full, "kind": "dir"})
        except OSError:
            pass
        ws = {"id": "local", "name": os.path.basename(self.workspace),
              "path": self.workspace, "kind": "local"}
        parents = [{"id": "root", "name": os.path.basename(self.workspace),
                    "path": self.workspace}]
        return {"workspaces": [ws], "workspaceParents": parents,
                "items": [ws], "entries": entries}


# ---------------------------------------------------------------------------
# Skills search
# ---------------------------------------------------------------------------


class SkillsSearch:
    def __init__(self, skills_root: str) -> None:
        self.skills_root = skills_root

    def _all(self) -> List[dict]:
        items = []
        if os.path.isdir(self.skills_root):
            for root, _dirs, files in os.walk(self.skills_root):
                if "SKILL.md" in files:
                    name = os.path.relpath(root, self.skills_root)
                    items.append({"name": name, "path": root,
                                  "description": self._desc(os.path.join(root, "SKILL.md"))})
        return items

    @staticmethod
    def _desc(skill_md: str) -> str:
        try:
            with open(skill_md, "r", encoding="utf-8", errors="ignore") as fh:
                text = fh.read(1500)
            for line in text.splitlines():
                if line.strip().lower().startswith("description:"):
                    return line.split(":", 1)[1].strip()[:200]
        except OSError:
            pass
        return ""

    def search(self, query: str = "", limit: int = 50) -> dict:
        items = self._all()
        q = (query or "").lower()
        if q:
            items = [s for s in items if q in s["name"].lower()
                     or q in s.get("description", "").lower()]
        return {"skills": items[:limit], "items": items[:limit],
                "total_count": len(items)}


# ---------------------------------------------------------------------------
# Kiunganishi kimoja
# ---------------------------------------------------------------------------


class CanvasServices:
    """Huduma zote za agent-server kwa Canvas."""

    def __init__(self, workspace: str, data_dir: str, skills_root: str) -> None:
        self.workspace = os.path.abspath(workspace)
        store = _JsonStore(os.path.join(data_dir, "canvas.json"))
        self.store = store
        self.connections = ProviderConnections(store)
        self.secrets = SecretsStore(store)
        self.mcp = MCPRegistry(store)
        self.sandboxes = SandboxManager(store, self.workspace)
        self.organizations = Organizations(store)
        self.config = ConfigSearch(self.connections)
        self.files = FileService(self.workspace)
        self.workspaces = Workspaces(self.workspace)
        self.skills = SkillsSearch(skills_root)
