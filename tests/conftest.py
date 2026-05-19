"""Pytest fixtures for engrammic-mcp tests."""

import tempfile
from collections.abc import Generator
from pathlib import Path

import pytest


@pytest.fixture
def temp_credentials_dir() -> Generator[Path]:
    """Temporary directory for credential storage tests."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def mock_settings(temp_credentials_dir: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Configure settings to use temp directory."""
    monkeypatch.setenv("ENGRAMMIC_BACKEND_URL", "http://localhost:8000")
    monkeypatch.setenv("ENGRAMMIC_CREDENTIALS_PATH", str(temp_credentials_dir / "creds.json"))

    from engrammic_mcp import config
    config._settings = None
