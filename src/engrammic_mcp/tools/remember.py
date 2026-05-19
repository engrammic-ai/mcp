"""MCP tool: remember - Store an observation."""

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


async def remember(
    content: str,
    tags: list[str] | None = None,
    decay: str = "standard",
) -> dict[str, Any]:
    """Store an observation to memory layer.

    Args:
        content: What to remember.
        tags: Optional categorization tags.
        decay: How long to keep: ephemeral|standard|durable|permanent.

    Returns:
        {node_id, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "remember",
        "content": content,
    }
    if tags:
        payload["tags"] = tags
    if decay != "standard":
        payload["decay_class"] = decay

    return await client.post("/v1/context/store", payload)
