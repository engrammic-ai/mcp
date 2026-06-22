# Engrammic Go Installer CLI & Wizard Spec

**Date:** 2026-06-22  
**Status:** Approved  
**Builds on:** 2026-06-21-installer-go-migration-design.md

## Overview

Full CLI and wizard specification for the Go installer. Single binary with embedded Charm TUI, supporting both cloud and self-hosted deployments.

## Target Users

Mixed audience: developers who want speed AND technical non-devs who prefer guided flows. The wizard auto-detects terminal capabilities and falls back gracefully.

## Entry Point

```bash
# Unix
curl -fsSL https://get.engrammic.ai/install.sh | sh

# Windows
irm https://get.engrammic.ai/install.ps1 | iex
```

The shell script downloads the Go binary and executes it. The binary IS the wizard.

## Command Structure

```
engrammic                    # Runs install wizard (default)
engrammic install            # Alias for wizard
engrammic install -y         # Non-interactive, use defaults
engrammic install --tool=X   # Pre-select specific tools

engrammic status             # Show installed harnesses, server state, endpoint
engrammic doctor             # Run diagnostics (ports, docker, configs)
engrammic remove             # Uninstall wizard (select what to remove)
engrammic remove -y          # Remove everything non-interactively

engrammic selfhost           # Selfhost setup wizard
engrammic selfhost up        # Start containers
engrammic selfhost down      # Stop containers
engrammic selfhost logs      # Tail container logs
engrammic selfhost upgrade   # Upgrade containers (preserve config)

engrammic license            # Show/set license key
engrammic skills             # Install/manage skills
engrammic version            # Show version info
```

### Global Flags

| Flag | Description |
|------|-------------|
| `-y, --yes` | Accept defaults, no prompts |
| `-v, --verbose` | Debug output |
| `--no-color` | Disable colors (auto-detected for dumb terminals) |
| `--endpoint URL` | Override endpoint (cloud or selfhost) |
| `--tool=X,Y` | Pre-select specific tools |

## Wizard Behavior

- **Always wizard** — runs full wizard every time, pre-fills detected values
- **Linear with back** — Step 1 → 2 → 3 → Review → Execute, can go back anytime
- **First question** — "Cloud or Self-hosted?" then completely separate flows

## Cloud Install Wizard (4 steps)

### Step 1: Deployment Mode

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Installer                                  Step 1/4  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  How do you want to connect to Engrammic?                       │
│                                                                 │
│  › Cloud (recommended)                                          │
│      Connect to engrammic.ai - no setup required                │
│                                                                 │
│    Self-hosted                                                  │
│      Run your own server with Docker                            │
│                                                                 │
│  ↑/↓ select  •  enter confirm  •  q quit                        │
└─────────────────────────────────────────────────────────────────┘
```

### Step 2: Select Editors

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Installer                                  Step 2/4  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Select editors to configure:                                   │
│                                                                 │
│  › [x] Claude Code           ~/.claude/settings.json            │
│    [x] Windsurf              ~/.codeium/windsurf/mcp_config     │
│    [ ] Cursor                ~/.cursor/ (not detected)          │
│    [ ] VS Code               ~/.config/Code/User/mcp.json       │
│    [ ] Gemini CLI            ~/.gemini/settings.json            │
│                                                                 │
│  Detected editors are pre-selected.                             │
│                                                                 │
│  ↑/↓ move  •  space toggle  •  enter next  •  esc back          │
└─────────────────────────────────────────────────────────────────┘
```

- Editors are configured via file edit (JSON/YAML/TOML depending on harness)
- Deeplinks only used for web interfaces (Claude.ai web, etc.)
- Detected editors are pre-selected

### Step 3: Install Skills (Optional)

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Installer                                  Step 3/4  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Install Engrammic skills? (optional)                           │
│                                                                 │
│  › [x] Claude Code           ~/.claude/skills/                  │
│    [ ] Cursor (project)      .cursor/rules/                     │
│                                                                 │
│  Skills add slash commands like /engrammic-recall               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Step 4: Review & Execute

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Installer                                  Step 4/4  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Ready to install:                                              │
│                                                                 │
│  Endpoint:  https://beta.engrammic.ai/mcp/                      │
│                                                                 │
│  Editors:   Claude Code, Windsurf                               │
│  Skills:    Claude Code                                         │
│                                                                 │
│  › Install now                                                  │
│    Go back                                                      │
│    Cancel                                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Execution Display

```
│  Installing...                                                  │
│                                                                 │
│  ✓ Claude Code         configured                               │
│  ⠋ Windsurf            writing config...                        │
│  ○ Skills              waiting                                  │
```

## Selfhost Wizard (8 steps)

### Step 1: Deployment Mode
Same as cloud, select "Self-hosted"

### Step 2: Runtime Selection

```
│  Select container runtime:                                      │
│                                                                 │
│  › Docker                                                       │
│    Podman                                                       │
```

### Step 3: Tier Selection

```
│  Select deployment tier:                                        │
│                                                                 │
│  › Standalone                                                   │
│      Run everything locally - no API keys needed                │
│      Requires: 16GB+ RAM, ~10GB disk                            │
│                                                                 │
│    Cloud Providers                                              │
│      Use your own API keys (OpenAI, Anthropic, etc.)            │
│      Lower resource requirements                                │
```

### Step 4a (Standalone): RAM Check

```
│  System Resources                                               │
│                                                                 │
│  Available RAM: 32 GB  ✓                                        │
│  Required:      16 GB                                           │
│                                                                 │
│  › Continue                                                     │
│    Choose Cloud Providers instead                               │
```

### Step 4b (Cloud tier): Provider Selection

**LLM Provider:**
```
│  Select LLM Provider:                                           │
│                                                                 │
│  › OpenAI           gpt-4o / gpt-4o-mini                        │
│    Anthropic        claude-sonnet / claude-haiku                │
│    Google Gemini    gemini-2.5-pro / gemini-2.5-flash           │
│    Azure OpenAI     (requires endpoint URL)                     │
│    AWS Bedrock      (uses AWS credentials)                      │
│    Vertex AI        (uses GCP service account)                  │
│    Other            (custom litellm provider)                   │
```

**Embedding Provider:**
```
│  Select Embedding Provider:                                     │
│                                                                 │
│  › OpenAI           text-embedding-3-large (3072 dims)          │
│    Google Gemini    text-embedding-004 (768 dims)               │
│    Azure OpenAI     text-embedding-3-large                      │
│    AWS Bedrock      titan-embed-text-v2 (1024 dims)             │
│    Vertex AI        text-embedding-005 (768 dims)               │
```

**Reranker (optional):**
```
│  Select Reranker (optional):                                    │
│                                                                 │
│  › None             Skip reranking                              │
│    Local (light)    MiniLM-L6 - 22MB, <1GB RAM                  │
│    Local (quality)  Jina v2 - 278MB, 4GB RAM                    │
│    Cohere           rerank-v3.5 (requires API key)              │
│    Vertex AI        semantic-ranker                             │
```

### Step 5: Credentials

```
│  Enter API Credentials                                          │
│                                                                 │
│  OPENAI_API_KEY:                                                │
│  > sk-********************************                          │
│                                                                 │
│  (Keys are stored in ~/.engrammic/.env, never logged)           │
```

- Only prompts for credentials required by selected providers
- Credentials deduplicated (e.g., OpenAI for both LLM and embedding = one prompt)

### Step 6: License

```
│  Enter License Key                                              │
│                                                                 │
│  > ENGRAM-XXXX-XXXX-XXXX-XXXX                                   │
│                                                                 │
│  No license? Get one at https://engrammic.ai/license            │
│  Or press Enter to start a 14-day trial.                        │
```

### Step 7: Configuration

```
│  Server Configuration                                           │
│                                                                 │
│  Port:           8000  (available ✓)                            │
│  Data directory: ~/.engrammic/data                              │
│  Postgres pass:  (auto-generated, shown once)                   │
│                                                                 │
│  › Use defaults                                                 │
│    Customize                                                    │
```

### Step 8: Review & Execute

```
│  Ready to deploy:                                               │
│                                                                 │
│  Tier:       Cloud Providers                                    │
│  LLM:        OpenAI (gpt-4o)                                    │
│  Embedding:  OpenAI (text-embedding-3-large)                    │
│  Reranker:   Local (MiniLM-L6)                                  │
│  Port:       8000                                               │
│  Endpoint:   http://localhost:8000/mcp                          │
│                                                                 │
│  › Deploy now                                                   │
│    Go back                                                      │
│    Cancel                                                       │
```

### Selfhost Execution

```
│  Deploying...                                                   │
│                                                                 │
│  ✓ Generated docker-compose.yml                                 │
│  ✓ Generated .env                                               │
│  ⠋ Pulling images...  postgres:16  [=====>    ] 45%             │
│  ○ Starting containers                                          │
│  ○ Waiting for health checks                                    │
│  ○ Configuring editors                                          │
```

## Status Command

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Status                                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Server:    http://localhost:8000/mcp  ✓ running (container)    │
│  License:   ENGRAM-XXXX  ✓ valid until 2027-01-15               │
│                                                                 │
│  Configured Editors:                                            │
│  ✓ Claude Code       ~/.claude/settings.json                    │
│  ✓ Windsurf          ~/.codeium/windsurf/mcp_config.json        │
│  ✗ Cursor            config missing (run: engrammic install)    │
│                                                                 │
│  Skills:                                                        │
│  ✓ Claude Code       ~/.claude/skills/engrammic-*               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Doctor Command

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Doctor                                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Connectivity                                                   │
│  ✓ Endpoint reachable      http://localhost:8000/mcp            │
│  ✓ MCP handshake           tools: 12, resources: 3              │
│                                                                 │
│  Docker                                                         │
│  ✓ Docker running          v24.0.7                              │
│  ✓ Containers healthy      engrammic-server, engrammic-db       │
│  ⚠ Disk usage              78% (consider cleanup)               │
│                                                                 │
│  Ports                                                          │
│  ✓ 8000                    engrammic-server                     │
│  ✓ 5432                    engrammic-db (internal)              │
│                                                                 │
│  Configs                                                        │
│  ✓ Claude Code             valid JSON, endpoint matches         │
│  ✓ Windsurf                valid JSON, endpoint matches         │
│  ⚠ Cursor                  not configured                       │
│                                                                 │
│  2 warnings, 0 errors                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Remove Command

### Step 1: Select What to Remove

```
┌─────────────────────────────────────────────────────────────────┐
│  Engrammic Remove                                     Step 1/2  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  What do you want to remove?                                    │
│                                                                 │
│  › [x] Editor configs      (Claude Code, Windsurf)              │
│    [x] Skills              (~/.claude/skills/engrammic-*)       │
│    [ ] Selfhost server     (docker containers + data)           │
│    [ ] Everything          (all of the above + state)           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Step 2: Confirmation

```
│  Confirm removal:                                               │
│                                                                 │
│  Will remove:                                                   │
│  • Claude Code config entry (keeps other MCP servers)           │
│  • Windsurf config entry                                        │
│  • Skills: engrammic-recall, engrammic-learn, ...               │
│                                                                 │
│  Will preserve:                                                 │
│  • Docker containers and data                                   │
│  • ~/.engrammic/state.json                                      │
│                                                                 │
│  › Remove now                                                   │
│    Go back                                                      │
│    Cancel                                                       │
```

## Error Handling

### Failure During Execution (Stop and Ask)

```
│  ✗ Windsurf            permission denied: ~/.codeium/...        │
│                                                                 │
│  › [R]etry                                                      │
│    [S]kip and continue                                          │
│    [A]bort installation                                         │
```

### Missing Dependency

```
│  ⚠ Docker not found                                             │
│                                                                 │
│  Selfhost requires Docker. Install it from:                     │
│  https://docs.docker.com/get-docker/                            │
│                                                                 │
│  › Check again                                                  │
│    Switch to Cloud mode                                         │
│    Exit                                                         │
```

### Port Conflict

```
│  ⚠ Port 8000 in use                                             │
│                                                                 │
│  Used by: node (PID 12345)                                      │
│                                                                 │
│  › Use port 8001 instead                                        │
│    Enter custom port                                            │
│    Exit and resolve manually                                    │
```

### License Validation Failure

```
│  ✗ Invalid license key                                          │
│                                                                 │
│  The key ENGRAM-XXXX-... could not be validated.                │
│  Check for typos or get a new key at engrammic.ai/license       │
│                                                                 │
│  › Try again                                                    │
│    Start 14-day trial instead                                   │
│    Exit                                                         │
```

## Terminal Fallback

### Auto-Detection

```go
if os.Getenv("TERM") == "dumb" || !term.IsTerminal(os.Stdout.Fd()) {
    // Plain mode - no colors, basic prompts
}
if os.Getenv("NO_COLOR") != "" {
    // Structured but no ANSI
}
```

### Plain Terminal Mode

```
Engrammic Installer

How do you want to connect?
  1. Cloud (recommended)
  2. Self-hosted
Choice [1]: 1

Select editors to configure:
  1. [x] Claude Code (detected)
  2. [x] Windsurf (detected)
  3. [ ] Cursor
  4. [ ] VS Code
Toggle (1-4), done (d), or all (a) [d]: d

Install skills? [Y/n]: y

Ready to install:
  Endpoint: https://beta.engrammic.ai/mcp/
  Editors: Claude Code, Windsurf
  Skills: Claude Code

Proceed? [Y/n]: y

Installing...
  [OK] Claude Code
  [OK] Windsurf
  [OK] Skills

Done! Your editors are now connected to Engrammic.
```

### Non-Interactive Mode (`-y`)

```bash
$ engrammic install -y
Detecting editors... found: Claude Code, Windsurf
Using endpoint: https://beta.engrammic.ai/mcp/
Configuring Claude Code... OK
Configuring Windsurf... OK
Installing skills... OK
Done!

$ engrammic install -y --tool=claude --endpoint=http://localhost:8000/mcp
Configuring Claude Code... OK
Done!
```

### Selfhost Non-Interactive

```bash
$ engrammic selfhost -y --tier=cloud \
    --llm=openai --embedding=openai --reranker=none \
    --port=8000

Generating docker-compose.yml... OK
Required env vars: OPENAI_API_KEY
Starting containers... OK
Waiting for health... OK
Endpoint: http://localhost:8000/mcp
```

## State & File Management

### State Directory

```
~/.engrammic/
├── state.json           # Installation state (PIDs, containers, harnesses)
├── .env                 # Selfhost credentials (gitignored)
├── docker-compose.yml   # Generated compose file
├── data/                # Postgres data, vector store
│   ├── postgres/
│   └── qdrant/
└── backups/             # Config backups before modification
    ├── 2026-06-22T19:00:00_claude_settings.json
    └── 2026-06-22T19:00:00_windsurf_mcp_config.json
```

### Config Modification Rules

1. **Always backup first** — copy to `~/.engrammic/backups/` with timestamp
2. **Merge, don't overwrite** — only touch the `engrammic` entry, preserve user's other MCP servers
3. **Atomic writes** — write to `.tmp`, rename to target
4. **Clean removal** — remove only our entry, leave empty container if user had nothing else

### State File (`state.json`)

```json
{
  "version": 1,
  "config_version": 1,
  "last_updated": "2026-06-22T19:15:00Z",
  "server": {
    "container_id": "abc123...",
    "port": 8000,
    "endpoint": "http://localhost:8000/mcp",
    "started_at": "2026-06-22T19:10:00Z"
  },
  "harnesses": {
    "claude": {
      "installed_at": "2026-06-22T19:15:00Z",
      "config_path": "/home/user/.claude/settings.json",
      "endpoint": "http://localhost:8000/mcp"
    }
  }
}
```

## Dependencies

```
github.com/spf13/cobra              # CLI framework
github.com/charmbracelet/huh        # Wizard forms
github.com/charmbracelet/lipgloss   # Styled output
github.com/charmbracelet/bubbles    # Spinners, progress
gopkg.in/yaml.v3                    # YAML config (already added)
github.com/pelletier/go-toml/v2     # TOML config (already added)
```

## Package Structure (extends existing)

```
installer-go/
├── cmd/engrammic/
│   └── main.go                    # Cobra root command
├── internal/
│   ├── cli/                       # Command handlers
│   │   ├── root.go                # Root command, global flags
│   │   ├── install.go             # install command
│   │   ├── status.go              # status command
│   │   ├── doctor.go              # doctor command
│   │   ├── remove.go              # remove command
│   │   ├── selfhost.go            # selfhost command group
│   │   ├── license.go             # license command
│   │   ├── skills.go              # skills command
│   │   └── version.go             # version command
│   ├── wizard/                    # Wizard flows
│   │   ├── wizard.go              # Wizard runner (step machine)
│   │   ├── install.go             # Cloud install wizard
│   │   ├── selfhost.go            # Selfhost wizard
│   │   ├── remove.go              # Remove wizard
│   │   └── forms.go               # Shared form builders
│   ├── ui/                        # Presentation layer
│   │   ├── theme.go               # Lipgloss theme/colors
│   │   ├── banner.go              # ASCII banner
│   │   ├── output.go              # Styled print helpers
│   │   ├── progress.go            # Execution progress display
│   │   └── plain.go               # Plain terminal fallback
│   ├── core/                      # Business logic (DONE)
│   │   ├── providers.go
│   │   ├── harness.go
│   │   ├── skills.go
│   │   ├── state.go
│   │   ├── ports.go
│   │   └── migrate.go
│   └── platform/
│       ├── paths.go               # Config paths per OS
│       ├── detect.go              # Editor detection
│       └── terminal.go            # Terminal capability detection
```

## Success Criteria

1. `curl ... | sh` downloads binary and launches wizard
2. Cloud install wizard configures selected editors in <30 seconds
3. Selfhost wizard deploys working docker-compose stack
4. `engrammic status` shows accurate installation state
5. `engrammic doctor` catches common issues (port conflicts, missing docker, bad configs)
6. `engrammic remove` cleanly removes only Engrammic entries
7. Plain terminal mode works in SSH/CI environments
8. Non-interactive mode (`-y`) works for automation
