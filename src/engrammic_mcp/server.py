"""FastMCP server for Engrammic."""

from typing import Any, Literal

from fastmcp import FastMCP

from engrammic_mcp.tools import (
    # Internal-only tools (not in any profile)
    context_accept_belief,
    context_admin,
    context_belief_state,
    context_reject_belief,
    # Verb-based agent surface tools
    believe,
    commit,
    hypothesize,
    learn,
    link,
    patterns,
    reason,
    recall,
    reflect,
    remember,
    revise,
    trace,
)


def create_server() -> FastMCP:
    """Create and configure the Engrammic MCP server."""
    mcp = FastMCP(
        name="engrammic",
        instructions=(
            "Engrammic: Epistemic memory for AI agents.\n\n"
            "Quick start:\n"
            "- remember: store observations\n"
            "- learn: record claims WITH evidence\n"
            "- believe: declare conclusions\n"
            "- recall: search your knowledge\n"
            "- trace: understand why you believe something\n"
            "- link: connect related knowledge\n\n"
            "Guidelines:\n"
            "- Always provide evidence when using learn\n"
            "- Hint source_tier on learn when you know the source quality "
            "(authoritative for .gov/.edu, validated for curated data)\n"
            "- Reference existing nodes when forming beliefs\n"
            "- Use recall before storing to avoid duplicates\n\n"
            "Onboarding:\n"
            "- At session start, call patterns(action='get', name='onboarding') "
            "for your workflow guide\n\n"
            "Internal-only tools (SAGE and admin use only):\n"
            "context_admin, context_accept_belief, context_reject_belief, "
            "context_belief_state"
        ),
    )

    # --- Verb-based agent surface tools ---

    @mcp.tool()
    async def remember_tool(
        content: str,
        tags: list[str] | None = None,
        decay: str = "standard",
    ) -> dict[str, Any]:
        """Store an observation to memory layer."""
        return await remember.remember(
            content=content,
            tags=tags,
            decay=decay,
        )

    @mcp.tool()
    async def learn_tool(
        claim: str,
        evidence: list[str],
        source: str,
        confidence: float = 0.8,
        tags: list[str] | None = None,
        source_tier: str | None = None,
    ) -> dict[str, Any]:
        """Record something you learned with evidence."""
        return await learn.learn(
            claim=claim,
            evidence=evidence,
            source=source,
            confidence=confidence,
            tags=tags,
            source_tier=source_tier,
        )

    @mcp.tool()
    async def believe_tool(
        belief: str,
        about: list[str],
        confidence: float = 0.8,
        reasoning: str | None = None,
    ) -> dict[str, Any]:
        """Declare a belief as a commitment."""
        return await believe.believe(
            belief=belief,
            about=about,
            confidence=confidence,
            reasoning=reasoning,
        )

    @mcp.tool()
    async def recall_tool(
        query: str | None = None,
        node_ids: list[str] | None = None,
        depth: int = 0,
        layers: list[str] | None = None,
        top_k: int = 10,
        as_of: str | None = None,
    ) -> dict[str, Any]:
        """Retrieve knowledge by search or node ID."""
        return await recall.recall(
            query=query,
            node_ids=node_ids,
            depth=depth,
            layers=layers,
            top_k=top_k,
            as_of=as_of,
        )

    @mcp.tool()
    async def trace_tool(
        node_id: str,
    ) -> dict[str, Any]:
        """Trace provenance of a belief back to sources."""
        return await trace.trace(node_id=node_id)

    @mcp.tool()
    async def link_tool(
        source_id: str,
        target_id: str,
        relation: str,
        metadata: dict[str, Any] | None = None,
        weight: float | None = None,
    ) -> dict[str, Any]:
        """Create a typed relationship between nodes."""
        return await link.link(
            source_id=source_id,
            target_id=target_id,
            relation=relation,
            metadata=metadata,
            weight=weight,
        )

    @mcp.tool()
    async def reason_tool(
        problem: str,
        steps: list[dict[str, Any]],
        tags: list[str] | None = None,
    ) -> dict[str, Any]:
        """Record explicit reasoning steps."""
        return await reason.reason(
            problem=problem,
            steps=steps,
            tags=tags,
        )

    @mcp.tool()
    async def reflect_tool(
        observation: str,
        about: list[str] | None = None,
        observation_type: str | None = None,
    ) -> dict[str, Any]:
        """Record a meta-observation about your knowledge."""
        return await reflect.reflect(
            observation=observation,
            about=about,
            observation_type=observation_type,
        )

    @mcp.tool()
    async def hypothesize_tool(
        hypothesis: str,
        about: list[str],
        confidence: float = 0.8,
        session_id: str | None = None,
    ) -> dict[str, Any]:
        """Form a tentative belief during reasoning."""
        return await hypothesize.hypothesize(
            hypothesis=hypothesis,
            about=about,
            confidence=confidence,
            session_id=session_id,
        )

    @mcp.tool()
    async def revise_tool(
        belief_id: str,
        confidence: float,
        reason: str,
        content: str | None = None,
    ) -> dict[str, Any]:
        """Update a WorkingHypothesis."""
        return await revise.revise(
            belief_id=belief_id,
            confidence=confidence,
            reason=reason,
            content=content,
        )

    @mcp.tool()
    async def commit_tool(
        belief_ids: list[str],
        reason: str | None = None,
    ) -> dict[str, Any]:
        """Promote tentative hypotheses to permanent commitments."""
        return await commit.commit(
            belief_ids=belief_ids,
            reason=reason,
        )

    @mcp.tool()
    async def patterns_tool(
        action: Literal["list", "get", "search"],
        name: str | None = None,
        query: str | None = None,
        namespace: str | None = None,
        limit: int = 50,
        offset: int = 0,
    ) -> dict[str, Any]:
        """Discover workflow templates."""
        return await patterns.patterns(
            action=action,
            name=name,
            query=query,
            namespace=namespace,
            limit=limit,
            offset=offset,
        )

    # --- Internal-only tools (SAGE and admin use only, not in any profile) ---

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

    return mcp
