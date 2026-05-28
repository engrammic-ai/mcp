"""FastMCP server for Engrammic."""

from typing import Any, Literal

from fastmcp import FastMCP

from engrammic_mcp.tools import (
    believe,
    commit,
    dismiss,
    forget,
    hypothesize,
    learn,
    link,
    patterns,
    reason,
    recall,
    reflect,
    remember,
    revise,
    tick,
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
            "for your workflow guide"
        ),
    )

    # --- Verb-based agent surface tools ---

    @mcp.tool()
    async def remember_tool(
        content: str,
        tags: list[str] | None = None,
        decay: str = "standard",
        supersedes: str | None = None,
    ) -> dict[str, Any]:
        """Store an observation to memory layer."""
        return await remember.remember(
            content=content,
            tags=tags,
            decay=decay,
            supersedes=supersedes,
        )

    @mcp.tool()
    async def learn_tool(
        claim: str,
        evidence: list[str],
        source: str,
        confidence: float = 0.8,
        tags: list[str] | None = None,
        source_tier: str | None = None,
        supersedes: str | None = None,
    ) -> dict[str, Any]:
        """Record something you learned with evidence."""
        return await learn.learn(
            claim=claim,
            evidence=evidence,
            source=source,
            confidence=confidence,
            tags=tags,
            source_tier=source_tier,
            supersedes=supersedes,
        )

    @mcp.tool()
    async def believe_tool(
        belief: str,
        about: list[str] | str,
        confidence: float = 0.8,
        reasoning: str | None = None,
        supersedes: str | None = None,
    ) -> dict[str, Any]:
        """Declare a belief as a commitment."""
        return await believe.believe(
            belief=belief,
            about=about,
            confidence=confidence,
            reasoning=reasoning,
            supersedes=supersedes,
        )

    @mcp.tool()
    async def recall_tool(
        query: str | None = None,
        node_ids: list[str] | None = None,
        depth: int = 0,
        layers: list[str] | None = None,
        top_k: int = 10,
        include_hypotheses: bool = False,
        bypass_cache: bool = False,
        max_age_seconds: int | None = None,
    ) -> dict[str, Any]:
        """Retrieve knowledge by search or node ID."""
        return await recall.recall(
            query=query,
            node_ids=node_ids,
            depth=depth,
            layers=layers,
            top_k=top_k,
            include_hypotheses=include_hypotheses,
            bypass_cache=bypass_cache,
            max_age_seconds=max_age_seconds,
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
        profile: str | None = None,
    ) -> dict[str, Any]:
        """Discover workflow templates."""
        return await patterns.patterns(
            action=action,
            name=name,
            query=query,
            profile=profile,
        )

    @mcp.tool()
    async def forget_tool(
        node_id: str,
        reason: str | None = None,
        cascade: bool = False,
    ) -> dict[str, Any]:
        """Request deletion of a node."""
        return await forget.forget(
            node_id=node_id,
            reason=reason,
            cascade=cascade,
        )

    @mcp.tool()
    async def dismiss_tool(
        marker_id: str,
        reason: str,
        silo_id: str | None = None,
    ) -> dict[str, Any]:
        """Dismiss an engagement marker without resolving it."""
        return await dismiss.dismiss(
            marker_id=marker_id,
            reason=reason,
            silo_id=silo_id,
        )

    @mcp.tool()
    async def tick_tool(
        about_hint: list[str] | None = None,
        silo_id: str | None = None,
        session_id: str | None = None,
        recent_context: str | None = None,
    ) -> dict[str, Any]:
        """Session heartbeat - returns engagement nudges."""
        return await tick.tick(
            about_hint=about_hint,
            silo_id=silo_id,
            session_id=session_id,
            recent_context=recent_context,
        )

    return mcp
