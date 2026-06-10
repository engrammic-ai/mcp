# Connector Submission Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all Claude connector directory submission blockers: tool annotations, privacy policy, version sync, and broken tests.

**Architecture:** Four independent fixes that can be done in any order. Tool annotations are mechanical edits to server.py. Privacy policy is new content in README.md. Version sync is a one-line fix. Test fixes remove dead imports.

**Tech Stack:** Python, FastMCP, pytest

---

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `src/engrammic_mcp/server.py` | Modify | Add tool annotations (title, readOnlyHint, destructiveHint) |
| `src/engrammic_mcp/__init__.py` | Modify | Sync version to 0.6.0 |
| `pyproject.toml` | Modify | Sync version to 0.6.0 |
| `README.md` | Modify | Add Privacy Policy section |
| `tests/test_tools.py` | Modify | Remove dead imports and test classes |

---

### Task 1: Fix Version Mismatch

**Files:**
- Modify: `src/engrammic_mcp/__init__.py:3`
- Modify: `pyproject.toml:3`

- [ ] **Step 1: Update __init__.py version**

```python
__version__ = "0.6.0"
```

- [ ] **Step 2: Update pyproject.toml version**

```toml
version = "0.6.0"
```

- [ ] **Step 3: Verify versions match**

Run: `grep -E "version|__version__" src/engrammic_mcp/__init__.py pyproject.toml`
Expected: Both show 0.6.0

- [ ] **Step 4: Commit**

```bash
git add src/engrammic_mcp/__init__.py pyproject.toml
git commit -m "fix: sync version to 0.6.0"
```

---

### Task 2: Fix Broken Test Imports

**Files:**
- Modify: `tests/test_tools.py:8-25` (imports)
- Modify: `tests/test_tools.py:28-46` (fixture)
- Modify: `tests/test_tools.py:50-72` (settings fixture)
- Remove: `tests/test_tools.py:75-114` (dead test classes)

The tests import 4 modules that don't exist: `context_admin`, `context_belief_state`, `context_accept_belief`, `context_reject_belief`. These modules were removed but the tests weren't updated.

- [ ] **Step 1: Remove dead imports**

Replace lines 8-25:

```python
from engrammic_mcp.tools import (
    believe,
    commit,
    hypothesize,
    learn,
    link,
    patterns,
    reason,
    recall,
    reflect,
    remember,
    revise,
    trace,
)
```

- [ ] **Step 2: Fix reset_clients fixture**

Replace the fixture (lines 28-46):

```python
@pytest.fixture(autouse=True)
def reset_clients() -> None:
    reset_http_client()
    remember.reset_client()
    learn.reset_client()
    believe.reset_client()
    recall.reset_client()
    trace.reset_client()
    link.reset_client()
    reason.reset_client()
    reflect.reset_client()
    hypothesize.reset_client()
    revise.reset_client()
    commit.reset_client()
    patterns.reset_client()
```

- [ ] **Step 3: Fix settings fixture monkeypatches**

Replace the settings fixture (lines 49-72):

```python
@pytest.fixture
def settings(temp_credentials_dir, monkeypatch) -> Settings:
    s = Settings(
        backend_url="https://api.test.com",
        api_key="test_key",
        credentials_path=temp_credentials_dir / "creds.json",
    )
    monkeypatch.setattr("engrammic_mcp.tools.remember.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.learn.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.believe.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.recall.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.trace.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.link.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.reason.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.reflect.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.hypothesize.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.revise.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.commit.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.patterns.get_settings", lambda: s)
    return s
```

- [ ] **Step 4: Remove dead test classes**

Delete these 4 test classes entirely (lines 75-114):
- `TestContextAdmin`
- `TestContextBeliefState`
- `TestContextAcceptBelief`
- `TestContextRejectBelief`

- [ ] **Step 5: Run tests to verify fix**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && python -m pytest tests/test_tools.py -v --collect-only`
Expected: No ImportError, all test classes collected

- [ ] **Step 6: Commit**

```bash
git add tests/test_tools.py
git commit -m "fix(tests): remove dead context_* imports and tests"
```

---

### Task 3: Add Tool Annotations

**Files:**
- Modify: `src/engrammic_mcp/server.py:1-10` (add import)
- Modify: `src/engrammic_mcp/server.py:53-274` (all 15 tool decorators)

Each tool needs:
- `title`: Human-readable name (no "_tool" suffix)
- `annotations`: ToolAnnotations with appropriate hints

**Annotation assignments:**
| Tool | title | readOnlyHint | destructiveHint |
|------|-------|--------------|-----------------|
| remember_tool | "Remember" | False | False |
| learn_tool | "Learn" | False | False |
| believe_tool | "Believe" | False | False |
| recall_tool | "Recall" | **True** | False |
| trace_tool | "Trace" | **True** | False |
| link_tool | "Link" | False | False |
| reason_tool | "Reason" | False | False |
| reflect_tool | "Reflect" | False | False |
| hypothesize_tool | "Hypothesize" | False | False |
| revise_tool | "Revise" | False | False |
| commit_tool | "Commit" | False | False |
| patterns_tool | "Patterns" | **True** | False |
| forget_tool | "Forget" | False | **True** |
| dismiss_tool | "Dismiss" | False | False |
| tick_tool | "Tick" | **True** | False |

- [ ] **Step 1: Add ToolAnnotations import**

Add to imports at top of server.py:

```python
from mcp.types import ToolAnnotations
```

- [ ] **Step 2: Annotate remember_tool**

```python
@mcp.tool(
    title="Remember",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def remember_tool(
```

- [ ] **Step 3: Annotate learn_tool**

```python
@mcp.tool(
    title="Learn",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def learn_tool(
```

- [ ] **Step 4: Annotate believe_tool**

```python
@mcp.tool(
    title="Believe",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def believe_tool(
```

- [ ] **Step 5: Annotate recall_tool (readOnly)**

```python
@mcp.tool(
    title="Recall",
    annotations=ToolAnnotations(readOnlyHint=True),
)
async def recall_tool(
```

- [ ] **Step 6: Annotate trace_tool (readOnly)**

```python
@mcp.tool(
    title="Trace",
    annotations=ToolAnnotations(readOnlyHint=True),
)
async def trace_tool(
```

- [ ] **Step 7: Annotate link_tool**

```python
@mcp.tool(
    title="Link",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def link_tool(
```

- [ ] **Step 8: Annotate reason_tool**

```python
@mcp.tool(
    title="Reason",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def reason_tool(
```

- [ ] **Step 9: Annotate reflect_tool**

```python
@mcp.tool(
    title="Reflect",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def reflect_tool(
```

- [ ] **Step 10: Annotate hypothesize_tool**

```python
@mcp.tool(
    title="Hypothesize",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def hypothesize_tool(
```

- [ ] **Step 11: Annotate revise_tool**

```python
@mcp.tool(
    title="Revise",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def revise_tool(
```

- [ ] **Step 12: Annotate commit_tool**

```python
@mcp.tool(
    title="Commit",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def commit_tool(
```

- [ ] **Step 13: Annotate patterns_tool (readOnly)**

```python
@mcp.tool(
    title="Patterns",
    annotations=ToolAnnotations(readOnlyHint=True),
)
async def patterns_tool(
```

- [ ] **Step 14: Annotate forget_tool (destructive)**

```python
@mcp.tool(
    title="Forget",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=True),
)
async def forget_tool(
```

- [ ] **Step 15: Annotate dismiss_tool**

```python
@mcp.tool(
    title="Dismiss",
    annotations=ToolAnnotations(readOnlyHint=False, destructiveHint=False),
)
async def dismiss_tool(
```

- [ ] **Step 16: Annotate tick_tool (readOnly)**

```python
@mcp.tool(
    title="Tick",
    annotations=ToolAnnotations(readOnlyHint=True),
)
async def tick_tool(
```

- [ ] **Step 17: Verify syntax**

Run: `python -c "from engrammic_mcp.server import create_server; print('OK')"`
Expected: OK

- [ ] **Step 18: Commit**

```bash
git add src/engrammic_mcp/server.py
git commit -m "feat: add tool annotations for connector submission"
```

---

### Task 4: Add Privacy Policy to README

**Files:**
- Modify: `README.md` (add section before License)

- [ ] **Step 1: Add Privacy Policy section**

Insert before the "## License" section (line 91):

```markdown
## Privacy Policy

Engrammic collects and processes the following data:

**Data Collected:**
- Memory content you explicitly store (observations, claims, beliefs, reasoning)
- Authentication tokens (stored locally at `~/.engrammic/credentials.json`)
- Session metadata for engagement tracking

**Data Usage:**
- Memory content is used to provide persistent context across agent sessions
- Tokens are used solely for API authentication
- No data is used for training or shared with third parties

**Data Storage:**
- **Hosted mode** (default): Data stored on Engrammic servers at `beta.engrammic.ai`
- **Self-hosted mode**: Data stored on your own infrastructure via [engrammic-engine](https://github.com/engrammic-ai/engine)
- Local credentials stored with restricted permissions (0600)

**Data Retention:**
- Memory persists until explicitly deleted via the `forget` tool
- Session data expires after 24 hours of inactivity
- Account deletion removes all associated data

**Your Rights:**
- Export your data via the recall API
- Delete specific memories via the forget tool
- Delete all data by contacting support

**Contact:**
For privacy inquiries: privacy@engrammic.ai

For the complete privacy policy, see: https://engrammic.ai/privacy

```

- [ ] **Step 2: Verify README renders**

Run: `head -120 README.md | tail -40`
Expected: Privacy Policy section visible

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add privacy policy section for connector submission"
```

---

### Task 5: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && python -m pytest tests/ -v`
Expected: All tests pass (no ImportError)

- [ ] **Step 2: Verify server loads**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && python -c "from engrammic_mcp.server import create_server; s = create_server(); print(f'Server: {s.name}, Tools: {len(s._tool_manager._tools)}')"`
Expected: Server: engrammic, Tools: 15

- [ ] **Step 3: Verify versions synced**

Run: `grep -E "^version|^__version__" src/engrammic_mcp/__init__.py pyproject.toml`
Expected: Both show 0.6.0

- [ ] **Step 4: Run linter**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && ruff check src/engrammic_mcp/server.py`
Expected: No errors

---

## Submission Checklist (Post-Implementation)

After completing all tasks, verify against submission requirements:

| Requirement | Status |
|-------------|--------|
| Tool annotations (title) | All 15 tools |
| Tool annotations (readOnlyHint) | recall, trace, patterns, tick |
| Tool annotations (destructiveHint) | forget |
| Privacy Policy in README | Present |
| Version consistency | 0.6.0 everywhere |
| Tests passing | No dead imports |

Next step: Run `claude plugin validate` to check for any remaining issues.
