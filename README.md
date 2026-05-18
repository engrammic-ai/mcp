# Engrammic MCP

Persistent memory for AI agents.

## Quickstart

1. Install from [MCP Registry](https://registry.mcp.io/engrammic) or `uvx engrammic-mcp`
2. Set `ENGRAMMIC_API_KEY` ([join the waitlist](https://engrammic.ai) for access)
3. Done

## Examples

Store something:
> "Remember that the user prefers dark mode"

Recall it later:
> "What do I know about user preferences?"

## Tools

| Tool | Purpose |
|------|---------|
| `remember` | Store observations to memory |
| `learn` | Store claims with evidence to knowledge |
| `believe` | Form commitments in wisdom |
| `recall` | Search and retrieve context |
| `link` | Connect related concepts |
| `trace` | Query provenance chains |
| `hypothesize` | Form tentative beliefs |
| `commit` | Crystallize hypotheses to commitments |

## Configuration

```bash
export ENGRAMMIC_API_KEY=eng_xxx
```

Or add to Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "engrammic": {
      "command": "engrammic-mcp",
      "env": {
        "ENGRAMMIC_API_KEY": "eng_xxx"
      }
    }
  }
}
```

## Self-Hosting

For local/offline usage, see [engrammic-engine](https://github.com/engrammic-ai/engine).

## Learn More

- [EAG Manifesto](https://github.com/engrammic-ai/primitives/blob/main/docs/manifesto.md) - the paradigm explained
- [EAG Concepts](docs/eag-concepts.md) - understand the memory model
- Using Claude Code? Copy [skills/](https://github.com/engrammic-ai/context-service/tree/main/skills) to `~/.claude/skills/` for EAG workflow guidance

## License

Apache 2.0
