# Troubleshooting

## OAuth fails or times out

- Check firewall isn't blocking localhost
- Ensure no other process on callback port
- Try again with browser devtools open

## MCP not connecting

- Verify config: `~/.claude/settings.json` or harness equivalent
- Restart editor after install
- Test endpoint: `curl https://beta.engrammic.ai/mcp/`

## Skills not showing

- Check: `ls ~/.claude/skills/engrammic-*`
- Run: `patterns(action: 'list')`
