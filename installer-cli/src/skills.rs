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
