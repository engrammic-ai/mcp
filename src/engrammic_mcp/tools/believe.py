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
    about: list[str] | str,
    confidence: float = 0.8,
    reasoning: str | None = None,
    supersedes: str | None = None,
) -> dict[str, Any]:
    """Declare a belief as a commitment.

    Args:
        belief: What you believe.
        about: REQUIRED. Node ID(s) this belief concerns. Accepts a single ID or list.
        confidence: 0.0-1.0 (default 0.8).
        reasoning: Why you believe this.
        supersedes: Node ID this belief replaces (for version chaining).

    Returns:
        {node_id, created_at}
    """
    client = _get_client()
    about_list = [about] if isinstance(about, str) else about
    payload: dict[str, Any] = {
        "intent": "commit",
        "content": belief,
        "about": about_list,
    }
    if confidence != 0.8:
        payload["confidence"] = confidence
    if reasoning:
        payload["reasoning"] = reasoning
    if supersedes:
        payload["supersedes"] = supersedes

    return await client.post("/v1/context/store", payload)
