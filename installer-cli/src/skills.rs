use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use tar::Archive;

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

const SKILLS_TARBALL_URL: &str =
    "https://github.com/engrammic-ai/skills/archive/refs/heads/main.tar.gz";

pub fn download_skills_tarball() -> Result<Vec<u8>> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build();
    let resp = agent
        .get(SKILLS_TARBALL_URL)
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

    let tmp = std::env::temp_dir()
        .join(format!("engrammic-skills-unpack-{}", std::process::id()));
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
    #[ignore = "hits the network; run with --ignored"]
    fn download_skills_tarball_returns_gzip() {
        let bytes = download_skills_tarball().unwrap();
        // gzip magic number
        assert_eq!(&bytes[0..2], &[0x1f, 0x8b]);
        assert!(bytes.len() > 1000);
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
}
