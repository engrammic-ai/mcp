"""MCP tool: context_belief_state - Query session WorkingHypotheses."""

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


async def belief_state(
    session_id: str,
    about: list[str] | None = None,
) -> dict[str, Any]:
    """Query the session's active WorkingHypotheses with contradiction detection."""
    client = _get_client()
    payload: dict[str, Any] = {"session_id": session_id}
    if about:
        payload["about"] = about
    return await client.post("/v1/context/belief_state", payload)
