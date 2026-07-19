# ─── PyTreXT Production Dockerfile ───
# Stage 1: Build Rust core
FROM python:3.12-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install maturin
RUN pip install --no-cache-dir maturin

# Copy and build
WORKDIR /build
COPY . .
RUN PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 python -m maturin build --release 2>/dev/null || true
RUN pip install --no-cache-dir target/wheels/*.whl 2>/dev/null || true

# ─── Stage 2: Production Runtime ───
FROM python:3.12-slim

LABEL app="test" \
      version="1.0.0" \
      framework="PyTreXT" \
      maintainer="PyTreXT Builder"

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev \
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
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD python -c "import json,urllib.request; json.load(urllib.request.urlopen('http://localhost:9000/health'))" || exit 1

# Environment
ENV PYTREX_ENV=production \
    PYTREX_PORT=9000 \
    PYTREX_LOG_LEVEL=INFO \
    PYTREX_DB_PATH=/data/pytrex.db

# Expose port
EXPOSE 9000 9090

# Security hardening
RUN chmod 755 /app && chmod 644 /app/*.py 2>/dev/null || true

# Entrypoint
CMD ["python", "main.py"]
