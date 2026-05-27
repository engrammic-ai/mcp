//! Docker detection and compose installation.

use anyhow::{Context, Result};
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

/// Upgrade the self-hosted Docker stack to latest version.
pub fn upgrade_docker_stack(dir: &Path) -> Result<()> {
    let compose_path = dir.join("docker-compose.yml");

    if !compose_path.exists() {
        anyhow::bail!(
            "No docker-compose.yml found in {}. Run 'engrammic docker' first to install.",
            dir.display()
        );
    }

    println!("Pulling latest images...");
    let pull_status = Command::new("docker")
        .args(["compose", "-f", compose_path.to_str().unwrap(), "pull"])
        .current_dir(dir)
        .status()
        .context("Failed to run docker compose pull")?;

    if !pull_status.success() {
        anyhow::bail!("docker compose pull failed");
    }

    println!("Restarting services with new images...");
    let up_status = Command::new("docker")
        .args(["compose", "-f", compose_path.to_str().unwrap(), "up", "-d"])
        .current_dir(dir)
        .status()
        .context("Failed to run docker compose up")?;

    if !up_status.success() {
        anyhow::bail!("docker compose up failed");
    }

    println!("\nUpgrade complete! Cleaning up old images...");
    let _ = Command::new("docker")
        .args(["image", "prune", "-f"])
        .status();

    Ok(())
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
