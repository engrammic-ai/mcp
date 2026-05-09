# Engrammic MCP Registry Ready

**Goal:** Prepare the MCP client for publishing to the MCP registry with Engrammic branding, complete tool surface, and basic DX.

## Scope

### 1. Rename delta-prime to engrammic

**Package identity:**
- PyPI package: `engrammic-mcp`
- Python module: `engrammic_mcp`
- CLI command: `engrammic-mcp`
- MCP server name: `engrammic`

**Environment variables:**
- `ENGRAMMIC_API_KEY` (was `DELTA_PRIME_API_KEY`)
- `ENGRAMMIC_BACKEND_URL` (was `DELTA_PRIME_BACKEND_URL`)
- `ENGRAMMIC_CREDENTIALS_PATH` (was `DELTA_PRIME_CREDENTIALS_PATH`)

**File paths:**
- Credentials: `~/.engrammic/credentials.json`
- Source: `src/engrammic_mcp/`

**Documentation updates:**
- README.md
- .env.example
- pyproject.toml metadata

### 2. Complete tool surface

Add 6 missing tools to match backend:

| Tool | Purpose |
|------|---------|
| `context_belief_state` | Query live session WorkingHypotheses with contradiction detection |
| `context_update_belief` | Mutate a WorkingHypothesis in-place (confidence, evidence, status) |
| `context_crystallize` | Promote WorkingHypotheses to Commitments (wisdom layer) |
| `context_accept_belief` | Accept a ProposedBelief, convert to Belief |
| `context_reject_belief` | Reject a ProposedBelief with optional reason |
| `context_skills` | Read-only skill registry access (list, get, search) |

Each tool follows the existing pattern:
- Thin proxy to backend REST endpoint
- Singleton client reuse
- Error sanitization

### 3. CLI commands

**Login command:**
```bash
engrammic-mcp login
```
- Starts local HTTP server on ephemeral port (e.g., 18234)
- Opens browser to `{backend_url}/v1/oauth/authorize?redirect_uri=http://localhost:{port}/callback`
- Receives callback with auth code, exchanges for tokens via `POST {backend_url}/v1/oauth/token`
- Stores tokens to `~/.engrammic/credentials.json` with 600 permissions
- Prints success message with org name
- Shuts down local server after callback received (or 120s timeout)

**Version/help:**
```bash
engrammic-mcp --version  # prints version
engrammic-mcp --help     # prints usage
engrammic-mcp            # runs MCP server (default)
```

**CLI structure:**
- `cli.py` contains argument parsing and command dispatch
- `__main__.py` imports and calls `cli.main()`
- Default (no args) runs the MCP server via `server.py`

**Startup health check:**
- On server start, call `context_admin(action="whoami")` 
- On failure: log warning to stderr with error message, continue serving
- On success: log "Connected as {user} in {org}" to stderr

### 4. File structure after changes

```
engrammic-mcp/
├── src/
│   └── engrammic_mcp/
│       ├── __init__.py
│       ├── __main__.py
│       ├── cli.py              # NEW: login, version, help commands
│       ├── server.py
│       ├── client.py
│       ├── config.py
│       ├── credentials.py
│       ├── errors.py
│       └── tools/
│           ├── __init__.py
│           ├── context_store.py
│           ├── context_recall.py
│           ├── context_link.py
│           ├── context_admin.py
│           ├── context_belief_state.py   # NEW
│           ├── context_update_belief.py  # NEW
│           ├── context_crystallize.py    # NEW
│           ├── context_accept_belief.py  # NEW
│           ├── context_reject_belief.py  # NEW
│           └── context_skills.py         # NEW
├── tests/
│   ├── test_cli.py             # NEW
│   ├── test_client.py
│   ├── test_credentials.py
│   ├── test_errors.py
│   └── test_tools.py           # extend with new tools
├── pyproject.toml
├── README.md
└── ...
```

## Out of scope

- OAuth provider integration (just the client flow, backend handles OAuth)
- PyPI publishing automation
- MCP registry submission (manual after this work)
- Backwards compatibility with delta-prime naming

## Done criteria

- [ ] All references to "delta-prime" replaced with "engrammic"
- [ ] 10 tools exposed (4 existing + 6 new)
- [ ] `engrammic-mcp login` works end-to-end
- [ ] `engrammic-mcp --version` and `--help` work
- [ ] Startup health check logs connection status
- [ ] All tests pass
- [ ] README updated with new branding and commands
