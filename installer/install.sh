#!/usr/bin/env bash
set -euo pipefail

REPO="engrammic-ai/mcp"
BINARY="engrammic"

echo ""
echo "Engrammic MCP Setup"
echo ""

# Detect OS and architecture
detect_target() {
    local os arch

    case "$(uname -s)" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="pc-windows-msvc" ;;
        *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

TARGET=$(detect_target)
echo "Detected: $TARGET"

# Get latest release URL
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/${BINARY}-${TARGET}"

# Download to temp
TMPDIR="${TMPDIR:-/tmp}"
INSTALLER="$TMPDIR/$BINARY"

echo "Downloading installer..."
if command -v curl &> /dev/null; then
    curl -fsSL "$RELEASE_URL" -o "$INSTALLER"
elif command -v wget &> /dev/null; then
    wget -q "$RELEASE_URL" -O "$INSTALLER"
else
    echo "Error: Need curl or wget" >&2
    exit 1
fi

chmod +x "$INSTALLER"

# Run installer, passing through all arguments
exec "$INSTALLER" "$@"
