# Engrammic MCP

Persistent, structured memory for AI agents.

Most agent memory is a bag of text chunks. Engrammic organizes memory into cognitive layers: observations become claims, claims become facts, facts become beliefs. Your agent doesn't just recall what happened; it knows what it learned and why it believes what it believes.

## Quickstart

```bash
uvx engrammic-mcp
```

Set your API key:

```bash
export ENGRAMMIC_API_KEY=eng_xxx  # join waitlist at engrammic.ai
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

## What It Does

Store something:
> "Remember that the user prefers dark mode"

Recall it later:
> "What do I know about user preferences?"

Trace why you believe something:
> "Why do I think the user prefers dark mode?"

## Tools

| Tool | Purpose |
|------|---------|
| `remember` | Store observations (no evidence needed) |
| `learn` | Store claims with evidence |
| `believe` | Form commitments grounded in facts |
| `recall` | Search and retrieve context |
| `link` | Connect related concepts |
| `trace` | Query provenance chains |
| `hypothesize` | Form tentative beliefs |
| `commit` | Crystallize hypotheses to commitments |

## Self-Hosting

For local/offline usage without an API key, see [engrammic-engine](https://github.com/engrammic-ai/engine).

## Learn More

- [EAG Paradigm](https://github.com/engrammic-ai/primitives/blob/main/docs/README.md) - the cognitive architecture
- [Skills](https://github.com/engrammic-ai/skills) - workflow guidance for Claude Code, Codex, Cursor, Gemini CLI

## License

Apache 2.0
