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
