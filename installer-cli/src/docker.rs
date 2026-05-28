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
pub fn write_compose_bundle(dir: &Path, license_key: &str, telemetry_enabled: bool) -> Result<()> {
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

# Anonymous usage statistics (can be changed anytime)
TELEMETRY_ENABLED={}
"#,
        license_key,
        telemetry_enabled
    );

    let env_path = dir.join(".env");
    fs::write(&env_path, env_content)?;

    Ok(())
}

/// Check if the current compose file differs from the embedded template.
/// Returns list of new services if template has services the current file doesn't.
pub fn check_compose_updates(dir: &Path) -> Result<Option<Vec<String>>> {
    let compose_path = dir.join("docker-compose.yml");

    if !compose_path.exists() {
        return Ok(None);
    }

    let current = fs::read_to_string(&compose_path)?;

    // Simple service detection: look for "  servicename:" pattern
    let current_services: Vec<&str> = current
        .lines()
        .filter(|l| l.starts_with("  ") && l.ends_with(":") && !l.contains("#"))
        .filter_map(|l| l.trim().strip_suffix(':'))
        .collect();

    let template_services: Vec<&str> = COMPOSE_TEMPLATE
        .lines()
        .filter(|l| l.starts_with("  ") && l.ends_with(":") && !l.contains("#"))
        .filter_map(|l| l.trim().strip_suffix(':'))
        .collect();

    let new_services: Vec<String> = template_services
        .iter()
        .filter(|s| !current_services.contains(s))
        .map(|s| s.to_string())
        .collect();

    if new_services.is_empty() {
        Ok(None)
    } else {
        Ok(Some(new_services))
    }
}

/// Refresh compose file with latest template, preserving .env.
pub fn refresh_compose(dir: &Path) -> Result<()> {
    let compose_path = dir.join("docker-compose.yml");
    let backup_path = dir.join("docker-compose.yml.bak");

    // Backup existing
    if compose_path.exists() {
        fs::copy(&compose_path, &backup_path)?;
    }

    // Write new template
    fs::write(&compose_path, COMPOSE_TEMPLATE)?;

    Ok(())
}

/// Update the license key in an existing .env file.
pub fn update_license_key(dir: &Path, license_key: &str) -> Result<()> {
    let env_path = dir.join(".env");

    if !env_path.exists() {
        anyhow::bail!(
            "No .env file found in {}. Run 'engrammic docker' first to install.",
            dir.display()
        );
    }

    let content = fs::read_to_string(&env_path)?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut found = false;

    for line in &mut lines {
        if line.starts_with("ENGRAMMIC_LICENSE_KEY=") {
            *line = format!("ENGRAMMIC_LICENSE_KEY={}", license_key);
            found = true;
            break;
        }
    }

    if !found {
        lines.insert(1, format!("ENGRAMMIC_LICENSE_KEY={}", license_key));
    }

    fs::write(&env_path, lines.join("\n") + "\n")?;

    Ok(())
}
