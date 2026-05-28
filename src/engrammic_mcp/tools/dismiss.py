"""MCP tool: dismiss - Dismiss an engagement marker."""

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


async def dismiss(
    marker_id: str,
    reason: str,
    silo_id: str | None = None,
) -> dict[str, Any]:
    """Dismiss a Contradiction or StaleCommitment marker without resolving it.

    Use this to acknowledge a marker that does not require action (e.g., false
    positive, already handled externally, or intentionally accepted contradiction).

    Args:
        marker_id: ID of the marker to dismiss.
        reason: Reason for dismissal (stored for audit trail).
        silo_id: UUID of the silo. Optional; defaults to the org's primary silo.

    Returns:
        {marker_id, status, reason, resolved_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {"marker_id": marker_id, "reason": reason}
    if silo_id is not None:
        payload["silo_id"] = silo_id

    return await client.post("/v1/context/dismiss", payload)
