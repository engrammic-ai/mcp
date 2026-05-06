"""MCP tool: context_link - Create relationships between nodes."""

from typing import Any

from delta_prime_mcp.client import DeltaPrimeClient
from delta_prime_mcp.config import get_settings

_client: DeltaPrimeClient | None = None


def _get_client() -> DeltaPrimeClient:
    global _client
    if _client is None:
        _client = DeltaPrimeClient(get_settings())
    return _client


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
