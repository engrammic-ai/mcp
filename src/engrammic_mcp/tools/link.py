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
