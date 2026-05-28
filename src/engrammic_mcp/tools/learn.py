"""MCP tool: learn - Record a claim with evidence."""

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


async def learn(
    claim: str,
    evidence: list[str],
    source: str,
    confidence: float = 0.8,
    tags: list[str] | None = None,
    source_tier: str | None = None,
    supersedes: str | None = None,
) -> dict[str, Any]:
    """Record something you learned with evidence.

    Args:
        claim: What you learned.
        evidence: REQUIRED. References: node:<uuid> or URI.
        source: Source type: document|user|external|agent.
        confidence: 0.0-1.0 (default 0.8).
        tags: Optional categorization.
        source_tier: Quality tier hint: authoritative|validated|community|unknown.
        supersedes: Node ID this claim replaces (for version chaining).

    Returns:
        {node_id, evidence_status, created_at}
    """
    client = _get_client()
    payload: dict[str, Any] = {
        "intent": "assert",
        "content": claim,
        "claims": [{"claim": claim, "evidence": evidence, "source": source}],
    }
    if confidence != 0.8:
        payload["confidence"] = confidence
    if tags:
        payload["tags"] = tags
    if source_tier:
        payload["source_tier"] = source_tier
    if supersedes:
        payload["supersedes"] = supersedes

    return await client.post("/v1/context/store", payload)
