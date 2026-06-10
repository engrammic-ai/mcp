// Tests for the PLATFORM_MAP lookup logic.
// Run with: node --test npm/install/test.js
// Does NOT test actual binary spawning (that requires a real binary).
"use strict";
const { test } = require("node:test");
const assert = require("node:assert/strict");

// Inline a testable copy of PLATFORM_MAP (same values as index.js).
const PLATFORM_MAP = {
  "linux-x64":    "@engrammic/install-linux-x64",
  "linux-arm64":  "@engrammic/install-linux-arm64",
  "darwin-x64":   "@engrammic/install-darwin-x64",
  "darwin-arm64": "@engrammic/install-darwin-arm64",
  "win32-x64":    "@engrammic/install-win32-x64",
};

test("all five platforms resolve to a package name", () => {
  const keys = Object.keys(PLATFORM_MAP);
  assert.equal(keys.length, 5, "must have exactly 5 entries");
  for (const k of keys) {
    assert.match(
      PLATFORM_MAP[k],
      /^@engrammic\/install-/,
      `${k} → package name must start with @engrammic/install-`
    );
  }
});

test("linux-x64 resolves correctly", () => {
  assert.equal(PLATFORM_MAP["linux-x64"], "@engrammic/install-linux-x64");
});

test("darwin-arm64 resolves correctly", () => {
  assert.equal(PLATFORM_MAP["darwin-arm64"], "@engrammic/install-darwin-arm64");
});

test("win32-x64 resolves correctly", () => {
  assert.equal(PLATFORM_MAP["win32-x64"], "@engrammic/install-win32-x64");
});

test("unknown platform returns undefined (caller must handle)", () => {
  assert.equal(PLATFORM_MAP["freebsd-x64"], undefined);
});
