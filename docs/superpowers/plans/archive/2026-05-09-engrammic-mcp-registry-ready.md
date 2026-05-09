# Engrammic MCP Registry Ready Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebrand delta-prime-mcp to engrammic-mcp, add 6 missing tools, and add CLI commands for MCP registry publishing.

**Architecture:** Thin MCP proxy to Engrammic backend REST API. Each tool is a standalone module with singleton client. CLI uses argparse with subcommands.

**Tech Stack:** Python 3.12+, FastMCP, httpx, pydantic-settings, structlog

---

## File Structure

```
src/engrammic_mcp/           # renamed from delta_prime_mcp
├── __init__.py              # update version, docstring
├── __main__.py              # delegate to cli.main()
├── cli.py                   # NEW: argparse, login, version, serve
├── server.py                # update imports, add health check
├── client.py                # rename class to EngrammicClient
├── config.py                # update env prefix, defaults
├── credentials.py           # update default path
├── errors.py                # update docstrings
└── tools/
    ├── __init__.py          # add new tool imports
    ├── context_store.py     # update imports
    ├── context_recall.py    # update imports
    ├── context_link.py      # update imports
    ├── context_admin.py     # update imports
    ├── context_belief_state.py   # NEW
    ├── context_update_belief.py  # NEW
    ├── context_crystallize.py    # NEW
    ├── context_accept_belief.py  # NEW
    ├── context_reject_belief.py  # NEW
    └── context_skills.py         # NEW

tests/
├── conftest.py              # update fixture imports
├── test_client.py           # update imports
├── test_credentials.py      # update imports
├── test_errors.py           # update imports
├── test_tools.py            # update imports, add new tool tests
└── test_cli.py              # NEW
```

---

### Task 1: Rename Package Directory

**Files:**
- Rename: `src/delta_prime_mcp/` to `src/engrammic_mcp/`

- [ ] **Step 1: Rename the source directory**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
git mv src/delta_prime_mcp src/engrammic_mcp
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "refactor: rename delta_prime_mcp to engrammic_mcp"
```

---

### Task 2: Update pyproject.toml

**Files:**
- Modify: `pyproject.toml`

- [ ] **Step 1: Update package metadata and paths**

Replace the entire pyproject.toml content:

```toml
[project]
name = "engrammic-mcp"
version = "0.2.0"
description = "MCP server for Engrammic context management"
readme = "README.md"
license = { text = "Apache-2.0" }
requires-python = ">=3.12"
authors = [{ name = "Engrammic", email = "hello@engrammic.com" }]
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3.12",
]
dependencies = [
    "fastmcp>=0.1.0",
    "httpx[http2]>=0.27.0",
    "pydantic-settings>=2.0.0",
    "structlog>=24.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0.0",
    "pytest-asyncio>=0.23.0",
    "pytest-httpx>=0.30.0",
    "ruff>=0.4.0",
    "mypy>=1.10.0",
]

[project.scripts]
engrammic-mcp = "engrammic_mcp.cli:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/engrammic_mcp"]

[tool.pytest.ini_options]
asyncio_mode = "auto"
asyncio_default_fixture_loop_scope = "function"
testpaths = ["tests"]

[tool.ruff]
target-version = "py312"
line-length = 100

[tool.ruff.lint]
select = ["E", "F", "I", "UP", "B", "SIM", "ARG"]

[tool.ruff.lint.per-file-ignores]
"tests/*" = ["ARG002"]

[tool.mypy]
python_version = "3.12"
strict = true

[dependency-groups]
dev = [
    "pytest-httpx>=0.36.2",
]
```

- [ ] **Step 2: Commit**

```bash
git add pyproject.toml
git commit -m "refactor: update pyproject.toml for engrammic-mcp"
```

---

### Task 3: Update Config Module

**Files:**
- Modify: `src/engrammic_mcp/config.py`

- [ ] **Step 1: Update config with new env prefix and defaults**

```python
"""Configuration from environment variables."""

from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Engrammic MCP settings."""

    backend_url: str = "https://api.engrammic.com"
    api_key: str | None = None
    credentials_path: Path = Path.home() / ".engrammic" / "credentials.json"

    model_config = SettingsConfigDict(
        env_prefix="ENGRAMMIC_",
        env_file=".env",
        env_file_encoding="utf-8",
    )


_settings: Settings | None = None


def get_settings() -> Settings:
    """Return cached settings instance."""
    global _settings
    if _settings is None:
        _settings = Settings()
    return _settings


def reset_settings() -> None:
    """Reset cached settings. For testing only."""
    global _settings
    _settings = None
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/config.py
git commit -m "refactor: update config for engrammic branding"
```

---

### Task 4: Update Client Module

**Files:**
- Modify: `src/engrammic_mcp/client.py`

- [ ] **Step 1: Rename class and update imports**

```python
"""HTTP client for Engrammic backend communication."""

from __future__ import annotations

import uuid
from typing import Any

import httpx
import structlog

from engrammic_mcp.config import Settings
from engrammic_mcp.credentials import load_credentials, store_credentials
from engrammic_mcp.errors import (
    EngrammicError,
    sanitize_error_message,
    status_to_error_code,
)

logger = structlog.get_logger(__name__)

_http_client: httpx.AsyncClient | None = None


def get_http_client() -> httpx.AsyncClient:
    """Return singleton HTTP client for connection reuse."""
    global _http_client
    if _http_client is None:
        _http_client = httpx.AsyncClient(
            timeout=30.0,
            http2=True,
        )
    return _http_client


def reset_http_client() -> None:
    """Reset the singleton client. For testing only."""
    global _http_client
    _http_client = None


class EngrammicClient:
    """Client for Engrammic backend API."""

    def __init__(self, settings: Settings) -> None:
        self.base_url = settings.backend_url.rstrip("/")
        self.settings = settings
        self._token: str | None = settings.api_key
        self._refresh_token: str | None = None

        if not self._token:
            self._load_oauth_credentials()

    def _load_oauth_credentials(self) -> None:
        """Load OAuth tokens from credential storage."""
        creds = load_credentials(self.settings.credentials_path)
        if creds:
            self._token = creds.get("access_token")
            self._refresh_token = creds.get("refresh_token")
            logger.debug("Loaded OAuth credentials from storage")

    async def post(self, path: str, data: dict[str, Any]) -> dict[str, Any]:
        """POST request to backend."""
        return await self._request("POST", path, data)

    async def get(self, path: str) -> dict[str, Any]:
        """GET request to backend."""
        return await self._request("GET", path)

    async def _request(
        self,
        method: str,
        path: str,
        data: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Execute HTTP request with auth, retry on 401, and error handling."""
        client = get_http_client()
        request_id = str(uuid.uuid4())

        headers = {
            "X-Request-ID": request_id,
        }
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"

        url = f"{self.base_url}{path}"

        resp = await client.request(
            method,
            url,
            json=data if method != "GET" else None,
            headers=headers,
        )

        if resp.status_code == 401 and self._refresh_token:
            logger.debug("Got 401, attempting token refresh")
            if await self._refresh_access_token():
                headers["Authorization"] = f"Bearer {self._token}"
                resp = await client.request(
                    method,
                    url,
                    json=data if method != "GET" else None,
                    headers=headers,
                )

        return self._handle_response(resp, request_id)

    async def _refresh_access_token(self) -> bool:
        """Attempt to refresh the access token. Returns True on success."""
        try:
            client = get_http_client()
            resp = await client.post(
                f"{self.base_url}/v1/oauth/token",
                json={"refresh_token": self._refresh_token, "grant_type": "refresh_token"},
            )
            if resp.status_code == 200:
                data = resp.json()
                self._token = data["access_token"]
                self._refresh_token = data.get("refresh_token", self._refresh_token)
                store_credentials(
                    self._token,
                    self._refresh_token or "",
                    self.settings.credentials_path,
                )
                logger.info("Successfully refreshed access token")
                return True
        except Exception as e:
            logger.warning("Failed to refresh token", error=str(e))
        return False

    def _handle_response(self, resp: httpx.Response, request_id: str) -> dict[str, Any]:
        """Handle response, sanitizing errors before returning."""
        if resp.status_code >= 400:
            try:
                body = resp.json()
                raw_message = body.get("message")
            except Exception:
                raw_message = resp.text[:500] if resp.text else None

            logger.error(
                "Backend error",
                status=resp.status_code,
                request_id=request_id,
                raw_message=raw_message,
            )

            raise EngrammicError(
                code=status_to_error_code(resp.status_code),
                message=sanitize_error_message(resp.status_code, raw_message),
                request_id=request_id,
            )

        return resp.json()
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/client.py
git commit -m "refactor: rename DeltaPrimeClient to EngrammicClient"
```

---

### Task 5: Update Errors Module

**Files:**
- Modify: `src/engrammic_mcp/errors.py`

- [ ] **Step 1: Rename error class**

```python
"""Error handling and sanitization for Engrammic MCP."""

from typing import Any


class EngrammicError(Exception):
    """Error from Engrammic backend, sanitized for agent consumption."""

    def __init__(self, code: str, message: str, request_id: str) -> None:
        self.code = code
        self.message = message
        self.request_id = request_id
        super().__init__(message)

    def to_dict(self) -> dict[str, Any]:
        """Return error as dictionary for MCP response."""
        return {
            "error": self.code,
            "message": self.message,
            "request_id": self.request_id,
        }


def status_to_error_code(status: int) -> str:
    """Map HTTP status code to error code."""
    return {
        400: "invalid_request",
        401: "unauthorized",
        403: "forbidden",
        404: "not_found",
        429: "rate_limited",
    }.get(status, "internal_error")


_FALLBACK_MESSAGES: dict[int, str] = {
    400: "Invalid request parameters",
    401: "Authentication failed - try logging in again",
    403: "Access denied",
    404: "Resource not found",
    429: "Rate limit exceeded - please slow down",
}


_INTERNAL_PATTERNS = [
    "traceback",
    "file \"",
    "line ",
    "memgraph",
    "qdrant",
    "silo_",
    "redis",
    "postgres",
]


def _contains_internal_details(msg: str) -> bool:
    """Check if message contains internal implementation details."""
    lower = msg.lower()
    return any(p in lower for p in _INTERNAL_PATTERNS)


def sanitize_error_message(status: int, message: str | None) -> str:
    """Return a safe error message, stripping internal details."""
    if message and not _contains_internal_details(message):
        return message
    return _FALLBACK_MESSAGES.get(status, "An unexpected error occurred")
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/errors.py
git commit -m "refactor: rename DeltaPrimeError to EngrammicError"
```

---

### Task 6: Update Credentials Module

**Files:**
- Modify: `src/engrammic_mcp/credentials.py`

- [ ] **Step 1: Update docstrings**

```python
"""Secure credential storage for OAuth tokens."""

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import structlog

logger = structlog.get_logger(__name__)


def store_credentials(
    access_token: str,
    refresh_token: str,
    path: Path,
) -> None:
    """Store OAuth credentials securely.

    Creates parent directories if needed. Sets file permissions to 600
    (owner read/write only) to protect tokens.
    """
    path.parent.mkdir(parents=True, exist_ok=True)

    data = {
        "access_token": access_token,
        "refresh_token": refresh_token,
        "stored_at": datetime.now(UTC).isoformat(),
    }

    path.write_text(json.dumps(data, indent=2))
    path.chmod(0o600)

    logger.info("Credentials stored", path=str(path))


def load_credentials(path: Path) -> dict[str, Any] | None:
    """Load stored credentials if they exist and have secure permissions.

    Returns None if:
    - File doesn't exist
    - File has insecure permissions (group or world readable)
    - File is not valid JSON
    """
    if not path.exists():
        return None

    mode = path.stat().st_mode
    if mode & 0o077:
        logger.warning(
            "Credentials file has insecure permissions, refusing to read",
            path=str(path),
            mode=oct(mode),
        )
        return None

    try:
        return json.loads(path.read_text())
    except (json.JSONDecodeError, OSError) as e:
        logger.warning("Failed to load credentials", path=str(path), error=str(e))
        return None
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/credentials.py
git commit -m "refactor: update credentials module docstrings"
```

---

### Task 7: Update Package Init

**Files:**
- Modify: `src/engrammic_mcp/__init__.py`

- [ ] **Step 1: Update init**

```python
"""Engrammic MCP Server."""

__version__ = "0.2.0"
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/__init__.py
git commit -m "refactor: update package init for engrammic"
```

---

### Task 8: Update Existing Tools

**Files:**
- Modify: `src/engrammic_mcp/tools/__init__.py`
- Modify: `src/engrammic_mcp/tools/context_store.py`
- Modify: `src/engrammic_mcp/tools/context_recall.py`
- Modify: `src/engrammic_mcp/tools/context_link.py`
- Modify: `src/engrammic_mcp/tools/context_admin.py`

- [ ] **Step 1: Update tools/__init__.py**

```python
"""MCP tool implementations for Engrammic."""

from engrammic_mcp.tools import (
    context_admin,
    context_link,
    context_recall,
    context_store,
)

__all__ = [
    "context_admin",
    "context_link",
    "context_recall",
    "context_store",
]
```

- [ ] **Step 2: Update context_store.py**

```python
"""MCP tool: context_store - Write to Engrammic context layers."""

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


async def store(
    intent: Literal["remember", "assert", "commit", "reflect"],
    content: str,
    tags: list[str] | None = None,
    metadata: dict[str, Any] | None = None,
    decay_class: str = "standard",
    claims: list[dict[str, Any]] | None = None,
    steps: list[dict[str, Any]] | None = None,
    observation_type: str | None = None,
) -> dict[str, Any]:
    """Store context to Engrammic."""
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": intent,
        "content": content,
    }

    if tags:
        payload["tags"] = tags
    if metadata:
        payload["metadata"] = metadata
    if decay_class != "standard":
        payload["decay_class"] = decay_class
    if claims:
        payload["claims"] = claims
    if steps:
        payload["steps"] = steps
    if observation_type:
        payload["observation_type"] = observation_type

    return await client.post("/v1/context/store", payload)
```

- [ ] **Step 3: Update context_recall.py**

```python
"""MCP tool: context_recall - Read from Engrammic context layers."""

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
    include_reflections: bool = False,
    include_steps: bool = False,
) -> dict[str, Any]:
    """Recall context from Engrammic."""
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
    if include_reflections:
        payload["include_reflections"] = include_reflections
    if include_steps:
        payload["include_steps"] = include_steps

    return await client.post("/v1/context/recall", payload)
```

- [ ] **Step 4: Update context_link.py**

```python
"""MCP tool: context_link - Create relationships between nodes."""

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
    """Create a typed relationship between two nodes."""
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

- [ ] **Step 5: Update context_admin.py**

```python
"""MCP tool: context_admin - Administrative operations."""

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


async def admin(
    action: Literal["whoami", "usage", "provenance", "history"],
    node_id: str | None = None,
    since: str | None = None,
) -> dict[str, Any]:
    """Administrative operations for Engrammic."""
    client = _get_client()
    payload: dict[str, Any] = {
        "action": action,
    }

    if node_id:
        payload["node_id"] = node_id
    if since:
        payload["since"] = since

    return await client.post("/v1/context/admin", payload)
```

- [ ] **Step 6: Commit**

```bash
git add src/engrammic_mcp/tools/
git commit -m "refactor: update existing tools for engrammic branding"
```

---

### Task 9: Add context_belief_state Tool

**Files:**
- Create: `src/engrammic_mcp/tools/context_belief_state.py`

- [ ] **Step 1: Create the tool**

```python
"""MCP tool: context_belief_state - Query session WorkingHypotheses."""

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


async def belief_state(
    session_id: str,
    about: list[str] | None = None,
) -> dict[str, Any]:
    """Query the session's active WorkingHypotheses with contradiction detection.

    Args:
        session_id: ID of the ReasoningSession to query.
        about: Optional list of node IDs to filter hypotheses.

    Returns:
        {working_hypotheses, potential_contradictions, reflection_suggested, session_id}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "session_id": session_id,
    }

    if about:
        payload["about"] = about

    return await client.post("/v1/context/belief_state", payload)
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/context_belief_state.py
git commit -m "feat: add context_belief_state tool"
```

---

### Task 10: Add context_update_belief Tool

**Files:**
- Create: `src/engrammic_mcp/tools/context_update_belief.py`

- [ ] **Step 1: Create the tool**

```python
"""MCP tool: context_update_belief - Mutate a WorkingHypothesis."""

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


async def update_belief(
    belief_id: str,
    confidence: float,
    reason: str,
    content: str | None = None,
) -> dict[str, Any]:
    """Update a WorkingHypothesis's confidence and optionally its content.

    Args:
        belief_id: ID of the WorkingHypothesis to update.
        confidence: New confidence score (0.0-1.0).
        reason: Human-readable reason for the update.
        content: If provided, replaces the belief's content text.

    Returns:
        {belief_id, confidence, content, updated_at, reason}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "belief_id": belief_id,
        "confidence": confidence,
        "reason": reason,
    }

    if content is not None:
        payload["content"] = content

    return await client.post("/v1/context/update_belief", payload)
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/context_update_belief.py
git commit -m "feat: add context_update_belief tool"
```

---

### Task 11: Add context_crystallize Tool

**Files:**
- Create: `src/engrammic_mcp/tools/context_crystallize.py`

- [ ] **Step 1: Create the tool**

```python
"""MCP tool: context_crystallize - Promote hypotheses to commitments."""

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


async def crystallize(
    belief_ids: list[str],
    reason: str | None = None,
) -> dict[str, Any]:
    """Crystallize WorkingHypotheses into Commitments.

    Args:
        belief_ids: List of WorkingHypothesis IDs to promote.
        reason: Optional reason stored on SUPERSEDES edges.

    Returns:
        {commitment_ids, crystallized_belief_ids, not_found?}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "belief_ids": belief_ids,
    }

    if reason is not None:
        payload["reason"] = reason

    return await client.post("/v1/context/crystallize", payload)
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/context_crystallize.py
git commit -m "feat: add context_crystallize tool"
```

---

### Task 12: Add context_accept_belief Tool

**Files:**
- Create: `src/engrammic_mcp/tools/context_accept_belief.py`

- [ ] **Step 1: Create the tool**

```python
"""MCP tool: context_accept_belief - Accept a ProposedBelief."""

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


async def accept_belief(
    belief_id: str,
    confidence: float | None = None,
) -> dict[str, Any]:
    """Accept a ProposedBelief and promote it to Belief.

    Args:
        belief_id: ID of the ProposedBelief to accept.
        confidence: Optional confidence override (0.0-1.0).

    Returns:
        {proposed_belief_id, status, created_belief_id, accepted_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "belief_id": belief_id,
    }

    if confidence is not None:
        payload["confidence"] = confidence

    return await client.post("/v1/context/accept_belief", payload)
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/context_accept_belief.py
git commit -m "feat: add context_accept_belief tool"
```

---

### Task 13: Add context_reject_belief Tool

**Files:**
- Create: `src/engrammic_mcp/tools/context_reject_belief.py`

- [ ] **Step 1: Create the tool**

```python
"""MCP tool: context_reject_belief - Reject a ProposedBelief."""

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


async def reject_belief(
    belief_id: str,
    reason: str | None = None,
) -> dict[str, Any]:
    """Reject a ProposedBelief.

    Args:
        belief_id: ID of the ProposedBelief to reject.
        reason: Optional reason for rejection.

    Returns:
        {proposed_belief_id, status, reason, rejected_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "belief_id": belief_id,
    }

    if reason is not None:
        payload["reason"] = reason

    return await client.post("/v1/context/reject_belief", payload)
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/context_reject_belief.py
git commit -m "feat: add context_reject_belief tool"
```

---

### Task 14: Add context_skills Tool

**Files:**
- Create: `src/engrammic_mcp/tools/context_skills.py`

- [ ] **Step 1: Create the tool**

```python
"""MCP tool: context_skills - Read-only skill registry access."""

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


async def skills(
    action: Literal["list", "get", "search"],
    name: str | None = None,
    query: str | None = None,
    namespace: str | None = None,
    limit: int = 50,
    offset: int = 0,
) -> dict[str, Any]:
    """Read-only access to the skill registry.

    Args:
        action: list, get, or search.
        name: Skill name (required for get).
        query: Search query (required for search).
        namespace: Filter by namespace prefix.
        limit: Max results (default 50, max 200).
        offset: Pagination offset.

    Returns:
        For list/search: {skills: [...], total, limit, offset}
        For get: {skill: {...}}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "action": action,
        "limit": min(limit, 200),
        "offset": offset,
    }

    if name is not None:
        payload["name"] = name
    if query is not None:
        payload["query"] = query
    if namespace is not None:
        payload["namespace"] = namespace

    return await client.post("/v1/context/skills", payload)
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/context_skills.py
git commit -m "feat: add context_skills tool"
```

---

### Task 15: Update Tools Init with New Tools

**Files:**
- Modify: `src/engrammic_mcp/tools/__init__.py`

- [ ] **Step 1: Add all new tool imports**

```python
"""MCP tool implementations for Engrammic."""

from engrammic_mcp.tools import (
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_crystallize,
    context_link,
    context_recall,
    context_reject_belief,
    context_skills,
    context_store,
    context_update_belief,
)

__all__ = [
    "context_accept_belief",
    "context_admin",
    "context_belief_state",
    "context_crystallize",
    "context_link",
    "context_recall",
    "context_reject_belief",
    "context_skills",
    "context_store",
    "context_update_belief",
]
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/tools/__init__.py
git commit -m "feat: export all 10 tools from tools package"
```

---

### Task 16: Create CLI Module

**Files:**
- Create: `src/engrammic_mcp/cli.py`

- [ ] **Step 1: Create CLI with login, version, and serve commands**

```python
"""CLI for Engrammic MCP server."""

from __future__ import annotations

import argparse
import asyncio
import http.server
import sys
import threading
import urllib.parse
import webbrowser
from typing import Any

import structlog

from engrammic_mcp import __version__
from engrammic_mcp.client import EngrammicClient, get_http_client
from engrammic_mcp.config import get_settings
from engrammic_mcp.credentials import store_credentials

logger = structlog.get_logger(__name__)


def main() -> None:
    """Main entry point for the CLI."""
    parser = argparse.ArgumentParser(
        prog="engrammic-mcp",
        description="MCP server for Engrammic context management",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"engrammic-mcp {__version__}",
    )

    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("login", help="Authenticate with Engrammic")
    subparsers.add_parser("serve", help="Run the MCP server")

    args = parser.parse_args()

    if args.command == "login":
        _run_login()
    elif args.command == "serve" or args.command is None:
        _run_server()


def _run_login() -> None:
    """Run the OAuth login flow."""
    settings = get_settings()
    result = asyncio.run(_oauth_login(settings))

    if result:
        print(f"Logged in successfully as {result.get('user', 'unknown')}")
        print(f"Organization: {result.get('org', 'unknown')}")
    else:
        print("Login failed or timed out", file=sys.stderr)
        sys.exit(1)


async def _oauth_login(settings: Any) -> dict[str, Any] | None:
    """Perform OAuth login flow with local callback server."""
    auth_code: str | None = None
    server_ready = threading.Event()

    class CallbackHandler(http.server.BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            nonlocal auth_code
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)

            if "code" in params:
                auth_code = params["code"][0]
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(b"<html><body><h1>Login successful!</h1>")
                self.wfile.write(b"<p>You can close this window.</p></body></html>")
            else:
                self.send_response(400)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(b"<html><body><h1>Login failed</h1></body></html>")

        def log_message(self, format: str, *args: Any) -> None:
            pass

    server = http.server.HTTPServer(("127.0.0.1", 0), CallbackHandler)
    port = server.server_address[1]
    redirect_uri = f"http://localhost:{port}/callback"

    def serve() -> None:
        server_ready.set()
        server.timeout = 120
        server.handle_request()
        server.server_close()

    thread = threading.Thread(target=serve)
    thread.start()
    server_ready.wait()

    auth_url = (
        f"{settings.backend_url}/v1/oauth/authorize"
        f"?redirect_uri={urllib.parse.quote(redirect_uri)}"
    )
    print(f"Opening browser for authentication...")
    webbrowser.open(auth_url)

    thread.join(timeout=120)

    if auth_code is None:
        return None

    client = get_http_client()
    resp = await client.post(
        f"{settings.backend_url}/v1/oauth/token",
        json={
            "code": auth_code,
            "redirect_uri": redirect_uri,
            "grant_type": "authorization_code",
        },
    )

    if resp.status_code != 200:
        return None

    data = resp.json()
    store_credentials(
        data["access_token"],
        data.get("refresh_token", ""),
        settings.credentials_path,
    )

    return {
        "user": data.get("user"),
        "org": data.get("org"),
    }


def _run_server() -> None:
    """Run the MCP server."""
    structlog.configure(
        processors=[
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.JSONRenderer(),
        ],
        wrapper_class=structlog.BoundLogger,
        context_class=dict,
        logger_factory=structlog.PrintLoggerFactory(file=sys.stderr),
    )

    asyncio.run(_startup_health_check())

    from engrammic_mcp.server import create_server

    server = create_server()
    server.run()


async def _startup_health_check() -> None:
    """Check connection to backend on startup."""
    try:
        settings = get_settings()
        client = EngrammicClient(settings)
        result = await client.post("/v1/context/admin", {"action": "whoami"})
        user = result.get("user_id", "unknown")
        org = result.get("org_id", "unknown")
        logger.info("Connected to Engrammic", user=user, org=org)
    except Exception as e:
        logger.warning("Failed to connect to Engrammic backend", error=str(e))
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/cli.py
git commit -m "feat: add CLI with login, version, and serve commands"
```

---

### Task 17: Update __main__.py

**Files:**
- Modify: `src/engrammic_mcp/__main__.py`

- [ ] **Step 1: Delegate to CLI**

```python
"""Entry point for Engrammic MCP server."""

from engrammic_mcp.cli import main

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/__main__.py
git commit -m "refactor: delegate __main__ to cli module"
```

---

### Task 18: Update Server Module

**Files:**
- Modify: `src/engrammic_mcp/server.py`

- [ ] **Step 1: Update imports and add all tools**

```python
"""FastMCP server for Engrammic."""

from typing import Any, Literal

from fastmcp import FastMCP

from engrammic_mcp.tools import (
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_crystallize,
    context_link,
    context_recall,
    context_reject_belief,
    context_skills,
    context_store,
    context_update_belief,
)


def create_server() -> FastMCP:
    """Create and configure the Engrammic MCP server."""
    mcp = FastMCP(
        name="engrammic",
        instructions=(
            "Engrammic context management for AI agents. "
            "Use context_store to save memories, knowledge, decisions, and reasoning. "
            "Use context_recall to search and retrieve context. "
            "Use context_link to connect related concepts. "
            "Use context_admin for usage info and provenance. "
            "Use context_belief_state to query active hypotheses. "
            "Use context_update_belief to revise hypothesis confidence. "
            "Use context_crystallize to promote hypotheses to commitments. "
            "Use context_accept_belief/context_reject_belief for proposed beliefs. "
            "Use context_skills for skill registry access."
        ),
    )

    @mcp.tool()
    async def context_store_tool(
        intent: Literal["remember", "assert", "commit", "reflect"],
        content: str,
        tags: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
        decay_class: str = "standard",
        claims: list[dict[str, Any]] | None = None,
        steps: list[dict[str, Any]] | None = None,
        observation_type: str | None = None,
    ) -> dict[str, Any]:
        """Store context to Engrammic."""
        return await context_store.store(
            intent=intent,
            content=content,
            tags=tags,
            metadata=metadata,
            decay_class=decay_class,
            claims=claims,
            steps=steps,
            observation_type=observation_type,
        )

    @mcp.tool()
    async def context_recall_tool(
        query: str | None = None,
        node_ids: list[str] | None = None,
        depth: int = 0,
        layers: list[str] | None = None,
        top_k: int = 10,
        as_of: str | None = None,
        include_reflections: bool = False,
        include_steps: bool = False,
    ) -> dict[str, Any]:
        """Recall context from Engrammic."""
        return await context_recall.recall(
            query=query,
            node_ids=node_ids,
            depth=depth,
            layers=layers,
            top_k=top_k,
            as_of=as_of,
            include_reflections=include_reflections,
            include_steps=include_steps,
        )

    @mcp.tool()
    async def context_link_tool(
        source_id: str,
        target_id: str,
        relation: str,
        metadata: dict[str, Any] | None = None,
        weight: float | None = None,
    ) -> dict[str, Any]:
        """Create a relationship between two context nodes."""
        return await context_link.link(
            source_id=source_id,
            target_id=target_id,
            relation=relation,
            metadata=metadata,
            weight=weight,
        )

    @mcp.tool()
    async def context_admin_tool(
        action: Literal["whoami", "usage", "provenance", "history"],
        node_id: str | None = None,
        since: str | None = None,
    ) -> dict[str, Any]:
        """Administrative operations."""
        return await context_admin.admin(
            action=action,
            node_id=node_id,
            since=since,
        )

    @mcp.tool()
    async def context_belief_state_tool(
        session_id: str,
        about: list[str] | None = None,
    ) -> dict[str, Any]:
        """Query session's active WorkingHypotheses with contradiction detection."""
        return await context_belief_state.belief_state(
            session_id=session_id,
            about=about,
        )

    @mcp.tool()
    async def context_update_belief_tool(
        belief_id: str,
        confidence: float,
        reason: str,
        content: str | None = None,
    ) -> dict[str, Any]:
        """Update a WorkingHypothesis's confidence and optionally its content."""
        return await context_update_belief.update_belief(
            belief_id=belief_id,
            confidence=confidence,
            reason=reason,
            content=content,
        )

    @mcp.tool()
    async def context_crystallize_tool(
        belief_ids: list[str],
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Crystallize WorkingHypotheses into Commitments."""
        return await context_crystallize.crystallize(
            belief_ids=belief_ids,
            reason=reason,
        )

    @mcp.tool()
    async def context_accept_belief_tool(
        belief_id: str,
        confidence: float | None = None,
    ) -> dict[str, Any]:
        """Accept a ProposedBelief and promote it to Belief."""
        return await context_accept_belief.accept_belief(
            belief_id=belief_id,
            confidence=confidence,
        )

    @mcp.tool()
    async def context_reject_belief_tool(
        belief_id: str,
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Reject a ProposedBelief."""
        return await context_reject_belief.reject_belief(
            belief_id=belief_id,
            reason=reason,
        )

    @mcp.tool()
    async def context_skills_tool(
        action: Literal["list", "get", "search"],
        name: str | None = None,
        query: str | None = None,
        namespace: str | None = None,
        limit: int = 50,
        offset: int = 0,
    ) -> dict[str, Any]:
        """Read-only access to the skill registry."""
        return await context_skills.skills(
            action=action,
            name=name,
            query=query,
            namespace=namespace,
            limit=limit,
            offset=offset,
        )

    return mcp
```

- [ ] **Step 2: Commit**

```bash
git add src/engrammic_mcp/server.py
git commit -m "feat: register all 10 tools in MCP server"
```

---

### Task 19: Update Tests

**Files:**
- Modify: `tests/conftest.py`
- Modify: `tests/test_client.py`
- Modify: `tests/test_credentials.py`
- Modify: `tests/test_errors.py`
- Modify: `tests/test_tools.py`

- [ ] **Step 1: Update conftest.py**

```python
"""Pytest fixtures for engrammic-mcp tests."""

import tempfile
from pathlib import Path
from typing import Generator

import pytest


@pytest.fixture
def temp_credentials_dir() -> Generator[Path, None, None]:
    """Temporary directory for credential storage tests."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def mock_settings(temp_credentials_dir: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Configure settings to use temp directory."""
    monkeypatch.setenv("ENGRAMMIC_BACKEND_URL", "http://localhost:8000")
    monkeypatch.setenv("ENGRAMMIC_CREDENTIALS_PATH", str(temp_credentials_dir / "creds.json"))

    from engrammic_mcp import config

    config._settings = None
```

- [ ] **Step 2: Update test_client.py**

```python
"""Tests for Engrammic HTTP client."""

import pytest
from pytest_httpx import HTTPXMock

from engrammic_mcp.client import EngrammicClient, get_http_client, reset_http_client
from engrammic_mcp.config import Settings
from engrammic_mcp.errors import EngrammicError


@pytest.fixture(autouse=True)
def reset_client() -> None:
    """Reset singleton client between tests."""
    reset_http_client()


@pytest.fixture
def settings(temp_credentials_dir) -> Settings:
    return Settings(
        backend_url="https://api.test.com",
        api_key="test_key",
        credentials_path=temp_credentials_dir / "creds.json",
    )


class TestGetHttpClient:
    def test_returns_singleton(self) -> None:
        client1 = get_http_client()
        client2 = get_http_client()
        assert client1 is client2


class TestEngrammicClient:
    async def test_post_success(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "abc123"},
        )
        client = EngrammicClient(settings)
        result = await client.post("/v1/context/store", {"content": "test"})
        assert result == {"node_id": "abc123"}

    async def test_includes_auth_header(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(json={})
        client = EngrammicClient(settings)
        await client.post("/v1/test", {})

        request = httpx_mock.get_request()
        assert request is not None
        assert request.headers["authorization"] == "Bearer test_key"

    async def test_includes_request_id(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(json={})
        client = EngrammicClient(settings)
        await client.post("/v1/test", {})

        request = httpx_mock.get_request()
        assert request is not None
        assert "x-request-id" in request.headers

    async def test_raises_sanitized_error_on_failure(
        self, settings: Settings, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            status_code=500,
            json={"message": "Traceback: internal error in memgraph"},
        )
        client = EngrammicClient(settings)

        with pytest.raises(EngrammicError) as exc_info:
            await client.post("/v1/test", {})

        assert exc_info.value.code == "internal_error"
        assert "Traceback" not in exc_info.value.message
        assert "memgraph" not in exc_info.value.message

    async def test_retries_on_401_with_refresh(
        self, settings: Settings, httpx_mock: HTTPXMock, temp_credentials_dir
    ) -> None:
        from engrammic_mcp.credentials import store_credentials

        store_credentials("old_token", "refresh_123", settings.credentials_path)

        settings_no_key = Settings(
            backend_url="https://api.test.com",
            api_key=None,
            credentials_path=settings.credentials_path,
        )

        httpx_mock.add_response(
            url="https://api.test.com/v1/test",
            status_code=401,
        )
        httpx_mock.add_response(
            url="https://api.test.com/v1/oauth/token",
            json={"access_token": "new_token", "refresh_token": "refresh_456"},
        )
        httpx_mock.add_response(
            url="https://api.test.com/v1/test",
            json={"success": True},
        )

        client = EngrammicClient(settings_no_key)
        result = await client.post("/v1/test", {})

        assert result == {"success": True}
        assert len(httpx_mock.get_requests()) == 3
```

- [ ] **Step 3: Update test_credentials.py**

```python
"""Tests for secure credential storage."""

import json
import stat
from pathlib import Path

import pytest

from engrammic_mcp.credentials import load_credentials, store_credentials


class TestStoreCredentials:
    def test_creates_parent_directory(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "subdir" / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            path=creds_path,
        )
        assert creds_path.exists()

    def test_stores_tokens(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            path=creds_path,
        )
        data = json.loads(creds_path.read_text())
        assert data["access_token"] == "tok_123"
        assert data["refresh_token"] == "ref_456"
        assert "stored_at" in data

    def test_sets_secure_permissions(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            path=creds_path,
        )
        mode = creds_path.stat().st_mode
        assert mode & stat.S_IRWXG == 0
        assert mode & stat.S_IRWXO == 0


class TestLoadCredentials:
    def test_returns_none_if_missing(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "nonexistent.json"
        result = load_credentials(creds_path)
        assert result is None

    def test_loads_stored_credentials(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials("tok_123", "ref_456", creds_path)
        result = load_credentials(creds_path)
        assert result is not None
        assert result["access_token"] == "tok_123"
        assert result["refresh_token"] == "ref_456"

    def test_refuses_insecure_permissions(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        creds_path.write_text('{"access_token": "tok"}')
        creds_path.chmod(0o644)
        result = load_credentials(creds_path)
        assert result is None
```

- [ ] **Step 4: Update test_errors.py**

```python
"""Tests for error handling and sanitization."""

import pytest

from engrammic_mcp.errors import (
    EngrammicError,
    sanitize_error_message,
    status_to_error_code,
)


class TestStatusToErrorCode:
    def test_known_status_codes(self) -> None:
        assert status_to_error_code(400) == "invalid_request"
        assert status_to_error_code(401) == "unauthorized"
        assert status_to_error_code(403) == "forbidden"
        assert status_to_error_code(404) == "not_found"
        assert status_to_error_code(429) == "rate_limited"

    def test_unknown_status_code(self) -> None:
        assert status_to_error_code(500) == "internal_error"
        assert status_to_error_code(502) == "internal_error"


class TestSanitizeErrorMessage:
    def test_safe_message_passed_through(self) -> None:
        assert sanitize_error_message(400, "Invalid intent parameter") == "Invalid intent parameter"

    def test_traceback_filtered(self) -> None:
        msg = "Traceback (most recent call last):\n  File \"/app/main.py\""
        result = sanitize_error_message(500, msg)
        assert "Traceback" not in result
        assert result == "An unexpected error occurred"

    def test_internal_paths_filtered(self) -> None:
        msg = "Error in memgraph_store.py line 123"
        result = sanitize_error_message(500, msg)
        assert "memgraph" not in result

    def test_silo_id_filtered(self) -> None:
        msg = "silo_abc123 not found"
        result = sanitize_error_message(404, msg)
        assert "silo_" not in result

    def test_fallback_by_status(self) -> None:
        assert sanitize_error_message(401, None) == "Authentication failed - try logging in again"
        assert sanitize_error_message(429, None) == "Rate limit exceeded - please slow down"


class TestEngrammicError:
    def test_to_dict(self) -> None:
        err = EngrammicError(
            code="invalid_request",
            message="Bad input",
            request_id="req-123",
        )
        assert err.to_dict() == {
            "error": "invalid_request",
            "message": "Bad input",
            "request_id": "req-123",
        }
```

- [ ] **Step 5: Update test_tools.py with all tools**

```python
"""Tests for MCP tool implementations."""

import pytest
from pytest_httpx import HTTPXMock

from engrammic_mcp.client import reset_http_client
from engrammic_mcp.config import Settings
from engrammic_mcp.tools import (
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_crystallize,
    context_link,
    context_recall,
    context_reject_belief,
    context_skills,
    context_store,
    context_update_belief,
)


@pytest.fixture(autouse=True)
def reset_clients() -> None:
    reset_http_client()
    context_store.reset_client()
    context_recall.reset_client()
    context_link.reset_client()
    context_admin.reset_client()
    context_belief_state.reset_client()
    context_update_belief.reset_client()
    context_crystallize.reset_client()
    context_accept_belief.reset_client()
    context_reject_belief.reset_client()
    context_skills.reset_client()


@pytest.fixture
def settings(temp_credentials_dir, monkeypatch) -> Settings:
    s = Settings(
        backend_url="https://api.test.com",
        api_key="test_key",
        credentials_path=temp_credentials_dir / "creds.json",
    )
    monkeypatch.setattr("engrammic_mcp.tools.context_store.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_recall.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_link.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_admin.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_belief_state.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_update_belief.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_crystallize.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_accept_belief.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_reject_belief.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_skills.get_settings", lambda: s)
    return s


class TestContextStore:
    async def test_remember(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "abc123", "layer": "memory"},
        )
        result = await context_store.store(
            intent="remember",
            content="User prefers dark mode",
        )
        assert result["node_id"] == "abc123"
        assert result["layer"] == "memory"


class TestContextRecall:
    async def test_query(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/recall",
            json={"nodes": [{"node_id": "abc", "content": "test"}]},
        )
        result = await context_recall.recall(query="dark mode preference")
        assert len(result["nodes"]) == 1


class TestContextLink:
    async def test_link_nodes(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/link",
            json={"edge_id": "edge123"},
        )
        result = await context_link.link(
            source_id="node1",
            target_id="node2",
            relation="RELATES_TO",
        )
        assert result["edge_id"] == "edge123"


class TestContextAdmin:
    async def test_whoami(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/admin",
            json={"org_id": "org123", "user_id": "user456"},
        )
        result = await context_admin.admin(action="whoami")
        assert result["org_id"] == "org123"


class TestContextBeliefState:
    async def test_query_session(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/belief_state",
            json={
                "working_hypotheses": [{"belief_id": "h1", "content": "test"}],
                "potential_contradictions": [],
                "reflection_suggested": False,
            },
        )
        result = await context_belief_state.belief_state(session_id="session123")
        assert len(result["working_hypotheses"]) == 1


class TestContextUpdateBelief:
    async def test_update_confidence(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/update_belief",
            json={"belief_id": "h1", "confidence": 0.9, "updated_at": "2026-05-09T12:00:00Z"},
        )
        result = await context_update_belief.update_belief(
            belief_id="h1",
            confidence=0.9,
            reason="New evidence supports this",
        )
        assert result["confidence"] == 0.9


class TestContextCrystallize:
    async def test_crystallize_beliefs(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/crystallize",
            json={
                "commitment_ids": ["c1"],
                "crystallized_belief_ids": ["h1"],
            },
        )
        result = await context_crystallize.crystallize(belief_ids=["h1"])
        assert result["commitment_ids"] == ["c1"]


class TestContextAcceptBelief:
    async def test_accept(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/accept_belief",
            json={
                "proposed_belief_id": "p1",
                "status": "accepted",
                "created_belief_id": "b1",
            },
        )
        result = await context_accept_belief.accept_belief(belief_id="p1")
        assert result["status"] == "accepted"


class TestContextRejectBelief:
    async def test_reject(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/reject_belief",
            json={
                "proposed_belief_id": "p1",
                "status": "rejected",
                "reason": "Not enough evidence",
            },
        )
        result = await context_reject_belief.reject_belief(
            belief_id="p1",
            reason="Not enough evidence",
        )
        assert result["status"] == "rejected"


class TestContextSkills:
    async def test_list_skills(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/skills",
            json={
                "skills": [{"name": "skill1"}],
                "total": 1,
            },
        )
        result = await context_skills.skills(action="list")
        assert len(result["skills"]) == 1
```

- [ ] **Step 6: Commit**

```bash
git add tests/
git commit -m "test: update all tests for engrammic branding and new tools"
```

---

### Task 20: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `.env.example`

- [ ] **Step 1: Update README.md**

```markdown
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
```

- [ ] **Step 2: Update .env.example**

```
# Engrammic MCP Configuration

# Backend URL (default: Engrammic Cloud)
ENGRAMMIC_BACKEND_URL=https://api.engrammic.com

# API Key (alternative to OAuth)
ENGRAMMIC_API_KEY=

# Credentials file location (default: ~/.engrammic/credentials.json)
# ENGRAMMIC_CREDENTIALS_PATH=
```

- [ ] **Step 3: Commit**

```bash
git add README.md .env.example
git commit -m "docs: update documentation for engrammic branding"
```

---

### Task 21: Final Verification

**Files:**
- All files

- [ ] **Step 1: Sync dependencies**

```bash
cd /home/novusedge/Projects/delta-prime/mcp-client
uv sync --all-extras
```

- [ ] **Step 2: Run all tests**

```bash
uv run pytest -v
```

Expected: All tests pass

- [ ] **Step 3: Run type checker**

```bash
uv run mypy src
```

Expected: No errors

- [ ] **Step 4: Run linter**

```bash
uv run ruff check src tests
```

Expected: No errors

- [ ] **Step 5: Verify CLI commands**

```bash
uv run engrammic-mcp --version
uv run engrammic-mcp --help
```

Expected: Shows version and help

- [ ] **Step 6: Final commit if any fixes needed**

```bash
git add -A
git commit -m "chore: final cleanup and verification"
```
