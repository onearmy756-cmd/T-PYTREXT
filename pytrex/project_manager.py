"""
PyTreXT Project Manager — Simamia na Endesha Miradi Yako Yote
================================================================
Inawezesha developer kusimamia miradi yote aliyotengeneza kwa PyTreXT:
- Auto-discover miradi kwenye kompyuta
- Run miradi yoyote kwa amri moja
- Test miradi yote kwa pamoja
- Build miradi kwa production
- Dashboard ya kusimamia miradi
- Watch mode kwa development

Usage:
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    pm.scan()                        # Discover miradi yote
    pm.list_projects()               # Orodhesha miradi
    pm.run("SmartBank")              # Endesha mradi
    pm.test_all()                    # Pima miradi yote
    pm.serve_dashboard()             # Fungua dashboard kwenye browser
"""

import json
import os
import re
import subprocess
import sys
import threading
import time
import webbrowser
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from concurrent.futures import ThreadPoolExecutor, as_completed


@dataclass
class Project:
    """Taarifa za mradi wa PyTreXT"""
    name: str
    path: str
    main_file: str = "main.py"
    has_frontend: bool = False
    has_tauri_conf: bool = False
    has_blockchain: bool = False
    has_database: bool = False
    has_events: List[str] = field(default_factory=list)
    has_elixir: bool = False
    description: str = ""
    last_modified: float = 0.0
    size_kb: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "path": self.path,
            "main_file": self.main_file,
            "has_frontend": self.has_frontend,
            "has_tauri_conf": self.has_tauri_conf,
            "has_blockchain": self.has_blockchain,
            "has_database": self.has_database,
            "events": self.has_events,
            "has_elixir": self.has_elixir,
            "description": self.description,
            "last_modified": self.last_modified,
            "size_kb": round(self.size_kb, 1),
        }

    @property
    def status_emoji(self) -> str:
        """Return status indicators"""
        parts = []
        if self.has_blockchain:
            parts.append("🔗")
        if self.has_database:
            parts.append("🗄️")
        if self.has_frontend:
            parts.append("🖥️")
        if self.has_elixir:
            parts.append("🌐")
        if self.has_events:
            parts.append(f"⚡({len(self.has_events)})")
        return " ".join(parts) if parts else "📦"


class ProjectManager:
    """
    PyTreXT Project Manager — simamia miradi yako yote.

    Features:
    - Auto-scan folders kutafuta miradi ya PyTreXT
    - Run, test, build miradi yoyote
    - Dashboard HTML kwa ajili ya usimamizi
    - Parallel execution kwa speed
    """

    SCAN_DIRS = [
        os.path.expanduser("~"),
        os.path.expanduser("~/Desktop"),
        os.path.expanduser("~/Documents"),
        os.path.expanduser("~/Projects"),
        os.path.expanduser("~/Dev"),
        "C:\\",
        "D:\\",
        "E:\\",
    ]

    PYREX_SIGNATURE = re.compile(
        r'(from pytrex|import pytrex|from pytrex\.core import|PyTreXApp)',
        re.IGNORECASE
    )

    def __init__(self, root_dirs: Optional[List[str]] = None):
        self._projects: Dict[str, Project] = {}
        self._scanning = False
        self._root_dirs = root_dirs or self._default_scan_dirs()
        self._dashboard_port = 8080
        self._dashboard_running = False

    def _default_scan_dirs(self) -> List[str]:
        """Get default scan directories that exist"""
        valid = []
        for d in self.SCAN_DIRS:
            if os.path.exists(d):
                valid.append(d)
        # Always scan current directory
        valid.insert(0, os.getcwd())
        # Scan common project locations
        home = os.path.expanduser("~")
        for sub in ["projects", "Projects", "dev", "Dev", "code", "Code", "src", "repos"]:
            path = os.path.join(home, sub)
            if os.path.exists(path) and path not in valid:
                valid.append(path)
        return valid

    # ─── SCANNING ─────────────────────────────────────────────

    def scan(self, max_depth: int = 4, show_progress: bool = True) -> List[Project]:
        """
        Scan directories kutafuta miradi yote ya PyTreXT.

        Args:
            max_depth: Kina cha kuscan subdirectories
            show_progress: Onyesha progress wakati wa scanning

        Returns:
            List ya miradi iliyogunduliwa
        """
        self._projects = {}
        self._scanning = True

        if show_progress:
            print(f"\n{'='*55}")
            print(f"  🔍 PyTreXT Scanner — Inatafuta miradi...")
            print(f"  📂 Scanning {len(self._root_dirs)} directories...")
            print(f"{'='*55}\n")

        found = []

        # Scan each root directory
        for root in self._root_dirs:
            if not os.path.exists(root):
                continue

            try:
                discovered = self._scan_directory(root, max_depth=max_depth)
                found.extend(discovered)

                if show_progress and discovered:
                    for proj in discovered:
                        print(f"  ✅ {proj.name:.<30} {proj.status_emoji}  ({proj.path})")

            except PermissionError:
                continue
            except Exception as e:
                if show_progress:
                    print(f"  ⚠️  Skipped {root}: {e}")

        # Store discovered projects
        for proj in found:
            self._projects[proj.name.lower()] = proj

        self._scanning = False

        if show_progress:
            print(f"\n  {'='*55}")
            print(f"  📊 Total: {len(found)} PyTreXT project(s) found")
            print(f"  📂 Scanned: {len(self._root_dirs)} directories")
            print(f"  {'='*55}\n")

        return found

    def _scan_directory(self, root: str, max_depth: int, current_depth: int = 0) -> List[Project]:
        """Scan a single directory recursively"""
        found = []
        if current_depth > max_depth:
            return found

        try:
            entries = list(os.scandir(root))
        except (PermissionError, OSError):
            return found

        # Check if this directory itself is a PyTreXT project
        main_file = os.path.join(root, "main.py")
        if os.path.isfile(main_file):
            if self._is_pytrex_project(main_file):
                proj = self._analyze_project(root, main_file)
                if proj:
                    found.append(proj)
                    # Don't recurse into project directories
                    return found

        # Recurse into subdirectories
        for entry in entries:
            if entry.is_dir() and not entry.name.startswith(".") and not entry.name.startswith("_"):
                if entry.name not in ("node_modules", "target", "__pycache__",
                                       ".git", ".venv", "venv", "env",
                                       "dist", "build", "egg-info"):
                    try:
                        sub_found = self._scan_directory(
                            entry.path, max_depth, current_depth + 1
                        )
                        found.extend(sub_found)
                    except (PermissionError, OSError):
                        continue

        return found

    def _is_pytrex_project(self, main_file: str) -> bool:
        """Check if a main.py is a PyTreXT project"""
        try:
            with open(main_file, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read(5000)  # Read first 5KB
                return bool(self.PYREX_SIGNATURE.search(content))
        except Exception:
            return False

    def _analyze_project(self, root: str, main_file: str) -> Optional[Project]:
        """Analyze a PyTreXT project directory"""
        try:
            name = os.path.basename(root)
            stat = os.stat(main_file)

            # Read main.py for analysis
            with open(main_file, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()

            # Detect features
            has_blockchain = "blockchain" in content.lower() or "BlockchainBridge" in content
            has_database = "kuandaa_database" in content or "sqlx" in content.lower()
            has_frontend = os.path.exists(os.path.join(root, "frontend", "index.html"))
            has_tauri = os.path.exists(os.path.join(root, "tauri.conf.json"))
            has_elixir = os.path.exists(os.path.join(root, "pytrex_engine"))

            # Extract events
            events = re.findall(r'@event\(["\']([^"\']+)["\']\)', content)

            # Extract description (first docstring or comment)
            desc = ""
            doc_match = re.search(r'"""(.*?)"""', content, re.DOTALL)
            if doc_match:
                desc = doc_match.group(1).strip().split("\n")[0][:100]

            # Calculate project size
            total_size = 0
            for dirpath, _, filenames in os.walk(root):
                for fn in filenames:
                    fp = os.path.join(dirpath, fn)
                    try:
                        if os.path.isfile(fp):
                            total_size += os.path.getsize(fp)
                    except OSError:
                        pass

            return Project(
                name=name,
                path=root,
                main_file="main.py",
                has_frontend=has_frontend,
                has_tauri_conf=has_tauri,
                has_blockchain=has_blockchain,
                has_database=has_database,
                has_events=events,
                has_elixir=has_elixir,
                description=desc,
                last_modified=stat.st_mtime,
                size_kb=total_size / 1024.0,
            )
        except Exception:
            return None

    # ─── LISTING ──────────────────────────────────────────────

    def list_projects(self, refresh: bool = False) -> List[Project]:
        """Orodhesha miradi yote iliyogunduliwa"""
        if refresh or not self._projects:
            self.scan(show_progress=False)
        return sorted(self._projects.values(), key=lambda p: p.name.lower())

    def get_project(self, name: str) -> Optional[Project]:
        """Pata mradi kwa jina"""
        if not self._projects:
            self.scan(show_progress=False)
        return self._projects.get(name.lower())

    def projects_table(self) -> str:
        """Generate table ya miradi yote"""
        projects = self.list_projects()
        if not projects:
            return "Hakuna miradi ya PyTreXT iliyogunduliwa."

        lines = []
        lines.append(f"\n{'='*80}")
        lines.append(f"  📦 PyTreXT Projects ({len(projects)} found)")
        lines.append(f"{'='*80}")
        lines.append(f"  {'#':<4} {'Name':<25} {'Features':<25} {'Path'}")
        lines.append(f"  {'-'*4} {'-'*25} {'-'*25} {'-'*40}")

        for i, proj in enumerate(projects, 1):
            features = []
            if proj.has_blockchain: features.append("🔗BC")
            if proj.has_database: features.append("🗄️DB")
            if proj.has_frontend: features.append("🖥️UI")
            if proj.has_elixir: features.append("🌐EX")
            if proj.has_events: features.append(f"⚡x{len(proj.has_events)}")
            feat_str = " ".join(features) if features else "📦"

            path_short = proj.path
            if len(path_short) > 40:
                path_short = "..." + path_short[-37:]

            lines.append(f"  {i:<4} {proj.name[:25]:<25} {feat_str:<25} {path_short}")

        lines.append(f"  {'='*80}")
        return "\n".join(lines)

    # ─── RUNNING ──────────────────────────────────────────────

    def run(self, project_name: str, dev_mode: bool = True) -> bool:
        """
        Endesha mradi wa PyTreXT.

        Args:
            project_name: Jina la mradi
            dev_mode: Tumia dev mode (True) au production (False)

        Returns:
            True kama imefanikiwa
        """
        proj = self.get_project(project_name)
        if not proj:
            print(f"❌ Mradi '{project_name}' haujagunduliwa.")
            print(f"   Tumia 'pytrex scan' kwanza kutafuta miradi.")
            return False

        main_path = os.path.join(proj.path, proj.main_file)

        if not os.path.exists(main_path):
            print(f"❌ Faili '{proj.main_file}' halijapatikana kwenye {proj.path}")
            return False

        print(f"\n{'='*55}")
        print(f"  🚀 Inaendesha: {proj.name}")
        print(f"  📂 Path: {proj.path}")
        print(f"  🏷️  Features: {proj.status_emoji}")
        print(f"  📝 Events: {len(proj.has_events)} registered")
        print(f"{'='*55}\n")

        # Run in the project directory
        try:
            env = os.environ.copy()
            env["PYTREX_PROJECT"] = proj.path

            if dev_mode:
                proc = subprocess.Popen(
                    [sys.executable, main_path],
                    cwd=proj.path,
                    env=env,
                )
                print(f"  ⚡ App started (PID: {proc.pid})")
                print(f"  💡 Press Ctrl+C to stop\n")
                proc.wait()
            else:
                result = subprocess.run(
                    [sys.executable, main_path],
                    cwd=proj.path,
                    env=env,
                    capture_output=True,
                    text=True,
                )
                if result.returncode == 0:
                    print(f"  ✅ App finished successfully")
                else:
                    print(f"  ❌ App exited with code {result.returncode}")
                    if result.stderr:
                        print(f"  Error: {result.stderr[:200]}")

            return True

        except KeyboardInterrupt:
            print(f"\n  ⏹️  App '{proj.name}' stopped by user")
            return True
        except Exception as e:
            print(f"  ❌ Failed: {e}")
            return False

    def run_all(self, dev_mode: bool = False) -> Dict[str, bool]:
        """Endesha miradi yote kwa sambamba (production mode tu)"""
        projects = self.list_projects()
        if not projects:
            return {}

        print(f"\n  🚀 Running {len(projects)} project(s)...\n")

        results = {}
        with ThreadPoolExecutor(max_workers=4) as executor:
            futures = {
                executor.submit(self._run_single, p, dev_mode): p.name
                for p in projects
            }

            for future in as_completed(futures):
                name = futures[future]
                try:
                    results[name] = future.result(timeout=60)
                    status = "✅" if results[name] else "❌"
                    print(f"  {status} {name}")
                except Exception as e:
                    results[name] = False
                    print(f"  ❌ {name}: {e}")

        return results

    def _run_single(self, proj: Project, dev_mode: bool) -> bool:
        """Run a single project silently"""
        main_path = os.path.join(proj.path, proj.main_file)
        try:
            result = subprocess.run(
                [sys.executable, main_path],
                cwd=proj.path,
                capture_output=True,
                text=True,
                timeout=30,
            )
            return result.returncode == 0
        except Exception:
            return False

    # ─── TESTING ──────────────────────────────────────────────

    def test(self, project_name: Optional[str] = None) -> Dict[str, Any]:
        """
        Pima mradi au miradi yote.

        Args:
            project_name: Jina la mradi (None = yote)

        Returns:
            Dict ya matokeo
        """
        from pytrex.test_runner import TestRunner

        if project_name:
            proj = self.get_project(project_name)
            if not proj:
                return {"error": f"Project '{project_name}' not found"}

            print(f"\n  🧪 Testing: {proj.name}\n")
            runner = TestRunner(verbose=True, color=True, parallel=True)
            suite = runner.run_all()
            return {
                "project": proj.name,
                "passed": suite.passed,
                "failed": suite.failed,
                "total": suite.total,
                "rate": suite.success_rate,
                "duration": suite.duration,
            }

        # Test all projects
        projects = self.list_projects()
        all_results = {}

        for proj in projects:
            print(f"\n{'='*55}")
            print(f"  🧪 Testing: {proj.name}")
            print(f"{'='*55}")

            # Change to project directory for testing
            original_cwd = os.getcwd()
            try:
                os.chdir(proj.path)
                sys.path.insert(0, proj.path)

                runner = TestRunner(verbose=True, color=True, parallel=True)
                suite = runner.run_all()

                all_results[proj.name] = {
                    "passed": suite.passed,
                    "failed": suite.failed,
                    "total": suite.total,
                    "rate": suite.success_rate,
                    "duration": suite.duration,
                }
            except Exception as e:
                all_results[proj.name] = {"error": str(e)}
            finally:
                os.chdir(original_cwd)
                if proj.path in sys.path:
                    sys.path.remove(proj.path)

        # Print final summary
        print(f"\n{'='*55}")
        print(f"  📊 TEST SUMMARY — {len(projects)} Project(s)")
        print(f"{'='*55}")
        total_passed = sum(r.get("passed", 0) for r in all_results.values())
        total_all = sum(r.get("total", 0) for r in all_results.values())
        for name, r in all_results.items():
            if "error" in r:
                print(f"  ❌ {name}: {r['error']}")
            else:
                pct = r.get("rate", 0)
                icon = "✅" if pct >= 90 else ("⚠️" if pct >= 70 else "❌")
                print(f"  {icon} {name}: {r['passed']}/{r['total']} passed ({pct:.0f}%)")
        print(f"  {'='*55}")
        print(f"  Total: {total_passed}/{total_all} passed")
        print(f"  {'='*55}\n")

        return all_results

    # ─── BUILDING ─────────────────────────────────────────────

    def build(self, project_name: str, target: str = "local") -> bool:
        """
        Build mradi kwa production.

        Args:
            project_name: Jina la mradi
            target: "local", "mesh", "serverless", "vps", "android", "ios", "web"

        Returns:
            True kama build imefanikiwa
        """
        proj = self.get_project(project_name)
        if not proj:
            print(f"❌ Mradi '{project_name}' haujagunduliwa.")
            return False

        print(f"\n  🔨 Building: {proj.name} → {target}")
        print(f"  📂 Path: {proj.path}\n")

        try:
            result = subprocess.run(
                [sys.executable, "-m", "pytrex", "build", target],
                cwd=proj.path,
            )
            return result.returncode == 0
        except Exception as e:
            print(f"  ❌ Build failed: {e}")
            return False

    # ─── WATCH MODE ───────────────────────────────────────────

    def watch(self, project_name: str) -> None:
        """
        Watch mode — auto-restart app inapobadilika code.

        Args:
            project_name: Jina la mradi
        """
        proj = self.get_project(project_name)
        if not proj:
            print(f"❌ Mradi '{project_name}' haujagunduliwa.")
            return

        print(f"\n  👁️  Watching: {proj.name}")
        print(f"  📂 {proj.path}")
        print(f"  💡 Badilisha code → auto-restart")
        print(f"  ⏹️  Press Ctrl+C to stop\n")

        main_path = os.path.join(proj.path, proj.main_file)
        proc = None
        last_restart = 0

        def start_process():
            nonlocal proc
            if proc and proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
            proc = subprocess.Popen([sys.executable, main_path], cwd=proj.path)

        def get_mod_times():
            mod_times = {}
            for dirpath, _, filenames in os.walk(proj.path):
                if "__pycache__" in dirpath or ".git" in dirpath:
                    continue
                for fn in filenames:
                    if fn.endswith(".py"):
                        fp = os.path.join(dirpath, fn)
                        try:
                            mod_times[fp] = os.path.getmtime(fp)
                        except OSError:
                            pass
            return mod_times

        start_process()
        last_times = get_mod_times()

        try:
            while proc.poll() is None:
                time.sleep(1)
                new_times = get_mod_times()
                changed = [f for f, t in new_times.items()
                           if f in last_times and t != last_times[f]]

                if changed and time.time() - last_restart > 2:
                    last_restart = time.time()
                    for f in changed[:3]:
                        rel = os.path.relpath(f, proj.path)
                        print(f"  📝 Changed: {rel}")
                    print(f"  🔄 Restarting...\n")
                    start_process()

                last_times = new_times

        except KeyboardInterrupt:
            print(f"\n  ⏹️  Watch stopped")
            if proc:
                proc.terminate()

    # ─── DASHBOARD ────────────────────────────────────────────

    def serve_dashboard(self, port: int = 8080, open_browser: bool = True) -> None:
        """
        Fungua PyTreXT Dashboard kwenye browser.
        Inaonyesha miradi yote na kuweza ku-run, test, build.

        Args:
            port: Port ya dashboard
            open_browser: Fungua browser automatically
        """
        self._dashboard_port = port
        projects = self.list_projects()

        html = self._generate_dashboard_html(projects)

        # Save dashboard to temp file
        import tempfile
        dash_path = os.path.join(tempfile.gettempdir(), "pytrex_dashboard.html")
        with open(dash_path, "w", encoding="utf-8") as f:
            f.write(html)

        if open_browser:
            webbrowser.open(f"file://{dash_path}")

        print(f"\n  📊 PyTreXT Dashboard")
        print(f"  🌐 File: {dash_path}")
        print(f"  📦 Projects: {len(projects)}")
        print(f"  💡 Open in browser: file://{dash_path}\n")

    def _generate_dashboard_html(self, projects: List[Project]) -> str:
        """Generate dashboard HTML"""
        project_cards = ""
        for proj in projects:
            features_html = ""
            if proj.has_blockchain:
                features_html += '<span class="tag bc">🔗 Blockchain</span>'
            if proj.has_database:
                features_html += '<span class="tag db">🗄️ Database</span>'
            if proj.has_frontend:
                features_html += '<span class="tag ui">🖥️ Frontend</span>'
            if proj.has_elixir:
                features_html += '<span class="tag ex">🌐 Elixir</span>'
            if proj.has_events:
                features_html += f'<span class="tag ev">⚡ {len(proj.has_events)} Events</span>'

            events_list = ""
            for ev in proj.has_events[:5]:
                events_list += f'<code>{ev}</code> '

            modified = time.strftime("%Y-%m-%d %H:%M", time.localtime(proj.last_modified)) if proj.last_modified > 0 else "unknown"

            project_cards += f"""
            <div class="project-card">
                <div class="card-header">
                    <h3>{proj.name}</h3>
                    <span class="size">{proj.size_kb:.0f} KB</span>
                </div>
                <div class="features">{features_html or '📦 Basic'}</div>
                <div class="events">{events_list}</div>
                <div class="path" title="{proj.path}">📂 {proj.path[:60]}...</div>
                <div class="modified">🕐 {modified}</div>
                <div class="actions">
                    <button class="btn-run" title="Run {proj.name}">▶ Run</button>
                    <button class="btn-test" title="Test {proj.name}">🧪 Test</button>
                    <button class="btn-build" title="Build {proj.name}">🔨 Build</button>
                </div>
            </div>"""

        return f"""<!DOCTYPE html>
<html lang="sw">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>PyTreXT Dashboard — Miradi Yangu</title>
<style>
:root {{
    --bg: #0a0a0f; --card-bg: #111827; --gold: #f0c040;
    --neon-blue: #00d4ff; --neon-green: #22d3a0; --neon-purple: #a855f7;
    --text: #e2e8f0; --muted: #64748b; --border: #1e293b;
}}
* {{ margin:0; padding:0; box-sizing:border-box; }}
body {{ background:var(--bg); color:var(--text); font-family:'Segoe UI',system-ui,sans-serif; min-height:100vh; }}
.header {{ background:linear-gradient(135deg,#111827,#1a1a2e); border-bottom:1px solid var(--border); padding:24px 32px; text-align:center; }}
.header h1 {{ background:linear-gradient(135deg,#f7d774,#f0c040,#c8960c); -webkit-background-clip:text; -webkit-text-fill-color:transparent; font-size:36px; letter-spacing:3px; }}
.header p {{ color:var(--muted); margin-top:8px; font-size:14px; }}
.stats {{ display:flex; justify-content:center; gap:24px; margin-top:16px; flex-wrap:wrap; }}
.stat {{ background:rgba(255,255,255,0.03); border:1px solid var(--border); border-radius:12px; padding:12px 20px; text-align:center; min-width:80px; }}
.stat .val {{ font-size:22px; font-weight:800; color:var(--gold); }}
.stat .lbl {{ font-size:10px; color:var(--muted); letter-spacing:2px; margin-top:4px; }}
.grid {{ display:grid; grid-template-columns:repeat(auto-fill,minmax(320px,1fr)); gap:16px; padding:24px 32px; }}
.project-card {{ background:var(--card-bg); border:1px solid var(--border); border-radius:16px; padding:20px; transition:all 0.3s; }}
.project-card:hover {{ border-color:rgba(240,192,64,0.4); box-shadow:0 0 30px rgba(240,192,64,0.06); transform:translateY(-2px); }}
.card-header {{ display:flex; justify-content:space-between; align-items:center; margin-bottom:12px; }}
.card-header h3 {{ color:var(--text); font-size:18px; }}
.size {{ color:var(--muted); font-size:11px; }}
.features {{ display:flex; flex-wrap:wrap; gap:6px; margin-bottom:10px; }}
.tag {{ font-size:10px; padding:3px 8px; border-radius:12px; letter-spacing:0.5px; }}
.tag.bc {{ background:rgba(240,192,64,0.15); color:#f0c040; }}
.tag.db {{ background:rgba(0,212,255,0.15); color:#00d4ff; }}
.tag.ui {{ background:rgba(34,211,160,0.15); color:#22d3a0; }}
.tag.ex {{ background:rgba(168,85,247,0.15); color:#a855f7; }}
.tag.ev {{ background:rgba(244,114,182,0.15); color:#f472b6; }}
.events {{ margin:8px 0; }}
.events code {{ background:rgba(255,255,255,0.05); padding:2px 6px; border-radius:4px; font-size:10px; margin:0 2px; color:var(--muted); }}
.path {{ font-size:11px; color:var(--muted); margin:8px 0; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }}
.modified {{ font-size:10px; color:var(--muted); margin-bottom:12px; }}
.actions {{ display:flex; gap:8px; }}
.actions button {{ flex:1; padding:8px; border-radius:8px; border:1px solid var(--border); background:rgba(255,255,255,0.03); color:var(--text); font-size:12px; cursor:pointer; transition:all 0.2s; font-weight:600; letter-spacing:0.5px; }}
.btn-run:hover {{ background:rgba(34,211,160,0.15); border-color:#22d3a0; color:#22d3a0; }}
.btn-test:hover {{ background:rgba(0,212,255,0.15); border-color:#00d4ff; color:#00d4ff; }}
.btn-build:hover {{ background:rgba(240,192,64,0.15); border-color:#f0c040; color:#f0c040; }}
.empty {{ text-align:center; padding:60px; color:var(--muted); grid-column:1/-1; }}
.empty h2 {{ font-size:24px; margin-bottom:8px; }}
.footer {{ text-align:center; padding:20px; color:var(--muted); font-size:11px; border-top:1px solid var(--border); }}
</style>
</head>
<body>
<div class="header">
    <h1>⚡ T-PYTREXT</h1>
    <p>Full-Stack AI & Real-Time Desktop Framework</p>
    <div class="stats">
        <div class="stat"><div class="val">{len(projects)}</div><div class="lbl">Projects</div></div>
        <div class="stat"><div class="val">{sum(1 for p in projects if p.has_blockchain)}</div><div class="lbl">Blockchain</div></div>
        <div class="stat"><div class="val">{sum(1 for p in projects if p.has_frontend)}</div><div class="lbl">Frontend</div></div>
        <div class="stat"><div class="val">{sum(1 for p in projects if p.has_elixir)}</div><div class="lbl">Elixir</div></div>
        <div class="stat"><div class="val">{sum(len(p.has_events) for p in projects)}</div><div class="lbl">Events</div></div>
    </div>
</div>
<div class="grid">
    {project_cards if projects else '<div class="empty"><h2>📭 No Projects Found</h2><p>Run <code>pytrex scan</code> to discover your PyTreXT projects</p></div>'}
</div>
<div class="footer">PyTreXT Framework v1.0 &copy; 2026 — Built with ❤️</div>
<script>
document.querySelectorAll('.btn-run').forEach(btn => {{
    btn.addEventListener('click', function() {{
        const name = this.closest('.project-card').querySelector('h3').textContent;
        alert('▶ Running: ' + name + '\\n\\nUse CLI: pytrex run "' + name + '"');
    }});
}});
document.querySelectorAll('.btn-test').forEach(btn => {{
    btn.addEventListener('click', function() {{
        const name = this.closest('.project-card').querySelector('h3').textContent;
        alert('🧪 Testing: ' + name + '\\n\\nUse CLI: pytrex test --project "' + name + '"');
    }});
}});
document.querySelectorAll('.btn-build').forEach(btn => {{
    btn.addEventListener('click', function() {{
        const name = this.closest('.project-card').querySelector('h3').textContent;
        alert('🔨 Building: ' + name + '\\n\\nUse CLI: pytrex build "' + name + '"');
    }});
}});
</script>
</body>
</html>"""

    # ─── UTILITY ──────────────────────────────────────────────

    def export_json(self, filepath: str = "pytrex_projects.json") -> str:
        """Export taarifa za miradi yote kwenye JSON"""
        projects = self.list_projects()
        data = {
            "framework": "PyTreXT",
            "version": "1.0.0",
            "total_projects": len(projects),
            "projects": [p.to_dict() for p in projects],
            "scanned_dirs": self._root_dirs,
            "exported_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        }
        with open(filepath, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        return filepath

    def to_dict(self) -> Dict[str, Any]:
        return {
            "total_projects": len(self._projects),
            "projects": [p.to_dict() for p in self._projects.values()],
            "scanned_dirs": self._root_dirs,
        }

    def __repr__(self) -> str:
        return f"ProjectManager(projects={len(self._projects)}, dirs={len(self._root_dirs)})"
