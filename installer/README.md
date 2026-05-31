# Engrammic MCP Installer

Scripts and landing page for `get.engrammic.ai`.

## Files

- `install.sh` - macOS/Linux installer
- `install.ps1` - Windows PowerShell installer  
- `index.html` - Landing page with copy-paste commands
- `Dockerfile` - nginx container for Cloud Run
- `nginx.conf` - serves scripts as text/plain

## Skills

The installer also offers to install the 21 open-source Engrammic skills from
the public `engrammic-ai/skills` repo. During `install` it prompts after
writing MCP config (opt-out, default yes) and lets you choose destinations:

- `~/.claude/skills/` (Claude Code, native)
- `~/.agents/skills/` (cross-harness: Codex, Gemini CLI, Cursor, Pi Agents)
- `./.agents/skills/` (project-local, current directory)

`update` refreshes skills in any destination that already has them.
`uninstall` removes them. `status` shows per-destination counts.

## Deployment (Cloud Run)

```bash
cd installer

# Build and push
gcloud builds submit --tag gcr.io/engrammic/get-engrammic

# Deploy
gcloud run deploy get-engrammic \
  --image gcr.io/engrammic/get-engrammic \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated

# Map custom domain
gcloud run domain-mappings create \
  --service get-engrammic \
  --domain get.engrammic.ai \
  --region us-central1
```

Then add DNS A/AAAA records per gcloud output.

## PyPI

The `install` command is built into `engrammic-mcp`:

```bash
uvx engrammic-mcp install
```

## Usage

```bash
# Full install (MCP + skills) - macOS/Linux
curl -fsSL https://get.engrammic.ai | bash

# Full install - Windows PowerShell
irm https://get.engrammic.ai/install.ps1 | iex

# Skills only (no MCP config changes) - macOS/Linux
curl -fsSL https://get.engrammic.ai/skills | bash

# Skills only - Windows PowerShell
irm https://get.engrammic.ai/install-skills.ps1 | iex

# Via PyPI
uvx engrammic-mcp install
```

## Endpoints

| Path | Description |
|------|-------------|
| `/` | Full installer (MCP + skills) |
| `/skills` | Skills-only installer |
| `/install.sh` | Direct link to Unix installer |
| `/install.ps1` | Direct link to Windows installer |
| `/install-skills.sh` | Direct link to Unix skills installer |
| `/install-skills.ps1` | Direct link to Windows skills installer |
