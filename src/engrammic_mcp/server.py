"""FastMCP server for Engrammic."""

from typing import Any, Literal

from fastmcp import FastMCP

from engrammic_mcp.tools import (
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_crystallize,
    context_link,
    context_recall,
    context_reject_belief,
    context_skills,
    context_store,
    context_update_belief,
)


def create_server() -> FastMCP:
    """Create and configure the Engrammic MCP server."""
    mcp = FastMCP(
        name="engrammic",
        instructions=(
            "Engrammic context management for AI agents. "
            "Use context_store to save memories, knowledge, decisions, and reasoning. "
            "Use context_recall to search and retrieve context. "
            "Use context_link to connect related concepts. "
            "Use context_admin for usage info and provenance. "
            "Use context_belief_state to query active hypotheses. "
            "Use context_update_belief to revise hypothesis confidence. "
            "Use context_crystallize to promote hypotheses to commitments. "
            "Use context_accept_belief/context_reject_belief for proposed beliefs. "
            "Use context_skills for skill registry access."
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
        """Store context to Engrammic."""
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
        """Recall context from Engrammic."""
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
        """Create a relationship between two context nodes."""
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
        """Administrative operations."""
        return await context_admin.admin(
            action=action,
            node_id=node_id,
            since=since,
        )

    @mcp.tool()
    async def context_belief_state_tool(
        session_id: str,
        about: list[str] | None = None,
    ) -> dict[str, Any]:
        """Query session's active WorkingHypotheses with contradiction detection."""
        return await context_belief_state.belief_state(
            session_id=session_id,
            about=about,
        )

    @mcp.tool()
    async def context_update_belief_tool(
        belief_id: str,
        confidence: float,
        reason: str,
        content: str | None = None,
    ) -> dict[str, Any]:
        """Update a WorkingHypothesis's confidence and optionally its content."""
        return await context_update_belief.update_belief(
            belief_id=belief_id,
            confidence=confidence,
            reason=reason,
            content=content,
        )

    @mcp.tool()
    async def context_crystallize_tool(
        belief_ids: list[str],
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Crystallize WorkingHypotheses into Commitments."""
        return await context_crystallize.crystallize(
            belief_ids=belief_ids,
            reason=reason,
        )

    @mcp.tool()
    async def context_accept_belief_tool(
        belief_id: str,
        confidence: float | None = None,
    ) -> dict[str, Any]:
        """Accept a ProposedBelief and promote it to Belief."""
        return await context_accept_belief.accept_belief(
            belief_id=belief_id,
            confidence=confidence,
        )

    @mcp.tool()
    async def context_reject_belief_tool(
        belief_id: str,
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Reject a ProposedBelief."""
        return await context_reject_belief.reject_belief(
            belief_id=belief_id,
            reason=reason,
        )

    @mcp.tool()
    async def context_skills_tool(
        action: Literal["list", "get", "search"],
        name: str | None = None,
        query: str | None = None,
        namespace: str | None = None,
        limit: int = 50,
        offset: int = 0,
    ) -> dict[str, Any]:
        """Read-only access to the skill registry."""
        return await context_skills.skills(
            action=action,
            name=name,
            query=query,
            namespace=namespace,
            limit=limit,
            offset=offset,
        )

    return mcp
