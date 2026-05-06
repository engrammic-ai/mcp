"""Pytest fixtures for delta-prime-mcp tests."""

import tempfile
from pathlib import Path
from typing import Generator

import pytest


@pytest.fixture
def temp_credentials_dir() -> Generator[Path, None, None]:
    """Temporary directory for credential storage tests."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def mock_settings(temp_credentials_dir: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Configure settings to use temp directory."""
    monkeypatch.setenv("DELTA_PRIME_BACKEND_URL", "http://localhost:8000")
    monkeypatch.setenv("DELTA_PRIME_CREDENTIALS_PATH", str(temp_credentials_dir / "creds.json"))

    from delta_prime_mcp import config
    config._settings = None
