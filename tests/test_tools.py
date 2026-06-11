"""Tests for MCP tool implementations."""

import pytest
from pytest_httpx import HTTPXMock

from engrammic_mcp.client import reset_http_client
from engrammic_mcp.config import Settings
from engrammic_mcp.tools import (
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


@pytest.fixture(autouse=True)
def reset_clients() -> None:
    reset_http_client()
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
