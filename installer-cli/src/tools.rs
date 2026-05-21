use std::path::PathBuf;

pub const ENDPOINT: &str = "https://beta.engrammic.ai/mcp/";

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
                config_path: home.join(".windsurf/mcp.json"),
            },
            Tool {
                name: "Antigravity",
                id: "antigravity",
                config_path: home.join(".antigravity/mcp.json"),
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
