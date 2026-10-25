"""pytrex.editor — kiolesura halisi cha workspace (OpenVSCode Server).

Hii inaunganisha **OpenVSCode Server** (VS Code ya wavuti, MIT) ndani ya
MBILINYI IDE kama kilele cha "Open Workspace". Ni editor HALISI inayoendesha
kwenye mchakato wake; MBILINYI IDE huiweka nyuma ya `/ide/vscode` kwa proxy.

Kanuni ile ile ya framework: **hakuna kubuni**. Binary haipo → `enabled: false`
wazi; mchakato umezinduka → `running: true`; umeshindwa → `running: false`
pamoja na ujumbe. `/api/vscode/status` hurudisha hali hii halisi ili Canvas
iamue iendelee au la (hakuna kitufe kinachoonyesha editor ambayo haipo).
"""

from __future__ import annotations

import os
import shutil
import socket
import subprocess
import threading
import time
from typing import List, Optional

DEFAULT_EDITOR_PORT = 8001
EDITOR_BASE_PATH = "/ide/vscode"

# Sehemu za kawaida za OpenVSCode Server (kama OpenHands inavyosakinisha).
_EDITOR_CANDIDATES: List[str] = [
    "/openhands/.openvscode-server/bin/openvscode-server",
    os.path.expanduser("~/.openvscode-server/bin/openvscode-server"),
    "/opt/openvscode-server/bin/openvscode-server",
    "/usr/local/bin/openvscode-server",
    "/usr/bin/openvscode-server",
]


def find_editor() -> Optional[str]:
    """Tafuta binary halisi ya OpenVSCode Server, au `None`."""
    env = os.environ.get("MTS_EDITOR_BIN")
    if env and os.path.isfile(env) and os.access(env, os.X_OK):
        return env
    for cand in _EDITOR_CANDIDATES:
        if os.path.isfile(cand) and os.access(cand, os.X_OK):
            return cand
    return shutil.which("openvscode-server") or shutil.which("code-server")


class EditorServer:
    """Mchakato halisi wa OpenVSCode Server + hali yake.

    Token ya muunganisho = `session_api_key` ya agent-server, ili kikao kile
    kile kinachofikia Canvas kifike pia editor (na hakuna mtu mwingine).
    """

    def __init__(
        self,
        workspace: str,
        port: int = DEFAULT_EDITOR_PORT,
        base_path: str = EDITOR_BASE_PATH,
        token: str = "",
        binary: Optional[str] = None,
    ) -> None:
        self.workspace = os.path.abspath(workspace)
        self.port = port
        self.base_path = base_path
        self.token = token
        self.binary = binary if binary is not None else find_editor()
        self._proc: Optional[subprocess.Popen] = None
        self._lock = threading.Lock()
        self._last_error = ""
        self._log_path = os.path.join(
            os.environ.get("TMPDIR", "/tmp"), "mts-editor.log"
        )

    # ---------- hali ----------

    @property
    def enabled(self) -> bool:
        """Binary ipo (deployment ina editor)."""
        return bool(self.binary and os.path.isfile(self.binary))

    @property
    def running(self) -> bool:
        return self._proc is not None and self._proc.poll() is None

    def status(self) -> dict:
        if not self.enabled:
            message = "OpenVSCode Server haijasakinishwa"
        elif self.running:
            message = f"inaendesha (port {self.port})"
        elif self._last_error:
            message = self._last_error
        else:
            message = "haijaendesha"
        return {
            "enabled": self.enabled,
            "running": self.running,
            "message": message,
            "port": self.port,
            "base_path": self.base_path,
        }

    # ---------- maisha ----------

    def start(self, wait: float = 25.0) -> dict:
        if not self.enabled:
            return {"status": "error", "message": "editor binary haipo",
                    "enabled": False}
        if self.running:
            return {"status": "ok", "message": "editor tayari inaendesha",
                    "skipped": True}

        data_dir = os.path.join(
            os.environ.get("MTS_EDITOR_DATA_DIR", "/tmp/mts-editor-data")
        )
        cmd = [
            self.binary,
            "--host", "127.0.0.1",
            "--port", str(self.port),
            "--server-base-path", self.base_path,
            "--user-data-dir", data_dir,
            "--without-telemetry",
            self.workspace,
        ]
        if self.token:
            cmd += ["--connection-token", self.token]

        env = dict(os.environ)
        env["VSCODE_TELEMETRY_LEVEL"] = "off"
        logf = open(self._log_path, "ab")
        try:
            self._proc = subprocess.Popen(cmd, env=env, stdout=logf,
                                          stderr=logf, stdin=subprocess.DEVNULL)
        except OSError as e:  # noqa: BLE001
            self._last_error = f"editor imeshindwa kuzinduka: {e}"
            return {"status": "error", "message": self._last_error}

        deadline = time.time() + wait
        while time.time() < deadline:
            if self._proc.poll() is not None:
                self._last_error = "editor ilikwama — angalia mts-editor.log"
                return {"status": "error", "message": self._last_error,
                        "log": self._log_path}
            if self._port_open():
                return {"status": "ok", "message": f"editor inaendesha (port {self.port})",
                        "url": f"{self.base_path}?tkn={self.token}" if self.token else self.base_path}
            time.sleep(0.5)
        self._last_error = "editor haikuanza kwa wakati"
        return {"status": "error", "message": self._last_error}

    def stop(self) -> dict:
        with self._lock:
            if not self.running:
                return {"status": "ok", "message": "editor haijaendesha",
                        "skipped": True}
            assert self._proc is not None
            self._proc.terminate()
            try:
                self._proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self._proc.kill()
            self._proc = None
            return {"status": "ok", "message": "editor imesimamishwa"}

    # ---------- helpers ----------

    def _port_open(self) -> bool:
        import socket
        try:
            with socket.create_connection(("127.0.0.1", self.port), timeout=1.0):
                return True
        except OSError:
            return False

    def url(self, base_url: str) -> Optional[str]:
        """URL ya editor inayotumika kwenye kivinjari (kupitia proxy yetu)."""
        if not self.enabled:
            return None
        base = base_url.rstrip("/")
        url = f"{base}{self.base_path}"
        return f"{url}?tkn={self.token}" if self.token else url


__all__ = ["EditorServer", "find_editor", "DEFAULT_EDITOR_PORT", "EDITOR_BASE_PATH"]
