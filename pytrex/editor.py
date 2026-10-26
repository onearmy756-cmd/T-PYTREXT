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

        # Editor ya kikao kilichopita inaweza kuwa bado imeshikilia port yetu
        # (token yake ni ya zamani). Bila kuisafisha, mchakato wetu hushindwa
        # ku-bind, `_port_open()` hurudisha true kwa sababu MTU MWINGINE
        # anasikiliza, na kila ombi hupata 403 kimyakimya.
        self._free_port()

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
            # Sio kutosha port kufunguka — LAZIMA iwe mchakato wetu. Vinginevyo
            # tunatangaza mafanikio huku editor ya zamani ikikataa token yetu.
            if self._port_owned_by_us():
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

    def _port_owned_by_us(self) -> bool:
        """Port inasikilizwa NA mchakato wetu (sio mchakato mwingine).

        `_port_open()` pekee haitoshi: editor ya kikao kilichopita inaweza
        kushikilia port hiyo na token tofauti, hivyo maombi yetu yote yanapata
        403 wakati `status()` inasema `running: true`.
        """
        if not self.running or not self._port_open():
            return False
        assert self._proc is not None
        return self._listening_pids(self.port) <= self._proc_tree_pids()

    def _proc_tree_pids(self) -> set:
        """PID za mchakato wetu pamoja na watoto wake (node, extension host…)."""
        assert self._proc is not None
        root = self._proc.pid
        pids = {root}
        try:
            entries = os.listdir("/proc")
        except OSError:
            return pids
        children = {}
        for name in entries:
            if not name.isdigit():
                continue
            try:
                with open(f"/proc/{name}/stat", "rb") as f:
                    fields = f.read().rsplit(b")", 1)[1].split()
                children.setdefault(int(fields[1]), []).append(int(name))
            except (OSError, IndexError, ValueError):
                continue
        stack = [root]
        while stack:
            for child in children.get(stack.pop(), []):
                if child not in pids:
                    pids.add(child)
                    stack.append(child)
        return pids

    def _listening_pids(self, port: int) -> set:
        """PID zinazosikiliza `port` kwenye TCP (inode → /proc/<pid>/fd)."""
        inodes = set()
        for path in ("/proc/net/tcp", "/proc/net/tcp6"):
            try:
                with open(path) as f:
                    next(f, None)
                    for line in f:
                        parts = line.split()
                        if len(parts) < 10 or parts[3] != "0A":  # 0A = LISTEN
                            continue
                        if int(parts[1].split(":")[1], 16) == port:
                            inodes.add(parts[9])
            except (OSError, ValueError, IndexError):
                continue
        if not inodes:
            return set()
        pids = set()
        try:
            entries = os.listdir("/proc")
        except OSError:
            return pids
        for name in entries:
            if not name.isdigit():
                continue
            fd_dir = f"/proc/{name}/fd"
            try:
                fds = os.listdir(fd_dir)
            except OSError:
                continue
            for fd in fds:
                try:
                    target = os.readlink(os.path.join(fd_dir, fd))
                except OSError:
                    continue
                if target.startswith("socket:[") and target[8:-1] in inodes:
                    pids.add(int(name))
                    break
        return pids

    def _free_port(self) -> None:
        """Zima msikilizaji yeyote wa zamani kwenye port yetu (na watoto wake)."""
        for pid in self._listening_pids(self.port):
            if pid == os.getpid():
                continue
            try:
                os.kill(pid, 15)
            except OSError:
                continue
        deadline = time.time() + 8.0
        while time.time() < deadline and self._port_open():
            time.sleep(0.25)
        if self._port_open():
            for pid in self._listening_pids(self.port):
                if pid != os.getpid():
                    try:
                        os.kill(pid, 9)
                    except OSError:
                        pass
            time.sleep(0.5)

    def url(self, base_url: str) -> Optional[str]:
        """URL ya editor inayotumika kwenye kivinjari (kupitia proxy yetu)."""
        if not self.enabled:
            return None
        base = base_url.rstrip("/")
        url = f"{base}{self.base_path}"
        return f"{url}?tkn={self.token}" if self.token else url


__all__ = ["EditorServer", "find_editor", "DEFAULT_EDITOR_PORT", "EDITOR_BASE_PATH"]
