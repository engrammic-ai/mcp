"""Secure credential storage for Engrammic OAuth tokens."""

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, cast

import structlog

logger = structlog.get_logger(__name__)


def store_credentials(
    access_token: str,
    refresh_token: str,
    path: Path,
) -> None:
    """Store OAuth credentials securely.

    Creates parent directories if needed. Sets file permissions to 600
    (owner read/write only) to protect tokens.
    """
    path.parent.mkdir(parents=True, exist_ok=True)

    data = {
        "access_token": access_token,
        "refresh_token": refresh_token,
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
