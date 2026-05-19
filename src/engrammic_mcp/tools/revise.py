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
