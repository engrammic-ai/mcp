"""Configuration from environment variables."""

from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Delta Prime MCP settings."""

    backend_url: str = "https://api.deltaprime.ai"
    api_key: str | None = None
    credentials_path: Path = Path.home() / ".delta-prime" / "credentials.json"

    model_config = SettingsConfigDict(
        env_prefix="DELTA_PRIME_",
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
