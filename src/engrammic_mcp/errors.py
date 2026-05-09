"""Error handling and sanitization for Engrammic MCP."""

from typing import Any


class EngrammicError(Exception):
    """Error from Engrammic backend, sanitized for agent consumption."""

    def __init__(self, code: str, message: str, request_id: str) -> None:
        self.code = code
        self.message = message
        self.request_id = request_id
        super().__init__(message)

    def to_dict(self) -> dict[str, Any]:
        """Return error as dictionary for MCP response."""
        return {
            "error": self.code,
            "message": self.message,
            "request_id": self.request_id,
        }


def status_to_error_code(status: int) -> str:
    """Map HTTP status code to error code."""
    return {
        400: "invalid_request",
        401: "unauthorized",
        403: "forbidden",
        404: "not_found",
        429: "rate_limited",
    }.get(status, "internal_error")


_FALLBACK_MESSAGES: dict[int, str] = {
    400: "Invalid request parameters",
    401: "Authentication failed - try logging in again",
    403: "Access denied",
    404: "Resource not found",
    429: "Rate limit exceeded - please slow down",
}


_INTERNAL_PATTERNS = [
    "traceback",
    "file \"",
    "line ",
    "memgraph",
    "qdrant",
    "silo_",
    "redis",
    "postgres",
]


def _contains_internal_details(msg: str) -> bool:
    """Check if message contains internal implementation details."""
    lower = msg.lower()
    return any(p in lower for p in _INTERNAL_PATTERNS)


def sanitize_error_message(status: int, message: str | None) -> str:
    """Return a safe error message, stripping internal details."""
    if message and not _contains_internal_details(message):
        return message
    return _FALLBACK_MESSAGES.get(status, "An unexpected error occurred")
