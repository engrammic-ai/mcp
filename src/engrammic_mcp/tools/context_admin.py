"""MCP tool: context_admin - Administrative operations."""

from typing import Any, Literal

from delta_prime_mcp.client import DeltaPrimeClient
from delta_prime_mcp.config import get_settings

_client: DeltaPrimeClient | None = None


def _get_client() -> DeltaPrimeClient:
    global _client
    if _client is None:
        _client = DeltaPrimeClient(get_settings())
    return _client


async def admin(
    action: Literal["whoami", "usage", "provenance", "history"],
    node_id: str | None = None,
    since: str | None = None,
) -> dict[str, Any]:
    """Administrative operations for Delta Prime."""
    client = _get_client()
    payload: dict[str, Any] = {
        "action": action,
    }

    if node_id:
        payload["node_id"] = node_id
    if since:
        payload["since"] = since

    return await client.post("/v1/context/admin", payload)
