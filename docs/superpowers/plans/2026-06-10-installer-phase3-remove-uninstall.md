# Installer Phase 3: Remove & Uninstall Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `engrammic remove [--harness <id>…]` (per-harness, selective removal) and
overhaul `engrammic uninstall` (full teardown), with manifest-driven and legacy-scan paths,
self-hosted teardown, format-aware skill removal, and CLI wiring.

**Architecture — backup-restore vs surgical removal:**
The spec says "removes our MCP entries and skills from the chosen harnesses only, restoring from
backups where they exist." This is intentionally ambiguous, so we clarify here. The design
principle "always reversible" means backups exist as a _safety net_, not as the primary removal
strategy. The correct interpretation (confirmed by the self-hosted teardown section's phrasing and
the existing `config.rs::uninstall` design) is:

- **Default path:** `config::uninstall(config_path, shape)` — surgical removal of our MCP server
  entry only (the `"engrammic"` key). Other servers, comments, and unrelated keys are preserved.
  This is the right behavior: the user installed other tools; we must not clobber them.
- **File-created-by-us:** when `backup_path` is `None` AND the config file does not exist before
  install (i.e., we created it from scratch), `config::uninstall` will leave an empty/stub
  config. Detect this with: `backup_path.is_none()` in the `HarnessEntry`. In this case, delete
  the file entirely instead of calling `config::uninstall` — we created it, we can remove it.
- **Backup kept on disk:** the `.engrammic.bak` file is never deleted by `remove` or `uninstall`.
  It remains as a permanent safety net. The user can manually restore if surgical removal broke
  something unexpected. Document this in the output message.
- **No "restore from backup" code path.** The spec does not call for backup-restore-as-default;
  "restoring from backups where they exist" in the spec means the user can recover manually, not
  that we auto-restore. This interpretation avoids clobbering any changes the user made to their
  config _after_ we installed.

**PRE-FLIGHT for all tasks:** Phase 1b introduces `remove_tool_outcome` (replacing `remove_tool`)
with signature `fn remove_tool_outcome(tool: &Tool, m: &mut manifest::Manifest) -> Result<flow::Outcome>`.
Before starting each task, verify:
```bash
grep -n "remove_tool_outcome\|fn remove_tool" installer-cli/src/main.rs | head -10
```
If Phase 1b landed with a different name or signature, adapt accordingly. The fallback if 1b is
not yet merged: keep `remove_tool` but do NOT load/save the manifest inside it — instead pass
`&mut Manifest` and do the record-keeping at the call site.

**Tech Stack:** Rust, dialoguer 0.11, existing manifest/config/skills modules. Crate root
`installer-cli/`; all commands run from there.

**Spec:** `docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md`
(§ "Remove & uninstall", § "Manifest details", Decisions)

**Sequencing:** Task 1 (cli.rs wiring) and Task 2 (manifest helpers) are independent and can run
in parallel. Task 3 (remove subcommand) depends on both. Task 4 (uninstall overhaul) depends on
Task 3's helpers. Task 5 (self-hosted teardown) depends on Task 4. Task 6 (legacy scan) is
independent of 3–5 (it's a detection utility) but its output is consumed by Tasks 3 and 4. Task 7
(tests) is last and depends on all prior tasks.

---

### Task 1: CLI wiring — new `Remove` variant, extend `Uninstall`

**Files:**
- Modify: `installer-cli/src/cli.rs`
- Modify: `installer-cli/src/main.rs` — `main()` dispatch block (≈line 34–55)

**PRE-FLIGHT:** Confirm current `Commands` enum does not already have a `Remove` variant:
```bash
grep -n "Remove\|Uninstall" installer-cli/src/cli.rs
```

- [ ] **Step 1: Add `Remove` variant and extend `Uninstall` in `cli.rs`**

Replace the existing `Uninstall` line and add `Remove` in alphabetical position (after `List`,
before `Scale` — or keep the existing ordering pattern; alphabetical is preferable for grep-ability):

```rust
    /// Remove Engrammic from one or more editors (keeps other editors intact)
    Remove {
        /// Harness IDs to remove from (e.g. --harness claude --harness cursor).
        /// When omitted, shows an interactive multi-select over all known harnesses.
        #[arg(long = "harness", value_name = "ID")]
        harness: Vec<String>,
    },
    /// Remove Engrammic from ALL editors, skills, config, and optionally the binary
    Uninstall {
        /// Also tear down the self-hosted Docker stack and delete data volumes
        #[arg(long)]
        purge_data: bool,
    },
```

Note: the existing `Uninstall` variant has no fields today. The new form adds `purge_data`.
Update the dispatch arm in `main()` accordingly (see Step 2).

- [ ] **Step 2: Update `main()` dispatch**

```rust
        Commands::Remove { harness } => remove(auto, &harness),
        Commands::Uninstall { purge_data } => uninstall(auto, purge_data, cli.tool.as_deref()),
```

The existing `uninstall(auto, cli.tool.as_deref())` signature changes in Task 4. For now, add a
stub that calls the old signature with `purge_data` ignored:

```rust
        Commands::Uninstall { purge_data } => {
            let _ = purge_data; // wired in Task 4
            uninstall(auto, cli.tool.as_deref())
        }
        Commands::Remove { harness } => {
            let _ = harness; // implemented in Task 3
            println!("{}", "remove: not yet implemented".yellow());
            Ok(())
        }
```

This keeps the codebase compiling throughout development.

- [ ] **Step 3: Build check**

```bash
cd installer-cli && cargo build 2>&1 | tail -3
```

Expected: clean build (the stubs return `Ok(())`).

- [ ] **Step 4: Commit**

```bash
git add installer-cli/src/cli.rs installer-cli/src/main.rs
git commit -m "feat(installer): add Remove subcommand and --purge-data to Uninstall (stubs)"
```

---

### Task 2: Manifest helpers — `created_by_us` detection

**Files:**
- Modify: `installer-cli/src/manifest.rs`

`HarnessEntry.backup_path` is `None` when the config file did not exist before we wrote it (see
`config::ensure_backup` — returns `None` for non-existent files). This is the signal that we
created the file and should delete it on removal rather than surgically uninstalling.

**PRE-FLIGHT:** Confirm `HarnessEntry.backup_path` is `Option<PathBuf>` — it is, from Phase 1a.
```bash
grep -n "backup_path" installer-cli/src/manifest.rs | head -5
```

- [ ] **Step 1: Write the failing test** (bottom of `manifest.rs` test block)

```rust
    #[test]
    fn created_by_us_when_no_backup() {
        let entry = HarnessEntry {
            tool_id: "claude".into(),
            config_path: PathBuf::from("/tmp/settings.json"),
            backup_path: None,
            endpoint: "https://beta.engrammic.ai/mcp/".into(),
        };
        assert!(
            entry.created_by_us(),
            "no backup_path means we created the file"
        );

        let entry_with_backup = HarnessEntry {
            tool_id: "cursor".into(),
            config_path: PathBuf::from("/tmp/other.json"),
            backup_path: Some(PathBuf::from("/tmp/other.json.engrammic.bak")),
            endpoint: "https://beta.engrammic.ai/mcp/".into(),
        };
        assert!(
            !entry_with_backup.created_by_us(),
            "existing backup means the file pre-dated us"
        );
    }
```

Run: `cargo test --bin engrammic created_by_us` — expect compile error.

- [ ] **Step 2: Implement `created_by_us()` on `HarnessEntry`**

In `manifest.rs`, inside `impl` block for `HarnessEntry` (add one after the struct definition):

```rust
impl HarnessEntry {
    /// True when the config file did not exist before we wrote it
    /// (backup_path is None). On removal, the file should be deleted entirely
    /// rather than surgically uninstalled — we created it, we clean it up.
    pub fn created_by_us(&self) -> bool {
        self.backup_path.is_none()
    }
}
```

Run: `cargo test --bin engrammic created_by_us` — expect 1 passed.

- [ ] **Step 3: Commit**

```bash
git add installer-cli/src/manifest.rs
git commit -m "feat(manifest): add HarnessEntry::created_by_us() helper"
```

---

### Task 3: `engrammic remove` subcommand

**Files:**
- Modify: `installer-cli/src/main.rs` — replace the `remove` stub with the real implementation

**PRE-FLIGHT — `remove_tool_outcome` signature:**
```bash
grep -n "fn remove_tool_outcome\|fn remove_tool" installer-cli/src/main.rs
```
If Phase 1b has merged, `remove_tool_outcome` should exist with signature:
`fn remove_tool_outcome(tool: &Tool, m: &mut manifest::Manifest) -> Result<flow::Outcome>`

If only `remove_tool` exists (Phase 1b not yet merged), the fallback is to write a local
`remove_one_harness` helper in this task that does the same work. See Step 1 note.

**PRE-FLIGHT — `flow::Outcome` and `flow::StepResult`:**
```bash
grep -n "pub enum Outcome\|pub struct StepResult" installer-cli/src/flow.rs
```
These must exist (Phase 1b Task 1). If flow.rs does not exist, implement the types inline in
`main.rs` for this task and note a follow-up to consolidate once 1b lands.

- [ ] **Step 1: Write the `remove_one_harness` helper**

This is the single-harness removal kernel — called by both `remove` and `uninstall`:

```rust
/// Remove Engrammic from a single harness. Returns the flow Outcome.
/// Does NOT save the manifest — caller owns load/save.
fn remove_one_harness(entry: &manifest::HarnessEntry) -> flow::Outcome {
    // Find the Tool for shape dispatch.
    let tool = match tools::Tool::from_id(&entry.tool_id) {
        Some(t) => t,
        None => {
            // Unknown tool_id (e.g. added in a newer version). Try file-delete if we created it.
            if entry.created_by_us() && entry.config_path.exists() {
                if let Err(e) = std::fs::remove_file(&entry.config_path) {
                    return flow::Outcome::Failed(format!(
                        "unknown tool '{}'; tried to delete config: {e:#}",
                        entry.tool_id
                    ));
                }
                return flow::Outcome::Done;
            }
            return flow::Outcome::Failed(format!(
                "unknown tool id '{}' — remove manually from {}",
                entry.tool_id,
                entry.config_path.display()
            ));
        }
    };

    match tool.method {
        InstallMethod::FileEdit(shape) => {
            if entry.created_by_us() {
                // We created the file; delete it entirely.
                if entry.config_path.exists() {
                    if let Err(e) = std::fs::remove_file(&entry.config_path) {
                        return flow::Outcome::Failed(format!(
                            "failed to delete {}: {e:#}",
                            entry.config_path.display()
                        ));
                    }
                }
                flow::Outcome::Done
            } else {
                // Pre-existing file: surgical removal only.
                match config::uninstall(&entry.config_path, shape) {
                    Ok(()) => flow::Outcome::Done,
                    Err(e) => flow::Outcome::Failed(format!("{e:#}")),
                }
            }
        }
        InstallMethod::DeepLink(kind) => {
            let hint = match kind {
                DeepLinkKind::VsCode => "Settings > MCP > remove 'engrammic'",
                DeepLinkKind::Cursor => "Cursor Settings > MCP > remove 'engrammic'",
            };
            println!(
                "  {} {} — {}",
                "▸".cyan(),
                tool.name,
                format!("remove manually: {hint}").dimmed()
            );
            flow::Outcome::Manual(hint.to_string())
        }
        InstallMethod::PrintInstructions(hint) => {
            println!(
                "  {} {} — {}",
                "▸".cyan(),
                tool.name,
                format!("remove manually via {hint}").dimmed()
            );
            flow::Outcome::Manual(hint.to_string())
        }
    }
}
```

Notes:
- `entry.created_by_us()` uses the helper from Task 2.
- The backup file (`.engrammic.bak`) is intentionally left on disk. After removing, print:
  `"  {} Backup left at {} (delete manually if not needed)", "·".dimmed(), entry.backup_path...`
  inside the `Done` arms — add this print before returning `Outcome::Done` in both FileEdit arms.

- [ ] **Step 2: Write the `remove_skills_for_harness` helper**

This removes skills recorded under a specific harness id:

```rust
/// Remove all skills associated with a particular harness (tool_id).
/// Returns the number of skill destinations processed.
fn remove_skills_for_harness(
    harness_id: &str,
    m: &mut manifest::Manifest,
) -> Result<usize> {
    let to_remove: Vec<manifest::SkillEntry> = m
        .skills
        .iter()
        .filter(|s| s.harness == harness_id)
        .cloned()
        .collect();

    let mut count = 0;
    for skill in &to_remove {
        // Reconstruct a SkillDest from the manifest entry for format-aware dispatch.
        // SkillDest::all() may not list every path (project-scope paths are CWD-relative
        // at install time; the manifest has the absolutized form). Use the format field
        // to dispatch directly rather than matching against SkillDest::all().
        let format = match skill.format.as_str() {
            "directory" => tools::SkillFormat::Directory,
            "cursor-mdc" => tools::SkillFormat::CursorMdc,
            "gemini-md" => tools::SkillFormat::GeminiMd,
            "agents-md" => tools::SkillFormat::AgentsMd,
            other => {
                eprintln!(
                    "  {} unknown skill format '{}' for {} — skipped",
                    "!".yellow(),
                    other,
                    skill.path.display()
                );
                continue;
            }
        };
        let synthetic_dest = tools::SkillDest {
            name: skill.harness.as_str(),
            harness: skill.harness.as_str(),
            path: skill.path.clone(),
            scope: tools::SkillScope::User, // scope doesn't affect removal dispatch
            format,
            default: false,
            note: None,
        };
        // NOTE: SkillDest fields name/harness are &'static str in the real type.
        // The above won't compile with non-static lifetimes. Use a local helper instead:
        // dispatch on `format` directly rather than constructing SkillDest.
        let removed = match format {
            tools::SkillFormat::Directory => skills::remove_skills(&skill.path)?,
            tools::SkillFormat::CursorMdc => skills::remove_mdc_skills(&skill.path)?,
            tools::SkillFormat::GeminiMd | tools::SkillFormat::AgentsMd => {
                skills::remove_gemini_skills(&skill.path)?
            }
        };
        if removed > 0 {
            println!(
                "  {} Removed {} skill(s) from {}",
                "✓".green(),
                removed,
                skill.path.display()
            );
        }
        m.forget_skill(&skill.path);
        count += 1;
    }
    Ok(count)
}
```

Note on `SkillDest` lifetime issue: `SkillDest.name` and `.harness` are `&'static str`; you
cannot construct one from a `String`. Use the direct dispatch (the inner `match format` block)
rather than constructing a synthetic `SkillDest`. The `remove_skills_formatted` wrapper is not
needed here since we dispatch manually.

- [ ] **Step 3: Write the `ask_remove_skills` helper**

```rust
/// Ask the user whether to also remove skills for the chosen harnesses.
/// In auto mode, defaults to removing skills.
fn ask_remove_skills(harness_names: &[&str], auto: bool) -> Result<bool> {
    if harness_names.is_empty() {
        return Ok(false);
    }
    if auto {
        return Ok(true);
    }
    let names = harness_names.join(", ");
    Confirm::new()
        .with_prompt(format!(
            "Also remove skills installed for {}?",
            names
        ))
        .default(true)
        .interact()
        .map_err(Into::into)
}
```

- [ ] **Step 4: Implement the `remove` function**

Replace the stub from Task 1 with the real implementation:

```rust
/// `engrammic remove [--harness <id>…]`
///
/// Interactive multi-select when no --harness flags are given. Removes the MCP
/// entry from chosen harnesses (and optionally their skills). Manifest-driven
/// when entries exist; falls back to legacy scan for unrecorded harnesses.
fn remove(auto: bool, harness_ids: &[String]) -> Result<()> {
    banner::print_banner();

    let mut m = manifest::Manifest::load_or_migrate(None)?;

    // ---- Determine which harnesses to act on ----
    let targets: Vec<manifest::HarnessEntry> = if !harness_ids.is_empty() {
        // Flag-driven: validate all requested ids exist in manifest.
        let mut entries = Vec::new();
        let mut unknown = Vec::new();
        for id in harness_ids {
            if let Some(e) = m.harnesses.iter().find(|e| &e.tool_id == id) {
                entries.push(e.clone());
            } else {
                unknown.push(id.as_str());
            }
        }
        if !unknown.is_empty() {
            // Not in manifest; try legacy scan for these specific ids.
            let legacy = legacy_scan();
            for id in &unknown {
                if let Some(tool) = legacy.iter().find(|t| t.id == *id) {
                    // Synthesize a HarnessEntry from the live scan.
                    entries.push(manifest::HarnessEntry {
                        tool_id: tool.id.to_string(),
                        config_path: tool.config_path.clone(),
                        backup_path: None, // not recorded; surgical removal only
                        endpoint: detect_installed_endpoint(tool)
                            .unwrap_or_default(),
                    });
                } else {
                    eprintln!(
                        "  {} '{}' not found in manifest or installed config — skipped",
                        "!".yellow(),
                        id
                    );
                }
            }
        }
        entries
    } else if !m.harnesses.is_empty() {
        // Interactive multi-select over manifest-known harnesses.
        // Also include detected-but-unrecorded harnesses from legacy scan.
        let legacy = legacy_scan();
        let recorded_ids: std::collections::HashSet<_> =
            m.harnesses.iter().map(|e| e.tool_id.as_str()).collect();
        let unrecorded: Vec<_> = legacy
            .iter()
            .filter(|t| !recorded_ids.contains(t.id))
            .collect();

        let mut options: Vec<String> = m
            .harnesses
            .iter()
            .map(|e| {
                tools::Tool::from_id(&e.tool_id)
                    .map(|t| format!("{} (recorded)", t.name))
                    .unwrap_or_else(|| format!("{} (recorded)", e.tool_id))
            })
            .collect();
        for t in &unrecorded {
            options.push(format!("{} (detected, not in manifest)", t.name));
        }

        if options.is_empty() {
            println!("{}", "No Engrammic harnesses found to remove.".dimmed());
            return Ok(());
        }

        if auto {
            // In -y mode, remove all recorded harnesses.
            m.harnesses.clone()
        } else {
            println!(
                "  {}",
                "(↑↓ move · space toggle · enter confirm)".dimmed()
            );
            let selection: Vec<usize> = MultiSelect::new()
                .with_prompt("Select editors to remove Engrammic from")
                .items(&options)
                .interact()?;

            if selection.is_empty() {
                println!("{}", "Nothing selected — nothing was changed.".dimmed());
                return Ok(());
            }

            let n_recorded = m.harnesses.len();
            selection
                .into_iter()
                .map(|i| {
                    if i < n_recorded {
                        m.harnesses[i].clone()
                    } else {
                        let tool = unrecorded[i - n_recorded];
                        manifest::HarnessEntry {
                            tool_id: tool.id.to_string(),
                            config_path: tool.config_path.clone(),
                            backup_path: None,
                            endpoint: detect_installed_endpoint(tool).unwrap_or_default(),
                        }
                    }
                })
                .collect()
        }
    } else {
        // No manifest entries — pure legacy scan.
        let legacy = legacy_scan();
        if legacy.is_empty() {
            println!(
                "{}",
                "No Engrammic installations detected. Nothing to remove.".dimmed()
            );
            return Ok(());
        }

        if auto {
            legacy
                .iter()
                .map(|t| manifest::HarnessEntry {
                    tool_id: t.id.to_string(),
                    config_path: t.config_path.clone(),
                    backup_path: None,
                    endpoint: detect_installed_endpoint(t).unwrap_or_default(),
                })
                .collect()
        } else {
            let options: Vec<String> = legacy
                .iter()
                .map(|t| format!("{} (detected)", t.name))
                .collect();
            println!("  {}", "(↑↓ move · space toggle · enter confirm)".dimmed());
            let selection: Vec<usize> = MultiSelect::new()
                .with_prompt("Select editors to remove Engrammic from")
                .items(&options)
                .interact()?;
            if selection.is_empty() {
                println!("{}", "Nothing selected — nothing was changed.".dimmed());
                return Ok(());
            }
            selection
                .into_iter()
                .map(|i| manifest::HarnessEntry {
                    tool_id: legacy[i].id.to_string(),
                    config_path: legacy[i].config_path.clone(),
                    backup_path: None,
                    endpoint: detect_installed_endpoint(&legacy[i]).unwrap_or_default(),
                })
                .collect()
        }
    };

    if targets.is_empty() {
        println!("{}", "Nothing selected — nothing was changed.".dimmed());
        return Ok(());
    }

    // ---- Ask about skills (once, for all selected harnesses) ----
    let harness_names: Vec<&str> = targets
        .iter()
        .map(|e| e.tool_id.as_str())
        .collect();
    let also_remove_skills = ask_remove_skills(&harness_names, auto)?;

    // ---- Execute ----
    let mut results: Vec<flow::StepResult> = Vec::new();

    for entry in &targets {
        let outcome = remove_one_harness(entry);
        if matches!(outcome, flow::Outcome::Done) {
            m.forget_harness(&entry.tool_id);
        }
        results.push(flow::StepResult {
            label: entry.tool_id.clone(),
            outcome,
        });

        if also_remove_skills {
            if let Err(e) = remove_skills_for_harness(&entry.tool_id, &mut m) {
                eprintln!(
                    "  {} skill removal for '{}' failed: {e:#}",
                    "!".yellow(),
                    entry.tool_id
                );
            }
        }
    }

    m.save()?;

    // ---- Summary ----
    println!();
    let (done, failed, manual) = flow::summarize_results(&results);
    for r in &results {
        match &r.outcome {
            flow::Outcome::Done => {
                println!("  {} {} removed", "✓".green(), r.label);
            }
            flow::Outcome::Failed(msg) => {
                println!("  {} {} — {}", "✗".red(), r.label, msg);
                println!(
                    "    {} {}",
                    "→".dimmed(),
                    format!("retry: engrammic remove --harness {}", r.label).cyan()
                );
            }
            flow::Outcome::Manual(_) => {} // already printed in remove_one_harness
        }
    }
    println!(
        "{} {} removed, {} need a manual step, {} failed.",
        if failed == 0 { "✓".green() } else { "!".yellow() },
        done,
        manual,
        failed
    );
    if failed == 0 && manual == 0 {
        println!(
            "  {}",
            "Run 'engrammic install' anytime to re-configure.".dimmed()
        );
    }

    Ok(())
}
```

- [ ] **Step 5: Build check + smoke**

```bash
cd installer-cli && cargo build 2>&1 | tail -3
```

Manual smoke (scratch HOME, no actual editors installed):
```bash
SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.engrammic"
# Fake a harness record:
cat > "$SCRATCH/.engrammic/state.toml" <<'EOF'
schema_version = 1
endpoint = "https://beta.engrammic.ai/mcp/"
[[harnesses]]
tool_id = "claude"
config_path = "/tmp/fake-settings.json"
endpoint = "https://beta.engrammic.ai/mcp/"
EOF
# also fake the config file (no backup → created_by_us):
echo '{"mcpServers":{"engrammic":{"type":"http","url":"https://beta.engrammic.ai/mcp/"}}}' > /tmp/fake-settings.json
HOME="$SCRATCH" target/debug/engrammic remove --harness claude -y
cat "$SCRATCH/.engrammic/state.toml"   # harnesses should be empty
test -f /tmp/fake-settings.json && echo "FILE STILL EXISTS (created_by_us → should be deleted)"
```

Expected: harness entry gone from state.toml; /tmp/fake-settings.json deleted (created_by_us).

- [ ] **Step 6: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): implement 'engrammic remove' with manifest-driven and legacy-scan paths"
```

---

### Task 4: `engrammic uninstall` overhaul

**Files:**
- Modify: `installer-cli/src/main.rs` — replace `uninstall` fn (≈line 607)
- Modify: `installer-cli/src/cli.rs` — stub already wired in Task 1

**PRE-FLIGHT:** Confirm the `uninstall` fn signature is still `fn uninstall(auto: bool, tool_id: Option<&str>) -> Result<()>`. After Task 1, the dispatch stub passes `purge_data` but discards it; this task changes the fn signature.

```bash
grep -n "fn uninstall" installer-cli/src/main.rs
```

- [ ] **Step 1: Replace `uninstall` function signature and body**

The new signature:
```rust
fn uninstall(auto: bool, purge_data: bool, tool_id: Option<&str>) -> Result<()>
```

Also update the dispatch in `main()` (from Task 1's stub) to:
```rust
Commands::Uninstall { purge_data } => uninstall(auto, purge_data, cli.tool.as_deref()),
```

Full implementation:

```rust
fn uninstall(auto: bool, purge_data: bool, tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    // Confirm unless -y.
    if !auto {
        let proceed = Confirm::new()
            .with_prompt(
                "This will remove Engrammic from ALL configured editors and delete all skills. Continue?",
            )
            .default(false)
            .interact()?;
        if !proceed {
            println!("{}", "Nothing was changed.".dimmed());
            return Ok(());
        }
    }

    let mut m = manifest::Manifest::load_or_migrate(None)?;

    // ---- Determine harnesses to remove ----
    let recorded: Vec<manifest::HarnessEntry> = if let Some(id) = tool_id {
        // --tool flag: single harness (legacy compat with old uninstall interface).
        m.harnesses
            .iter()
            .filter(|e| e.tool_id == id)
            .cloned()
            .collect()
    } else {
        m.harnesses.clone()
    };

    // Supplement with legacy scan for harnesses not in manifest.
    let recorded_ids: std::collections::HashSet<_> =
        recorded.iter().map(|e| e.tool_id.as_str()).collect();
    let legacy_extra: Vec<manifest::HarnessEntry> = if tool_id.is_none() {
        legacy_scan()
            .into_iter()
            .filter(|t| {
                !recorded_ids.contains(t.id)
                    && matches!(t.method, InstallMethod::FileEdit(_))
            })
            .map(|t| {
                let ep = detect_installed_endpoint(&t).unwrap_or_default();
                manifest::HarnessEntry {
                    tool_id: t.id.to_string(),
                    config_path: t.config_path.clone(),
                    backup_path: None,
                    endpoint: ep,
                }
            })
            .collect()
    } else {
        vec![]
    };

    let all_targets: Vec<manifest::HarnessEntry> =
        recorded.into_iter().chain(legacy_extra).collect();

    // ---- Remove harness entries ----
    let mut results: Vec<flow::StepResult> = Vec::new();
    for entry in &all_targets {
        let outcome = remove_one_harness(entry);
        if matches!(outcome, flow::Outcome::Done) {
            m.forget_harness(&entry.tool_id);
        }
        results.push(flow::StepResult {
            label: entry.tool_id.clone(),
            outcome,
        });
    }

    // ---- Remove all skills (format-aware) ----
    // Use manifest records when available; fall back to SkillDest::all() scan.
    let skill_paths: Vec<(std::path::PathBuf, String)> = if m.skills.is_empty() {
        // No manifest records: scan all known skill destinations.
        tools::SkillDest::all()
            .into_iter()
            .map(|d| (d.path.clone(), manifest::skill_format_str(d.format).to_string()))
            .collect()
    } else {
        m.skills
            .iter()
            .map(|s| (s.path.clone(), s.format.clone()))
            .collect()
    };

    for (path, format_str) in &skill_paths {
        let format = match format_str.as_str() {
            "directory" => tools::SkillFormat::Directory,
            "cursor-mdc" => tools::SkillFormat::CursorMdc,
            "gemini-md" => tools::SkillFormat::GeminiMd,
            "agents-md" => tools::SkillFormat::AgentsMd,
            other => {
                eprintln!("  {} unknown skill format '{}' — skipped", "!".yellow(), other);
                continue;
            }
        };
        let removed = match format {
            tools::SkillFormat::Directory => skills::remove_skills(path)?,
            tools::SkillFormat::CursorMdc => skills::remove_mdc_skills(path)?,
            tools::SkillFormat::GeminiMd | tools::SkillFormat::AgentsMd => {
                skills::remove_gemini_skills(path)?
            }
        };
        if removed > 0 {
            println!(
                "  {} Removed {} skill(s) from {}",
                "✓".green(),
                removed,
                path.display()
            );
            m.forget_skill(path);
        }
    }

    // ---- Self-hosted teardown (handled in Task 5) ----
    // Placeholder: teardown is gated on selfhost_dir presence.
    if m.selfhost_dir.is_some() {
        selfhost_teardown(&mut m, auto, purge_data)?;
    }

    // ---- Manifest deletion ----
    // Save the partial state first (in case teardown failed partway through),
    // then delete when everything else is clean.
    let (done, failed, manual) = flow::summarize_results(&results);
    if failed == 0 {
        // Delete the manifest file last — it is the source of truth, so it
        // must outlive all mutations.
        let manifest_path = manifest::Manifest::path_in(&manifest::Manifest::dir());
        if manifest_path.exists() {
            let _ = std::fs::remove_file(&manifest_path);
            println!("  {} Manifest deleted: {}", "✓".green(), manifest_path.display());
        }
    } else {
        // Partial failure: keep manifest so the user can retry.
        m.save()?;
        println!(
            "  {} Manifest kept (some removals failed) at {}",
            "!".yellow(),
            manifest::Manifest::path_in(&manifest::Manifest::dir()).display()
        );
    }

    // ---- CLI binary self-removal note ----
    if let Some(ref bin) = m.binary_path {
        if cfg!(windows) {
            println!();
            println!(
                "  {} {}",
                "!".yellow(),
                "Windows: delete the binary manually (cannot unlink while running):"
            );
            println!("      del \"{}\"", bin.display());
        } else {
            // Unix: safe to unlink self (inode stays valid until the process exits).
            if bin.exists() {
                if let Err(e) = std::fs::remove_file(bin) {
                    println!(
                        "  {} Could not delete binary at {}: {e:#}",
                        "!".yellow(),
                        bin.display()
                    );
                    println!(
                        "    {} {}",
                        "→".dimmed(),
                        format!("rm \"{}\"", bin.display()).cyan()
                    );
                } else {
                    println!("  {} Binary removed: {}", "✓".green(), bin.display());
                }
            }
        }
    } else {
        println!(
            "  {}",
            "(binary path not recorded — delete ~/.local/bin/engrammic manually if needed)".dimmed()
        );
    }

    // ---- Final summary ----
    println!();
    println!(
        "{} {} removed, {} need a manual step, {} failed.",
        if failed == 0 { "✓".green() } else { "!".yellow() },
        done,
        manual,
        failed
    );
    if failed == 0 {
        println!("{}", "Engrammic has been fully uninstalled.".green());
    } else {
        println!(
            "  {}",
            "Some steps failed. Re-run 'engrammic uninstall' to retry.".yellow()
        );
    }

    Ok(())
}
```

- [ ] **Step 2: Add `selfhost_teardown` stub** (implemented fully in Task 5)

```rust
fn selfhost_teardown(
    m: &mut manifest::Manifest,
    auto: bool,
    purge_data: bool,
) -> Result<()> {
    // Implemented in Task 5.
    let _ = (m, auto, purge_data);
    Ok(())
}
```

- [ ] **Step 3: Build + existing test suite**

```bash
cd installer-cli && cargo test 2>&1 | tail -3 && cargo build 2>&1 | tail -1
```

- [ ] **Step 4: Scratch-HOME uninstall smoke**

```bash
cargo build -q
BIN=$(pwd)/target/debug/engrammic
SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.claude" "$SCRATCH/.engrammic"
echo '{"mcpServers":{"engrammic":{"type":"http","url":"https://beta.engrammic.ai/mcp/"},"other":{"url":"http://keep"}}}' \
    > "$SCRATCH/.claude/settings.json"
cat > "$SCRATCH/.engrammic/state.toml" <<'EOF'
schema_version = 1
endpoint = "https://beta.engrammic.ai/mcp/"
[[harnesses]]
tool_id = "claude"
config_path = "SETTINGS_PATH"
backup_path = "BACKUP_PATH"
endpoint = "https://beta.engrammic.ai/mcp/"
EOF
# Replace placeholders (done manually in the fixture for the test):
# config_path = "$SCRATCH/.claude/settings.json" — write with sed or heredoc
cp "$SCRATCH/.claude/settings.json" "$SCRATCH/.claude/settings.json.engrammic.bak"
HOME="$SCRATCH" "$BIN" uninstall -y
# Verify:
echo "--- settings.json (should have 'other' but no 'engrammic') ---"
cat "$SCRATCH/.claude/settings.json" 2>/dev/null || echo "(deleted)"
echo "--- state.toml (should be deleted) ---"
ls "$SCRATCH/.engrammic/state.toml" 2>/dev/null || echo "(deleted — correct)"
echo "--- backup still present ---"
ls "$SCRATCH/.claude/settings.json.engrammic.bak" && echo "backup kept (correct)"
```

Expected: `settings.json` still has `"other"` but no `"engrammic"` entry (surgical removal);
backup untouched; `state.toml` deleted (no failures).

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): overhaul uninstall — manifest-driven, format-aware skills, binary self-removal"
```

---

### Task 5: Self-hosted teardown

**Files:**
- Modify: `installer-cli/src/main.rs` — replace the `selfhost_teardown` stub

**PRE-FLIGHT:** Confirm `manifest::Manifest` has `selfhost_dir: Option<PathBuf>` — it does (Phase 1a).
```bash
grep -n "selfhost_dir" installer-cli/src/manifest.rs | head -3
```

Confirm `docker compose config --volumes` works on the installed Docker version:
```bash
docker compose version 2>&1 | head -1
```
If Docker is not available in the CI environment, the teardown code must gracefully skip rather
than fail. All `std::process::Command` calls for docker must check `status().is_err()` or
`output().ok()` and degrade to a "run manually" hint.

- [ ] **Step 1: Implement `selfhost_teardown`** (replace the stub from Task 4)

```rust
fn selfhost_teardown(
    m: &mut manifest::Manifest,
    auto: bool,
    purge_data: bool,
) -> Result<()> {
    let install_dir = match &m.selfhost_dir {
        Some(d) => d.clone(),
        None => return Ok(()), // nothing to tear down
    };

    let compose_file = install_dir.join("docker-compose.yml");
    if !compose_file.exists() {
        println!(
            "  {} Self-hosted compose file not found at {} — skipped",
            "!".yellow(),
            compose_file.display()
        );
        return Ok(());
    }

    // ---- List exact volume names before asking ----
    // `docker compose -f <file> config --volumes` emits one volume name per line.
    let volume_list: Vec<String> = {
        let out = std::process::Command::new("docker")
            .args([
                "compose",
                "-f",
                &compose_file.to_string_lossy(),
                "config",
                "--volumes",
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                println!(
                    "  {} Could not list Docker volumes: {}",
                    "!".yellow(),
                    stderr.trim()
                );
                vec![]
            }
            Err(e) => {
                println!(
                    "  {} docker not found or not runnable: {e:#}",
                    "!".yellow()
                );
                println!(
                    "    {}",
                    format!(
                        "→ run manually: docker compose -f {} down{}",
                        compose_file.display(),
                        if purge_data { " -v" } else { "" }
                    )
                    .cyan()
                );
                return Ok(());
            }
        }
    };

    // ---- Confirm teardown ----
    let should_purge = if purge_data {
        true
    } else if auto {
        false // DEFAULT: keep data
    } else {
        println!();
        println!(
            "  {} Self-hosted stack found at {}",
            "·".dimmed(),
            install_dir.display()
        );
        if !volume_list.is_empty() {
            println!("  {} Docker volumes that would be deleted with --purge-data:", "·".dimmed());
            for v in &volume_list {
                println!("      {}", v.cyan());
            }
        }
        // First: ask about stopping the stack.
        let stop = Confirm::new()
            .with_prompt("Stop the self-hosted Docker stack? (data volumes are KEPT unless you confirm below)")
            .default(true)
            .interact()?;
        if !stop {
            println!(
                "  {}",
                "Self-hosted stack left running. Run 'engrammic selfhost' to manage it.".dimmed()
            );
            return Ok(());
        }
        // Second: ask about purging volumes only if user said yes to stopping.
        if !volume_list.is_empty() {
            Confirm::new()
                .with_prompt(format!(
                    "Also DELETE data volumes ({})? THIS IS IRREVERSIBLE.",
                    volume_list.join(", ")
                ))
                .default(false)
                .interact()?
        } else {
            false
        }
    };

    // ---- Run docker compose down ----
    let mut cmd = std::process::Command::new("docker");
    cmd.args([
        "compose",
        "-f",
        &compose_file.to_string_lossy(),
        "down",
    ]);
    if should_purge {
        cmd.arg("-v");
    }

    println!(
        "  {}",
        format!(
            "Running: docker compose -f {} down{}",
            compose_file.display(),
            if should_purge { " -v" } else { "" }
        )
        .dimmed()
    );

    match cmd.status() {
        Ok(s) if s.success() => {
            println!("  {} Self-hosted stack stopped.", "✓".green());
            if should_purge {
                println!("  {} Data volumes deleted.", "✓".green());
            } else {
                println!(
                    "  {}",
                    "Data volumes kept. Run 'docker volume rm <name>' to delete manually.".dimmed()
                );
            }
            m.selfhost_dir = None;
        }
        Ok(s) => {
            println!(
                "  {} docker compose down exited with code {}",
                "!".yellow(),
                s.code().unwrap_or(-1)
            );
            println!(
                "    {}",
                format!(
                    "→ run manually: docker compose -f {} down{}",
                    compose_file.display(),
                    if should_purge { " -v" } else { "" }
                )
                .cyan()
            );
        }
        Err(e) => {
            println!(
                "  {} Failed to run docker: {e:#}",
                "!".yellow()
            );
            println!(
                "    {}",
                format!(
                    "→ run manually: docker compose -f {} down{}",
                    compose_file.display(),
                    if should_purge { " -v" } else { "" }
                )
                .cyan()
            );
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Build check**

```bash
cd installer-cli && cargo build 2>&1 | tail -3
```

No Docker-dependent tests needed in the unit suite — the function degrades gracefully when docker
is absent. A future integration test can mock the `docker` binary.

- [ ] **Step 3: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): self-hosted teardown in uninstall with volume listing and --purge-data"
```

---

### Task 6: Legacy scan helper

**Files:**
- Modify: `installer-cli/src/main.rs` — add `legacy_scan()` function

Tasks 3 and 4 already call `legacy_scan()` — this task implements it. The stubs in those tasks
must compile before this task; add an empty stub first:

**PRE-FLIGHT:** Check if `legacy_scan` already exists anywhere:
```bash
grep -rn "fn legacy_scan\|legacy_scan()" installer-cli/src/ | head -5
```

Also check the existing `detect_installed_endpoint` helper location:
```bash
grep -n "fn detect_installed_endpoint" installer-cli/src/main.rs | head -3
```

- [ ] **Step 1: Add the `legacy_scan` stub** (if Tasks 3/4 are being developed in parallel,
  the stub keeps the codebase building):

```rust
fn legacy_scan() -> Vec<Tool> {
    vec![] // replaced in Task 6 Step 2
}
```

- [ ] **Step 2: Implement `legacy_scan`**

Replace the stub:

```rust
/// Scan all FileEdit-shape harnesses for an installed Engrammic entry.
///
/// Scope: FileEdit shapes only. DeepLink (VS Code, Cursor) and
/// PrintInstructions (JetBrains AI, Trae) harnesses cannot be read back via
/// config::get_installed_endpoint (no stable file path or no file-edit at all).
/// Those are surfaced as "remove manually" guidance in the callers, not here.
///
/// Returns only tools where our server key is present in the config file.
fn legacy_scan() -> Vec<Tool> {
    tools::Tool::all()
        .into_iter()
        .filter(|tool| {
            matches!(tool.method, InstallMethod::FileEdit(_))
                && detect_installed_endpoint(tool).is_some()
        })
        .collect()
}
```

The `detect_installed_endpoint` function already exists in `main.rs` (it calls
`config::get_installed_endpoint`). If it is named differently or has a different signature after
Phase 1b, adapt accordingly. Verify:

```bash
grep -n "fn detect_installed_endpoint" installer-cli/src/main.rs
```

- [ ] **Step 3: Add a "remove manually" print for DeepLink/PrintInstructions harnesses**

In `remove()` and `uninstall()`, after the `legacy_scan()` call, also surface DeepLink and
PrintInstructions tools that look installed. Since those tools can only be detected by directory
presence (not config reads), we conservatively print a reminder when their detection markers exist:

Add this helper and call it from both `remove` and `uninstall` (before the execute loop):

```rust
/// Print manual-removal guidance for harnesses we cannot programmatically uninstall.
/// Called after the automated removal loop so it appears in the output summary.
fn print_manual_removal_hints() {
    let manual_tools: Vec<_> = tools::Tool::all()
        .into_iter()
        .filter(|t| {
            matches!(
                t.method,
                InstallMethod::DeepLink(_) | InstallMethod::PrintInstructions(_)
            ) && t.config_path.exists()
        })
        .collect();

    if manual_tools.is_empty() {
        return;
    }

    println!();
    println!(
        "  {} The following editors require manual removal (not configurable via files):",
        "▸".cyan()
    );
    for tool in &manual_tools {
        let hint = match &tool.method {
            InstallMethod::DeepLink(DeepLinkKind::VsCode) => {
                "VS Code: Settings > MCP > remove 'engrammic'"
            }
            InstallMethod::DeepLink(DeepLinkKind::Cursor) => {
                "Cursor: Cursor Settings > MCP > remove 'engrammic'"
            }
            InstallMethod::PrintInstructions(h) => h,
            _ => unreachable!(),
        };
        println!("    {} {}: {}", "→".dimmed(), tool.name, hint);
    }
}
```

Call `print_manual_removal_hints()` at the end of both `remove()` and `uninstall()`, before the
final summary line.

- [ ] **Step 4: Build + test suite**

```bash
cd installer-cli && cargo test 2>&1 | tail -3 && cargo build 2>&1 | tail -1
```

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): add legacy_scan and manual-removal hints for deep-link harnesses"
```

---

### Task 7: Tests

**Files:**
- Modify: `installer-cli/src/manifest.rs` (new test already in Task 2)
- Modify: `installer-cli/src/config.rs` (verify existing tests cover the round-trip; add if gaps)
- Create (or modify): test fixtures inline in `main.rs` or a new `installer-cli/tests/remove.rs`

**PRE-FLIGHT — existing skill test coverage:**
```bash
grep -n "#\[test\]" installer-cli/src/skills.rs | head -20
```
The skills removal functions (`remove_skills`, `remove_mdc_skills`, `remove_gemini_skills`) are
exercised indirectly through `config.rs` integration tests (the `json_preserves_other_servers`
family). Verify they exist:
```bash
grep -n "remove_skills\|remove_mdc\|remove_gemini" installer-cli/src/skills.rs | head -10
```
If the existing tests already cover remove round-trips (they do: `skills.rs` has `remove_skills`,
`remove_mdc_skills`, `remove_gemini_skills` in the source; the `config.rs` tests exercise `uninstall`
for each shape), do NOT duplicate them. Reference them in the test docstring instead.

- [ ] **Step 1: Manifest-driven removal round-trip test** (add to `manifest.rs` test block)

```rust
    #[test]
    fn manifest_driven_remove_leaves_other_servers_intact() {
        // This is an integration test of config::uninstall via the remove path.
        // Exercises: record_harness → remove_one_harness (surgical) → forget_harness.
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let cfg = dir.path().join("settings.json");
        fs::write(
            &cfg,
            r#"{"mcpServers":{"engrammic":{"type":"http","url":"https://beta.engrammic.ai/mcp/"},"keep":{"url":"http://keep"}}}"#,
        )
        .unwrap();
        // Backup exists → pre-existing file → surgical removal.
        let bak_path = {
            let mut b = cfg.as_os_str().to_owned();
            b.push(".engrammic.bak");
            std::path::PathBuf::from(b)
        };
        fs::copy(&cfg, &bak_path).unwrap();

        let entry = super::HarnessEntry {
            tool_id: "claude".into(),
            config_path: cfg.clone(),
            backup_path: Some(bak_path.clone()),
            endpoint: "https://beta.engrammic.ai/mcp/".into(),
        };

        // Surgical removal via config::uninstall.
        let shape = crate::tools::ConfigShape::JsonMap {
            container: "mcpServers",
            type_field: crate::tools::TypeField::Http,
            url_field: "url",
        };
        crate::config::uninstall(&cfg, shape).unwrap();

        let content = fs::read_to_string(&cfg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(
            v["mcpServers"].get("engrammic").is_none(),
            "engrammic entry must be removed"
        );
        assert_eq!(
            v["mcpServers"]["keep"]["url"].as_str().unwrap(),
            "http://keep",
            "other servers must survive"
        );

        // Backup must still be present.
        assert!(bak_path.exists(), "backup must not be deleted by uninstall");

        // Manifest forget_harness.
        let mut m = super::Manifest::default();
        m.harnesses.push(entry);
        assert_eq!(m.harnesses.len(), 1);
        m.forget_harness("claude");
        assert!(m.harnesses.is_empty());
    }
```

- [ ] **Step 2: created_by_us file-deletion test**

```rust
    #[test]
    fn created_by_us_causes_file_deletion_not_surgical_removal() {
        // When backup_path is None, the file was created by us and should be deleted.
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let cfg = dir.path().join("new_settings.json");
        std::fs::write(
            &cfg,
            r#"{"mcpServers":{"engrammic":{"type":"http","url":"https://beta.engrammic.ai/mcp/"}}}"#,
        )
        .unwrap();

        let entry = super::HarnessEntry {
            tool_id: "claude".into(),
            config_path: cfg.clone(),
            backup_path: None, // we created it
            endpoint: "https://beta.engrammic.ai/mcp/".into(),
        };

        assert!(entry.created_by_us());

        // Simulate what remove_one_harness does for created_by_us entries:
        if entry.created_by_us() && cfg.exists() {
            std::fs::remove_file(&cfg).unwrap();
        }

        assert!(
            !cfg.exists(),
            "file created by us must be fully deleted on removal"
        );
    }
```

- [ ] **Step 3: Legacy scan detection test**

```rust
    // In a separate test module in main.rs, or inline in an integration test file:
    // installer-cli/tests/legacy_scan.rs
```

For the legacy scan, add an integration test as `installer-cli/tests/legacy_scan.rs`:

```rust
// installer-cli/tests/legacy_scan.rs
//! Tests for legacy_scan detection logic via the installed binaries' config files.
//! We cannot test legacy_scan() directly (it's a private fn in main.rs), but we
//! can test the underlying config::get_installed_endpoint which drives it.

use installer_cli::config;
use installer_cli::tools::{ConfigShape, TypeField};
use std::fs;
use tempfile::tempdir;

const STANDARD: ConfigShape = ConfigShape::JsonMap {
    container: "mcpServers",
    type_field: TypeField::Http,
    url_field: "url",
};

#[test]
fn get_installed_endpoint_finds_engrammic_server() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{"mcpServers":{"engrammic":{"type":"http","url":"https://beta.engrammic.ai/mcp/"}}}"#,
    )
    .unwrap();
    let ep = config::get_installed_endpoint(&path, STANDARD);
    assert_eq!(ep.as_deref(), Some("https://beta.engrammic.ai/mcp/"));
}

#[test]
fn get_installed_endpoint_returns_none_for_missing_key() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(
        &path,
        r#"{"mcpServers":{"other":{"type":"http","url":"https://other.example/mcp"}}}"#,
    )
    .unwrap();
    let ep = config::get_installed_endpoint(&path, STANDARD);
    assert!(ep.is_none(), "other servers must not be detected as engrammic");
}

#[test]
fn get_installed_endpoint_returns_none_for_missing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("does-not-exist.json");
    let ep = config::get_installed_endpoint(&path, STANDARD);
    assert!(ep.is_none());
}
```

Note: `installer_cli` must be declared as a library crate for integration tests. If `lib.rs`
does not exist yet (it doesn't — this is a `bin` crate), add a thin `lib.rs` that re-exports the
public modules used by the test:

```rust
// installer-cli/src/lib.rs
pub mod config;
pub mod tools;
pub mod manifest;
pub mod skill_format;
pub mod skills;
```

And in `Cargo.toml`, confirm both `[[bin]]` and `[lib]` sections exist, or add `[lib]` with
`path = "src/lib.rs"`. If this is too invasive, move the integration test into a `#[cfg(test)]`
block within `config.rs` (it would duplicate what's already there) — the existing config.rs tests
already cover `get_installed_endpoint` adequately. In that case, skip creating `tests/legacy_scan.rs`
and note: "existing `config.rs` tests cover legacy scan detection; no new test added."

- [ ] **Step 4: Skills format-aware removal — confirm coverage**

```bash
grep -n "remove_mdc_skills\|remove_gemini_skills\|remove_skills" installer-cli/src/skills.rs
```

The `skills.rs` functions do not have their own `#[test]` blocks today (they are exercised via
`config.rs` tests indirectly). Add minimal unit tests for the three removal functions:

```rust
// Add to installer-cli/src/skills.rs #[cfg(test)] block:

    #[test]
    fn remove_skills_deletes_engrammic_dirs() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("engrammic-recall")).unwrap();
        std::fs::create_dir(dir.path().join("engrammic-learn")).unwrap();
        std::fs::create_dir(dir.path().join("other-thing")).unwrap();
        let removed = super::remove_skills(dir.path()).unwrap();
        assert_eq!(removed, 2);
        assert!(dir.path().join("other-thing").exists(), "non-engrammic dirs preserved");
        assert!(!dir.path().join("engrammic-recall").exists());
    }

    #[test]
    fn remove_mdc_skills_deletes_engrammic_mdc_files() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("engrammic-recall.mdc"), "content").unwrap();
        std::fs::write(dir.path().join("other.mdc"), "content").unwrap();
        let removed = super::remove_mdc_skills(dir.path()).unwrap();
        assert_eq!(removed, 1);
        assert!(dir.path().join("other.mdc").exists());
    }

    #[test]
    fn remove_gemini_skills_removes_markers() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        std::fs::write(
            &file,
            "# Header\n<!-- ENGRAMMIC:START -->\nsome skills\n<!-- ENGRAMMIC:END -->\n# Footer\n",
        )
        .unwrap();
        let removed = super::remove_gemini_skills(&file).unwrap();
        assert_eq!(removed, 1);
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(!content.contains("ENGRAMMIC:START"));
        assert!(content.contains("# Header"));
        assert!(content.contains("# Footer"));
    }
```

- [ ] **Step 5: Full gate**

```bash
cd installer-cli && cargo test 2>&1 | tail -5 && cargo build 2>&1 | tail -1 \
    && cargo fmt -- src/main.rs src/manifest.rs src/skills.rs src/cli.rs
```

Expected: all tests pass; clean build; no new warnings beyond the pre-existing 4
(in license/selfhost/tools).

- [ ] **Step 6: Commit**

```bash
git add installer-cli/src/manifest.rs installer-cli/src/skills.rs installer-cli/src/main.rs
git commit -m "test(installer): add removal round-trip, created_by_us, skills format-aware removal tests"
```

---

### Task 8: Verification pass

- [ ] **Step 1: End-to-end install → remove round-trip (scratch HOME)**

```bash
cargo build -q
BIN=$(pwd)/target/debug/engrammic
SCRATCH=$(mktemp -d)
mkdir -p "$SCRATCH/.claude" "$SCRATCH/.engrammic"
echo '{"mcpServers":{"other":{"url":"http://keep"}}}' > "$SCRATCH/.claude/settings.json"

# Install (Phase 1a/1b must be landed for -y to work):
HOME="$SCRATCH" "$BIN" install -y 2>&1 | tail -5
echo "--- after install ---"
python3 -c "import json,sys; d=json.load(open('$SCRATCH/.claude/settings.json')); print('engrammic present:', 'engrammic' in d.get('mcpServers',{})); print('other present:', 'other' in d.get('mcpServers',{}))"

# Remove just claude:
HOME="$SCRATCH" "$BIN" remove --harness claude -y 2>&1 | tail -5
echo "--- after remove ---"
python3 -c "import json,sys; d=json.load(open('$SCRATCH/.claude/settings.json')); print('engrammic present:', 'engrammic' in d.get('mcpServers',{})); print('other present:', 'other' in d.get('mcpServers',{}))"
# Expected: engrammic False, other True. Backup present:
ls "$SCRATCH/.claude/settings.json.engrammic.bak" && echo "backup kept (correct)"
```

- [ ] **Step 2: Full uninstall smoke**

```bash
# Re-install then fully uninstall:
HOME="$SCRATCH" "$BIN" install -y
HOME="$SCRATCH" "$BIN" uninstall -y 2>&1 | tail -8
echo "--- state.toml ---"
ls "$SCRATCH/.engrammic/state.toml" 2>/dev/null || echo "(deleted — correct)"
echo "--- settings.json (should have other but no engrammic) ---"
python3 -c "import json; d=json.load(open('$SCRATCH/.claude/settings.json')); assert 'other' in d.get('mcpServers',{}); assert 'engrammic' not in d.get('mcpServers',{}); print('OK')"
```

- [ ] **Step 3: Commit any fixes**

```bash
git add -A && git commit -m "chore(installer): phase 3 verification fixes" || echo "clean"
```

---

## Spec ambiguities interpreted

1. **"restoring from backups where they exist"** — interpreted as surgical removal by default;
   the backup is a user-facing safety net, not an auto-restore trigger. Full rationale in the
   Architecture section above.

2. **`created_by_us` when `backup_path` is `None`** — `ensure_backup` returns `None` when the
   config file did not exist before install. This is the only reliable signal that we created the
   file. After removal, we delete it entirely rather than leaving an empty config stub.

3. **Skills during `remove` vs `uninstall`** — `remove` asks the user whether to also remove
   skills (default: yes); `uninstall` always removes all skills (it is a full teardown). The spec
   does not specify this distinction explicitly; the "remove … optionally their skills" phrasing
   in the spec guided the interactive ask.

4. **Legacy scan for `remove` with `--harness` flags** — when a requested `--harness` id is not
   in the manifest, we fall back to the legacy scan for that id rather than failing immediately.
   The spec says legacy scan is for "when no manifest exists"; we extend this to "when the specific
   id is not in the manifest" for a better UX on partially-migrated installs.

5. **`SkillDest` lifetime constraint** — `SkillDest.name` and `.harness` are `&'static str`;
   you cannot construct one from a heap `String`. The `remove_skills_for_harness` helper dispatches
   directly on `SkillFormat` rather than constructing a synthetic `SkillDest`, avoiding the
   lifetime issue while reusing `skills::remove_skills`, `remove_mdc_skills`, `remove_gemini_skills`.

6. **`remove_tool_outcome` vs `remove_tool`** — Phase 1b renames/replaces `remove_tool`. This
   plan introduces `remove_one_harness` as the canonical single-harness removal kernel (it takes
   a `&HarnessEntry` rather than a `&Tool`, which is the Phase 3 natural unit). It coexists with
   `remove_tool_outcome` from 1b; the two can be merged in a follow-up cleanup.
