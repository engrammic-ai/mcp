"""MCP tool: context_crystallize - Promote hypotheses to commitments."""

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


async def crystallize(
    belief_ids: list[str],
    reason: str | None = None,
) -> dict[str, Any]:
    """Crystallize WorkingHypotheses into Commitments."""
    client = _get_client()
    payload: dict[str, Any] = {"belief_ids": belief_ids}
    if reason is not None:
        payload["reason"] = reason
    return await client.post("/v1/context/crystallize", payload)
