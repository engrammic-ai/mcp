"""MCP tool: reason - Record reasoning steps."""

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


async def reason(
    problem: str,
    steps: list[dict[str, Any]],
    tags: list[str] | None = None,
) -> dict[str, Any]:
    """Record explicit reasoning steps.

    Args:
        problem: Problem being reasoned about.
        steps: Reasoning steps [{step, rationale, confidence?}].
        tags: Optional categorization.

    Returns:
        {node_id, step_ids, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "reason",
        "content": problem,
        "steps": steps,
    }
    if tags:
        payload["tags"] = tags

    return await client.post("/v1/context/store", payload)
