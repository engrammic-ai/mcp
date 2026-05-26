# Configuration

## Supported Harnesses

| Harness | Config Path |
|---------|-------------|
| Claude Code | `~/.claude/settings.json` |
| Cursor | `~/.cursor/mcp.json` |
| Windsurf | `~/.windsurf/mcp.json` |
| Gemini CLI | `~/.gemini/settings.json` |

## Manual Configuration

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

## Skills Paths

- Claude Code: `~/.claude/skills/`
- Cross-harness: `~/.agents/skills/`
- Project-local: `./.agents/skills/`
