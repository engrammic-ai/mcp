"""Engrammic MCP settings."""

from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Engrammic MCP settings."""

    backend_url: str = "https://beta.engrammic.ai/mcp/"
    api_key: str | None = None
    credentials_path: Path = Path.home() / ".engrammic" / "credentials.json"

    model_config = SettingsConfigDict(
        env_prefix="ENGRAMMIC_",
        env_file=".env",
        env_file_encoding="utf-8",
    )


_settings: Settings | None = None


def get_settings() -> Settings:
    """Return cached settings instance."""
    global _settings
    if _settings is None:
        _settings = Settings()
    return _settings


def reset_settings() -> None:
    """Reset the cached settings instance. For testing only."""
    global _settings
    _settings = None
