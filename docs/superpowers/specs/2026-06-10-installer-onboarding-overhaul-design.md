# Installer & Onboarding Overhaul — Design

**Date:** 2026-06-10
**Scope:** `mcp-client/installer` (install.sh / install.ps1), `mcp-client/installer-cli` (Rust), `web/join` (onboarding page). Cross-repo: join-page changes land in `../web/join`.
**Status:** Approved pending review

## Problem

The flagship onboarding path (`join.engrammic.ai` → `curl | sh`) breaks in practice:

- `install.sh` `exec`s the binary with no argument passthrough, so `-y` is unreachable from the pipe; missing `/dev/tty` crashes prompts before any guidance appears.
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

## Design principles (the "human" contract)

1. **One obvious path.** Enter-Enter-Enter from `curl | sh` yields a working setup. Every prompt has a sane default. Options exist behind `--help` and submenus, never as required decisions.
2. **Plain language.** Prompts use human phrasing; jargon (MCP, harness, JWT, compose) appears only parenthetically or in `--help`.
3. **Never dead-end.** No prompt failure exits the wizard. Errors state *what happened* then *what to do next*. Bad input → retry loop with Esc-to-skip.
4. **Never lose work.** All questions are asked before any action runs. A failed step reports what succeeded and prints how to retry; it never aborts remaining steps.
5. **Always reversible.** Every mutation is recorded with a backup. Uninstall restores. Data volumes survive uninstall unless `--purge-data`.

## Architecture: interview → plan → execute → manifest

Refactor `installer-cli`'s wizard from a linear `?`/`bail!` chain into:

1. **Interview** — all prompts; zero side effects; every input retryable. Proactive TTY detection before the first prompt: if no `/dev/tty` and not `-y`, print one clear message including the exact re-run command (`curl … | sh -s -- -y`). Flags/`-y` pre-answer interview questions rather than branching into a separate code path — one flow, two input sources (eliminates drift like the skills-defaults inconsistency between `run_full_install` and `engrammic skills`).

   **Selection prompts (editors, skill destinations): nothing is pre-checked.** Detected/already-installed items are *labeled* (e.g. `Cursor  (detected)`, `Claude Code  (already configured)`) but the user makes every selection explicitly. Confirming with zero selections is valid: the wizard says what was skipped and how to do it later (`engrammic install` / `engrammic skills`) instead of treating it as an error. In `-y` mode, detection still drives selection (all detected editors, default skill scope) since there is no one to ask.
2. **Plan summary** — a printed recap of everything about to happen ("Will configure: Cursor, Claude Code · 21 skills (user scope) · Self-hosted Standard tier (~24 GB RAM; detected 32 GB). Proceed?"). `-y` prints it without pausing.
3. **Execute** — steps run sequentially with per-step ✓/✗, skip-and-continue on failure, and a final summary table. Long operations (compose up, model pulls) stream progress or show a spinner — never silence.
4. **Manifest** — `~/.engrammic/state.toml` records every mutation: harness config files edited (+ backup paths), skills installed (+ destinations), compose stack location/tier, CLI binary path, version. The manifest is the single source of truth for `status`, `remove`, `uninstall`, and upgrades.

## Distribution: one Rust core, thin shims

### install.sh rewrite (rustup-style)

- Detect OS/arch (current logic), download binary **and** SHA256 from the GitHub release, verify before executing.
- Install the binary to `~/.local/bin/engrammic` **in the script** (persistence is no longer an opt-in wizard afterthought; `-y` users get a persistent CLI).
- Offer to append PATH to the detected shell rc; `--no-modify-path` opts out; always print the manual line too.
- Pass `"$@"` through (`curl … | sh -s -- -y --tier lite` works).
- Then launch `engrammic install` from its installed location.
- Friendly diagnostics for: noexec `$TMPDIR`, unsupported arch (with an issues link), missing curl/wget, MSYS/Git-Bash (redirect to the PowerShell command).
- `install.ps1` gets the equivalent treatment.

### npm shim (later phase)

`@engrammic/install`: JS launcher + per-platform binary packages as `optionalDependencies`, reusing release artifacts. No logic in JS beyond binary resolution.

## Self-host flow

1. Tier select shows per-tier RAM requirements (Lite 8 GB / Standard 24–32 GB / Pro 48–64 GB) alongside detected system RAM, with the highest safely-fitting tier pre-selected.
2. License entry: visible input, offline Ed25519 validation, **retry loop** surfacing the specific failure (expired / bad signature / wrong prefix) with guidance; Esc skips and marks the step for later (`engrammic license set`).
3. Generate compose + .env (current behavior), then one question: **"Start it now? (Y/n)"**
   - **Yes (default):** `docker compose up -d` with streamed pull progress, health-endpoint polling, ending on "✓ Engrammic is live at http://localhost:8080".
   - **No:** print the current hands-off instructions.
4. Existing lifecycle commands (`selfhost`, `scale`, `logs`, `doctor`) are aligned to read the manifest rather than rebuilt; gaps filled to cover `up / down / status / upgrade`.

## Remove & uninstall

- `engrammic remove [--harness <id>…]` — interactive multi-select when no flag; removes our MCP entries and skills from the chosen harnesses only, restoring from backups where they exist; updates the manifest.
- `engrammic uninstall` — removes all harness entries, skills, config, and the CLI binary. Self-hosted: asks about (or `--purge-data` forces) `docker compose down` + volume deletion — **data kept by default**.
- **Legacy scan:** when no manifest exists, scan known harness config locations for identifiable Engrammic entries (server name / endpoint URL match) and confirm each removal individually.

## Join-page alignment (`web/join`)

- Curl stays the hero command; add a small headless/CI toggle showing the `… | sh -s -- -y` variant.
- "Step 2: Install Skills" curl (`get.engrammic.ai/skills`) wired to the `engrammic skills` subcommand.
- Per-editor restart hints retained.
- Copy pass for non-technical users across all three paths (one-command / pick-your-editor / self-host).
- JOIN-UX-SPEC.md archived or rewritten to match shipped reality; PLAN-onboarding-improvements.md updated to reference this spec.

## Post-install experience

A fresh install ends with: an automatic lightweight verification (doctor-lite: endpoint reachable, harness configs parse), the restart-editor reminder, a docs link, and one concrete example of using the tools. `engrammic doctor` exit codes distinguish warnings from errors.

## Error-handling conventions

- Format: `✗ <what happened>` then `→ <what to do>`, always both.
- No raw anyhow chains reach users; `bail!` may not discard a more specific message already produced (the "Invalid license" bug class).
- Non-TTY, non-`-y` invocations get the proactive one-liner, never a prompt crash.

## Testing

- Shell: bats (or plain sh harness) tests for install.sh — arch detection, checksum failure, arg passthrough, no-curl/no-wget paths.
- Rust: unit tests for interview→plan mapping (flag pre-answers vs interactive parity), manifest round-trip, legacy-scan matching; integration test that a full `-y` install followed by `uninstall` restores all touched files byte-for-byte (temp HOME fixture).
- Self-host bring-up tested against a mock health endpoint; compose interactions behind a trait for testability.

## Phasing (high level — detail in implementation plan)

1. **Foundation:** manifest + interview/plan/execute refactor + error conventions (everything else depends on it).
2. **Curl path:** install.sh/install.ps1 rewrite + checksums + TTY proactivity.
3. **Remove/uninstall** (incl. legacy scan).
4. **Self-host guided bring-up** + lifecycle alignment.
5. **Join page** copy/structure alignment.
6. **npm shim** + post-install polish.

## Out of scope

- Homebrew/AUR/scoop/winget channels (follow-up after npm shim proves the pattern).
- Any rewrite of installer logic in another language.
- context-service deployment changes (the installer adapts to what the service ships).
