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
