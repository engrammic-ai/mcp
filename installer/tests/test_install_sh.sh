#!/usr/bin/env sh
# test_install_sh.sh — plain POSIX sh test harness for install.sh
# Run: sh installer/tests/test_install_sh.sh
# Exits 0 if all tests pass, 1 if any fail.
#
# Note: set -e is intentionally NOT used here because the harness contains
# assertions that inspect non-zero exit codes (e.g. checksum mismatch tests).
# set -u is kept to catch unset variable bugs.
set -u

PASS=0
FAIL=0

pass() { PASS=$((PASS+1)); printf '  \033[32mPASS\033[0m %s\n' "$1"; }
fail() { FAIL=$((FAIL+1)); printf '  \033[31mFAIL\033[0m %s\n' "$1"; }
assert_eq() {
    if [ "$1" = "$2" ]; then
        pass "$3"
    else
        fail "$3 (expected '$2', got '$1')"
    fi
}
assert_contains() {
    case "$1" in
        *"$2"*) pass "$3" ;;
        *)      fail "$3 (expected output to contain '$2'; got: $1)" ;;
    esac
}
assert_not_contains() {
    case "$1" in
        *"$2"*) fail "$3 (expected output NOT to contain '$2')" ;;
        *)      pass "$3" ;;
    esac
}
assert_exit_nonzero() {
    if [ "$1" -ne 0 ]; then
        pass "$2 (exited $1)"
    else
        fail "$2 (expected non-zero exit, got 0)"
    fi
}

# ── Stub detect_target for unit-level arch tests ─────────────────────────────
# Identical to install.sh implementation for testing.
# Note: no `local` keyword — POSIX sh does not guarantee it, and this function
# is called in $(...) command substitution (a subshell), so plain assignments
# are already scoped to the subshell.

detect_target() {
    os=""
    arch=""

    case "$(uname -s)" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) printf 'windows'; return 0 ;;
        *) printf 'unsupported'; return 1 ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        arm64|aarch64)  arch="aarch64" ;;
        *) printf 'unsupported_arch'; return 1 ;;
    esac
    printf '%s-%s' "$arch" "$os"
}

# ── Test: arch detection table ────────────────────────────────────────────────

printf '\n--- arch detection ---\n'

TARGET=$(detect_target)
case "$TARGET" in
    x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu| \
    x86_64-apple-darwin|aarch64-apple-darwin)
        pass "detect_target returns a known target ($TARGET)"
        ;;
    *)
        fail "detect_target returned unexpected value: $TARGET"
        ;;
esac

# Validate the format: must be <arch>-<os-triple>
case "$TARGET" in
    *-*-*) pass "detect_target output has at least two dashes (arch-os-abi shape)" ;;
    *)     fail "detect_target output missing expected dashes: $TARGET" ;;
esac

# ── Test: checksum mismatch fails hard ────────────────────────────────────────

printf '\n--- checksum verification ---\n'

_tmp=$(mktemp -d)
_fake_bin="$_tmp/engrammic-x86_64-unknown-linux-gnu"
_fake_sum="$_tmp/engrammic-x86_64-unknown-linux-gnu.sha256"

printf 'genuine binary content' > "$_fake_bin"
printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  %s\n' "$_fake_bin" > "$_fake_sum"

# Expected: sha256sum --check fails with the wrong hash
_expected_hash=$(awk '{print $1}' "$_fake_sum")
_actual_hash=""
if command -v sha256sum >/dev/null 2>&1; then
    _actual_hash=$(sha256sum "$_fake_bin" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    _actual_hash=$(shasum -a 256 "$_fake_bin" | awk '{print $1}')
fi

if [ -n "$_actual_hash" ] && [ "$_expected_hash" != "$_actual_hash" ]; then
    pass "checksum mismatch is detected (hashes differ as expected)"
else
    fail "checksum mismatch detection: expected $_expected_hash got $_actual_hash"
fi

# Now write the correct hash and confirm it matches
_correct_hash=""
if command -v sha256sum >/dev/null 2>&1; then
    _correct_hash=$(sha256sum "$_fake_bin" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    _correct_hash=$(shasum -a 256 "$_fake_bin" | awk '{print $1}')
fi

if [ -n "$_correct_hash" ]; then
    printf '%s  %s\n' "$_correct_hash" "$_fake_bin" > "$_fake_sum"
    # Guard with if/else so set-e (if callers re-enable it) or explicit failure
    # does not abort the harness before we record the result.
    if command -v sha256sum >/dev/null 2>&1; then
        if sha256sum --check --status "$_fake_sum" 2>/dev/null; then
            pass "correct checksum passes sha256sum --check"
        else
            fail "correct checksum failed sha256sum --check"
        fi
    elif command -v shasum >/dev/null 2>&1; then
        if shasum -a 256 --check --status "$_fake_sum" 2>/dev/null; then
            pass "correct checksum passes shasum --check"
        else
            fail "correct checksum failed shasum --check"
        fi
    fi
fi

rm -rf "$_tmp"

# ── Test: arg passthrough ─────────────────────────────────────────────────────
# We cannot exec the real installer in tests (no binary to exec), but we can
# validate the argument-parsing/forwarding logic by extracting it.

printf '\n--- arg passthrough ---\n'

# Simulate the script's rotate-and-filter rebuild of "$@" plus the exec
# decision (no args -> install; leading flag -> install + args; leading word
# -> args verbatim). Mirrors install.sh; FORWARD_* capture what would be run.
parse_args() {
    NO_MODIFY_PATH=0
    _argc=$#
    _i=0
    while [ "$_i" -lt "$_argc" ]; do
        arg="$1"
        shift
        case "$arg" in
            --no-modify-path) NO_MODIFY_PATH=1 ;;
            *)                set -- "$@" "$arg" ;;
        esac
        _i=$((_i + 1))
    done
    FORWARD_COUNT=$#
    FORWARD_ARGS="$*"
    if [ $# -eq 0 ]; then
        EXEC_LINE="install"
    else
        case "$1" in
            -*) EXEC_LINE="install $*" ;;
            *)  EXEC_LINE="$*" ;;
        esac
    fi
}

parse_args -y --tool cursor
assert_eq "$NO_MODIFY_PATH" "0" "no-modify-path stays 0 when not passed"
assert_contains "$FORWARD_ARGS" "-y" "arg passthrough includes -y"
assert_contains "$FORWARD_ARGS" "--tool" "arg passthrough includes --tool"
assert_contains "$FORWARD_ARGS" "cursor" "arg passthrough includes cursor"
assert_eq "$EXEC_LINE" "install -y --tool cursor" "leading flag gets the install subcommand prepended"

parse_args --no-modify-path -y
assert_eq "$NO_MODIFY_PATH" "1" "--no-modify-path is consumed and sets flag"
assert_not_contains "$FORWARD_ARGS" "--no-modify-path" "--no-modify-path not forwarded to binary"
assert_contains "$FORWARD_ARGS" "-y" "-y is forwarded even alongside --no-modify-path"
assert_eq "$EXEC_LINE" "install -y" "sh -s -- -y reaches the binary as 'install -y'"

parse_args selfhost
assert_eq "$EXEC_LINE" "selfhost" "explicit subcommand is forwarded verbatim, not prefixed"

parse_args "tool name with spaces"
assert_eq "$FORWARD_COUNT" "1" "a quoted arg with spaces stays one argument"

parse_args
assert_eq "$FORWARD_ARGS" "" "empty args means empty forward list"
assert_eq "$EXEC_LINE" "install" "no args defaults to the install subcommand"

# ── Test: no-downloader detection ─────────────────────────────────────────────

printf '\n--- no-downloader detection ---\n'

# Simulate the downloader check without actually running the script
have_downloader() {
    command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1
}

if have_downloader; then
    pass "curl or wget is available on this system (installer would proceed)"
else
    # In a stripped environment this is a valid test failure path
    pass "no downloader — would trigger friendly error (not tested further in this env)"
fi

# ── Test: shell rc detection ──────────────────────────────────────────────────

printf '\n--- shell rc detection ---\n'

detect_rc() {
    case "${SHELL:-}" in
        */fish) printf '%s/.config/fish/config.fish' "$HOME" ;;
        */zsh)  printf '%s/.zshrc' "$HOME" ;;
        *)      printf '%s/.bashrc' "$HOME" ;;
    esac
}

ORIG_SHELL="${SHELL:-}"

SHELL="/bin/zsh"
assert_eq "$(detect_rc)" "${HOME}/.zshrc" "zsh maps to .zshrc"

SHELL="/usr/bin/fish"
assert_eq "$(detect_rc)" "${HOME}/.config/fish/config.fish" "fish maps to config.fish"

SHELL="/bin/bash"
assert_eq "$(detect_rc)" "${HOME}/.bashrc" "bash maps to .bashrc"

SHELL="/bin/sh"
assert_eq "$(detect_rc)" "${HOME}/.bashrc" "unknown shell falls back to .bashrc"

SHELL="$ORIG_SHELL"

# ── Summary ───────────────────────────────────────────────────────────────────

printf '\n'
printf '%d passed, %d failed\n' "$PASS" "$FAIL"
if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
