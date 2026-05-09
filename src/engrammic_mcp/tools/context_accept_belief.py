"""MCP tool: context_accept_belief - Accept a ProposedBelief."""

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


async def accept_belief(
    belief_id: str,
    confidence: float | None = None,
) -> dict[str, Any]:
    """Accept a ProposedBelief and promote it to Belief."""
    client = _get_client()
    payload: dict[str, Any] = {"belief_id": belief_id}
    if confidence is not None:
        payload["confidence"] = confidence
    return await client.post("/v1/context/accept_belief", payload)
