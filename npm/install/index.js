#!/usr/bin/env node
// @engrammic/install — platform binary launcher
// Resolves the correct @engrammic/install-{platform} optional package,
// spawns the prebuilt Rust binary with inherited stdio, and propagates
// the exit code. No logic beyond binary resolution.
"use strict";

const { spawnSync } = require("child_process");
const path = require("path");

// Maps Node's process.platform + process.arch to our package suffix.
// Must stay in sync with the optionalDependencies in package.json.
const PLATFORM_MAP = {
  "linux-x64":   "@engrammic/install-linux-x64",
  "linux-arm64": "@engrammic/install-linux-arm64",
  "darwin-x64":  "@engrammic/install-darwin-x64",
  "darwin-arm64":"@engrammic/install-darwin-arm64",
  "win32-x64":   "@engrammic/install-win32-x64",
};

const CURL_FALLBACK =
  "curl -fsSL https://get.engrammic.ai/install.sh | sh";

function resolveBinary() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORM_MAP[key];
  if (!pkg) {
    console.error(
      `[engrammic] Unsupported platform: ${process.platform}/${process.arch}`
    );
    console.error(
      `[engrammic] Try the curl installer instead:\n    ${CURL_FALLBACK}`
    );
    process.exit(1);
  }

  let binDir;
  try {
    // require.resolve locates the package's package.json; the binary lives
    // in the same directory under the platform-specific name.
    binDir = path.dirname(require.resolve(`${pkg}/package.json`));
  } catch {
    console.error(
      `[engrammic] Platform package ${pkg} is not installed.`
    );
    console.error(
      `[engrammic] Re-run: npm install @engrammic/install\n` +
      `[engrammic] Or use the curl installer:\n    ${CURL_FALLBACK}`
    );
    process.exit(1);
  }

  const isWindows = process.platform === "win32";
  return path.join(binDir, isWindows ? "engrammic.exe" : "engrammic");
}

const bin = resolveBinary();
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(`[engrammic] Failed to spawn binary: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  // Re-raise so callers see the real signal, not exit code 1.
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 1);
