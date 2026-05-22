# Skill Distribution + Installer DX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `get.engrammic.ai` installer deliver the 21 open-source Engrammic skills, configure multiple agent harnesses in one run, and present a polished banner.

**Architecture:** Phase 1 renames skill directories in the `engrammic-ai/skills` repo to remove colons (a hard prerequisite for Windows extraction). Phase 2 extends the `engrammic-install` Rust CLI: it downloads the public skills tarball, unpacks it, copies skills into chosen destinations, configures multiple harnesses via a multi-select, and prints a styled banner. The bootstrap shell scripts stay unchanged.

**Tech Stack:** Rust 2021, `clap`, `dialoguer`, `colored`, `dirs`, `anyhow`, `serde_json`. New: `ureq` (HTTP), `flate2` (gzip), `tar` (extraction), `indicatif` (spinner), `tempfile` (dev).

**Spec:** `docs/superpowers/specs/2026-05-22-skill-distribution-installer-dx-design.md`

---

## File Structure

`skills` repo (Phase 1):
- 21 `engrammic:<name>/` directories renamed to `engrammic-<name>/`, each with updated `SKILL.md` frontmatter.
- `README.md` install examples updated.

`mcp-client/installer-cli/` (Phase 2):
- `Cargo.toml` — add runtime + dev dependencies.
- `src/main.rs` — banner call, multi-harness loop, skills step, summary. Orchestration only.
- `src/cli.rs` — unchanged (no new subcommands).
- `src/config.rs` — unchanged (reused per harness).
- `src/tools.rs` — add `SkillDest` (skill destination definitions).
- `src/banner.rs` — NEW. Banner rendering.
- `src/skills.rs` — NEW. Download, unpack, copy, remove, count skills.

`mcp-client/installer/README.md` — document skills behavior.

---

## Phase 1: Skills repo rename

### Task 1: Rename skill directories and frontmatter to colonless

This task runs in the **`engrammic-ai/skills` repo** (`../skills` relative to `mcp-client`), not in `mcp-client`.

**Files:**
- Rename: all 21 `engrammic:<name>/` directories to `engrammic-<name>/`
- Modify: `engrammic-<name>/SKILL.md` frontmatter `name:` field in each
- Modify: `README.md`

- [ ] **Step 1: Create a branch in the skills repo**

```bash
cd ../skills
git checkout -b chore/colonless-skill-dirs
```

- [ ] **Step 2: Rename directories and update frontmatter**

Run this script from the skills repo root. It renames each directory and rewrites the `name:` line inside its `SKILL.md`.

```bash
for dir in engrammic:*; do
  new="${dir/engrammic:/engrammic-}"
  git mv "$dir" "$new"
  # Rewrite the frontmatter name: field to match the new directory name
  python3 - "$new/SKILL.md" "$new" <<'PY'
import sys
path, newname = sys.argv[1], sys.argv[2]
lines = open(path).read().splitlines(keepends=True)
out = []
for line in lines:
    if line.startswith("name:"):
        out.append(f"name: {newname}\n")
    else:
        out.append(line)
open(path, "w").writelines(out)
PY
done
```

- [ ] **Step 3: Verify no colons remain**

Run:
```bash
ls -d engrammic-* | wc -l
ls -d engrammic:* 2>/dev/null | wc -l
grep -rl 'name: engrammic:' . --include=SKILL.md
```
Expected: first command prints `21`, second prints `0`, third prints nothing.

- [ ] **Step 4: Update README install examples**

In `README.md`, change both `cp -r engrammic:*` commands to `cp -r engrammic-*`.

- [ ] **Step 5: Verify a skill still loads**

Copy one renamed skill into the local Claude Code skills dir and confirm it is recognized:
```bash
cp -r engrammic-recall ~/.claude/skills/
```
Expected: no error; the skill is invocable as `/engrammic-recall` in a new Claude Code session. (Manual check. If it fails, stop and report — the rename assumption is wrong.)

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: rename skill directories to colonless engrammic-<name>"
```

---

## Phase 2: Installer

All Phase 2 tasks run in `mcp-client`. Work on a branch:

```bash
cd ../mcp-client
git checkout feat/skill-distribution-installer-dx   # branch already exists with the spec
```

Run all `cargo` commands from `mcp-client/installer-cli/`.

### Task 2: Add dependencies

**Files:**
- Modify: `installer-cli/Cargo.toml`

- [ ] **Step 1: Add runtime and dev dependencies**

Replace the `[dependencies]` block and add a `[dev-dependencies]` block:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dialoguer = "0.11"
colored = "2"
dirs = "5"
anyhow = "1"
ureq = "2"
flate2 = "1"
tar = "0.4"
indicatif = "0.17"

[dev-dependencies]
tempfile = "3"
```

Also fix the stale `repository` field at the top of the file: change `https://github.com/engrammic-ai/mcp-client` to `https://github.com/engrammic-ai/mcp`.

- [ ] **Step 2: Verify the build resolves**

Run: `cargo build`
Expected: compiles successfully, new crates downloaded.

- [ ] **Step 3: Commit**

```bash
git add installer-cli/Cargo.toml installer-cli/Cargo.lock
git commit -m "build: add skill-fetch and DX dependencies to installer"
```

### Task 3: Skill destinations in tools.rs

**Files:**
- Modify: `installer-cli/src/tools.rs`
- Test: inline `#[cfg(test)]` module in `installer-cli/src/tools.rs`

- [ ] **Step 1: Write the failing test**

Append to `installer-cli/src/tools.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_dest_all_returns_three() {
        let dests = SkillDest::all();
        assert_eq!(dests.len(), 3);
        assert!(dests[0].path.ends_with(".claude/skills"));
        assert!(dests[1].path.ends_with(".agents/skills"));
        assert_eq!(dests[2].path, PathBuf::from(".agents/skills"));
    }

    #[test]
    fn project_dest_is_never_default() {
        let dests = SkillDest::all();
        assert!(!dests[2].default);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin engrammic-install skill_dest`
Expected: FAIL with "cannot find type `SkillDest`".

- [ ] **Step 3: Add the SkillDest type**

Append to `installer-cli/src/tools.rs`, before the `#[cfg(test)]` module:

```rust
#[derive(Clone)]
pub struct SkillDest {
    pub name: &'static str,
    pub path: PathBuf,
    pub default: bool,
}

impl SkillDest {
    pub fn all() -> Vec<SkillDest> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let claude = home.join(".claude/skills");
        let claude_present = claude
            .parent()
            .map(|p| p.exists())
            .unwrap_or(false);
        vec![
            SkillDest {
                name: "Claude Code        ~/.claude/skills/",
                path: claude,
                default: claude_present,
            },
            SkillDest {
                name: "Cross-harness      ~/.agents/skills/",
                path: home.join(".agents/skills"),
                default: !claude_present,
            },
            SkillDest {
                name: "Project-local      ./.agents/skills/",
                path: PathBuf::from(".agents/skills"),
                default: false,
            },
        ]
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --bin engrammic-install skill_dest`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/tools.rs
git commit -m "feat: define skill install destinations"
```

### Task 4: Skill copy / remove / count logic

**Files:**
- Create: `installer-cli/src/skills.rs`
- Modify: `installer-cli/src/main.rs` (register the module)
- Test: inline `#[cfg(test)]` module in `installer-cli/src/skills.rs`

- [ ] **Step 1: Register the module**

Add to the top of `installer-cli/src/main.rs`, after the existing `mod` lines:

```rust
mod skills;
```

- [ ] **Step 2: Write the failing test**

Create `installer-cli/src/skills.rs` with only the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_skill(root: &std::path::Path, name: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
    }

    #[test]
    fn copy_skills_copies_only_prefixed_dirs() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_skill(src.path(), "engrammic-recall");
        make_skill(src.path(), "engrammic-learn");
        make_skill(src.path(), "unrelated-thing");
        fs::write(src.path().join("README.md"), "x").unwrap();

        let count = copy_skills(src.path(), dest.path()).unwrap();
        assert_eq!(count, 2);
        assert!(dest.path().join("engrammic-recall/SKILL.md").exists());
        assert!(!dest.path().join("unrelated-thing").exists());
        assert!(!dest.path().join("README.md").exists());
    }

    #[test]
    fn count_skills_counts_prefixed_dirs() {
        let dir = tempdir().unwrap();
        make_skill(dir.path(), "engrammic-recall");
        make_skill(dir.path(), "other");
        assert_eq!(count_skills(dir.path()), 1);
    }

    #[test]
    fn count_skills_on_missing_dir_is_zero() {
        assert_eq!(count_skills(std::path::Path::new("/no/such/dir")), 0);
    }

    #[test]
    fn remove_skills_removes_only_prefixed_dirs() {
        let dir = tempdir().unwrap();
        make_skill(dir.path(), "engrammic-recall");
        make_skill(dir.path(), "keep-me");
        let removed = remove_skills(dir.path()).unwrap();
        assert_eq!(removed, 1);
        assert!(!dir.path().join("engrammic-recall").exists());
        assert!(dir.path().join("keep-me").exists());
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --bin engrammic-install skills::`
Expected: FAIL with "cannot find function `copy_skills`".

- [ ] **Step 4: Write the implementation**

Prepend to `installer-cli/src/skills.rs`, above the test module:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const SKILL_PREFIX: &str = "engrammic-";

pub fn count_skills(dest: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dest) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            e.file_name().to_string_lossy().starts_with(SKILL_PREFIX)
                && e.path().is_dir()
        })
        .count()
}

pub fn copy_skills(src: &Path, dest: &Path) -> Result<usize> {
    fs::create_dir_all(dest)
        .with_context(|| format!("failed to create {}", dest.display()))?;
    let mut count = 0;
    for entry in fs::read_dir(src)
        .with_context(|| format!("failed to read {}", src.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with(SKILL_PREFIX)
            || !entry.path().is_dir()
        {
            continue;
        }
        let target = dest.join(&name);
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        copy_dir_recursive(&entry.path(), &target)?;
        count += 1;
    }
    Ok(count)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

pub fn remove_skills(dest: &Path) -> Result<usize> {
    let mut count = 0;
    let Ok(entries) = fs::read_dir(dest) else {
        return Ok(0);
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(SKILL_PREFIX)
            && entry.path().is_dir()
        {
            fs::remove_dir_all(entry.path())?;
            count += 1;
        }
    }
    Ok(count)
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --bin engrammic-install skills::`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add installer-cli/src/skills.rs installer-cli/src/main.rs
git commit -m "feat: add skill copy, remove, and count logic"
```

### Task 5: Tarball unpack

**Files:**
- Modify: `installer-cli/src/skills.rs`
- Test: `#[cfg(test)]` module in `installer-cli/src/skills.rs`

- [ ] **Step 1: Write the failing test**

Add inside the existing `#[cfg(test)] mod tests` block in `installer-cli/src/skills.rs`:

```rust
    #[test]
    fn unpack_tarball_extracts_top_level_dir() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Build an in-memory tar.gz with one top-level dir and a file.
        let mut tar_buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_buf);
            let content = b"hello";
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(
                    &mut header,
                    "skills-main/engrammic-recall/SKILL.md",
                    &content[..],
                )
                .unwrap();
            builder.finish().unwrap();
        }
        let mut gz = Vec::new();
        {
            let mut encoder = GzEncoder::new(&mut gz, Compression::default());
            encoder.write_all(&tar_buf).unwrap();
            encoder.finish().unwrap();
        }

        let dest = tempfile::tempdir().unwrap();
        let top = unpack_tarball(&gz, dest.path()).unwrap();
        assert!(top.ends_with("skills-main"));
        assert!(top.join("engrammic-recall/SKILL.md").exists());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin engrammic-install unpack_tarball`
Expected: FAIL with "cannot find function `unpack_tarball`".

- [ ] **Step 3: Write the implementation**

Add to the implementation section of `installer-cli/src/skills.rs` (above the test module), and add the imports at the top:

```rust
use flate2::read::GzDecoder;
use tar::Archive;
```

```rust
pub fn unpack_tarball(gz_bytes: &[u8], dest: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dest)?;
    let decoder = GzDecoder::new(gz_bytes);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(dest)
        .context("failed to unpack skills tarball")?;
    // GitHub tarballs contain exactly one top-level directory.
    fs::read_dir(dest)?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .context("skills tarball had no top-level directory")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --bin engrammic-install unpack_tarball`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/skills.rs
git commit -m "feat: add skills tarball unpack"
```

### Task 6: Download and install orchestration

These functions require network access and cannot be unit tested deterministically. They get an `#[ignore]`-marked integration test plus a manual verification step.

**Files:**
- Modify: `installer-cli/src/skills.rs`

- [ ] **Step 1: Write the network-gated test**

Add inside the `#[cfg(test)] mod tests` block in `installer-cli/src/skills.rs`:

```rust
    #[test]
    #[ignore = "hits the network; run with --ignored"]
    fn download_skills_tarball_returns_gzip() {
        let bytes = download_skills_tarball().unwrap();
        // gzip magic number
        assert_eq!(&bytes[0..2], &[0x1f, 0x8b]);
        assert!(bytes.len() > 1000);
    }
```

- [ ] **Step 2: Write the download and install functions**

Add to the implementation section of `installer-cli/src/skills.rs`, with imports at the top:

```rust
use std::io::Read;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
```

```rust
const SKILLS_TARBALL_URL: &str =
    "https://github.com/engrammic-ai/skills/archive/refs/heads/main.tar.gz";

pub fn download_skills_tarball() -> Result<Vec<u8>> {
    let resp = ureq::get(SKILLS_TARBALL_URL)
        .call()
        .context("failed to download skills tarball")?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .context("failed to read skills tarball body")?;
    Ok(bytes)
}

/// Downloads, unpacks, and copies skills into each destination.
/// Returns one (destination, skill count) pair per destination.
pub fn install_skills(dests: &[PathBuf]) -> Result<Vec<(PathBuf, usize)>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("  {spinner} {msg}")
            .expect("valid spinner template"),
    );
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message("Downloading skills...");

    let bytes = download_skills_tarball()?;
    spinner.finish_and_clear();

    let tmp = std::env::temp_dir().join("engrammic-skills-unpack");
    if tmp.exists() {
        fs::remove_dir_all(&tmp).ok();
    }
    let src = unpack_tarball(&bytes, &tmp)?;

    let mut results = Vec::new();
    for dest in dests {
        let count = copy_skills(&src, dest)?;
        results.push((dest.clone(), count));
    }

    fs::remove_dir_all(&tmp).ok();
    Ok(results)
}
```

- [ ] **Step 3: Verify it compiles and the unit tests still pass**

Run: `cargo test --bin engrammic-install skills::`
Expected: PASS (the 5 non-ignored tests; the download test shows as `ignored`).

- [ ] **Step 4: Manually verify the network path**

Run: `cargo test --bin engrammic-install download_skills_tarball -- --ignored`
Expected: PASS. If it fails, confirm `engrammic-ai/skills` is public and the default branch is `main`.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/skills.rs
git commit -m "feat: add skills download and install orchestration"
```

### Task 7: Banner module

**Files:**
- Create: `installer-cli/src/banner.rs`
- Modify: `installer-cli/src/main.rs` (register module)
- Test: `#[cfg(test)]` module in `installer-cli/src/banner.rs`

- [ ] **Step 1: Register the module**

Add to the top of `installer-cli/src/main.rs`, after the other `mod` lines:

```rust
mod banner;
```

- [ ] **Step 2: Write the failing test**

Create `installer-cli/src/banner.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_lines_include_name_and_link() {
        let lines = banner_lines();
        assert!(lines.iter().any(|l| l.contains("engrammic")));
        assert!(lines.iter().any(|l| l.contains("engrammic.ai")));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --bin engrammic-install banner`
Expected: FAIL with "cannot find function `banner_lines`".

- [ ] **Step 4: Write the implementation**

Prepend to `installer-cli/src/banner.rs`, above the test module:

```rust
use colored::Colorize;

// Oxide red border, bone white text. Hex values are tunable.
const OXIDE: (u8, u8, u8) = (0xA3, 0x3B, 0x2A);
const BONE: (u8, u8, u8) = (0xE9, 0xE2, 0xD2);
const INNER_WIDTH: usize = 45;

/// The text content of the banner, one entry per content line.
pub fn banner_lines() -> Vec<String> {
    vec![
        "engrammic   MCP Installer".to_string(),
        "epistemic memory for AI agents".to_string(),
        "engrammic.ai".to_string(),
    ]
}

pub fn print_banner() {
    let (o0, o1, o2) = OXIDE;
    let (b0, b1, b2) = BONE;
    let edge = |s: &str| s.truecolor(o0, o1, o2);

    let border = "─".repeat(INNER_WIDTH);
    let blank = " ".repeat(INNER_WIDTH);

    println!();
    println!("  {}", edge(&format!("╭{}╮", border)));
    println!("  {}{}{}", edge("│"), blank, edge("│"));

    for (i, line) in banner_lines().iter().enumerate() {
        let padded = format!("   {:<width$}", line, width = INNER_WIDTH - 3);
        let colored = if i == 0 {
            // Bold the product name at the start of the first line.
            let rest = padded.replacen("engrammic", "", 1);
            format!(
                "   {}{}",
                "engrammic".truecolor(b0, b1, b2).bold(),
                format!("{:<width$}", rest.trim_start(), width = INNER_WIDTH - 12)
                    .truecolor(b0, b1, b2)
            )
        } else if i == 2 {
            format!(
                "   {}{}",
                "→ ".truecolor(o0, o1, o2),
                format!("{:<width$}", line, width = INNER_WIDTH - 5)
                    .truecolor(b0, b1, b2)
            )
        } else {
            padded.truecolor(b0, b1, b2).to_string()
        };
        println!("  {}{}{}", edge("│"), colored, edge("│"));
    }

    println!("  {}{}{}", edge("│"), blank, edge("│"));
    println!("  {}", edge(&format!("╰{}╯", border)));
    println!();
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --bin engrammic-install banner`
Expected: PASS.

- [ ] **Step 6: Manually verify the banner renders**

Temporarily add `banner::print_banner();` as the first line of `main()`, run `cargo run`, and confirm the box renders with an oxide-red border and bone-white text and no misaligned edges. Then remove that temporary line (Task 8 wires it in properly).

- [ ] **Step 7: Commit**

```bash
git add installer-cli/src/banner.rs installer-cli/src/main.rs
git commit -m "feat: add styled installer banner"
```

### Task 8: Multi-harness MCP install

Replaces the single-harness `Select` with a `MultiSelect`, and wires the banner into every subcommand.

**Files:**
- Modify: `installer-cli/src/main.rs`

- [ ] **Step 1: Update imports**

Change the `dialoguer` import line in `installer-cli/src/main.rs` to:

```rust
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect, Select};
```

- [ ] **Step 2: Replace `select_tool` with `select_tools`**

Delete the existing `select_tool` function and replace it with:

```rust
fn select_tools(yes: bool, tool_id: Option<&str>) -> Result<Vec<Tool>> {
    // Explicit --tool flag wins.
    if let Some(id) = tool_id {
        let tool = Tool::from_id(id).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool: {}. Use: claude, cursor, windsurf, antigravity, gemini, pi",
                id
            )
        })?;
        return Ok(vec![tool]);
    }

    let detected = Tool::detect_installed();

    // -y with detected harnesses: take all detected, no prompt.
    if yes && !detected.is_empty() {
        for tool in &detected {
            println!("Auto-selected: {}", tool.name.cyan());
        }
        return Ok(detected);
    }

    let all_tools = Tool::all();
    let items: Vec<&str> = all_tools.iter().map(|t| t.name).collect();
    let detected_ids: Vec<&str> = detected.iter().map(|t| t.id).collect();
    let defaults: Vec<bool> = all_tools
        .iter()
        .map(|t| detected_ids.contains(&t.id))
        .collect();

    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select harnesses to configure (space toggles, enter confirms)")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    Ok(selection.into_iter().map(|i| all_tools[i].clone()).collect())
}
```

NOTE: the `Select` import is retained only if still used elsewhere. After this task, if `cargo build` warns that `Select` is unused, remove it from the import line.

- [ ] **Step 3: Update the `install` function to loop over harnesses**

Replace the existing `install` function body with:

```rust
fn install(yes: bool, tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(yes, tool_id)?;
    if tools.is_empty() {
        println!("{} No harness selected.", "!".yellow());
        return Ok(());
    }

    println!("{}", "Writing MCP config".bold());
    for tool in &tools {
        config::install(&tool.config_path, ENDPOINT)?;
        println!(
            "  {} {}  {}",
            "✓".green(),
            tool.name,
            tool.config_path.display().to_string().dimmed()
        );
    }
    println!();

    install_skills_step(yes)?;

    println!();
    println!(
        "Done. Tools available: {}",
        "remember, recall, learn, believe, trace, link".dimmed()
    );
    Ok(())
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build`
Expected: compiles. It will fail only if Task 9 is not yet done, because `install_skills_step` is undefined. That is expected; proceed to Task 9 before testing.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat: configure multiple harnesses in one install run"
```

### Task 9: Skills step in the install flow

**Files:**
- Modify: `installer-cli/src/main.rs`

- [ ] **Step 1: Add the `install_skills_step` function**

Add to `installer-cli/src/main.rs`:

```rust
fn install_skills_step(yes: bool) -> Result<()> {
    let proceed = if yes {
        true
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Also install 21 Engrammic skills?")
            .default(true)
            .interact()?
    };

    if !proceed {
        println!("  {} Skipped skills.", "-".dimmed());
        return Ok(());
    }

    let all_dests = SkillDest::all();
    let chosen: Vec<&SkillDest> = if yes {
        all_dests.iter().filter(|d| d.default).collect()
    } else {
        let items: Vec<&str> = all_dests.iter().map(|d| d.name).collect();
        let defaults: Vec<bool> = all_dests.iter().map(|d| d.default).collect();
        let picked = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Install skills to (space toggles, enter confirms)")
            .items(&items)
            .defaults(&defaults)
            .interact()?;
        picked.into_iter().map(|i| &all_dests[i]).collect()
    };

    if chosen.is_empty() {
        println!("  {} No skill destination selected.", "-".dimmed());
        return Ok(());
    }

    let paths: Vec<std::path::PathBuf> =
        chosen.iter().map(|d| d.path.clone()).collect();
    let results = skills::install_skills(&paths)?;

    println!("{}", "Installing skills".bold());
    for (path, count) in results {
        println!(
            "  {} {} skills  {}",
            "✓".green(),
            count,
            path.display().to_string().dimmed()
        );
    }
    Ok(())
}
```

- [ ] **Step 2: Confirm `SkillDest` is imported**

At the top of `installer-cli/src/main.rs`, the `use tools::...` line must include `SkillDest`. Update it to:

```rust
use tools::{SkillDest, Tool, ENDPOINT};
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles successfully.

- [ ] **Step 4: Manually verify the full install flow**

Run: `cargo run -- install`
Expected: banner renders, harness multi-select appears, MCP config lines print, the skills confirm prompt appears, the destination multi-select appears, the download spinner shows, and skill counts print. Pick a throwaway destination or inspect `~/.agents/skills/` afterward. Confirm 21 skills landed.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat: add opt-out skills step to install flow"
```

### Task 10: Skills in update / uninstall / status

**Files:**
- Modify: `installer-cli/src/main.rs`

- [ ] **Step 1: Update the `update` function**

Replace the existing `update` function body with:

```rust
fn update(tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(false, tool_id)?;
    for tool in &tools {
        if config::is_installed(&tool.config_path, ENDPOINT) {
            config::install(&tool.config_path, ENDPOINT)?;
            println!("{} Updated engrammic in {}", "✓".green(), tool.name);
        } else {
            println!("{} Not installed for {}", "!".yellow(), tool.name);
        }
    }

    // Refresh skills in any destination that already has them.
    let dests_with_skills: Vec<std::path::PathBuf> = SkillDest::all()
        .into_iter()
        .filter(|d| skills::count_skills(&d.path) > 0)
        .map(|d| d.path)
        .collect();

    if !dests_with_skills.is_empty() {
        let results = skills::install_skills(&dests_with_skills)?;
        for (path, count) in results {
            println!(
                "{} Refreshed {} skills in {}",
                "✓".green(),
                count,
                path.display()
            );
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Update the `uninstall` function**

Replace the existing `uninstall` function body with:

```rust
fn uninstall(tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(false, tool_id)?;
    for tool in &tools {
        config::uninstall(&tool.config_path)?;
        println!("{} Removed engrammic from {}", "✓".green(), tool.name);
    }

    for dest in SkillDest::all() {
        let removed = skills::remove_skills(&dest.path)?;
        if removed > 0 {
            println!(
                "{} Removed {} skills from {}",
                "✓".green(),
                removed,
                dest.path.display()
            );
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Update the `status` function**

Replace the existing `status` function body with:

```rust
fn status() -> Result<()> {
    banner::print_banner();

    println!("{}", "Harnesses".bold());
    let mut any_installed = false;
    for tool in Tool::all() {
        let installed = config::is_installed(&tool.config_path, ENDPOINT);
        let label = if installed {
            any_installed = true;
            "✓ installed".green()
        } else if tool.config_path.parent().map(|p| p.exists()).unwrap_or(false) {
            "- not configured".dimmed()
        } else {
            "- not detected".dimmed()
        };
        println!("  {} {}", label, tool.name);
    }

    println!();
    println!("{}", "Skills".bold());
    for dest in SkillDest::all() {
        let count = skills::count_skills(&dest.path);
        let label = if count > 0 {
            format!("✓ {} skills", count).green()
        } else {
            "- none".dimmed()
        };
        println!("  {} {}", label, dest.name);
    }

    if !any_installed {
        println!();
        println!("Run {} to install", "engrammic-install".cyan());
    }

    Ok(())
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build`
Expected: compiles successfully.

- [ ] **Step 5: Manually verify status, update, and uninstall**

Run, in order:
```bash
cargo run -- status
cargo run -- update --tool claude
cargo run -- uninstall --tool claude
```
Expected: `status` lists harnesses and per-destination skill counts; `update` refreshes; `uninstall` removes the MCP entry and any `engrammic-*` skill directories. Inspect `~/.agents/skills/` to confirm removal.

- [ ] **Step 6: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat: handle skills in update, uninstall, and status"
```

### Task 11: Run the full check and update the README

**Files:**
- Modify: `installer/README.md`

- [ ] **Step 1: Run the full test suite and lints**

Run:
```bash
cargo test --bin engrammic-install
cargo clippy --bin engrammic-install -- -D warnings
```
Expected: all non-ignored tests PASS; clippy reports no warnings. Fix any warnings (commonly an unused `Select` import from Task 8).

- [ ] **Step 2: Document skills behavior in the installer README**

In `installer/README.md`, under the existing `## Files` or a new `## Skills` section, add:

```markdown
## Skills

The installer also offers to install the 21 open-source Engrammic skills from
the public `engrammic-ai/skills` repo. During `install` it prompts after
writing MCP config (opt-out, default yes) and lets you choose destinations:

- `~/.claude/skills/` (Claude Code, native)
- `~/.agents/skills/` (cross-harness: Codex, Gemini CLI, Cursor, Pi Agents)
- `./.agents/skills/` (project-local, current directory)

`update` refreshes skills in any destination that already has them.
`uninstall` removes them. `status` shows per-destination counts.
```

- [ ] **Step 3: Commit**

```bash
git add installer/README.md
git commit -m "docs: document installer skills behavior"
```

---

## Self-Review

**Spec coverage:**
- Banner (spec 1) — Task 7.
- Multi-harness MCP install + opt-out skills step (spec 2) — Tasks 8, 9.
- Skill fetch via public tarball (spec 3) — Tasks 5, 6.
- Skill destinations, three options (spec 4) — Task 3.
- Colonless skill rename (spec 5) — Task 1.
- update / uninstall / status handle skills (spec 6) — Task 10.
- Bootstrap scripts unchanged (spec 7) — no task needed; intentionally untouched.
- Cargo.toml `repository` field fix (spec deferred item) — Task 2 Step 1.
- Extracted top-level dir name not hardcoded (spec deferred item) — resolved in Task 5 (`unpack_tarball` finds the dir dynamically).

**Deferred spec items not covered by a task (intentionally):**
- `%APPDATA%` harness research — no harness in the current table needs it; left as a future override point in `tools.rs`. No task.
- `indicatif` vs static line — resolved: `indicatif` is included (Task 6).
- `ureq` TLS feature / binary size — `ureq = "2"` pulls its default TLS; binary size is acceptable under the existing `opt-level = "z"` + `strip` profile. No task.

**Type consistency:** `SkillDest` (fields `name`, `path`, `default`), `skills::install_skills` returning `Vec<(PathBuf, usize)>`, `count_skills` / `copy_skills` / `remove_skills` signatures are used consistently across Tasks 3, 4, 6, 9, 10. `select_tools` (plural) replaces `select_tool` everywhere it is called (install, update, uninstall).

**Placeholder scan:** no TBD/TODO/"handle edge cases" steps; every code step contains complete code; every command step states expected output.
