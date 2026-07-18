import os
import sys
import subprocess
import time
import click


def _read_template(filename: str) -> str:
    template_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "templates")
    path = os.path.join(template_dir, filename)
    with open(path, "r", encoding="utf-8") as f:
        return f.read()


@click.group()
def main():
    """PyTreX Framework CLI — Full-Stack AI & Real-Time Desktop Framework"""
    pass


@main.command()
@click.argument("project_name")
def init(project_name):
    """Initialize a new PyTreX project."""
    click.echo(f"[PyTreX CLI] Inatengeneza mradi: {project_name}...")

    # 1. Kutengeneza ma-folder
    os.makedirs(f"{project_name}/frontend", exist_ok=True)
    os.makedirs(f"{project_name}/backend/ai", exist_ok=True)
    os.makedirs(f"{project_name}/backend/network", exist_ok=True)
    os.makedirs(f"{project_name}/models", exist_ok=True)

    # 2. Kutengeneza faili la main.py kutoka template
    main_code = _read_template("main.py.tmpl").format(project_name=project_name)
    with open(f"{project_name}/main.py", "w") as f:
        f.write(main_code)

    # 3. Kutengeneza faili la frontend/index.html kutoka template
    html_code = _read_template(os.path.join("frontend", "index.html.tmpl")).format(project_name=project_name)
    with open(f"{project_name}/frontend/index.html", "w") as f:
        f.write(html_code)

    # 4. Kutengeneza tauri.conf.json kutoka template
    tauri_code = _read_template("tauri.conf.json.tmpl").format(project_name=project_name)
    with open(f"{project_name}/tauri.conf.json", "w") as f:
        f.write(tauri_code)

    # 5. Kutengeneza requirements.txt
    with open(f"{project_name}/requirements.txt", "w") as f:
        f.write("pytrex-framework\n")

    # 6. Kutengeneza .gitignore
    gitignore = """__pycache__/
*.pyc
*.db
*.log
target/
node_modules/
.env
*.whl
"""
    with open(f"{project_name}/.gitignore", "w") as f:
        f.write(gitignore)

    # 7. Kutengeneza README.md
    readme = f"""# {project_name}

Built with **PyTreX Framework** — Full-Stack AI, Real-Time & Containerized Desktop Framework.

## Quick Start

```bash
pip install -r requirements.txt
pytrex dev
```

## Build for Production

```bash
pytrex build local    # Desktop App
pytrex build mesh     # Elixir Cluster
pytrex build serverless  # Docker
pytrex build vps      # VPS Deployment
```

## Features

- Rust + PyO3 native core
- Tauri v2 desktop UI
- SQLx encrypted database (AES-256)
- Blockchain engine (SHA-256)
- PyTorch AI integration
- Elixir concurrency engine
"""
    with open(f"{project_name}/README.md", "w") as f:
        f.write(readme)

    click.echo(f"[PyTreX CLI] Hongera! Mradi '{project_name}' umekamilika.")
    click.echo(f"  cd {project_name}")
    click.echo(f"  pytrex dev")


@main.command()
@click.option("--no-watch", is_flag=True, help="Disable hot reloading")
def dev(no_watch):
    """Run the project in development mode with hot reloading."""
    if not os.path.exists("main.py"):
        click.echo("[PyTreX CLI] Makosa: Faili la 'main.py' halijapatikana kwenye folder hili!")
        sys.exit(1)

    if no_watch:
        click.echo("[PyTreX CLI] Inawasha bila hot reloading...")
        subprocess.run([sys.executable, "main.py"])
        return

    click.echo("[PyTreX CLI] Inawasha mazingira ya majaribio (Hot-Reloading)...")
    click.echo("[PyTreX CLI] Kufunga: Ctrl+C")

    try:
        from watchdog.observers import Observer
        from watchdog.events import FileSystemEventHandler

        class RestartHandler(FileSystemEventHandler):
            def __init__(self):
                self._process = None
                self._last_restart = 0
                self._start_process()

            def _start_process(self):
                if self._process and self._process.poll() is None:
                    self._process.terminate()
                    self._process.wait(timeout=5)
                self._process = subprocess.Popen([sys.executable, "main.py"])

            def on_modified(self, event):
                if event.src_path.endswith(".py"):
                    now = time.time()
                    if now - self._last_restart > 2.0:
                        self._last_restart = now
                        click.echo(f"\n[PyTreX CLI] Badiliko: {event.src_path} — Inareload...")
                        self._start_process()

        handler = RestartHandler()
        observer = Observer()
        observer.schedule(handler, ".", recursive=True, patterns=["*.py"])
        observer.start()

        try:
            while handler._process.poll() is None:
                time.sleep(1)
        except KeyboardInterrupt:
            click.echo("\n[PyTreX CLI] Inazima...")
            if handler._process and handler._process.poll() is None:
                handler._process.terminate()
            observer.stop()
        observer.join()

    except ImportError:
        click.echo("[PyTreX CLI] 'watchdog' haipo — inawasha bila hot reloading...")
        click.echo("[PyTreX CLI] Sakinisha: pip install watchdog")
        subprocess.run([sys.executable, "main.py"])


@main.command()
@click.argument("target", required=False, default="local")
def build(target):
    """Build the project for production. Target: local | mesh | serverless | vps"""
    click.echo(f"[PyTreX CLI] Inaanza ujenzi wa mfumo kwa ajili ya mazingira ya: {target.upper()}")

    if target == "local":
        click.echo("[PyTreX] Kifurushi kinatengenezwa kama Standalone Desktop App (Inatumia Local AI & Database).")
        click.echo("[PyTreX CLI] Step 1: Compiling Rust core (maturin build --release)...")
        subprocess.run([sys.executable, "-m", "maturin", "build", "--release"])
        click.echo("[PyTreX CLI] Step 2: Bundling Tauri desktop installer...")
        subprocess.run(["cargo", "tauri", "build"])
        click.echo("[PyTreX CLI] Build complete! Check target/wheels/ for .whl files.")

    elif target == "mesh":
        click.echo("[PyTreX] Inasanidi Elixir Cluster kwa ajili ya kuunganisha kompyuta za ofisi bila intaneti.")
        click.echo("[PyTreX] Inawasha protokoli ya Erlang Distributed Nodes (EPMD)...")
        elixir_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "pytrex_engine")
        if os.path.exists(elixir_dir):
            subprocess.run(["mix", "deps.get"], cwd=elixir_dir)
            subprocess.run(["mix", "release", "--prod"], cwd=elixir_dir)
        click.echo("[PyTreX CLI] Mesh build complete! Elixir release ready for cluster deployment.")

    elif target == "serverless":
        click.echo("[PyTreX] Inafunganya mfumo kuwa Docker nyepesi tayari kwa Cloud Run / Lambda.")
        dockerfile_content = """FROM python:3.12-slim
RUN pip install pytrex-framework
COPY . /app
WORKDIR /app
CMD ["python", "main.py"]
"""
        with open("Dockerfile", "w") as f:
            f.write(dockerfile_content)
        click.echo("[PyTreX CLI] Dockerfile created! Build with: docker build -t myapp .")

    elif target == "vps":
        click.echo("[PyTreX] Inaandaa mfumo kwa ajili ya Cloud Server ya kawaida (VPS).")
        click.echo("[PyTreX CLI] Step 1: Compiling Rust core...")
        subprocess.run([sys.executable, "-m", "maturin", "build", "--release"])
        click.echo("[PyTreX CLI] Step 2: Creating deployment package (.zip)...")
        click.echo("[PyTreX CLI] VPS build complete! Upload target/wheels/*.whl to your server.")

    elif target == "android":
        click.echo("[PyTreX] Inatengeneza Android APK/AAB kwa kutumia Tauri v2 mobile.")
        click.echo("[PyTreX CLI] Step 1: Initializing Android project...")
        subprocess.run(["cargo", "tauri", "android", "init"])
        click.echo("[PyTreX CLI] Step 2: Building Android APK...")
        subprocess.run(["cargo", "tauri", "android", "build"])
        click.echo("[PyTreX CLI] Android build complete! Check gen/android/ for APK files.")

    elif target == "ios":
        click.echo("[PyTreX] Inatengeneza iOS app kwa kutumia Tauri v2 mobile.")
        click.echo("[PyTreX CLI] Step 1: Initializing iOS project...")
        subprocess.run(["cargo", "tauri", "ios", "init"])
        click.echo("[PyTreX CLI] Step 2: Building iOS app...")
        subprocess.run(["cargo", "tauri", "ios", "build"])
        click.echo("[PyTreX CLI] iOS build complete! Check gen/apple/ for Xcode project.")

    elif target == "web":
        click.echo("[PyTreX] Inatengeneza Web App (FastAPI server mode).")
        web_template = '''import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from pytrex.core import PyTreXApp, APIServer, event
import json

app = PyTreXApp(name="PyTreX Web Server")
server = APIServer(host="0.0.0.0", port=int(os.environ.get("PORT", 8000)))

@server.endpoint("/api/status")
def status(data):
    return {"status": "ok", "framework": "PyTreX", "version": "1.0.0"}

@server.endpoint("/api/event")
def trigger_event(data):
    if isinstance(data, str):
        payload = json.loads(data) if data else {}
    else:
        payload = data
    event_name = payload.get("event", "ping")
    event_data = payload.get("data", "{}")
    from pytrex.core import execute_python_event
    result = execute_python_event(event_name, event_data)
    return json.loads(result)

if __name__ == "__main__":
    server.start()
    print(f"[PyTreX Web] Server running on port {server.port}")
    import time
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        server.stop()
        print("[PyTreX Web] Server stopped")
'''
        with open("web_server.py", "w") as f:
            f.write(web_template)
        click.echo("[PyTreX CLI] web_server.py created!")
        click.echo("[PyTreX CLI] Run: python web_server.py")
        click.echo("[PyTreX CLI] Endpoints: /api/status, /api/event")

    else:
        click.echo(f"[PyTreX CLI] Target haijulikani: {target}")
        click.echo("Tafadhali chagua: local | mesh | serverless | vps | android | ios | web")
        sys.exit(1)


@main.command()
@click.argument("root_path", required=False, default="isolated_root")
def containerize(root_path):
    """Isolate the app in a Linux container (Linux only)."""
    try:
        import my_framework
        my_framework.anzisha_container(root_path)
    except Exception as e:
        click.echo(f"[PyTreX CLI] Container engine error: {e}")
        sys.exit(1)


@main.command()
@click.option("--all", "run_all", is_flag=True, help="Test modules zote")
@click.option("--module", "-m", multiple=True, help="Pima module maalum (inaweza kurudiwa)")
@click.option("--watch", is_flag=True, help="Watch mode — auto-run tests on file changes")
@click.option("--json", "output_json", is_flag=True, help="Export matokeo kwenye JSON")
@click.option("--quick", is_flag=True, help="Jaribio la haraka (core tests pekee)")
def test(run_all, module, watch, output_json, quick):
    """
    Pima mfumo wako uliotengenezwa kwa PyTreXT.

    Examples:
        pytrex test                    # Test app yako
        pytrex test --all              # Test modules zote
        pytrex test -m Blockchain      # Test blockchain pekee
        pytrex test -m Core -m AI/ML   # Test modules maalum
        pytrex test --watch            # Watch mode
        pytrex test --json             # Export to JSON
        pytrex test --quick            # Quick test (core only)
    """
    try:
        from pytrex.test_runner import TestRunner
    except ImportError as e:
        click.echo(f"[PyTreX CLI] Error importing TestRunner: {e}")
        click.echo("[PyTreX CLI] Make sure pytrex is installed: pip install -e .")
        sys.exit(1)

    # Watch mode
    if watch:
        _run_tests_watch(run_all, list(module), quick, output_json)
        return

    # Run tests
    runner = TestRunner(verbose=True, color=True, parallel=not watch)

    if quick:
        click.echo("[PyTreX CLI] Running quick test (core modules only)...")
        result = runner.quick_test()
        suite = runner.suite
    elif module:
        click.echo(f"[PyTreX CLI] Testing modules: {', '.join(module)}")
        suite = runner.run_all(modules=list(module))
    else:
        click.echo("[PyTreX CLI] Testing your PyTreXT application...")
        suite = runner.run_all()

    # Export JSON if requested
    if output_json:
        filepath = runner.export_json("pytrex_test_results.json")
        click.echo(f"\n[PyTreX CLI] Results exported to: {filepath}")

    # Exit with proper code
    sys.exit(0 if suite.failed == 0 else 1)


def _run_tests_watch(run_all: bool, modules: list, quick: bool, output_json: bool):
    """Watch mode — auto-run tests when files change"""
    click.echo("[PyTreX CLI] Watch mode enabled. Press Ctrl+C to stop.")
    click.echo("[PyTreX CLI] Watching for file changes...\n")

    try:
        from watchdog.observers import Observer
        from watchdog.events import FileSystemEventHandler

        class TestRunnerHandler(FileSystemEventHandler):
            def __init__(self):
                self._last_run = 0
                self._run_tests()

            def _run_tests(self):
                from pytrex.test_runner import TestRunner
                runner = TestRunner(verbose=True, color=True, parallel=True)

                if quick:
                    runner.quick_test()
                elif modules:
                    runner.run_all(modules=modules)
                else:
                    runner.run_all()

                if output_json:
                    runner.export_json("pytrex_test_results.json")
                    click.echo("[PyTreX CLI] Results exported to: pytrex_test_results.json")

            def on_modified(self, event):
                if event.src_path.endswith(".py"):
                    now = time.time()
                    if now - self._last_run > 3.0:
                        self._last_run = now
                        click.echo(f"\n[PyTreX CLI] File changed: {os.path.basename(event.src_path)}")
                        self._run_tests()

        handler = TestRunnerHandler()
        observer = Observer()
        observer.schedule(handler, ".", recursive=True, patterns=["*.py"])
        observer.start()

        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            click.echo("\n[PyTreX CLI] Watch mode stopped.")
            observer.stop()
        observer.join()

    except ImportError:
        click.echo("[PyTreX CLI] 'watchdog' not installed. Install with: pip install watchdog")
        sys.exit(1)


# ═══════════════════════════════════════════════════════════════
#  PROJECT MANAGER COMMANDS
# ═══════════════════════════════════════════════════════════════

@main.command()
@click.option("--dir", "-d", "scan_dir", multiple=True, help="Directory ya kuscan (inaweza kurudiwa)")
@click.option("--depth", default=4, help="Kina cha kuscan subdirectories")
def scan(scan_dir, depth):
    """
    Scan kompyuta kutafuta miradi yote ya PyTreXT.

    Examples:
        pytrex scan                    # Scan default locations
        pytrex scan -d ~/Projects      # Scan folder maalum
        pytrex scan -d ~/Dev -d ~/Code  # Scan folders kadhaa
    """
    from pytrex.project_manager import ProjectManager

    root_dirs = list(scan_dir) if scan_dir else None
    pm = ProjectManager(root_dirs=root_dirs)
    projects = pm.scan(max_depth=depth)

    if not projects:
        click.echo("\n  📭 Hakuna miradi ya PyTreXT iliyogunduliwa.")
        click.echo("  💡 Tumia 'pytrex init jina-la-mradi' kutengeneza mradi mpya.\n")
        return

    # Show the projects table
    click.echo(pm.projects_table())


@main.command()
@click.option("--refresh", "-r", is_flag=True, help="Scan upya kabla ya kuorodhesha")
def projects(refresh):
    """
    Orodhesha miradi yote ya PyTreXT iliyogunduliwa.

    Examples:
        pytrex projects           # List all projects
        pytrex projects --refresh  # Re-scan then list
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    click.echo(pm.projects_table())


@main.command()
@click.argument("project_name")
@click.option("--prod", is_flag=True, help="Run in production mode (no live output)")
def run(project_name, prod):
    """
    Endesha mradi wa PyTreXT.

    Examples:
        pytrex run SmartBank        # Endesha SmartBank
        pytrex run MyApp --prod     # Endesha kwa production mode
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    success = pm.run(project_name, dev_mode=not prod)
    if not success:
        sys.exit(1)


@main.command()
@click.argument("project_name")
@click.option("--target", "-t", default="local", help="Build target: local|mesh|serverless|vps")
def build_project(project_name, target):
    """
    Build mradi kwa production.

    Examples:
        pytrex build-project SmartBank            # Build ya kawaida
        pytrex build-project SmartBank -t serverless  # Docker build
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    success = pm.build(project_name, target)
    if not success:
        sys.exit(1)


@main.command()
@click.argument("project_name", required=False)
def test_project(project_name):
    """
    Pima mradi au miradi yote.

    Examples:
        pytrex test-project              # Pima miradi YOTE
        pytrex test-project SmartBank    # Pima SmartBank pekee
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()

    if project_name:
        pm.test(project_name)
    else:
        results = pm.test()
        if any("error" in r for r in results.values()):
            sys.exit(1)


@main.command()
@click.argument("project_name")
def watch_project(project_name):
    """
    Watch mode — auto-restart mradi inapobadilika code.

    Examples:
        pytrex watch SmartBank    # Watch SmartBank kwa development
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    pm.watch(project_name)


@main.command()
@click.option("--port", "-p", default=8080, help="Dashboard port")
@click.option("--no-browser", is_flag=True, help="Usifungue browser")
def dashboard(port, no_browser):
    """
    Fungua PyTreXT Dashboard kwenye browser.
    Inaonyesha miradi yote, features, na actions.

    Examples:
        pytrex dashboard                  # Fungua dashboard
        pytrex dashboard -p 3000          # Port maalum
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    pm.serve_dashboard(port=port, open_browser=not no_browser)


@main.command()
@click.option("--file", "-f", default="pytrex_projects.json", help="Output JSON file")
def export(file):
    """
    Export taarifa za miradi yote kwenye JSON.

    Examples:
        pytrex export                     # Default: pytrex_projects.json
        pytrex export -f my_projects.json # Custom filename
    """
    from pytrex.project_manager import ProjectManager
    pm = ProjectManager()
    pm.scan(show_progress=True)
    filepath = pm.export_json(file)
    click.echo(f"\n  ✅ Exported to: {filepath}")
    click.echo(f"  📦 Projects: {len(pm._projects)}\n")


# ═══════════════════════════════════════════════════════════════
#  PRODUCTION DEPLOYMENT COMMANDS
# ═══════════════════════════════════════════════════════════════

@main.command()
@click.argument("project_name", required=False, default=".")
@click.option("--target", "-t", default="docker",
              help="Deployment target: docker|standalone|vps|aws|gcp|azure|k8s|systemd")
@click.option("--port", "-p", default=8080, help="Application port")
@click.option("--workers", "-w", default=4, help="Number of workers")
@click.option("--memory", "-m", default="512m", help="Memory limit")
@click.option("--cpu", "-c", default="1.0", help="CPU limit")
@click.option("--domain", "-d", default="", help="Domain name for SSL")
@click.option("--no-ssl", is_flag=True, help="Disable SSL")
@click.option("--skip-tests", is_flag=True, help="Skip tests before deploy")
def deploy(project_name, target, port, workers, memory, cpu, domain, no_ssl, skip_tests):
    """
    Deploy mradi wako kwa production.

    Supports: docker, standalone, vps, aws, gcp, azure, k8s, systemd

    Examples:
        pytrex deploy                              # Docker deploy (current dir)
        pytrex deploy SmartBank                    # Deploy SmartBank kwa Docker
        pytrex deploy -t standalone                # Build standalone .exe/.app
        pytrex deploy -t aws                       # Deploy to AWS ECS
        pytrex deploy -t k8s                       # Generate Kubernetes manifests
        pytrex deploy -t vps -p 80 -d myapp.com    # VPS + Nginx + SSL
        pytrex deploy -t systemd                   # Linux systemd service
    """
    from pytrex.production import ProductionBuilder, ProductionConfig

    # Resolve project path
    if project_name != ".":
        from pytrex.project_manager import ProjectManager
        pm = ProjectManager()
        proj = pm.get_project(project_name)
        if proj:
            project_path = proj.path
            click.echo(f"\n  📦 Project: {proj.name}")
            click.echo(f"  📂 Path: {proj.path}")
        else:
            click.echo(f"  ❌ Project '{project_name}' not found.")
            click.echo(f"  💡 Run 'pytrex scan' first to discover projects.")
            sys.exit(1)
    else:
        project_path = "."

    # Build config
    config = ProductionConfig(
        app_name=os.path.basename(os.path.abspath(project_path)),
        port=port,
        workers=workers,
        memory_limit=memory,
        cpu_limit=cpu,
        domain=domain,
        enable_ssl=not no_ssl,
    )

    builder = ProductionBuilder(project_path=project_path, config=config, verbose=True)

    # Run all deployment steps
    try:
        results = builder.deploy_all(target=target)
        if results.get("build", False) or results.get("configs", False):
            sys.exit(0)
        else:
            sys.exit(1)
    except KeyboardInterrupt:
        click.echo("\n  ⏹️  Deployment cancelled")
        sys.exit(1)


@main.command()
@click.option("--target", "-t", default="docker", help="Build target")
def deploy_quick(target):
    """
    Deploy ya haraka — current directory kwa Docker moja kwa moja.

    Examples:
        pytrex deploy-quick           # Docker build & run
        pytrex deploy-quick -t docker  # Same
    """
    from pytrex.production import ProductionBuilder

    builder = ProductionBuilder(verbose=True)
    success = builder.build_docker()

    if success:
        click.echo("\n  ✅ Docker image built successfully!")
        click.echo(f"  🚀 Run: docker run -d -p {builder.config.port}:{builder.config.port} {builder.config.app_name}")
    else:
        click.echo("\n  ❌ Docker build failed. Check errors above.")
        sys.exit(1)


if __name__ == "__main__":
    main()
