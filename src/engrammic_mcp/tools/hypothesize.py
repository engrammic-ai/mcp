"""MCP tool: hypothesize - Form tentative beliefs."""

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


async def hypothesize(
    hypothesis: str,
    about: list[str],
    confidence: float = 0.8,
    session_id: str | None = None,
) -> dict[str, Any]:
    """Form a tentative belief during reasoning.

    Args:
        hypothesis: Tentative belief.
        about: REQUIRED. Node IDs this concerns.
        confidence: 0.0-1.0 (default 0.8).
        session_id: Optional session override.

    Returns:
        {belief_id, session_id, potential_conflicts, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "hypothesis": hypothesis,
        "about": about,
    }
    if confidence != 0.8:
        payload["confidence"] = confidence
    if session_id:
        payload["session_id"] = session_id

    return await client.post("/v1/context/hypothesize", payload)
