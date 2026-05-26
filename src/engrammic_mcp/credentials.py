"""Secure credential storage for Engrammic OAuth tokens."""

import json
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any, cast

import structlog

logger = structlog.get_logger(__name__)


def store_credentials(
    access_token: str,
    refresh_token: str,
    expires_in: int,
    path: Path,
) -> None:
    """Store OAuth credentials securely.

    Creates parent directories if needed. Sets file permissions to 600
    (owner read/write only) to protect tokens.

    Args:
        access_token: The OAuth access token.
        refresh_token: The OAuth refresh token.
        expires_in: Token lifetime in seconds.
        path: Path to store credentials file.
    """
    path.parent.mkdir(parents=True, exist_ok=True)

    expires_at = datetime.now(UTC) + timedelta(seconds=expires_in)
    data = {
        "access_token": access_token,
        "refresh_token": refresh_token,
        "expires_at": expires_at.isoformat(),
        "stored_at": datetime.now(UTC).isoformat(),
    }

    path.write_text(json.dumps(data, indent=2))
    path.chmod(0o600)

    logger.info("Credentials stored", path=str(path))


def load_credentials(path: Path) -> dict[str, Any] | None:
    """Load stored credentials if they exist and have secure permissions.

    Returns None if:
    - File doesn't exist
    - File has insecure permissions (group or world readable)
    - File is not valid JSON
    """
    if not path.exists():
        return None

    mode = path.stat().st_mode
    if mode & 0o077:
        logger.warning(
            "Credentials file has insecure permissions, refusing to read",
            path=str(path),
            mode=oct(mode),
        )
        return None

    try:
        return cast(dict[str, Any], json.loads(path.read_text()))
    except (json.JSONDecodeError, OSError) as e:
        logger.warning("Failed to load credentials", path=str(path), error=str(e))
        return None


def clear_credentials(path: Path) -> None:
    """Remove stored credentials."""
    if path.exists():
        path.unlink()
        logger.info("Credentials cleared", path=str(path))

    lock_path = path.with_suffix(".lock")
    if lock_path.exists():
        lock_path.unlink()
