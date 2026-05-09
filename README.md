# Engrammic MCP Server

MCP server for [Engrammic](https://engrammic.com) context management. Connects AI agents to your Engrammic workspace.

## Installation

```bash
pip install engrammic-mcp
```

## Configuration

Set your API key:

```bash
export ENGRAMMIC_API_KEY=eng_xxx
```

Or use OAuth (opens browser on first use):

```bash
engrammic-mcp login
```

## Usage with Claude Desktop

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

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

Point to your own backend:

```bash
export ENGRAMMIC_BACKEND_URL=http://localhost:8000
```

## Available Tools

| Tool | Description |
|------|-------------|
| `context_store` | Store memories, knowledge, decisions, reasoning |
| `context_recall` | Search and retrieve context |
| `context_link` | Create relationships between nodes |
| `context_admin` | Usage info, provenance, history |
| `context_belief_state` | Query active hypotheses |
| `context_update_belief` | Update hypothesis confidence |
| `context_crystallize` | Promote hypotheses to commitments |
| `context_accept_belief` | Accept proposed beliefs |
| `context_reject_belief` | Reject proposed beliefs |
| `context_skills` | List and search skills |

## CLI Commands

```bash
engrammic-mcp --version   # Show version
engrammic-mcp --help      # Show help
engrammic-mcp login       # Authenticate with Engrammic
engrammic-mcp             # Run MCP server (default)
engrammic-mcp serve       # Run MCP server (explicit)
```

## License

Apache 2.0
