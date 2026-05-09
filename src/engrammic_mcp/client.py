"""HTTP client for Delta Prime backend communication."""

from __future__ import annotations

import uuid
from typing import Any, cast

import httpx
import structlog

from delta_prime_mcp.config import Settings
from delta_prime_mcp.credentials import load_credentials, store_credentials
from delta_prime_mcp.errors import (
    DeltaPrimeError,
    sanitize_error_message,
    status_to_error_code,
)

logger = structlog.get_logger(__name__)

_http_client: httpx.AsyncClient | None = None


def get_http_client() -> httpx.AsyncClient:
    """Return singleton HTTP client for connection reuse."""
    global _http_client
    if _http_client is None:
        _http_client = httpx.AsyncClient(
            timeout=30.0,
            http2=True,
        )
    return _http_client


def reset_http_client() -> None:
    """Reset the singleton client. For testing only."""
    global _http_client
    _http_client = None


class DeltaPrimeClient:
    """Client for Delta Prime backend API."""

    def __init__(self, settings: Settings) -> None:
        self.base_url = settings.backend_url.rstrip("/")
        self.settings = settings
        self._token: str | None = settings.api_key
        self._refresh_token: str | None = None

        if not self._token:
            self._load_oauth_credentials()

    def _load_oauth_credentials(self) -> None:
        """Load OAuth tokens from credential storage."""
        creds = load_credentials(self.settings.credentials_path)
        if creds:
            self._token = creds.get("access_token")
            self._refresh_token = creds.get("refresh_token")
            logger.debug("Loaded OAuth credentials from storage")

    async def post(self, path: str, data: dict[str, Any]) -> dict[str, Any]:
        """POST request to backend."""
        return await self._request("POST", path, data)

    async def get(self, path: str) -> dict[str, Any]:
        """GET request to backend."""
        return await self._request("GET", path)

    async def _request(
        self,
        method: str,
        path: str,
        data: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Execute HTTP request with auth, retry on 401, and error handling."""
        client = get_http_client()
        request_id = str(uuid.uuid4())

        headers = {
            "X-Request-ID": request_id,
        }
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"

        url = f"{self.base_url}{path}"

        resp = await client.request(
            method,
            url,
            json=data if method != "GET" else None,
            headers=headers,
        )

        if resp.status_code == 401 and self._refresh_token:
            logger.debug("Got 401, attempting token refresh")
            if await self._refresh_access_token():
                headers["Authorization"] = f"Bearer {self._token}"
                resp = await client.request(
                    method,
                    url,
                    json=data if method != "GET" else None,
                    headers=headers,
                )

        return self._handle_response(resp, request_id)

    async def _refresh_access_token(self) -> bool:
        """Attempt to refresh the access token. Returns True on success."""
        try:
            client = get_http_client()
            resp = await client.post(
                f"{self.base_url}/v1/auth/token/refresh",
                json={"refresh_token": self._refresh_token},
            )
            if resp.status_code == 200:
                data = resp.json()
                self._token = data["access_token"]
                self._refresh_token = data.get("refresh_token", self._refresh_token)
                store_credentials(
                    self._token,
                    self._refresh_token or "",
                    self.settings.credentials_path,
                )
                logger.info("Successfully refreshed access token")
                return True
        except Exception as e:
            logger.warning("Failed to refresh token", error=str(e))
        return False

    def _handle_response(self, resp: httpx.Response, request_id: str) -> dict[str, Any]:
        """Handle response, sanitizing errors before returning."""
        if resp.status_code >= 400:
            try:
                body = resp.json()
                raw_message = body.get("message")
            except Exception:
                raw_message = resp.text[:500] if resp.text else None

            logger.error(
                "Backend error",
                status=resp.status_code,
                request_id=request_id,
                raw_message=raw_message,
            )

            raise DeltaPrimeError(
                code=status_to_error_code(resp.status_code),
                message=sanitize_error_message(resp.status_code, raw_message),
                request_id=request_id,
            )

        return cast(dict[str, Any], resp.json())
