/// Format conversion logic for multi-format skill installation.
///
/// Handles:
/// - Frontmatter parsing from SKILL.md files
/// - Conversion to Cursor .mdc format
/// - Gemini GEMINI.md section management (merge + remove)

const MARKER_START: &str = "<!-- ENGRAMMIC:START -->";
const MARKER_END: &str = "<!-- ENGRAMMIC:END -->";

/// Metadata extracted from a SKILL.md frontmatter block.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
}

/// Parse name and description from YAML frontmatter.
///
/// Frontmatter is delimited by `---` lines at the start of the file.
/// Missing fields default to empty strings. If there is no frontmatter,
/// both fields are empty.
pub fn parse_skill_metadata(content: &str) -> SkillMetadata {
    let mut name = String::new();
    let mut description = String::new();

    let Some(rest) = content.strip_prefix("---") else {
        return SkillMetadata { name, description };
    };
    // Skip optional newline after opening ---
    let rest = rest.strip_prefix('\n').unwrap_or(rest);

    // Find closing ---
    let end = rest.find("\n---").unwrap_or(rest.len());
    let front = &rest[..end];

    for line in front.lines() {
        if let Some(val) = line.strip_prefix("name:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("description:") {
            description = val.trim().to_string();
        }
    }

    SkillMetadata { name, description }
}

/// Return the content after the frontmatter block.
///
/// If no frontmatter is present, returns the full content.
pub fn extract_body(content: &str) -> &str {
    let Some(rest) = content.strip_prefix("---") else {
        return content;
    };
    let rest = rest.strip_prefix('\n').unwrap_or(rest);

    // Find closing ---
    if let Some(end) = rest.find("\n---") {
        let after_close = &rest[end + 4..]; // skip \n---
                                            // Skip trailing newlines after closing --- (handles blank line after frontmatter)
        after_close.trim_start_matches('\n')
    } else {
        // Malformed: opening --- but no closing ---; treat full content as body.
        content
    }
}

/// Convert a SKILL.md file's content to Cursor `.mdc` format.
///
/// Cursor .mdc files use frontmatter with `description` and `globs` (empty).
/// The body follows unchanged.
pub fn to_cursor_mdc(skill_content: &str) -> String {
    let meta = parse_skill_metadata(skill_content);
    let body = extract_body(skill_content);
    format!(
        "---\ndescription: {}\nglobs: \n---\n\n{}",
        meta.description, body
    )
}

/// A skill's name and body for Gemini section building.
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub body: String,
}

/// Build the marked Gemini section from a list of skill entries.
///
/// Each skill is rendered as `## engrammic-<name>` followed by its body.
pub fn build_gemini_section(skills: &[SkillEntry]) -> String {
    let mut out = String::new();
    out.push_str(MARKER_START);
    out.push('\n');

    for skill in skills {
        out.push_str(&format!("## engrammic-{}\n", skill.name));
        if !skill.description.is_empty() {
            out.push_str(&skill.description);
            out.push('\n');
        }
        let body = skill.body.trim();
        if !body.is_empty() {
            out.push('\n');
            out.push_str(body);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str(MARKER_END);
    out
}

/// Merge skills into an existing GEMINI.md content string.
///
/// - If markers already exist, replace the content between them.
/// - If no markers exist, append the marked section at the end.
///
/// User content outside markers is always preserved.
pub fn merge_into_gemini_md(existing: &str, skills: &[SkillEntry]) -> String {
    let section = build_gemini_section(skills);

    if let Some(start) = existing.find(MARKER_START) {
        // Replace existing marked section.
        let end = existing.find(MARKER_END);
        let after_end = end
            .map(|pos| pos + MARKER_END.len())
            .unwrap_or(existing.len());

        let before = &existing[..start];
        let after = &existing[after_end..];

        // Strip a single trailing newline from before to avoid double-blank-lines.
        let before = before.trim_end_matches('\n');
        // Strip a single leading newline from after.
        let after = if after.starts_with('\n') {
            &after[1..]
        } else {
            after
        };

        if before.is_empty() && after.is_empty() {
            return section;
        }
        if before.is_empty() {
            return format!("{}\n{}", section, after);
        }
        if after.is_empty() {
            return format!("{}\n\n{}", before, section);
        }
        format!("{}\n\n{}\n{}", before, section, after)
    } else {
        // Append.
        let existing = existing.trim_end_matches('\n');
        if existing.is_empty() {
            return section;
        }
        format!("{}\n\n{}", existing, section)
    }
}

/// Remove the Engrammic marked section from GEMINI.md content.
///
/// Returns the content with the section (inclusive of markers) removed.
/// Extra blank lines at the join point are collapsed to at most one blank line.
/// If no markers are present, returns the content unchanged.
pub fn remove_from_gemini_md(content: &str) -> String {
    let Some(start) = content.find(MARKER_START) else {
        return content.to_string();
    };

    let end = content.find(MARKER_END);
    let after_end = end
        .map(|pos| pos + MARKER_END.len())
        .unwrap_or(content.len());

    let before = &content[..start];
    let after = &content[after_end..];

    // Trim trailing whitespace/newlines from before and leading from after,
    // then join with a single newline if both parts have content.
    let before = before.trim_end_matches('\n');
    let after = after.trim_start_matches('\n');

    match (before.is_empty(), after.is_empty()) {
        (true, true) => String::new(),
        (true, false) => after.to_string(),
        (false, true) => before.to_string(),
        (false, false) => format!("{}\n\n{}", before, after),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_skill_metadata ----

    #[test]
    fn parses_name_and_description() {
        let content = "---\nname: recall\ndescription: Search memory\n---\n\nBody here.";
        let meta = parse_skill_metadata(content);
        assert_eq!(meta.name, "recall");
        assert_eq!(meta.description, "Search memory");
    }

    #[test]
    fn parses_name_only_description_empty() {
        let content = "---\nname: learn\n---\n\nBody.";
        let meta = parse_skill_metadata(content);
        assert_eq!(meta.name, "learn");
        assert_eq!(meta.description, "");
    }

    #[test]
    fn no_frontmatter_returns_empty_fields() {
        let content = "Just some content without frontmatter.";
        let meta = parse_skill_metadata(content);
        assert_eq!(meta.name, "");
        assert_eq!(meta.description, "");
    }

    #[test]
    fn description_with_colon_in_value() {
        let content = "---\nname: x\ndescription: Foo: bar baz\n---\n\n";
        let meta = parse_skill_metadata(content);
        assert_eq!(meta.description, "Foo: bar baz");
    }

    // ---- extract_body ----

    #[test]
    fn extract_body_strips_frontmatter() {
        let content = "---\nname: recall\n---\n\nThe actual body.";
        assert_eq!(extract_body(content), "The actual body.");
    }

    #[test]
    fn extract_body_no_frontmatter_returns_full() {
        let content = "No frontmatter here.";
        assert_eq!(extract_body(content), content);
    }

    #[test]
    fn extract_body_multiline_body() {
        let content = "---\nname: x\n---\n\nLine one.\nLine two.\n";
        assert_eq!(extract_body(content), "Line one.\nLine two.\n");
    }

    // ---- to_cursor_mdc ----

    #[test]
    fn cursor_mdc_has_correct_frontmatter() {
        let skill = "---\nname: recall\ndescription: Search memory for relevant context\n---\n\nBody content here.";
        let mdc = to_cursor_mdc(skill);
        assert!(mdc.starts_with("---\n"));
        assert!(mdc.contains("description: Search memory for relevant context\n"));
        assert!(mdc.contains("globs: \n"));
        assert!(!mdc.contains("name:"));
    }

    #[test]
    fn cursor_mdc_preserves_body() {
        let skill = "---\nname: recall\ndescription: Desc\n---\n\nBody content here.";
        let mdc = to_cursor_mdc(skill);
        assert!(mdc.contains("Body content here."));
    }

    #[test]
    fn cursor_mdc_empty_description() {
        let skill = "---\nname: recall\n---\n\nBody.";
        let mdc = to_cursor_mdc(skill);
        assert!(mdc.contains("description: \n"));
        assert!(mdc.contains("globs: \n"));
    }

    // ---- build_gemini_section ----

    fn make_entry(name: &str, description: &str, body: &str) -> SkillEntry {
        SkillEntry {
            name: name.to_string(),
            description: description.to_string(),
            body: body.to_string(),
        }
    }

    #[test]
    fn gemini_section_has_markers() {
        let skills = vec![make_entry("recall", "Search memory", "Body.")];
        let section = build_gemini_section(&skills);
        assert!(section.starts_with(MARKER_START));
        assert!(section.ends_with(MARKER_END));
    }

    #[test]
    fn gemini_section_includes_skill_headers() {
        let skills = vec![
            make_entry("recall", "Search memory", "Do recall."),
            make_entry("learn", "Store knowledge", "Do learn."),
        ];
        let section = build_gemini_section(&skills);
        assert!(section.contains("## engrammic-recall\n"));
        assert!(section.contains("## engrammic-learn\n"));
    }

    #[test]
    fn gemini_section_empty_skills() {
        let section = build_gemini_section(&[]);
        assert_eq!(section, format!("{}\n{}", MARKER_START, MARKER_END));
    }

    // ---- merge_into_gemini_md ----

    #[test]
    fn merge_appends_to_empty_file() {
        let skills = vec![make_entry("recall", "Desc", "Body.")];
        let result = merge_into_gemini_md("", &skills);
        assert!(result.starts_with(MARKER_START));
        assert!(result.ends_with(MARKER_END));
    }

    #[test]
    fn merge_appends_after_existing_content() {
        let existing = "# My Rules\nSome user content.";
        let skills = vec![make_entry("recall", "Desc", "Body.")];
        let result = merge_into_gemini_md(existing, &skills);
        assert!(result.starts_with("# My Rules"));
        assert!(result.contains(MARKER_START));
        assert!(result.ends_with(MARKER_END));
    }

    #[test]
    fn merge_replaces_existing_markers() {
        let existing = format!(
            "# Rules\n\n{}\n## engrammic-old\nOld body.\n{}\n",
            MARKER_START, MARKER_END
        );
        let skills = vec![make_entry("recall", "Desc", "New body.")];
        let result = merge_into_gemini_md(&existing, &skills);
        assert!(result.contains("## engrammic-recall"));
        assert!(!result.contains("## engrammic-old"));
        // Should not duplicate markers
        assert_eq!(result.matches(MARKER_START).count(), 1);
        assert_eq!(result.matches(MARKER_END).count(), 1);
    }

    #[test]
    fn merge_is_idempotent() {
        let skills = vec![make_entry("recall", "Desc", "Body.")];
        let first = merge_into_gemini_md("# Rules\nContent.", &skills);
        let second = merge_into_gemini_md(&first, &skills);
        assert_eq!(first, second);
    }

    // ---- remove_from_gemini_md ----

    #[test]
    fn remove_no_markers_returns_unchanged() {
        let content = "# My Rules\nUser content.";
        assert_eq!(remove_from_gemini_md(content), content);
    }

    #[test]
    fn remove_strips_marked_section() {
        let content = format!(
            "# Rules\nUser content.\n\n{}\n## engrammic-recall\nBody.\n{}\n",
            MARKER_START, MARKER_END
        );
        let result = remove_from_gemini_md(&content);
        assert!(!result.contains(MARKER_START));
        assert!(!result.contains(MARKER_END));
        assert!(result.contains("User content."));
    }

    #[test]
    fn remove_preserves_content_after_markers() {
        let content = format!(
            "{}\n## engrammic-recall\nBody.\n{}\n\n# After section\nMore user content.",
            MARKER_START, MARKER_END
        );
        let result = remove_from_gemini_md(&content);
        assert!(!result.contains(MARKER_START));
        assert!(result.contains("# After section"));
        assert!(result.contains("More user content."));
    }

    #[test]
    fn remove_only_marked_section_returns_empty() {
        let content = format!(
            "{}\n## engrammic-recall\nBody.\n{}",
            MARKER_START, MARKER_END
        );
        let result = remove_from_gemini_md(&content);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_then_merge_roundtrip() {
        let original = "# My Rules\nUser content.";
        let skills = vec![make_entry("recall", "Desc", "Body.")];
        let merged = merge_into_gemini_md(original, &skills);
        let removed = remove_from_gemini_md(&merged);
        assert_eq!(removed, original);
    }
}
