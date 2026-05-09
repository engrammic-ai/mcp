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
    """Recall context from Delta Prime."""
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
