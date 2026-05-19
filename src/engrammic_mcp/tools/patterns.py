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
