"""MCP tool: context_admin - Administrative operations."""

from typing import Any, Literal

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
