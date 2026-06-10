# Installer Phase 1c: Error Conventions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply consistent `✗ <what happened>` / `→ <what to do>` error formatting to all user-facing failures; fix every `bail!` site that discards a more specific message already printed; add Esc-to-skip to `selfhost::prompt_license`; and give `engrammic doctor` exit codes that distinguish warnings from hard errors.

**Architecture:** No new modules. Changes are surgical: a helper function `fmt_error` in `main.rs` (or inlined), targeted rewrites of specific `bail!` and `println!` sites in `main.rs`, `selfhost.rs`, and `doctor.rs`. The config-parse error wrappers in `config.rs` (`read_json` / `read_yaml` / `read_toml_doc`) already use `.with_context(|| format!("failed to parse … in {}", path))` which is correct — those stay. The printing layer in `main.rs` (`install_tool`, `run_docker_setup`) needs to forward those context chains rather than replacing them with vague `bail!("Docker not available")`.

**Tech Stack:** Rust, `colored` (already a dependency), `dialoguer` 0.11 (already a dependency — `dialoguer::Confirm` Esc support). Crate root `installer-cli/`; all commands run from there.

**Spec:** `docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md` — "Error-handling conventions" section; Design principle 3 ("Never dead-end").

**Sequencing note for the controller:** Tasks 1 and 2 are independent of each other and may run in parallel (different files/regions). Task 3 depends on nothing in this phase but should be read before Task 2 because 1b may delete `prompt_for_license` from `main.rs`. Task 4 is entirely self-contained (only `doctor.rs`). Task 5 is a gate — run after all other tasks pass.

---

### Task 1: `✗ / →` format helper + apply to `run_docker_setup` / `check_prerequisites` bail sites

**PRE-FLIGHT:** Phase 1b deletes `run_docker_setup` and `install_docker` from `main.rs` and consolidates into `selfhost::run_wizard`. Verify before starting:
- If `run_docker_setup` still exists in `main.rs` (≈line 364), apply steps below to it.
- If it has been deleted, the Docker-check bail site to fix is `selfhost::check_prerequisites` (selfhost.rs:217) — apply the same treatment there.
- Both may need fixing; check both files before starting.

**Files:**
- Modify: `installer-cli/src/main.rs` — `run_docker_setup` (≈line 364-495) if it still exists
- Modify: `installer-cli/src/selfhost.rs` — `check_prerequisites` (lines 206-255), `prompt_install_dir` (line 708-735), `start_and_wait` (lines 976-1016)

**The bug pattern being fixed in this task:**

`run_docker_setup` (main.rs:367-374) prints `"✗ Docker is not running or not installed."` then `bail!("Docker not available")`. The `bail!` message replaces the specific message in the error chain seen by the caller. The fix is to propagate the specific context into the bail.

`check_prerequisites` (selfhost.rs:217) does the same: prints then bare `bail!("Docker not available")`.

`start_and_wait` (selfhost.rs:994-995) prints `"✗ Failed to pull images: {stderr}"` then `bail!("docker compose pull failed")` — discards the stderr.

**The `→` line** must be added wherever a `✗` line exists without one.

- [ ] **Step 1: Write the failing tests** (add to bottom of `selfhost.rs` `tests` module)

```rust
    #[test]
    fn check_prerequisites_bail_message_is_specific() {
        // This test is documentation: check_prerequisites is not unit-testable
        // without Docker. Compile-check only — assert the function exists.
        let _: fn() -> anyhow::Result<()> = super::check_prerequisites;
    }
```

(Pure compile-check; the real validation is the smoke run in Task 5.)

- [ ] **Step 2: Add `fmt_err` helper to `main.rs`** (insert near top, after `use` block, before `fn main`)

```rust
/// Print a two-line error in the standard ✗ / → format.
///
///   ✗ <what_happened>
///   → <what_to_do>
///
/// Use this for all user-facing failures so the format is consistent.
fn fmt_err(what_happened: &str, what_to_do: &str) {
    eprintln!("  {} {}", "✗".red(), what_happened);
    eprintln!("  {} {}", "→".yellow(), what_to_do);
}
```

Add the same helper to `selfhost.rs` (since many of its callsites are in that file and it does not call back into `main`):

```rust
fn fmt_err(what_happened: &str, what_to_do: &str) {
    eprintln!("  {} {}", "✗".red().bold(), what_happened);
    eprintln!("  {} {}", "→".yellow(), what_to_do);
}
```

(Duplicate is intentional: they are in separate modules with no shared util module. If a `util.rs` module exists, put it there instead and import.)

- [ ] **Step 3: Fix `run_docker_setup` Docker-check bail (main.rs ≈369-374) — IF IT STILL EXISTS**

Old:
```rust
        println!("{} Docker is not running or not installed.", "✗".red());
        println!(
            "  Install Docker Desktop from {} then try again.",
            "https://docs.docker.com/get-docker/".cyan()
        );
        anyhow::bail!("Docker not available");
```

New:
```rust
        fmt_err(
            "Docker is not running or not installed.",
            &format!(
                "Install Docker Desktop from {} then try again.",
                "https://docs.docker.com/get-docker/".cyan()
            ),
        );
        anyhow::bail!("Docker is not running or not installed");
```

(The `bail!` message now matches the printed message so the error chain is consistent if it ever surfaces to a caller.)

- [ ] **Step 4: Fix `check_prerequisites` Docker-check bail (selfhost.rs:206-219)**

Old:
```rust
    print!("  Checking Docker... ");
    if !docker::check_docker()? {
        println!("{}", "not found".red());
        println!();
        println!(
            "  {} Docker is required. Install from: {}",
            "!".yellow(),
            "https://docs.docker.com/get-docker/".cyan()
        );
        anyhow::bail!("Docker not available");
    }
    println!("{}", "ok".green());
```

New:
```rust
    print!("  Checking Docker... ");
    if !docker::check_docker()? {
        println!("{}", "not found".red());
        println!();
        fmt_err(
            "Docker is not running or not installed.",
            &format!(
                "Install Docker Desktop from {} then try again.",
                "https://docs.docker.com/get-docker/".cyan()
            ),
        );
        anyhow::bail!("Docker is not running or not installed");
    }
    println!("{}", "ok".green());
```

Similarly fix the Docker Compose check (selfhost.rs:228-239):

Old:
```rust
        _ => {
            println!("{}", "not found".red());
            println!();
            println!(
                "  {} Docker Compose v2 is required. Upgrade Docker Desktop or install the compose plugin.",
                "!".yellow()
            );
            anyhow::bail!("Docker Compose v2 not available");
        }
```

New:
```rust
        _ => {
            println!("{}", "not found".red());
            println!();
            fmt_err(
                "Docker Compose v2 not found.",
                "Upgrade Docker Desktop or install the compose plugin, then try again.",
            );
            anyhow::bail!("Docker Compose v2 not found");
        }
```

- [ ] **Step 5: Fix `start_and_wait` image-pull bail (selfhost.rs:990-995)**

Old:
```rust
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("  {} Failed to pull images: {}", "✗".red(), stderr);
        anyhow::bail!("docker compose pull failed");
    }
```

New:
```rust
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        fmt_err(
            &format!("Failed to pull Docker images: {}", stderr.trim()),
            "Check your internet connection and Docker daemon, then run `engrammic selfhost` again.",
        );
        anyhow::bail!("docker compose pull failed: {}", stderr.trim());
    }
```

- [ ] **Step 6: Fix `prompt_install_dir` bail (selfhost.rs:730-733)**

Old:
```rust
        if !overwrite {
            anyhow::bail!("Cancelled - existing installation preserved");
        }
```

New:
```rust
        if !overwrite {
            println!(
                "  {} Existing installation preserved at {}",
                "→".yellow(),
                path.display()
            );
            anyhow::bail!("Cancelled — existing installation preserved");
        }
```

(This is a user-initiated cancel, so `✗` is wrong; use `→` to confirm the safe outcome, then bail to unwind without printing an error at the top level.)

- [ ] **Step 7: Build check**

```bash
cargo build 2>&1 | tail -5
```

Expected: clean (fix any unused-import warnings from removed `println!`).

- [ ] **Step 8: Commit**

```bash
git add installer-cli/src/main.rs installer-cli/src/selfhost.rs
git commit -m "fix(installer): apply ✗/→ format and propagate specific messages in bail! sites"
```

---

### Task 2: Audit every `bail!`/`anyhow!` that discards a specific printed message — full grep pass

**PRE-FLIGHT:** Phase 1b deletes `prompt_for_license` from `main.rs` (≈line 333). If it is gone, skip the main.rs entry for it in the table below. If it still exists, fix it as described.

**Files:**
- Modify: `installer-cli/src/main.rs` — `prompt_for_license` (if still present, ≈line 333-362)
- Modify: `installer-cli/src/selfhost.rs` — already covered in Task 1; verify no remaining sites

**Audit table** (compiled from reading the source; every `bail!` after a user-visible `println!`):

| File | Location | Printed message | bail! message | Verdict |
|------|----------|-----------------|---------------|---------|
| main.rs | `prompt_for_license` ≈359 | `"  ✗ {e}"` (the license error) | `"Invalid license"` | **FIX** — discards `e` |
| main.rs | `run_docker_setup` ≈374 | `"✗ Docker is not running..."` | `"Docker not available"` | FIX — covered in Task 1 |
| selfhost.rs | `check_prerequisites` ≈217 | `"Docker is required. Install from:..."` | `"Docker not available"` | FIX — covered in Task 1 |
| selfhost.rs | `check_prerequisites` ≈237 | `"Docker Compose v2 is required..."` | `"Docker Compose v2 not available"` | FIX — covered in Task 1 |
| selfhost.rs | `start_and_wait` ≈994 | `"✗ Failed to pull images: {stderr}"` | `"docker compose pull failed"` | FIX — covered in Task 1 |
| selfhost.rs | `start_and_wait` ≈1005 | (none before bail) | `"docker compose up failed"` | OK — no specific message to lose |
| selfhost.rs | `prompt_install_dir` ≈730 | (user chose not to overwrite) | `"Cancelled - existing installation preserved"` | OK — cancel, not an error; addressed in Task 1 |
| config.rs | `read_json` ≈101 | (none — context chain only) | n/a (uses `.with_context`) | OK — context is propagated |
| config.rs | `read_toml_doc` ≈206 | (none — context chain only) | n/a (uses `.with_context`) | OK |
| config.rs | `read_yaml` ≈303 | (none — context chain only) | n/a (uses `.with_context`) | OK |
| doctor.rs | all bail! | (errors returned, not printed before bail) | various | OK — no print-then-bail pattern |
| license.rs | all bail! | (pure validation — no prior println) | various | OK |

**The canonical bug class (main.rs `prompt_for_license`):**

```rust
// BEFORE (line 357-360):
Err(e) => {
    println!("  {} {}", "✗".red(), e);   // prints specific: "License key has expired"
    anyhow::bail!("Invalid license");    // DISCARDS e — caller sees only "Invalid license"
}
```

```rust
// AFTER:
Err(e) => {
    fmt_err(
        &format!("{}", e),
        "Check the key is correct, not expired, and starts with ENGR_. \
         Get a new key at engrammic.ai/self-hosted.",
    );
    anyhow::bail!("{}", e);   // propagates the specific message
}
```

- [ ] **Step 1: Fix `prompt_for_license` in main.rs (if it still exists)**

Locate the `Err(e)` arm in `prompt_for_license` (≈line 357). Apply the rewrite above.

If `prompt_for_license` has been deleted by Phase 1b, skip this step and note it in the commit message.

- [ ] **Step 2: Verify no remaining print-then-bail sites**

```bash
grep -n "bail!" installer-cli/src/main.rs installer-cli/src/selfhost.rs
```

For each line printed: scan the 5 lines above it for a `println!` or `eprintln!` with a specific error. Any found that are NOT already in the audit table above must be fixed using the same pattern (propagate `e` into the bail message).

- [ ] **Step 3: Build and test**

```bash
cargo test 2>&1 | tail -3 && cargo build 2>&1 | tail -1
```

Expected: all pass, clean build.

- [ ] **Step 4: Commit**

```bash
git add installer-cli/src/main.rs installer-cli/src/selfhost.rs
git commit -m "fix(installer): propagate specific error messages through bail! — no message discarded"
```

---

### Task 3: `selfhost::prompt_license` — Esc-to-skip and `✗/→` format per attempt

**PRE-FLIGHT:** The existing `prompt_license` in `selfhost.rs` (line 434) already has a retry loop (lines 454-478) — the Opus review was correct. This task adds only the delta:
1. Esc exits the loop and returns a sentinel that the caller treats as "skip" (try `engrammic license set` later).
2. Each failed attempt prints `✗ <specific failure>` then `→ <what to do>`.
3. The existing valid-key path (lines 463-469) is fine; leave it.

Note: `prompt_license` loops are **not unit-testable** (require a TTY). No test step here — document this explicitly. The smoke test in Task 5 is the validation gate.

**Files:**
- Modify: `installer-cli/src/selfhost.rs` — `prompt_license` (lines 434-479), `run_wizard` (line 97 call site)

- [ ] **Step 1: Change `prompt_license` return type to `Option<String>` and add Esc-to-skip**

```rust
/// Prompt for a license key with a retry loop.
///
/// Returns `Some(key)` on success, `None` if the user presses Esc to skip.
/// The caller should then record that the license step is pending and print
/// how to complete it later (`engrammic license set`).
///
/// Not unit-testable (requires a TTY).
fn prompt_license() -> Result<Option<String>> {
    let existing = UserConfig::load().ok().and_then(|c| c.license_key);

    if let Some(ref key) = existing {
        if let Ok(info) = license::validate_license_format(key) {
            println!(
                "  Found existing license: {} ({} days remaining)",
                info.customer.cyan(),
                info.days_remaining
            );
            let keep = Confirm::new()
                .with_prompt("  Use this license?")
                .default(true)
                .interact()?;
            if keep {
                return Ok(Some(key.clone()));
            }
        }
    }

    println!("  {}", "(Press Esc or leave blank to skip — finish later with `engrammic license set`)".dimmed());

    loop {
        println!(
            "  {}",
            "(Starts with ENGR_ - request at founders@engrammic.ai)".dimmed()
        );

        // dialoguer Input does not surface Esc directly; we use an empty string
        // submitted via Enter as the skip signal (user is told to leave blank).
        let raw: String = dialoguer::Input::new()
            .with_prompt("License key (input visible, blank to skip)")
            .allow_empty(true)
            .interact_text()?;

        let key = raw.trim().to_string();

        if key.is_empty() {
            println!();
            println!(
                "  {} License skipped.",
                "→".yellow()
            );
            println!(
                "  Run {} to add your license key later.",
                "engrammic license set".cyan()
            );
            println!();
            return Ok(None);
        }

        match license::validate_license_format(&key) {
            Ok(info) => {
                println!(
                    "  {} Valid — {}, {} days remaining",
                    "✓".green(),
                    info.customer.cyan(),
                    info.days_remaining
                );
                return Ok(Some(key));
            }
            Err(e) => {
                fmt_err(
                    &format!("{}", e),
                    "Check the key starts with ENGR_, is not expired, and was copied in full.",
                );
                println!();
                // loop continues
            }
        }
    }
}
```

Note on `allow_empty`: verify `dialoguer::Input::allow_empty` exists in the vendored version.
Check with: `grep -r "allow_empty" ~/.cargo/registry/src/*/dialoguer*/src/` — if absent, use
`.validate_with(|_: &String| Ok::<(), &str>(()))` and handle the empty check manually (same
logic, slightly different syntax). Do NOT use `dialoguer::Confirm` for Esc; the blank-to-skip
is simpler and works without TTY-level key detection.

- [ ] **Step 2: Update `run_wizard` call site (selfhost.rs:97)**

Old:
```rust
    let license_key = prompt_license()?;
```

New:
```rust
    let license_key_opt = prompt_license()?;
    let license_key = license_key_opt.unwrap_or_default();
    // If empty: the .env will have ENGRAMMIC_LICENSE_KEY= (blank); the user must run
    // `engrammic license set` before Engrammic will accept connections. The wizard
    // continues so the rest of the setup is not lost.
```

(Using `unwrap_or_default()` means the compose/.env are written with an empty key. This is
intentional — writing files first and fixing the license later is better than aborting the whole
install. The "skipped" message from `prompt_license` already told the user what to do.)

- [ ] **Step 3: Build check**

```bash
cargo build 2>&1 | tail -5
```

Fix any type-mismatch errors (the `license_key: String` field in `SelfHostConfig` expects a `String`; `unwrap_or_default()` on `Option<String>` yields `String` — should compile cleanly).

- [ ] **Step 4: Commit**

```bash
git add installer-cli/src/selfhost.rs
git commit -m "feat(installer): add Esc-to-skip and ✗/→ formatting to selfhost license prompt"
```

---

### Task 4: `engrammic doctor` exit codes — distinguish warnings from errors

**Files:**
- Modify: `installer-cli/src/doctor.rs` — `run_diagnostics` (lines 7-106)

**Current behavior:** `run_diagnostics` always returns `Ok(())` (line 105). Exit code is 0 regardless of failures. The caller in `main.rs` (line 48: `Commands::Doctor => doctor::run_diagnostics()`) propagates the `Result` to `main()`, which exits 0 on `Ok`.

**Convention to adopt** (matches common CLI tool practice; spec says "distinguish warnings from errors"):

| Condition | Exit code | Meaning |
|-----------|-----------|---------|
| All checks passed or only warning-level issues | 0 | Healthy (or degraded but not broken) |
| One or more hard-error checks failed | 1 | Errors present — action required |

Warnings vs errors: in the current `check_*` functions, checks that set `all_passed = false` are hard errors (Docker not running, containers unhealthy, license missing/invalid). Checks that print yellow but do NOT set `all_passed = false` (low disk, unreachable telemetry endpoint, OOM events) are warnings — they print advisory lines and continue but should not change the exit code from 0.

This convention matches the current code's `all_passed` boolean. The only change is: instead of returning `Ok(())` at the end, return `Ok(())` when `all_passed` is true and exit with code 1 (via `std::process::exit(1)`) when false.

Note: `run_diagnostics` returns `Result<()>`. To exit with 1 without losing the anyhow error-chain plumbing, use `std::process::exit` directly (same pattern as the TTY check in `main.rs`). Do NOT change the return type to `Result<i32>` — that would touch the call site.

- [ ] **Step 1: Write the failing test** (add to `doctor.rs` — compile-check only since we cannot run Docker in tests)

```rust
#[cfg(test)]
mod tests {
    /// Document the exit-code contract. Not runnable without Docker;
    /// presence in the test module ensures the function signature stays stable.
    #[test]
    fn run_diagnostics_signature_returns_result() {
        let _: fn() -> anyhow::Result<()> = super::run_diagnostics;
    }
}
```

Run: `cargo test --bin engrammic doctor` — expect 1 passed (compile-check only).

- [ ] **Step 2: Change the end of `run_diagnostics`**

Old (lines 98-105):
```rust
    println!();
    if all_passed {
        println!("{}", "All checks passed.".green().bold());
    } else {
        println!("{}", "Some checks need attention. See above.".yellow());
    }

    Ok(())
```

New:
```rust
    println!();
    if all_passed {
        println!("{}", "All checks passed.".green().bold());
        Ok(())
    } else {
        // Print the ✗ / → summary line before exiting.
        eprintln!("  {} One or more checks failed.", "✗".red());
        eprintln!(
            "  {} Review the items marked {} above and address them before continuing.",
            "→".yellow(),
            "red".red()
        );
        // Exit with code 1 so callers (scripts, CI) can detect unhealthy state.
        // We use process::exit rather than Err(...) to avoid printing a redundant
        // anyhow error chain — the output above is already the full diagnosis.
        std::process::exit(1);
    }
```

- [ ] **Step 3: Add the `std::process` import if not already present** (it is used in `main.rs` already; `doctor.rs` may not import it)

Check: if `doctor.rs` has no `use std::process` at the top, the inline `std::process::exit(1)` is fine without an explicit import (fully qualified path). No change needed.

- [ ] **Step 4: Test and build**

```bash
cargo test --bin engrammic doctor 2>&1 | tail -3 && cargo build 2>&1 | tail -1
```

Expected: 1 test passed, clean build.

Also update the `run_diagnostics` doc comment to document the exit codes:

```rust
/// Run all self-hosted diagnostics.
///
/// Exit codes:
///   0 — all hard checks passed (warnings may still be printed)
///   1 — one or more hard checks failed; output describes each failure
pub fn run_diagnostics() -> Result<()> {
```

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/doctor.rs
git commit -m "feat(doctor): exit 1 on hard failures, 0 for warnings-only — documented exit codes"
```

---

### Task 5: Verification pass

- [ ] **Step 1: Full test suite and build**

```bash
cargo test 2>&1 | tail -5 && cargo build 2>&1 | tail -1
```

Expected: all tests pass (fix any regressions), clean build with no NEW warnings beyond the pre-existing ones in `license`/`selfhost`/`tools`.

- [ ] **Step 2: Format**

```bash
cargo fmt -- installer-cli/src/main.rs installer-cli/src/selfhost.rs installer-cli/src/doctor.rs
```

- [ ] **Step 3: Smoke — error paths**

```bash
cargo build -q
BIN=$(pwd)/target/debug/engrammic
SCRATCH=$(mktemp -d)
mkdir -p "$SCRATCH/.claude"
```

**3a. Docker-missing path** (simulate by a fake PATH; or just read the code if no Docker-free environment):

If a Docker-free environment is available:
```bash
PATH=/usr/bin HOME="$SCRATCH" "$BIN" selfhost 2>&1 | head -10
```
Expected first two error lines:
```
  ✗ Docker is not running or not installed.
  → Install Docker Desktop from https://docs.docker.com/get-docker/ then try again.
```

**3b. License-skip path** (interactive; controller runs this manually or skips if no TTY):
```bash
HOME="$SCRATCH" "$BIN" selfhost
```
At the license prompt, press Enter with a blank input. Expected:
```
  → License skipped.
  Run engrammic license set to add your license key later.
```
Wizard should continue to next step (not abort).

**3c. License invalid format** (at the license prompt, type `BAD_KEY` and Enter):
Expected:
```
  ✗ License key must start with ENGR_
  → Check the key starts with ENGR_, is not expired, and was copied in full.
```
Then the prompt should loop (not exit).

**3d. Doctor exit code** (requires a running or non-running Docker install):
```bash
HOME="$SCRATCH" "$BIN" doctor; echo "exit: $?"
```
- If Docker is not running: expected `exit: 1` plus `✗ / →` summary at end.
- If Docker is running and healthy: expected `exit: 0` plus "All checks passed."

- [ ] **Step 4: Commit any fixes found during smoke**

```bash
git add installer-cli/src/main.rs installer-cli/src/selfhost.rs installer-cli/src/doctor.rs
git commit -m "chore(installer): phase 1c verification fixes" 2>/dev/null || echo "clean"
```
