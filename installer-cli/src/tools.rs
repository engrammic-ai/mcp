use std::path::PathBuf;

pub const CLOUD_ENDPOINT: &str = "https://beta.engrammic.ai/mcp/";
pub const LOCAL_ENDPOINT: &str = "http://localhost:8000/mcp";

/// The value written into a server entry's `type` field (the transport discriminator),
/// which differs across harnesses. `None` omits the field entirely (url-only shape).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TypeField {
    /// Omit the `type` field entirely (url-only shape, e.g. Zed/Amp/Junie/Cline).
    None,
    /// "http" (Claude/Cursor/Copilot CLI/Amazon Q/Kiro/Windsurf/etc.)
    Http,
    /// "streamable-http" (Roo Code, Continue.dev)
    StreamableHttp,
    /// "remote" (OpenCode)
    Remote,
}

impl TypeField {
    pub fn value(self) -> Option<&'static str> {
        match self {
            TypeField::None => None,
            TypeField::Http => Some("http"),
            TypeField::StreamableHttp => Some("streamable-http"),
            TypeField::Remote => Some("remote"),
        }
    }
}

/// How a harness's MCP config is serialized on disk. Container key, format, and the
/// per-entry layout differ per harness family. Dispatched in `config.rs`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ConfigShape {
    /// JSON object keyed by server name, e.g. `{ "<container>": { "engrammic": { ... } } }`.
    JsonMap {
        container: &'static str,
        type_field: TypeField,
        url_field: &'static str,
    },
    /// Codex TOML: `[mcp_servers.engrammic]` table with a `url` key.
    CodexToml,
    /// Goose YAML: `extensions` map keyed by server name.
    /// Entry: `{ type: streamable_http, name: engrammic, description: "...", uri: <url>,
    ///           enabled: true, timeout: 300 }`.
    GooseYaml,
    /// OpenCode JSON: `mcp` map with entries containing `type: "remote"`, `url`, `enabled: true`.
    OpenCodeJson,
    /// Continue.dev project YAML: top-level `mcpServers` list with `{ name, type, url }` objects.
    ContinueYaml,
}

/// How the installer registers the MCP server for a harness.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InstallMethod {
    /// Write/merge a config file on disk (the only Engrammic entry is touched).
    FileEdit(ConfigShape),
    /// Open an editor deep-link URI; the editor's own UI adds the server.
    DeepLink(DeepLinkKind),
    /// Print copy-paste instructions; the user adds via an in-app GUI panel.
    /// The inner string is the one-line "where to paste" hint shown after the JSON block.
    PrintInstructions(&'static str),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DeepLinkKind {
    /// VS Code: `vscode:mcp/install?<urlencoded-json>` (Copilot).
    VsCode,
    /// Cursor: `cursor://anysphere.cursor-deeplink/mcp/install?...`.
    Cursor,
}

/// The canonical JSON-map shape shared by the original 6 harnesses and Copilot CLI:
/// `{ "mcpServers": { "engrammic": { "type": "http", "url": ... } } }`.
const STANDARD_JSON: ConfigShape = ConfigShape::JsonMap {
    container: "mcpServers",
    type_field: TypeField::Http,
    url_field: "url",
};

/// Amp uses a flat dotted key ("amp.mcpServers") and omits `type`.
const AMP_JSON: ConfigShape = ConfigShape::JsonMap {
    container: "amp.mcpServers",
    type_field: TypeField::None,
    url_field: "url",
};

/// Zed uses `context_servers` and omits `type`.
const ZED_JSON: ConfigShape = ConfigShape::JsonMap {
    container: "context_servers",
    type_field: TypeField::None,
    url_field: "url",
};

/// Junie CLI uses `mcpServers` and omits `type`.
const JUNIE_JSON: ConfigShape = ConfigShape::JsonMap {
    container: "mcpServers",
    type_field: TypeField::None,
    url_field: "url",
};

/// Cline uses `mcpServers` and omits `type`.
const CLINE_JSON: ConfigShape = ConfigShape::JsonMap {
    container: "mcpServers",
    type_field: TypeField::None,
    url_field: "url",
};

/// Roo Code uses `mcpServers` with `streamable-http` type.
const ROO_JSON: ConfigShape = ConfigShape::JsonMap {
    container: "mcpServers",
    type_field: TypeField::StreamableHttp,
    url_field: "url",
};

#[derive(Clone)]
pub struct Tool {
    pub name: &'static str,
    pub id: &'static str,
    /// Config file path (FileEdit) or a presence marker for detection (DeepLink).
    pub config_path: PathBuf,
    pub method: InstallMethod,
}

impl Tool {
    pub fn all() -> Vec<Tool> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        vec![
            Tool {
                name: "Claude Code",
                id: "claude",
                config_path: home.join(".claude/settings.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "Cursor",
                id: "cursor",
                config_path: home.join(".cursor/extensions"), // detection marker only
                method: InstallMethod::DeepLink(DeepLinkKind::Cursor),
            },
            Tool {
                name: "Windsurf",
                id: "windsurf",
                config_path: home.join(".codeium/windsurf/mcp_config.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "Antigravity",
                id: "antigravity",
                config_path: home.join(".gemini/antigravity/mcp_config.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "Gemini CLI",
                id: "gemini",
                config_path: home.join(".gemini/settings.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "Pi Agents",
                id: "pi",
                config_path: home.join(".pi/agent/mcp.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "GitHub Copilot CLI",
                id: "copilot",
                config_path: home.join(".copilot/mcp-config.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "OpenAI Codex CLI",
                id: "codex",
                config_path: home.join(".codex/config.toml"),
                method: InstallMethod::FileEdit(ConfigShape::CodexToml),
            },
            Tool {
                name: "VS Code (Copilot)",
                id: "vscode",
                // No stable per-OS user config path; detect by the presence of the
                // VS Code dir (`~/.vscode`, this path's parent) and register via
                // deep-link. config_path is a detection marker only, never read/written.
                config_path: home.join(".vscode/extensions"),
                method: InstallMethod::DeepLink(DeepLinkKind::VsCode),
            },
            // --- Phase 2: user-level file-edit ---
            Tool {
                name: "Goose",
                id: "goose",
                config_path: home.join(".config/goose/config.yaml"),
                method: InstallMethod::FileEdit(ConfigShape::GooseYaml),
            },
            Tool {
                name: "Sourcegraph Amp",
                id: "amp",
                config_path: home.join(".config/amp/settings.json"),
                method: InstallMethod::FileEdit(AMP_JSON),
            },
            Tool {
                name: "OpenCode",
                id: "opencode",
                config_path: home.join(".config/opencode/opencode.json"),
                method: InstallMethod::FileEdit(ConfigShape::OpenCodeJson),
            },
            Tool {
                name: "Amazon Q",
                id: "amazonq",
                config_path: home.join(".aws/amazonq/mcp.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "Zed",
                id: "zed",
                config_path: home.join(".config/zed/settings.json"),
                method: InstallMethod::FileEdit(ZED_JSON),
            },
            Tool {
                name: "Kiro",
                id: "kiro",
                config_path: home.join(".kiro/settings/mcp.json"),
                method: InstallMethod::FileEdit(STANDARD_JSON),
            },
            Tool {
                name: "JetBrains Junie CLI",
                id: "junie",
                config_path: home.join(".junie/mcp/mcp.json"),
                method: InstallMethod::FileEdit(JUNIE_JSON),
            },
            // --- Phase 3: project-level / GUI-only ---
            Tool {
                name: "Continue.dev",
                id: "continue",
                config_path: PathBuf::from(".continue/mcpServers/engrammic.yaml"),
                method: InstallMethod::FileEdit(ConfigShape::ContinueYaml),
            },
            Tool {
                name: "Roo Code",
                id: "roo",
                config_path: PathBuf::from(".roo/mcp.json"),
                method: InstallMethod::FileEdit(ROO_JSON),
            },
            Tool {
                name: "Cline",
                id: "cline",
                config_path: PathBuf::from(".cline/mcp.json"),
                method: InstallMethod::FileEdit(CLINE_JSON),
            },
            Tool {
                name: "JetBrains AI Assistant",
                id: "jetbrains",
                // Detection marker: presence of ~/.config/JetBrains parent dir.
                config_path: home.join(".config/JetBrains"),
                method: InstallMethod::PrintInstructions("Settings > Tools > AI Assistant > MCP"),
            },
            Tool {
                name: "Trae",
                id: "trae",
                // No stable on-disk path; GUI-managed only.
                config_path: home.join(".config/trae"),
                method: InstallMethod::PrintInstructions("the in-app MCP panel"),
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

    /// Comma-separated list of valid `--tool` ids, for help text and error messages.
    pub fn valid_ids() -> String {
        Self::all()
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SkillScope {
    User,    // Global, user-level
    Project, // Project-local
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SkillFormat {
    Directory, // engrammic-<name>/SKILL.md
    CursorMdc, // .cursor/rules/engrammic-<name>.mdc
    GeminiMd,  // Single GEMINI.md with markers
    AgentsMd,  // Single AGENTS.md with markers (Codex, Copilot CLI) - same merge logic as GeminiMd
}

#[derive(Clone)]
pub struct SkillDest {
    pub name: &'static str,
    pub harness: &'static str,
    pub path: PathBuf,
    pub scope: SkillScope,
    pub format: SkillFormat,
    pub default: bool,
    pub note: Option<&'static str>,
}

impl SkillDest {
    pub fn all() -> Vec<SkillDest> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));

        let claude_present = home.join(".claude").exists();
        let pi_present = home.join(".pi/agent").exists();

        let cursor_present = home.join(".cursor").exists();
        let gemini_present = home.join(".gemini").exists();
        let codex_present = home.join(".codex").exists();

        vec![
            // === User-level (global) destinations ===
            SkillDest {
                name: "Claude Code",
                harness: "claude",
                path: home.join(".claude/skills"),
                scope: SkillScope::User,
                format: SkillFormat::Directory,
                default: claude_present,
                note: None,
            },
            SkillDest {
                name: "Pi Agents",
                harness: "pi",
                path: home.join(".pi/agent/skills"),
                scope: SkillScope::User,
                format: SkillFormat::Directory,
                default: pi_present && !claude_present,
                note: None,
            },
            SkillDest {
                name: "Gemini CLI",
                harness: "gemini",
                path: home.join(".gemini/GEMINI.md"),
                scope: SkillScope::User,
                format: SkillFormat::GeminiMd,
                default: gemini_present && !claude_present,
                note: None,
            },
            SkillDest {
                name: "OpenAI Codex CLI",
                harness: "codex",
                path: home.join(".codex/AGENTS.md"),
                scope: SkillScope::User,
                format: SkillFormat::AgentsMd,
                default: codex_present && !claude_present,
                note: None,
            },
            // === Project-level destinations ===
            SkillDest {
                name: "Cursor (project)",
                harness: "cursor",
                path: PathBuf::from(".cursor/rules"),
                scope: SkillScope::Project,
                format: SkillFormat::CursorMdc,
                default: cursor_present,
                note: None,
            },
            SkillDest {
                name: "Gemini CLI (project)",
                harness: "gemini",
                path: PathBuf::from("GEMINI.md"),
                scope: SkillScope::Project,
                format: SkillFormat::GeminiMd,
                default: false,
                note: None,
            },
            SkillDest {
                name: "Windsurf (project)",
                harness: "windsurf",
                path: PathBuf::from(".windsurf/skills"),
                scope: SkillScope::Project,
                format: SkillFormat::Directory,
                default: false,
                note: None,
            },
            SkillDest {
                name: "Antigravity (project)",
                harness: "antigravity",
                path: PathBuf::from(".agent/skills"),
                scope: SkillScope::Project,
                format: SkillFormat::Directory,
                default: false,
                note: None,
            },
            SkillDest {
                name: "Cross-harness (project)",
                harness: "cross",
                path: PathBuf::from(".agents/skills"),
                scope: SkillScope::Project,
                format: SkillFormat::Directory,
                default: false,
                note: Some("Works with Pi Agents, Claude Code"),
            },
            SkillDest {
                name: "AGENTS.md (project)",
                harness: "agents",
                path: PathBuf::from("AGENTS.md"),
                scope: SkillScope::Project,
                format: SkillFormat::AgentsMd,
                default: false,
                note: Some("Works with Codex, GitHub Copilot CLI, and other AGENTS.md-aware tools"),
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
