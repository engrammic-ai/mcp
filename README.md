# Engrammic MCP

Persistent, structured memory for AI agents.

Most agent memory is a bag of text chunks. Engrammic organizes memory into cognitive layers: observations become claims, claims become facts, facts become beliefs. Your agent doesn't just recall what happened; it knows what it learned and why it believes what it believes.

## Quickstart

Install with one command:

```bash
curl -fsSL https://get.engrammic.ai/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://get.engrammic.ai/install.ps1 | iex
```

The installer detects your agent harnesses (Claude Code, Cursor, Codex, Gemini CLI, and more), wires Engrammic in as an MCP server, and optionally installs the Engrammic skills. Run `engrammic-mcp login` to authenticate after installation.

See [Getting Started](docs/GETTING-STARTED.md) after installation.

To configure a harness by hand, add the MCP server directly:

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
| `reason` | Record explicit reasoning steps |
| `reflect` | Store meta-observations |
| `hypothesize` | Form tentative beliefs |
| `revise` | Update tentative hypotheses |
| `commit` | Crystallize hypotheses to commitments |
| `accept` | Ratify system-synthesized beliefs |
| `reject` | Reject system-synthesized beliefs |
| `forget` | Request node deletion |
| `dismiss` | Dismiss contradiction markers |
| `patterns` | Discover workflow templates |
| `tick` | Lightweight engagement check |

## Hosted vs Local

The default setup connects to `beta.engrammic.ai`. This is the right choice for most users.

Use the local engine ([engrammic-engine](https://github.com/engrammic-ai/engine)) if you:

- Need fully offline or air-gapped operation
- Cannot send data to external services
- Are running a self-hosted production deployment

The local engine is a drop-in replacement. Point your harness config at `http://localhost:PORT/mcp/` instead of the hosted URL.

## Self-Hosting

For fully local, offline usage, see [engrammic-engine](https://github.com/engrammic-ai/engine).

## Learn More

- [EAG Paradigm](https://github.com/engrammic-ai/primitives/blob/main/docs/README.md) - the cognitive architecture
- [Skills](https://github.com/engrammic-ai/skills) - workflow guidance for Claude Code, Codex, Cursor, Gemini CLI

## License

Apache 2.0
