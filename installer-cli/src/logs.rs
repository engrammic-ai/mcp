//! Log viewing for self-hosted deployments.

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Select;
use std::process::Command;

use crate::user_config::UserConfig;

const SERVICES: &[&str] = &[
    "app",
    "reaction-worker",
    "dagster",
    "dagster-daemon",
    "memgraph",
    "qdrant",
    "redis",
    "postgres",
];

pub fn show_logs(service: Option<&str>, follow: bool, lines: u32) -> Result<()> {
    let dir = UserConfig::dir();
    let compose_path = dir.join("docker-compose.yml");

    if !compose_path.exists() {
        println!(
            "{} No self-hosted installation found at {}",
            "!".yellow(),
            dir.display()
        );
        println!("  Run {} to set up", "engrammic selfhost".cyan());
        anyhow::bail!("No installation found");
    }

    let service_name = match service {
        Some(s) => s.to_string(),
        None => {
            // Interactive selection
            let options: Vec<&str> = std::iter::once("all (combined)")
                .chain(SERVICES.iter().copied())
                .collect();

            let idx = Select::new()
                .with_prompt("Which service?")
                .items(&options)
                .default(0)
                .interact()?;
            options[idx].to_string()
        }
    };

    let lines_str = lines.to_string();
    let compose_str = compose_path.to_str().unwrap();

    let mut args = vec!["compose", "-f", compose_str, "logs", "--tail", &lines_str];

    if follow {
        args.push("-f");
    }

    if service_name != "all (combined)" {
        args.push(&service_name);
    }

    println!("{}", format!("Showing logs for: {}", service_name).dimmed());
    println!();

    Command::new("docker")
        .args(&args)
        .current_dir(&dir)
        .status()
        .context("Failed to run docker compose logs")?;

    Ok(())
}
