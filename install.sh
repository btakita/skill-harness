#!/bin/sh
# Install skill-harness
# Usage: curl -fsSL https://raw.githubusercontent.com/btakita/skill-harness/main/install.sh | sh

set -e

REPO="btakita/skill-harness"
BIN="skill-harness"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux) OS="unknown-linux-gnu" ;;
  darwin) OS="apple-darwin" ;;
  mingw*|msys*|cygwin*) OS="pc-windows-msvc" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH}-${OS}"

if [ "$OS" = "pc-windows-msvc" ]; then
  ARCHIVE="$BIN-$TARGET.zip"
else
  ARCHIVE="$BIN-$TARGET.tar.gz"
fi

if [ "$VERSION" = "latest" ]; then
  need curl
  VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')

  if [ -z "$VERSION" ]; then
    echo "Failed to fetch latest release" >&2
    exit 1
  fi
fi

URL="https://github.com/$REPO/releases/download/$VERSION/$ARCHIVE"

echo "Installing $BIN $VERSION ($TARGET)..."

mkdir -p "$INSTALL_DIR"

TMPDIR="${TMPDIR:-/tmp}"
TMPFILE=$(mktemp "$TMPDIR/$BIN.XXXXXX")
trap 'rm -f "$TMPFILE"' EXIT INT TERM

need curl
curl -fsSL "$URL" -o "$TMPFILE"

if [ "$OS" = "pc-windows-msvc" ]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -o "$TMPFILE" -d "$INSTALL_DIR" >/dev/null
  elif command -v powershell.exe >/dev/null 2>&1; then
    PS_TMPFILE="$TMPFILE"
    PS_INSTALL_DIR="$INSTALL_DIR"
    if command -v cygpath >/dev/null 2>&1; then
      PS_TMPFILE=$(cygpath -w "$TMPFILE")
      PS_INSTALL_DIR=$(cygpath -w "$INSTALL_DIR")
    fi
    powershell.exe -NoProfile -Command "Expand-Archive -Force '$PS_TMPFILE' '$PS_INSTALL_DIR'"
  else
    echo "Missing required command: unzip or powershell.exe" >&2
    exit 1
  fi
  chmod +x "$INSTALL_DIR/$BIN.exe" 2>/dev/null || true
else
  need tar
  tar xzf "$TMPFILE" -C "$INSTALL_DIR"
  chmod +x "$INSTALL_DIR/$BIN"
fi

if [ "$OS" = "pc-windows-msvc" ]; then
  echo "Installed $BIN to $INSTALL_DIR/$BIN.exe"
else
  echo "Installed $BIN to $INSTALL_DIR/$BIN"
fi

# Check if in PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
  echo ""
  echo "Add to your PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
  ;;
esac
