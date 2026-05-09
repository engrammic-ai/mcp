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
    """Read-only access to the skill registry."""
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
