# Skill distribution + installer DX redesign

Date: 2026-05-22
Status: Design approved, ready for implementation planning
Repos affected: `mcp-client` (installer), `skills` (one-time rename)

## Problem

The `get.engrammic.ai` installer does two things today and neither covers skills:

1. `install.sh` / `install.ps1` are thin bootstrap scripts that download the
   `engrammic-install` Rust binary from GitHub releases and exec it.
2. `engrammic-install` writes the `engrammic` MCP server entry into a single
   selected harness's config file. That is all it does.

Two gaps:

- The open-source `engrammic-ai/skills` repo (21 skills) is not distributed by
  the installer at all. Users are told to `cp -r` skills by hand.
- The installer experience is bare. No banner, single-harness only, minimal
  output. It does not reflect the product.

## Goals

- The installer also delivers skills, as an opt-out step in the main flow.
- One install run can wire up multiple harnesses, not just one.
- Skills are not Claude Code specific. Codex, Gemini CLI, Cursor, and Pi Agents
  users are first-class.
- The installer looks polished: a banner, clear sections, a tidy summary.
- Cross-platform parity. macOS, Linux, and Windows behave identically.

## Non-goals

- No landing page (`index.html`) redesign. Out of scope for this spec.
- No change to the MCP endpoint or the MCP server itself.
- No new `skills` subcommand. Skills live inside the main install flow.
- No per-skill selection. Skills install as a set of 21.

## Design

### 1. Banner

A styled-text banner (no ASCII art). Weight and color do the work.

```
  ╭─────────────────────────────────────────────╮
  │                                             │
  │   engrammic   MCP Installer                 │
  │   epistemic memory for AI agents            │
  │                                             │
  │   → engrammic.ai                            │
  │                                             │
  ╰─────────────────────────────────────────────╯
```

- Box border: oxide red. Text: bone white. `engrammic` is bold.
- Includes the `engrammic.ai` link.
- Exact hex values are implementation-tunable. Starting points: oxide red
  `#A33B2A`, bone white `#E9E2D2`. Note: this palette diverges from the current
  landing page accent (`#3b82f6`); that is a deliberate choice to revisit if
  brand consistency is wanted later.
- Rendered once at the top of `install`, `update`, `uninstall`, and `status`.

### 2. Install flow

`engrammic-install` (default `install` subcommand):

1. Print banner.
2. Detect harnesses (config-dir parent exists). Show a detected/not-detected
   list.
3. Multi-harness MCP install. Replace the current single `Select` with a
   `MultiSelect` over detected harnesses, pre-checked. Write the `engrammic`
   MCP server entry into each selected harness's config file. This reuses the
   existing `config::install` logic, looped over the selection.
4. Skills step, opt-out: prompt `Install 21 Engrammic skills? [Y/n]`, default
   yes.
5. If yes, `MultiSelect` over skill destinations (see section 4), then fetch and
   install (see section 3).
6. Print a summary: harnesses configured, skill count, destinations.

The `-y` / `--yes` flag accepts all defaults: all detected harnesses, skills
yes, default skill destination (Claude Code if present, else global
`~/.agents/skills/`).

### 3. Skill fetch mechanism

The `engrammic-ai/skills` repo is public. The installer downloads the
GitHub-served tarball of the default branch:

```
https://github.com/engrammic-ai/skills/archive/refs/heads/main.tar.gz
```

No release-publishing step is needed in the skills repo; GitHub serves this URL
for any public repo. The tarball extracts to a top-level `skills-main/`
directory containing the per-skill folders.

Steps: download to a temp file, unpack to a temp dir, copy each
`engrammic-*` skill folder into every chosen destination, clean up temp.

New Rust dependencies for `engrammic-install`:

- `ureq` — lightweight blocking HTTP client (with TLS).
- `flate2` — gzip decompression.
- `tar` — tarball extraction.
- `indicatif` (optional, recommended) — one spinner during the download.

The binary today has no network dependency. These additions are accepted. The
release profile stays `opt-level = "z"` + `strip = true`.

A single spinner covers the download. Copy steps print static checkmark lines.
This matches the agreed "polished but calm" bar: animation only for the one
genuinely slow step.

### 4. Skill destinations

`SKILL.md` is a Claude Code convention. Most harnesses have no native skills
directory, so the installer does not invent per-harness paths. It offers three
destinations as a `MultiSelect`:

| Destination          | Path                  | Serves                              |
|----------------------|-----------------------|-------------------------------------|
| Claude Code (native) | `~/.claude/skills/`   | Claude Code                         |
| Cross-harness global | `~/.agents/skills/`   | Codex, Gemini CLI, Cursor, Pi Agents|
| Project-local        | `./.agents/skills/`   | Current working directory           |

The `~/.agents/skills/` convention is the one the skills repo README already
documents as cross-harness. It is the path that makes non-Claude harnesses
first-class without reverse-engineering each one.

Default selection: Claude Code if its config dir exists, otherwise the global
cross-harness path. Project-local is always offered, never pre-checked.

Config-file paths (for the MCP step) remain home-relative via the `dirs` crate;
Rust normalizes the separator on Windows. If research during planning finds a
harness that stores config under `%APPDATA%`, that becomes a single per-tool
override in the harness table, not a structural change.

### 5. Skill naming change (skills repo)

This is a prerequisite change in the `engrammic-ai/skills` repo, not in the
installer.

Claude Code identifies a skill by its directory name; the directory name
becomes the invocation command. The `name:` frontmatter field is optional,
display-only, and constrained to lowercase letters, numbers, and hyphens.
Colons are not valid.

The skills repo currently names directories `engrammic:remember` etc., with
matching `name: engrammic:remember` frontmatter. The colon is illegal in
Windows filenames, so the tarball cannot extract on Windows, and the frontmatter
is already off-spec.

Fix: rename all 21 skill directories and their `name:` frontmatter to colonless
`engrammic-<name>` (for example `engrammic-remember`). Apply this on all
platforms, not conditionally on Windows. A Windows-only remap would make the
same skill invoke as `/engrammic-remember` on Windows and `/engrammic:remember`
elsewhere, which forks the docs and the user's muscle memory.

Consequences:

- The installer needs zero path remapping and zero Windows special-casing for
  skills. It extracts and copies, unchanged, on every platform.
- Skill invocation is identical on every OS.
- The skills repo README `cp -r engrammic:*` examples must update to
  `engrammic-*`.
- This rename must land before or together with the installer release that
  depends on it.
- Users who already installed skills the old way keep their colon directories
  until they reinstall. Acceptable during closed beta.

### 6. update / uninstall / status

All three keep working for MCP config and additionally handle skills.

- `update`: re-download the skills tarball and overwrite skill folders in
  whichever of the three destinations currently contain `engrammic-*` skills.
  Also re-write MCP config as today.
- `uninstall`: remove `engrammic-*` skill directories from the three known
  destinations, in addition to removing the MCP server entry.
- `status`: for each destination, count `engrammic-*` skill directories present
  and show it alongside the existing per-harness MCP status.

No manifest file is needed. Because every skill directory carries the
`engrammic-` prefix, the installer can manage the set by prefix.

### 7. Bootstrap scripts

`install.sh` and `install.ps1` stay thin downloaders. They continue to fetch and
exec the `engrammic-install` binary. All banner and polish work lives in the
binary so it is written once and is consistent across platforms. The first
visible banner appears immediately after the binary starts, which is acceptable;
the bootstrap scripts themselves stay quiet.

## Affected files

`mcp-client`:

- `installer-cli/Cargo.toml` — add `ureq`, `flate2`, `tar`, `indicatif`.
- `installer-cli/src/main.rs` — banner, multi-harness loop, skills step,
  summary.
- `installer-cli/src/cli.rs` — no new subcommands; behavior changes only.
- `installer-cli/src/tools.rs` — harness table unchanged for MCP; add skill
  destination definitions.
- `installer-cli/src/config.rs` — unchanged (reused per selected harness).
- New module, for example `installer-cli/src/skills.rs` — download, unpack,
  copy, remove, count.
- `installer/README.md` — document the skills behavior.

`skills`:

- Rename 21 `engrammic:<name>/` directories to `engrammic-<name>/`.
- Update `name:` frontmatter in each `SKILL.md`.
- Update `README.md` install examples.

## Deferred to planning

- Confirm the skills repo default branch tarball path and the extracted
  top-level directory name.
- Research whether any supported harness stores MCP config under `%APPDATA%`
  rather than a home-relative dir.
- Confirm `ureq` TLS feature selection and resulting binary size delta.
- Decide whether `indicatif` is included or the download uses a static line.
- Minor: `installer-cli/Cargo.toml` `repository` field reads
  `engrammic-ai/mcp-client`; the actual remote is `engrammic-ai/mcp`. Fix
  opportunistically.

## Out of scope

- Landing page redesign.
- Per-skill selection or skill profiles in the installer.
- Any change to the MCP server or its endpoint.
