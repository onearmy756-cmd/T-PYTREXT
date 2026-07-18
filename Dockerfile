FROM python:3.12-slim

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install maturin
RUN pip install maturin

# Copy project
COPY . /app
WORKDIR /app

# Build Rust extension
RUN maturin build --release

# Install the framework
RUN pip install target/wheels/*.whl

# Default command
CMD ["python", "main.py"]
