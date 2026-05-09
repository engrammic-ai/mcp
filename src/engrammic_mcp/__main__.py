# src/delta_prime_mcp/__main__.py
"""Entry point for Delta Prime MCP server."""

import sys

import structlog

structlog.configure(
    processors=[
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.JSONRenderer(),
    ],
    wrapper_class=structlog.BoundLogger,
    context_class=dict,
    logger_factory=structlog.PrintLoggerFactory(file=sys.stderr),
)


def main() -> None:
    """Run the Delta Prime MCP server."""
    from delta_prime_mcp.server import create_server

    server = create_server()
    server.run()


if __name__ == "__main__":
    main()
