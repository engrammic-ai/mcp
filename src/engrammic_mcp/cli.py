"""CLI for Engrammic MCP server."""

from __future__ import annotations

import argparse
import asyncio
import http.server
import sys
import threading
import urllib.parse
import webbrowser
from typing import Any

import structlog

from engrammic_mcp import __version__
from engrammic_mcp.client import EngrammicClient, get_http_client
from engrammic_mcp.config import get_settings
from engrammic_mcp.credentials import store_credentials

logger = structlog.get_logger(__name__)


def main() -> None:
    """Main entry point for the CLI."""
    parser = argparse.ArgumentParser(
        prog="engrammic-mcp",
        description="MCP server for Engrammic context management",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"engrammic-mcp {__version__}",
    )

    subparsers = parser.add_subparsers(dest="command")
    subparsers.add_parser("login", help="Authenticate with Engrammic")
    subparsers.add_parser("serve", help="Run the MCP server")

    args = parser.parse_args()

    if args.command == "login":
        _run_login()
    elif args.command == "serve" or args.command is None:
        _run_server()


def _run_login() -> None:
    """Run the OAuth login flow."""
    settings = get_settings()
    result = asyncio.run(_oauth_login(settings))

    if result:
        print(f"Logged in successfully as {result.get('user', 'unknown')}")
        print(f"Organization: {result.get('org', 'unknown')}")
    else:
        print("Login failed or timed out", file=sys.stderr)
        sys.exit(1)


async def _oauth_login(settings: Any) -> dict[str, Any] | None:
    """Perform OAuth login flow with local callback server."""
    auth_code: str | None = None
    server_ready = threading.Event()

    class CallbackHandler(http.server.BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            nonlocal auth_code
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)

            if "code" in params:
                auth_code = params["code"][0]
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(b"<html><body><h1>Login successful!</h1>")
                self.wfile.write(b"<p>You can close this window.</p></body></html>")
            else:
                self.send_response(400)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(b"<html><body><h1>Login failed</h1></body></html>")

        def log_message(self, format: str, *args: Any) -> None:
            pass

    server = http.server.HTTPServer(("127.0.0.1", 0), CallbackHandler)
    port = server.server_address[1]
    redirect_uri = f"http://localhost:{port}/callback"

    def serve() -> None:
        server_ready.set()
        server.timeout = 120
        server.handle_request()
        server.server_close()

    thread = threading.Thread(target=serve)
    thread.start()
    server_ready.wait()

    auth_url = (
        f"{settings.backend_url}/v1/oauth/authorize"
        f"?redirect_uri={urllib.parse.quote(redirect_uri)}"
    )
    print("Opening browser for authentication...")
    webbrowser.open(auth_url)

    thread.join(timeout=120)

    if auth_code is None:
        return None

    client = get_http_client()
    resp = await client.post(
        f"{settings.backend_url}/v1/oauth/token",
        json={
            "code": auth_code,
            "redirect_uri": redirect_uri,
            "grant_type": "authorization_code",
        },
    )

    if resp.status_code != 200:
        return None

    data = resp.json()
    store_credentials(
        data["access_token"],
        data.get("refresh_token", ""),
        settings.credentials_path,
    )

    return {
        "user": data.get("user"),
        "org": data.get("org"),
    }


def _run_server() -> None:
    """Run the MCP server."""
    structlog.configure(
        processors=[
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.JSONRenderer(),
        ],
        wrapper_class=structlog.BoundLogger,
        context_class=dict,
        logger_factory=structlog.PrintLoggerFactory(file=sys.stderr),
    )

    asyncio.run(_startup_health_check())

    from engrammic_mcp.server import create_server

    server = create_server()
    server.run()


async def _startup_health_check() -> None:
    """Check connection to backend on startup."""
    try:
        settings = get_settings()
        client = EngrammicClient(settings)
        result = await client.post("/v1/context/admin", {"action": "whoami"})
        user = result.get("user_id", "unknown")
        org = result.get("org_id", "unknown")
        logger.info("Connected to Engrammic", user=user, org=org)
    except Exception as e:
        logger.warning("Failed to connect to Engrammic backend", error=str(e))
