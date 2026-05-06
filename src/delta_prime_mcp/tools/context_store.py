"""MCP tool: context_store - Write to Delta Prime context layers."""

from typing import Any, Literal

from delta_prime_mcp.client import DeltaPrimeClient
from delta_prime_mcp.config import get_settings

_client: DeltaPrimeClient | None = None


def _get_client() -> DeltaPrimeClient:
    global _client
    if _client is None:
        _client = DeltaPrimeClient(get_settings())
    return _client


async def store(
    intent: Literal["remember", "assert", "commit", "reflect"],
    content: str,
    tags: list[str] | None = None,
    metadata: dict[str, Any] | None = None,
    decay_class: str = "standard",
    claims: list[dict[str, Any]] | None = None,
    steps: list[dict[str, Any]] | None = None,
    observation_type: str | None = None,
) -> dict[str, Any]:
    """Store context to Delta Prime."""
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": intent,
        "content": content,
    }

    if tags:
        payload["tags"] = tags
    if metadata:
        payload["metadata"] = metadata
    if decay_class != "standard":
        payload["decay_class"] = decay_class
    if claims:
        payload["claims"] = claims
    if steps:
        payload["steps"] = steps
    if observation_type:
        payload["observation_type"] = observation_type

    return await client.post("/v1/context/store", payload)
