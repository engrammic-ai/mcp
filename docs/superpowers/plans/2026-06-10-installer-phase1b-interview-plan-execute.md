# Installer Phase 1b: Interview → Plan → Execute Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the install wizard from a linear chain of `?`/`bail!` calls into interview (all questions, no side effects) → plan summary → execute (skip-and-continue, per-step results), consolidate the two self-host flows, un-pre-check selection prompts, add proactive TTY detection, and delete the legacy `config.toml` after migration.

**Architecture:** New `flow.rs` module owns the `Answers` struct (interview output), pure option-building/summary-rendering helpers (unit-testable without a TTY), and the `execute` engine that returns `Vec<StepResult>` instead of aborting on first error. `main.rs` keeps the returning-user menu and command dispatch but delegates fresh installs and `-y` runs to the same interview→execute path (auto mode pre-answers the interview; one flow, two input sources). `engrammic docker`/`run_docker_setup`/`docker::write_compose_bundle` are deleted; `Docker` becomes an alias for `selfhost::run_wizard`, which is also where the fresh-install "Self-hosted" choice now lands.

**Tech Stack:** Rust, dialoguer 0.11, existing manifest module from Phase 1a. Crate root `installer-cli/`; all commands run from there.

**Spec:** `docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md` (Architecture §1–3, Decisions, "Selection prompts" block)

**Sequencing note for the controller:** Tasks 1 is foundational. Tasks 2 and 5 and 6 are mutually independent after Task 1 (different files/regions) and may be parallelized. Task 3 depends on 1+2; Task 4 is independent of everything (main.rs entry only); Task 7 is last.

---

### Task 1: flow.rs — Answers, StepResult, pure helpers

**Files:**
- Create: `installer-cli/src/flow.rs`
- Modify: `installer-cli/src/main.rs:1-16` (add `mod flow;` in alphabetical position: after `mod doctor;`, before `mod license;`)

- [ ] **Step 1: Write the failing tests** (bottom of new `flow.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    fn tool(id: &str) -> Tool {
        Tool::from_id(id).expect("known tool id")
    }

    #[test]
    fn harness_label_marks_detected_and_configured() {
        let t = tool("claude");
        assert_eq!(harness_label(&t, true, false), "Claude Code  (detected)");
        assert_eq!(harness_label(&t, true, true), "Claude Code  (already configured)");
        assert_eq!(harness_label(&t, false, false), "Claude Code");
    }

    #[test]
    fn render_plan_lists_all_decisions() {
        let answers = Answers {
            endpoint: "https://beta.engrammic.ai/mcp/".to_string(),
            to_install: vec![tool("claude"), tool("windsurf")],
            to_remove: vec![],
            skill_dests: vec![],
        };
        let plan = render_plan(&answers);
        assert!(plan.contains("Claude Code"));
        assert!(plan.contains("Windsurf"));
        assert!(plan.contains("beta.engrammic.ai"));
        assert!(plan.contains("no skills"), "empty skill dests must be stated, not omitted");
    }

    #[test]
    fn summarize_results_counts_outcomes() {
        let results = vec![
            StepResult { label: "Claude Code".into(), outcome: Outcome::Done },
            StepResult { label: "Windsurf".into(), outcome: Outcome::Failed("permission denied".into()) },
            StepResult { label: "Cursor".into(), outcome: Outcome::Manual("open this link".into()) },
        ];
        let (ok, failed, manual) = summarize_results(&results);
        assert_eq!((ok, failed, manual), (1, 1, 1));
    }
}
```

- [ ] **Step 2: Add `mod flow;` to main.rs, run `cargo test --bin engrammic flow` — expect compile error**

- [ ] **Step 3: Implement** (top of `flow.rs`)

```rust
//! Interview → plan → execute flow types and pure helpers.
//!
//! Interview functions ask everything up front with zero side effects; the
//! execute engine runs steps skip-and-continue and reports per-step results.
//! Pure helpers live here so they are unit-testable without a TTY.

use crate::tools::{SkillDest, Tool};

/// Everything the wizard needs to know, collected before any mutation.
#[derive(Debug)]
pub struct Answers {
    pub endpoint: String,
    pub to_install: Vec<Tool>,
    pub to_remove: Vec<Tool>,
    pub skill_dests: Vec<SkillDest>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Done,
    Failed(String),
    /// Needs a user action we cannot perform (deep-link approval, GUI config).
    Manual(String),
}

#[derive(Debug)]
pub struct StepResult {
    pub label: String,
    pub outcome: Outcome,
}

/// Display label for a harness in the selection prompt. Detection is shown as
/// information, never as a pre-checked default (spec: "nothing is pre-checked").
pub fn harness_label(tool: &Tool, detected: bool, configured: bool) -> String {
    if configured {
        format!("{}  (already configured)", tool.name)
    } else if detected {
        format!("{}  (detected)", tool.name)
    } else {
        tool.name.to_string()
    }
}

/// Human-readable recap of everything about to happen, shown before execution.
pub fn render_plan(answers: &Answers) -> String {
    let mut out = String::from("About to:\n");
    if answers.to_install.is_empty() {
        out.push_str("  • configure no editors\n");
    } else {
        let names: Vec<&str> = answers.to_install.iter().map(|t| t.name).collect();
        out.push_str(&format!("  • configure: {}\n", names.join(", ")));
    }
    for t in &answers.to_remove {
        out.push_str(&format!("  • remove Engrammic from: {}\n", t.name));
    }
    if answers.skill_dests.is_empty() {
        out.push_str("  • install no skills\n");
    } else {
        let names: Vec<&str> = answers.skill_dests.iter().map(|d| d.name).collect();
        out.push_str(&format!("  • install skills to: {}\n", names.join(", ")));
    }
    out.push_str(&format!("  • endpoint: {}\n", answers.endpoint));
    out
}

/// (done, failed, manual) counts for the final summary line.
pub fn summarize_results(results: &[StepResult]) -> (usize, usize, usize) {
    let mut done = 0;
    let mut failed = 0;
    let mut manual = 0;
    for r in results {
        match r.outcome {
            Outcome::Done => done += 1,
            Outcome::Failed(_) => failed += 1,
            Outcome::Manual(_) => manual += 1,
        }
    }
    (done, failed, manual)
}
```

Check `Tool::from_id` exists (tools.rs:317) and `Tool` derives Clone (it does — `all_tools.clone()` compiles in main.rs today). If `Tool` is not `Debug`, drop the `#[derive(Debug)]` on `Answers` rather than touching tools.rs.

- [ ] **Step 4: `cargo test --bin engrammic flow` — expect 3 passed**

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/flow.rs installer-cli/src/main.rs
git commit -m "feat(installer): add flow module with Answers, StepResult, plan rendering"
```

---

### Task 2: Un-pre-check selection prompts; label detected/configured

**Files:**
- Modify: `installer-cli/src/main.rs` — `select_tools` (≈line 842-928), `install_skills_step` (≈line 1061-1150), `install_skills_only` (≈line 1152+, its MultiSelect mirrors install_skills_step)

Spec rule: detected/configured items are **labeled, never pre-checked**; confirming zero selections is valid and prints how to do it later; `-y` keeps detection-driven selection (unchanged).

- [ ] **Step 1: Change `select_tools` interactive branch** (≈lines 884-907). Replace the options/defaults construction:

```rust
    // Interactive mode: label detected/configured tools; nothing pre-checked.
    let all_tools = Tool::all();
    let detected_ids: std::collections::HashSet<_> = detected.iter().map(|t| t.id).collect();
    let options: Vec<String> = all_tools
        .iter()
        .map(|t| {
            flow::harness_label(t, detected_ids.contains(t.id), installed_ids.contains(t.id))
        })
        .collect();

    println!(
        "  {}",
        "(↑↓ move · space toggle · enter confirm (deselect a configured one to remove))".dimmed()
    );
    let selection_indices = MultiSelect::new()
        .with_prompt("Select editors to configure")
        .items(&options)
        .interact()?;

    let selected: std::collections::HashSet<usize> = selection_indices.into_iter().collect();

    let to_install: Vec<Tool> = all_tools
        .iter()
        .enumerate()
        .filter(|(i, _)| selected.contains(i))
        .map(|(_, t)| t.clone())
        .collect();

    // Tools to remove: were installed, but now deselected
    let to_remove: Vec<Tool> = all_tools
        .iter()
        .enumerate()
        .filter(|(i, t)| installed_ids.contains(t.id) && !selected.contains(i))
        .map(|(_, t)| t.clone())
        .collect();
```

(The old code matched selections back by `t.name` string against display labels; with labels now decorated, index-based matching is REQUIRED — name-matching would silently select nothing. This is why the whole block is replaced.)

- [ ] **Step 2: Zero-selection message.** In `run_full_install` (≈line 248-259) the empty-selection branch already prints the manual JSON snippet — extend it with the "later" hint:

```rust
        println!("{} No editors selected — nothing was changed.", "!".yellow());
        println!(
            "  Run {} anytime to configure editors.",
            "engrammic install".cyan()
        );
```

(Keep the existing manual-config JSON print below it.)

- [ ] **Step 3: Skills destinations.** In `install_skills_step` (≈1093-1120) the interactive MultiSelect currently has NO `.defaults(...)` — already un-pre-checked; only add the label decoration: where the option string is built (≈1096-1105), append `(detected)` for dests whose `default` flag is true:

```rust
            .map(|d| {
                let scope = match d.scope {
                    tools::SkillScope::User => "(user)",
                    tools::SkillScope::Project => "(project)",
                };
                let detected = if d.default { "  (detected)" } else { "" };
                format!("{:<25} {}{}", d.name, scope.dimmed(), detected.dimmed())
            })
```

Apply the same decoration to the identical options-building block in `install_skills_only`. If `install_skills_only`'s MultiSelect passes `.defaults(...)` (it pre-checks `d.default` today per the spec's named inconsistency), REMOVE the `.defaults(...)` call there so both prompts behave identically.

Also extend its zero-selection message (≈line 1122-1125) with: `Run engrammic skills anytime to install them.`

- [ ] **Step 4: Build, run full suite, eyeball the prompt**

Run: `cargo test 2>&1 | tail -2 && cargo build 2>&1 | tail -1`
Expected: all pass, clean build.
Manual check (interactive TTY): `cargo run -q -- install` in a scratch HOME — confirm labels render and nothing is pre-checked, then Ctrl+C out:
`SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.claude"; HOME="$SCRATCH" cargo run -q -- install` (controller does this if the worker has no TTY; cosmetic verification only).

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): label detected editors instead of pre-checking selections"
```

---

### Task 3: Interview → plan summary → execute in run_full_install

**Files:**
- Modify: `installer-cli/src/flow.rs` (add `execute_harness` helper)
- Modify: `installer-cli/src/main.rs` — `run_full_install` (≈line 240-308), `install_tool` (≈line 615+), `install_skills_step` signature

The current `run_full_install` interleaves prompts and mutations and aborts the whole run when one harness fails (`install_tool(...)?`). Restructure:

1. **Interview:** `select_tools` + the skills questions move BEFORE any mutation. `install_skills_step` is split: its prompting half (confirm + destination MultiSelect) becomes `ask_skill_dests(auto) -> Result<Vec<SkillDest>>` in main.rs; its acting half (download/install/record) becomes `install_skills_to(dests) -> Result<()>`.
2. **Plan summary:** print `flow::render_plan(&answers)`; in interactive mode gate on `Confirm::new().with_prompt("Proceed?").default(true)`; in `-y` mode print it without pausing.
3. **Execute:** loop steps collecting `StepResult` — a failing harness records `Outcome::Failed(msg)` and CONTINUES. One manifest load before the loop, one save after (final-review recommendation: stop the per-tool load/save churn). Deep-link/GUI harnesses record `Outcome::Manual`.
4. **Summary:** print per-step ✓/✗/▸ lines plus `summarize_results` counts; failed steps print their error and a retry hint (`engrammic install --tool <id>`).

- [ ] **Step 1: Refactor `install_tool` to return an outcome instead of printing-and-erroring.** Change its signature and FileEdit arm to:

```rust
/// Register the engrammic MCP server for one tool. Never returns Err for
/// per-harness problems — those become Outcome::Failed so other steps continue.
fn install_tool(tool: &Tool, endpoint: &str, m: &mut manifest::Manifest) -> flow::Outcome {
    match tool.method {
        InstallMethod::FileEdit(shape) => {
            let backup = match config::ensure_backup(&tool.config_path) {
                Ok(b) => b,
                Err(e) => return flow::Outcome::Failed(format!("backup failed: {e:#}")),
            };
            match config::install(&tool.config_path, endpoint, shape) {
                Ok(_) => {
                    m.record_harness(tool.id, &tool.config_path, backup, endpoint);
                    flow::Outcome::Done
                }
                Err(e) => flow::Outcome::Failed(format!("{e:#}")),
            }
        }
        InstallMethod::DeepLink(_) | InstallMethod::PrintInstructions(_) => {
            // Keep the existing per-kind println! blocks (deep-link open attempt,
            // redirect URL, JSON snippet) EXACTLY as they are today, then:
            flow::Outcome::Manual("requires an in-app step (shown above)".to_string())
        }
    }
}
```

Preserve the existing detailed printing inside the DeepLink/PrintInstructions arms (move the current code into this match unchanged); for FileEdit, the old Created/Updated/Unchanged print moves to the caller's per-step result line (print `✓ <name> (added)` / `(updated)` / `(unchanged)` — keep the same wording as today by returning the InstallResult inside Done if simpler: it is acceptable to print inside `install_tool` as today and return the Outcome solely for tallying).

- [ ] **Step 2: Restructure `run_full_install`:**

```rust
fn run_full_install(
    endpoint: String,
    auto: bool,
    tool_id: Option<&str>,
    skill_path: Option<&str>,
) -> Result<()> {
    // ---- Interview: every question, zero side effects ----
    let selection = select_tools(auto, tool_id)?;

    if selection.to_install.is_empty() && selection.to_remove.is_empty() {
        println!("{} No editors selected — nothing was changed.", "!".yellow());
        println!("  Run {} anytime to configure editors.", "engrammic install".cyan());
        println!();
        println!("Add this to your MCP config manually:");
        println!();
        println!(r#"  "engrammic": {{ "type": "http", "url": "{}" }}"#, endpoint);
        println!();
        return Ok(());
    }

    let skill_dests = ask_skill_dests(auto, skill_path)?;

    let answers = flow::Answers {
        endpoint: endpoint.clone(),
        to_install: selection.to_install,
        to_remove: selection.to_remove,
        skill_dests,
    };

    // ---- Plan summary ----
    println!();
    print!("{}", flow::render_plan(&answers));
    println!();
    if !auto {
        let proceed = Confirm::new()
            .with_prompt("Proceed?")
            .default(true)
            .interact()?;
        if !proceed {
            println!("{}", "Nothing was changed.".dimmed());
            return Ok(());
        }
        println!();
    }

    // ---- Execute: skip-and-continue, one manifest load/save ----
    let mut m = manifest::Manifest::load_or_migrate(None)?;
    let mut results: Vec<flow::StepResult> = Vec::new();

    for tool in &answers.to_remove {
        let outcome = match remove_tool_outcome(tool, &mut m) {
            Ok(o) => o,
            Err(e) => flow::Outcome::Failed(format!("{e:#}")),
        };
        results.push(flow::StepResult { label: format!("remove {}", tool.name), outcome });
    }

    for tool in &answers.to_install {
        let outcome = install_tool(tool, &answers.endpoint, &mut m);
        results.push(flow::StepResult { label: tool.name.to_string(), outcome });
    }

    if !answers.skill_dests.is_empty() {
        let outcome = match install_skills_to(&answers.skill_dests, &mut m, skill_path) {
            Ok(()) => flow::Outcome::Done,
            Err(e) => flow::Outcome::Failed(format!("{e:#}")),
        };
        results.push(flow::StepResult { label: "skills".to_string(), outcome });
    }

    m.save()?;

    // Save endpoint so returning users get the menu (merges via manifest).
    let existing = user_config::UserConfig::load().unwrap_or_default();
    let config = user_config::UserConfig {
        endpoint: Some(endpoint),
        license_key: existing.license_key,
        selfhost_dir: existing.selfhost_dir,
    };
    config.save()?;

    // ---- Summary ----
    println!();
    let (done, failed, manual) = flow::summarize_results(&results);
    for r in &results {
        match &r.outcome {
            flow::Outcome::Done => {}
            flow::Outcome::Failed(msg) => {
                println!("  {} {} — {}", "✗".red(), r.label, msg);
                println!("    {} {}", "→ retry:".dimmed(), format!("engrammic install --tool {}", r.label).cyan());
            }
            flow::Outcome::Manual(msg) => {
                println!("  {} {} — {}", "▸".cyan(), r.label, msg);
            }
        }
    }
    println!(
        "{} {} configured, {} need a manual step, {} failed.",
        if failed == 0 { "✓".green() } else { "!".yellow() },
        done,
        manual,
        failed
    );

    println!();
    println!(
        "Done. Tools available: {}",
        "remember, recall, learn, believe, trace, link".dimmed()
    );
    print_restart_reminder();
    println!();
    cli_install::offer_cli_install(auto)?;
    Ok(())
}
```

Notes:
- `remove_tool_outcome` is `remove_tool` adapted to take `&mut Manifest` and return `Result<flow::Outcome>` (FileEdit → uninstall + `m.forget_harness` + `Outcome::Done`; DeepLink/PrintInstructions → keep prints, `Outcome::Manual(...)`). The retry hint uses `r.label`, which for installs is the tool NAME not id — store the id: change `StepResult.label` usage to `format!("{}", tool.name)` for display but build the retry hint from `tool.id` captured at push time. Simplest correct form: make the retry hint generic (`engrammic install` + select that editor) OR push label as `format!("{} ({})", tool.name, tool.id)`. Implementer picks one and notes it; do not let the hint print a non-existent id.
- `install_skills_to(dests, m, skill_path)` = the acting half of today's `install_skills_step`: custom `skill_path` branch (unrecorded, as today), `skills::install_skills(&dests)`, per-dest `m.record_skill(...)` (no internal manifest load/save — uses the passed `&mut m`).
- `ask_skill_dests(auto, skill_path)` = the prompting half: returns `vec![]` when the user declines or selects nothing; in auto mode returns the `d.default` dests; when `skill_path` is Some, return a one-element marker? No — keep `skill_path` handling inside `install_skills_to` (it bypasses dests), and have `ask_skill_dests` return the normal dest list; if `skill_path.is_some()` skip the prompts and return `vec![]`-with-skills-step-still-running: to keep this unambiguous, when `skill_path` is Some, `ask_skill_dests` returns `Ok(vec![])` AND `run_full_install` adds the skills step regardless: change the condition to `if !answers.skill_dests.is_empty() || skill_path.is_some()`.
- The returning-user menu path (`handle_returning_user` → "Add or update harnesses") still calls `install_tool`/`remove_tool` — update those call sites for the new signatures (load manifest once around its loops, mirroring the execute loop).
- `update` flow (≈line 480-560) also calls these — same mechanical signature update there (it already loads/saves the manifest per call; consolidate to once-per-flow like the execute loop).

- [ ] **Step 3: Full suite + smoke**

Run: `cargo test 2>&1 | tail -2 && cargo build 2>&1 | tail -1`
Smoke (controller): scratch-HOME `install -y` must show the plan recap, per-step results, and end with the summary counts; `state.toml` and `.engrammic.bak` as in Phase 1a.

- [ ] **Step 4: Commit**

```bash
git add installer-cli/src/flow.rs installer-cli/src/main.rs
git commit -m "feat(installer): interview-plan-execute flow with skip-and-continue results"
```

---

### Task 4: Proactive TTY detection

**Files:**
- Modify: `installer-cli/src/main.rs` — `main()` (≈line 30-53)

Today prompts crash first and the error handler pattern-matches the message. Instead, detect before dispatching any interactive command.

- [ ] **Step 1: Add the check before the `match cli.command`:**

```rust
    // Interactive commands need a terminal for prompts (dialoguer reads /dev/tty).
    // Detect up front so users get one clear message instead of a prompt crash.
    let interactive_command = matches!(
        cli.command,
        Commands::Install | Commands::Update | Commands::Uninstall | Commands::Skills
            | Commands::Selfhost | Commands::Docker | Commands::License
    );
    if interactive_command && !auto && !console::user_attended_stderr() {
        eprintln!("{} No interactive terminal detected.", "error:".red().bold());
        eprintln!(
            "  Re-run with {} to auto-configure detected editors:",
            "-y".cyan()
        );
        eprintln!("    {}", "curl -fsSL https://get.engrammic.ai/install.sh | sh -s -- -y".cyan());
        eprintln!("  Or run {} from an interactive terminal.", "engrammic install".cyan());
        std::process::exit(1);
    }
```

`console` (0.15) is already a dependency; `console::user_attended_stderr()` checks stderr-is-a-terminal which matches dialoguer's behavior (dialoguer renders prompts on stderr via console). Verify with: `grep user_attended ~/.cargo/registry/src/*/console-0.15*/src/lib.rs` or simply that it compiles; if the function is named differently in the vendored version, use `console::Term::stderr().is_term()`.

Keep the existing reactive handler in `main()` as a fallback (it still catches mid-flow TTY loss); change nothing else there.

- [ ] **Step 2: Verify both paths**

Run: `cargo build 2>&1 | tail -1`
Then: `target/debug/engrammic install < /dev/null 2>&1 | head -5` from a NON-tty stdin context — controller verifies via: `echo | setsid target/debug/engrammic install` or in practice `target/debug/engrammic install 2>&1 < /dev/null` — expected: the new proactive message, exit code 1, NO dialoguer panic. And `target/debug/engrammic install -y` in a scratch HOME must still run.
(Note: when stderr is still a TTY the check passes and prompts work — that is correct; the broken case is stderr piped/absent.)

- [ ] **Step 3: Commit**

```bash
git add installer-cli/src/main.rs
git commit -m "feat(installer): detect missing TTY before prompting, with -y guidance"
```

---

### Task 5: Self-host consolidation — selfhost wizard is the only path

**Files:**
- Modify: `installer-cli/src/main.rs` — `main()` dispatch (line ≈41), `select_deployment_mode` (≈310-331), DELETE `install_docker` (≈930-1058 region) and `run_docker_setup` (≈364-500 region) and `prompt_for_license` if unused after deletions (check: selfhost.rs has its own `prompt_license`)
- Modify: `installer-cli/src/docker.rs` — DELETE `write_compose_bundle` (line 67) and `COMPOSE_TEMPLATE` IF unused after main.rs deletions (CAUTION: selfhost.rs:834 uses `docker::COMPOSE_TEMPLATE` for Tier::Cloud — keep the template, delete only `write_compose_bundle`)
- Modify: `installer-cli/src/cli.rs:39-41` (Docker subcommand doc comment)

- [ ] **Step 1: Rewire dispatch.** In `main()`: `Commands::Docker => selfhost::run_wizard(),` — and update the cli.rs doc comment to `/// Alias for 'selfhost' (kept for compatibility)`.

- [ ] **Step 2: Rewire fresh-install deployment choice.** `select_deployment_mode` currently calls `run_docker_setup` for Self-hosted. Change the Self-hosted branch to hand off to the wizard and END the install flow there (the wizard configures editors, skills, and CLI itself):

```rust
fn select_deployment_mode(_existing_config: &user_config::UserConfig) -> Result<DeploymentChoice> {
    let modes = vec![
        "Cloud - free tier, no setup (recommended)",
        "Self-hosted - run locally with Docker (license required)",
    ];
    println!("  {}", "(Self-hosted requires Docker and a license key)".dimmed());
    let idx = Select::new()
        .with_prompt("Where should Engrammic run?")
        .items(&modes)
        .default(0)
        .interact()?;
    if idx == 1 {
        Ok(DeploymentChoice::SelfHost)
    } else {
        Ok(DeploymentChoice::Cloud(CLOUD_ENDPOINT.to_string()))
    }
}

enum DeploymentChoice {
    Cloud(String),
    SelfHost,
}
```

And in `install` (≈line 217-238):

```rust
    let endpoint = if auto {
        existing_config
            .endpoint
            .unwrap_or_else(|| CLOUD_ENDPOINT.to_string())
    } else {
        match select_deployment_mode(&existing_config)? {
            DeploymentChoice::Cloud(ep) => ep,
            DeploymentChoice::SelfHost => return selfhost::run_wizard(),
        }
    };
```

Same change at the "Start fresh (reconfigure everything)" arm of `handle_returning_user` (≈line 206-209): it calls `select_deployment_mode` — apply the same match.

NOTE: the UI string previously said "connect to mcp.engrammic.ai" while the constant is beta.engrammic.ai (spec Decision). The new Cloud label above deliberately drops the hostname; do not reintroduce it.

- [ ] **Step 3: Delete dead code.** Remove `install_docker`, `run_docker_setup`, and main.rs's `prompt_for_license` (selfhost.rs has its own). Remove `docker::write_compose_bundle` only if no remaining callers (`grep -rn "write_compose_bundle" src/` must return only docker.rs); keep `COMPOSE_TEMPLATE` (used by selfhost Tier::Cloud). Keep `upgrade_docker`, `manage_license`, scale/logs untouched.

- [ ] **Step 4: Build + suite; grep for orphans**

Run: `cargo build 2>&1 | tail -3 && cargo test 2>&1 | tail -2 && grep -rn "run_docker_setup\|install_docker\|write_compose_bundle" src/ | grep -v "docker.rs"`
Expected: clean build (fix any now-unused imports), tests pass, grep returns nothing outside docker.rs.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/main.rs installer-cli/src/docker.rs installer-cli/src/cli.rs
git commit -m "refactor(installer): consolidate self-host into selfhost wizard, docker becomes alias"
```

---

### Task 6: Delete legacy config.toml after successful migration

**Files:**
- Modify: `installer-cli/src/manifest.rs` — `load_or_migrate_in` (+ test update)

Phase 1a left `config.toml` in place; nothing reads it anymore except the migration itself, so migration can now remove it (spec: "config.toml removed after successful migration").

- [ ] **Step 1: Update the existing test** `migrates_legacy_config_toml`: change the last assertion to

```rust
        assert!(dir.path().join("state.toml").exists());
        assert!(
            !dir.path().join("config.toml").exists(),
            "legacy config.toml is removed after successful migration"
        );
```

Run `cargo test --bin engrammic migrates` — expect FAIL (file still exists).

- [ ] **Step 2: Implement.** In `load_or_migrate_in`, after `manifest.save_in(dir)?;`:

```rust
        // Migration succeeded and is persisted; the legacy file has no readers left.
        let _ = fs::remove_file(&legacy);
```

(Best-effort removal: a failure to delete must not fail the load.)

- [ ] **Step 3: `cargo test --bin engrammic manifest` — all pass. Commit:**

```bash
git add installer-cli/src/manifest.rs
git commit -m "feat(installer): remove legacy config.toml after successful migration"
```

---

### Task 7: Verification pass

- [ ] **Step 1: Full gate**

Run: `cargo test 2>&1 | tail -3 && cargo build 2>&1 | tail -1 && cargo fmt -- src/flow.rs src/main.rs src/manifest.rs src/docker.rs src/cli.rs`
Expected: all tests pass; build clean (no NEW warnings vs the 4 pre-existing ones in license/selfhost/tools).

- [ ] **Step 2: Scratch-HOME end-to-end (controller runs; binary, not cargo, to keep rustup happy)**

```bash
cargo build -q; BIN=$(pwd)/target/debug/engrammic
SCRATCH=$(mktemp -d); mkdir -p "$SCRATCH/.claude"
echo '{"mcpServers":{"other":{"url":"http://keep"}}}' > "$SCRATCH/.claude/settings.json"
# legacy migration + deletion:
mkdir -p "$SCRATCH/.engrammic"; echo 'endpoint = "http://old:8000/mcp"' > "$SCRATCH/.engrammic/config.toml"
cd /tmp && HOME="$SCRATCH" "$BIN" install -y
cat "$SCRATCH/.engrammic/state.toml"; ls "$SCRATCH/.engrammic/"
```

Expected: plan recap printed before execution; per-step summary with counts; `state.toml` has the claude harness + backup + skills; `config.toml` GONE; endpoint in state.toml is `http://old:8000/mcp` (auto mode reuses existing endpoint).
Also: `HOME="$SCRATCH" "$BIN" install < /dev/null` (no `-y`, no tty on stderr when piped — verify via `2>/dev/null`) prints the proactive TTY message with the `sh -s -- -y` hint.

- [ ] **Step 3: Commit any fixes**

```bash
git add -A && git commit -m "chore(installer): phase 1b verification fixes" || echo "clean"
```
