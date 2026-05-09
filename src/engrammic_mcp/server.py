# src/delta_prime_mcp/server.py
"""FastMCP server for Delta Prime."""

from typing import Any, Literal

from fastmcp import FastMCP

from delta_prime_mcp.tools import context_admin, context_link, context_recall, context_store


def create_server() -> FastMCP:
    """Create and configure the Delta Prime MCP server."""
    mcp = FastMCP(
        name="delta-prime",
        instructions=(
            "Delta Prime context management for AI agents. "
            "Use context_store to save memories, knowledge, decisions, and reasoning. "
            "Use context_recall to search and retrieve context. "
            "Use context_link to connect related concepts. "
            "Use context_admin for usage info and provenance."
        ),
    )

    @mcp.tool()
    async def context_store_tool(
        intent: Literal["remember", "assert", "commit", "reflect"],
        content: str,
        tags: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
        decay_class: str = "standard",
        claims: list[dict[str, Any]] | None = None,
        steps: list[dict[str, Any]] | None = None,
        observation_type: str | None = None,
    ) -> dict[str, Any]:
        """Store context to Delta Prime.

        intent options:
        - remember: Store observations and documents (memory layer)
        - assert: Store claims and facts (knowledge layer)
        - commit: Store decisions and commitments (wisdom layer)
        - reflect: Store reasoning chains (intelligence layer)
        """
        return await context_store.store(
            intent=intent,
            content=content,
            tags=tags,
            metadata=metadata,
            decay_class=decay_class,
            claims=claims,
            steps=steps,
            observation_type=observation_type,
        )

    @mcp.tool()
    async def context_recall_tool(
        query: str | None = None,
        node_ids: list[str] | None = None,
        depth: int = 0,
        layers: list[str] | None = None,
        top_k: int = 10,
        as_of: str | None = None,
        include_reflections: bool = False,
        include_steps: bool = False,
    ) -> dict[str, Any]:
        """Recall context from Delta Prime.

        Provide either query (semantic search) or node_ids (direct fetch).
        Use depth > 0 to traverse graph relationships.
        Use as_of for time-travel queries.
        """
        return await context_recall.recall(
            query=query,
            node_ids=node_ids,
            depth=depth,
            layers=layers,
            top_k=top_k,
            as_of=as_of,
            include_reflections=include_reflections,
            include_steps=include_steps,
        )

    @mcp.tool()
    async def context_link_tool(
        source_id: str,
        target_id: str,
        relation: str,
        metadata: dict[str, Any] | None = None,
        weight: float | None = None,
    ) -> dict[str, Any]:
        """Create a relationship between two context nodes.

        Common relations: RELATES_TO, SUPPORTS, CONTRADICTS, DERIVED_FROM, SUPERSEDES
        """
        return await context_link.link(
            source_id=source_id,
            target_id=target_id,
            relation=relation,
            metadata=metadata,
            weight=weight,
        )

    @mcp.tool()
    async def context_admin_tool(
        action: Literal["whoami", "usage", "provenance", "history"],
        node_id: str | None = None,
        since: str | None = None,
    ) -> dict[str, Any]:
        """Administrative operations.

        Actions:
        - whoami: Get current user and organization info
        - usage: Get usage statistics for current period
        - provenance: Get creation history for a node (requires node_id)
        - history: Get edit history for a node (requires node_id)
        """
        return await context_admin.admin(
            action=action,
            node_id=node_id,
            since=since,
        )

    return mcp
