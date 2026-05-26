//! Docker detection and compose installation.

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Check if Docker is available and running.
pub fn check_docker() -> Result<bool> {
    let output = Command::new("docker")
        .args(["info"])
        .output();

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}

/// Docker compose template (embedded at compile time).
pub const COMPOSE_TEMPLATE: &str = include_str!("../assets/docker-compose.yml");
pub const README_TEMPLATE: &str = include_str!("../assets/README.md");

/// Write compose file and .env to target directory.
pub fn write_compose_bundle(dir: &Path, license_key: &str) -> Result<()> {
    fs::create_dir_all(dir)?;

    let compose_path = dir.join("docker-compose.yml");
    fs::write(&compose_path, COMPOSE_TEMPLATE)?;

    let readme_path = dir.join("README.md");
    fs::write(&readme_path, README_TEMPLATE)?;

    let env_content = format!(
        r#"# Engrammic Self-Hosted Configuration
ENGRAMMIC_LICENSE_KEY={}

# Database passwords (change in production)
POSTGRES_PASSWORD=engrammic

# Optional: LLM for full SAGE features
# LLM_PROVIDER=openai
# LLM_API_KEY=sk-...

TELEMETRY_ENABLED=true
"#,
        license_key
    );

    let env_path = dir.join(".env");
    fs::write(&env_path, env_content)?;

    Ok(())
}
