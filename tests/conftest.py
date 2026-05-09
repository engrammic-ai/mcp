"""Pytest fixtures for engrammic-mcp tests."""

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
    monkeypatch.setenv("ENGRAMMIC_BACKEND_URL", "http://localhost:8000")
    monkeypatch.setenv("ENGRAMMIC_CREDENTIALS_PATH", str(temp_credentials_dir / "creds.json"))

    from engrammic_mcp import config
    config._settings = None
