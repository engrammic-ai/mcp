"""Tests for Engrammic HTTP client."""

import pytest
from pytest_httpx import HTTPXMock

from engrammic_mcp.client import EngrammicClient, get_http_client, reset_http_client
from engrammic_mcp.config import Settings
from engrammic_mcp.errors import EngrammicError


@pytest.fixture(autouse=True)
def reset_client() -> None:
    """Reset singleton client between tests."""
    reset_http_client()


@pytest.fixture
def settings(temp_credentials_dir) -> Settings:
    return Settings(
        backend_url="https://api.test.com",
        api_key="test_key",
        credentials_path=temp_credentials_dir / "creds.json",
    )


class TestGetHttpClient:
    def test_returns_singleton(self) -> None:
        client1 = get_http_client()
        client2 = get_http_client()
        assert client1 is client2


class TestEngrammicClient:
    async def test_post_success(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url="https://api.test.com/v1/context/store",
            json={"node_id": "abc123"},
        )
        client = EngrammicClient(settings)
        result = await client.post("/v1/context/store", {"content": "test"})
        assert result == {"node_id": "abc123"}

    async def test_includes_auth_header(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(json={})
        client = EngrammicClient(settings)
        await client.post("/v1/test", {})

        request = httpx_mock.get_request()
        assert request is not None
        assert request.headers["authorization"] == "Bearer test_key"

    async def test_includes_request_id(self, settings: Settings, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(json={})
        client = EngrammicClient(settings)
        await client.post("/v1/test", {})

        request = httpx_mock.get_request()
        assert request is not None
        assert "x-request-id" in request.headers

    async def test_raises_sanitized_error_on_failure(
        self, settings: Settings, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            status_code=500,
            json={"message": "Traceback: internal error in memgraph"},
        )
        client = EngrammicClient(settings)

        with pytest.raises(EngrammicError) as exc_info:
            await client.post("/v1/test", {})

        assert exc_info.value.code == "internal_error"
        assert "Traceback" not in exc_info.value.message
        assert "memgraph" not in exc_info.value.message

    async def test_retries_on_401_with_refresh(
        self, settings: Settings, httpx_mock: HTTPXMock, temp_credentials_dir
    ) -> None:
        from engrammic_mcp.credentials import store_credentials

        store_credentials("old_token", "refresh_123", settings.credentials_path)

        settings_no_key = Settings(
            backend_url="https://api.test.com",
            api_key=None,
            credentials_path=settings.credentials_path,
        )

        httpx_mock.add_response(
            url="https://api.test.com/v1/test",
            status_code=401,
        )
        httpx_mock.add_response(
            url="https://api.test.com/v1/oauth/token",
            json={"access_token": "new_token", "refresh_token": "refresh_456"},
        )
        httpx_mock.add_response(
            url="https://api.test.com/v1/test",
            json={"success": True},
        )

        client = EngrammicClient(settings_no_key)
        result = await client.post("/v1/test", {})

        assert result == {"success": True}
        assert len(httpx_mock.get_requests()) == 3
