"""Tests for secure credential storage."""

import json
import stat
from datetime import UTC, datetime, timedelta
from pathlib import Path

from engrammic_mcp.credentials import clear_credentials, load_credentials, store_credentials


class TestStoreCredentials:
    def test_creates_parent_directory(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "subdir" / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            expires_in=3600,
            path=creds_path,
        )
        assert creds_path.exists()

    def test_stores_tokens(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            expires_in=3600,
            path=creds_path,
        )
        data = json.loads(creds_path.read_text())
        assert data["access_token"] == "tok_123"
        assert data["refresh_token"] == "ref_456"
        assert "stored_at" in data

    def test_stores_expires_at(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        before = datetime.now(UTC)
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            expires_in=3600,
            path=creds_path,
        )
        after = datetime.now(UTC)

        data = json.loads(creds_path.read_text())
        assert "expires_at" in data
        expires_at = datetime.fromisoformat(data["expires_at"])

        expected_min = before + timedelta(seconds=3600)
        expected_max = after + timedelta(seconds=3600)
        assert expected_min <= expires_at <= expected_max

    def test_sets_secure_permissions(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials(
            access_token="tok_123",
            refresh_token="ref_456",
            expires_in=3600,
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
        store_credentials("tok_123", "ref_456", 3600, creds_path)
        result = load_credentials(creds_path)
        assert result is not None
        assert result["access_token"] == "tok_123"
        assert result["refresh_token"] == "ref_456"
        assert "expires_at" in result

    def test_refuses_insecure_permissions(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        creds_path.write_text('{"access_token": "tok"}')
        creds_path.chmod(0o644)  # world-readable
        result = load_credentials(creds_path)
        assert result is None


class TestClearCredentials:
    def test_removes_credentials_file(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        store_credentials("tok_123", "ref_456", 3600, creds_path)
        assert creds_path.exists()

        clear_credentials(creds_path)
        assert not creds_path.exists()

    def test_removes_lock_file(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "creds.json"
        lock_path = creds_path.with_suffix(".lock")
        store_credentials("tok_123", "ref_456", 3600, creds_path)
        lock_path.touch()

        clear_credentials(creds_path)
        assert not lock_path.exists()

    def test_noop_if_missing(self, temp_credentials_dir: Path) -> None:
        creds_path = temp_credentials_dir / "nonexistent.json"
        clear_credentials(creds_path)  # Should not raise
