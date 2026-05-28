"""MCP tool: tick - Session heartbeat with engagement nudges."""

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


async def tick(
    about_hint: list[str] | None = None,
    silo_id: str | None = None,
    session_id: str | None = None,
    recent_context: str | None = None,
) -> dict[str, Any]:
    """Check for pending engagement markers without a full recall operation.

    Safe to call frequently; reads the precomputed marker index only. Returns
    engagement markers, contextual nudges, and session state.

    Args:
        about_hint: Optional list of node IDs to scope the check. When provided,
            only markers touching those nodes are returned.
        silo_id: UUID of the silo. Optional; defaults to the org's primary silo.
        session_id: Session ID returned from a previous tick() call. Pass this
            back to maintain session continuity and enable debouncing.
        recent_context: Brief description of what the agent is currently working
            on. Used for context-aware nudge matching.

    Returns:
        {status, session_id, engagement, markers, nudges, meta}
    """
    client = _get_client()
    payload: dict[str, Any] = {}
    if about_hint is not None:
        payload["about_hint"] = about_hint
    if silo_id is not None:
        payload["silo_id"] = silo_id
    if session_id is not None:
        payload["session_id"] = session_id
    if recent_context is not None:
        payload["recent_context"] = recent_context

    return await client.post("/v1/context/tick", payload)
