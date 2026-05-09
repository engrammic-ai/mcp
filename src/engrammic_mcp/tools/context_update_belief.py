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
    global _client
    _client = None


async def update_belief(
    belief_id: str,
    confidence: float,
    reason: str,
    content: str | None = None,
) -> dict[str, Any]:
    """Update a WorkingHypothesis's confidence and optionally its content."""
    client = _get_client()
    payload: dict[str, Any] = {
        "belief_id": belief_id,
        "confidence": confidence,
        "reason": reason,
    }
    if content is not None:
        payload["content"] = content
    return await client.post("/v1/context/update_belief", payload)
