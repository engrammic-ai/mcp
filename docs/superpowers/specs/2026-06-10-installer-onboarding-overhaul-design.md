# Installer & Onboarding Overhaul — Design

**Date:** 2026-06-10
**Scope:** `mcp-client/installer` (install.sh / install.ps1), `mcp-client/installer-cli` (Rust), `web/join` (onboarding page). Cross-repo: join-page changes land in `../web/join`.
**Status:** Approved pending review

## Problem

The flagship onboarding path (`join.engrammic.ai` → `curl | sh`) breaks in practice:

- `install.sh` hardcodes the `install` subcommand when invoked with no args, and the join page's curl box uses plain `| bash` with no `-s --` scaffold — so `-y` is effectively unreachable from the pipe (note: `install.ps1` already passes args via `@args`; the sh-side gap is default-subcommand handling plus join-page documentation). Missing `/dev/tty` crashes prompts before any guidance appears.
- The binary runs from `/tmp`; persistence to `~/.local/bin` is an opt-in final prompt that `-y` mode silently skips — non-interactive installs leave **no** `engrammic` command behind. PATH is hinted, never written.
- No checksum verification of the downloaded binary.
- The wizard is a linear chain of `?`/`bail!` calls: a mistyped license key aborts the whole run (losing the specific validation error), a failure on harness #3 abandons the rest, and nothing records what was done — which is also why **uninstall / per-harness removal does not exist**.
- Self-hosted setup writes compose + .env then abandons the user before the hard part (multi-GB model pulls, bring-up, verification).
- `web/join` has two contradictory design docs (JOIN-UX-SPEC.md says remove curl; PLAN-onboarding-improvements.md says curl-primary; implementation follows the plan).
- The curl audience includes many non-technical users; current copy and error output assume developer fluency.

## Decisions made

- **No rewrite.** Pain points are flow-architecture problems, not language problems. Keep the Rust core; the inquire→dialoguer/TTY learning is sunk value. `npx @engrammic/install` ships later as a thin npm wrapper (esbuild platform-binary pattern) over the same binaries — a distribution shim, not a port.
- **Curl-primary stands.** PLAN-onboarding-improvements.md wins; JOIN-UX-SPEC.md is archived or updated to match.
- **License key input stays visible** (deliberate: aids entry confirmation; not treated as a maskable secret).
- **Self-host depth:** guided bring-up by default with hands-off as an explicit choice; lifecycle commands aligned to the manifest (rustup model: one obvious path, options revealed not required).
- **Uninstall includes a best-effort legacy scan** for pre-manifest installs, with per-item confirmation.

Decisions added after design review (2026-06-10, Opus reviewer):

- **`engrammic selfhost` is the canonical self-host flow; `engrammic docker` becomes an alias for it.** `run_docker_setup` and the `docker.rs` compose bundle are removed; migration code must read both legacy `.env` schemas (`TELEMETRY_ENABLED` vs `TELEMETRY__ENABLED`, divergent endpoint/embedding blocks).
- **Three tiers stand** (lite / standard gemma4:12b / pro gemma4:26b) — all three compose files exist in `context-service/docker/`. `standalone.just` is stale (still phi4-mini/deepseek, no pro) — flagged as a context-service follow-up, out of scope here.
- **Cloud endpoint: `https://beta.engrammic.ai/mcp/` stays canonical for now.** Fix the wizard UI string that prints `mcp.engrammic.ai`; doctor/status must compare against the constant, never literals. Swap to `mcp.` at GA.
- **`get.engrammic.ai/skills` is cut.** The main installer handles skills; the separate skills-curl is redundant surface. Join page's "Step 2" copy is updated accordingly.
- **SHA256 publishing is a release-CI deliverable** (Phase 2 dependency, start early): emit `<binary>-<target>.sha256` per asset in `sha256sum` two-column format so install.sh and install.ps1 parse identically. The maintainer controls the get.engrammic.ai pipeline.

## Design principles (the "human" contract)

1. **One obvious path.** Enter-Enter-Enter from `curl | sh` yields a working setup. Every prompt has a sane default. Options exist behind `--help` and submenus, never as required decisions.
2. **Plain language.** Prompts use human phrasing; jargon (MCP, harness, JWT, compose) appears only parenthetically or in `--help`.
3. **Never dead-end.** No prompt failure exits the wizard. Errors state *what happened* then *what to do next*. Bad input → retry loop with Esc-to-skip.
4. **Never lose work.** All questions are asked before any action runs. A failed step reports what succeeded and prints how to retry; it never aborts remaining steps.
5. **Always reversible.** Every mutation is recorded with a backup. Uninstall restores. Data volumes survive uninstall unless `--purge-data`.

## Architecture: interview → plan → execute → manifest

Refactor `installer-cli`'s wizard from a linear `?`/`bail!` chain into:

1. **Interview** — all prompts; zero side effects; every input retryable. Proactive TTY detection before the first prompt: if no `/dev/tty` and not `-y`, print one clear message including the exact re-run command (`curl … | sh -s -- -y`). Flags/`-y` pre-answer interview questions rather than branching into a separate code path — one flow, two input sources (eliminates drift like the skills-defaults inconsistency between `run_full_install` and `engrammic skills`).

   **Selection prompts (editors, skill destinations): nothing is pre-checked.** This is a deliberate behavior change from today's code, which pre-checks detected tools in `select_tools`, `install_skills_step`, and `install_skills_only` — all three must change together. Consequence accepted: the interactive path requires explicit spacebar toggles on these two questions (principle 1's "Enter all the way" applies to every *other* prompt); a zero-selection confirm configures nothing and says so. Detected/already-installed items are *labeled* (e.g. `Cursor  (detected)`, `Claude Code  (already configured)`) but the user makes every selection explicitly. Confirming with zero selections is valid: the wizard says what was skipped and how to do it later (`engrammic install` / `engrammic skills`) instead of treating it as an error. In `-y` mode, detection still drives selection (all detected editors, default skill scope) since there is no one to ask.
2. **Plan summary** — a printed recap of everything about to happen ("Will configure: Cursor, Claude Code · 21 skills (user scope) · Self-hosted Standard tier (~24 GB RAM; detected 32 GB). Proceed?"). `-y` prints it without pausing.
3. **Execute** — steps run sequentially with per-step ✓/✗, skip-and-continue on failure, and a final summary table. Long operations (compose up, model pulls) stream progress or show a spinner — never silence.
4. **Manifest** — `~/.engrammic/state.toml` records every mutation: harness config files edited (+ backup paths), skills installed (+ destination **and install format**), compose stack location/tier, CLI binary path, version. The manifest is the single source of truth for `status`, `remove`, `uninstall`, and upgrades.

### Manifest details (from review)

- **`schema_version` field from day one**, with a migration path — the schema will change.
- **Atomic writes** (temp file + rename). Concurrent invocations: last-writer-wins is acceptable; no lockfile for v1.
- **Migration of existing users:** on first post-upgrade run, synthesize a manifest from the existing `config.toml` (endpoint, license_key, selfhost_dir) plus a legacy scan of harness configs; `config.toml` contents fold into `state.toml` (single file going forward, `config.toml` removed after successful migration).
- **Backup infrastructure must be built — it does not exist today.** `config.rs` `install`/`uninstall` mutate harness configs in place with no backup. New behavior: before the first mutation of any file, write `<path>.engrammic.bak` and record it in the manifest. This is the load-bearing piece of the "always reversible" principle and lands first (Phase 1a).
- **Skills are not uniformly restorable from backup.** Copy-style destinations (plain files, Cursor `.mdc`) are recorded and deleted on removal; merge-style destinations (Gemini shared-file merges) are removed by marker-based content surgery (`remove_gemini_skills` already exists). The manifest's per-destination format field is what lets `remove` dispatch correctly.
- **Windows parity:** manifest and backups live under the same `dirs::home_dir()`-based `.engrammic` dir; binary persistence targets a per-user bin dir with PATH appended via the registry/profile by install.ps1; binary self-deletion on uninstall is not possible while running — uninstall on Windows prints the one remaining manual step (delete the exe) instead of pretending.

## Distribution: one Rust core, thin shims

### install.sh rewrite (rustup-style)

- Detect OS/arch (current logic), download binary **and** SHA256 from the GitHub release, verify before executing.
- Install the binary to `~/.local/bin/engrammic` **in the script** (persistence is no longer an opt-in wizard afterthought; `-y` users get a persistent CLI).
- Offer to append PATH to the detected shell rc; `--no-modify-path` opts out; always print the manual line too. **PATH/persistence is owned by the script alone:** `cli_install.rs::offer_cli_install` is removed — the binary never copies itself or touches PATH again (it would be a self-copy now that the script installs it first).
- Pass `"$@"` through (`curl … | sh -s -- -y --tier lite` works).
- Then launch `engrammic install` from its installed location.
- Friendly diagnostics for: noexec `$TMPDIR`, unsupported arch (with an issues link), missing curl/wget, MSYS/Git-Bash (redirect to the PowerShell command).
- `install.ps1` gets the equivalent treatment.

### npm shim (later phase)

`@engrammic/install`: JS launcher + per-platform binary packages as `optionalDependencies`, reusing release artifacts. No logic in JS beyond binary resolution.

## Self-host flow

Much of this already exists in `selfhost.rs::run_wizard` (tier select with RAM detection, license retry loop, `start_and_wait`, `wait_for_healthy` polling). **This section is consolidation and polish of that flow, not a rebuild** — plus folding it into the interview→plan→execute structure and the manifest.

1. Tier select shows per-tier RAM requirements (Lite 8 GB / Standard 24–32 GB / Pro 48–64 GB) alongside detected system RAM, with the highest safely-fitting tier pre-selected.
2. License entry: visible input, offline Ed25519 validation, **retry loop** surfacing the specific failure (expired / bad signature / wrong prefix) with guidance; Esc skips and marks the step for later (`engrammic license set`).
3. Generate compose + .env, then one question: **"Start it now? (Y/n)"**
   - **Yes (default):** `docker compose up -d` with streamed pull progress, health-endpoint polling, ending on "✓ Engrammic is live at http://localhost:{port}" (default port **8000**, interpolated from the user's choice — never hardcoded).
   - **No:** print the hands-off instructions.
4. **Verification item:** confirm the app container serves `/health` on the user-facing MCP port (the known `/health` route is in `beacon_service`); if it doesn't, the existing poll checks the wrong thing and the service needs a health route before guided bring-up can claim "live".
5. Existing lifecycle commands (`selfhost`, `scale`, `logs`, `doctor`) are aligned to read the manifest rather than rebuilt; gaps filled to cover `up / down / status / upgrade`. `engrammic docker` aliases the selfhost wizard (see Decisions).

## Remove & uninstall

- `engrammic remove [--harness <id>…]` — interactive multi-select when no flag; removes our MCP entries and skills from the chosen harnesses only, restoring from backups where they exist; updates the manifest.
- `engrammic uninstall` — removes all harness entries, skills, config, and the CLI binary. Self-hosted: asks about (or `--purge-data` forces) `docker compose down` + volume deletion — **data kept by default**.
- **Legacy scan:** when no manifest exists, scan known harness config locations for identifiable Engrammic entries (the MCP server key is always `"engrammic"`, which is reliable for file-edit shapes) and confirm each removal individually. **Scope limit:** deep-link harnesses (VS Code, Cursor one-click) and print-instructions/GUI harnesses cannot be read back (`detect_installed_endpoint` returns `None` by design) — the scan covers file-edit shapes only and surfaces the rest as "remove manually" with per-harness instructions, never claiming a clean uninstall it can't verify.
- **Self-hosted teardown targeting:** uninstall reads `install_dir` from the manifest, runs `docker compose -f <install_dir>/docker-compose.yml down` (with `-v` only under `--purge-data`/confirmation), and lists the exact volume names before deletion — never guessing the compose project from the directory basename.

## Join-page alignment (`web/join`)

- Curl stays the hero command; add a small headless/CI toggle showing the `… | sh -s -- -y` variant.
- "Step 2: Install Skills" curl is **removed** (`get.engrammic.ai/skills` cut — see Decisions); the panel instead points at the main installer / `engrammic skills` for users who already have the CLI.
- Per-editor restart hints retained.
- Copy pass for non-technical users across all three paths (one-command / pick-your-editor / self-host).
- JOIN-UX-SPEC.md archived or rewritten to match shipped reality; PLAN-onboarding-improvements.md updated to reference this spec.

## Post-install experience

A fresh install ends with: an automatic lightweight verification (doctor-lite), the restart-editor reminder, a docs link, and one concrete example of using the tools. `engrammic doctor` exit codes distinguish warnings from errors.

Doctor-lite semantics per mode (today's `doctor.rs` is entirely self-host-oriented and needs a cloud branch):
- **Cloud:** "reachable" = a TCP/TLS connect (or HEAD) to the configured endpoint host — the MCP endpoint is JSON-RPC, so a 405/406 response still counts as reachable; do not require 200. Compare against the endpoint *constant*, not literals.
- **Self-hosted:** existing container + `/health` checks, driven by the manifest's install_dir/port.
- **Both:** harness config files named in the manifest parse cleanly.

## Error-handling conventions

- Format: `✗ <what happened>` then `→ <what to do>`, always both.
- No raw anyhow chains reach users; `bail!` may not discard a more specific message already produced (the "Invalid license" bug class).
- Non-TTY, non-`-y` invocations get the proactive one-liner, never a prompt crash.

## Testing

- Shell: bats (or plain sh harness) tests for install.sh — arch detection, checksum failure, arg passthrough, no-curl/no-wget paths.
- Rust: unit tests for interview→plan mapping (flag pre-answers vs interactive parity), manifest round-trip, legacy-scan matching; integration test that a full `-y` install followed by `uninstall` restores all touched files byte-for-byte (temp HOME fixture).
- Self-host bring-up tested against a mock health endpoint; compose interactions behind a trait for testability.

## Phasing (high level — detail in implementation plan)

1. **Foundation** (split per review — 1a is independently shippable and unblocks Phase 3):
   - **1a. Manifest + backup-on-write** in config.rs: schema, atomic writes, `.engrammic.bak` creation, config.toml migration synthesis. Small, isolated, testable.
   - **1b. Interview→plan→execute refactor** of main.rs, folding in the selfhost-consolidation decision (docker→alias) so the manifest never has to represent two flows.
   - **1c. Error conventions** + proactive TTY detection.
   Also started in parallel here: **release-CI SHA256 publishing** (different system, gates Phase 2).
2. **Curl path:** install.sh/install.ps1 rewrite + checksum verification + script-owned persistence/PATH (delete `offer_cli_install`).
3. **Remove/uninstall** (incl. legacy scan with its file-edit-only scope and manual-removal guidance).
4. **Self-host guided bring-up polish** + lifecycle alignment to the manifest + `/health` verification item.
5. **Join page** copy/structure alignment (incl. removing the skills-curl step, adding the `-s -- -y` toggle).
6. **npm shim** + post-install polish.

## Out of scope

- Homebrew/AUR/scoop/winget channels (follow-up after npm shim proves the pattern).
- Any rewrite of installer logic in another language.
- context-service deployment changes (the installer adapts to what the service ships). One flagged follow-up there: `standalone.just` is stale relative to the tiered compose files (no pro tier, old model names) and should be updated in that repo.
- `get.engrammic.ai/skills` endpoint (cut — see Decisions).
