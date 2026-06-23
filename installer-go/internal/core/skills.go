package core

import (
	"os"
	"path/filepath"
)

type SkillScope int

const (
	SkillScopeUser    SkillScope = iota // Global, user-level
	SkillScopeProject                   // Project-local
)

type SkillFormat int

const (
	SkillFormatDirectory SkillFormat = iota // engrammic-<name>/SKILL.md
	SkillFormatCursorMdc                    // .cursor/rules/engrammic-<name>.mdc
	SkillFormatGeminiMd                     // Single GEMINI.md with markers
	SkillFormatAgentsMd                     // Single AGENTS.md with markers
)

type SkillDest struct {
	Name    string
	Harness string
	Path    string // absolute for user-level, relative for project
	Scope   SkillScope
	Format  SkillFormat
	Default bool
	Note    *string
}

func note(s string) *string { return &s }

func AllSkillDests() []SkillDest {
	home := homeDir()

	claudePresent := dirExists(filepath.Join(home, ".claude"))
	piPresent := dirExists(filepath.Join(home, ".pi/agent"))
	veilPresent := dirExists(filepath.Join(home, ".veil"))
	openclawPresent := dirExists(filepath.Join(home, ".openclaw"))
	hermesPresent := dirExists(filepath.Join(home, ".hermes"))

	cursorPresent := dirExists(filepath.Join(home, ".cursor"))
	geminiPresent := dirExists(filepath.Join(home, ".gemini"))
	codexPresent := dirExists(filepath.Join(home, ".codex"))

	return []SkillDest{
		// User-level (global) destinations
		{
			Name:    "Claude Code",
			Harness: "claude",
			Path:    filepath.Join(home, ".claude/skills"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatDirectory,
			Default: claudePresent,
			Note:    nil,
		},
		{
			Name:    "Pi Agents",
			Harness: "pi",
			Path:    filepath.Join(home, ".pi/agent/skills"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatDirectory,
			Default: piPresent && !claudePresent && !veilPresent,
			Note:    nil,
		},
		{
			Name:    "\033[38;5;213mV\033[38;5;177me\033[38;5;141mi\033[38;5;105ml\033[0m",
			Harness: "veil",
			Path:    filepath.Join(home, ".veil/skills"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatDirectory,
			Default: veilPresent && !claudePresent,
			Note:    nil,
		},
		{
			Name:    "Gemini CLI",
			Harness: "gemini",
			Path:    filepath.Join(home, ".gemini/GEMINI.md"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatGeminiMd,
			Default: geminiPresent && !claudePresent,
			Note:    nil,
		},
		{
			Name:    "OpenAI Codex CLI",
			Harness: "codex",
			Path:    filepath.Join(home, ".codex/AGENTS.md"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatAgentsMd,
			Default: codexPresent && !claudePresent,
			Note:    nil,
		},
		{
			Name:    "OpenClaw",
			Harness: "openclaw",
			Path:    filepath.Join(home, ".openclaw/skills"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatDirectory,
			Default: openclawPresent && !claudePresent,
			Note:    nil,
		},
		{
			Name:    "Hermes Agent",
			Harness: "hermes",
			Path:    filepath.Join(home, ".hermes/skills"),
			Scope:   SkillScopeUser,
			Format:  SkillFormatDirectory,
			Default: hermesPresent && !claudePresent,
			Note:    nil,
		},
		// Project-level destinations
		{
			Name:    "Cursor (project)",
			Harness: "cursor",
			Path:    ".cursor/rules",
			Scope:   SkillScopeProject,
			Format:  SkillFormatCursorMdc,
			Default: cursorPresent,
			Note:    nil,
		},
		{
			Name:    "Gemini CLI (project)",
			Harness: "gemini",
			Path:    "GEMINI.md",
			Scope:   SkillScopeProject,
			Format:  SkillFormatGeminiMd,
			Default: false,
			Note:    nil,
		},
		{
			Name:    "Windsurf (project)",
			Harness: "windsurf",
			Path:    ".windsurf/skills",
			Scope:   SkillScopeProject,
			Format:  SkillFormatDirectory,
			Default: false,
			Note:    nil,
		},
		{
			Name:    "Antigravity (project)",
			Harness: "antigravity",
			Path:    ".agent/skills",
			Scope:   SkillScopeProject,
			Format:  SkillFormatDirectory,
			Default: false,
			Note:    nil,
		},
		{
			Name:    "Cross-harness (project)",
			Harness: "cross",
			Path:    ".agents/skills",
			Scope:   SkillScopeProject,
			Format:  SkillFormatDirectory,
			Default: false,
			Note:    note("Works with Pi Agents, Claude Code"),
		},
		{
			Name:    "AGENTS.md (project)",
			Harness: "agents",
			Path:    "AGENTS.md",
			Scope:   SkillScopeProject,
			Format:  SkillFormatAgentsMd,
			Default: false,
			Note:    note("Works with Codex, GitHub Copilot CLI, and other AGENTS.md-aware tools"),
		},
	}
}

func UserLevelDests() []SkillDest {
	var out []SkillDest
	for _, d := range AllSkillDests() {
		if d.Scope == SkillScopeUser {
			out = append(out, d)
		}
	}
	return out
}

func ProjectLevelDests() []SkillDest {
	var out []SkillDest
	for _, d := range AllSkillDests() {
		if d.Scope == SkillScopeProject {
			out = append(out, d)
		}
	}
	return out
}

func DefaultDests() []SkillDest {
	var out []SkillDest
	for _, d := range AllSkillDests() {
		if d.Default {
			out = append(out, d)
		}
	}
	return out
}

func dirExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.IsDir()
}
