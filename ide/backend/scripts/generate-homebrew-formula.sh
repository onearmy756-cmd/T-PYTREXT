#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-v0.1.0}"
REPO="https://github.com/Lumio-Research/hermes-agent-rs"
FORMULA_DIR="Formula"
FORMULA_FILE="${FORMULA_DIR}/fundi.rb"

mkdir -p "${FORMULA_DIR}"

cat > "${FORMULA_FILE}" <<EOF
class Fundi < Formula
  desc "Fundi autonomous AI agent"
  homepage "${REPO}"
  version "${VERSION#v}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "${REPO}/releases/download/${VERSION}/fundi-macos-aarch64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    else
      url "${REPO}/releases/download/${VERSION}/fundi-macos-x86_64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "${REPO}/releases/download/${VERSION}/fundi-linux-aarch64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    else
      url "${REPO}/releases/download/${VERSION}/fundi-linux-x86_64.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  def install
    bin.install "fundi"
  end

  test do
    system "#{bin}/fundi", "--version"
  end
end
EOF

echo "Generated ${FORMULA_FILE} for ${VERSION}"
