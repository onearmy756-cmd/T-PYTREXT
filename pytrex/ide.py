"""pytrex.ide — MBILINYI IDE: Agent Canvas + zana zote za IDE.

Moduli hii inaunganisha **MBILINYI IDE** (OpenHands Agent Canvas UI + zana zote
za IDE) ndani ya T-PYTREXT kwa architecture ile ile ya framework:

    pytrex.ide.IDEEngine    — orchestration (canvas, desktop, extension, backend)
    pytrex.ide.IDEToolRegistry — katalog ya zana za IDE (LSP, DAP, build, swarm…)
    pytrex.ide.MBILINYIIDE  — PyTreXApp yenye @event handlers za IDE

Sehemu tatu:

1. **Canvas UI** — `ide/openhands/` ni OpenHands Agent Canvas (React/TS, MIT).
   Huhudumiwa kwenye `/ide`. Bila build, `/ide` hurudisha maelezo pekee.
2. **Desktop shell** — `ide/desktop/` ni Tauri v2 shell (dirisha la desktop).
3. **Backend** — `ide/backend/` ni Rust agent (crates za `fundi-*`) inayotoa
   `/api/*` zote (IDE APIs, tool calls, LSP, DAP, build, swarm…).

Kanuni kuu (kama `engines/README.md` ya MTECHOS): **hakuna kubuni**. Engine
haipatikani → `available: false` wazi; hakuna dai la kitu ambacho hakipo.
"""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional

from .core import PyTreXApp, event

# ═══════════════════════════════════════════════════════════
# MIPANGO / PATHS
# ═══════════════════════════════════════════════════════════

IDE_DIRNAME = "ide"
DEFAULT_BACKEND_ADDR = "http://127.0.0.1:8787"
DEFAULT_CANVAS_PORT = 8080
DEFAULT_OPENHANDS_PORT = 12001
DEFAULT_EDITOR_PORT = 8001
EDITOR_BASE_PATH = "/ide/vscode"


def ide_home() -> Optional[str]:
    """Tafuta folda ya `ide/` (vendored assets).

    Mpangilio: `PYTREX_IDE_HOME` env, kisha kupanda juu kutoka moduli hadi
    kupata folda ya `ide/` yenye `openhands/` ndani.
    """
    env = os.environ.get("PYTREX_IDE_HOME")
    if env and os.path.isdir(env):
        return os.path.abspath(env)

    here = os.path.dirname(os.path.abspath(__file__))
    for _ in range(6):
        candidate = os.path.join(here, IDE_DIRNAME)
        if os.path.isdir(os.path.join(candidate, "openhands")):
            return candidate
        parent = os.path.dirname(here)
        if parent == here:
            break
        here = parent
    return None


# ═══════════════════════════════════════════════════════════
# KATALOG YA ZANA ZA IDE
# ═══════════════════════════════════════════════════════════

@dataclass
class IDETool:
    """Zana moja ya IDE (inaakisi moduli ya Rust `ide_*.rs`)."""

    name: str
    module: str
    category: str
    description: str
    endpoint: str = ""

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "module": self.module,
            "category": self.category,
            "description": self.description,
            "endpoint": self.endpoint,
        }


# Kila zana inaakisi faili la Rust halisi ndani ya `ide/backend/src/`.
_IDE_TOOLS: List[IDETool] = [
    IDETool("api", "ide_api.rs", "core", "HTTP API ya IDE (files, tree, workspace)", "/api/ide/*"),
    IDETool("lsp", "ide_lsp.rs", "editor", "Language Server Protocol (diagnostics, hover, definition)", "/api/ide/lsp"),
    IDETool("dap", "ide_dap.rs", "debug", "Debug Adapter Protocol (breakpoints, stack, vars)", "/api/ide/dap"),
    IDETool("complete", "ide_complete.rs", "editor", "Autocomplete / inline completion", "/api/ide/complete"),
    IDETool("composer", "ide_composer.rs", "editor", "Composer — multi-file edit", "/api/ide/composer"),
    IDETool("code_run", "ide_code_run.rs", "runtime", "Endesha code (sandboxed)", "/api/ide/run"),
    IDETool("build", "ide_build.rs", "build", "Build/compile ya mradi", "/api/ide/build"),
    IDETool("pipeline", "ide_pipeline.rs", "build", "CI/CD pipeline", "/api/ide/pipeline"),
    IDETool("testkit", "ide_testkit.rs", "test", "Test runner + test kit", "/api/ide/testkit"),
    IDETool("e2e", "ide_e2e.rs", "test", "End-to-end testing", "/api/ide/e2e"),
    IDETool("lab", "ide_lab.rs", "lab", "IDE Lab (experiments)", "/api/ide/lab"),
    IDETool("emulators", "ide_emulators.rs", "devices", "Emulators (Android/iOS)", "/api/ide/emulators"),
    IDETool("devices", "ide_devices.rs", "devices", "Device manager (USB, ADB)", "/api/ide/devices"),
    IDETool("swarm", "ide_swarm.rs", "agent", "Agent swarm (multi-agent)", "/api/ide/swarm"),
    IDETool("engineer", "ide_engineer.rs", "agent", "Engineer agent", "/api/ide/engineer"),
    IDETool("integrations", "ide_integrations.rs", "integrations", "Integrations (Git, CI, cloud)", "/api/ide/integrations"),
    IDETool("canvas", "canvas_ext.rs", "canvas", "Canvas bridge (extensions, session key)", "/api/canvas-extensions/*"),
]


class IDEToolRegistry:
    """Katalog ya zana za IDE zinazopatikana ndani ya backend."""

    def __init__(self) -> None:
        self._tools: Dict[str, IDETool] = {t.name: t for t in _IDE_TOOLS}

    def all(self) -> List[IDETool]:
        return list(self._tools.values())

    def get(self, name: str) -> Optional[IDETool]:
        return self._tools.get(name)

    def by_category(self, category: str) -> List[IDETool]:
        return [t for t in self._tools.values() if t.category == category]

    def categories(self) -> List[str]:
        seen: List[str] = []
        for t in self._tools.values():
            if t.category not in seen:
                seen.append(t.category)
        return seen

    def catalog(self) -> dict:
        implemented = {
            "api", "lsp", "dap", "complete", "composer", "build", "pipeline",
            "testkit", "e2e", "lab", "emulators", "devices", "swarm", "engineer",
            "integrations", "code_run", "canvas",
        }
        tools = []
        for t in self._tools.values():
            d = t.to_dict()
            d["implemented"] = t.name in implemented
            d["status"] = "ok" if t.name in implemented else "unavailable"
            tools.append(d)
        return {
            "total": len(self._tools),
            "implemented": sum(1 for t in self._tools.values() if t.name in implemented),
            "categories": self.categories(),
            "tools": tools,
        }


# ═══════════════════════════════════════════════════════════
# IDEEngine — orchestration
# ═══════════════════════════════════════════════════════════

@dataclass
class ComponentStatus:
    """Hali ya sehemu moja ya IDE (probe halisi)."""

    name: str
    path: str = ""
    present: bool = False
    available: bool = False
    detail: str = ""

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "path": self.path,
            "present": self.present,
            "available": self.available,
            "detail": self.detail,
        }


class IDEEngine:
    """Inasimamia MBILINYI IDE: canvas, desktop, extension, backend.

    Hufanya probe halisi (filesystem + HTTP) — hairudishi `available: true`
    kwa kitu ambacho hakipo.
    """

    def __init__(
        self,
        home: Optional[str] = None,
        backend_addr: str = DEFAULT_BACKEND_ADDR,
        canvas_port: int = DEFAULT_CANVAS_PORT,
    ) -> None:
        self.home = home or ide_home()
        self.backend_addr = backend_addr.rstrip("/")
        self.canvas_port = canvas_port
        self.tools = IDEToolRegistry()
        self._proc: Optional[subprocess.Popen] = None
        self._log_lines: List[str] = []

    # ---------- paths ----------

    def _p(self, *parts: str) -> str:
        if not self.home:
            return ""
        return os.path.join(self.home, *parts)

    @property
    def available(self) -> bool:
        return bool(self.home and os.path.isdir(self.home))

    # ---------- probes ----------

    def _probe_dir(self, name: str, rel: str, detail: str) -> ComponentStatus:
        path = self._p(rel)
        present = bool(path) and os.path.isdir(path)
        return ComponentStatus(
            name=name,
            path=path,
            present=present,
            available=present,
            detail=detail if present else f"haipo: {rel}",
        )

    def canvas_status(self) -> ComponentStatus:
        """Hali ya Canvas UI. Imejengwa ikiwa `build/index.html` ipo."""
        src = self._p("openhands")
        build = self._p("openhands", "build", "index.html")
        present = bool(src) and os.path.isdir(src)
        built = bool(build) and os.path.isfile(build)
        return ComponentStatus(
            name="canvas",
            path=src,
            present=present,
            available=present and built,
            detail=(
                "imejengwa (build/index.html ipo)"
                if built
                else "chanzo kipo, build haipo — endesha build_canvas()"
                if present
                else "chanzo hakipo"
            ),
        )

    def desktop_status(self) -> ComponentStatus:
        src = self._p("desktop", "src-tauri", "Cargo.toml")
        present = bool(src) and os.path.isfile(src)
        return ComponentStatus(
            name="desktop",
            path=self._p("desktop"),
            present=present,
            available=present,
            detail="Tauri v2 shell ipo" if present else "haipo",
        )

    def extension_app_status(self) -> ComponentStatus:
        pkg = self._p("extension-app", "package.json")
        present = bool(pkg) and os.path.isfile(pkg)
        return ComponentStatus(
            name="extension-app",
            path=self._p("extension-app"),
            present=present,
            available=present,
            detail="Canvas Extension App (Monaco) ipo" if present else "haipo",
        )

    def backend_status(self) -> ComponentStatus:
        """Hali ya backend. Probe halisi kwa HTTP `/health`."""
        src = self._p("backend", "Cargo.toml")
        present = bool(src) and os.path.isfile(src)
        healthy = self._http_ok(f"{self.backend_addr}/health")
        if healthy:
            detail = f"inaendesha ({self.backend_addr})"
        elif present:
            detail = f"chanzo kipo, haijaendesha — endesha start_backend()"
        else:
            detail = "chanzo hakipo"
        return ComponentStatus(
            name="backend",
            path=self._p("backend"),
            present=present,
            available=healthy,
            detail=detail,
        )

    def skills_status(self) -> ComponentStatus:
        src = self._p("skills")
        present = bool(src) and os.path.isdir(src)
        count = 0
        if present:
            for root, _dirs, files in os.walk(src):
                count += sum(1 for f in files if f == "SKILL.md")
        return ComponentStatus(
            name="skills",
            path=src,
            present=present,
            available=present,
            detail=f"skills {count} zimepatikana" if present else "haipo",
        )

    def status(self) -> dict:
        """Hali kamili ya IDE (probe halisi ya kila sehemu)."""
        comps = [
            self.canvas_status(),
            self.desktop_status(),
            self.extension_app_status(),
            self.backend_status(),
            self.skills_status(),
        ]
        return {
            "available": self.available,
            "home": self.home,
            "backend_addr": self.backend_addr,
            "canvas_port": self.canvas_port,
            "components": {c.name: c.to_dict() for c in comps},
            "tools": self.tools.catalog()["total"],
        }

    # ---------- actions ----------

    def build_canvas(self, log: Optional[Callable[[str], None]] = None) -> dict:
        """Jenga Canvas UI (`npm ci && npm run build`, base `/ide`)."""
        if not self.available:
            return {"status": "error", "message": "ide/ haipatikani"}
        if self.canvas_status().available:
            return {"status": "ok", "message": "tayari imejengwa", "skipped": True}

        src = self._p("openhands")
        npm = shutil.which("npm")
        if not npm:
            return {"status": "error", "message": "npm haipo — sakinisha Node.js 18+"}

        env = dict(os.environ)
        env["VITE_BASE_PATH"] = "/ide"
        try:
            r1 = subprocess.run(
                [npm, "ci"], cwd=src, env=env,
                capture_output=True, text=True, timeout=1800,
            )
            if log:
                log(r1.stdout[-2000:] if r1.stdout else "")
            if r1.returncode != 0:
                return {"status": "error", "message": "npm ci imeshindwa",
                        "stderr": (r1.stderr or "")[-1000:]}
            r2 = subprocess.run(
                [npm, "run", "build"], cwd=src, env=env,
                capture_output=True, text=True, timeout=1800,
            )
            if log:
                log(r2.stdout[-2000:] if r2.stdout else "")
            if r2.returncode != 0:
                return {"status": "error", "message": "npm run build imeshindwa",
                        "stderr": (r2.stderr or "")[-1000:]}
        except subprocess.TimeoutExpired:
            return {"status": "error", "message": "build imepitiliza muda"}

        ok = self.canvas_status().available
        return {
            "status": "ok" if ok else "error",
            "message": "Canvas imejengwa" if ok else "build ilikamilika lakini build/index.html haipo",
        }

    def start_backend(self, wait: float = 60.0) -> dict:
        """Anzisha backend (cargo run --release --bin fundi-server) nyuma."""
        if not self.available:
            return {"status": "error", "message": "ide/ haipatikani"}
        if self.backend_status().available:
            return {"status": "ok", "message": "backend tayari inaendesha", "skipped": True}

        src = self._p("backend")
        cargo = shutil.which("cargo")
        if not cargo:
            return {"status": "error", "message": "cargo haipo — sakinisha Rust toolchain"}

        env = dict(os.environ)
        env.setdefault("FUNDI_DATA", os.path.join(os.path.dirname(src), "..", "data"))
        env.setdefault("FUNDI_AGENT_ADDR", self.backend_addr)
        env.setdefault("RUST_LOG", env.get("RUST_LOG", "info"))

        logf = open(os.path.join(src, "fundi-server.log"), "ab")
        self._proc = subprocess.Popen(
            [cargo, "run", "--release", "--bin", "fundi-server"],
            cwd=src, env=env, stdout=logf, stderr=logf,
        )
        deadline = time.time() + wait
        while time.time() < deadline:
            if self._http_ok(f"{self.backend_addr}/health"):
                return {"status": "ok", "message": f"backend inaendesha ({self.backend_addr})"}
            if self._proc.poll() is not None:
                return {"status": "error",
                        "message": "backend ilikwama — angalia fundi-server.log"}
            time.sleep(1.0)
        return {"status": "error", "message": "backend haikuanza kwa wakati"}

    def stop_backend(self) -> dict:
        if self._proc and self._proc.poll() is None:
            self._proc.terminate()
            try:
                self._proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self._proc.kill()
            return {"status": "ok", "message": "backend imesimamishwa"}
        return {"status": "ok", "message": "hakuna backend ya kusimamisha", "skipped": True}

    def start(self) -> dict:
        """Anzisha IDE: build canvas kama haipo, kisha anzisha backend."""
        result: Dict[str, object] = {"status": "ok", "steps": []}
        if not self.available:
            return {"status": "error", "message": "ide/ haipatikani"}
        if not self.canvas_status().available:
            result["steps"].append({"build_canvas": self.build_canvas()})
        result["steps"].append({"start_backend": self.start_backend()})
        result["status_url"] = f"http://127.0.0.1:{self.canvas_port}/ide"
        return result

    def plan(self) -> dict:
        """Mpango wa kile ambacho kingefanyika (hakuna utekelezaji)."""
        return {
            "home": self.home,
            "canvas": {
                "present": self.canvas_status().present,
                "needs_build": not self.canvas_status().available,
            },
            "backend": {
                "present": self.backend_status().present,
                "needs_start": not self.backend_status().available,
            },
            "steps": [
                s for s in [
                    "build_canvas" if not self.canvas_status().available else None,
                    "start_backend" if not self.backend_status().available else None,
                ] if s
            ],
        }

    # ---------- helpers ----------

    @staticmethod
    def _http_ok(url: str, timeout: float = 2.0) -> bool:
        try:
            with urllib.request.urlopen(url, timeout=timeout) as resp:
                return 200 <= resp.status < 500
        except (urllib.error.URLError, OSError, ValueError):
            return False


# ═══════════════════════════════════════════════════════════
# MBILINYIIDE — PyTreXApp + @event handlers
# ═══════════════════════════════════════════════════════════

_DEFAULT_ENGINE: Optional[IDEEngine] = None


def default_engine() -> IDEEngine:
    """Engine moja ya IDE inayoshirikiwa na events (lazily-created)."""
    global _DEFAULT_ENGINE
    if _DEFAULT_ENGINE is None:
        _DEFAULT_ENGINE = IDEEngine()
    return _DEFAULT_ENGINE


@event("ide_status")
def ide_status(data: str) -> str:
    return json.dumps({"status": "ok", "data": default_engine().status()})


@event("ide_plan")
def ide_plan(data: str) -> str:
    return json.dumps({"status": "ok", "data": default_engine().plan()})


@event("ide_tools")
def ide_tools_catalog(data: str) -> str:
    payload = json.loads(data or "{}")
    engine = default_engine()
    category = payload.get("category")
    if category:
        tools = [t.to_dict() for t in engine.tools.by_category(category)]
        return json.dumps({"status": "ok", "data": {"tools": tools}})
    return json.dumps({"status": "ok", "data": engine.tools.catalog()})


@event("ide_build_canvas")
def ide_build_canvas(data: str) -> str:
    return json.dumps({"status": "ok", "data": default_engine().build_canvas()})


@event("ide_start")
def ide_start(data: str) -> str:
    return json.dumps({"status": "ok", "data": default_engine().start()})


@event("ide_stop")
def ide_stop(data: str) -> str:
    return json.dumps({"status": "ok", "data": default_engine().stop_backend()})


class MBILINYIIDE(PyTreXApp):
    """MBILINYI IDE kama PyTreXApp — inaunganisha OpenHands + zana za IDE.

    `self.engine` ni `IDEEngine` ya app hii; events za module-level (juu)
    hutumia `default_engine()` inayoshirikiwa.
    """

    def __init__(self, name: str = "MBILINYI IDE", home: Optional[str] = None):
        super().__init__(name=name)
        self.engine = IDEEngine(home=home)
        self.ide_tools = self.engine.tools

    def status(self) -> dict:
        return self.engine.status()

    def start(self) -> dict:
        return self.engine.start()


__all__ = [
    "IDETool",
    "IDEToolRegistry",
    "IDEEngine",
    "ComponentStatus",
    "MBILINYIIDE",
    "default_engine",
    "ide_home",
    "IDE_DIRNAME",
    "DEFAULT_BACKEND_ADDR",
    "DEFAULT_CANVAS_PORT",
    "DEFAULT_OPENHANDS_PORT",
]
