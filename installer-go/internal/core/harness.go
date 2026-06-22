package core

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

// TypeField is the transport discriminator written into MCP config entries.
type TypeField int

const (
	TypeFieldNone           TypeField = iota // omit type field
	TypeFieldHttp                            // "http"
	TypeFieldStreamableHttp                  // "streamable-http"
	TypeFieldRemote                          // "remote"
)

func (t TypeField) Value() string {
	switch t {
	case TypeFieldHttp:
		return "http"
	case TypeFieldStreamableHttp:
		return "streamable-http"
	case TypeFieldRemote:
		return "remote"
	}
	return ""
}

// ConfigShapeKind identifies the on-disk config format for a harness.
type ConfigShapeKind int

const (
	ConfigShapeJsonMap      ConfigShapeKind = iota
	ConfigShapeCodexToml
	ConfigShapeGooseYaml
	ConfigShapeOpenCodeJson
	ConfigShapeContinueYaml
	ConfigShapeHermesYaml
	ConfigShapeVSCodeJson // servers key, not mcpServers
)

// ConfigShape describes how MCP config is serialized.
type ConfigShape struct {
	Kind      ConfigShapeKind
	Container string    // top-level key for JsonMap
	TypeField TypeField // transport type field
	UrlField  string    // "url" or "serverUrl"
}

// InstallMethod describes how the installer registers the MCP server.
type InstallMethod int

const (
	InstallMethodFileEdit          InstallMethod = iota
	InstallMethodDeepLink
	InstallMethodPrintInstructions
)

// DeepLinkKind identifies the URI scheme used by a deeplink harness.
type DeepLinkKind int

const (
	DeepLinkVSCode     DeepLinkKind = iota // vscode:mcp/install?{urlencoded}
	DeepLinkCursor                         // cursor://anysphere.cursor-deeplink/mcp/install?...
	DeepLinkWindsurf                       // windsurf://windsurf-mcp-registry?serverName=X
	DeepLinkClaudeWeb                      // https://claude.ai/install-mcp
)

// Harness represents an AI coding tool that can be configured with the Engrammic MCP server.
type Harness struct {
	Name         string
	ID           string
	ConfigPath   string
	Method       InstallMethod
	Shape        *ConfigShape  // non-nil only for FileEdit
	DeepLink     *DeepLinkKind // non-nil only for DeepLink
	Instructions string        // non-empty only for PrintInstructions
}

// pre-defined config shapes shared by multiple harnesses
var (
	standardJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "mcpServers",
		TypeField: TypeFieldHttp,
		UrlField:  "url",
	}
	ampJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "amp.mcpServers",
		TypeField: TypeFieldNone,
		UrlField:  "url",
	}
	zedJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "context_servers",
		TypeField: TypeFieldNone,
		UrlField:  "url",
	}
	junieJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "mcpServers",
		TypeField: TypeFieldNone,
		UrlField:  "url",
	}
	clineJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "mcpServers",
		TypeField: TypeFieldNone,
		UrlField:  "url",
	}
	windsurfJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "mcpServers",
		TypeField: TypeFieldNone,
		UrlField:  "serverUrl",
	}
	rooJSON = &ConfigShape{
		Kind:      ConfigShapeJsonMap,
		Container: "mcpServers",
		TypeField: TypeFieldStreamableHttp,
		UrlField:  "url",
	}
	vsCodeFileJSON = &ConfigShape{
		Kind:      ConfigShapeVSCodeJson,
		Container: "servers",
		TypeField: TypeFieldHttp,
		UrlField:  "url",
	}
)

func homeDir() string {
	if h, err := os.UserHomeDir(); err == nil {
		return h
	}
	return "~"
}

func claudeDesktopConfigPath() string {
	home := homeDir()
	switch runtime.GOOS {
	case "darwin":
		return filepath.Join(home, "Library", "Application Support", "Claude", "claude_desktop_config.json")
	case "windows":
		if appdata := os.Getenv("APPDATA"); appdata != "" {
			return filepath.Join(appdata, "Claude", "claude_desktop_config.json")
		}
		return filepath.Join(home, "AppData", "Roaming", "Claude", "claude_desktop_config.json")
	default:
		return filepath.Join(home, ".config", "Claude", "claude_desktop_config.json")
	}
}

func vsCodeUserConfigPath() string {
	home := homeDir()
	switch runtime.GOOS {
	case "darwin":
		return filepath.Join(home, "Library", "Application Support", "Code", "User", "mcp.json")
	case "windows":
		if appdata := os.Getenv("APPDATA"); appdata != "" {
			return filepath.Join(appdata, "Code", "User", "mcp.json")
		}
		return filepath.Join(home, "AppData", "Roaming", "Code", "User", "mcp.json")
	default:
		return filepath.Join(home, ".config", "Code", "User", "mcp.json")
	}
}

func deepLinkPtr(k DeepLinkKind) *DeepLinkKind {
	return &k
}

// AllHarnesses returns the full list of supported harnesses.
func AllHarnesses() []Harness {
	home := homeDir()
	return []Harness{
		// --- FileEdit: user-level ---
		{
			Name:       "Claude Code",
			ID:         "claude",
			ConfigPath: filepath.Join(home, ".claude", "settings.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "Claude Desktop",
			ID:         "claude-desktop",
			ConfigPath: claudeDesktopConfigPath(),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "Windsurf",
			ID:         "windsurf",
			ConfigPath: filepath.Join(home, ".codeium", "windsurf", "mcp_config.json"),
			Method:     InstallMethodFileEdit,
			Shape:      windsurfJSON,
		},
		{
			Name:       "Antigravity",
			ID:         "antigravity",
			ConfigPath: filepath.Join(home, ".gemini", "antigravity", "mcp_config.json"),
			Method:     InstallMethodFileEdit,
			Shape:      windsurfJSON,
		},
		{
			Name:       "Gemini CLI",
			ID:         "gemini",
			ConfigPath: filepath.Join(home, ".gemini", "settings.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "Pi Agents",
			ID:         "pi",
			ConfigPath: filepath.Join(home, ".pi", "agent", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "GitHub Copilot CLI",
			ID:         "copilot",
			ConfigPath: filepath.Join(home, ".copilot", "mcp-config.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "OpenAI Codex CLI",
			ID:         "codex",
			ConfigPath: filepath.Join(home, ".codex", "config.toml"),
			Method:     InstallMethodFileEdit,
			Shape:      &ConfigShape{Kind: ConfigShapeCodexToml},
		},
		{
			Name:       "Goose",
			ID:         "goose",
			ConfigPath: filepath.Join(home, ".config", "goose", "config.yaml"),
			Method:     InstallMethodFileEdit,
			Shape:      &ConfigShape{Kind: ConfigShapeGooseYaml},
		},
		{
			Name:       "Sourcegraph Amp",
			ID:         "amp",
			ConfigPath: filepath.Join(home, ".config", "amp", "settings.json"),
			Method:     InstallMethodFileEdit,
			Shape:      ampJSON,
		},
		{
			Name:       "OpenCode",
			ID:         "opencode",
			ConfigPath: filepath.Join(home, ".config", "opencode", "opencode.json"),
			Method:     InstallMethodFileEdit,
			Shape:      &ConfigShape{Kind: ConfigShapeOpenCodeJson},
		},
		{
			Name:       "Amazon Q",
			ID:         "amazonq",
			ConfigPath: filepath.Join(home, ".aws", "amazonq", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "Zed",
			ID:         "zed",
			ConfigPath: filepath.Join(home, ".config", "zed", "settings.json"),
			Method:     InstallMethodFileEdit,
			Shape:      zedJSON,
		},
		{
			Name:       "Kiro",
			ID:         "kiro",
			ConfigPath: filepath.Join(home, ".kiro", "settings", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "JetBrains Junie CLI",
			ID:         "junie",
			ConfigPath: filepath.Join(home, ".junie", "mcp", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      junieJSON,
		},
		{
			Name:       "OpenClaw",
			ID:         "openclaw",
			ConfigPath: filepath.Join(home, ".openclaw", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      standardJSON,
		},
		{
			Name:       "Hermes Agent",
			ID:         "hermes",
			ConfigPath: filepath.Join(home, ".hermes", "config.yaml"),
			Method:     InstallMethodFileEdit,
			Shape:      &ConfigShape{Kind: ConfigShapeHermesYaml},
		},
		// --- NEW: VS Code file-edit (in addition to deeplink) ---
		{
			Name:       "VS Code",
			ID:         "vscode-file",
			ConfigPath: vsCodeUserConfigPath(),
			Method:     InstallMethodFileEdit,
			Shape:      vsCodeFileJSON,
		},
		// --- FileEdit: project-level ---
		{
			Name:       "Continue.dev",
			ID:         "continue",
			ConfigPath: filepath.Join(".continue", "mcpServers", "engrammic.yaml"),
			Method:     InstallMethodFileEdit,
			Shape:      &ConfigShape{Kind: ConfigShapeContinueYaml},
		},
		{
			Name:       "Roo Code",
			ID:         "roo",
			ConfigPath: filepath.Join(".roo", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      rooJSON,
		},
		{
			Name:       "Cline",
			ID:         "cline",
			ConfigPath: filepath.Join(".cline", "mcp.json"),
			Method:     InstallMethodFileEdit,
			Shape:      clineJSON,
		},
		// --- DeepLink ---
		{
			Name:       "Cursor",
			ID:         "cursor",
			ConfigPath: filepath.Join(home, ".cursor", "extensions"), // detection marker
			Method:     InstallMethodDeepLink,
			DeepLink:   deepLinkPtr(DeepLinkCursor),
		},
		{
			Name:       "VS Code (Copilot)",
			ID:         "vscode",
			ConfigPath: filepath.Join(home, ".vscode", "extensions"), // detection marker
			Method:     InstallMethodDeepLink,
			DeepLink:   deepLinkPtr(DeepLinkVSCode),
		},
		{
			Name:       "Windsurf (deeplink)",
			ID:         "windsurf-dl",
			ConfigPath: filepath.Join(home, ".codeium", "windsurf"), // detection marker
			Method:     InstallMethodDeepLink,
			DeepLink:   deepLinkPtr(DeepLinkWindsurf),
		},
		// --- PrintInstructions ---
		{
			Name:         "JetBrains AI Assistant",
			ID:           "jetbrains",
			ConfigPath:   filepath.Join(home, ".config", "JetBrains"),
			Method:       InstallMethodPrintInstructions,
			Instructions: "Settings > Tools > AI Assistant > MCP",
		},
		{
			Name:         "Trae",
			ID:           "trae",
			ConfigPath:   filepath.Join(home, ".config", "trae"),
			Method:       InstallMethodPrintInstructions,
			Instructions: "the in-app MCP panel",
		},
	}
}

// DetectInstalled returns harnesses whose config parent directory exists on disk.
func DetectInstalled() []Harness {
	var out []Harness
	for _, h := range AllHarnesses() {
		if !filepath.IsAbs(h.ConfigPath) {
			continue
		}
		dir := filepath.Dir(h.ConfigPath)
		if info, err := os.Stat(dir); err == nil && info.IsDir() {
			out = append(out, h)
		}
	}
	return out
}

// FromID returns the harness with the given ID, or nil if not found.
func FromID(id string) *Harness {
	all := AllHarnesses()
	for i := range all {
		if all[i].ID == id {
			return &all[i]
		}
	}
	return nil
}

// ValidIDs returns a comma-separated list of all harness IDs.
func ValidIDs() string {
	all := AllHarnesses()
	ids := make([]string, len(all))
	for i, h := range all {
		ids[i] = h.ID
	}
	return strings.Join(ids, ", ")
}
