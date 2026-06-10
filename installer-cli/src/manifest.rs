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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        self.save_in(&Self::dir())
    }

    pub fn save_in(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
        let path = Self::path_in(dir);
        let tmp = dir.join(format!("state.toml.{}.tmp", std::process::id()));
        let content = toml::to_string_pretty(self).context("failed to serialize manifest")?;
        fs::write(&tmp, content)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        if let Err(e) = fs::rename(&tmp, &path) {
            let _ = fs::remove_file(&tmp);
            return Err(e).with_context(|| {
                format!("failed to move manifest into place at {}", path.display())
            });
        }
        Ok(())
    }

    /// Load the manifest, synthesizing one from the legacy config.toml on first run.
    /// The legacy file is left in place until Phase 1b removes its last readers.
    #[allow(dead_code)]
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
}

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
        let leftover_tmp = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().is_some_and(|ext| ext == "tmp"));
        assert!(!leftover_tmp, "no .tmp file may remain after save");
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
}
