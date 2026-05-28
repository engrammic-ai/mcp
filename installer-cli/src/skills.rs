use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use tar::Archive;

use crate::skill_format::{merge_into_gemini_md, remove_from_gemini_md, to_cursor_mdc, SkillEntry};
use crate::tools::{SkillDest, SkillFormat};

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

/// Count skills in a directory of `engrammic-*.mdc` files.
pub fn count_mdc_skills(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.starts_with(SKILL_PREFIX) && s.ends_with(".mdc")
        })
        .count()
}

/// Return 1 if the file contains Engrammic markers, 0 otherwise.
pub fn count_gemini_skills(file: &Path) -> usize {
    let Ok(content) = fs::read_to_string(file) else {
        return 0;
    };
    if content.contains("<!-- ENGRAMMIC:START -->") {
        1
    } else {
        0
    }
}

/// Count installed skills, dispatching on the destination format.
pub fn count_skills_formatted(dest: &SkillDest) -> usize {
    match dest.format {
        SkillFormat::Directory => count_skills(&dest.path),
        SkillFormat::CursorMdc => count_mdc_skills(&dest.path),
        SkillFormat::GeminiMd => count_gemini_skills(&dest.path),
    }
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

/// Install skills into a single destination, dispatching on format.
pub fn install_skills_formatted(src: &Path, dest: &SkillDest) -> Result<usize> {
    match dest.format {
        SkillFormat::Directory => copy_skills(src, &dest.path),
        SkillFormat::CursorMdc => copy_skills_as_mdc(src, &dest.path),
        SkillFormat::GeminiMd => merge_skills_to_gemini(src, &dest.path),
    }
}

/// Copy each `engrammic-<name>/SKILL.md` as `engrammic-<name>.mdc` into dest_dir.
pub fn copy_skills_as_mdc(src: &Path, dest_dir: &Path) -> Result<usize> {
    fs::create_dir_all(dest_dir)
        .with_context(|| format!("failed to create {}", dest_dir.display()))?;
    let mut count = 0;
    for entry in fs::read_dir(src)
        .with_context(|| format!("failed to read {}", src.display()))?
    {
        let entry = entry?;
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_string_lossy();
        if !dir_name_str.starts_with(SKILL_PREFIX) || !entry.path().is_dir() {
            continue;
        }
        let skill_md = entry.path().join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let content = fs::read_to_string(&skill_md)
            .with_context(|| format!("failed to read {}", skill_md.display()))?;
        let mdc = to_cursor_mdc(&content);
        let out_name = format!("{}.mdc", dir_name_str);
        let out_path = dest_dir.join(&out_name);
        fs::write(&out_path, mdc)
            .with_context(|| format!("failed to write {}", out_path.display()))?;
        count += 1;
    }
    Ok(count)
}

/// Collect all skills from src and merge them into the single dest_file (GEMINI.md).
pub fn merge_skills_to_gemini(src: &Path, dest_file: &Path) -> Result<usize> {
    if let Some(parent) = dest_file.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }

    let mut entries: Vec<SkillEntry> = Vec::new();
    for entry in fs::read_dir(src)
        .with_context(|| format!("failed to read {}", src.display()))?
    {
        let entry = entry?;
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_string_lossy();
        if !dir_name_str.starts_with(SKILL_PREFIX) || !entry.path().is_dir() {
            continue;
        }
        let skill_md = entry.path().join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let content = fs::read_to_string(&skill_md)
            .with_context(|| format!("failed to read {}", skill_md.display()))?;
        let meta = crate::skill_format::parse_skill_metadata(&content);
        let body = crate::skill_format::extract_body(&content).to_string();
        // Skill name: strip "engrammic-" prefix from directory name.
        let name = dir_name_str
            .strip_prefix(SKILL_PREFIX)
            .unwrap_or(&dir_name_str)
            .to_string();
        entries.push(SkillEntry {
            name,
            description: meta.description,
            body,
        });
    }

    let count = entries.len();
    let existing = if dest_file.exists() {
        fs::read_to_string(dest_file)
            .with_context(|| format!("failed to read {}", dest_file.display()))?
    } else {
        String::new()
    };
    let merged = merge_into_gemini_md(&existing, &entries);
    fs::write(dest_file, merged)
        .with_context(|| format!("failed to write {}", dest_file.display()))?;
    Ok(count)
}

/// Remove skills from a destination, dispatching on format.
pub fn remove_skills_formatted(dest: &SkillDest) -> Result<usize> {
    match dest.format {
        SkillFormat::Directory => remove_skills(&dest.path),
        SkillFormat::CursorMdc => remove_mdc_skills(&dest.path),
        SkillFormat::GeminiMd => remove_gemini_skills(&dest.path),
    }
}

/// Remove all `engrammic-*.mdc` files from dir.
pub fn remove_mdc_skills(dir: &Path) -> Result<usize> {
    let mut count = 0;
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(0);
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(SKILL_PREFIX) && name_str.ends_with(".mdc") {
            fs::remove_file(entry.path())?;
            count += 1;
        }
    }
    Ok(count)
}

/// Remove the Engrammic section from a GEMINI.md file.
/// Returns 1 if markers were found and removed, 0 otherwise.
pub fn remove_gemini_skills(file: &Path) -> Result<usize> {
    if !file.exists() {
        return Ok(0);
    }
    let content = fs::read_to_string(file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let cleaned = remove_from_gemini_md(&content);
    if cleaned == content {
        return Ok(0);
    }
    fs::write(file, cleaned)
        .with_context(|| format!("failed to write {}", file.display()))?;
    Ok(1)
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

/// Downloads, unpacks, and installs skills into each destination.
/// Dispatches based on the destination format.
/// Returns one (destination path, skill count) pair per destination.
pub fn install_skills(dests: &[SkillDest]) -> Result<Vec<(PathBuf, usize)>> {
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
        let count = install_skills_formatted(&src, dest)?;
        results.push((dest.path.clone(), count));
    }

    fs::remove_dir_all(&tmp).ok();
    Ok(results)
}

/// Downloads, unpacks, and copies skills into raw path destinations using Directory format.
///
/// This is a compatibility shim for callsites that only have a PathBuf (e.g. the
/// `--skill-path` CLI flag which has no format information).
pub fn install_skills_to_paths(dests: &[PathBuf]) -> Result<Vec<(PathBuf, usize)>> {
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
        .join(format!("engrammic-skills-unpack-paths-{}", std::process::id()));
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

    // ---- count_mdc_skills ----

    #[test]
    fn count_mdc_skills_counts_only_engrammic_mdc_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("engrammic-recall.mdc"), "mdc").unwrap();
        fs::write(dir.path().join("engrammic-learn.mdc"), "mdc").unwrap();
        fs::write(dir.path().join("other-rule.mdc"), "mdc").unwrap();
        fs::write(dir.path().join("engrammic-keep.txt"), "txt").unwrap();
        assert_eq!(count_mdc_skills(dir.path()), 2);
    }

    #[test]
    fn count_mdc_skills_on_missing_dir_is_zero() {
        assert_eq!(count_mdc_skills(std::path::Path::new("/no/such/dir")), 0);
    }

    // ---- count_gemini_skills ----

    #[test]
    fn count_gemini_skills_returns_one_when_markers_present() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        fs::write(&file, "# Rules\n<!-- ENGRAMMIC:START -->\ncontent\n<!-- ENGRAMMIC:END -->\n").unwrap();
        assert_eq!(count_gemini_skills(&file), 1);
    }

    #[test]
    fn count_gemini_skills_returns_zero_when_no_markers() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        fs::write(&file, "# Rules\nUser content only.").unwrap();
        assert_eq!(count_gemini_skills(&file), 0);
    }

    #[test]
    fn count_gemini_skills_on_missing_file_is_zero() {
        assert_eq!(count_gemini_skills(std::path::Path::new("/no/such/GEMINI.md")), 0);
    }

    // ---- count_skills_formatted ----

    #[test]
    fn count_skills_formatted_dispatches_directory() {
        use crate::tools::{SkillDest, SkillFormat, SkillScope};
        let dir = tempdir().unwrap();
        make_skill(dir.path(), "engrammic-recall");
        make_skill(dir.path(), "engrammic-learn");
        let dest = SkillDest {
            name: "test",
            harness: "test",
            path: dir.path().to_path_buf(),
            format: SkillFormat::Directory,
            default: false,
            scope: SkillScope::User,
            note: None,
        };
        assert_eq!(count_skills_formatted(&dest), 2);
    }

    #[test]
    fn count_skills_formatted_dispatches_cursor_mdc() {
        use crate::tools::{SkillDest, SkillFormat, SkillScope};
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("engrammic-recall.mdc"), "mdc").unwrap();
        fs::write(dir.path().join("other.mdc"), "other").unwrap();
        let dest = SkillDest {
            name: "test",
            harness: "test",
            path: dir.path().to_path_buf(),
            format: SkillFormat::CursorMdc,
            default: false,
            scope: SkillScope::User,
            note: None,
        };
        assert_eq!(count_skills_formatted(&dest), 1);
    }

    #[test]
    fn count_skills_formatted_dispatches_gemini_md() {
        use crate::tools::{SkillDest, SkillFormat, SkillScope};
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        fs::write(&file, "<!-- ENGRAMMIC:START -->\ncontent\n<!-- ENGRAMMIC:END -->").unwrap();
        let dest = SkillDest {
            name: "test",
            harness: "test",
            path: file,
            format: SkillFormat::GeminiMd,
            default: false,
            scope: SkillScope::User,
            note: None,
        };
        assert_eq!(count_skills_formatted(&dest), 1);
    }

    #[test]
    fn count_skills_formatted_gemini_md_no_markers_is_zero() {
        use crate::tools::{SkillDest, SkillFormat, SkillScope};
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        fs::write(&file, "# Rules\nNo engrammic content.").unwrap();
        let dest = SkillDest {
            name: "test",
            harness: "test",
            path: file,
            format: SkillFormat::GeminiMd,
            default: false,
            scope: SkillScope::User,
            note: None,
        };
        assert_eq!(count_skills_formatted(&dest), 0);
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

    fn make_skill_with_content(root: &std::path::Path, dir_name: &str, content: &str) {
        let dir = root.join(dir_name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    // ---- copy_skills_as_mdc ----

    #[test]
    fn copy_skills_as_mdc_creates_mdc_files() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Search memory\n---\n\nBody.",
        );
        make_skill_with_content(
            src.path(),
            "engrammic-learn",
            "---\nname: learn\ndescription: Store knowledge\n---\n\nBody.",
        );

        let count = copy_skills_as_mdc(src.path(), dest.path()).unwrap();
        assert_eq!(count, 2);
        assert!(dest.path().join("engrammic-recall.mdc").exists());
        assert!(dest.path().join("engrammic-learn.mdc").exists());
    }

    #[test]
    fn copy_skills_as_mdc_skips_non_prefixed_dirs() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Desc\n---\n\nBody.",
        );
        make_skill_with_content(src.path(), "unrelated-tool", "---\nname: x\n---\n\nBody.");
        fs::write(src.path().join("README.md"), "x").unwrap();

        let count = copy_skills_as_mdc(src.path(), dest.path()).unwrap();
        assert_eq!(count, 1);
        assert!(!dest.path().join("unrelated-tool.mdc").exists());
        assert!(!dest.path().join("README.md.mdc").exists());
    }

    #[test]
    fn copy_skills_as_mdc_mdc_content_has_cursor_frontmatter() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Search memory\n---\n\nBody here.",
        );
        copy_skills_as_mdc(src.path(), dest.path()).unwrap();
        let mdc = fs::read_to_string(dest.path().join("engrammic-recall.mdc")).unwrap();
        assert!(mdc.contains("description: Search memory"));
        assert!(mdc.contains("globs: "));
        assert!(mdc.contains("Body here."));
    }

    #[test]
    fn copy_skills_as_mdc_roundtrip_install_remove() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Desc\n---\n\nBody.",
        );
        // Install
        let count = copy_skills_as_mdc(src.path(), dest.path()).unwrap();
        assert_eq!(count, 1);
        assert!(dest.path().join("engrammic-recall.mdc").exists());
        // Add unrelated .mdc that should be preserved
        fs::write(dest.path().join("other-rule.mdc"), "unrelated").unwrap();
        // Remove
        let removed = remove_mdc_skills(dest.path()).unwrap();
        assert_eq!(removed, 1);
        assert!(!dest.path().join("engrammic-recall.mdc").exists());
        assert!(dest.path().join("other-rule.mdc").exists());
    }

    // ---- merge_skills_to_gemini ----

    #[test]
    fn merge_skills_to_gemini_creates_file() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        let dest_file = dest.path().join("GEMINI.md");
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Search memory\n---\n\nBody.",
        );

        let count = merge_skills_to_gemini(src.path(), &dest_file).unwrap();
        assert_eq!(count, 1);
        assert!(dest_file.exists());
        let content = fs::read_to_string(&dest_file).unwrap();
        assert!(content.contains("<!-- ENGRAMMIC:START -->"));
        assert!(content.contains("<!-- ENGRAMMIC:END -->"));
        assert!(content.contains("## engrammic-recall"));
    }

    #[test]
    fn merge_skills_to_gemini_creates_parent_dir() {
        let base = tempdir().unwrap();
        let src = tempdir().unwrap();
        let nested = base.path().join("subdir").join("deep");
        let dest_file = nested.join("GEMINI.md");
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Desc\n---\n\nBody.",
        );

        let count = merge_skills_to_gemini(src.path(), &dest_file).unwrap();
        assert_eq!(count, 1);
        assert!(dest_file.exists());
    }

    #[test]
    fn merge_skills_to_gemini_preserves_existing_content() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        let dest_file = dest.path().join("GEMINI.md");
        fs::write(&dest_file, "# My Rules\nUser content.").unwrap();
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Desc\n---\n\nBody.",
        );

        merge_skills_to_gemini(src.path(), &dest_file).unwrap();
        let content = fs::read_to_string(&dest_file).unwrap();
        assert!(content.contains("# My Rules"));
        assert!(content.contains("User content."));
        assert!(content.contains("## engrammic-recall"));
    }

    #[test]
    fn merge_skills_to_gemini_is_idempotent() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        let dest_file = dest.path().join("GEMINI.md");
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Desc\n---\n\nBody.",
        );

        merge_skills_to_gemini(src.path(), &dest_file).unwrap();
        let first = fs::read_to_string(&dest_file).unwrap();
        merge_skills_to_gemini(src.path(), &dest_file).unwrap();
        let second = fs::read_to_string(&dest_file).unwrap();
        assert_eq!(first, second);
    }

    // ---- remove_mdc_skills ----

    #[test]
    fn remove_mdc_skills_removes_only_engrammic_mdc() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("engrammic-recall.mdc"), "mdc").unwrap();
        fs::write(dir.path().join("engrammic-learn.mdc"), "mdc").unwrap();
        fs::write(dir.path().join("other-rule.mdc"), "keep").unwrap();
        fs::write(dir.path().join("engrammic-keep.txt"), "keep").unwrap();

        let removed = remove_mdc_skills(dir.path()).unwrap();
        assert_eq!(removed, 2);
        assert!(!dir.path().join("engrammic-recall.mdc").exists());
        assert!(!dir.path().join("engrammic-learn.mdc").exists());
        assert!(dir.path().join("other-rule.mdc").exists());
        assert!(dir.path().join("engrammic-keep.txt").exists());
    }

    #[test]
    fn remove_mdc_skills_missing_dir_returns_zero() {
        let removed = remove_mdc_skills(std::path::Path::new("/no/such/dir")).unwrap();
        assert_eq!(removed, 0);
    }

    // ---- remove_gemini_skills ----

    #[test]
    fn remove_gemini_skills_removes_engrammic_section() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        let content = "# Rules\nUser content.\n\n<!-- ENGRAMMIC:START -->\n## engrammic-recall\nBody.\n<!-- ENGRAMMIC:END -->\n";
        fs::write(&file, content).unwrap();

        let removed = remove_gemini_skills(&file).unwrap();
        assert_eq!(removed, 1);
        let after = fs::read_to_string(&file).unwrap();
        assert!(!after.contains("<!-- ENGRAMMIC:START -->"));
        assert!(after.contains("User content."));
    }

    #[test]
    fn remove_gemini_skills_no_markers_returns_zero() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("GEMINI.md");
        fs::write(&file, "# Rules\nUser content.").unwrap();

        let removed = remove_gemini_skills(&file).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn remove_gemini_skills_missing_file_returns_zero() {
        let removed = remove_gemini_skills(std::path::Path::new("/no/such/GEMINI.md")).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn gemini_roundtrip_install_remove() {
        let src = tempdir().unwrap();
        let dest = tempdir().unwrap();
        let dest_file = dest.path().join("GEMINI.md");
        let original = "# My Rules\nUser content.";
        fs::write(&dest_file, original).unwrap();
        make_skill_with_content(
            src.path(),
            "engrammic-recall",
            "---\nname: recall\ndescription: Desc\n---\n\nBody.",
        );

        merge_skills_to_gemini(src.path(), &dest_file).unwrap();
        let after_install = fs::read_to_string(&dest_file).unwrap();
        assert!(after_install.contains("<!-- ENGRAMMIC:START -->"));

        let removed = remove_gemini_skills(&dest_file).unwrap();
        assert_eq!(removed, 1);
        let after_remove = fs::read_to_string(&dest_file).unwrap();
        assert_eq!(after_remove, original);
    }
}
