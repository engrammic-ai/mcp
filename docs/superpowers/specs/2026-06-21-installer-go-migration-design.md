# Installer CLI Migration: Rust to Go + Bubbletea

**Date:** 2026-06-21  
**Status:** Draft  
**Author:** NovusEdge + Claude

## Summary

Rewrite the `installer-cli` from Rust (dialoguer/colored/indicatif) to Go with the Charm stack (Bubbletea, Huh, Lipgloss, Bubbles) for better TUI aesthetics, easier maintenance, and improved UX.

## Goals

1. **Better TUI** — Wizard-style interface with step navigation, themed forms, progress display
2. **Easier Maintenance** — Go is simpler than Rust for this use case; faster iteration
3. **UX Redesign** — Step-based wizards with back/next, clear progress indicators, plan preview before execution
4. **Full Feature Parity** — All current functionality migrated (install, update, remove, selfhost, doctor, skills, etc.)

## Non-Goals

- Full-screen dashboard TUI (problematic on server terminals)
- New features beyond current scope
- Backward compatibility with Rust installer config format (manifest format stays the same)

## Current State

The Rust installer is ~11k lines across 18 modules:

| Module | Lines | Purpose |
|--------|-------|---------|
| `selfhost.rs` | 3,485 | Docker wizard with provider selection |
| `main.rs` | 2,193 | CLI entry, install/update/remove flows |
| `config.rs` | 1,230 | Config file editing (JSON/YAML/TOML) |
| `skills.rs` | 976 | Skills installation and formatting |
| `manifest.rs` | 573 | Installation manifest tracking |
| `tools.rs` | 541 | Harness definitions and detection |
| `doctor.rs` | 462 | Diagnostic checks |
| `providers.rs` | 361 | LLM/embedding provider definitions |
| Others | ~1,500 | License, deeplink, banner, etc. |

**Dependencies:** clap, serde, dialoguer, colored, indicatif, toml, ureq, ed25519-dalek

## Proposed Architecture

### Technology Stack

| Package | Purpose |
|---------|---------|
| `github.com/spf13/cobra` | CLI framework |
| `github.com/charmbracelet/huh` | Wizard forms (multi-step, validation) |
| `github.com/charmbracelet/lipgloss` | Styled output (colors, borders, layout) |
| `github.com/charmbracelet/bubbles` | Spinners, progress bars |
| `gopkg.in/yaml.v3` | YAML config editing |
| `github.com/pelletier/go-toml/v2` | TOML config editing |
| `golang.org/x/crypto/ed25519` | License validation |

### Package Structure

```
installer-go/
├── cmd/
│   └── engrammic/
│       └── main.go              # Entry point, cobra setup
│
├── internal/
│   ├── cli/                     # Command handlers
│   │   ├── root.go              # Root command, global flags
│   │   ├── install.go           # install command
│   │   ├── update.go            # update command
│   │   ├── remove.go            # remove command
│   │   ├── selfhost.go          # selfhost/docker command
│   │   ├── doctor.go            # doctor command
│   │   ├── status.go            # status command (plain output)
│   │   ├── logs.go              # logs command
│   │   └── license.go           # license command
│   │
│   ├── wizard/                  # Wizard flows
│   │   ├── wizard.go            # Wizard runner (step machine)
│   │   ├── install.go           # Install wizard steps
│   │   ├── selfhost.go          # Selfhost wizard steps
│   │   └── forms.go             # Shared form builders
│   │
│   ├── ui/                      # Presentation layer
│   │   ├── theme.go             # Lipgloss theme/colors
│   │   ├── banner.go            # ASCII banner
│   │   ├── output.go            # Styled print helpers
│   │   ├── progress.go          # Execution progress display
│   │   └── table.go             # Status tables
│   │
│   ├── core/                    # Business logic (no UI deps)
│   │   ├── harness.go           # Harness definitions
│   │   ├── config.go            # Config file read/write
│   │   ├── manifest.go          # Manifest management
│   │   ├── skills.go            # Skills installation
│   │   ├── docker.go            # Docker compose operations
│   │   ├── doctor.go            # Diagnostic checks
│   │   ├── license.go           # License validation
│   │   └── providers.go         # LLM/embedding providers
│   │
│   └── platform/                # OS-specific code
│       ├── paths.go             # Config paths per OS
│       ├── detect.go            # Editor detection
│       └── terminal.go          # Terminal capability detection
│
├── go.mod
├── go.sum
└── Makefile                     # Build targets
```

### Wizard UI Design

Step-based wizard with clear navigation:

```
┌────────────────────────────────────────────────────────────┐
│  Engrammic Installer                          Step 2 of 5  │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Select editors to configure:                              │
│                                                            │
│    ● Claude Code        (detected)                         │
│    ○ Cursor             (detected)                         │
│    ● Windsurf           (detected)                         │
│    ○ VS Code                                               │
│                                                            │
│  ↑/↓ move  •  space toggle  •  enter next  •  esc back    │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Wizard State Model

```go
type WizardState struct {
    CurrentStep  int
    TotalSteps   int
    
    // Collected answers
    Endpoint     string
    ToInstall    []Harness
    ToRemove     []Harness
    SkillDests   []SkillDest
    
    // Selfhost-specific
    Tier         string
    LicenseKey   string
    Providers    ProviderSet
    Credentials  map[string]string
}

type Step interface {
    Title() string
    Form(state *WizardState) *huh.Form
    Validate(state *WizardState) error
    Skip(state *WizardState) bool
}
```

### Install Wizard Steps

1. **Deployment Mode** — Cloud vs Self-hosted
2. **Select Harnesses** — Multi-select detected editors
3. **Select Skills** — Multi-select skill destinations
4. **Review Plan** — Summary with confirm
5. **Execute** — Progress with live checkmarks

### Selfhost Wizard Steps

1. **Runtime** — Docker vs Podman
2. **Tier** — Cloud providers vs Standalone
3. **RAM Check** — System resource validation (standalone only)
4. **Provider Selection** — LLM, Embedding, Reranker (cloud tier only)
5. **Credentials** — API keys for providers
6. **License** — License key entry/validation
7. **Config** — Port, directory, passwords
8. **Review** — Full summary
9. **Execute** — Docker compose with progress

### Execution Display

```
┌────────────────────────────────────────────────────────────┐
│  Installing Engrammic                                      │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ✓ Claude Code         configured                          │
│  ✓ Windsurf            configured                          │
│  ⠋ Cursor              opening deep link...                │
│  ○ Skills              waiting                             │
│                                                            │
├────────────────────────────────────────────────────────────┤
│  Progress: 2/4                                             │
└────────────────────────────────────────────────────────────┘
```

### Execution Model

```go
type Executor struct {
    Steps    []ExecutionStep
    OnUpdate func(step int, status StepStatus)
}

type StepStatus int
const (
    Pending StepStatus = iota
    Running
    Done
    Failed
    Manual  // Deep links requiring user action
)
```

Skip-and-continue: one failure doesn't abort remaining steps.

### Terminal Fallback

```go
func DetectTerminal() TerminalMode {
    if os.Getenv("TERM") == "dumb" || !term.IsTerminal(os.Stdout.Fd()) {
        return ModePlain      // No colors, basic prompts
    }
    if os.Getenv("NO_COLOR") != "" {
        return ModeNoColor    // Structured but no ANSI
    }
    return ModeFull           // Full wizard experience
}
```

**Non-interactive mode (`-y`):** Skip prompts, use detected defaults, plain output.

## Module Migration Map

| Rust | Go | Notes |
|------|-----|-------|
| `config.rs` | `core/config.go` | JSON/YAML/TOML editing |
| `manifest.rs` | `core/manifest.go` | Direct port |
| `skills.rs` | `core/skills.go` | Use `embed.FS` for bundled skills |
| `tools.rs` | `core/harness.go` | Rename Tool → Harness |
| `selfhost.rs` | `core/docker.go` + `wizard/selfhost.go` | Split orchestration from docker ops |
| `doctor.rs` | `core/doctor.go` | Direct port |
| `providers.rs` | `core/providers.go` | Direct port |
| `flow.rs` | `wizard/wizard.go` | Absorbed into wizard runner |
| `deeplink.rs` | `core/deeplink.go` | Direct port |
| `license.rs` | `core/license.go` | Same ed25519 validation |

## Embedded Assets

```go
//go:embed skills/*
var skillsFS embed.FS

//go:embed docker-compose.yml .env.template
var dockerAssets embed.FS
```

## Testing Strategy

- **Unit tests:** `core/` packages with standard Go tests (no UI dependencies)
- **Wizard tests:** Step logic with mock state
- **Integration:** Golden file tests for CLI output
- **Manual:** Linux, macOS, Windows, SSH sessions, dumb terminals

## Build & Distribution

```makefile
# Cross-compilation
build-all:
    GOOS=linux GOARCH=amd64 go build -o dist/engrammic-linux-amd64
    GOOS=darwin GOARCH=amd64 go build -o dist/engrammic-darwin-amd64
    GOOS=darwin GOARCH=arm64 go build -o dist/engrammic-darwin-arm64
    GOOS=windows GOARCH=amd64 go build -o dist/engrammic-windows-amd64.exe
```

Single binary with embedded assets, no runtime dependencies.

## Migration Strategy

**Big bang rewrite:**
1. Build complete Go installer with full feature parity
2. Test thoroughly across platforms
3. Replace Rust version in single release
4. Archive Rust codebase

**Rationale:** Clean slate allows UX improvements without legacy constraints. Selfhost wizard benefits from fresh implementation rather than incremental porting.

## Success Criteria

1. All current commands work identically (`install`, `update`, `remove`, `selfhost`, `doctor`, `status`, `logs`, `license`, `skills`, `upgrade`, `scale`)
2. Wizard experience feels polished on modern terminals
3. Graceful fallback on dumb/non-interactive terminals
4. Binary size comparable or smaller than Rust version
5. Cross-platform builds work (Linux, macOS, Windows)

## Open Questions

None at this time.

## References

- [Charm Bubbletea](https://github.com/charmbracelet/bubbletea)
- [Charm Huh](https://github.com/charmbracelet/huh)
- [Charm Lipgloss](https://github.com/charmbracelet/lipgloss)
- Current Rust installer: `installer-cli/`
