//! Diagnostic checks for self-hosted installation.

use anyhow::Result;
use colored::Colorize;
use std::process::Command;

use crate::manifest::Manifest;

/// A single doctor-lite verification result.
/// `ok = true` means the check passed. `detail` is the human-readable line
/// printed after the check label (e.g. "reachable", "config parse error: …").
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

/// Classification input for the endpoint reachability check.
/// Separates the network call from the classification logic so the
/// classification is unit-testable without a live network.
#[derive(Debug)]
pub enum ReachError {
    Connect,
    Timeout,
}

/// Returns true iff the outcome represents a live server.
/// Any HTTP status code counts; only transport errors fail the check.
pub fn classify_http_outcome(outcome: Result<u16, ReachError>) -> bool {
    outcome.is_ok()
}

/// Perform a HEAD request to `url` and return the HTTP status code,
/// or a `ReachError` if the connection failed or timed out.
/// 405/406 both count as reachable (see spec: doctor-lite semantics).
fn head_reachable(url: &str) -> Result<u16, ReachError> {
    match ureq::head(url)
        .timeout(std::time::Duration::from_secs(8))
        .call()
    {
        Ok(resp) => Ok(resp.status()),
        Err(ureq::Error::Status(code, _)) => Ok(code),
        Err(ureq::Error::Transport(t)) => {
            // Distinguish timeout vs other connect errors for the error enum.
            // ureq::Transport does not expose a typed variant for timeout in v2;
            // stringify and match as a best-effort classification.
            let msg = t.to_string().to_ascii_lowercase();
            if msg.contains("timed out") || msg.contains("timeout") {
                Err(ReachError::Timeout)
            } else {
                Err(ReachError::Connect)
            }
        }
    }
}

/// Lightweight verification run at the end of a fresh install.
/// Does not print anything — the caller formats and emits the results.
///
/// Checks performed:
/// - Cloud mode: HEAD reachability of the configured endpoint host.
/// - Self-hosted mode: Docker running + containers healthy (re-uses private helpers).
/// - Both: each harness config file in the manifest parses without error.
pub fn lite(manifest: &Manifest) -> Vec<CheckResult> {
    let mut results: Vec<CheckResult> = Vec::new();

    // --- Endpoint reachability ---
    let is_cloud = manifest
        .endpoint
        .as_deref()
        .map(|ep| ep.contains("engrammic.ai"))
        .unwrap_or(false);

    if is_cloud {
        let endpoint = manifest
            .endpoint
            .as_deref()
            .unwrap_or(crate::tools::CLOUD_ENDPOINT);
        let outcome = head_reachable(endpoint);
        let reachable = classify_http_outcome(outcome);
        results.push(CheckResult {
            label: "endpoint reachable".to_string(),
            ok: reachable,
            detail: if reachable {
                format!("{} is up", endpoint)
            } else {
                format!(
                    "could not reach {}  →  check your internet connection",
                    endpoint
                )
            },
        });
    } else if manifest.selfhost_dir.is_some() {
        // Self-hosted: re-use the existing Docker + container checks.
        let docker_ok = check_docker_running();
        results.push(CheckResult {
            label: "Docker running".to_string(),
            ok: docker_ok,
            detail: if docker_ok {
                "running".to_string()
            } else {
                "not running  →  run `docker info` to diagnose".to_string()
            },
        });

        if docker_ok {
            match check_containers() {
                Ok((healthy, total)) if total > 0 => {
                    let ok = healthy == total;
                    results.push(CheckResult {
                        label: "containers healthy".to_string(),
                        ok,
                        detail: format!("{}/{} healthy", healthy, total),
                    });
                }
                Ok(_) => {
                    results.push(CheckResult {
                        label: "containers healthy".to_string(),
                        ok: false,
                        detail: "no containers found  →  run `docker compose up -d`".to_string(),
                    });
                }
                Err(e) => {
                    results.push(CheckResult {
                        label: "containers healthy".to_string(),
                        ok: false,
                        detail: format!("could not check: {e:#}"),
                    });
                }
            }
        }
    }

    // --- Harness config files parse cleanly ---
    let harness_check = if manifest.harnesses.is_empty() {
        CheckResult {
            label: "harness configs".to_string(),
            ok: true,
            detail: "none configured".to_string(),
        }
    } else {
        let mut failed: Vec<String> = Vec::new();
        for h in &manifest.harnesses {
            match std::fs::read_to_string(&h.config_path) {
                Err(_) => {
                    // Missing file is a warning, not hard failure.
                    // (user may have moved/deleted the editor)
                }
                Ok(content) => {
                    let ext = h
                        .config_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    let parse_ok = match ext {
                        "toml" => toml::from_str::<toml::Value>(&content).is_ok(),
                        _ => serde_json::from_str::<serde_json::Value>(&content).is_ok(),
                    };
                    if !parse_ok {
                        failed.push(h.config_path.display().to_string());
                    }
                }
            }
        }
        if failed.is_empty() {
            CheckResult {
                label: "harness configs".to_string(),
                ok: true,
                detail: format!("{} file(s) parse cleanly", manifest.harnesses.len()),
            }
        } else {
            CheckResult {
                label: "harness configs".to_string(),
                ok: false,
                detail: format!("parse error in: {}", failed.join(", ")),
            }
        }
    };
    results.push(harness_check);

    results
}

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
            // Advisory only: a past OOM kill doesn't mean the stack is broken
            // now, so it must not flip the exit code to 1.
            println!("{}", format!("{} OOM events", events.len()).yellow());
            for event in events {
                println!("  {} was OOM-killed", event.yellow());
            }
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
    use super::*;

    /// Document the exit-code contract. Not runnable without Docker;
    /// presence in the test module ensures the function signature stays stable.
    #[test]
    fn run_diagnostics_signature_returns_result() {
        let _: fn() -> anyhow::Result<()> = super::run_diagnostics;
    }

    #[test]
    fn classify_http_response_is_reachable() {
        // Any HTTP status code means the server is up (even 405/406).
        // Only a connect/timeout error means unreachable.
        let ok_statuses = [200u16, 405, 406, 404, 500];
        for status in ok_statuses {
            assert!(
                classify_http_outcome(Ok(status)),
                "HTTP {status} must be classified as reachable"
            );
        }
    }

    #[test]
    fn classify_connect_error_is_not_reachable() {
        // A transport-level error means unreachable.
        assert!(
            !classify_http_outcome(Err(ReachError::Connect)),
            "connect error must be not-reachable"
        );
        assert!(
            !classify_http_outcome(Err(ReachError::Timeout)),
            "timeout must be not-reachable"
        );
    }

    #[test]
    #[ignore = "live network call; run explicitly with --ignored"]
    fn lite_returns_one_check_per_domain() {
        // With an empty manifest and a cloud endpoint, lite must return
        // at least the endpoint check and the harness check (zero harnesses = pass).
        let m = crate::manifest::Manifest {
            endpoint: Some("https://beta.engrammic.ai/mcp/".to_string()),
            selfhost_dir: None,
            harnesses: vec![],
            ..Default::default()
        };
        let results = lite(&m);
        assert!(
            results.iter().any(|r| r.label.contains("endpoint")),
            "must include an endpoint check"
        );
        assert!(
            results.iter().any(|r| r.label.contains("harness")),
            "must include a harness config check"
        );
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
