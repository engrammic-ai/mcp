# Installer Phase 6: npm Shim + Post-Install Polish

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `@engrammic/install` as a platform-binary npm shim (JS launcher + per-platform optional packages), and add doctor-lite auto-run + a polished end-of-install message block to the Rust CLI.

**Architecture:**
- New top-level `npm/` directory under `mcp-client/` — one root package (`@engrammic/install`) and five per-platform packages (`@engrammic/install-{platform}`), each containing only a prebuilt binary extracted from the GitHub release.
- `npm/install/index.js` — 40-line JS launcher: resolves the correct platform package via `require.resolve`, spawns the binary with `spawnSync` inheriting stdio, propagates the exit code.
- `installer-cli/src/doctor.rs` — new `pub fn lite(manifest: &Manifest) -> Vec<CheckResult>` function: cloud branch (ureq HEAD, 405/406 counts as reachable), self-host branch (existing container logic re-used), and a harness-config parse check for both.
- `installer-cli/src/main.rs` — `run_full_install` end block: call `doctor::lite`, print verification results, then the polished block (restart reminder, docs link, usage example).
- `.github/workflows/release-installer.yml` — extend with SHA256 generation and npm publish steps.

**Tech Stack:** Node.js (no bundler — the launcher is a plain CJS script), Rust + ureq 2 (already in Cargo.toml), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md` (npm shim, Post-install experience, Doctor-lite semantics, Decisions)

**Sequencing note for the controller:** Task 1 (npm package layout) and Task 2 (doctor-lite Rust) are fully independent — run in parallel. Task 3 (post-install message) depends on Task 2. Task 4 (CI) depends on Task 1. Task 5 is last.

---

## PRE-FLIGHT bullets

Read these before touching any file:

- **Phase 1b** (`2026-06-10-installer-phase1b-interview-plan-execute.md`) restructures `run_full_install` and adds `flow::StepResult`. If shipped, the end-of-install block (Task 3) attaches after the existing `flow::summarize_results` print, not after the old linear sequence. The function signature may also have changed. Read `installer-cli/src/main.rs` lines 240–310 before editing.
- **Phase 1c** (`2026-06-10-installer-phase1c-*` — not yet written at plan time). Phase 1c defines exit code conventions for `engrammic doctor`. If Phase 1c ships before Phase 6, read what exit codes it assigned (expected: 0 = all clear, 1 = warnings, 2 = errors) and add the cloud branch in `run_diagnostics` using the same convention. If Phase 1c has NOT shipped, this phase only adds the cloud branch to `doctor::lite`; `run_diagnostics` gets the cloud path stubbed but exit codes are deferred to 1c.
- **Phase 2** (SHA256 release CI): read `.github/workflows/release-installer.yml` before writing the npm publish step. At plan time the release job downloads artifacts and calls `softprops/action-gh-release`. Phase 2 may have added a `sha256sum` step between download and release. Slot the npm publish AFTER the existing release step, not before.
- **No npm packaging exists** in `mcp-client/` at plan time (`find . -name "package.json"` returns nothing). The `npm/` directory is a net-new addition.
- **Docs URL:** the only docs URL in the codebase is `https://docs.engrammic.ai/docs/reference/configuration` (selfhost.rs:964). Use `https://docs.engrammic.ai` as the base for the end-of-install docs link. Do NOT use `https://engrammic.ai/docs` (not attested in the codebase).

---

## Task 1: npm package layout

**Files to create:**
- `npm/install/package.json`
- `npm/install/index.js` (the launcher — complete code in this task)
- `npm/install-linux-x64/package.json` (template; copy for the other four platforms)
- `npm/install-linux-arm64/package.json`
- `npm/install-darwin-x64/package.json`
- `npm/install-darwin-arm64/package.json`
- `npm/install-win32-x64/package.json`
- `npm/.npmrc` (workspace-level; sets `access=public`)

**Why this layout:** mirrors the esbuild/biome pattern. The root package has no native code — it is a thin launcher that resolves whichever optional platform package was installed. `npm install @engrammic/install` pulls only the matching platform binary (npm skips optional deps when the platform `os`/`cpu` fields do not match the current machine). `npx @engrammic/install` invokes the `bin` entry in the root package, which is `index.js`.

### 1.1 Root package.json

- [ ] **Step 1: Write `npm/install/package.json`**

```json
{
  "name": "@engrammic/install",
  "version": "0.12.0",
  "description": "Engrammic MCP installer — thin npm shim over the Rust CLI",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/engrammic-ai/mcp"
  },
  "bin": {
    "engrammic-install": "index.js"
  },
  "scripts": {
    "test": "node --test test.js"
  },
  "optionalDependencies": {
    "@engrammic/install-linux-x64": "0.12.0",
    "@engrammic/install-linux-arm64": "0.12.0",
    "@engrammic/install-darwin-x64": "0.12.0",
    "@engrammic/install-darwin-arm64": "0.12.0",
    "@engrammic/install-win32-x64": "0.12.0"
  },
  "engines": {
    "node": ">=16"
  }
}
```

Note: `version` must be bumped in sync with the Rust crate version on each release; the CI publish step (Task 4) overwrites it from the git tag, so keeping `0.12.0` here is a safe placeholder.

### 1.2 Launcher (complete, ~45 lines)

- [ ] **Step 2: Write `npm/install/index.js`**

```js
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
process.exit(result.status ?? 1);
```

- [ ] **Step 3: Make it executable**

The file needs the shebang and executable permission so npx can run it directly. On a POSIX system: `chmod +x npm/install/index.js`. In CI (see Task 4) this is handled by the publish step's `git` attributes or the `npm publish` pack step; note it in the CI task.

### 1.3 Per-platform package.json template

Each per-platform package contains only `package.json` and the binary (placed by CI). The `os` and `cpu` fields tell npm to skip the package on non-matching machines.

- [ ] **Step 4: Write all five per-platform package.json files**

`npm/install-linux-x64/package.json`:
```json
{
  "name": "@engrammic/install-linux-x64",
  "version": "0.12.0",
  "description": "Engrammic installer binary — linux x64",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/engrammic-ai/mcp"
  },
  "os": ["linux"],
  "cpu": ["x64"],
  "bin": {}
}
```

`npm/install-linux-arm64/package.json`:
```json
{
  "name": "@engrammic/install-linux-arm64",
  "version": "0.12.0",
  "description": "Engrammic installer binary — linux arm64",
  "license": "Apache-2.0",
  "repository": { "type": "git", "url": "https://github.com/engrammic-ai/mcp" },
  "os": ["linux"],
  "cpu": ["arm64"],
  "bin": {}
}
```

`npm/install-darwin-x64/package.json`:
```json
{
  "name": "@engrammic/install-darwin-x64",
  "version": "0.12.0",
  "description": "Engrammic installer binary — macOS x64",
  "license": "Apache-2.0",
  "repository": { "type": "git", "url": "https://github.com/engrammic-ai/mcp" },
  "os": ["darwin"],
  "cpu": ["x64"],
  "bin": {}
}
```

`npm/install-darwin-arm64/package.json`:
```json
{
  "name": "@engrammic/install-darwin-arm64",
  "version": "0.12.0",
  "description": "Engrammic installer binary — macOS arm64 (Apple Silicon)",
  "license": "Apache-2.0",
  "repository": { "type": "git", "url": "https://github.com/engrammic-ai/mcp" },
  "os": ["darwin"],
  "cpu": ["arm64"],
  "bin": {}
}
```

`npm/install-win32-x64/package.json`:
```json
{
  "name": "@engrammic/install-win32-x64",
  "version": "0.12.0",
  "description": "Engrammic installer binary — Windows x64",
  "license": "Apache-2.0",
  "repository": { "type": "git", "url": "https://github.com/engrammic-ai/mcp" },
  "os": ["win32"],
  "cpu": ["x64"],
  "bin": {}
}
```

Note: `bin` is empty on the platform packages — the launcher in the root package resolves the binary path itself (avoids naming conflicts and keeps the binary a data file, not a registered npm bin).

### 1.4 Workspace npmrc

- [ ] **Step 5: Write `npm/.npmrc`**

```ini
access=public
```

This ensures `npm publish` treats all scoped packages as public (npm defaults scoped packages to private).

### 1.5 Launcher tests

- [ ] **Step 6: Write `npm/install/test.js`**

```js
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
```

- [ ] **Step 7: Run tests**

```bash
node --test npm/install/test.js
```

Expected: 5 passed, 0 failed.

- [ ] **Step 8: Commit**

```bash
git add npm/
git commit -m "feat(npm): add @engrammic/install shim with platform binary packages and launcher"
```

---

## Task 2: doctor-lite in Rust

**Files:**
- Modify: `installer-cli/src/doctor.rs` — add `pub fn lite(manifest: &Manifest) -> Vec<CheckResult>` and the cloud reachability helper
- Modify: `installer-cli/src/main.rs` — add `mod doctor;` reference (already present) and import `CheckResult` in the doctor command handler for the cloud branch addition

### 2.1 Design

`doctor::lite` is a fast, non-interactive verification run automatically at the end of a fresh install. It returns a `Vec<CheckResult>` (not printed inside the function) so the caller can format and emit the results in the end-of-install block. This keeps the check logic unit-testable without a TTY.

**Cloud reachability:** the endpoint is a JSON-RPC MCP server — it responds to a valid JSON-RPC request with data, but responds to a plain HTTP HEAD with 405 (Method Not Allowed) or 406. Both responses prove TLS terminated and the server is listening. A TCP connect error or timeout means unreachable. `ureq` is already in `Cargo.toml` at version 2.

**Self-hosted reachability:** re-use the existing container checks from `run_diagnostics` — do not duplicate logic. Wrap the Docker+container checks into a shared private helper.

**Harness config parse:** for each `HarnessEntry` in the manifest, attempt to `fs::read_to_string` + `serde_json::from_str` (JSON harnesses) or `toml::from_str` (TOML harnesses). A missing file is a warning, not an error (the harness may have been uninstalled by the user). A parse failure is an error.

### 2.2 CheckResult type

- [ ] **Step 1: Write the failing test** (add at bottom of `doctor.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_http_response_is_reachable() {
        // Any HTTP status code means the server is up (even 405/406).
        // Only a connect/timeout error means unreachable.
        let ok_statuses = [200u16, 405, 406, 404, 500];
        for status in ok_statuses {
            assert!(
                classify_http_outcome(Ok(status)),
                "HTTP {status} must be classified as reachable"
            );
        }
    }

    #[test]
    fn classify_connect_error_is_not_reachable() {
        // A transport-level error means unreachable.
        assert!(
            !classify_http_outcome(Err(ReachError::Connect)),
            "connect error must be not-reachable"
        );
        assert!(
            !classify_http_outcome(Err(ReachError::Timeout)),
            "timeout must be not-reachable"
        );
    }

    #[test]
    fn lite_returns_one_check_per_domain() {
        // With an empty manifest and a cloud endpoint, lite must return
        // at least the endpoint check and the harness check (zero harnesses = pass).
        let m = crate::manifest::Manifest {
            endpoint: Some("https://beta.engrammic.ai/mcp/".to_string()),
            selfhost_dir: None,
            harnesses: vec![],
            ..Default::default()
        };
        let results = lite(&m);
        assert!(
            results.iter().any(|r| r.label.contains("endpoint")),
            "must include an endpoint check"
        );
        assert!(
            results.iter().any(|r| r.label.contains("harness")),
            "must include a harness config check"
        );
    }
}
```

Run `cargo test --bin engrammic doctor` — expect compile error (types not yet defined).

### 2.3 Implementation

- [ ] **Step 2: Add types and `lite` to `doctor.rs`**

Add at the top of `doctor.rs`, after the existing `use` lines:

```rust
use crate::manifest::Manifest;

/// A single doctor-lite verification result.
/// `ok = true` means the check passed. `detail` is the human-readable line
/// printed after the check label (e.g. "reachable", "config parse error: …").
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

/// Classification input for the endpoint reachability check.
/// Separates the network call from the classification logic so the
/// classification is unit-testable without a live network.
#[derive(Debug)]
pub enum ReachError {
    Connect,
    Timeout,
}

/// Returns true iff the outcome represents a live server.
/// Any HTTP status code counts; only transport errors fail the check.
pub fn classify_http_outcome(outcome: Result<u16, ReachError>) -> bool {
    outcome.is_ok()
}

/// Perform a HEAD request to `url` and return the HTTP status code,
/// or a `ReachError` if the connection failed or timed out.
/// 405/406 both count as reachable (see spec: doctor-lite semantics).
fn head_reachable(url: &str) -> Result<u16, ReachError> {
    match ureq::head(url).timeout(std::time::Duration::from_secs(8)).call() {
        Ok(resp) => Ok(resp.status()),
        Err(ureq::Error::Status(code, _)) => Ok(code),
        Err(ureq::Error::Transport(t)) => {
            // Distinguish timeout vs other connect errors for the error enum.
            // ureq::Transport does not expose a typed variant for timeout in v2;
            // stringify and match as a best-effort classification.
            let msg = t.to_string().to_ascii_lowercase();
            if msg.contains("timed out") || msg.contains("timeout") {
                Err(ReachError::Timeout)
            } else {
                Err(ReachError::Connect)
            }
        }
    }
}

/// Lightweight verification run at the end of a fresh install.
/// Does not print anything — the caller formats and emits the results.
///
/// Checks performed:
/// - Cloud mode: HEAD reachability of the configured endpoint host.
/// - Self-hosted mode: Docker running + containers healthy (re-uses private helpers).
/// - Both: each harness config file in the manifest parses without error.
pub fn lite(manifest: &Manifest) -> Vec<CheckResult> {
    let mut results: Vec<CheckResult> = Vec::new();

    // --- Endpoint reachability ---
    let is_cloud = manifest
        .endpoint
        .as_deref()
        .map(|ep| ep.contains("engrammic.ai"))
        .unwrap_or(false);

    if is_cloud {
        let endpoint = manifest
            .endpoint
            .as_deref()
            .unwrap_or(crate::tools::CLOUD_ENDPOINT);
        let outcome = head_reachable(endpoint);
        let reachable = classify_http_outcome(outcome);
        results.push(CheckResult {
            label: "endpoint reachable".to_string(),
            ok: reachable,
            detail: if reachable {
                "beta.engrammic.ai is up".to_string()
            } else {
                format!(
                    "could not reach {}  →  check your internet connection",
                    endpoint
                )
            },
        });
    } else if manifest.selfhost_dir.is_some() {
        // Self-hosted: re-use the existing Docker + container checks.
        let docker_ok = check_docker_running();
        results.push(CheckResult {
            label: "Docker running".to_string(),
            ok: docker_ok,
            detail: if docker_ok {
                "running".to_string()
            } else {
                "not running  →  run `docker info` to diagnose".to_string()
            },
        });

        if docker_ok {
            match check_containers() {
                Ok((healthy, total)) if total > 0 => {
                    let ok = healthy == total;
                    results.push(CheckResult {
                        label: "containers healthy".to_string(),
                        ok,
                        detail: format!("{}/{} healthy", healthy, total),
                    });
                }
                Ok(_) => {
                    results.push(CheckResult {
                        label: "containers healthy".to_string(),
                        ok: false,
                        detail: "no containers found  →  run `docker compose up -d`".to_string(),
                    });
                }
                Err(e) => {
                    results.push(CheckResult {
                        label: "containers healthy".to_string(),
                        ok: false,
                        detail: format!("could not check: {e:#}"),
                    });
                }
            }
        }
    }

    // --- Harness config files parse cleanly ---
    let harness_check = if manifest.harnesses.is_empty() {
        CheckResult {
            label: "harness configs".to_string(),
            ok: true,
            detail: "none configured".to_string(),
        }
    } else {
        let mut failed: Vec<String> = Vec::new();
        for h in &manifest.harnesses {
            match std::fs::read_to_string(&h.config_path) {
                Err(_) => {
                    // Missing file is a warning, not hard failure.
                    // (user may have moved/deleted the editor)
                }
                Ok(content) => {
                    let ext = h
                        .config_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    let parse_ok = match ext {
                        "toml" => toml::from_str::<toml::Value>(&content).is_ok(),
                        _ => serde_json::from_str::<serde_json::Value>(&content).is_ok(),
                    };
                    if !parse_ok {
                        failed.push(h.config_path.display().to_string());
                    }
                }
            }
        }
        if failed.is_empty() {
            CheckResult {
                label: "harness configs".to_string(),
                ok: true,
                detail: format!("{} file(s) parse cleanly", manifest.harnesses.len()),
            }
        } else {
            CheckResult {
                label: "harness configs".to_string(),
                ok: false,
                detail: format!("parse error in: {}", failed.join(", ")),
            }
        }
    };
    results.push(harness_check);

    results
}
```

Add the missing `use` imports at the top of `doctor.rs` (alongside the existing ones):

```rust
use crate::manifest::Manifest;
```

And add `toml` and `serde_json` imports (they are already in `Cargo.toml`; use their crate names directly — no new dependencies needed):
The existing `doctor.rs` does not yet import `serde_json` or `toml`. Add at the top:
```rust
use serde_json;  // already in Cargo.toml — for harness config parse check
```
`toml` is already used via `crate::manifest`; for the parse check call it as `toml::from_str::<toml::Value>` which compiles without an explicit use.

- [ ] **Step 3: Run tests**

```bash
cargo test --bin engrammic doctor
```

Expected: 3 passed (the three new tests in `doctor::tests`). Existing `run_diagnostics` tests (none today — the function is untested) are unaffected.

- [ ] **Step 4: Commit**

```bash
git add installer-cli/src/doctor.rs
git commit -m "feat(installer): add doctor::lite with cloud endpoint reachability and harness config parse check"
```

---

## Task 3: Post-install polish in run_full_install

**Files:**
- Modify: `installer-cli/src/main.rs` — `run_full_install` ending block (currently lines 296–307; Phase 1b may have moved these — PRE-FLIGHT: read the file before editing)

### 3.1 Design

The current end-of-install block:

```rust
println!("Done. Tools available: {}", "remember, recall, learn, believe, trace, link".dimmed());
print_restart_reminder();
println!();
cli_install::offer_cli_install(auto)?;
```

This phase replaces and extends it with:
1. Doctor-lite auto-run (call `doctor::lite(&manifest)`, format results inline)
2. Verification block output
3. Restart reminder (keep existing `print_restart_reminder()`)
4. Docs link
5. One concrete usage example

`offer_cli_install` is deleted by Phase 2 (script-owned persistence); if Phase 2 has shipped, remove the call here too — if not, leave it after the new block.

### 3.2 New end block

- [ ] **Step 1: PRE-FLIGHT** — read the current state of `run_full_install`:

```bash
grep -n "Done\. Tools\|print_restart\|offer_cli_install\|doctor\|lite" installer-cli/src/main.rs | head -20
```

Verify lines before editing. If Phase 1b has shipped, `run_full_install` ends with `flow::summarize_results` output — insert the new block after the counts print but before `Ok(())`.

- [ ] **Step 2: Replace the ending block** (locate the exact lines, then replace with):

```rust
    // ---- Doctor-lite auto-run ----
    let manifest_for_check = manifest::Manifest::load_or_migrate(None).unwrap_or_default();
    let checks = doctor::lite(&manifest_for_check);
    println!();
    println!("{}", "Verification".bold());
    for c in &checks {
        if c.ok {
            println!("  {} {:<28} {}", "✓".green(), c.label, c.detail.dimmed());
        } else {
            println!("  {} {:<28} {}", "✗".red(), c.label, c.detail.yellow());
        }
    }
    let all_ok = checks.iter().all(|c| c.ok);

    // ---- End-of-install message ----
    println!();
    print_restart_reminder();
    println!();
    println!(
        "  {} {}",
        "Docs:".dimmed(),
        "https://docs.engrammic.ai".cyan()
    );
    println!();
    println!("  {}", "Try it out — ask your agent:".bold());
    println!(
        "    {}",
        r#""Remember that the API base URL is https://api.example.com""#.cyan()
    );
    println!();
    if !all_ok {
        println!(
            "  {} Run {} for more details.",
            "!".yellow(),
            "engrammic doctor".cyan()
        );
        println!();
    }

    cli_install::offer_cli_install(auto)?;  // remove this line if Phase 2 has shipped
    Ok(())
```

Important notes:
- The manifest is loaded a second time here (after the execute loop already saved it) to pick up the just-written state. This is intentional and cheap — the file is tiny TOML.
- `print_restart_reminder` already exists in `main.rs`; do not remove or duplicate it.
- The `cli_install::offer_cli_install(auto)?` line should be removed when Phase 2 ships (script-owned persistence). Add a `// TODO(phase2): remove after Phase 2 ships` comment for now if Phase 2 hasn't landed yet.

- [ ] **Step 3: Add the `doctor` import in `main.rs`** if it is not already used in a non-test path:

`mod doctor;` is already declared at line 7 of `main.rs`. Add `use doctor::CheckResult;` is NOT needed — we reference `doctor::lite` directly. Verify `doctor::CheckResult` is accessible as `c.label` etc. — since `CheckResult` is `pub`, `doctor::lite(...)` returns `Vec<doctor::CheckResult>` and fields are accessed directly without importing the type.

- [ ] **Step 4: Build + smoke**

```bash
cargo build 2>&1 | tail -3
```

Then in a scratch HOME:
```bash
SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.claude"
echo '{"mcpServers":{}}' > "$SCRATCH/.claude/settings.json"
HOME="$SCRATCH" cargo run -q -- install -y 2>&1 | tail -20
```

Expected output tail includes:
- `Verification` heading with at least one `✓` line (endpoint or harness)
- Restart reminder line
- `Docs: https://docs.engrammic.ai`
- The usage example line

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): add doctor-lite auto-run and polished end-of-install message block"
```

---

## Task 4: CI — SHA256 + npm publish

**Files:**
- Modify: `.github/workflows/release-installer.yml`

### 4.1 PRE-FLIGHT

Read the current workflow file before editing:
```bash
cat .github/workflows/release-installer.yml
```

At plan time the release job (runs only on tag push) has three steps: download artifacts, create GitHub release. Phase 2 may have inserted a `sha256sum` step between these two. Identify what exists, then add the npm steps AFTER the existing release step.

### 4.2 SHA256 step (if Phase 2 has not shipped)

If Phase 2 has NOT added it, insert this step BEFORE the `Create Release` step in the `release` job:

```yaml
      - name: Generate SHA256 checksums
        run: |
          cd artifacts
          sha256sum engrammic-* > SHA256SUMS
          # Also emit per-binary sidecar files in the two-column sha256sum format
          # so install.sh and install.ps1 can verify with: sha256sum -c <binary>.sha256
          for f in engrammic-*; do
            sha256sum "$f" > "${f}.sha256"
          done
```

If Phase 2 HAS shipped this step, skip it (do not duplicate).

### 4.3 npm publish step

Add a new `publish-npm` job to the workflow, running after `release`:

```yaml
  publish-npm:
    name: Publish npm packages
    needs: release
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')

    steps:
      - uses: actions/checkout@v4

      - name: Set up Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: Download build artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true

      - name: Extract version from tag
        id: version
        run: |
          # Tag format: installer-v0.12.0 → version 0.12.0
          echo "version=${GITHUB_REF_NAME#installer-v}" >> "$GITHUB_OUTPUT"

      - name: Populate platform packages and set versions
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          # Mapping: artifact filename → npm platform package directory
          declare -A PKG_MAP=(
            ["engrammic-x86_64-unknown-linux-gnu"]="npm/install-linux-x64"
            ["engrammic-aarch64-unknown-linux-gnu"]="npm/install-linux-arm64"
            ["engrammic-x86_64-apple-darwin"]="npm/install-darwin-x64"
            ["engrammic-aarch64-apple-darwin"]="npm/install-darwin-arm64"
            ["engrammic-x86_64-pc-windows-msvc.exe"]="npm/install-win32-x64"
          )
          for artifact in "${!PKG_MAP[@]}"; do
            dir="${PKG_MAP[$artifact]}"
            src="artifacts/${artifact}"
            if [ ! -f "$src" ]; then
              echo "WARNING: artifact not found: $src — skipping"
              continue
            fi
            if [[ "$artifact" == *.exe ]]; then
              cp "$src" "$dir/engrammic.exe"
            else
              cp "$src" "$dir/engrammic"
              chmod +x "$dir/engrammic"
            fi
            # Set the version in each platform package.json
            node -e "
              const fs = require('fs');
              const p = '$dir/package.json';
              const pkg = JSON.parse(fs.readFileSync(p, 'utf8'));
              pkg.version = '$VERSION';
              fs.writeFileSync(p, JSON.stringify(pkg, null, 2) + '\n');
            "
          done
          # Set version in root launcher package
          node -e "
            const fs = require('fs');
            const p = 'npm/install/package.json';
            const pkg = JSON.parse(fs.readFileSync(p, 'utf8'));
            pkg.version = '$VERSION';
            // Bump all optionalDependencies to the same version
            for (const k of Object.keys(pkg.optionalDependencies || {})) {
              pkg.optionalDependencies[k] = '$VERSION';
            }
            fs.writeFileSync(p, JSON.stringify(pkg, null, 2) + '\n');
          "
          # Ensure launcher is executable in the tarball
          chmod +x npm/install/index.js

      - name: Publish platform packages
        run: |
          for dir in npm/install-linux-x64 npm/install-linux-arm64 \
                     npm/install-darwin-x64 npm/install-darwin-arm64 \
                     npm/install-win32-x64; do
            echo "Publishing $dir ..."
            npm publish "$dir" --access public
          done
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Publish root launcher package
        run: npm publish npm/install --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

**Required secret:** `NPM_TOKEN` must be added to the repository's GitHub Actions secrets. This is a manual one-time step by a maintainer — document it in a comment in the workflow file:

```yaml
      # PRE-REQUISITE: add NPM_TOKEN to repository secrets.
      # Create at https://www.npmjs.com/settings/<org>/tokens
      # Token type: Automation (does not require 2FA on publish).
```

- [ ] **Step 1: Edit `.github/workflows/release-installer.yml`** with the SHA256 step (if needed) and the `publish-npm` job as above.

- [ ] **Step 2: Verify workflow syntax**

```bash
node -e "const fs=require('fs'); require('js-yaml').load(fs.readFileSync('.github/workflows/release-installer.yml','utf8')); console.log('ok')" 2>/dev/null || python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release-installer.yml')); print('ok')"
```

(Either js-yaml or Python's PyYAML will work; one is likely available.)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-installer.yml
git commit -m "ci: add npm publish job for @engrammic/install platform packages"
```

---

## Task 5: Verification pass

- [ ] **Step 1: Full Rust test suite**

```bash
cd installer-cli && cargo test 2>&1 | tail -5 && cargo build 2>&1 | tail -1
```

Expected: all tests pass (including the 3 new doctor tests); clean build.

- [ ] **Step 2: Launcher unit tests**

```bash
node --test npm/install/test.js
```

Expected: 5 passed, 0 failed.

- [ ] **Step 3: Lint pass**

```bash
cd installer-cli && cargo fmt -- src/doctor.rs src/main.rs
```

Check no new warnings appear that weren't in the pre-existing set (the known pre-existing warnings are in license/selfhost/tools per Phase 1b).

- [ ] **Step 4: Manual npx smoke (local)**

Install the root launcher package locally to verify `require.resolve` works with a real binary present:

```bash
# Create a temp workspace
WSTMP=$(mktemp -d)
# Copy the root package
cp -r npm/install "$WSTMP/install"
# Copy the current platform package (linux-x64 example)
mkdir -p "$WSTMP/node_modules/@engrammic"
cp -r npm/install-linux-x64 "$WSTMP/node_modules/@engrammic/install-linux-x64"
# Copy a real binary into the platform package dir
cp installer-cli/target/debug/engrammic "$WSTMP/node_modules/@engrammic/install-linux-x64/engrammic"
# Run the launcher
node "$WSTMP/install/index.js" --version 2>&1 | head -3
```

Expected: the Rust binary prints its version (or an `--version` unrecognized flag message); no "[engrammic] Failed to spawn" error.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A && git commit -m "chore(installer): phase 6 verification fixes" || echo "clean"
```

---

## Verification commands (quick reference)

| What | Command |
|------|---------|
| Rust tests (doctor module) | `cd installer-cli && cargo test doctor` |
| Rust full suite | `cd installer-cli && cargo test` |
| Launcher unit tests | `node --test npm/install/test.js` |
| Build (release) | `cd installer-cli && cargo build --release` |
| Scratch install smoke | `SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.claude"; echo '{"mcpServers":{}}' > "$SCRATCH/.claude/settings.json"; HOME="$SCRATCH" ./installer-cli/target/debug/engrammic install -y` |
| YAML lint | `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-installer.yml'))"` |

---

## Commit sequence summary

1. `feat(npm): add @engrammic/install shim with platform binary packages and launcher` — Task 1
2. `feat(installer): add doctor::lite with cloud endpoint reachability and harness config parse check` — Task 2
3. `feat(installer): add doctor-lite auto-run and polished end-of-install message block` — Task 3
4. `ci: add npm publish job for @engrammic/install platform packages` — Task 4
5. `chore(installer): phase 6 verification fixes` — Task 5 (only if needed)
