"""MCP tool: reflect - Record meta-observations."""

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


async def reflect(
    observation: str,
    about: list[str] | None = None,
    observation_type: str | None = None,
) -> dict[str, Any]:
    """Record a meta-observation about your knowledge.

    Args:
        observation: What you observed about your knowledge/reasoning.
        about: Node IDs this reflection concerns.
        observation_type: Type: contradiction|uncertainty|update|correction.

    Returns:
        {node_id, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "reflect",
        "content": observation,
    }
    if about:
        payload["about"] = about
    if observation_type:
        payload["observation_type"] = observation_type

    return await client.post("/v1/context/store", payload)
