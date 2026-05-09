"""Tests for error handling and sanitization."""


from engrammic_mcp.errors import (
    EngrammicError,
    sanitize_error_message,
    status_to_error_code,
)


class TestStatusToErrorCode:
    def test_known_status_codes(self) -> None:
        assert status_to_error_code(400) == "invalid_request"
        assert status_to_error_code(401) == "unauthorized"
        assert status_to_error_code(403) == "forbidden"
        assert status_to_error_code(404) == "not_found"
        assert status_to_error_code(429) == "rate_limited"

    def test_unknown_status_code(self) -> None:
        assert status_to_error_code(500) == "internal_error"
        assert status_to_error_code(502) == "internal_error"


class TestSanitizeErrorMessage:
    def test_safe_message_passed_through(self) -> None:
        assert sanitize_error_message(400, "Invalid intent parameter") == "Invalid intent parameter"

    def test_traceback_filtered(self) -> None:
        msg = "Traceback (most recent call last):\n  File \"/app/main.py\""
        result = sanitize_error_message(500, msg)
        assert "Traceback" not in result
        assert result == "An unexpected error occurred"

    def test_internal_paths_filtered(self) -> None:
        msg = "Error in memgraph_store.py line 123"
        result = sanitize_error_message(500, msg)
        assert "memgraph" not in result

    def test_silo_id_filtered(self) -> None:
        msg = "silo_abc123 not found"
        result = sanitize_error_message(404, msg)
        assert "silo_" not in result

    def test_fallback_by_status(self) -> None:
        assert sanitize_error_message(401, None) == "Authentication failed - try logging in again"
        assert sanitize_error_message(429, None) == "Rate limit exceeded - please slow down"


class TestEngrammicError:
    def test_to_dict(self) -> None:
        err = EngrammicError(
            code="invalid_request",
            message="Bad input",
            request_id="req-123",
        )
        assert err.to_dict() == {
            "error": "invalid_request",
            "message": "Bad input",
            "request_id": "req-123",
        }
