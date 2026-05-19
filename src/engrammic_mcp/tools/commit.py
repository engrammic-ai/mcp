"""MCP tool: commit - Crystallize hypotheses to commitments."""

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


async def commit(
    belief_ids: list[str],
    reason: str | None = None,
) -> dict[str, Any]:
    """Promote tentative hypotheses to permanent commitments.

    Args:
        belief_ids: Hypotheses to crystallize.
        reason: Why committing now.

    Returns:
        {committed: [...], created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {"belief_ids": belief_ids}
    if reason:
        payload["reason"] = reason

    return await client.post("/v1/context/commit", payload)
