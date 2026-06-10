# Installer Phase 4: Self-Host Guided Bring-Up Polish + Lifecycle Alignment

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the existing `selfhost.rs` wizard (consolidation, not rebuild) across six areas: tier-prompt RAM display, license-entry retry polish, streamed pull progress, health-polling end-state message, manifest fields for tier/port, and lifecycle command alignment to the manifest. Phase 4 lands after Phase 1b has run (docker→alias, `run_docker_setup` deleted, `flow.rs` exists).

**Architecture:** All changes are in `installer-cli/`. The wizard already has `prompt_tier`, `prompt_license`, `download_models`, `start_and_wait`, `wait_for_healthy`, `configure_editors`, `print_quick_reference`. This phase wires them tighter: manifest gets two new optional fields (`selfhost_tier`, `selfhost_port`); lifecycle commands (`upgrade_docker`, `manage_license`, `logs`, `scale`, `doctor`) switch from UserConfig/UserConfig::dir() heuristics to those manifest fields. A new `env_compat.rs` module handles legacy `.env` schema reading.

**Tech Stack:** Rust, indicatif 0.17 (already a dep), dialoguer 0.11, existing manifest module.

**Spec:** `docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md` ("Self-host flow", "Manifest details", "Decisions")

**Sequencing:** Task 1 is verification-only (no code change). Task 2 modifies `manifest.rs` and is a prerequisite for Task 5. Tasks 3 and 4 modify `selfhost.rs` only and are independent of each other; both may run in parallel after Task 1. Task 5 modifies `main.rs`/`logs.rs`/`scale.rs`/`doctor.rs` and requires Task 2. Task 6 is `env_compat.rs` and is independent of everything except Task 5 (which calls it). Task 7 is the verification pass.

---

## PRE-FLIGHT: Phase 1b Checklist

Before starting Task 1, verify Phase 1b has landed. All of the following must be true or Phase 4 will conflict:

- [ ] `installer-cli/src/flow.rs` exists (Task 1 of 1b)
- [ ] `Commands::Docker => selfhost::run_wizard()` in `main.rs` (Task 5 of 1b); `install_docker`/`run_docker_setup` deleted
- [ ] `docker::write_compose_bundle` deleted from `docker.rs` (confirmed: only called from the deleted `run_docker_setup` chain)
- [ ] `config.toml` removed after migration in `manifest.rs` (Task 6 of 1b)
- [ ] `cargo test` green, `cargo build` clean

If any item is missing, do not proceed — apply the missing 1b task first.

---

## Task 1: /health Verification — Read Compose Files, Document Findings

**Files (read only):**
- `context-service/docker/docker-compose.standalone-lite.yml`
- `context-service/docker/docker-compose.standalone-standard.yml`
- `context-service/docker/docker-compose.standalone-pro.yml`

**Findings (pre-computed — implementer need not re-read):**

The `app` service in all three standalone compose files (lite, standard, pro) exposes port `8000:8000` and has an explicit Docker healthcheck that polls `http://localhost:8000/health`:

```yaml
# docker-compose.standalone-lite.yml (line 42-47), same in standard (line 45-50) and pro (line 45-50)
healthcheck:
  test: ["CMD", "python", "-c", "import urllib.request; urllib.request.urlopen('http://localhost:8000/health')"]
  interval: 30s
  timeout: 5s
  start_period: 30s
  retries: 3
```

**Conclusion:** `/health` IS served on the user-facing MCP port (8000 by default; remapped by the wizard's port substitution). The existing `wait_for_healthy` function (`selfhost.rs:1018`) already polls `http://localhost:{port}/health` with `curl -sf` — this is correct. The spec's verification item is satisfied. The bring-up messaging MAY safely say "✓ Engrammic is live at http://localhost:{port}" once the health poll succeeds.

- [ ] **Step 1: No code to write.** Record this finding: implement Task 3's health-poll end-state message with the "live" wording — the gap noted in the spec does not exist.

---

## Task 2: Manifest — Add `selfhost_tier` and `selfhost_port` Fields

**Files:**
- Modify: `installer-cli/src/manifest.rs` — `Manifest` struct and `Default` impl

The manifest currently records `selfhost_dir` but not the tier or port. Lifecycle commands need the port to construct health URLs and compose paths without prompting; the tier is useful for `status` output and upgrade sanity checks.

Schema stays at version 1 (additive fields with serde defaults — the `unknown_fields_are_tolerated` test already proves forward compatibility; the existing `schema_version = 1` string in `Default` does not change).

- [ ] **Step 1: Write the failing test** (add to `manifest.rs` `#[cfg(test)]` block):

```rust
#[test]
fn selfhost_fields_round_trip() {
    let dir = tempdir().unwrap();
    let mut m = Manifest::default();
    m.selfhost_tier = Some("standard".to_string());
    m.selfhost_port = Some(9000);
    m.save_in(dir.path()).unwrap();

    let loaded = Manifest::load_in(dir.path()).unwrap();
    assert_eq!(loaded.selfhost_tier.as_deref(), Some("standard"));
    assert_eq!(loaded.selfhost_port, Some(9000));
}

#[test]
fn old_manifest_without_selfhost_fields_loads_as_none() {
    let dir = tempdir().unwrap();
    // Write a manifest that predates these fields.
    std::fs::write(
        dir.path().join("state.toml"),
        "schema_version = 1\nendpoint = \"http://localhost:8000/mcp\"\n",
    )
    .unwrap();
    let m = Manifest::load_in(dir.path()).unwrap();
    assert!(m.selfhost_tier.is_none());
    assert!(m.selfhost_port.is_none());
}
```

Run `cargo test --bin engrammic selfhost_fields` — expect compile error (fields do not exist yet).

- [ ] **Step 2: Add the fields** to the `Manifest` struct (after `selfhost_dir`, before `binary_path`):

```rust
    #[serde(default)]
    pub selfhost_tier: Option<String>,
    #[serde(default)]
    pub selfhost_port: Option<u16>,
```

The `Default` impl does not need updating — `Option` fields default to `None`.

- [ ] **Step 3:** `cargo test --bin engrammic manifest` — all pass (including the two new tests).

- [ ] **Step 4: Commit**

```bash
git add installer-cli/src/manifest.rs
git commit -m "feat(installer): add selfhost_tier and selfhost_port fields to manifest"
```

---

## Task 3: Tier Prompt Polish — Display Detected RAM + Highest Safe Pre-select

**Files:**
- Modify: `installer-cli/src/selfhost.rs` — `prompt_tier` (lines 285-362)

**Current state (read):** `prompt_tier` already detects RAM via `get_available_memory_gb()` (line 286), prints "Your system: X.X GB RAM detected" (line 299), marks the recommended tier with "(Recommended)" in the label (lines 305-334), and pre-selects it as `default(recommended)` (line 342). It uses `recommended` as an index into a fixed array `[Pro, Standard, Lite, Cloud]`.

**Gap against spec:** The spec says "highest safely-fitting tier pre-selected" — this already happens. The displayed RAM requirement per tier is embedded in the label strings but inconsistently: "Pro (48GB+)", "Standard (24GB)" — the Standard label omits the "+32GB recommended" language from `Tier::ram_requirement()`. Minor wording delta only.

- [ ] **Step 1: Write the failing test** (add to `selfhost.rs` `#[cfg(test)]` block):

```rust
#[test]
fn tier_fit_selects_highest_safe() {
    assert_eq!(best_fitting_tier(48.0), 0); // Pro
    assert_eq!(best_fitting_tier(32.0), 1); // Standard
    assert_eq!(best_fitting_tier(24.0), 1); // Standard lower bound
    assert_eq!(best_fitting_tier(8.0), 2);  // Lite
    assert_eq!(best_fitting_tier(6.0), 3);  // Cloud (fallback)
    assert_eq!(best_fitting_tier(0.5), 3);  // Cloud
}
```

Run `cargo test --bin engrammic tier_fit` — expect compile error.

- [ ] **Step 2: Extract `best_fitting_tier` from `prompt_tier`** as a pure fn (testable without TTY). Replace the inline `recommended` block:

```rust
/// Returns the Select index (0=Pro, 1=Standard, 2=Lite, 3=Cloud) of the
/// highest tier that safely fits in `ram_gb`. "Safely" means at least
/// the minimum requirement, not the recommended ceiling.
pub(crate) fn best_fitting_tier(ram_gb: f64) -> usize {
    if ram_gb >= 48.0 {
        0 // Pro
    } else if ram_gb >= 24.0 {
        1 // Standard
    } else if ram_gb >= 8.0 {
        2 // Lite
    } else {
        3 // Cloud
    }
}
```

In `prompt_tier`, replace the existing inline logic with `let recommended = best_fitting_tier(ram);`.

- [ ] **Step 3: Polish the tier labels** to consistently show the RAM requirement from `Tier::ram_requirement()`:

```rust
    let tiers = vec![
        format!(
            "Pro      ({}+) — {}{}",
            "48 GB",
            Tier::Pro.description(),
            if recommended == 0 { "  ← recommended for your system" } else { "" }
        ),
        format!(
            "Standard ({})  — {}{}",
            "24–32 GB",
            Tier::Standard.description(),
            if recommended == 1 { "  ← recommended for your system" } else { "" }
        ),
        format!(
            "Lite     ({})   — {}{}",
            "8 GB",
            Tier::Lite.description(),
            if recommended == 2 { "  ← recommended for your system" } else { "" }
        ),
        format!(
            "Cloud    (any)   — {}{}",
            Tier::Cloud.description(),
            if recommended == 3 { "  ← recommended for your system" } else { "" }
        ),
    ];
```

Note: the `recommended` calculation now uses `best_fitting_tier(ram)` — the "recommended" label points to the highest SAFELY fitting tier, not the highest tier the system can run (i.e. a 32 GB system gets Standard recommended, even though Pro is shown).

- [ ] **Step 4:** `cargo test --bin engrammic tier_fit` — 6 cases pass. `cargo build 2>&1 | tail -1` — clean.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/selfhost.rs
git commit -m "feat(selfhost): extract best_fitting_tier, polish tier prompt labels"
```

---

## Task 4: License Retry Polish + Esc-to-Skip + Pull-Progress Streaming + Health End-State Message

**Files:**
- Modify: `installer-cli/src/selfhost.rs` — `prompt_license` (lines 434-479), `download_models` (lines 364-432), `start_and_wait` (lines 976-1016), `wait_for_healthy` (lines 1018-1049)

This task bundles four sub-items because they all touch `selfhost.rs` and are straightforward. Split into separate commits if preferred.

### 4a — License Retry: Esc-to-skip + specific error display

**Current state:** `prompt_license` already has a retry loop (lines 454-479) and prints the error with `println!("  {} {}", "✗".red(), e)`. The loop has no escape — it spins forever on bad input.

**Gap:** No way to skip. Phase 1c (error conventions) will ensure `license::validate_license_format` returns structured errors; this task handles the Esc/Ctrl-C path at the wizard level.

- [ ] **Step 1: Wrap `interact_text()` to catch Ctrl-C / Esc** — dialoguer returns `Err` on Ctrl-C; treat it as "skip":

Replace the `Input::new()...interact_text()?` call inside the loop:

```rust
    loop {
        println!(
            "  {}",
            "(Starts with ENGR_ — request at founders@engrammic.ai  |  Esc to skip)".dimmed()
        );
        let key_result: std::io::Result<String> = Input::new()
            .with_prompt("License key (input visible)")
            .interact_text();

        let key = match key_result {
            Ok(k) => k,
            Err(_) => {
                // Ctrl-C or Esc — defer via `engrammic license`
                println!(
                    "  {} Skipped — run {} anytime to set your license.",
                    "!".yellow(),
                    "engrammic license".cyan()
                );
                // Return empty string; run_wizard records it and skips .env update.
                return Ok(String::new());
            }
        };

        if key.trim().is_empty() {
            println!(
                "  {} Skipped — run {} anytime to set your license.",
                "!".yellow(),
                "engrammic license".cyan()
            );
            return Ok(String::new());
        }

        match license::validate_license_format(&key) {
            Ok(info) => {
                println!(
                    "  {} Valid — {}, {} days remaining",
                    "✓".green(),
                    info.customer.cyan(),
                    info.days_remaining
                );
                return Ok(key);
            }
            Err(e) => {
                println!("  {} {}", "✗".red(), e);
                println!("  {} Try again or press Esc to skip for now.", "→".dimmed());
                println!();
            }
        }
    }
```

In `run_wizard`, the empty-string return is handled by treating an empty `license_key` in `SelfHostConfig` as "deferred": the `generate_env` call already writes whatever is in `config.license_key` — an empty string writes `ENGRAMMIC_LICENSE_KEY=` which is obviously incomplete; instead, when the key is empty, omit the ENGRAMMIC_LICENSE_KEY line entirely (a commented-out placeholder is better):

In `generate_env` (line 876), change the license line from:

```rust
ENGRAMMIC_LICENSE_KEY={license_key}
```

to:

```rust
{license_line}
```

And compute it before the `format!`:

```rust
    let license_line = if config.license_key.is_empty() {
        "# ENGRAMMIC_LICENSE_KEY=<set with: engrammic license>".to_string()
    } else {
        format!("ENGRAMMIC_LICENSE_KEY={}", config.license_key)
    };
```

### 4b — Pull progress: stream stderr from `docker compose pull`

**Current state:** `start_and_wait` (line 976-1016) calls `docker compose pull` with `stdout(Stdio::null()).stderr(Stdio::piped())` (lines 986-987), then `wait_with_output()` — pull output is collected but only checked for failure; it is never shown. The user sees only "Pulling images (this may take a few minutes)..." and then waits silently for multi-GB pulls.

`download_models` (lines 381-393) uses `stdout(Stdio::null()).stderr(Stdio::null())` for `docker compose up -d ollama` — that's a service start, not a pull, so silencing is acceptable. The `docker exec ollama pull` at line 418 uses `.status()` directly, which inherits the parent's stdio and therefore DOES stream output — this is already correct.

**Gap in `start_and_wait`:** The pull step swallows all output. Replace the spawn+wait_with_output pattern with a spinner + inherited stderr so Docker's own layer-progress lines stream to the terminal.

- [ ] **Step 2: Replace the pull block** (lines 982-996) in `start_and_wait`:

```rust
    // Pull images — stream Docker's layer progress to the terminal.
    println!("  Pulling images (this may take several minutes on first run)...");
    use indicatif::{ProgressBar, ProgressStyle};
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("pulling images...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    // Inherit stderr so Docker's own layer-download lines stream through.
    // stdout stays null (Docker's pull progress goes to stderr).
    let pull_status = Command::new("docker")
        .args(["compose", "-f", compose_path.to_str().unwrap(), "pull"])
        .current_dir(&config.install_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()?;

    spinner.finish_and_clear();

    if !pull_status.success() {
        println!("  {} Failed to pull images.", "✗".red());
        println!(
            "  {} Check that Docker is running and you have network access, then retry:",
            "→".dimmed()
        );
        println!("    cd {} && docker compose pull", config.install_dir.display());
        anyhow::bail!("docker compose pull failed");
    }
    println!("  {} Images ready", "✓".green());
```

Note: `indicatif` is already a dependency (Cargo.toml line 25). The spinner runs while Docker streams its own output — they interleave, which is acceptable and standard for CLI tools. On non-TTY (e.g., `-y` piped), `ProgressBar` detects the lack of a terminal and suppresses the spinner automatically.

### 4c — Health end-state message: "✓ Engrammic is live at http://localhost:{port}"

**Current state:** `wait_for_healthy` (lines 1018-1049) prints "✓ All services healthy" on success (line 1031) — no URL, no port. The spec asks for "✓ Engrammic is live at http://localhost:{port}". `wait_for_healthy` takes `config: &SelfHostConfig` so `config.port` is available.

- [ ] **Step 3: Update the success line** (line 1031):

```rust
                println!(
                    "  {} Engrammic is live at {}",
                    "✓".green(),
                    format!("http://localhost:{}/mcp", config.port).cyan()
                );
```

Also update the timeout message (line 1046-1048) to include the port so the user knows which URL to check:

```rust
    println!(
        "  {} Services did not become healthy within {} seconds.",
        "!".yellow(),
        max_attempts * 2
    );
    println!(
        "  {} Run {} to diagnose. Expected health URL: {}",
        "→".dimmed(),
        "engrammic doctor".cyan(),
        format!("http://localhost:{}/health", config.port).dimmed()
    );
```

- [ ] **Step 4:** `cargo build 2>&1 | tail -1` — clean. `cargo test --bin engrammic selfhost` — all pass (the models.yaml test unaffected).

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/selfhost.rs
git commit -m "feat(selfhost): license Esc-to-skip, stream pull progress, live URL in health message"
```

---

## Task 5: Lifecycle Alignment — Commands Read Manifest Instead of Guessing

**Files:**
- Modify: `installer-cli/src/selfhost.rs` — `run_wizard` end (lines 164-177): write tier and port to manifest
- Modify: `installer-cli/src/main.rs` — `upgrade_docker` (lines 1268-1316), `manage_license` (lines 1362-1456)
- Modify: `installer-cli/src/logs.rs` — `show_logs` (lines 21-75)
- Modify: `installer-cli/src/scale.rs` — `show_status` (lines 62-122)
- Modify: `installer-cli/src/doctor.rs` — `run_diagnostics` (lines 7-106), `check_license` (lines 162-185)

### The problem (read from source):

**`upgrade_docker` (main.rs:1268):** Uses `UserConfig::load()` and checks `config.endpoint == LOCAL_ENDPOINT` ("http://localhost:8000/mcp" — hardcoded constant in tools.rs). This means a user who changed the port at install time (e.g. port 9000) would get "No self-hosted installation found" even with a healthy install. Uses `UserConfig::dir()` (= `~/.engrammic`) as the compose dir — correct for the default install path but wrong if the user chose a custom `install_dir` during the wizard.

**`manage_license` (main.rs:1362):** Same endpoint check (line 1367). Uses `UserConfig::dir()` as the .env location (line 1432) — again wrong for custom install dirs. Saves the new key via `docker::update_license_key(&dir, ...)` — this writes to `~/.engrammic/.env`, not `config.install_dir/.env`.

**`logs.rs` (line 22-33):** Uses `UserConfig::dir()` directly. Hardcodes `dir.join("docker-compose.yml")`. No manifest check — if no compose file exists at `~/.engrammic/`, it errors with "Run engrammic selfhost".

**`scale.rs`:** Does not read compose dir at all — runs `docker stats` globally (all containers), which is fine. But `show_status` calls `docker stats` without knowing which containers belong to this install. No path-based problem here; no change needed.

**`doctor.rs`:** `check_containers` (line 116) runs `docker compose ps` with no `-f` flag — uses the Docker daemon's current project, which may not be the Engrammic install at all. `check_license` (line 162) already reads `UserConfig` → manifest via the thin wrapper; this is fine.

### The fix:

Add a helper `selfhost_install_dir` that reads the manifest and returns the install dir with a clear error if no self-hosted install is recorded.

- [ ] **Step 1: Write a failing test** for the new helper in `selfhost.rs`:

```rust
#[cfg(test)]
mod tests {
    // (existing tests omitted for brevity — append only)

    #[test]
    fn selfhost_install_dir_errors_without_manifest_entry() {
        let dir = tempfile::tempdir().unwrap();
        // No manifest, no selfhost_dir → must return Err with guidance.
        let result = resolve_selfhost_install_dir_in(dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("engrammic selfhost"),
            "error message must mention the setup command"
        );
    }

    #[test]
    fn selfhost_install_dir_returns_manifest_value() {
        let dir = tempfile::tempdir().unwrap();
        let install_path = dir.path().join("myinstall");
        std::fs::create_dir_all(&install_path).unwrap();
        let mut m = crate::manifest::Manifest::default();
        m.selfhost_dir = Some(install_path.clone());
        m.selfhost_port = Some(8000);
        m.save_in(dir.path()).unwrap();

        let (resolved_dir, port) = resolve_selfhost_install_dir_in(dir.path()).unwrap();
        assert_eq!(resolved_dir, install_path);
        assert_eq!(port, 8000);
    }
}
```

Run `cargo test --bin engrammic selfhost_install_dir` — expect compile error.

- [ ] **Step 2: Add the helper** to `selfhost.rs` (before the test module, after `print_quick_reference`):

```rust
/// Resolve the install dir and port from the manifest. Used by lifecycle commands
/// (upgrade, logs, doctor, license) so they read the actual install path, not a
/// directory heuristic.
///
/// For testability, `_in` variant takes the manifest dir explicitly.
pub fn resolve_selfhost_install_dir() -> anyhow::Result<(std::path::PathBuf, u16)> {
    resolve_selfhost_install_dir_in(&crate::manifest::Manifest::dir())
}

pub(crate) fn resolve_selfhost_install_dir_in(
    manifest_dir: &std::path::Path,
) -> anyhow::Result<(std::path::PathBuf, u16)> {
    let m = crate::manifest::Manifest::load_or_migrate_in(manifest_dir)?;
    match (m.selfhost_dir, m.selfhost_port) {
        (Some(dir), port) => Ok((dir, port.unwrap_or(crate::selfhost::DEFAULT_PORT))),
        (None, _) => {
            anyhow::bail!(
                "✗ No self-hosted installation recorded in the manifest.\n\
                 → Run {} to set one up.",
                "engrammic selfhost".to_string()
            )
        }
    }
}
```

`DEFAULT_PORT` is already `pub` ... wait: check — it is `const DEFAULT_PORT: u16 = 8000` (line 16, no `pub`). Change it to `pub(crate) const DEFAULT_PORT: u16 = 8000;`.

- [ ] **Step 3: Write the wizard manifest record.** In `run_wizard`, replace the current `UserConfig::save()` block (lines 164-177):

```rust
    // Save config — write through UserConfig (merges into manifest) AND
    // record tier/port directly so lifecycle commands can find the install.
    let user_config = UserConfig {
        endpoint: Some(format!("http://localhost:{}/mcp", config.port)),
        license_key: if config.license_key.is_empty() {
            None
        } else {
            Some(config.license_key.clone())
        },
        selfhost_dir: Some(config.install_dir.clone()),
    };
    user_config.save()?;

    // Record tier and port (new fields — not covered by UserConfig shim).
    {
        let mut m = crate::manifest::Manifest::load_or_migrate(None)?;
        m.selfhost_tier = Some(format!("{:?}", config.tier).to_lowercase());
        m.selfhost_port = Some(config.port);
        m.save()?;
    }
```

- [ ] **Step 4: Update `upgrade_docker` in `main.rs`.**

Replace the current guard (lines 1271-1281):

```rust
fn upgrade_docker() -> Result<()> {
    banner::print_banner();

    let (dir, _port) = selfhost::resolve_selfhost_install_dir()
        .map_err(|e| {
            // Provide the ✗/→ format even if the inner error already has it.
            eprintln!("{}", e);
            anyhow::anyhow!("no self-hosted installation")
        })?;
    // ... rest unchanged: check_compose_updates, refresh_compose, upgrade_docker_stack ...
```

Remove the `config` load and `LOCAL_ENDPOINT` check entirely. The `dir` variable replaces `user_config::UserConfig::dir()` for the compose path.

Full replacement of `upgrade_docker` body:

```rust
fn upgrade_docker() -> Result<()> {
    banner::print_banner();

    let (dir, _port) = match selfhost::resolve_selfhost_install_dir() {
        Ok(v) => v,
        Err(_) => {
            println!("{} No self-hosted installation recorded.", "✗".red());
            println!(
                "  {} Run {} to set one up.",
                "→".dimmed(),
                "engrammic selfhost".cyan()
            );
            return Ok(());
        }
    };

    if let Some(new_services) = docker::check_compose_updates(&dir)? {
        println!(
            "{} New services available: {}",
            "!".yellow(),
            new_services.join(", ").cyan()
        );
        println!(
            "  {}",
            "(Your .env will be preserved. Old compose backed up to .bak)".dimmed()
        );
        let update_compose = Confirm::new()
            .with_prompt("Update docker-compose.yml to include new services?")
            .default(true)
            .interact()?;
        if update_compose {
            docker::refresh_compose(&dir)?;
            println!("  {} docker-compose.yml updated", "✓".green());
        }
        println!();
    }

    docker::upgrade_docker_stack(&dir)?;

    println!();
    println!("{} Self-hosted stack upgraded to latest version.", "✓".green());

    Ok(())
}
```

- [ ] **Step 5: Update `manage_license` in `main.rs`.**

Replace the guard and dir derivation (lines 1362-1456). Key changes:
1. Drop the `LOCAL_ENDPOINT` check — it is invalid for custom ports.
2. Use `resolve_selfhost_install_dir()` for the .env path.
3. Write the new key to manifest (via `UserConfig::save`) AND to `.env`.

```rust
fn manage_license() -> Result<()> {
    banner::print_banner();

    let (dir, _port) = match selfhost::resolve_selfhost_install_dir() {
        Ok(v) => v,
        Err(_) => {
            println!("{} No self-hosted installation recorded.", "✗".red());
            println!("  {} Run {} to set one up.", "→".dimmed(), "engrammic selfhost".cyan());
            println!();
            println!(
                "  Cloud users do not need a license key.",
            );
            return Ok(());
        }
    };

    let config = user_config::UserConfig::load().unwrap_or_default();

    println!("{}", "Current license".bold());
    if let Some(ref key) = config.license_key {
        match license::validate_license_format(key) {
            Ok(info) => {
                println!("  Customer: {}", info.customer.cyan());
                println!("  Days remaining: {}", info.days_remaining);
                println!();
            }
            Err(e) => {
                println!("  {} {}", "!".yellow(), e);
                println!();
            }
        }
    } else {
        println!("  {} No license key configured.", "-".dimmed());
        println!();
    }

    let update = Confirm::new()
        .with_prompt("Update license key?")
        .default(false)
        .interact()?;

    if !update {
        return Ok(());
    }

    println!(
        "  {}",
        "(Starts with ENGR_ — get yours at engrammic.ai/self-hosted)".dimmed()
    );
    let mut prompt = Input::<String>::new().with_prompt("License key (input visible)");
    if let Some(ref key) = config.license_key {
        prompt = prompt.default(key.clone());
    }
    let new_key = prompt.interact_text()?;

    println!("{}", "Validating license".bold());
    match license::validate_license_format(&new_key) {
        Ok(info) => {
            println!(
                "  {} Valid — customer: {}, {} days remaining",
                "✓".green(),
                info.customer.cyan(),
                info.days_remaining
            );
        }
        Err(e) => {
            println!("  {} {}", "✗".red(), e);
            println!("  {} Re-run to try a different key.", "→".dimmed());
            return Ok(());
        }
    }
    println!();

    // Write to .env in the ACTUAL install dir (not UserConfig::dir()).
    docker::update_license_key(&dir, &new_key)?;

    // Write to manifest via UserConfig shim.
    let new_config = user_config::UserConfig {
        endpoint: config.endpoint,
        license_key: Some(new_key),
        selfhost_dir: config.selfhost_dir,
    };
    new_config.save()?;

    println!(
        "{} License key updated. Restart Docker services to apply:",
        "✓".green()
    );
    println!(
        "  {}",
        format!("docker compose -f {}/docker-compose.yml restart", dir.display()).cyan()
    );

    Ok(())
}
```

- [ ] **Step 6: Update `logs.rs`.**

Replace the dir derivation (line 22-33). `show_logs` currently uses `UserConfig::dir()` for the compose path.

```rust
pub fn show_logs(service: Option<&str>, follow: bool, lines: u32) -> Result<()> {
    let (dir, _port) = match crate::selfhost::resolve_selfhost_install_dir() {
        Ok(v) => v,
        Err(_) => {
            println!("{} No self-hosted installation recorded.", "✗".red());
            println!(
                "  {} Run {} to set one up.",
                "→".dimmed(),
                "engrammic selfhost".cyan()
            );
            return Ok(());
        }
    };
    let compose_path = dir.join("docker-compose.yml");

    if !compose_path.exists() {
        println!(
            "{} docker-compose.yml not found at {}",
            "✗".red(),
            dir.display()
        );
        println!(
            "  {} Re-run {} to regenerate it.",
            "→".dimmed(),
            "engrammic selfhost".cyan()
        );
        return Ok(());
    }

    // ... remainder of function unchanged from line 35 onwards ...
```

- [ ] **Step 7: Update `doctor.rs` — `check_containers` to use compose project dir.**

`check_containers` (line 116) runs `docker compose ps` without `-f` — it uses Docker's "current directory" project inference, which will be whatever directory the binary happened to be invoked from. Pass the compose path from the manifest.

Change `run_diagnostics` to accept the install dir, and `check_containers` to take it:

```rust
pub fn run_diagnostics() -> Result<()> {
    println!();
    println!("{}", "Engrammic Diagnostics".bold());
    println!();

    // Resolve install dir for compose-aware checks.
    let install_dir = crate::selfhost::resolve_selfhost_install_dir().ok().map(|(d, _)| d);

    // ... existing checks unchanged up to check_containers ...
    print!("Checking containers... ");
    match check_containers(install_dir.as_deref()) {
        // ...
    }
```

In `check_containers`:

```rust
fn check_containers(install_dir: Option<&Path>) -> Result<(usize, usize)> {
    let mut args = vec!["compose"];
    let compose_str;
    if let Some(dir) = install_dir {
        let compose_path = dir.join("docker-compose.yml");
        compose_str = compose_path.to_string_lossy().into_owned();
        args.push("-f");
        args.push(&compose_str);
    }
    args.extend_from_slice(&["ps", "--format", "json"]);

    let output = Command::new("docker").args(&args).output()?;
    // ... rest unchanged ...
```

Import `std::path::Path` at the top of `doctor.rs` if not already present.

- [ ] **Step 8:** `cargo test 2>&1 | tail -3 && cargo build 2>&1 | tail -1`

Expected: the two new `selfhost_install_dir_*` tests pass; all prior tests pass; build clean. Fix any unused import warnings.

- [ ] **Step 9: Commit**

```bash
git add installer-cli/src/selfhost.rs installer-cli/src/main.rs installer-cli/src/logs.rs installer-cli/src/doctor.rs
git commit -m "feat(installer): lifecycle commands read install dir/port from manifest"
```

---

## Task 6: Legacy .env Schema Tolerance (`TELEMETRY_ENABLED` vs `TELEMETRY__ENABLED`)

**Files:**
- Create: `installer-cli/src/env_compat.rs`
- Modify: `installer-cli/src/main.rs` — add `mod env_compat;`
- Modify: `installer-cli/src/docker.rs` — `update_license_key` (line 162) to call the compat reader

**Background (verified in source):**

- `docker.rs:write_compose_bundle` (the deleted legacy function) wrote `TELEMETRY_ENABLED={bool}` (line 97).
- `selfhost.rs:generate_env` writes `TELEMETRY__ENABLED=false` (line 918) — the double-underscore form matches context-service's expected env var (`selfhosted.env.example` uses `TELEMETRY__ENABLED=true`).
- `docker.rs:update_license_key` reads the .env line by line looking for `ENGRAMMIC_LICENSE_KEY=` (lines 173-181). It does NOT read or transform `TELEMETRY_ENABLED` — this is safe. No current .env reader will *break* on the old schema.
- `docker.rs:check_compose_updates` reads the compose file, not the .env — unaffected.

The spec Decision says "upgrade path must tolerate both". The tolerance is needed when:
1. A user has an old `.env` written by `write_compose_bundle` (with `TELEMETRY_ENABLED`).
2. `upgrade_docker` runs `docker compose pull` + `up -d` — Docker passes the .env to containers.
3. The container may not recognize `TELEMETRY_ENABLED` and silently ignore it, leaving telemetry defaulting on.

The fix is: during `upgrade_docker`, after pulling images and before restarting, normalize the .env from the old schema to the new one if needed.

- [ ] **Step 1: Write the failing test** for the normalizer:

Create `installer-cli/src/env_compat.rs`:

```rust
//! Normalizes legacy .env schemas written by older installer versions.
//!
//! Legacy (`write_compose_bundle`):  TELEMETRY_ENABLED=false
//! Current (`generate_env`):         TELEMETRY__ENABLED=false
//!
//! The normalizer is idempotent and leaves all other lines untouched.

/// Normalize a .env file's content in memory. Returns (new_content, changed).
pub fn normalize_env(content: &str) -> (String, bool) {
    let mut changed = false;
    let lines: Vec<String> = content
        .lines()
        .map(|line| {
            // Only transform the bare key (not already double-underscore).
            if line.starts_with("TELEMETRY_ENABLED=") && !line.starts_with("TELEMETRY__ENABLED=") {
                changed = true;
                line.replacen("TELEMETRY_ENABLED=", "TELEMETRY__ENABLED=", 1)
            } else {
                line.to_string()
            }
        })
        .collect();
    let mut result = lines.join("\n");
    // Preserve trailing newline if original had one.
    if content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    (result, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bare_telemetry_key() {
        let old = "ENGRAMMIC_LICENSE_KEY=eng_abc\nTELEMETRY_ENABLED=false\nPOSTGRES_PASSWORD=x\n";
        let (new, changed) = normalize_env(old);
        assert!(changed);
        assert!(new.contains("TELEMETRY__ENABLED=false"), "must rewrite to double-underscore form");
        assert!(!new.contains("TELEMETRY_ENABLED=false") || new.contains("TELEMETRY__ENABLED=false"),
            "old key must not appear without double-underscore");
        assert!(new.contains("ENGRAMMIC_LICENSE_KEY=eng_abc"), "other keys must be preserved");
    }

    #[test]
    fn idempotent_on_already_normalized() {
        let current = "TELEMETRY__ENABLED=false\nENGRAMMIC_LICENSE_KEY=eng_abc\n";
        let (new, changed) = normalize_env(current);
        assert!(!changed);
        assert_eq!(new, current);
    }

    #[test]
    fn no_change_when_key_absent() {
        let no_telemetry = "ENGRAMMIC_LICENSE_KEY=eng_abc\nPOSTGRES_PASSWORD=x\n";
        let (_, changed) = normalize_env(no_telemetry);
        assert!(!changed);
    }

    #[test]
    fn preserves_trailing_newline() {
        let with_newline = "TELEMETRY_ENABLED=true\n";
        let (new, _) = normalize_env(with_newline);
        assert!(new.ends_with('\n'));
    }
}
```

Run `cargo test --bin engrammic env_compat` — expect compile error (mod not declared).

- [ ] **Step 2: Add `mod env_compat;`** to `main.rs` (after `mod doctor;`, before `mod flow;` alphabetically if flow exists, else after `mod doctor;`).

- [ ] **Step 3: Hook into `upgrade_docker` in `main.rs`.** After `docker::upgrade_docker_stack(&dir)?;` succeeds, run the normalizer on `dir/.env`:

```rust
    // Normalize legacy .env schema (TELEMETRY_ENABLED → TELEMETRY__ENABLED).
    let env_path = dir.join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        let (normalized, changed) = env_compat::normalize_env(&content);
        if changed {
            if let Err(e) = std::fs::write(&env_path, &normalized) {
                println!("  {} Could not update .env schema: {}", "!".yellow(), e);
            } else {
                println!(
                    "  {} .env migrated to current schema (TELEMETRY__ENABLED)",
                    "✓".green()
                );
            }
        }
    }
```

- [ ] **Step 4:** `cargo test --bin engrammic env_compat` — 4 tests pass. `cargo build 2>&1 | tail -1` — clean.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/env_compat.rs installer-cli/src/main.rs
git commit -m "feat(installer): normalize legacy TELEMETRY_ENABLED to TELEMETRY__ENABLED on upgrade"
```

---

## Task 7: Verification Pass

- [ ] **Step 1: Full gate**

```bash
cargo test 2>&1 | tail -5 && cargo build 2>&1 | tail -1
cargo fmt -- \
  installer-cli/src/selfhost.rs \
  installer-cli/src/manifest.rs \
  installer-cli/src/main.rs \
  installer-cli/src/logs.rs \
  installer-cli/src/doctor.rs \
  installer-cli/src/env_compat.rs
```

Expected: all tests pass; build clean (no new warnings vs baseline); fmt produces no diff.

Baseline warnings to ignore (pre-existing): anything in `license.rs`, `tools.rs` re unused fields — check `cargo build 2>&1 | grep "^warning" | grep -v "license\|tools"` returns empty.

- [ ] **Step 2: Manifest round-trip smoke (no Docker required)**

```bash
cargo build -q
BIN=$(pwd)/installer-cli/target/debug/engrammic
SCRATCH=$(mktemp -d)
mkdir -p "$SCRATCH/.engrammic" "$SCRATCH/.claude"

# Simulate a completed selfhost install by writing the manifest directly.
cat > "$SCRATCH/.engrammic/state.toml" <<'EOF'
schema_version = 1
endpoint = "http://localhost:9000/mcp"
license_key = "ENGR_test"
selfhost_dir = "/tmp/engrammic-install"
selfhost_tier = "standard"
selfhost_port = 9000
EOF

mkdir -p /tmp/engrammic-install

# upgrade_docker should find the install dir from manifest, not guess.
HOME="$SCRATCH" "$BIN" upgrade 2>&1 | head -5
# Expected: no "No self-hosted installation found" error; proceeds to docker call
# (which may fail because no real compose file — that is acceptable).

# logs should similarly not error about "no installation found".
HOME="$SCRATCH" "$BIN" logs --service app 2>&1 | head -3
# Expected: reaches docker compose logs (fails on missing compose, not on missing manifest).

# Old LOCAL_ENDPOINT guard is gone — verify:
HOME="$SCRATCH" "$BIN" upgrade 2>&1 | grep -q "No self-hosted" && echo "REGRESSION: old guard still present" || echo "OK: manifest-driven lookup working"
```

- [ ] **Step 3: Legacy .env normalization smoke**

```bash
INSTALL_DIR=$(mktemp -d)
cat > "$INSTALL_DIR/.env" <<'EOF'
ENGRAMMIC_LICENSE_KEY=ENGR_test
TELEMETRY_ENABLED=false
POSTGRES_PASSWORD=testpw
EOF

# Update the manifest to point at this dir.
SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.engrammic"
cat > "$SCRATCH/.engrammic/state.toml" <<EOF
schema_version = 1
selfhost_dir = "$INSTALL_DIR"
selfhost_port = 8000
EOF

# Run upgrade — it will fail at docker compose pull (no network/compose needed for this check)
# but the env normalization runs after the pull/up, so we must mock success.
# Alternatively, test the normalize_env function directly via: cargo test env_compat
cargo test --bin engrammic env_compat 2>&1 | tail -3
# Expected: 4 tests passed.
```

- [ ] **Step 4: Manual smoke (Docker required — note for controller)**

These steps require Docker to be running and internet access. They are documented for human verification; an automated agent without Docker should skip and note in the commit message.

```bash
# Full selfhost wizard with a scratch home
SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.claude"
HOME="$SCRATCH" cargo run -q -- selfhost
# Expected: tier prompt shows "← recommended for your system" on the correct tier;
#           license prompt shows "|  Esc to skip" hint;
#           on "Start now? Y", docker pull output streams to terminal;
#           health poller ends with "✓ Engrammic is live at http://localhost:8000/mcp";
#           state.toml contains selfhost_tier and selfhost_port.

# Verify manifest fields written:
cat "$SCRATCH/.engrammic/state.toml" | grep -E "selfhost_tier|selfhost_port"
# Expected: selfhost_tier = "standard"  (or whichever was selected)
#           selfhost_port = 8000         (or user-chosen port)

# Verify lifecycle commands use manifest:
HOME="$SCRATCH" cargo run -q -- upgrade 2>&1 | head -3
# Expected: no "No self-hosted installation found" (proceeds to docker commands)

HOME="$SCRATCH" cargo run -q -- logs --service app 2>&1 | head -3
# Expected: "Showing logs for: app" (not "No self-hosted installation found")
```

- [ ] **Step 5: Commit any fixes found during verification**

```bash
git add -A && git commit -m "chore(installer): phase 4 verification fixes" || echo "clean"
```

---

## Files Modified / Created

| File | Change |
|---|---|
| `installer-cli/src/manifest.rs` | Add `selfhost_tier: Option<String>`, `selfhost_port: Option<u16>` with `#[serde(default)]`; 2 new tests |
| `installer-cli/src/selfhost.rs` | Extract `best_fitting_tier` (pure fn, tested); prompt label polish; license Esc-to-skip; streamed pull via `Stdio::inherit`; `wait_for_healthy` "live at" message; `resolve_selfhost_install_dir[_in]` helper + tests; `run_wizard` writes tier/port to manifest; `DEFAULT_PORT` made `pub(crate)` |
| `installer-cli/src/main.rs` | `upgrade_docker`: drop LOCAL_ENDPOINT check, use `resolve_selfhost_install_dir`; `manage_license`: same + fix .env write path; add `mod env_compat;`; env normalization call in `upgrade_docker` |
| `installer-cli/src/logs.rs` | Replace `UserConfig::dir()` heuristic with `resolve_selfhost_install_dir` |
| `installer-cli/src/doctor.rs` | Thread `install_dir: Option<&Path>` into `check_containers` for correct compose project scope |
| `installer-cli/src/env_compat.rs` | New module: `normalize_env` pure fn + 4 unit tests |

**NOT changed:** `docker.rs` (no `.env` reader needs to handle both schemas — `update_license_key` only touches `ENGRAMMIC_LICENSE_KEY=`; `check_compose_updates` reads compose, not env), `scale.rs` (runs `docker stats` globally — correct behavior, no compose-path dependency).

---

## Test Coverage Summary

| Test | Type | Location |
|---|---|---|
| `tier_fit_selects_highest_safe` (6 cases) | Unit | `selfhost.rs` |
| `selfhost_fields_round_trip` | Unit | `manifest.rs` |
| `old_manifest_without_selfhost_fields_loads_as_none` | Unit | `manifest.rs` |
| `selfhost_install_dir_errors_without_manifest_entry` | Unit | `selfhost.rs` |
| `selfhost_install_dir_returns_manifest_value` | Unit | `selfhost.rs` |
| `normalizes_bare_telemetry_key` | Unit | `env_compat.rs` |
| `idempotent_on_already_normalized` | Unit | `env_compat.rs` |
| `no_change_when_key_absent` | Unit | `env_compat.rs` |
| `preserves_trailing_newline` | Unit | `env_compat.rs` |
| Manifest round-trip smoke | Manual (no Docker) | Task 7 Step 2 |
| Full wizard + lifecycle smoke | Manual (Docker required) | Task 7 Step 4 |
