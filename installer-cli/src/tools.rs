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

#[derive(Clone)]
pub struct SkillDest {
    pub name: &'static str,
    pub path: PathBuf,
    pub default: bool,
}

impl SkillDest {
    pub fn all() -> Vec<SkillDest> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let claude = home.join(".claude/skills");
        let claude_present = claude
            .parent()
            .map(|p| p.exists())
            .unwrap_or(false);
        vec![
            SkillDest {
                name: "Claude Code        ~/.claude/skills/",
                path: claude,
                default: claude_present,
            },
            SkillDest {
                name: "Cross-harness      ~/.agents/skills/",
                path: home.join(".agents/skills"),
                default: !claude_present,
            },
            SkillDest {
                name: "Project-local      ./.agents/skills/",
                path: PathBuf::from(".agents/skills"),
                default: false,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_dest_all_returns_three() {
        let dests = SkillDest::all();
        assert_eq!(dests.len(), 3);
        assert!(dests[0].path.ends_with(".claude/skills"));
        assert!(dests[1].path.ends_with(".agents/skills"));
        assert_eq!(dests[2].path, PathBuf::from(".agents/skills"));
    }

    #[test]
    fn project_dest_is_never_default() {
        let dests = SkillDest::all();
        assert!(!dests[2].default);
    }
}
