use std::path::PathBuf;

pub const CLOUD_ENDPOINT: &str = "https://beta.engrammic.ai/mcp/";
pub const LOCAL_ENDPOINT: &str = "http://localhost:8000/mcp";

#[derive(Clone)]
pub struct Tool {
    pub name: &'static str,
    pub id: &'static str,
    pub config_path: PathBuf,
}

impl Tool {
    pub fn all() -> Vec<Tool> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        vec![
            Tool {
                name: "Claude Code",
                id: "claude",
                config_path: home.join(".claude/settings.json"),
            },
            Tool {
                name: "Cursor",
                id: "cursor",
                config_path: home.join(".cursor/mcp.json"),
            },
            Tool {
                name: "Windsurf",
                id: "windsurf",
                config_path: home.join(".codeium/windsurf/mcp_config.json"),
            },
            Tool {
                name: "Antigravity",
                id: "antigravity",
                config_path: home.join(".gemini/antigravity/mcp_config.json"),
            },
            Tool {
                name: "Gemini CLI",
                id: "gemini",
                config_path: home.join(".gemini/settings.json"),
            },
            Tool {
                name: "Pi Agents",
                id: "pi",
                config_path: home.join(".pi/agent/mcp.json"),
            },
        ]
    }

    pub fn detect_installed() -> Vec<Tool> {
        Self::all()
            .into_iter()
            .filter(|tool| {
                tool.config_path
                    .parent()
                    .map(|p| p.exists())
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn from_id(id: &str) -> Option<Tool> {
        Self::all().into_iter().find(|tool| tool.id == id)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SkillScope {
    User,    // Global, user-level
    Project, // Project-local
}

#[derive(Clone)]
pub struct SkillDest {
    pub name: &'static str,
    pub harness: &'static str,
    pub path: PathBuf,
    pub scope: SkillScope,
    pub default: bool,
    pub note: Option<&'static str>,
}

impl SkillDest {
    pub fn all() -> Vec<SkillDest> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));

        let claude_present = home.join(".claude").exists();
        let pi_present = home.join(".pi/agent").exists();

        vec![
            // === User-level (global) destinations ===
            SkillDest {
                name: "Claude Code",
                harness: "claude",
                path: home.join(".claude/skills"),
                scope: SkillScope::User,
                default: claude_present,
                note: None,
            },
            SkillDest {
                name: "Pi Agents",
                harness: "pi",
                path: home.join(".pi/agent/skills"),
                scope: SkillScope::User,
                default: pi_present && !claude_present,
                note: None,
            },
            // === Project-level destinations ===
            SkillDest {
                name: "Windsurf (project)",
                harness: "windsurf",
                path: PathBuf::from(".windsurf/skills"),
                scope: SkillScope::Project,
                default: false,
                note: None,
            },
            SkillDest {
                name: "Antigravity (project)",
                harness: "antigravity",
                path: PathBuf::from(".agent/skills"),
                scope: SkillScope::Project,
                default: false,
                note: None,
            },
            SkillDest {
                name: "Cross-harness (project)",
                harness: "cross",
                path: PathBuf::from(".agents/skills"),
                scope: SkillScope::Project,
                default: false,
                note: Some("Works with Pi Agents, Claude Code"),
            },
        ]
    }

    pub fn user_level() -> Vec<SkillDest> {
        Self::all()
            .into_iter()
            .filter(|d| d.scope == SkillScope::User)
            .collect()
    }

    pub fn project_level() -> Vec<SkillDest> {
        Self::all()
            .into_iter()
            .filter(|d| d.scope == SkillScope::Project)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_dest_has_user_and_project_levels() {
        let user = SkillDest::user_level();
        let project = SkillDest::project_level();

        assert!(!user.is_empty());
        assert!(!project.is_empty());
        assert!(user.iter().all(|d| d.scope == SkillScope::User));
        assert!(project.iter().all(|d| d.scope == SkillScope::Project));
    }

    #[test]
    fn claude_skills_path_correct() {
        let dests = SkillDest::all();
        let claude = dests.iter().find(|d| d.harness == "claude").unwrap();
        assert!(claude.path.ends_with(".claude/skills"));
    }

    #[test]
    fn project_dests_are_relative() {
        let project = SkillDest::project_level();
        for dest in project {
            assert!(dest.path.is_relative(), "{} should be relative", dest.name);
        }
    }
}
