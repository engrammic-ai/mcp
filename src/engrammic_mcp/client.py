"""HTTP client for Engrammic backend communication."""

from __future__ import annotations

import asyncio
import uuid
from datetime import UTC, datetime, timedelta
from typing import Any, cast

import httpx
import structlog
from filelock import FileLock, Timeout

from engrammic_mcp.config import Settings
from engrammic_mcp.credentials import clear_credentials, load_credentials, store_credentials
from engrammic_mcp.errors import (
    EngrammicError,
    sanitize_error_message,
    status_to_error_code,
)

logger = structlog.get_logger(__name__)

_http_client: httpx.AsyncClient | None = None
_refresh_lock = asyncio.Lock()

REFRESH_BUFFER_SECONDS = 60


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


class EngrammicClient:
    """Client for Engrammic backend API."""

    def __init__(self, settings: Settings) -> None:
        self.base_url = settings.backend_url.rstrip("/")
        self.settings = settings
        self._token: str | None = settings.api_key
        self._refresh_token: str | None = None
        self._needs_refresh = False

        if not self._token:
            self._load_oauth_credentials()

    def _load_oauth_credentials(self) -> None:
        """Load OAuth tokens from credential storage.

        If tokens are expired or expiring soon (within REFRESH_BUFFER_SECONDS),
        marks for proactive refresh on first request.
        """
        creds = load_credentials(self.settings.credentials_path)
        if not creds:
            return

        self._token = creds.get("access_token")
        self._refresh_token = creds.get("refresh_token")

        expires_at_str = creds.get("expires_at")
        if expires_at_str and self._token:
            try:
                expires_at = datetime.fromisoformat(expires_at_str)
                buffer = timedelta(seconds=REFRESH_BUFFER_SECONDS)
                if expires_at <= datetime.now(UTC) + buffer:
                    logger.debug("Access token expired or expiring soon, will refresh")
                    self._needs_refresh = True
            except ValueError:
                logger.warning("Invalid expires_at format in credentials")

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
        """Execute HTTP request with auth, proactive refresh, and 401 retry."""
        if self._needs_refresh and self._refresh_token:
            logger.debug("Proactive token refresh before request")
            await self._refresh_access_token()
            self._needs_refresh = False

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
        """Attempt to refresh the access token.

        Uses both in-process and cross-process locks to prevent concurrent
        refresh from multiple IDE windows/processes.

        Returns True on success, False if refresh failed.
        """
        lock_path = self.settings.credentials_path.with_suffix(".lock")

        async with _refresh_lock:
            try:
                file_lock = FileLock(lock_path, timeout=10)
                with file_lock:
                    creds = load_credentials(self.settings.credentials_path)
                    if creds and creds.get("access_token") != self._token:
                        self._token = creds.get("access_token")
                        self._refresh_token = creds.get("refresh_token")
                        logger.debug("Another process refreshed tokens, reloaded")
                        return True

                    return await self._do_refresh()

            except Timeout:
                logger.warning("Could not acquire refresh lock, another process may be refreshing")
                return False

    async def _do_refresh(self) -> bool:
        """Actually perform the token refresh HTTP call."""
        if not self._refresh_token:
            return False

        try:
            client = get_http_client()
            resp = await client.post(
                f"{self.base_url}/oauth/token",
                data={
                    "grant_type": "refresh_token",
                    "refresh_token": self._refresh_token,
                },
            )

            if resp.status_code == 200:
                tokens = resp.json()
                self._token = tokens["access_token"]
                new_refresh = tokens.get("refresh_token", self._refresh_token)
                self._refresh_token = new_refresh

                store_credentials(
                    access_token=self._token,
                    refresh_token=new_refresh or "",
                    expires_in=tokens.get("expires_in", 3600),
                    path=self.settings.credentials_path,
                )
                logger.info("Successfully refreshed access token")
                return True

            if resp.status_code in (400, 401):
                logger.warning("Refresh token invalid, re-authentication required")
                self._clear_credentials()
                return False

            logger.warning("Refresh failed with server error", status=resp.status_code)
            return False

        except httpx.NetworkError as e:
            logger.warning("Network error during token refresh", error=str(e))
            return False
        except Exception as e:
            logger.warning("Failed to refresh token", error=str(e))
            return False

    def _clear_credentials(self) -> None:
        """Clear stored credentials after auth failure."""
        self._token = None
        self._refresh_token = None
        clear_credentials(self.settings.credentials_path)

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

            raise EngrammicError(
                code=status_to_error_code(resp.status_code),
                message=sanitize_error_message(resp.status_code, raw_message),
                request_id=request_id,
            )

        return cast(dict[str, Any], resp.json())
