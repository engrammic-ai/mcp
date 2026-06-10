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
        assert_eq!(
            m.harnesses.len(),
            1,
            "save() must not clobber manifest entries"
        );

        let loaded = UserConfig::load_in(dir.path()).unwrap();
        assert_eq!(loaded.endpoint.as_deref(), Some("http://new"));
    }
}
