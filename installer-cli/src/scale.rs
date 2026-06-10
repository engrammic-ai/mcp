//! Resource scaling guidance for self-hosted containers.

use anyhow::{bail, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::process::Command;

/// Get current memory usage from docker stats.
fn get_memory_usage() -> Result<HashMap<String, (u64, u64)>> {
    let output = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}\t{{.MemUsage}}",
        ])
        .output()?;

    if !output.status.success() {
        bail!("docker stats failed - are containers running?");
    }

    let mut usage = HashMap::new();
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            if let Some((used, limit)) = parse_mem_usage(parts[1]) {
                usage.insert(name, (used, limit));
            }
        }
    }

    Ok(usage)
}

fn parse_mem_usage(s: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = s.split(" / ").collect();
    if parts.len() != 2 {
        return None;
    }
    let used = parse_mem_value(parts[0])?;
    let limit = parse_mem_value(parts[1])?;
    Some((used, limit))
}

fn parse_mem_value(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.ends_with("GiB") {
        let val: f64 = s.trim_end_matches("GiB").parse().ok()?;
        Some((val * 1024.0) as u64)
    } else if s.ends_with("MiB") {
        let val: f64 = s.trim_end_matches("MiB").parse().ok()?;
        Some(val as u64)
    } else {
        None
    }
}

pub fn show_status() -> Result<()> {
    println!();
    println!("{}", "Current resource usage:".bold());
    println!();
    println!(
        "  {:<25} {:<12} {:<12} {}",
        "Container", "Used", "Limit", "Usage"
    );
    println!("  {}", "-".repeat(60));

    let usage = get_memory_usage()?;
    let mut high_usage = Vec::new();

    for (name, (used, limit)) in &usage {
        let percent = if *limit > 0 {
            (*used as f64 / *limit as f64 * 100.0) as u32
        } else {
            0
        };
        let warning = if percent >= 80 {
            " !!".yellow().to_string()
        } else {
            "".to_string()
        };
        println!(
            "  {:<25} {:<12} {:<12} {}%{}",
            name,
            format!("{}MB", used),
            format!("{}MB", limit),
            percent,
            warning
        );
        if percent >= 80 {
            let new_limit = (*limit as f64 * 1.5) as u64;
            high_usage.push((name.clone(), *limit, new_limit));
        }
    }

    if !high_usage.is_empty() {
        println!();
        println!(
            "{}",
            "Containers near limit - recommended changes to docker-compose.yml:".yellow()
        );
        println!();
        for (name, old, new) in &high_usage {
            let service = name.trim_start_matches("engrammic-");
            println!("  {}: memory: {}M -> {}M", service, old, new);
        }
        println!();
        println!("After editing docker-compose.yml, run: docker compose up -d");
    } else if usage.is_empty() {
        println!();
        println!("{}", "No containers found. Is the stack running?".yellow());
    } else {
        println!();
        println!("{}", "All containers have healthy memory headroom.".green());
    }

    Ok(())
}
