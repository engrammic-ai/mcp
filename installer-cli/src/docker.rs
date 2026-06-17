//! Docker detection and compose installation.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Check if Docker is available and running.
pub fn check_docker() -> Result<bool> {
    let output = Command::new("docker").args(["info"]).output();

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

/// Docker compose templates (embedded at compile time).
pub const COMPOSE_TEMPLATE: &str = include_str!("../assets/docker-compose.yml");
pub const COMPOSE_LITE: &str = include_str!("../assets/docker-compose.lite.yml");
pub const COMPOSE_STANDARD: &str = include_str!("../assets/docker-compose.standard.yml");
pub const COMPOSE_PRO: &str = include_str!("../assets/docker-compose.pro.yml");

/// Get list of services from a compose file using docker compose.
fn get_compose_services(file: &Path) -> Result<Vec<String>> {
    let output = Command::new("docker")
        .args(["compose", "-f", file.to_str().unwrap(), "config", "--services"])
        .output()
        .context("Failed to run docker compose config")?;

    if !output.status.success() {
        anyhow::bail!("docker compose config failed");
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}

/// Check if the current compose file differs from the embedded template.
/// Returns list of new services if template has services the current file doesn't.
pub fn check_compose_updates(dir: &Path) -> Result<Option<Vec<String>>> {
    let compose_path = dir.join("docker-compose.yml");

    if !compose_path.exists() {
        return Ok(None);
    }

    let current_services = get_compose_services(&compose_path)?;

    // Write the embedded template to a temp file so docker compose can parse it.
    let tmp_path = std::env::temp_dir().join(format!(
        "engrammic-compose-template-{}.yml",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    fs::write(&tmp_path, COMPOSE_TEMPLATE).context("Failed to write template to temp file")?;
    let template_services = get_compose_services(&tmp_path);
    let _ = fs::remove_file(&tmp_path);
    let template_services = template_services?;

    let new_services: Vec<String> = template_services
        .into_iter()
        .filter(|s| !current_services.contains(s))
        .collect();

    if new_services.is_empty() {
        Ok(None)
    } else {
        Ok(Some(new_services))
    }
}

/// Refresh compose file with latest template, preserving .env and re-applying customizations.
pub fn refresh_compose(dir: &Path) -> Result<()> {
    let compose_path = dir.join("docker-compose.yml");
    let backup_path = dir.join("docker-compose.yml.bak");
    let env_path = dir.join(".env");

    // Backup existing
    if compose_path.exists() {
        fs::copy(&compose_path, &backup_path)?;
    }

    // Start with base template
    let mut compose = COMPOSE_TEMPLATE.to_string();

    // Check .env for TEI reranker configuration
    if env_path.exists() {
        let env_content = fs::read_to_string(&env_path).unwrap_or_default();

        // If TEI_RERANKER_URL is set to localhost:8082, inject TEI service
        if env_content.contains("TEI_RERANKER_URL=http://localhost:8082") {
            compose = crate::selfhost::inject_tei_reranker_service(&compose);
        }
    }

    // Write updated compose
    fs::write(&compose_path, compose)?;

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
