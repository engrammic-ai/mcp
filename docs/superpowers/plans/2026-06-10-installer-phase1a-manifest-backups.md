# Installer Phase 1a: Manifest + Backup-on-Write Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an install manifest (`~/.engrammic/state.toml`) that records every mutation the installer makes, with backup-on-write for harness config files and migration from the legacy `config.toml` — the foundation for reversible installs and a working uninstall.

**Architecture:** New `manifest.rs` module owns the manifest schema (versioned, atomic writes). `user_config.rs` becomes a thin delegation layer over the manifest so all existing call sites keep working unchanged. `config.rs` gains `ensure_backup` (creates `<path>.engrammic.bak` before first mutation). `main.rs` wiring is surgical: record harness/skill entries at existing install/remove call sites — the full interview→plan→execute refactor is Phase 1b, NOT this plan.

**Tech Stack:** Rust, serde + toml (already deps), tempfile (already a dev-dep). Crate root: `installer-cli/`. All commands below run from `installer-cli/`.

**Spec:** `docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md`

---

### Task 1: Manifest schema + atomic load/save

**Files:**
- Create: `installer-cli/src/manifest.rs`
- Modify: `installer-cli/src/main.rs:15` (add `mod manifest;` after `mod logs;` — keep mods alphabetical)

All functions take an explicit base directory (`*_in(dir)`) so tests never touch the real `$HOME`; thin no-arg wrappers use the real dir.

- [ ] **Step 1: Write the failing tests**

Create `installer-cli/src/manifest.rs` with the test module only (types come in step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_default() {
        let dir = tempdir().unwrap();
        let m = Manifest::load_in(dir.path()).unwrap();
        assert_eq!(m.schema_version, SCHEMA_VERSION);
        assert!(m.harnesses.is_empty());
        assert!(m.skills.is_empty());
        assert!(m.endpoint.is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let mut m = Manifest::load_in(dir.path()).unwrap();
        m.endpoint = Some("https://beta.engrammic.ai/mcp/".into());
        m.harnesses.push(HarnessEntry {
            tool_id: "cursor".into(),
            config_path: "/tmp/mcp.json".into(),
            backup_path: Some("/tmp/mcp.json.engrammic.bak".into()),
            endpoint: "https://beta.engrammic.ai/mcp/".into(),
        });
        m.skills.push(SkillEntry {
            harness: "claude".into(),
            path: "/tmp/skills".into(),
            format: "directory".into(),
            scope: "user".into(),
        });
        m.save_in(dir.path()).unwrap();

        let loaded = Manifest::load_in(dir.path()).unwrap();
        assert_eq!(loaded.endpoint.as_deref(), Some("https://beta.engrammic.ai/mcp/"));
        assert_eq!(loaded.harnesses.len(), 1);
        assert_eq!(loaded.harnesses[0].tool_id, "cursor");
        assert_eq!(loaded.skills[0].format, "directory");
    }

    #[test]
    fn save_is_atomic_no_tmp_left_behind() {
        let dir = tempdir().unwrap();
        let m = Manifest::default();
        m.save_in(dir.path()).unwrap();
        assert!(dir.path().join("state.toml").exists());
        assert!(!dir.path().join("state.toml.tmp").exists());
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(
            dir.path().join("state.toml"),
            "schema_version = 1\nfuture_field = \"x\"\n",
        )
        .unwrap();
        let m = Manifest::load_in(dir.path()).unwrap();
        assert_eq!(m.schema_version, 1);
    }
}
```

- [ ] **Step 2: Add `mod manifest;` to main.rs, run tests to verify they fail**

Run: `cargo test --bin engrammic manifest 2>&1 | head -20`
Expected: COMPILE ERROR (`Manifest` not found) — that's the failing state for Rust TDD.

- [ ] **Step 3: Write the implementation (above the test module)**

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

/// One harness (editor) whose config file we edited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEntry {
    pub tool_id: String,
    pub config_path: PathBuf,
    /// Backup created before our first mutation; None if the file didn't exist.
    pub backup_path: Option<PathBuf>,
    pub endpoint: String,
}

/// One skill destination we installed into.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub harness: String,
    pub path: PathBuf,
    /// "directory" | "cursor-mdc" | "gemini-md" | "agents-md" — drives removal strategy.
    pub format: String,
    /// "user" | "project"
    pub scope: String,
}

/// Single source of truth for everything the installer has done on this machine.
/// Stored at ~/.engrammic/state.toml; written atomically (tmp + rename).
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub license_key: Option<String>,
    #[serde(default)]
    pub selfhost_dir: Option<PathBuf>,
    #[serde(default)]
    pub binary_path: Option<PathBuf>,
    #[serde(default)]
    pub harnesses: Vec<HarnessEntry>,
    #[serde(default)]
    pub skills: Vec<SkillEntry>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            endpoint: None,
            license_key: None,
            selfhost_dir: None,
            binary_path: None,
            harnesses: Vec::new(),
            skills: Vec::new(),
        }
    }
}

impl Manifest {
    pub fn dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".engrammic")
    }

    pub fn path_in(dir: &Path) -> PathBuf {
        dir.join("state.toml")
    }

    pub fn load() -> Result<Self> {
        Self::load_in(&Self::dir())
    }

    pub fn load_in(dir: &Path) -> Result<Self> {
        let path = Self::path_in(dir);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        self.save_in(&Self::dir())
    }

    pub fn save_in(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
        let path = Self::path_in(dir);
        let tmp = dir.join("state.toml.tmp");
        let content = toml::to_string_pretty(self).context("failed to serialize manifest")?;
        fs::write(&tmp, content)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        fs::rename(&tmp, &path)
            .with_context(|| format!("failed to move manifest into place at {}", path.display()))
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin engrammic manifest`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/manifest.rs installer-cli/src/main.rs
git commit -m "feat(installer): add versioned install manifest with atomic writes"
```

---

### Task 2: Migration from legacy config.toml

**Files:**
- Modify: `installer-cli/src/manifest.rs` (add `load_or_migrate_in` + tests)

Behavior: if `state.toml` is missing but `config.toml` exists, import its three fields (`endpoint`, `license_key`, `selfhost_dir`), save `state.toml`, and **leave `config.toml` in place** (it is deleted in Phase 1b once nothing reads it). If both exist, `state.toml` wins.

- [ ] **Step 1: Write the failing tests** (append inside `mod tests`)

```rust
    #[test]
    fn migrates_legacy_config_toml() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "endpoint = \"http://localhost:8000/mcp\"\nlicense_key = \"eng_abc\"\nselfhost_dir = \"/opt/engrammic\"\n",
        )
        .unwrap();

        let m = Manifest::load_or_migrate_in(dir.path()).unwrap();
        assert_eq!(m.endpoint.as_deref(), Some("http://localhost:8000/mcp"));
        assert_eq!(m.license_key.as_deref(), Some("eng_abc"));
        assert_eq!(m.selfhost_dir.as_deref(), Some(std::path::Path::new("/opt/engrammic")));
        // state.toml persisted, config.toml untouched (removed in Phase 1b)
        assert!(dir.path().join("state.toml").exists());
        assert!(dir.path().join("config.toml").exists());
    }

    #[test]
    fn state_toml_wins_over_legacy() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), "endpoint = \"http://old\"\n").unwrap();
        let mut m = Manifest::default();
        m.endpoint = Some("http://new".into());
        m.save_in(dir.path()).unwrap();

        let loaded = Manifest::load_or_migrate_in(dir.path()).unwrap();
        assert_eq!(loaded.endpoint.as_deref(), Some("http://new"));
    }

    #[test]
    fn no_legacy_no_state_yields_default_without_writing() {
        let dir = tempdir().unwrap();
        let m = Manifest::load_or_migrate_in(dir.path()).unwrap();
        assert!(m.endpoint.is_none());
        assert!(!dir.path().join("state.toml").exists());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --bin engrammic manifest 2>&1 | head -10`
Expected: COMPILE ERROR (`load_or_migrate_in` not found).

- [ ] **Step 3: Implement** (in `impl Manifest`)

```rust
    /// Load the manifest, synthesizing one from the legacy config.toml on first run.
    /// The legacy file is left in place until Phase 1b removes its last readers.
    pub fn load_or_migrate(dir_override: Option<&Path>) -> Result<Self> {
        match dir_override {
            Some(d) => Self::load_or_migrate_in(d),
            None => Self::load_or_migrate_in(&Self::dir()),
        }
    }

    pub fn load_or_migrate_in(dir: &Path) -> Result<Self> {
        if Self::path_in(dir).exists() {
            return Self::load_in(dir);
        }
        let legacy = dir.join("config.toml");
        if !legacy.exists() {
            return Ok(Self::default());
        }

        #[derive(Deserialize, Default)]
        struct Legacy {
            #[serde(default)]
            endpoint: Option<String>,
            #[serde(default)]
            license_key: Option<String>,
            #[serde(default)]
            selfhost_dir: Option<PathBuf>,
        }
        let content = fs::read_to_string(&legacy)
            .with_context(|| format!("failed to read {}", legacy.display()))?;
        let old: Legacy = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", legacy.display()))?;

        let manifest = Self {
            endpoint: old.endpoint,
            license_key: old.license_key,
            selfhost_dir: old.selfhost_dir,
            ..Self::default()
        };
        manifest.save_in(dir)?;
        Ok(manifest)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin engrammic manifest`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/manifest.rs
git commit -m "feat(installer): migrate legacy config.toml into manifest on first load"
```

---

### Task 3: UserConfig delegates to the manifest

**Files:**
- Modify: `installer-cli/src/user_config.rs` (full rewrite — it is 49 lines)

Every existing call site (`UserConfig::load`, `::save`, `::dir`, `::path`, field access) keeps compiling unchanged; storage just moves to `state.toml`. `save()` merges the three fields into the manifest rather than overwriting harness/skill entries.

- [ ] **Step 1: Write the failing test**

Append to `installer-cli/src/user_config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use tempfile::tempdir;

    #[test]
    fn save_in_writes_through_to_manifest_preserving_entries() {
        let dir = tempdir().unwrap();
        // Pre-existing manifest with a harness entry that must survive.
        let mut m = Manifest::default();
        m.harnesses.push(crate::manifest::HarnessEntry {
            tool_id: "cursor".into(),
            config_path: "/tmp/mcp.json".into(),
            backup_path: None,
            endpoint: "http://e".into(),
        });
        m.save_in(dir.path()).unwrap();

        let cfg = UserConfig {
            endpoint: Some("http://new".into()),
            license_key: Some("eng_k".into()),
            selfhost_dir: None,
        };
        cfg.save_in(dir.path()).unwrap();

        let m = Manifest::load_in(dir.path()).unwrap();
        assert_eq!(m.endpoint.as_deref(), Some("http://new"));
        assert_eq!(m.license_key.as_deref(), Some("eng_k"));
        assert_eq!(m.harnesses.len(), 1, "save() must not clobber manifest entries");

        let loaded = UserConfig::load_in(dir.path()).unwrap();
        assert_eq!(loaded.endpoint.as_deref(), Some("http://new"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --bin engrammic user_config 2>&1 | head -10`
Expected: COMPILE ERROR (`save_in`/`load_in` not found).

- [ ] **Step 3: Rewrite user_config.rs as a delegation layer**

```rust
use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::manifest::Manifest;

/// Thin view over the manifest for the three legacy fields.
/// Kept so existing call sites compile unchanged; storage lives in state.toml.
#[derive(Debug, Default)]
pub struct UserConfig {
    pub endpoint: Option<String>,
    pub license_key: Option<String>,
    pub selfhost_dir: Option<PathBuf>,
}

impl UserConfig {
    pub fn dir() -> PathBuf {
        Manifest::dir()
    }

    /// Path shown to users in messages; now the manifest file.
    pub fn path() -> PathBuf {
        Manifest::path_in(&Self::dir())
    }

    pub fn load() -> Result<Self> {
        Self::load_in(&Self::dir())
    }

    pub fn load_in(dir: &Path) -> Result<Self> {
        let m = Manifest::load_or_migrate_in(dir)?;
        Ok(Self {
            endpoint: m.endpoint,
            license_key: m.license_key,
            selfhost_dir: m.selfhost_dir,
        })
    }

    pub fn save(&self) -> Result<()> {
        self.save_in(&Self::dir())
    }

    /// Merge these fields into the manifest without touching harness/skill entries.
    pub fn save_in(&self, dir: &Path) -> Result<()> {
        let mut m = Manifest::load_or_migrate_in(dir)?;
        m.endpoint = self.endpoint.clone();
        m.license_key = self.license_key.clone();
        m.selfhost_dir = self.selfhost_dir.clone();
        m.save_in(dir)
    }
}
```

Note: the old struct derived `Serialize, Deserialize`; the new one doesn't need them. If any call site fails to compile because of that (check with the build), re-add the derives — do not change call sites in this task.

- [ ] **Step 4: Build the whole crate and run all tests**

Run: `cargo build 2>&1 | tail -5 && cargo test 2>&1 | tail -5`
Expected: clean build; all tests pass. If main.rs/selfhost.rs call sites broke, fix ONLY by re-adding derives or adjusting the view struct — never by editing call-site logic in this task.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/user_config.rs
git commit -m "refactor(installer): UserConfig delegates storage to the manifest"
```

---

### Task 4: Backup-on-write in config.rs

**Files:**
- Modify: `installer-cli/src/config.rs` (add `ensure_backup` + tests at end of file)

Contract: called before the first mutation of a harness config. Creates `<path>.engrammic.bak` **once** (never overwritten — it must capture pre-Engrammic state, not the latest state). Returns `Ok(None)` when the config file doesn't exist yet (nothing to back up — created files are reverted by deletion).

- [ ] **Step 1: Write the failing tests** (append to config.rs)

```rust
#[cfg(test)]
mod backup_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_bak_once_and_never_overwrites() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("mcp.json");
        std::fs::write(&cfg, "{\"original\": true}").unwrap();

        let bak = ensure_backup(&cfg).unwrap().expect("backup path");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), "{\"original\": true}");

        // Mutate the config, call again: backup must keep the ORIGINAL content.
        std::fs::write(&cfg, "{\"mutated\": true}").unwrap();
        let bak2 = ensure_backup(&cfg).unwrap().expect("backup path");
        assert_eq!(bak, bak2);
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), "{\"original\": true}");
    }

    #[test]
    fn missing_config_yields_no_backup() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("does-not-exist.json");
        assert!(ensure_backup(&cfg).unwrap().is_none());
        assert!(!dir.path().join("does-not-exist.json.engrammic.bak").exists());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --bin engrammic backup 2>&1 | head -10`
Expected: COMPILE ERROR (`ensure_backup` not found).

- [ ] **Step 3: Implement** (in config.rs, after the public dispatchers)

```rust
/// Create `<path>.engrammic.bak` before our first mutation of a harness config.
/// Idempotent: an existing backup is never overwritten, so it always preserves
/// the pre-Engrammic state. Returns None when there is nothing to back up.
pub fn ensure_backup(config_path: &Path) -> Result<Option<std::path::PathBuf>> {
    if !config_path.exists() {
        return Ok(None);
    }
    let mut bak = config_path.as_os_str().to_owned();
    bak.push(".engrammic.bak");
    let bak = std::path::PathBuf::from(bak);
    if !bak.exists() {
        fs::copy(config_path, &bak).with_context(|| {
            format!("failed to back up {} to {}", config_path.display(), bak.display())
        })?;
    }
    Ok(Some(bak))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin engrammic backup`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add installer-cli/src/config.rs
git commit -m "feat(installer): backup harness configs before first mutation"
```

---

### Task 5: Wire manifest recording into the install/remove paths

**Files:**
- Modify: `installer-cli/src/main.rs` — `install_tool` (≈line 615), `remove_tool` (≈line 554), the skills-install result handling in `run_full_install`/`install_skills_step` (the `skills::install_skills(...)` call sites)
- Modify: `installer-cli/src/manifest.rs` (two small recording helpers + tests)

Surgical wiring only — no flow changes. The recording helpers live on `Manifest` so main.rs stays thin.

- [ ] **Step 1: Write the failing tests for the helpers** (append to manifest.rs tests)

```rust
    #[test]
    fn record_harness_upserts_by_tool_id() {
        let mut m = Manifest::default();
        m.record_harness("cursor", Path::new("/tmp/a.json"), None, "http://e1");
        m.record_harness("cursor", Path::new("/tmp/a.json"), Some(PathBuf::from("/tmp/a.json.engrammic.bak")), "http://e2");
        assert_eq!(m.harnesses.len(), 1);
        assert_eq!(m.harnesses[0].endpoint, "http://e2");
        assert!(m.harnesses[0].backup_path.is_some(), "backup path must not be lost on upsert");
        m.forget_harness("cursor");
        assert!(m.harnesses.is_empty());
    }

    #[test]
    fn record_skill_upserts_by_path() {
        let mut m = Manifest::default();
        m.record_skill("claude", Path::new("/tmp/skills"), "directory", "user");
        m.record_skill("claude", Path::new("/tmp/skills"), "directory", "user");
        assert_eq!(m.skills.len(), 1);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --bin engrammic manifest 2>&1 | head -10`
Expected: COMPILE ERROR (`record_harness` not found).

- [ ] **Step 3: Implement the helpers** (in `impl Manifest`)

```rust
    /// Upsert a harness entry. An existing backup_path is preserved when the
    /// caller passes None (backups are created once, on first mutation).
    pub fn record_harness(
        &mut self,
        tool_id: &str,
        config_path: &Path,
        backup_path: Option<PathBuf>,
        endpoint: &str,
    ) {
        if let Some(e) = self.harnesses.iter_mut().find(|e| e.tool_id == tool_id) {
            e.config_path = config_path.to_path_buf();
            e.endpoint = endpoint.to_string();
            if backup_path.is_some() {
                e.backup_path = backup_path;
            }
        } else {
            self.harnesses.push(HarnessEntry {
                tool_id: tool_id.to_string(),
                config_path: config_path.to_path_buf(),
                backup_path,
                endpoint: endpoint.to_string(),
            });
        }
    }

    pub fn forget_harness(&mut self, tool_id: &str) {
        self.harnesses.retain(|e| e.tool_id != tool_id);
    }

    pub fn record_skill(&mut self, harness: &str, path: &Path, format: &str, scope: &str) {
        if !self.skills.iter().any(|s| s.path == path) {
            self.skills.push(SkillEntry {
                harness: harness.to_string(),
                path: path.to_path_buf(),
                format: format.to_string(),
                scope: scope.to_string(),
            });
        }
    }

    pub fn forget_skill(&mut self, path: &Path) {
        self.skills.retain(|s| s.path != path);
    }
```

Also add a free function mapping `tools::SkillFormat`/`SkillScope` to manifest strings (manifest.rs, below the impl):

```rust
pub fn skill_format_str(f: crate::tools::SkillFormat) -> &'static str {
    match f {
        crate::tools::SkillFormat::Directory => "directory",
        crate::tools::SkillFormat::CursorMdc => "cursor-mdc",
        crate::tools::SkillFormat::GeminiMd => "gemini-md",
        crate::tools::SkillFormat::AgentsMd => "agents-md",
    }
}

pub fn skill_scope_str(s: crate::tools::SkillScope) -> &'static str {
    match s {
        crate::tools::SkillScope::User => "user",
        crate::tools::SkillScope::Project => "project",
    }
}
```

Run: `cargo test --bin engrammic manifest` — expected: all pass.

- [ ] **Step 4: Wire main.rs — harness installs**

In `install_tool` (≈line 615), for the `InstallMethod::FileEdit(shape)` arm, call `ensure_backup` BEFORE `config::install`, then record:

```rust
        InstallMethod::FileEdit(shape) => {
            let backup = config::ensure_backup(&tool.config_path)?;
            let result = config::install(&tool.config_path, endpoint, shape)?;
            let mut m = manifest::Manifest::load_or_migrate(None)?;
            m.record_harness(tool.id, &tool.config_path, backup, endpoint);
            m.save()?;
            match result {
                // ... existing match arms unchanged ...
```

(`tool.id` is `&'static str` on `Tool` — verify the field name in tools.rs:124 and adjust if it differs.)

Apply the same backup+record pattern to the refresh path in `run_update` (≈line 493-496, the `config::install(&tool.config_path, &ep, shape)` call): backup first, record with the refreshed endpoint `ep`.

- [ ] **Step 5: Wire main.rs — removals and skills**

In `remove_tool` (≈line 554), `InstallMethod::FileEdit` arm, after successful `config::uninstall`:

```rust
            config::uninstall(&tool.config_path, shape)?;
            let mut m = manifest::Manifest::load_or_migrate(None)?;
            m.forget_harness(tool.id);
            m.save()?;
```

At every `skills::install_skills(&dests)` call site in main.rs (grep: `skills::install_skills(` — lines ≈538, 1080, 1166), after the call succeeds, record each dest:

```rust
            let mut m = manifest::Manifest::load_or_migrate(None)?;
            for dest in &dests_with_skills {  // use the local Vec<SkillDest> name at each site
                m.record_skill(
                    dest.harness,
                    &dest.path,
                    manifest::skill_format_str(dest.format),
                    manifest::skill_scope_str(dest.scope),
                );
            }
            m.save()?;
```

In the `uninstall` flow (≈line 598), after `remove_skills_formatted(&dest)` returns `removed > 0`, call `m.forget_skill(&dest.path)` (load the manifest once before the loop, save once after).

Custom-path installs (`skills::install_skills_to_paths`) are NOT recorded — they're an explicit power-user escape hatch with no harness/format info; leave them out (note this in a code comment at one such site is NOT needed — just skip them).

- [ ] **Step 6: Build, test, and smoke-check**

Run: `cargo build && cargo test 2>&1 | tail -5`
Expected: clean build, all tests pass.

Smoke (uses a scratch HOME so your real configs are untouched):
Run: `HOME=$(mktemp -d) cargo run -q -- status 2>&1 | head -20`
Expected: runs without panic (output will show nothing installed).

- [ ] **Step 7: Commit**

```bash
git add installer-cli/src/main.rs installer-cli/src/manifest.rs
git commit -m "feat(installer): record harness and skill installs in the manifest"
```

---

### Task 6: Full verification pass

**Files:** none new.

- [ ] **Step 1: Run the full suite + lints**

Run: `cargo test 2>&1 | tail -5 && cargo clippy -- -D warnings 2>&1 | tail -5 && cargo fmt --check`
Expected: tests pass, no clippy warnings, no fmt diffs. Fix anything that surfaces (clippy fixes must not change behavior).

- [ ] **Step 2: End-to-end manifest sanity in a scratch HOME**

```bash
SCRATCH=$(mktemp -d)
mkdir -p "$SCRATCH/.cursor"
echo '{"mcpServers":{"other":{"url":"http://keep"}}}' > "$SCRATCH/.cursor/mcp.json"
HOME="$SCRATCH" cargo run -q -- install -y
cat "$SCRATCH/.engrammic/state.toml"
ls "$SCRATCH/.cursor/"
```

Expected: `state.toml` exists with `schema_version = 1`, a `[[harnesses]]` entry for cursor, and `mcp.json.engrammic.bak` exists containing the pre-install content (`other` server only). If `-y` exits early for unrelated reasons (e.g., network fetch of skills), that's acceptable for this check as long as the harness entry + backup exist.

- [ ] **Step 3: Commit any verification fixes**

```bash
git add -A && git commit -m "chore(installer): phase 1a verification fixes" || echo "nothing to fix"
```
