"""Tests for MCP tool implementations."""

import pytest
from pytest_httpx import HTTPXMock

from engrammic_mcp.client import reset_http_client
from engrammic_mcp.config import Settings
from engrammic_mcp.tools import (
    believe,
    commit,
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


@pytest.fixture(autouse=True)
def reset_clients() -> None:
    reset_http_client()
    context_store.reset_client()
    context_recall.reset_client()
    context_link.reset_client()
    context_admin.reset_client()
    context_belief_state.reset_client()
    context_update_belief.reset_client()
    context_crystallize.reset_client()
    context_accept_belief.reset_client()
    context_reject_belief.reset_client()
    context_skills.reset_client()
    remember.reset_client()
    learn.reset_client()
    believe.reset_client()
    recall.reset_client()
    trace.reset_client()
    link.reset_client()
    reason.reset_client()
    reflect.reset_client()
    hypothesize.reset_client()
    revise.reset_client()
    commit.reset_client()
    patterns.reset_client()


@pytest.fixture
def settings(temp_credentials_dir, monkeypatch) -> Settings:
    s = Settings(
        backend_url="https://api.test.com",
        api_key="test_key",
        credentials_path=temp_credentials_dir / "creds.json",
    )
    monkeypatch.setattr("engrammic_mcp.tools.context_store.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_recall.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_link.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_admin.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_belief_state.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_update_belief.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_crystallize.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_accept_belief.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_reject_belief.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.context_skills.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.remember.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.learn.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.believe.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.recall.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.trace.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.link.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.reason.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.reflect.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.hypothesize.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.revise.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.commit.get_settings", lambda: s)
    monkeypatch.setattr("engrammic_mcp.tools.patterns.get_settings", lambda: s)
    return s


class TestContextStore:
    async def test_remember(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "abc123", "layer": "memory"},
        )
        result = await context_store.store(intent="remember", content="User prefers dark mode")
        assert result["node_id"] == "abc123"


class TestContextRecall:
    async def test_query(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/recall",
            json={"nodes": [{"node_id": "abc", "content": "test"}]},
        )
        result = await context_recall.recall(query="dark mode preference")
        assert len(result["nodes"]) == 1


class TestContextLink:
    async def test_link_nodes(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/link",
            json={"edge_id": "edge123"},
        )
        result = await context_link.link(
            source_id="node1", target_id="node2", relation="RELATES_TO"
        )
        assert result["edge_id"] == "edge123"


class TestContextAdmin:
    async def test_whoami(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/admin",
            json={"org_id": "org123", "user_id": "user456"},
        )
        result = await context_admin.admin(action="whoami")
        assert result["org_id"] == "org123"


class TestContextBeliefState:
    async def test_query_session(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/belief_state",
            json={"working_hypotheses": [{"belief_id": "h1"}], "potential_contradictions": []},
        )
        result = await context_belief_state.belief_state(session_id="session123")
        assert len(result["working_hypotheses"]) == 1


class TestContextUpdateBelief:
    async def test_update_confidence(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/update_belief",
            json={"belief_id": "h1", "confidence": 0.9},
        )
        result = await context_update_belief.update_belief(
            belief_id="h1", confidence=0.9, reason="New evidence"
        )
        assert result["confidence"] == 0.9


class TestContextCrystallize:
    async def test_crystallize(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/crystallize",
            json={"commitment_ids": ["c1"], "crystallized_belief_ids": ["h1"]},
        )
        result = await context_crystallize.crystallize(belief_ids=["h1"])
        assert result["commitment_ids"] == ["c1"]


class TestContextAcceptBelief:
    async def test_accept(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/accept_belief",
            json={"proposed_belief_id": "p1", "status": "accepted", "created_belief_id": "b1"},
        )
        result = await context_accept_belief.accept_belief(belief_id="p1")
        assert result["status"] == "accepted"


class TestContextRejectBelief:
    async def test_reject(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/reject_belief",
            json={"proposed_belief_id": "p1", "status": "rejected"},
        )
        result = await context_reject_belief.reject_belief(
            belief_id="p1", reason="Not enough evidence"
        )
        assert result["status"] == "rejected"


class TestContextSkills:
    async def test_list_skills(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/skills",
            json={"skills": [{"name": "skill1"}], "total": 1},
        )
        result = await context_skills.skills(action="list")
        assert len(result["skills"]) == 1


class TestRemember:
    async def test_remember_basic(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "test-node-id", "created_at": "2026-05-19T00:00:00Z"},
        )
        result = await remember.remember(content="user prefers dark mode")
        assert result["node_id"] == "test-node-id"


class TestLearn:
    async def test_learn_basic(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={
                "node_id": "learn-node-id",
                "evidence_status": "linked",
                "created_at": "2026-05-19T00:00:00Z",
            },
        )
        result = await learn.learn(
            claim="Python 3.12 added type parameter syntax",
            evidence=["https://docs.python.org/3.12/whatsnew/3.12.html"],
            source="document",
        )
        assert result["node_id"] == "learn-node-id"
        assert result["evidence_status"] == "linked"


class TestBelieve:
    async def test_believe_basic(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "belief-node-id", "created_at": "2026-05-19T00:00:00Z"},
        )
        result = await believe.believe(
            belief="The system performs best with cache enabled",
            about=["node-abc", "node-def"],
        )
        assert result["node_id"] == "belief-node-id"


class TestRecall:
    async def test_recall_by_query(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/recall",
            json={"nodes": [{"node_id": "n1", "content": "dark mode preference"}]},
        )
        result = await recall.recall(query="dark mode")
        assert len(result["nodes"]) == 1
        assert result["nodes"][0]["node_id"] == "n1"


class TestTrace:
    async def test_trace_node(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/trace/node-abc",
            json={"chain": [{"node_id": "node-abc"}], "root_sources": ["node-xyz"]},
        )
        result = await trace.trace(node_id="node-abc")
        assert len(result["chain"]) == 1
        assert "root_sources" in result


class TestLink:
    async def test_link_nodes(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/link",
            json={"edge_id": "edge-xyz", "created_at": "2026-05-19T00:00:00Z"},
        )
        result = await link.link(
            source_id="node-1",
            target_id="node-2",
            relation="SUPPORTS",
        )
        assert result["edge_id"] == "edge-xyz"


class TestReason:
    async def test_reason_basic(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={
                "node_id": "reason-node-id",
                "step_ids": ["s1", "s2"],
                "created_at": "2026-05-19T00:00:00Z",
            },
        )
        result = await reason.reason(
            problem="Which caching strategy is best?",
            steps=[
                {"step": "Analyze workload patterns", "rationale": "Must know read/write ratio"},
                {
                    "step": "Select LRU for read-heavy workloads",
                    "rationale": "Maximizes cache hits",
                },
            ],
        )
        assert result["node_id"] == "reason-node-id"
        assert len(result["step_ids"]) == 2


class TestReflect:
    async def test_reflect_basic(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "reflect-node-id", "created_at": "2026-05-19T00:00:00Z"},
        )
        result = await reflect.reflect(
            observation="My earlier belief about cache size was incorrect",
            observation_type="correction",
        )
        assert result["node_id"] == "reflect-node-id"


class TestHypothesize:
    async def test_hypothesize_basic(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/hypothesize",
            json={
                "belief_id": "hyp-1",
                "session_id": "sess-abc",
                "potential_conflicts": [],
                "created_at": "2026-05-19T00:00:00Z",
            },
        )
        result = await hypothesize.hypothesize(
            hypothesis="Redis is the bottleneck under high load",
            about=["node-redis", "node-load-test"],
        )
        assert result["belief_id"] == "hyp-1"
        assert result["potential_conflicts"] == []


class TestRevise:
    async def test_revise_belief(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/revise",
            json={"belief_id": "hyp-1", "updated_at": "2026-05-19T00:00:00Z"},
        )
        result = await revise.revise(
            belief_id="hyp-1",
            confidence=0.95,
            reason="Load test confirmed Redis is the bottleneck",
        )
        assert result["belief_id"] == "hyp-1"


class TestCommit:
    async def test_commit_hypotheses(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/commit",
            json={"committed": ["hyp-1", "hyp-2"], "created_at": "2026-05-19T00:00:00Z"},
        )
        result = await commit.commit(
            belief_ids=["hyp-1", "hyp-2"], reason="Validated by production data"
        )
        assert result["committed"] == ["hyp-1", "hyp-2"]


class TestPatterns:
    async def test_list_patterns(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/patterns",
            json={"patterns": [{"name": "onboarding"}], "total": 1},
        )
        result = await patterns.patterns(action="list")
        assert len(result["patterns"]) == 1

    async def test_get_pattern(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/patterns",
            json={"pattern": {"name": "onboarding", "steps": []}},
        )
        result = await patterns.patterns(action="get", name="onboarding")
        assert result["pattern"]["name"] == "onboarding"
