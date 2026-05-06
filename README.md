# Delta Prime MCP Server

MCP server for [Delta Prime](https://deltaprime.ai) context management. Connects AI agents to your Delta Prime workspace.

## Installation

```bash
pip install delta-prime-mcp
```

## Configuration

Set your API key:

```bash
export DELTA_PRIME_API_KEY=dp_xxx
```

Or use OAuth (opens browser on first use):

```bash
delta-prime-mcp login
```

## Usage with Claude Desktop

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "delta-prime": {
      "command": "delta-prime-mcp",
      "env": {
        "DELTA_PRIME_API_KEY": "dp_xxx"
      }
    }
  }
}
```

## Self-Hosting

Point to your own backend:

```bash
export DELTA_PRIME_BACKEND_URL=http://localhost:8000
```

## License

Apache 2.0
