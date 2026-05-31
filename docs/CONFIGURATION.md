# Configuration

## Supported Harnesses

Most harnesses use a JSON config file with an `mcpServers` map. Codex uses TOML, and
VS Code is registered via a one-click deep-link (its config is managed in-app).

| Harness | `--tool` id | Config Path | Method |
|---------|-------------|-------------|--------|
| Claude Code | `claude` | `~/.claude/settings.json` | file edit (JSON) |
| Cursor | `cursor` | `~/.cursor/mcp.json` | file edit (JSON) |
| Windsurf | `windsurf` | `~/.codeium/windsurf/mcp_config.json` | file edit (JSON) |
| Antigravity | `antigravity` | `~/.gemini/antigravity/mcp_config.json` | file edit (JSON) |
| Gemini CLI | `gemini` | `~/.gemini/settings.json` | file edit (JSON) |
| Pi Agents | `pi` | `~/.pi/agent/mcp.json` | file edit (JSON) |
| GitHub Copilot CLI | `copilot` | `~/.copilot/mcp-config.json` | file edit (JSON) |
| OpenAI Codex CLI | `codex` | `~/.codex/config.toml` | file edit (TOML) |
| VS Code (Copilot) | `vscode` | managed in-app | deep-link |

Only the `engrammic` entry is created or updated; other MCP servers and unrelated
config (including comments in `config.toml`) are preserved.

## Manual Configuration

Most harnesses (JSON):

```json
{
  "mcpServers": {
    "engrammic": {
      "type": "http",
      "url": "https://beta.engrammic.ai/mcp/"
    }
  }
}
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.engrammic]
url = "https://beta.engrammic.ai/mcp/"
```

VS Code: run `engrammic --tool vscode`, or open the install link it prints
(`vscode:mcp/install?...`) and approve the prompt. The server key is `servers`, not
`mcpServers`.

## Skills Paths

- Claude Code: `~/.claude/skills/`
- Cross-harness: `~/.agents/skills/`
- Project-local: `./.agents/skills/`
- Codex: `~/.codex/AGENTS.md` (and project `./AGENTS.md`)
- Codex / GitHub Copilot CLI / other AGENTS.md-aware tools: project `./AGENTS.md`
