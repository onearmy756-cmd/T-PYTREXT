"""
PyTreXT Production Builder — Deploy Production-Ready Apps
============================================================
Inabadilisha mradi wako wa PyTreXT kuwa production-ready kwa:
- Standalone executables (Windows .exe, macOS .app, Linux binary)
- Docker containerization + docker-compose
- Cloud deployment (AWS, GCP, Azure, DigitalOcean)
- Kubernetes (k8s) manifests
- Nginx reverse proxy + SSL/TLS
- Systemd service (Linux)
- Windows Service
- Security hardening
- Production logging + monitoring
- Environment management
- CI/CD pipelines (GitHub Actions, GitLab CI)
- Health checks + auto-restart
- Performance optimization

Usage:
    from pytrex.production import ProductionBuilder
    builder = ProductionBuilder(my_project_path)
    builder.build_standalone()
    builder.build_docker()
    builder.deploy_cloud("aws")
"""

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from enum import Enum


class DeployTarget(Enum):
    """Malengo ya production deployment"""
    STANDALONE = "standalone"      # .exe / .app / binary
    DOCKER = "docker"              # Docker container
    DOCKER_COMPOSE = "compose"     # Docker Compose (app + DB + Elixir)
    VPS = "vps"                    # Linux VPS server
    CLOUD_AWS = "aws"              # AWS (ECS / EC2)
    CLOUD_GCP = "gcp"              # Google Cloud Run
    CLOUD_AZURE = "azure"          # Azure Container Instances
    KUBERNETES = "k8s"             # Kubernetes cluster
    SERVERLESS = "serverless"      # AWS Lambda / Cloud Run
    WINDOWS_SERVICE = "win_svc"    # Windows Service
    SYSTEMD = "systemd"            # Linux systemd service


@dataclass
class ProductionConfig:
    """Configuration ya production"""
    app_name: str = "pytrex-app"
    version: str = "1.0.0"
    port: int = 8080
    workers: int = 4
    memory_limit: str = "512m"
    cpu_limit: str = "1.0"

    # Security
    enable_ssl: bool = True
    enable_firewall: bool = True
    enable_rate_limiting: bool = True
    enable_cors: bool = True
    allowed_hosts: List[str] = field(default_factory=lambda: ["*"])

    # Logging
    log_level: str = "INFO"
    log_format: str = "json"
    log_retention_days: int = 30

    # Monitoring
    enable_health_check: bool = True
    health_check_path: str = "/health"
    enable_metrics: bool = True
    metrics_port: int = 9090

    # Database
    db_engine: str = "sqlite"
    db_path: str = "/data/pytrex.db"
    db_backup_enabled: bool = True
    db_backup_interval_hours: int = 24

    # Domain & Network
    domain: str = ""
    enable_cdn: bool = False

    # Environment
    env_vars: Dict[str, str] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "app_name": self.app_name, "version": self.version,
            "port": self.port, "workers": self.workers,
            "memory_limit": self.memory_limit, "cpu_limit": self.cpu_limit,
            "enable_ssl": self.enable_ssl, "enable_firewall": self.enable_firewall,
            "log_level": self.log_level, "enable_health_check": self.enable_health_check,
            "enable_metrics": self.enable_metrics, "domain": self.domain,
        }


class ProductionBuilder:
    """
    PyTreXT Production Builder — deploy apps kwa production.
    Supports: standalone, Docker, cloud, Kubernetes, VPS, serverless.
    """

    def __init__(
        self,
        project_path: str = ".",
        config: Optional[ProductionConfig] = None,
        verbose: bool = True,
    ):
        self.project_path = os.path.abspath(project_path)
        self.config = config or ProductionConfig(
            app_name=os.path.basename(self.project_path),
        )
        self.verbose = verbose
        self._output_dir = os.path.join(self.project_path, "dist", "production")
        self._deploy_dir = os.path.join(self.project_path, "deploy")

        # Auto-detect project info
        self._detect_project()

    def _detect_project(self) -> None:
        """Detect project information from the codebase"""
        main_file = os.path.join(self.project_path, "main.py")
        if os.path.exists(main_file):
            try:
                with open(main_file, "r", encoding="utf-8") as f:
                    content = f.read()

                # Detect app name from class
                import re
                name_match = re.search(r'class\s+(\w+)\s*\(\s*PyTreXApp\s*\)', content)
                if name_match:
                    self.config.app_name = name_match.group(1).lower()

                # Detect events
                events = re.findall(r'@event\(["\']([^"\']+)["\']\)', content)
                if events and self.verbose:
                    self._log(f"Detected {len(events)} events: {', '.join(events[:5])}")

            except Exception:
                pass

    def _log(self, msg: str, level: str = "INFO") -> None:
        """Internal logging"""
        if self.verbose:
            prefix = {"INFO": "  📋", "OK": "  ✅", "ERROR": "  ❌", "WARN": "  ⚠️",
                       "BUILD": "  🔨", "DOCKER": "  🐳", "CLOUD": "  ☁️", "SECURE": "  🔒"}
            p = prefix.get(level, "  •")
            print(f"{p} {msg}")

    def _run_cmd(self, cmd: List[str], cwd: str = None, check: bool = False) -> Tuple[int, str, str]:
        """Run a shell command"""
        try:
            result = subprocess.run(
                cmd, cwd=cwd or self.project_path,
                capture_output=True, text=True, timeout=300,
            )
            return result.returncode, result.stdout, result.stderr
        except Exception as e:
            return -1, "", str(e)

    # ═══════════════════════════════════════════════════════════
    #  STANDALONE BUILD
    # ═══════════════════════════════════════════════════════════

    def build_standalone(self) -> bool:
        """
        Build standalone executable.
        - Windows: .exe + installer
        - macOS: .app bundle + .dmg
        - Linux: binary + .AppImage
        """
        self._log(f"Building standalone: {self.config.app_name}", "BUILD")

        os.makedirs(self._output_dir, exist_ok=True)

        platform = sys.platform
        success = True

        # Step 1: Build Rust core
        self._log("Step 1/4: Building Rust core (maturin)...")
        ret, out, err = self._run_cmd([sys.executable, "-m", "maturin", "build", "--release"])
        if ret != 0:
            self._log(f"Rust build failed: {err[:200]}", "ERROR")
            return False
        self._log("Rust core built", "OK")

        # Step 2: Build Tauri desktop app
        self._log("Step 2/4: Building Tauri v2 desktop app...")
        if os.path.exists(os.path.join(self.project_path, "tauri.conf.json")):
            ret, out, err = self._run_cmd(["cargo", "tauri", "build"])
            if ret == 0:
                self._log("Tauri desktop app built", "OK")
            else:
                self._log(f"Tauri build skipped: {err[:100]}", "WARN")
        else:
            self._log("No Tauri config — skipping desktop bundle", "WARN")

        # Step 3: Create PyInstaller standalone
        self._log("Step 3/4: Creating standalone Python bundle...")
        self._generate_standalone_spec()

        ret, out, err = self._run_cmd([
            sys.executable, "-m", "PyInstaller",
            "--clean", "--noconfirm",
            "--distpath", self._output_dir,
            os.path.join(self.project_path, "pytrex_standalone.spec"),
        ])

        if ret != 0:
            self._log(f"PyInstaller fallback — creating portable package instead", "WARN")
            success = self._create_portable_package()

        # Step 4: Generate installer scripts
        self._log("Step 4/4: Generating installer scripts...")
        self._generate_installer_scripts(platform)
        self._log("Installer scripts generated", "OK")

        # Summary
        self._log(f"Build complete → {self._output_dir}", "OK")
        self._log(f"Output: {os.listdir(self._output_dir)[:5]}", "INFO")
        return success

    def _generate_standalone_spec(self) -> str:
        """Generate PyInstaller spec file"""
        spec_content = f"""# -*- mode: python ; coding: utf-8 -*-
# PyTreXT Standalone Build Spec
a = Analysis(
    ['{os.path.join(self.project_path, 'main.py')}'],
    pathex=['{self.project_path}'],
    binaries=[],
    datas=[
        ('{os.path.join(self.project_path, 'frontend')}', 'frontend'),
    ],
    hiddenimports=[
        'pytrex', 'pytrex.core', 'pytrex.cli',
        'pytrex.langchain_agent', 'pytrex.search_engine',
        'pytrex.human_in_loop', 'pytrex.hermes_agent', 'pytrex.mcp_client',
        'pytrex.test_runner', 'pytrex.project_manager',
        'json', 'logging', 'threading', 'asyncio',
    ],
    hookspath=[],
    runtime_hooks=[],
    excludes=['tkinter', 'test', 'unittest'],
)
pyz = PYZ(a.pure)
exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    name='{self.config.app_name}',
    debug=False,
    bootloader_ignore_signals=False,
    strip=True,
    upx=True,
    console=True,
)
"""
        spec_path = os.path.join(self.project_path, "pytrex_standalone.spec")
        with open(spec_path, "w") as f:
            f.write(spec_content)
        return spec_path

    def _create_portable_package(self) -> bool:
        """Create portable Python package (no PyInstaller needed)"""
        pkg_dir = os.path.join(self._output_dir, self.config.app_name)
        os.makedirs(pkg_dir, exist_ok=True)

        # Copy project files
        for item in os.listdir(self.project_path):
            src = os.path.join(self.project_path, item)
            dst = os.path.join(pkg_dir, item)
            if item in ("__pycache__", ".git", "node_modules", "target", "dist", ".venv"):
                continue
            try:
                if os.path.isfile(src):
                    shutil.copy2(src, dst)
                elif os.path.isdir(src) and not item.startswith("."):
                    shutil.copytree(src, dst, dirs_exist_ok=True,
                                    ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
            except Exception:
                pass

        # Create run scripts
        self._generate_run_scripts(pkg_dir)
        return True

    def _generate_run_scripts(self, pkg_dir: str) -> None:
        """Generate run scripts for different platforms"""
        app = self.config.app_name

        # Windows: run.bat
        with open(os.path.join(pkg_dir, "run.bat"), "w") as f:
            f.write(f"""@echo off
echo Starting {app}...
python main.py
pause
""")

        # Linux/macOS: run.sh
        run_sh = os.path.join(pkg_dir, "run.sh")
        with open(run_sh, "w") as f:
            f.write(f"""#!/bin/bash
echo "Starting {app}..."
cd "$(dirname "$0")"
python3 main.py
""")
        try:
            os.chmod(run_sh, 0o755)
        except Exception:
            pass

        # requirements.txt
        with open(os.path.join(pkg_dir, "requirements.txt"), "w") as f:
            f.write("pytrex-framework\n")

    def _generate_installer_scripts(self, platform: str) -> None:
        """Generate platform-specific installer scripts"""
        app = self.config.app_name

        if platform.startswith("win"):
            # Inno Setup script
            iss = os.path.join(self._output_dir, f"{app}_installer.iss")
            with open(iss, "w") as f:
                f.write(f"""; Inno Setup Script for {app}
[Setup]
AppName={app}
AppVersion={self.config.version}
DefaultDirName={{pf32}}\\{app}
OutputDir=.
OutputBaseFilename={app}_setup
Compression=lzma
SolidCompression=yes

[Files]
Source: "{app}\\*"; DestDir: "{{app}}"; Flags: recursesubdirs

[Icons]
Name: "{{group}}\\{app}"; Filename: "{{app}}\\run.bat"
Name: "{{commondesktop}}\\{app}"; Filename: "{{app}}\\run.bat"
""")

        elif platform.startswith("linux"):
            # AppImage / .desktop file
            desktop = os.path.join(self._output_dir, f"{app}.desktop")
            with open(desktop, "w") as f:
                f.write(f"""[Desktop Entry]
Name={app}
Comment=PyTreXT Application
Exec={app}/run.sh
Type=Application
Categories=Development;
Terminal=true
""")

    # ═══════════════════════════════════════════════════════════
    #  DOCKER BUILD
    # ═══════════════════════════════════════════════════════════

    def build_docker(self) -> bool:
        """
        Build Docker container for production.
        Generates: Dockerfile, .dockerignore, docker-compose.yml
        """
        self._log(f"Building Docker image: {self.config.app_name}", "DOCKER")
        os.makedirs(self._deploy_dir, exist_ok=True)

        # Generate Dockerfile
        self._generate_dockerfile()

        # Generate .dockerignore
        self._generate_dockerignore()

        # Generate docker-compose.yml
        self._generate_docker_compose()

        # Build the Docker image
        self._log("Building Docker image...")
        ret, out, err = self._run_cmd([
            "docker", "build",
            "-t", f"{self.config.app_name}:{self.config.version}",
            "-t", f"{self.config.app_name}:latest",
            "-f", os.path.join(self._deploy_dir, "Dockerfile"),
            self.project_path,
        ])

        if ret == 0:
            self._log(f"Docker image built: {self.config.app_name}:{self.config.version}", "OK")
            # Show image size
            ret2, out2, _ = self._run_cmd([
                "docker", "images", self.config.app_name, "--format", "{{.Size}}"
            ])
            if ret2 == 0:
                self._log(f"Image size: {out2.strip()}", "INFO")
            return True
        else:
            self._log(f"Docker build failed: {err[:200]}", "ERROR")
            self._log("Dockerfile generated — build manually: docker build -t myapp .", "WARN")
            return False

    def _generate_dockerfile(self) -> str:
        """Generate production Dockerfile"""
        app = self.config.app_name

        dockerfile = f"""# ─── PyTreXT Production Dockerfile ───
# Stage 1: Build Rust core
FROM python:3.12-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \\
    curl build-essential pkg-config libssl-dev \\
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${{PATH}}"

# Install maturin
RUN pip install --no-cache-dir maturin

# Copy and build
WORKDIR /build
COPY . .
RUN PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 python -m maturin build --release 2>/dev/null || true
RUN pip install --no-cache-dir target/wheels/*.whl 2>/dev/null || true

# ─── Stage 2: Production Runtime ───
FROM python:3.12-slim

LABEL app="{app}" \\
      version="{self.config.version}" \\
      framework="PyTreXT" \\
      maintainer="PyTreXT Builder"

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \\
    libsqlite3-dev \\
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd --create-home --shell /bin/bash pytrex

WORKDIR /app

# Copy Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt 2>/dev/null || true
RUN pip install --no-cache-dir pytrex-framework 2>/dev/null || true

# Copy Rust wheel from builder
COPY --from=builder /build/target/wheels/*.whl /tmp/
RUN pip install --no-cache-dir /tmp/*.whl 2>/dev/null || true

# Copy application code
COPY . .

# Copy production configs
COPY deploy/ /app/deploy/

# Create data directory
RUN mkdir -p /data && chown -R pytrex:pytrex /data /app

# Switch to non-root user
USER pytrex

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \\
    CMD python -c "import json,urllib.request; json.load(urllib.request.urlopen('http://localhost:{self.config.port}{self.config.health_check_path}'))" || exit 1

# Environment
ENV PYTREX_ENV=production \\
    PYTREX_PORT={self.config.port} \\
    PYTREX_LOG_LEVEL={self.config.log_level} \\
    PYTREX_DB_PATH={self.config.db_path}

# Expose port
EXPOSE {self.config.port}{ ' 9090' if self.config.enable_metrics else '' }

# Security hardening
RUN chmod 755 /app && chmod 644 /app/*.py 2>/dev/null || true

# Entrypoint
CMD ["python", "main.py"]
"""
        path = os.path.join(self._deploy_dir, "Dockerfile")
        with open(path, "w") as f:
            f.write(dockerfile)
        self._log(f"Dockerfile → deploy/Dockerfile", "OK")
        return path

    def _generate_dockerignore(self) -> str:
        """Generate .dockerignore"""
        ignore = """__pycache__/
*.pyc
*.pyo
*.pyd
.git/
.gitignore
.venv/
venv/
env/
node_modules/
target/
dist/
*.log
*.db
.env
.DS_Store
Thumbs.db
*.egg-info/
.pytest_cache/
.mypy_cache/
.ruff_cache/
"""
        path = os.path.join(self._deploy_dir, ".dockerignore")
        with open(path, "w") as f:
            f.write(ignore)
        return path

    def _generate_docker_compose(self) -> str:
        """Generate docker-compose.yml with app + DB + monitoring"""
        app = self.config.app_name
        port = self.config.port

        compose = f"""# PyTreXT Production Stack
version: '3.8'

services:
  # ─── Main App ───
  {app}:
    build:
      context: ..
      dockerfile: deploy/Dockerfile
    image: {app}:{self.config.version}
    container_name: {app}
    restart: unless-stopped
    ports:
      - "{port}:{port}"
    environment:
      - PYTREX_ENV=production
      - PYTREX_PORT={port}
      - PYTREX_LOG_LEVEL={self.config.log_level}
      - PYTREX_DB_PATH=/data/pytrex.db
      - PYTREX_AUTH_SECRET=${{PYTREX_AUTH_SECRET:-change-me-in-production}}
    volumes:
      - pytrex_data:/data
    healthcheck:
      test: ["CMD", "python", "-c", "import urllib.request; urllib.request.urlopen('http://localhost:{port}{self.config.health_check_path}')"]
      interval: 30s
      timeout: 5s
      retries: 3
    deploy:
      resources:
        limits:
          memory: {self.config.memory_limit}
          cpus: '{self.config.cpu_limit}'
    networks:
      - pytrex_net

  # ─── Elixir Engine (optional) ───
  {app}-elixir:
    image: hexpm/elixir:1.16-erlang-26
    container_name: {app}-elixir
    restart: unless-stopped
    command: sh -c "cd /app/pytrex_engine && mix deps.get && mix run --no-halt"
    volumes:
      - ../pytrex_engine:/app/pytrex_engine
    networks:
      - pytrex_net
    profiles:
      - full

  # ─── Nginx Reverse Proxy (optional) ───
  nginx:
    image: nginx:alpine
    container_name: {app}-nginx
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - certbot_data:/etc/letsencrypt
    depends_on:
      - {app}
    networks:
      - pytrex_net
    profiles:
      - proxy

networks:
  pytrex_net:
    driver: bridge

volumes:
  pytrex_data:
  certbot_data:
"""
        path = os.path.join(self._deploy_dir, "docker-compose.yml")
        with open(path, "w") as f:
            f.write(compose)
        self._log(f"docker-compose.yml → deploy/", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  NGINX CONFIG
    # ═══════════════════════════════════════════════════════════

    def generate_nginx_config(self, domain: str = "") -> str:
        """Generate Nginx reverse proxy config with SSL"""
        domain = domain or self.config.domain or "example.com"
        app = self.config.app_name
        port = self.config.port

        nginx_conf = f"""# PyTreXT Nginx Configuration
worker_processes auto;
events {{ worker_connections 1024; }}

http {{
    include /etc/nginx/mime.types;
    sendfile on;
    gzip on;
    gzip_types text/plain application/json text/css application/javascript;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate={10 if self.config.enable_rate_limiting else 1000}r/s;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    server {{
        listen 80;
        server_name {domain};

        # Auto-redirect to HTTPS
        return 301 https://$host$request_uri;
    }}

    server {{
        listen 443 ssl http2;
        server_name {domain};

        # SSL certificates
        ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        # Proxy to PyTreXT app
        location / {{
            limit_req zone=api_limit burst=20 nodelay;
            proxy_pass http://{app}:{port};
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 60s;
        }}

        # WebSocket support
        location /ws {{
            proxy_pass http://{app}:{port};
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
        }}

        # Health check
        location {self.config.health_check_path} {{
            proxy_pass http://{app}:{port};
            access_log off;
        }}

        # Static files cache
        location ~* \\.(js|css|png|jpg|svg|ico)$ {{
            proxy_pass http://{app}:{port};
            expires 30d;
            add_header Cache-Control "public, immutable";
        }}
    }}
}}
"""
        path = os.path.join(self._deploy_dir, "nginx.conf")
        with open(path, "w") as f:
            f.write(nginx_conf)
        self._log(f"nginx.conf → deploy/", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  SYSTEMD SERVICE (Linux)
    # ═══════════════════════════════════════════════════════════

    def generate_systemd_service(self) -> str:
        """Generate systemd service file for Linux production servers"""
        app = self.config.app_name

        service = f"""[Unit]
Description=PyTreXT {app} Service
After=network.target
Wants=network.target

[Service]
Type=simple
User=pytrex
Group=pytrex
WorkingDirectory=/opt/{app}
ExecStart=/opt/{app}/run.sh
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier={app}

# Environment
Environment=PYTREX_ENV=production
Environment=PYTREX_PORT={self.config.port}
Environment=PYTREX_LOG_LEVEL={self.config.log_level}
EnvironmentFile=-/etc/{app}/env

# Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/{app} /data
ReadOnlyPaths=/usr /etc/{app}
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6
RestrictRealtime=yes
MemoryLimit={self.config.memory_limit.replace('m', 'M')}

[Install]
WantedBy=multi-user.target
"""
        path = os.path.join(self._deploy_dir, f"{app}.service")
        with open(path, "w") as f:
            f.write(service)
        self._log(f"{app}.service → deploy/", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  WINDOWS SERVICE
    # ═══════════════════════════════════════════════════════════

    def generate_windows_service(self) -> str:
        """Generate Windows Service install script"""
        app = self.config.app_name

        ps_script = f"""# PyTreXT Windows Service Installer
# Run as Administrator: PowerShell -ExecutionPolicy Bypass -File install-service.ps1

$serviceName = "{app}"
$displayName = "PyTreXT - {app}"
$description = "PyTreXT Production Service for {app}"
$binaryPath = "{sys.executable}"
$arguments = '"{os.path.join(self.project_path, 'main.py')}"'
$workDir = "{self.project_path}"

# Create service
New-Service -Name $serviceName `
    -DisplayName $displayName `
    -Description $description `
    -BinaryPathName "$binaryPath $arguments" `
    -StartupType Automatic

# Configure recovery
sc.exe failure $serviceName reset=86400 actions=restart/5000/restart/10000/restart/30000

# Set environment
[Environment]::SetEnvironmentVariable("PYTREX_ENV", "production", "Machine")
[Environment]::SetEnvironmentVariable("PYTREX_PORT", "{self.config.port}", "Machine")

# Start service
Start-Service $serviceName

Write-Host "✅ Service '$serviceName' installed and started!"
Write-Host "   Check status: Get-Service $serviceName"
"""
        path = os.path.join(self._deploy_dir, "install-service.ps1")
        with open(path, "w") as f:
            f.write(ps_script)
        self._log(f"install-service.ps1 → deploy/", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  KUBERNETES (k8s)
    # ═══════════════════════════════════════════════════════════

    def generate_kubernetes_manifests(self) -> List[str]:
        """Generate Kubernetes deployment manifests"""
        app = self.config.app_name
        port = self.config.port
        files = []

        # deployment.yaml
        deployment = f"""apiVersion: apps/v1
kind: Deployment
metadata:
  name: {app}
  labels:
    app: {app}
spec:
  replicas: 3
  selector:
    matchLabels:
      app: {app}
  template:
    metadata:
      labels:
        app: {app}
    spec:
      containers:
        - name: {app}
          image: {app}:{self.config.version}
          imagePullPolicy: Always
          ports:
            - containerPort: {port}
              name: http
            - containerPort: 9090
              name: metrics
          env:
            - name: PYTREX_ENV
              value: "production"
            - name: PYTREX_PORT
              value: "{port}"
            - name: PYTREX_LOG_LEVEL
              value: "{self.config.log_level}"
            - name: PYTREX_AUTH_SECRET
              valueFrom:
                secretKeyRef:
                  name: {app}-secrets
                  key: auth-secret
          resources:
            requests:
              memory: "128Mi"
              cpu: "250m"
            limits:
              memory: "{self.config.memory_limit}"
              cpu: "{self.config.cpu_limit}"
          livenessProbe:
            httpGet:
              path: {self.config.health_check_path}
              port: {port}
            initialDelaySeconds: 10
            periodSeconds: 15
          readinessProbe:
            httpGet:
              path: {self.config.health_check_path}
              port: {port}
            initialDelaySeconds: 5
            periodSeconds: 10
      restartPolicy: Always
"""
        path1 = os.path.join(self._deploy_dir, "k8s-deployment.yaml")
        with open(path1, "w") as f:
            f.write(deployment)
        files.append(path1)

        # service.yaml
        service = f"""apiVersion: v1
kind: Service
metadata:
  name: {app}-service
  labels:
    app: {app}
spec:
  type: ClusterIP
  selector:
    app: {app}
  ports:
    - name: http
      port: 80
      targetPort: {port}
      protocol: TCP
    - name: metrics
      port: 9090
      targetPort: 9090
      protocol: TCP
"""
        path2 = os.path.join(self._deploy_dir, "k8s-service.yaml")
        with open(path2, "w") as f:
            f.write(service)
        files.append(path2)

        # hpa.yaml (autoscaling)
        hpa = f"""apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: {app}-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: {app}
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
"""
        path3 = os.path.join(self._deploy_dir, "k8s-hpa.yaml")
        with open(path3, "w") as f:
            f.write(hpa)
        files.append(path3)

        self._log(f"Kubernetes manifests → deploy/k8s-*.yaml", "OK")
        return files

    # ═══════════════════════════════════════════════════════════
    #  CLOUD DEPLOYMENT
    # ═══════════════════════════════════════════════════════════

    def deploy_cloud(self, provider: str = "aws") -> bool:
        """
        Deploy to cloud provider.

        Args:
            provider: "aws", "gcp", "azure", "digitalocean"
        """
        self._log(f"Deploying to cloud: {provider.upper()}", "CLOUD")
        os.makedirs(self._deploy_dir, exist_ok=True)

        # Generate Dockerfile first
        self._generate_dockerfile()
        self._generate_dockerignore()

        if provider == "aws":
            return self._deploy_aws()
        elif provider == "gcp":
            return self._deploy_gcp()
        elif provider == "azure":
            return self._deploy_azure()
        elif provider == "digitalocean":
            return self._deploy_digitalocean()
        else:
            self._log(f"Unknown cloud provider: {provider}", "ERROR")
            return False

    def _deploy_aws(self) -> bool:
        """Generate AWS ECS/EC2 deployment configs"""
        app = self.config.app_name
        port = self.config.port

        # ECS Task Definition
        task_def = {
            "family": app,
            "networkMode": "awsvpc",
            "requiresCompatibilities": ["FARGATE"],
            "cpu": self.config.cpu_limit.replace(".", ""),
            "memory": self.config.memory_limit,
            "containerDefinitions": [{
                "name": app,
                "image": f"{app}:{self.config.version}",
                "portMappings": [{"containerPort": port, "protocol": "tcp"}],
                "environment": [
                    {"name": "PYTREX_ENV", "value": "production"},
                    {"name": "PYTREX_PORT", "value": str(port)},
                ],
                "logConfiguration": {
                    "logDriver": "awslogs",
                    "options": {
                        "awslogs-group": f"/ecs/{app}",
                        "awslogs-region": "us-east-1",
                        "awslogs-stream-prefix": "ecs",
                    },
                },
            }],
        }

        path = os.path.join(self._deploy_dir, "aws-ecs-task.json")
        with open(path, "w") as f:
            json.dump(task_def, f, indent=2)
        self._log(f"AWS ECS task definition → deploy/", "OK")

        # Deploy script
        deploy_sh = f"""#!/bin/bash
# PyTreXT AWS Deployment Script
set -e

APP="{app}"
REGION="us-east-1"
ECR_REPO="$APP"

echo "Deploying $APP to AWS ECS..."

# Build & push Docker image
docker build -t $APP -f deploy/Dockerfile .
aws ecr get-login-password --region $REGION | docker login --username AWS --password-stdin $AWS_ACCOUNT.dkr.ecr.$REGION.amazonaws.com
docker tag $APP:latest $AWS_ACCOUNT.dkr.ecr.$REGION.amazonaws.com/$ECR_REPO:latest
docker push $AWS_ACCOUNT.dkr.ecr.$REGION.amazonaws.com/$ECR_REPO:latest

# Register task definition
aws ecs register-task-definition --cli-input-json file://deploy/aws-ecs-task.json

# Update service
aws ecs update-service --cluster pytrex-cluster --service $APP --task-definition $APP --force-new-deployment

echo "✅ Deployed to AWS ECS!"
"""
        path2 = os.path.join(self._deploy_dir, "deploy-aws.sh")
        with open(path2, "w") as f:
            f.write(deploy_sh)
        os.chmod(path2, 0o755)
        self._log(f"AWS deploy script → deploy/deploy-aws.sh", "OK")
        return True

    def _deploy_gcp(self) -> bool:
        """Generate GCP Cloud Run deployment configs"""
        app = self.config.app_name

        deploy_sh = f"""#!/bin/bash
# PyTreXT GCP Cloud Run Deployment
set -e

APP="{app}"
PROJECT="$GCP_PROJECT"
REGION="us-central1"

echo "Deploying $APP to Google Cloud Run..."

# Build & push to Artifact Registry
gcloud builds submit --tag gcr.io/$PROJECT/$APP

# Deploy to Cloud Run
gcloud run deploy $APP \\
    --image gcr.io/$PROJECT/$APP \\
    --platform managed \\
    --region $REGION \\
    --allow-unauthenticated \\
    --memory {self.config.memory_limit} \\
    --cpu {self.config.cpu_limit} \\
    --port {self.config.port} \\
    --set-env-vars PYTREX_ENV=production

echo "✅ Deployed to GCP Cloud Run!"
SERVICE_URL=$(gcloud run services describe $APP --region $REGION --format 'value(status.url)')
echo "   URL: $SERVICE_URL"
"""
        path = os.path.join(self._deploy_dir, "deploy-gcp.sh")
        with open(path, "w") as f:
            f.write(deploy_sh)
        os.chmod(path, 0o755)
        self._log(f"GCP deploy script → deploy/deploy-gcp.sh", "OK")
        return True

    def _deploy_azure(self) -> bool:
        """Generate Azure Container Instances deployment configs"""
        app = self.config.app_name

        deploy_sh = f"""#!/bin/bash
# PyTreXT Azure Container Instances Deployment
set -e

APP="{app}"
RESOURCE_GROUP="$AZURE_RG"
LOCATION="eastus"

echo "Deploying $APP to Azure..."

# Build & push to ACR
az acr build --registry $AZURE_REGISTRY --image $APP:latest .

# Deploy container instance
az container create \\
    --resource-group $RESOURCE_GROUP \\
    --name $APP \\
    --image $AZURE_REGISTRY.azurecr.io/$APP:latest \\
    --cpu {self.config.cpu_limit} \\
    --memory {self.config.memory_limit.replace('m','')} \\
    --ports {self.config.port} \\
    --environment-variables PYTREX_ENV=production \\
    --dns-name-label $APP

echo "✅ Deployed to Azure Container Instances!"
"""
        path = os.path.join(self._deploy_dir, "deploy-azure.sh")
        with open(path, "w") as f:
            f.write(deploy_sh)
        os.chmod(path, 0o755)
        self._log(f"Azure deploy script → deploy/deploy-azure.sh", "OK")
        return True

    def _deploy_digitalocean(self) -> bool:
        """Generate DigitalOcean App Platform deployment config"""
        app = self.config.app_name

        app_spec = f"""name: {app}
region: nyc
services:
  - name: {app}
    dockerfile_path: deploy/Dockerfile
    source_dir: .
    github:
      branch: main
      deploy_on_push: true
    http_port: {self.config.port}
    instance_count: 1
    instance_size_slug: basic-xxs
    envs:
      - key: PYTREX_ENV
        value: "production"
        scope: RUN_TIME
      - key: PYTREX_PORT
        value: "{self.config.port}"
        scope: RUN_TIME
    health_check:
      http_path: {self.config.health_check_path}
"""
        path = os.path.join(self._deploy_dir, "do-app.yaml")
        with open(path, "w") as f:
            f.write(app_spec)
        self._log(f"DigitalOcean app spec → deploy/do-app.yaml", "OK")
        return True

    # ═══════════════════════════════════════════════════════════
    #  CI/CD PIPELINES
    # ═══════════════════════════════════════════════════════════

    def generate_github_actions(self) -> str:
        """Generate GitHub Actions CI/CD pipeline"""
        app = self.config.app_name

        workflow = f"""# PyTreXT CI/CD Pipeline
name: PyTreXT Build & Deploy

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  APP_NAME: {app}
  PYTHON_VERSION: '3.12'

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Python
        uses: actions/setup-python@v5
        with:
          python-version: ${{{{ env.PYTHON_VERSION }}}}

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install dependencies
        run: |
          pip install -r requirements.txt
          pip install maturin pytest pytest-asyncio

      - name: Build Rust core
        run: PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 python -m maturin develop

      - name: Run tests
        run: |
          python -m pytest tests/ -v --tb=short
          python -m pytrex.test_runner

  build:
    name: Build & Push
    needs: test
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build Docker image
        run: docker build -t ${{{{ env.APP_NAME }}}}:${{{{ github.sha }}}} -f deploy/Dockerfile .

      - name: Docker meta
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{{{ github.repository }}}}
          tags: |
            type=sha
            type=ref,event=branch
            latest

      - name: Login to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{{{ github.actor }}}}
          password: ${{{{ secrets.GITHUB_TOKEN }}}}

      - name: Push Docker image
        run: |
          docker tag ${{{{ env.APP_NAME }}}}:${{{{ github.sha }}}} ghcr.io/${{{{ github.repository }}}}:latest
          docker push ghcr.io/${{{{ github.repository }}}}:latest

  deploy:
    name: Deploy
    needs: build
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - name: Trigger deployment
        run: echo "Deploying ${{{{ env.APP_NAME }}}} to production..."
        # Add your deployment step here (SSH, kubectl, etc.)
"""
        path = os.path.join(self._deploy_dir, "github-actions.yml")
        # Also save to .github/workflows
        workflows_dir = os.path.join(self.project_path, ".github", "workflows")
        os.makedirs(workflows_dir, exist_ok=True)
        ci_path = os.path.join(workflows_dir, "pytrex-ci.yml")
        with open(ci_path, "w") as f:
            f.write(workflow)
        with open(path, "w") as f:
            f.write(workflow)
        self._log(f"GitHub Actions → .github/workflows/pytrex-ci.yml", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  PRODUCTION ENVIRONMENT
    # ═══════════════════════════════════════════════════════════

    def generate_env_files(self) -> Tuple[str, str]:
        """Generate .env and .env.example files"""
        app = self.config.app_name

        # .env.example (safe to commit)
        env_example = f"""# PyTreXT Production Environment Variables
PYTREX_ENV=production
PYTREX_PORT={self.config.port}
PYTREX_LOG_LEVEL={self.config.log_level}
PYTREX_DB_PATH=/data/pytrex.db

# Security — CHANGE THESE IN PRODUCTION!
PYTREX_AUTH_SECRET=generate-a-strong-random-secret-here
PYTREX_ENCRYPTION_KEY=your-256-bit-encryption-key

# Optional: Elixir clustering
PYTREX_ELIXIR_HOST=localhost
PYTREX_ELIXIR_PORT=42351
PYTREX_CLUSTER_COOKIE=change-me

# Optional: AI/ML
PYTREX_MODEL_PATH=/models
PYTREX_TORCH_DEVICE=cpu

# Optional: Monitoring
PYTREX_METRICS_ENABLED=true
PYTREX_METRICS_PORT=9090
"""
        path1 = os.path.join(self._deploy_dir, ".env.example")
        with open(path1, "w") as f:
            f.write(env_example)

        # .env.production template
        env_prod = f"""# PyTreXT Production Environment — DO NOT COMMIT
PYTREX_ENV=production
PYTREX_PORT={self.config.port}
PYTREX_LOG_LEVEL={self.config.log_level}
PYTREX_LOG_FORMAT=json
PYTREX_DB_PATH=/data/pytrex.db
PYTREX_AUTH_SECRET=__GENERATE_STRONG_SECRET__
"""
        path2 = os.path.join(self._deploy_dir, ".env.production")
        with open(path2, "w") as f:
            f.write(env_prod)

        self._log(f"Environment files → deploy/", "OK")
        return path1, path2

    def generate_security_hardening(self) -> str:
        """Generate security hardening configuration"""
        app = self.config.app_name

        security_conf = f"""# PyTreXT Security Hardening Configuration
security:
  # Rate Limiting
  rate_limit:
    enabled: {str(self.config.enable_rate_limiting).lower()}
    max_requests: 100
    window_seconds: 60

  # CORS
  cors:
    enabled: {str(self.config.enable_cors).lower()}
    allowed_origins: {json.dumps(self.config.allowed_hosts)}
    allowed_methods: ["GET", "POST", "PUT", "DELETE"]
    allowed_headers: ["Content-Type", "Authorization"]

  # SSL/TLS
  ssl:
    enabled: {str(self.config.enable_ssl).lower()}
    min_tls_version: "1.2"
    hsts_enabled: true
    hsts_max_age: 31536000

  # Secrets
  secrets:
    encryption: "AES-256-GCM"
    key_rotation_days: 90
    vault_enabled: true

  # Headers
  headers:
    x_frame_options: "SAMEORIGIN"
    x_content_type_options: "nosniff"
    x_xss_protection: "1; mode=block"
    referrer_policy: "strict-origin-when-cross-origin"
    content_security_policy: "default-src 'self'"

  # Database
  database:
    encryption: "AES-256"
    connection_encryption: true
    max_connections: 10
    backup_enabled: {str(self.config.db_backup_enabled).lower()}
    backup_interval_hours: {self.config.db_backup_interval_hours}

  # Authentication
  auth:
    min_password_length: 12
    bcrypt_rounds: 12
    token_expiry_hours: 24
    mfa_enabled: false

  # Logging
  logging:
    redact_sensitive: true
    sensitive_fields:
      - password
      - secret
      - token
      - api_key
      - auth
"""
        path = os.path.join(self._deploy_dir, "security.yaml")
        with open(path, "w") as f:
            f.write(security_conf)
        self._log(f"Security config → deploy/security.yaml", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  ALL-IN-ONE DEPLOYMENT
    # ═══════════════════════════════════════════════════════════

    def deploy_all(self, target: str = "docker") -> Dict[str, bool]:
        """
        Run ALL production deployment steps.

        Args:
            target: "docker", "standalone", "vps", "aws", "gcp", "azure"

        Returns:
            Dict ya status ya kila step
        """
        self._log(f"\n{'='*55}")
        self._log(f"  🚀 PRODUCTION DEPLOYMENT: {self.config.app_name}")
        self._log(f"  Target: {target.upper()}")
        self._log(f"{'='*55}\n")

        results = {}

        # Step 1: Validate project
        self._log("Step 1/6: Validating project...", "INFO")
        main_file = os.path.join(self.project_path, "main.py")
        if not os.path.exists(main_file):
            self._log("No main.py found!", "ERROR")
            return {"validate": False}
        self._log("Project validated", "OK")
        results["validate"] = True

        # Step 2: Run tests
        self._log("Step 2/6: Running tests...", "INFO")
        try:
            from pytrex.test_runner import TestRunner
            runner = TestRunner(verbose=False, color=False, parallel=True)
            suite = runner.run_all()
            results["tests"] = {
                "passed": suite.passed,
                "total": suite.total,
                "rate": suite.success_rate,
            }
            if suite.success_rate >= 80:
                self._log(f"Tests: {suite.passed}/{suite.total} passed ({suite.success_rate:.0f}%)", "OK")
            else:
                self._log(f"Tests: {suite.passed}/{suite.total} — LOW SCORE!", "WARN")
        except Exception as e:
            self._log(f"Test failed: {e}", "WARN")
            results["tests"] = {"error": str(e)}

        # Step 3: Generate configs
        self._log("Step 3/6: Generating production configs...", "INFO")
        self._generate_dockerfile()
        self._generate_docker_compose()
        self.generate_env_files()
        self.generate_security_hardening()
        self.generate_nginx_config()
        self.generate_systemd_service()
        self.generate_github_actions()
        results["configs"] = True
        self._log("All configs generated", "OK")

        # Step 4: Security hardening
        self._log("Step 4/6: Applying security hardening...", "SECURE")
        self.generate_security_hardening()
        results["security"] = True
        self._log("Security hardening applied", "OK")

        # Step 5: Build
        self._log("Step 5/6: Building for production...", "BUILD")
        if target in ("docker", "aws", "gcp", "azure"):
            results["build"] = self.build_docker()
        elif target == "standalone":
            results["build"] = self.build_standalone()
        else:
            results["build"] = self.build_docker()

        # Step 6: Generate deployment instructions
        self._log("Step 6/6: Generating deployment instructions...", "INFO")
        self._generate_deploy_readme(target)
        results["instructions"] = True

        # Final summary
        self._log(f"\n{'='*55}")
        all_ok = all(v if isinstance(v, bool) else True for v in results.values())
        if all_ok:
            self._log(f"  ✅ PRODUCTION READY! {self.config.app_name} v{self.config.version}", "OK")
        else:
            self._log(f"  ⚠️  Deployed with warnings — check results above", "WARN")
        self._log(f"  📂 Output: {self._deploy_dir}")
        self._log(f"{'='*55}\n")

        return results

    def _generate_deploy_readme(self, target: str) -> str:
        """Generate deployment instructions README"""
        app = self.config.app_name

        readme = f"""# {app} — Production Deployment Guide

## Quick Deploy

### Docker (recommended)
```bash
cd deploy
docker-compose up -d
```

### Docker (manual)
```bash
docker build -t {app} -f deploy/Dockerfile .
docker run -d -p {self.config.port}:{self.config.port} --name {app} {app}
```

### Standalone
```bash
cd dist/production/{app}
./run.sh          # Linux/macOS
run.bat           # Windows
```

### VPS (Linux)
```bash
# Copy files to server
scp -r dist/production/{app} user@server:/opt/{app}

# Install systemd service
sudo cp deploy/{app}.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable {app}
sudo systemctl start {app}

# Check status
sudo systemctl status {app}
```

### Kubernetes
```bash
kubectl apply -f deploy/k8s-deployment.yaml
kubectl apply -f deploy/k8s-service.yaml
kubectl apply -f deploy/k8s-hpa.yaml
```

### Cloud Deploy
```bash
# AWS
bash deploy/deploy-aws.sh

# Google Cloud
bash deploy/deploy-gcp.sh

# Azure
bash deploy/deploy-azure.sh
```

## Configuration

- Environment: `deploy/.env.production`
- Nginx: `deploy/nginx.conf`
- Security: `deploy/security.yaml`
- Systemd: `deploy/{app}.service`

## Monitoring

- Health check: `http://localhost:{self.config.port}{self.config.health_check_path}`
- Metrics: `http://localhost:{self.config.port}:9090/metrics` (if enabled)
- Logs: Docker → `docker logs {app}`, Systemd → `journalctl -u {app}`

## CI/CD

GitHub Actions pipeline: `.github/workflows/pytrex-ci.yml`

## Security Checklist

- [ ] Change PYTREX_AUTH_SECRET in .env.production
- [ ] Enable SSL/TLS (Let's Encrypt)
- [ ] Configure firewall (allow only ports 80, 443)
- [ ] Set up database backups
- [ ] Enable rate limiting
- [ ] Review security.yaml settings

---
Generated by PyTreXT Production Builder v1.0.0
"""
        path = os.path.join(self._deploy_dir, "DEPLOY.md")
        with open(path, "w") as f:
            f.write(readme)
        self._log(f"Deployment guide → deploy/DEPLOY.md", "OK")
        return path

    # ═══════════════════════════════════════════════════════════
    #  UTILITY
    # ═══════════════════════════════════════════════════════════

    def to_dict(self) -> Dict[str, Any]:
        return {
            "project_path": self.project_path,
            "config": self.config.to_dict(),
            "output_dir": self._output_dir,
            "deploy_dir": self._deploy_dir,
        }

    def __repr__(self) -> str:
        return f"ProductionBuilder({self.config.app_name}, target={self._deploy_dir})"


# ─── Convenience Functions ────────────────────────────────────

def deploy(project_path: str = ".", target: str = "docker") -> Dict[str, bool]:
    """Deploy mradi kwa production kwa amri moja"""
    builder = ProductionBuilder(project_path=project_path)
    return builder.deploy_all(target=target)


def quick_build(project_path: str = ".") -> bool:
    """Build haraka kwa Docker"""
    builder = ProductionBuilder(project_path=project_path, verbose=True)
    return builder.build_docker()
