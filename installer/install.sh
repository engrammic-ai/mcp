#!/usr/bin/env sh
# install.sh — Engrammic installer (rustup-style)
# Usage: curl -fsSL https://get.engrammic.ai/install.sh | sh
#        curl -fsSL https://get.engrammic.ai/install.sh | sh -s -- -y --tool cursor
# Env:   ENGRAMMIC_NO_MODIFY_PATH=1  (same as --no-modify-path)
set -eu

REPO="engrammic-ai/mcp"
BINARY="engrammic"
INSTALL_DIR="${HOME}/.local/bin"
RELEASE_BASE="https://github.com/${REPO}/releases/latest/download"

# ── helpers ──────────────────────────────────────────────────────────────────

say()  { printf '\033[1;32m=>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
err()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ── argument parsing ──────────────────────────────────────────────────────────
# Collect flags we handle here; everything else is forwarded to the binary.

NO_MODIFY_PATH="${ENGRAMMIC_NO_MODIFY_PATH:-0}"
FORWARD_ARGS=""

for arg in "$@"; do
    case "$arg" in
        --no-modify-path) NO_MODIFY_PATH=1 ;;
        *)                FORWARD_ARGS="${FORWARD_ARGS} ${arg}" ;;
    esac
done

# ── MSYS / Git-Bash guard ─────────────────────────────────────────────────────

case "$(uname -s 2>/dev/null)" in
    MINGW*|MSYS*|CYGWIN*)
        err "Windows detected. Please use the PowerShell installer instead:
  $(printf '\033[36m')Invoke-Expression (Invoke-WebRequest -Uri https://get.engrammic.ai/install.ps1 -UseBasicParsing).Content$(printf '\033[0m')"
        ;;
esac

# ── OS / arch detection ───────────────────────────────────────────────────────
# Note: detect_target is called in $(...) command substitution (a subshell),
# so `local` is not used — plain assignments are subshell-scoped already.

detect_target() {
    os=""
    arch=""

    case "$(uname -s)" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        *)
            printf '\033[1;31merror:\033[0m Unsupported OS: %s\n  → Please open an issue: https://github.com/%s/issues\n' \
                "$(uname -s)" "${REPO}" >&2
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        arm64|aarch64)  arch="aarch64" ;;
        *)
            printf '\033[1;31merror:\033[0m Unsupported architecture: %s\n  → Pre-built binaries are available for x86_64 and aarch64 only.\n  → To request support: https://github.com/%s/issues\n' \
                "$(uname -m)" "${REPO}" >&2
            exit 1
            ;;
    esac

    printf '%s-%s' "$arch" "$os"
}

TARGET=$(detect_target)
say "Detected platform: ${TARGET}"

# ── downloader detection ──────────────────────────────────────────────────────

if command -v curl >/dev/null 2>&1; then
    download() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    download() { wget -q "$1" -O "$2"; }
else
    err "Neither curl nor wget is available.
  → On Debian/Ubuntu: sudo apt-get install curl
  → On Fedora/RHEL:   sudo dnf install curl
  → On macOS:         curl is pre-installed; if missing, install Xcode CLT"
fi

# ── tmp directory — noexec detection ─────────────────────────────────────────

TMPDIR="${TMPDIR:-/tmp}"
# Test whether TMPDIR allows execution by writing and running a no-op script.
# Use if/else rather than bare command + $? so set -e does not abort the script.
_test_exec_file="${TMPDIR}/_engrammic_exec_test_$$"
_exec_ok=1
if printf '#!/bin/sh\n:' > "$_test_exec_file" 2>/dev/null && \
   chmod +x "$_test_exec_file" 2>/dev/null; then
    if "$_test_exec_file" 2>/dev/null; then
        _exec_ok=0
    fi
fi
rm -f "$_test_exec_file" 2>/dev/null || true

if [ "$_exec_ok" -ne 0 ]; then
    warn "\$TMPDIR (${TMPDIR}) is mounted noexec. Falling back to ~/.cache/engrammic/tmp"
    TMPDIR="${HOME}/.cache/engrammic/tmp"
    mkdir -p "$TMPDIR"
fi

TMP_BIN="${TMPDIR}/${BINARY}-${TARGET}-$$"
TMP_SUM="${TMPDIR}/${BINARY}-${TARGET}-$$.sha256"

# ── download binary + checksum ────────────────────────────────────────────────

BIN_URL="${RELEASE_BASE}/${BINARY}-${TARGET}"
SUM_URL="${RELEASE_BASE}/${BINARY}-${TARGET}.sha256"

say "Downloading ${BINARY}-${TARGET}..."
download "$BIN_URL" "$TMP_BIN"

say "Downloading checksum..."
download "$SUM_URL" "$TMP_SUM"

# ── SHA256 verification ───────────────────────────────────────────────────────
# The .sha256 file is in sha256sum two-column format: "<hash>  <filename>"
# We rewrite the filename column to point at our local tmp file, then verify.

say "Verifying checksum..."
_expected_hash=$(awk '{print $1}' "$TMP_SUM")

if command -v sha256sum >/dev/null 2>&1; then
    # GNU coreutils (Linux + many macOS via brew)
    printf '%s  %s\n' "$_expected_hash" "$TMP_BIN" | sha256sum --check --status \
        || err "Checksum mismatch for ${BINARY}-${TARGET}.
  → The download may be corrupt or tampered with.
  → Delete ${TMP_BIN} and retry."
    say "Checksum verified."
elif command -v shasum >/dev/null 2>&1; then
    # macOS built-in
    printf '%s  %s\n' "$_expected_hash" "$TMP_BIN" | shasum -a 256 --check --status \
        || err "Checksum mismatch for ${BINARY}-${TARGET}.
  → The download may be corrupt or tampered with.
  → Delete ${TMP_BIN} and retry."
    say "Checksum verified."
else
    warn "No sha256sum or shasum found — skipping checksum verification.
  → Install coreutils for verification on future runs."
fi

rm -f "$TMP_SUM"

# ── install binary to ~/.local/bin ────────────────────────────────────────────

mkdir -p "$INSTALL_DIR"
chmod +x "$TMP_BIN"
mv "$TMP_BIN" "${INSTALL_DIR}/${BINARY}"

say "Installed to ${INSTALL_DIR}/${BINARY}"

# ── PATH update ───────────────────────────────────────────────────────────────

EXPORT_LINE="export PATH=\"\$HOME/.local/bin:\$PATH\""

# Detect shell rc
detect_rc() {
    case "${SHELL:-}" in
        */fish) printf '%s/.config/fish/config.fish' "$HOME" ;;
        */zsh)  printf '%s/.zshrc' "$HOME" ;;
        *)      printf '%s/.bashrc' "$HOME" ;;  # bash or unknown
    esac
}

RC_FILE=$(detect_rc)

# Check whether PATH already contains the dir (skip mutation if so)
_path_has_local_bin=0
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) _path_has_local_bin=1 ;;
esac

if [ "$_path_has_local_bin" -eq 0 ]; then
    if [ "$NO_MODIFY_PATH" -eq 0 ]; then
        # For fish, ensure the config directory exists before appending
        case "${SHELL:-}" in
            */fish) mkdir -p "${HOME}/.config/fish" ;;
        esac
        printf '\n# Added by Engrammic installer\n%s\n' "$EXPORT_LINE" >> "$RC_FILE"
        say "Added PATH entry to ${RC_FILE}"
    fi

    printf '\n'
    printf '\033[33m=>\033[0m To use engrammic in this shell session, run:\n'
    printf '     %s\n' "$EXPORT_LINE"
    if [ "$NO_MODIFY_PATH" -eq 0 ]; then
        printf '   (Already written to %s for future sessions)\n' "$RC_FILE"
    else
        printf '   (--no-modify-path: rc file was NOT modified)\n'
    fi
    printf '\n'
else
    say "${INSTALL_DIR} is already in PATH"
fi

# ── exec the installed binary ─────────────────────────────────────────────────
# Always exec from the installed location (not the tmp copy).
# Default subcommand when called with no args: install.

INSTALLED_BIN="${INSTALL_DIR}/${BINARY}"

if [ -z "${FORWARD_ARGS}" ]; then
    exec "$INSTALLED_BIN" install
else
    # shellcheck disable=SC2086
    exec "$INSTALLED_BIN" $FORWARD_ARGS
fi
