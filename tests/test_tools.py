"""Tests for MCP tool implementations."""

import pytest
from pytest_httpx import HTTPXMock

from delta_prime_mcp.client import reset_http_client
from delta_prime_mcp.config import Settings
from delta_prime_mcp.tools import context_admin, context_link, context_recall, context_store


@pytest.fixture(autouse=True)
def reset_client() -> None:
    reset_http_client()


@pytest.fixture
def settings(temp_credentials_dir, monkeypatch) -> Settings:
    s = Settings(
        backend_url="https://api.test.com",
        api_key="test_key",
        credentials_path=temp_credentials_dir / "creds.json",
    )
    monkeypatch.setattr("delta_prime_mcp.tools.context_store.get_settings", lambda: s)
    monkeypatch.setattr("delta_prime_mcp.tools.context_recall.get_settings", lambda: s)
    monkeypatch.setattr("delta_prime_mcp.tools.context_link.get_settings", lambda: s)
    monkeypatch.setattr("delta_prime_mcp.tools.context_admin.get_settings", lambda: s)
    return s


class TestContextStore:
    async def test_remember(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "abc123", "layer": "memory"},
        )
        result = await context_store.store(
            intent="remember",
            content="User prefers dark mode",
        )
        assert result["node_id"] == "abc123"
        assert result["layer"] == "memory"

    async def test_assert_with_claims(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "def456", "layer": "knowledge"},
        )
        result = await context_store.store(
            intent="assert",
            content="The sky is blue",
            claims=[{"subject": "sky", "predicate": "has_color", "object": "blue"}],
        )
        assert result["layer"] == "knowledge"


class TestContextRecall:
    async def test_query(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/recall",
            json={"nodes": [{"node_id": "abc", "content": "test"}]},
        )
        result = await context_recall.recall(query="dark mode preference")
        assert len(result["nodes"]) == 1

    async def test_fetch_by_ids(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/recall",
            json={"nodes": [{"node_id": "abc123"}]},
        )
        result = await context_recall.recall(node_ids=["abc123"])
        assert result["nodes"][0]["node_id"] == "abc123"


class TestContextLink:
    async def test_link_nodes(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/link",
            json={"edge_id": "edge123"},
        )
        result = await context_link.link(
            source_id="node1",
            target_id="node2",
            relation="RELATES_TO",
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
