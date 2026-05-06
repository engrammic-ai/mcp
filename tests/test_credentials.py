"""Tests for secure credential storage."""

import json
import stat
from pathlib import Path

import pytest

from delta_prime_mcp.credentials import load_credentials, store_credentials


class TestStoreCredentials:
    def test_creates_parent_directory(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "subdir" / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            path=creds_path,
        )
        assert creds_path.exists()

    def test_stores_tokens(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            path=creds_path,
        )
        data = json.loads(creds_path.read_text())
        assert data["access_token"] == "tok_123"
        assert data["refresh_token"] == "ref_456"
        assert "stored_at" in data

    def test_sets_secure_permissions(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            path=creds_path,
        )
        mode = creds_path.stat().st_mode
        assert mode & stat.S_IRWXG == 0  # no group permissions
        assert mode & stat.S_IRWXO == 0  # no other permissions


class TestLoadCredentials:
    def test_returns_none_if_missing(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "nonexistent.json"
        result = load_credentials(creds_path)
        assert result is None

    def test_loads_stored_credentials(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials("tok_123", "ref_456", creds_path)
        result = load_credentials(creds_path)
        assert result is not None
        assert result["access_token"] == "tok_123"
        assert result["refresh_token"] == "ref_456"

    def test_refuses_insecure_permissions(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        creds_path.write_text('{"access_token": "tok"}')
        creds_path.chmod(0o644)  # world-readable
        result = load_credentials(creds_path)
        assert result is None
