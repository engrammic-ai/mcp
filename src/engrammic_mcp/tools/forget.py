"""MCP tool: forget - Request deletion of a node."""

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


async def forget(
    node_id: str,
    reason: str | None = None,
    cascade: bool = False,
) -> dict[str, Any]:
    """Request deletion of a node.

    Args:
        node_id: ID of the node to forget.
        reason: Optional reason for the deletion (for audit).
        cascade: If True, also forget downstream nodes that reference this one.

    Returns:
        {status, node_id, tombstoned_at} or {status, node_id} on not_found.
        When cascade=True and status is tombstoned, also includes cascade_forgotten list.
    """
    client = _get_client()
    payload: dict[str, Any] = {"node_id": node_id, "cascade": cascade}
    if reason is not None:
        payload["reason"] = reason

    return await client.post("/v1/context/forget", payload)
