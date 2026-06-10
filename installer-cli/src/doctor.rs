//! Diagnostic checks for self-hosted installation.

use anyhow::Result;
use colored::Colorize;
use std::process::Command;

/// Run all self-hosted diagnostics.
///
/// Exit codes:
///   0 — all hard checks passed (warnings may still be printed)
///   1 — one or more hard checks failed; output describes each failure
pub fn run_diagnostics() -> Result<()> {
    println!();
    println!("{}", "Engrammic Diagnostics".bold());
    println!();

    let mut all_passed = true;

    // Check Docker
    print!("Checking Docker... ");
    if check_docker_running() {
        println!("{}", "Running".green());
    } else {
        println!("{}", "Not running".red());
        all_passed = false;
    }

    // Check containers
    print!("Checking containers... ");
    match check_containers() {
        Ok((healthy, total)) => {
            if healthy == total && total > 0 {
                println!("{}", format!("{}/{} healthy", healthy, total).green());
            } else if total == 0 {
                println!("{}", "No containers found".yellow());
            } else {
                println!("{}", format!("{}/{} healthy", healthy, total).yellow());
                all_passed = false;
            }
        }
        Err(_) => {
            println!("{}", "Could not check".red());
            all_passed = false;
        }
    }

    // Check for OOM events
    print!("Checking for OOM events... ");
    match check_oom_events() {
        Ok(events) if events.is_empty() => {
            println!("{}", "None in last hour".green());
        }
        Ok(events) => {
            println!("{}", format!("{} OOM events", events.len()).yellow());
            for event in events {
                println!("  {} was OOM-killed", event.red());
            }
            all_passed = false;
        }
        Err(_) => {
            println!("{}", "Could not check".dimmed());
        }
    }

    // Check license
    print!("Checking license... ");
    match check_license() {
        Ok(days) => {
            if days > 14 {
                println!("{}", format!("Valid ({} days remaining)", days).green());
            } else {
                println!("{}", format!("Expiring soon ({} days)", days).yellow());
            }
        }
        Err(e) => {
            println!("{}", format!("{}", e).red());
            all_passed = false;
        }
    }

    // Check connectivity
    print!("Checking connectivity... ");
    if check_telemetry_endpoint() {
        println!("{}", "tel.engrammic.ai reachable".green());
    } else {
        println!("{}", "tel.engrammic.ai unreachable".yellow());
    }

    // Check disk space
    print!("Checking disk space... ");
    match check_disk_space() {
        Ok(gb) if gb > 10.0 => {
            println!("{}", format!("{:.1}GB free", gb).green());
        }
        Ok(gb) => {
            println!("{}", format!("{:.1}GB free (low)", gb).yellow());
        }
        Err(_) => {
            println!("{}", "Could not check".dimmed());
        }
    }

    println!();
    if all_passed {
        println!("{}", "All checks passed.".green().bold());
        Ok(())
    } else {
        // Print the ✗ / → summary line before exiting.
        eprintln!("  {} One or more checks failed.", "✗".red());
        eprintln!(
            "  {} Review the items marked {} above and address them before continuing.",
            "→".yellow(),
            "red".red()
        );
        // Exit with code 1 so callers (scripts, CI) can detect unhealthy state.
        // We use process::exit rather than Err(...) to avoid printing a redundant
        // anyhow error chain — the output above is already the full diagnosis.
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    /// Document the exit-code contract. Not runnable without Docker;
    /// presence in the test module ensures the function signature stays stable.
    #[test]
    fn run_diagnostics_signature_returns_result() {
        let _: fn() -> anyhow::Result<()> = super::run_diagnostics;
    }
}

fn check_docker_running() -> bool {
    Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_containers() -> Result<(usize, usize)> {
    let output = Command::new("docker")
        .args(["compose", "ps", "--format", "json"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("docker compose ps failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let total = stdout.lines().filter(|l| !l.is_empty()).count();
    let healthy = stdout
        .lines()
        .filter(|line| {
            line.contains("\"Health\":\"healthy\"") || line.contains("\"State\":\"running\"")
        })
        .count();

    Ok((healthy, total))
}

fn check_oom_events() -> Result<Vec<String>> {
    let output = Command::new("docker")
        .args([
            "events",
            "--filter",
            "event=oom",
            "--since",
            "1h",
            "--until",
            "now",
            "--format",
            "{{.Actor.Attributes.name}}",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let events: Vec<String> = stdout
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    Ok(events)
}

fn check_license() -> Result<u32> {
    // First try user config (preferred)
    let user_config = crate::user_config::UserConfig::load()?;
    if let Some(key) = &user_config.license_key {
        let info = crate::license::validate_license_format(key)?;
        return Ok(info.days_remaining);
    }

    // Fall back to .env in selfhost_dir
    if let Some(dir) = &user_config.selfhost_dir {
        let env_path = dir.join(".env");
        if let Ok(env_content) = std::fs::read_to_string(&env_path) {
            for line in env_content.lines() {
                if line.starts_with("ENGRAMMIC_LICENSE_KEY=") {
                    let key = line.trim_start_matches("ENGRAMMIC_LICENSE_KEY=");
                    let info = crate::license::validate_license_format(key)?;
                    return Ok(info.days_remaining);
                }
            }
        }
    }

    anyhow::bail!("License key not found in config or selfhost .env")
}

fn check_telemetry_endpoint() -> bool {
    Command::new("curl")
        .args(["-sf", "--max-time", "5", "https://tel.engrammic.ai/health"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_disk_space() -> Result<f64> {
    let output = Command::new("df").args(["-BG", "."]).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let available = parts[3].trim_end_matches('G');
            return available
                .parse()
                .map_err(|_| anyhow::anyhow!("parse error"));
        }
    }
    anyhow::bail!("Could not parse df output")
}
