# Verb-Based Tool Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align mcp-client's MCP tool surface with context-service's verb-based API (remember, learn, believe, etc.) for OSS release.

**Architecture:** Replace the current `context_*_tool` naming with clean verb names. Each verb tool wraps the HTTP client and calls the appropriate backend endpoint. Internal-only tools (admin, accept/reject belief, belief_state) keep `context_` prefix.

**Tech Stack:** Python 3.13, FastMCP, httpx

**Backend Note:** The backend endpoints (`/v1/context/store`, `/v1/context/recall`, etc.) that this client calls are assumed to exist. If they don't, add REST routes to context-service separately.

---

## File Structure

```
src/engrammic_mcp/
  server.py           # MODIFY: register verb tools, update instructions
  tools/
    __init__.py       # MODIFY: export new modules
    remember.py       # CREATE: remember tool
    learn.py          # CREATE: learn tool  
    believe.py        # CREATE: believe tool
    recall.py         # CREATE: recall tool (from context_recall)
    trace.py          # CREATE: trace tool
    link.py           # CREATE: link tool (from context_link)
    reason.py         # CREATE: reason tool
    reflect.py        # CREATE: reflect tool
    hypothesize.py    # CREATE: hypothesize tool
    revise.py         # CREATE: revise tool (from context_update_belief)
    commit.py         # CREATE: commit tool (from context_crystallize)
    patterns.py       # CREATE: patterns tool (from context_skills)
    context_admin.py     # KEEP: internal-only
    context_accept_belief.py  # KEEP: internal-only
    context_reject_belief.py  # KEEP: internal-only
    context_belief_state.py   # KEEP: internal-only
    context_store.py     # DELETE: replaced by verb tools
    context_recall.py    # DELETE: replaced by recall.py
    context_link.py      # DELETE: replaced by link.py
    context_crystallize.py  # DELETE: replaced by commit.py
    context_update_belief.py # DELETE: replaced by revise.py
    context_skills.py    # DELETE: replaced by patterns.py
tests/
  conftest.py         # MODIFY: fix UP043 lint warning
  test_tools.py       # MODIFY: update for new tool names
```

---

### Task 1: Create remember tool

**Files:**
- Create: `src/engrammic_mcp/tools/remember.py`
- Test: `tests/test_tools.py`

- [ ] **Step 1: Write the failing test**

Add to `tests/test_tools.py`:

```python
@pytest.mark.asyncio
async def test_remember_basic(mock_client: None) -> None:
    """Test remember stores observation."""
    from engrammic_mcp.tools import remember

    result = await remember.remember(content="user prefers dark mode")
    assert result["node_id"] == "test-node-id"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest tests/test_tools.py::test_remember_basic -v`
Expected: FAIL with "cannot import name 'remember'"

- [ ] **Step 3: Create remember.py**

```python
"""MCP tool: remember - Store an observation."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def remember(
    content: str,
    tags: list[str] | None = None,
    decay: str = "standard",
) -> dict[str, Any]:
    """Store an observation to memory layer.
    
    Args:
        content: What to remember.
        tags: Optional categorization tags.
        decay: How long to keep: ephemeral|standard|durable|permanent.
    
    Returns:
        {node_id, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "remember",
        "content": content,
    }
    if tags:
        payload["tags"] = tags
    if decay != "standard":
        payload["decay_class"] = decay

    return await client.post("/v1/context/store", payload)
```

- [ ] **Step 4: Update tools/__init__.py export**

Add to `src/engrammic_mcp/tools/__init__.py`:

```python
from engrammic_mcp.tools import remember
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest tests/test_tools.py::test_remember_basic -v`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/remember.py tests/test_tools.py
git commit -m "feat: add remember tool for memory layer"
```

---

### Task 2: Create learn tool

**Files:**
- Create: `src/engrammic_mcp/tools/learn.py`
- Modify: `src/engrammic_mcp/tools/__init__.py`
- Test: `tests/test_tools.py`

- [ ] **Step 1: Write the failing test**

Add to `tests/test_tools.py`:

```python
@pytest.mark.asyncio
async def test_learn_with_evidence(mock_client: None) -> None:
    """Test learn requires evidence."""
    from engrammic_mcp.tools import learn

    result = await learn.learn(
        claim="Python 3.13 supports JIT",
        evidence=["https://docs.python.org/3.13/whatsnew/3.13.html"],
        source="document",
    )
    assert result["node_id"] == "test-node-id"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest tests/test_tools.py::test_learn_with_evidence -v`
Expected: FAIL

- [ ] **Step 3: Create learn.py**

```python
"""MCP tool: learn - Record a claim with evidence."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def learn(
    claim: str,
    evidence: list[str],
    source: str,
    confidence: float = 0.8,
    tags: list[str] | None = None,
    source_tier: str | None = None,
) -> dict[str, Any]:
    """Record something you learned with evidence.
    
    Args:
        claim: What you learned.
        evidence: REQUIRED. References: node:<uuid> or URI.
        source: Source type: document|user|external|agent.
        confidence: 0.0-1.0 (default 0.8).
        tags: Optional categorization.
        source_tier: Quality tier hint: authoritative|validated|community|unknown.
    
    Returns:
        {node_id, evidence_status, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "assert",
        "content": claim,
        "claims": [{"claim": claim, "evidence": evidence, "source": source}],
    }
    if confidence != 0.8:
        payload["confidence"] = confidence
    if tags:
        payload["tags"] = tags
    if source_tier:
        payload["source_tier"] = source_tier

    return await client.post("/v1/context/store", payload)
```

- [ ] **Step 4: Update tools/__init__.py**

Add: `from engrammic_mcp.tools import learn`

- [ ] **Step 5: Run test to verify it passes**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest tests/test_tools.py::test_learn_with_evidence -v`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/learn.py src/engrammic_mcp/tools/__init__.py tests/test_tools.py
git commit -m "feat: add learn tool for knowledge layer"
```

---

### Task 3: Create believe tool

**Files:**
- Create: `src/engrammic_mcp/tools/believe.py`
- Test: `tests/test_tools.py`

- [ ] **Step 1: Write the failing test**

```python
@pytest.mark.asyncio
async def test_believe_with_about(mock_client: None) -> None:
    """Test believe requires about nodes."""
    from engrammic_mcp.tools import believe

    result = await believe.believe(
        belief="FastAPI is the best framework for this project",
        about=["node:abc-123"],
    )
    assert result["node_id"] == "test-node-id"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest tests/test_tools.py::test_believe_with_about -v`

- [ ] **Step 3: Create believe.py**

```python
"""MCP tool: believe - Declare a commitment."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def believe(
    belief: str,
    about: list[str],
    confidence: float = 0.8,
    reasoning: str | None = None,
) -> dict[str, Any]:
    """Declare a belief as a commitment.
    
    Args:
        belief: What you believe.
        about: REQUIRED. Node IDs this belief concerns.
        confidence: 0.0-1.0 (default 0.8).
        reasoning: Why you believe this.
    
    Returns:
        {node_id, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "commit",
        "content": belief,
        "about": about,
    }
    if confidence != 0.8:
        payload["confidence"] = confidence
    if reasoning:
        payload["reasoning"] = reasoning

    return await client.post("/v1/context/store", payload)
```

- [ ] **Step 4: Update tools/__init__.py**

Add: `from engrammic_mcp.tools import believe`

- [ ] **Step 5: Run test, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
uv run pytest tests/test_tools.py::test_believe_with_about -v
git add src/engrammic_mcp/tools/believe.py src/engrammic_mcp/tools/__init__.py tests/test_tools.py
git commit -m "feat: add believe tool for wisdom layer"
```

---

### Task 4: Create recall tool

**Files:**
- Create: `src/engrammic_mcp/tools/recall.py`
- Delete: `src/engrammic_mcp/tools/context_recall.py` (after migration)

- [ ] **Step 1: Write the failing test**

```python
@pytest.mark.asyncio
async def test_recall_by_query(mock_client: None) -> None:
    """Test recall searches by query."""
    from engrammic_mcp.tools import recall

    result = await recall.recall(query="user preferences")
    assert "nodes" in result or "results" in result
```

- [ ] **Step 2: Create recall.py**

```python
"""MCP tool: recall - Retrieve knowledge."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def recall(
    query: str | None = None,
    node_ids: list[str] | None = None,
    depth: int = 0,
    layers: list[str] | None = None,
    top_k: int = 10,
    as_of: str | None = None,
) -> dict[str, Any]:
    """Retrieve knowledge by search or node ID.
    
    Args:
        query: Search query.
        node_ids: Specific nodes to fetch.
        depth: Graph traversal depth.
        layers: Filter by layers: memory|knowledge|wisdom|intelligence.
        top_k: Max results (default 10).
        as_of: Time-travel query (ISO timestamp).
    
    Returns:
        {nodes: [...]}
    """
    client = _get_client()
    payload: dict[str, Any] = {}

    if query:
        payload["query"] = query
    if node_ids:
        payload["node_ids"] = node_ids
    if depth > 0:
        payload["depth"] = depth
    if layers:
        payload["layers"] = layers
    if top_k != 10:
        payload["top_k"] = top_k
    if as_of:
        payload["as_of"] = as_of

    return await client.post("/v1/context/recall", payload)
```

- [ ] **Step 3: Update __init__.py, run test, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
uv run pytest tests/test_tools.py::test_recall_by_query -v
git add src/engrammic_mcp/tools/recall.py
git commit -m "feat: add recall tool for retrieval"
```

---

### Task 5: Create trace tool

**Files:**
- Create: `src/engrammic_mcp/tools/trace.py`

- [ ] **Step 1: Write the failing test**

```python
@pytest.mark.asyncio
async def test_trace_provenance(mock_client: None) -> None:
    """Test trace returns provenance chain."""
    from engrammic_mcp.tools import trace

    result = await trace.trace(node_id="node-abc-123")
    assert "chain" in result or "error" not in result
```

- [ ] **Step 2: Create trace.py**

```python
"""MCP tool: trace - Query provenance."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def trace(node_id: str) -> dict[str, Any]:
    """Trace provenance of a belief back to sources.
    
    Args:
        node_id: Node to trace.
    
    Returns:
        {chain: [...], root_sources: [...]}
    """
    client = _get_client()
    return await client.get(f"/v1/context/trace/{node_id}")
```

- [ ] **Step 3: Update __init__.py, run test, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/trace.py
git commit -m "feat: add trace tool for provenance"
```

---

### Task 6: Create link tool

**Files:**
- Create: `src/engrammic_mcp/tools/link.py`
- Delete: `src/engrammic_mcp/tools/context_link.py`

- [ ] **Step 1: Create link.py (copy from context_link.py, rename function)**

```python
"""MCP tool: link - Create typed relationships."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def link(
    source_id: str,
    target_id: str,
    relation: str,
    metadata: dict[str, Any] | None = None,
    weight: float | None = None,
) -> dict[str, Any]:
    """Create a typed relationship between nodes.
    
    Args:
        source_id: Source node.
        target_id: Target node.
        relation: Relationship type.
        metadata: Optional edge metadata.
        weight: Optional edge weight.
    
    Returns:
        {edge_id, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "source_id": source_id,
        "target_id": target_id,
        "relation": relation,
    }
    if metadata:
        payload["metadata"] = metadata
    if weight is not None:
        payload["weight"] = weight

    return await client.post("/v1/context/link", payload)
```

- [ ] **Step 2: Update __init__.py, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/link.py
git commit -m "feat: add link tool for relationships"
```

---

### Task 7: Create reason tool

**Files:**
- Create: `src/engrammic_mcp/tools/reason.py`

- [ ] **Step 1: Create reason.py**

```python
"""MCP tool: reason - Record reasoning steps."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def reason(
    problem: str,
    steps: list[dict[str, Any]],
    tags: list[str] | None = None,
) -> dict[str, Any]:
    """Record explicit reasoning steps.
    
    Args:
        problem: Problem being reasoned about.
        steps: Reasoning steps [{step, rationale, confidence?}].
        tags: Optional categorization.
    
    Returns:
        {node_id, step_ids, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "reason",
        "content": problem,
        "steps": steps,
    }
    if tags:
        payload["tags"] = tags

    return await client.post("/v1/context/store", payload)
```

- [ ] **Step 2: Update __init__.py, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/reason.py
git commit -m "feat: add reason tool for intelligence layer"
```

---

### Task 8: Create reflect tool

**Files:**
- Create: `src/engrammic_mcp/tools/reflect.py`

- [ ] **Step 1: Create reflect.py**

```python
"""MCP tool: reflect - Record meta-observations."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def reflect(
    observation: str,
    about: list[str] | None = None,
    observation_type: str | None = None,
) -> dict[str, Any]:
    """Record a meta-observation about your knowledge.
    
    Args:
        observation: What you observed about your knowledge/reasoning.
        about: Node IDs this reflection concerns.
        observation_type: Type: contradiction|uncertainty|update|correction.
    
    Returns:
        {node_id, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "reflect",
        "content": observation,
    }
    if about:
        payload["about"] = about
    if observation_type:
        payload["observation_type"] = observation_type

    return await client.post("/v1/context/store", payload)
```

- [ ] **Step 2: Update __init__.py, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/reflect.py
git commit -m "feat: add reflect tool for meta-observations"
```

---

### Task 9: Create hypothesize tool

**Files:**
- Create: `src/engrammic_mcp/tools/hypothesize.py`

- [ ] **Step 1: Create hypothesize.py**

```python
"""MCP tool: hypothesize - Form tentative beliefs."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def hypothesize(
    hypothesis: str,
    about: list[str],
    confidence: float = 0.8,
    session_id: str | None = None,
) -> dict[str, Any]:
    """Form a tentative belief during reasoning.
    
    Args:
        hypothesis: Tentative belief.
        about: REQUIRED. Node IDs this concerns.
        confidence: 0.0-1.0 (default 0.8).
        session_id: Optional session override.
    
    Returns:
        {belief_id, session_id, potential_conflicts, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "hypothesis": hypothesis,
        "about": about,
    }
    if confidence != 0.8:
        payload["confidence"] = confidence
    if session_id:
        payload["session_id"] = session_id

    return await client.post("/v1/context/hypothesize", payload)
```

- [ ] **Step 2: Update __init__.py, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/hypothesize.py
git commit -m "feat: add hypothesize tool for tentative beliefs"
```

---

### Task 10: Create revise and commit tools

**Files:**
- Create: `src/engrammic_mcp/tools/revise.py`
- Create: `src/engrammic_mcp/tools/commit.py`
- Delete: `src/engrammic_mcp/tools/context_update_belief.py`
- Delete: `src/engrammic_mcp/tools/context_crystallize.py`

- [ ] **Step 1: Create revise.py**

```python
"""MCP tool: revise - Update tentative hypotheses."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def revise(
    belief_id: str,
    confidence: float,
    reason: str,
    content: str | None = None,
) -> dict[str, Any]:
    """Update a WorkingHypothesis.
    
    Args:
        belief_id: Hypothesis to update.
        confidence: New confidence 0.0-1.0.
        reason: Why the update.
        content: Optional new content.
    
    Returns:
        {belief_id, updated_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "belief_id": belief_id,
        "confidence": confidence,
        "reason": reason,
    }
    if content:
        payload["content"] = content

    return await client.post("/v1/context/revise", payload)
```

- [ ] **Step 2: Create commit.py**

```python
"""MCP tool: commit - Crystallize hypotheses to commitments."""

from typing import Any

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def commit(
    belief_ids: list[str],
    reason: str | None = None,
) -> dict[str, Any]:
    """Promote tentative hypotheses to permanent commitments.
    
    Args:
        belief_ids: Hypotheses to crystallize.
        reason: Why committing now.
    
    Returns:
        {committed: [...], created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {"belief_ids": belief_ids}
    if reason:
        payload["reason"] = reason

    return await client.post("/v1/context/commit", payload)
```

- [ ] **Step 3: Update __init__.py, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/revise.py src/engrammic_mcp/tools/commit.py
git commit -m "feat: add revise and commit tools for hypothesis flow"
```

---

### Task 11: Create patterns tool

**Files:**
- Create: `src/engrammic_mcp/tools/patterns.py`
- Delete: `src/engrammic_mcp/tools/context_skills.py`

- [ ] **Step 1: Create patterns.py**

```python
"""MCP tool: patterns - Workflow templates and skills."""

from typing import Any, Literal

from engrammic_mcp.client import EngrammicClient
from engrammic_mcp.config import get_settings

_client: EngrammicClient | None = None


def _get_client() -> EngrammicClient:
    global _client
    if _client is None:
        _client = EngrammicClient(get_settings())
    return _client


def reset_client() -> None:
    """Reset the singleton client. For testing only."""
    global _client
    _client = None


async def patterns(
    action: Literal["list", "get", "search"],
    name: str | None = None,
    query: str | None = None,
    namespace: str | None = None,
    limit: int = 50,
    offset: int = 0,
) -> dict[str, Any]:
    """Discover workflow templates.
    
    Args:
        action: list|get|search.
        name: Pattern name (for get).
        query: Search query (for search).
        namespace: Filter by namespace.
        limit: Max results.
        offset: Pagination offset.
    
    Returns:
        {patterns: [...]} or {pattern: {...}}
    """
    client = _get_client()
    payload: dict[str, Any] = {"action": action}
    if name:
        payload["name"] = name
    if query:
        payload["query"] = query
    if namespace:
        payload["namespace"] = namespace
    if limit != 50:
        payload["limit"] = limit
    if offset > 0:
        payload["offset"] = offset

    return await client.post("/v1/context/patterns", payload)
```

- [ ] **Step 2: Update __init__.py, commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/patterns.py
git commit -m "feat: add patterns tool for workflow templates"
```

---

### Task 12: Refactor server.py to use verb tools

**Files:**
- Modify: `src/engrammic_mcp/server.py`

- [ ] **Step 1: Update server.py imports and tool registration**

Replace entire `server.py`:

```python
"""FastMCP server for Engrammic."""

from typing import Any, Literal

from fastmcp import FastMCP

from engrammic_mcp.tools import (
    believe,
    commit,
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_reject_belief,
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


def create_server() -> FastMCP:
    """Create and configure the Engrammic MCP server."""
    mcp = FastMCP(
        name="engrammic",
        instructions=(
            "Engrammic: Epistemic memory for AI agents.\n\n"
            "Quick start:\n"
            "- remember: store observations\n"
            "- learn: record claims WITH evidence\n"
            "- believe: declare conclusions\n"
            "- recall: search your knowledge\n"
            "- trace: understand why you believe something\n"
            "- link: connect related knowledge\n\n"
            "Guidelines:\n"
            "- Always provide evidence when using learn\n"
            "- Reference existing nodes when forming beliefs\n"
            "- Use recall before storing to avoid duplicates\n\n"
            "Onboarding:\n"
            "- Call patterns(action='get', name='onboarding') for your workflow guide"
        ),
    )

    # Standard profile tools
    @mcp.tool()
    async def remember_tool(
        content: str,
        tags: list[str] | None = None,
        decay: str = "standard",
    ) -> dict[str, Any]:
        """Store an observation. No evidence required."""
        return await remember.remember(content, tags, decay)

    @mcp.tool()
    async def learn_tool(
        claim: str,
        evidence: list[str],
        source: str,
        confidence: float = 0.8,
        tags: list[str] | None = None,
        source_tier: str | None = None,
    ) -> dict[str, Any]:
        """Record something you learned with evidence."""
        return await learn.learn(claim, evidence, source, confidence, tags, source_tier)

    @mcp.tool()
    async def believe_tool(
        belief: str,
        about: list[str],
        confidence: float = 0.8,
        reasoning: str | None = None,
    ) -> dict[str, Any]:
        """Declare a belief as a commitment."""
        return await believe.believe(belief, about, confidence, reasoning)

    @mcp.tool()
    async def recall_tool(
        query: str | None = None,
        node_ids: list[str] | None = None,
        depth: int = 0,
        layers: list[str] | None = None,
        top_k: int = 10,
        as_of: str | None = None,
    ) -> dict[str, Any]:
        """Retrieve knowledge by search or node ID."""
        return await recall.recall(query, node_ids, depth, layers, top_k, as_of)

    @mcp.tool()
    async def trace_tool(node_id: str) -> dict[str, Any]:
        """Trace provenance of a belief back to sources."""
        return await trace.trace(node_id)

    @mcp.tool()
    async def link_tool(
        source_id: str,
        target_id: str,
        relation: str,
        metadata: dict[str, Any] | None = None,
        weight: float | None = None,
    ) -> dict[str, Any]:
        """Create a typed relationship between nodes."""
        return await link.link(source_id, target_id, relation, metadata, weight)

    # Reasoning profile tools
    @mcp.tool()
    async def reason_tool(
        problem: str,
        steps: list[dict[str, Any]],
        tags: list[str] | None = None,
    ) -> dict[str, Any]:
        """Record explicit reasoning steps."""
        return await reason.reason(problem, steps, tags)

    @mcp.tool()
    async def reflect_tool(
        observation: str,
        about: list[str] | None = None,
        observation_type: str | None = None,
    ) -> dict[str, Any]:
        """Record a meta-observation about your knowledge."""
        return await reflect.reflect(observation, about, observation_type)

    @mcp.tool()
    async def hypothesize_tool(
        hypothesis: str,
        about: list[str],
        confidence: float = 0.8,
        session_id: str | None = None,
    ) -> dict[str, Any]:
        """Form a tentative belief during reasoning."""
        return await hypothesize.hypothesize(hypothesis, about, confidence, session_id)

    @mcp.tool()
    async def revise_tool(
        belief_id: str,
        confidence: float,
        reason: str,
        content: str | None = None,
    ) -> dict[str, Any]:
        """Update a tentative hypothesis."""
        return await revise.revise(belief_id, confidence, reason, content)

    @mcp.tool()
    async def commit_tool(
        belief_ids: list[str],
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Promote hypotheses to permanent commitments."""
        return await commit.commit(belief_ids, reason)

    # Always available
    @mcp.tool()
    async def patterns_tool(
        action: Literal["list", "get", "search"],
        name: str | None = None,
        query: str | None = None,
        namespace: str | None = None,
        limit: int = 50,
        offset: int = 0,
    ) -> dict[str, Any]:
        """Discover workflow templates."""
        return await patterns.patterns(action, name, query, namespace, limit, offset)

    # Internal-only tools (keep context_ prefix)
    @mcp.tool()
    async def context_admin_tool(
        action: Literal["whoami", "usage", "provenance", "history"],
        node_id: str | None = None,
        since: str | None = None,
    ) -> dict[str, Any]:
        """Administrative operations."""
        return await context_admin.admin(action, node_id, since)

    @mcp.tool()
    async def context_belief_state_tool(
        session_id: str,
        about: list[str] | None = None,
    ) -> dict[str, Any]:
        """Query session's active WorkingHypotheses."""
        return await context_belief_state.belief_state(session_id, about)

    @mcp.tool()
    async def context_accept_belief_tool(
        belief_id: str,
        confidence: float | None = None,
    ) -> dict[str, Any]:
        """Accept a ProposedBelief."""
        return await context_accept_belief.accept_belief(belief_id, confidence)

    @mcp.tool()
    async def context_reject_belief_tool(
        belief_id: str,
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Reject a ProposedBelief."""
        return await context_reject_belief.reject_belief(belief_id, reason)

    return mcp
```

- [ ] **Step 2: Run tests to verify nothing broke**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest -v`

- [ ] **Step 3: Commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/server.py
git commit -m "refactor: server.py to use verb-based tool registration"
```

---

### Task 13: Update tools/__init__.py

**Files:**
- Modify: `src/engrammic_mcp/tools/__init__.py`

- [ ] **Step 1: Update exports**

```python
"""Engrammic MCP tools."""

from engrammic_mcp.tools import (
    believe,
    commit,
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_reject_belief,
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

__all__ = [
    "believe",
    "commit",
    "context_accept_belief",
    "context_admin",
    "context_belief_state",
    "context_reject_belief",
    "hypothesize",
    "learn",
    "link",
    "patterns",
    "reason",
    "recall",
    "reflect",
    "remember",
    "revise",
    "trace",
]
```

- [ ] **Step 2: Commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add src/engrammic_mcp/tools/__init__.py
git commit -m "refactor: update tools __init__ exports"
```

---

### Task 14: Delete deprecated tool files

**Files:**
- Delete: `src/engrammic_mcp/tools/context_store.py`
- Delete: `src/engrammic_mcp/tools/context_recall.py`
- Delete: `src/engrammic_mcp/tools/context_link.py`
- Delete: `src/engrammic_mcp/tools/context_crystallize.py`
- Delete: `src/engrammic_mcp/tools/context_update_belief.py`
- Delete: `src/engrammic_mcp/tools/context_skills.py`

- [ ] **Step 1: Remove deprecated files**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git rm src/engrammic_mcp/tools/context_store.py
git rm src/engrammic_mcp/tools/context_recall.py
git rm src/engrammic_mcp/tools/context_link.py
git rm src/engrammic_mcp/tools/context_crystallize.py
git rm src/engrammic_mcp/tools/context_update_belief.py
git rm src/engrammic_mcp/tools/context_skills.py
```

- [ ] **Step 2: Run tests to verify nothing depends on old files**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest -v`

- [ ] **Step 3: Commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git commit -m "chore: remove deprecated context_* tool files"
```

---

### Task 15: Update tests

**Files:**
- Modify: `tests/test_tools.py`
- Modify: `tests/conftest.py`

- [ ] **Step 1: Fix conftest.py lint warning**

Change line 11 from:
```python
def temp_credentials_dir() -> Generator[Path, None, None]:
```
to:
```python
def temp_credentials_dir() -> Generator[Path]:
```

- [ ] **Step 2: Update test_tools.py to use new tool names**

Update imports and test function names to reference the verb-based tools.

- [ ] **Step 3: Run full test suite**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run pytest -v`

- [ ] **Step 4: Run lint and type checks**

Run: `cd /home/novusedge/Projects/delta-prime/mcp-client && uv run ruff check . && uv run mypy src/`

- [ ] **Step 5: Commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add tests/
git commit -m "test: update tests for verb-based tool surface"
```

---

### Task 16: Final verification

- [ ] **Step 1: Run full check**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
uv run ruff check .
uv run mypy src/
uv run pytest -v
```

- [ ] **Step 2: Verify tool names match README**

Confirm server.py exposes: remember, learn, believe, recall, trace, link, reason, reflect, hypothesize, revise, commit, patterns

- [ ] **Step 3: Bump version to 0.3.0**

Edit `pyproject.toml`: `version = "0.3.0"`

- [ ] **Step 4: Final commit**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git add pyproject.toml
git commit -m "chore: bump version to 0.3.0 for verb-based tool surface"
```
