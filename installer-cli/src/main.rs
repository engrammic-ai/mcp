mod banner;
mod cli;
mod config;
mod doctor;
mod docker;
mod license;
mod scale;
mod skill_format;
mod skills;
mod tools;
mod user_config;

pub use skill_format::*;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use inquire::{
    ui::{Attributes, Color, RenderConfig, StyleSheet, Styled},
    Confirm, MultiSelect, Select, Text,
};

use cli::{Cli, Commands};
use tools::{SkillDest, Tool, CLOUD_ENDPOINT, LOCAL_ENDPOINT};

const VIOLET: Color = Color::Rgb { r: 0x7E, g: 0x57, b: 0xC2 };

fn render_config() -> RenderConfig<'static> {
    RenderConfig::default()
        .with_prompt_prefix(Styled::new("▸").with_fg(VIOLET))
        .with_highlighted_option_prefix(Styled::new("▶").with_fg(VIOLET))
        .with_scroll_up_prefix(Styled::new("▲").with_fg(VIOLET))
        .with_scroll_down_prefix(Styled::new("▼").with_fg(VIOLET))
        .with_selected_checkbox(Styled::new("■").with_fg(Color::LightGreen))
        .with_unselected_checkbox(Styled::new("□").with_fg(Color::DarkGrey))
        .with_help_message(StyleSheet::new().with_fg(VIOLET).with_attr(Attributes::ITALIC))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Install) {
        Commands::Install => install(cli.yes, cli.tool.as_deref(), cli.skill_path.as_deref()),
        Commands::Update => update(cli.yes, cli.tool.as_deref(), cli.skill_path.as_deref()),
        Commands::Uninstall => uninstall(cli.yes, cli.tool.as_deref()),
        Commands::Status => status(),
        Commands::Docker => install_docker(),
        Commands::Upgrade => upgrade_docker(),
        Commands::Scale => scale::show_status(),
        Commands::Doctor => doctor::run_diagnostics(),
        Commands::License => manage_license(),
    }
}

fn handle_returning_user(config: user_config::UserConfig, tool_id: Option<&str>, skill_path: Option<&str>) -> Result<()> {
    let endpoint = config.endpoint.as_deref().unwrap_or(CLOUD_ENDPOINT);
    let is_self_hosted = endpoint == LOCAL_ENDPOINT;

    println!("{}", "Current setup".bold());
    println!(
        "  Mode: {}",
        if is_self_hosted { "Self-hosted".cyan() } else { "Cloud".cyan() }
    );
    println!("  Endpoint: {}", endpoint.dimmed());

    if is_self_hosted {
        if let Some(ref key) = config.license_key {
            match license::validate_license_format(key) {
                Ok(info) => {
                    println!(
                        "  License: {} ({} days remaining)",
                        info.customer.cyan(),
                        info.days_remaining
                    );
                }
                Err(_) => {
                    println!("  License: {}", "invalid or expired".yellow());
                }
            }
        }
    }
    println!();

    let mut options = vec![
        "Add or update harnesses",
        "Refresh skills",
        "View full status",
    ];

    if is_self_hosted {
        options.push("Update license key");
    }

    options.push("Start fresh (reconfigure everything)");

    let choice = Select::new("What would you like to do?", options)
        .with_render_config(render_config())
        .prompt()?;

    match choice {
        "Add or update harnesses" => {
            let tools = select_tools(false, tool_id)?;
            if tools.is_empty() {
                println!("{} No harness selected.", "!".yellow());
                return Ok(());
            }

            println!("{}", "Configuring harnesses".bold());
            for tool in &tools {
                let result = config::install(&tool.config_path, endpoint)?;
                match result {
                    config::InstallResult::Created => {
                        println!("  {} {} {}", "✓".green(), tool.name, "(added)".dimmed());
                    }
                    config::InstallResult::Updated { old_url } => {
                        println!(
                            "  {} {} {}",
                            "✓".green(),
                            tool.name,
                            format!("(updated: {} -> {})", old_url, endpoint).dimmed()
                        );
                    }
                    config::InstallResult::Unchanged => {
                        println!("  {} {} {}", "-".dimmed(), tool.name, "(unchanged)".dimmed());
                    }
                }
            }
            println!();
            println!("{} Done.", "✓".green());
        }

        "Refresh skills" => {
            install_skills_step(false, skill_path)?;
            println!();
            println!("{} Skills refreshed.", "✓".green());
        }

        "View full status" => {
            status()?;
        }

        "Update license key" => {
            manage_license()?;
        }

        "Start fresh (reconfigure everything)" => {
            let endpoint = select_deployment_mode(&config)?;
            run_full_install(endpoint, false, tool_id, skill_path)?;
        }

        _ => {}
    }

    Ok(())
}

fn install(yes: bool, tool_id: Option<&str>, skill_path: Option<&str>) -> Result<()> {
    banner::print_banner();

    let existing_config = user_config::UserConfig::load().unwrap_or_default();
    let has_existing_setup = existing_config.endpoint.is_some();

    // For returning users (not -y mode), show menu
    if has_existing_setup && !yes {
        return handle_returning_user(existing_config, tool_id, skill_path);
    }

    // Fresh install or -y mode
    let endpoint = if yes {
        existing_config.endpoint.unwrap_or_else(|| CLOUD_ENDPOINT.to_string())
    } else {
        select_deployment_mode(&existing_config)?
    };

    run_full_install(endpoint, yes, tool_id, skill_path)
}

fn run_full_install(endpoint: String, yes: bool, tool_id: Option<&str>, skill_path: Option<&str>) -> Result<()> {
    let tools = select_tools(yes, tool_id)?;
    if tools.is_empty() {
        println!("{} No harness selected.", "!".yellow());
        println!();
        println!("Add this to your MCP config manually:");
        println!();
        println!(
            r#"  "engrammic": {{ "type": "http", "url": "{}" }}"#,
            endpoint
        );
        println!();
        return Ok(());
    }

    println!("{}", "Configuring harnesses".bold());
    println!(
        "  {}",
        "Only the 'engrammic' MCP server entry is modified; other servers are preserved.".dimmed()
    );
    println!();
    for tool in &tools {
        let result = config::install(&tool.config_path, &endpoint)?;
        match result {
            config::InstallResult::Created => {
                println!(
                    "  {} {} {}",
                    "✓".green(),
                    tool.name,
                    "(added engrammic)".dimmed()
                );
            }
            config::InstallResult::Updated { old_url } => {
                println!(
                    "  {} {} {}",
                    "✓".green(),
                    tool.name,
                    format!("(updated: {} -> {})", old_url, endpoint).dimmed()
                );
            }
            config::InstallResult::Unchanged => {
                println!(
                    "  {} {} {}",
                    "-".dimmed(),
                    tool.name,
                    "(already configured)".dimmed()
                );
            }
        }
        println!("    {}", tool.config_path.display().to_string().dimmed());
    }
    println!();

    install_skills_step(yes, skill_path)?;

    // Save config so returning users get the menu
    let existing = user_config::UserConfig::load().unwrap_or_default();
    let config = user_config::UserConfig {
        endpoint: Some(endpoint),
        license_key: existing.license_key,
    };
    config.save()?;

    println!();
    println!(
        "Done. Tools available: {}",
        "remember, recall, learn, believe, trace, link".dimmed()
    );
    println!();

    offer_cli_install(yes)?;

    Ok(())
}

fn select_deployment_mode(existing_config: &user_config::UserConfig) -> Result<String> {
    let mode = Select::new(
        "Deployment mode",
        vec![
            "Cloud - connect to mcp.engrammic.ai (free tier available)",
            "Self-hosted - run locally with Docker (license required)",
        ],
    )
    .with_help_message("Self-hosted requires Docker and a license key")
    .with_render_config(render_config())
    .prompt()?;

    if mode.starts_with("Self-hosted -") {
        run_docker_setup(existing_config)
    } else {
        Ok(CLOUD_ENDPOINT.to_string())
    }
}

fn prompt_for_license(default: Option<&str>) -> Result<String> {
    let mut prompt = Text::new("License key")
        .with_help_message("Starts with ENGR_ - get yours at engrammic.ai/self-hosted")
        .with_render_config(render_config());

    if let Some(key) = default {
        prompt = prompt.with_default(key);
    }

    let license_key = prompt.prompt()?;

    println!("{}", "Validating license".bold());
    match license::validate_license_format(&license_key) {
        Ok(info) => {
            println!(
                "  {} Valid - customer: {}, {} days remaining",
                "✓".green(),
                info.customer.cyan(),
                info.days_remaining
            );
            Ok(license_key)
        }
        Err(e) => {
            println!("  {} {}", "✗".red(), e);
            anyhow::bail!("Invalid license");
        }
    }
}

fn run_docker_setup(existing_config: &user_config::UserConfig) -> Result<String> {
    println!();
    println!("{}", "Checking Docker".bold());
    if !docker::check_docker()? {
        println!(
            "{} Docker is not running or not installed.",
            "✗".red()
        );
        println!(
            "  Install Docker Desktop from {} then try again.",
            "https://docs.docker.com/get-docker/".cyan()
        );
        anyhow::bail!("Docker not available");
    }
    println!("  {} Docker is running", "✓".green());
    println!();

    let license_key = if let Some(ref key) = existing_config.license_key {
        println!("{}", "Existing license".bold());
        match license::validate_license_format(key) {
            Ok(info) => {
                println!("  Customer: {}", info.customer.cyan());
                println!("  Days remaining: {}", info.days_remaining);
                println!();

                let keep = Confirm::new("Keep this license?")
                    .with_default(true)
                    .with_render_config(render_config())
                    .prompt()?;

                if keep {
                    key.clone()
                } else {
                    prompt_for_license(None)?
                }
            }
            Err(e) => {
                println!("  {} {} - needs update", "!".yellow(), e);
                println!();
                prompt_for_license(Some(key))?
            }
        }
    } else {
        prompt_for_license(None)?
    };

    println!();

    let default_dir = user_config::UserConfig::dir();
    let default_dir_str = default_dir.display().to_string();

    let install_dir = Text::new("Install directory")
        .with_default(&default_dir_str)
        .with_help_message("Compose file and .env will be written here")
        .with_render_config(render_config())
        .prompt()?;

    let dir = std::path::Path::new(&install_dir);

    // Telemetry consent.
    let telemetry_enabled = Confirm::new("Share anonymous usage statistics?")
        .with_default(true)
        .with_help_message("Helps improve Engrammic. Can be changed later in .env")
        .with_render_config(render_config())
        .prompt()?;

    println!("{}", "Writing compose bundle".bold());
    docker::write_compose_bundle(dir, &license_key, telemetry_enabled)?;
    println!(
        "  {} {}",
        "✓".green(),
        dir.join("docker-compose.yml").display().to_string().dimmed()
    );
    println!(
        "  {} {}",
        "✓".green(),
        dir.join(".env").display().to_string().dimmed()
    );
    println!();

    let endpoint = Text::new("MCP endpoint URL")
        .with_default(LOCAL_ENDPOINT)
        .with_help_message("Change if running on a different host/port")
        .with_render_config(render_config())
        .prompt()?;

    println!();
    println!("{}", "Next steps after install completes".bold());
    println!(
        "  1. Review {} and set a strong POSTGRES_PASSWORD",
        dir.join(".env").display().to_string().cyan()
    );
    println!(
        "  2. Run {} to start services",
        format!("docker compose -f {} up -d", dir.join("docker-compose.yml").display()).cyan()
    );
    println!();
    println!(
        "  Harnesses will be configured to use: {}",
        endpoint.cyan()
    );
    println!(
        "  To change later: edit {} or {}",
        user_config::UserConfig::path().display().to_string().cyan(),
        "engrammic.url in your harness config".cyan()
    );
    println!();

    let new_config = user_config::UserConfig {
        endpoint: Some(endpoint.clone()),
        license_key: Some(license_key),
    };
    new_config.save()?;
    println!(
        "{} Saved config to {}",
        "✓".green(),
        user_config::UserConfig::path().display()
    );

    Ok(endpoint)
}

fn update(yes: bool, tool_id: Option<&str>, skill_path: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(yes, tool_id)?;
    for tool in &tools {
        let endpoint = detect_installed_endpoint(&tool.config_path);
        if let Some(ep) = endpoint {
            let _ = config::install(&tool.config_path, &ep)?;
            println!("{} Refreshed engrammic in {}", "✓".green(), tool.name);
        } else {
            println!("{} Not installed for {}", "!".yellow(), tool.name);
        }
    }

    // Refresh skills - use custom path if provided, otherwise refresh existing
    if let Some(custom_path) = skill_path {
        let path = std::path::PathBuf::from(custom_path);
        let results = skills::install_skills_to_paths(&[path])?;
        for (p, count) in results {
            println!("{} Refreshed {} skills in {}", "✓".green(), count, p.display());
        }
    } else {
        let dests_with_skills: Vec<SkillDest> = SkillDest::all()
            .into_iter()
            .filter(|d| skills::count_skills_formatted(d) > 0)
            .collect();

        if !dests_with_skills.is_empty() {
            let results = skills::install_skills(&dests_with_skills)?;
            for (path, count) in results {
                println!(
                    "{} Refreshed {} skills in {}",
                    "✓".green(),
                    count,
                    path.display()
                );
            }
        }
    }

    Ok(())
}

fn uninstall(yes: bool, tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(yes, tool_id)?;
    for tool in &tools {
        config::uninstall(&tool.config_path)?;
        println!("{} Removed engrammic from {}", "✓".green(), tool.name);
    }

    for dest in SkillDest::all() {
        let removed = skills::remove_skills_formatted(&dest)?;
        if removed > 0 {
            println!(
                "{} Removed {} skills from {}",
                "✓".green(),
                removed,
                dest.path.display()
            );
        }
    }

    Ok(())
}

fn detect_installed_endpoint(config_path: &std::path::Path) -> Option<String> {
    if config::is_installed(config_path, CLOUD_ENDPOINT) {
        Some(CLOUD_ENDPOINT.to_string())
    } else if config::is_installed(config_path, LOCAL_ENDPOINT) {
        Some(LOCAL_ENDPOINT.to_string())
    } else {
        None
    }
}

fn status() -> Result<()> {
    banner::print_banner();

    println!("{}", "Harnesses".bold());
    let mut any_installed = false;
    let mut has_cloud = false;
    let mut has_local = false;
    for tool in Tool::all() {
        let endpoint = detect_installed_endpoint(&tool.config_path);
        let label = if let Some(ep) = endpoint {
            any_installed = true;
            if ep == CLOUD_ENDPOINT {
                has_cloud = true;
                format!("{:<18}", "✓ cloud").green()
            } else {
                has_local = true;
                format!("{:<18}", "✓ self-hosted").green()
            }
        } else if tool.config_path.parent().map(|p| p.exists()).unwrap_or(false) {
            format!("{:<18}", "- not configured").dimmed()
        } else {
            format!("{:<18}", "- not detected").dimmed()
        };
        println!("  {} {}", label, tool.name);
    }

    if has_cloud {
        println!();
        println!("  Cloud endpoint: {}", CLOUD_ENDPOINT.cyan());
    }
    if has_local {
        println!();
        println!("  Self-hosted endpoint: {}", LOCAL_ENDPOINT.cyan());
        println!("  To change: edit the {} key in your harness config", "engrammic.url".cyan());
    }

    println!();
    println!("{}", "Skills".bold());
    for dest in SkillDest::all() {
        let count = skills::count_skills_formatted(&dest);
        let label = if count > 0 {
            format!("{:<18}", format!("✓ {} skills", count)).green()
        } else {
            format!("{:<18}", "- none").dimmed()
        };
        println!("  {} {}", label, dest.name);
    }

    if !any_installed {
        println!();
        println!("Run {} to install", "engrammic".cyan());
    }

    Ok(())
}

fn select_tools(yes: bool, tool_id: Option<&str>) -> Result<Vec<Tool>> {
    // Explicit --tool flag wins.
    if let Some(id) = tool_id {
        let tool = Tool::from_id(id).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool: {}. Use: claude, cursor, windsurf, antigravity, gemini, pi",
                id
            )
        })?;
        return Ok(vec![tool]);
    }

    let detected = Tool::detect_installed();

    // -y with detected harnesses: take all detected, no prompt.
    if yes && !detected.is_empty() {
        for tool in &detected {
            println!("Auto-selected: {}", tool.name.cyan());
        }
        return Ok(detected);
    }

    let all_tools = Tool::all();
    let options: Vec<&str> = all_tools.iter().map(|t| t.name).collect();

    let selection = MultiSelect::new("Select harnesses to configure", options)
        .with_help_message("↑↓ move · space toggle · enter confirm")
        .with_render_config(render_config())
        .prompt()?;

    Ok(all_tools
        .into_iter()
        .filter(|t| selection.contains(&t.name))
        .collect())
}

fn install_docker() -> Result<()> {
    banner::print_banner();

    let existing_config = user_config::UserConfig::load().unwrap_or_default();

    // Check Docker is available and running.
    println!("{}", "Checking Docker".bold());
    if !docker::check_docker()? {
        println!(
            "{} Docker is not running or not installed.",
            "✗".red()
        );
        println!(
            "  Install Docker Desktop from {} then try again.",
            "https://docs.docker.com/get-docker/".cyan()
        );
        return Ok(());
    }
    println!("  {} Docker is running", "✓".green());
    println!();

    // Prompt for license key.
    let mut license_prompt = Text::new("License key")
        .with_help_message("Starts with ENGR_ - get yours at engrammic.ai/self-hosted")
        .with_render_config(render_config());

    if let Some(ref key) = existing_config.license_key {
        license_prompt = license_prompt.with_default(key);
    }

    let license_key = license_prompt.prompt()?;

    // Validate format client-side (full validation is server-side).
    println!("{}", "Validating license".bold());
    match license::validate_license_format(&license_key) {
        Ok(info) => {
            println!(
                "  {} Valid - customer: {}, {} days remaining",
                "✓".green(),
                info.customer.cyan(),
                info.days_remaining
            );
        }
        Err(e) => {
            println!("  {} {}", "✗".red(), e);
            return Ok(());
        }
    }
    println!();

    // Prompt for install directory.
    let default_dir = user_config::UserConfig::dir();
    let default_dir_str = default_dir.display().to_string();

    let install_dir = Text::new("Install directory")
        .with_default(&default_dir_str)
        .with_help_message("Compose file and .env will be written here")
        .with_render_config(render_config())
        .prompt()?;

    let dir = std::path::Path::new(&install_dir);

    // Telemetry consent.
    let telemetry_enabled = Confirm::new("Share anonymous usage statistics?")
        .with_default(true)
        .with_help_message("Helps improve Engrammic. Can be changed later in .env")
        .with_render_config(render_config())
        .prompt()?;

    // Write compose bundle.
    println!("{}", "Writing compose bundle".bold());
    docker::write_compose_bundle(dir, &license_key, telemetry_enabled)?;
    println!(
        "  {} {}",
        "✓".green(),
        dir.join("docker-compose.yml").display().to_string().dimmed()
    );
    println!(
        "  {} {}",
        "✓".green(),
        dir.join(".env").display().to_string().dimmed()
    );
    println!();

    // Save config.
    let new_config = user_config::UserConfig {
        endpoint: Some(LOCAL_ENDPOINT.to_string()),
        license_key: Some(license_key),
    };
    new_config.save()?;
    println!(
        "{} Saved config to {}",
        "✓".green(),
        user_config::UserConfig::path().display()
    );
    println!();

    // Print next steps.
    println!("{}", "Next steps".bold());
    println!(
        "  1. Review {} and set a strong POSTGRES_PASSWORD",
        dir.join(".env").display().to_string().cyan()
    );
    println!(
        "  2. Run {} to start all services",
        format!("docker compose -f {} up -d", dir.join("docker-compose.yml").display()).cyan()
    );
    println!(
        "  3. MCP endpoint will be available at {}",
        LOCAL_ENDPOINT.cyan()
    );
    println!();
    println!(
        "Run {} to configure your harness.",
        "engrammic".cyan()
    );

    Ok(())
}

fn install_skills_step(yes: bool, skill_path: Option<&str>) -> Result<()> {
    // If custom skill path provided, use it directly (Directory format assumed)
    if let Some(custom_path) = skill_path {
        let path = std::path::PathBuf::from(custom_path);
        let results = skills::install_skills_to_paths(&[path])?;
        println!("{}", "Installing skills".bold());
        for (p, count) in results {
            println!(
                "  {} {} skills  {}",
                "✓".green(),
                count,
                p.display().to_string().dimmed()
            );
        }
        return Ok(());
    }

    let proceed = if yes {
        true
    } else {
        Confirm::new("Also install 21 Engrammic skills?")
            .with_default(true)
            .with_render_config(render_config())
            .prompt()?
    };

    if !proceed {
        println!("  {} Skipped skills.", "-".dimmed());
        return Ok(());
    }

    let all_dests = SkillDest::all();
    let chosen: Vec<&SkillDest> = if yes {
        all_dests.iter().filter(|d| d.default).collect()
    } else {
        let options: Vec<String> = all_dests
            .iter()
            .map(|d| {
                let scope = match d.scope {
                    tools::SkillScope::User => "(user)",
                    tools::SkillScope::Project => "(project)",
                };
                format!("{:<25} {}", d.name, scope.dimmed())
            })
            .collect();

        let picked = MultiSelect::new("Install skills to", options.iter().map(|s| s.as_str()).collect())
            .with_help_message("↑↓ move · space toggle · enter confirm")
            .with_render_config(render_config())
            .prompt()?;

        all_dests
            .iter()
            .enumerate()
            .filter(|(i, _)| picked.contains(&options[*i].as_str()))
            .map(|(_, d)| d)
            .collect()
    };

    if chosen.is_empty() {
        println!("  {} No skill destination selected.", "-".dimmed());
        return Ok(());
    }

    let dests_vec: Vec<SkillDest> = chosen.into_iter().cloned().collect();
    let results = skills::install_skills(&dests_vec)?;

    println!("{}", "Installing skills".bold());
    for (path, count) in results {
        println!(
            "  {} {} skills  {}",
            "✓".green(),
            count,
            path.display().to_string().dimmed()
        );
    }
    Ok(())
}

fn offer_cli_install(yes: bool) -> Result<()> {
    let install_cli = if yes {
        false
    } else {
        Confirm::new("Install the Engrammic CLI for future updates?")
            .with_default(true)
            .with_help_message("Allows running 'engrammic update', 'engrammic status', etc.")
            .with_render_config(render_config())
            .prompt()?
    };

    if !install_cli {
        return Ok(());
    }

    let current_exe = std::env::current_exe()?;
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let local_bin = home.join(".local").join("bin");

    std::fs::create_dir_all(&local_bin)?;

    let dest = local_bin.join("engrammic");
    std::fs::copy(&current_exe, &dest)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }

    println!(
        "  {} Installed to {}",
        "✓".green(),
        dest.display().to_string().cyan()
    );

    let path_env = std::env::var("PATH").unwrap_or_default();
    let local_bin_str = local_bin.display().to_string();
    if !path_env.split(':').any(|p| p == local_bin_str) {
        println!();
        println!(
            "  {} Add to your shell config:",
            "!".yellow()
        );
        println!(
            "    {}",
            format!("export PATH=\"$HOME/.local/bin:$PATH\"").cyan()
        );
        println!();
        println!(
            "  Then run {} anytime to update or check status.",
            "engrammic".cyan()
        );
    } else {
        println!(
            "  Run {} anytime to update or check status.",
            "engrammic".cyan()
        );
    }

    Ok(())
}

fn upgrade_docker() -> Result<()> {
    banner::print_banner();

    let config = user_config::UserConfig::load().unwrap_or_default();
    let dir = user_config::UserConfig::dir();

    if config.endpoint.as_deref() != Some(LOCAL_ENDPOINT) {
        println!(
            "{} No self-hosted installation found.",
            "!".yellow()
        );
        println!("  Run {} first to install the Docker stack.", "engrammic docker".cyan());
        return Ok(());
    }

    // Check if compose file has updates available
    if let Some(new_services) = docker::check_compose_updates(&dir)? {
        println!(
            "{} New services available: {}",
            "!".yellow(),
            new_services.join(", ").cyan()
        );

        let update_compose = Confirm::new("Update docker-compose.yml to include new services?")
            .with_default(true)
            .with_help_message("Your .env will be preserved. Old compose backed up to .bak")
            .with_render_config(render_config())
            .prompt()?;

        if update_compose {
            docker::refresh_compose(&dir)?;
            println!("  {} docker-compose.yml updated", "✓".green());
        }
        println!();
    }

    docker::upgrade_docker_stack(&dir)?;

    println!();
    println!(
        "{} Self-hosted stack upgraded to latest version.",
        "✓".green()
    );

    Ok(())
}

fn manage_license() -> Result<()> {
    banner::print_banner();

    let config = user_config::UserConfig::load().unwrap_or_default();

    if config.endpoint.as_deref() != Some(LOCAL_ENDPOINT) {
        println!(
            "{} License management is only for self-hosted installations.",
            "!".yellow()
        );
        println!("  Cloud users don't need a license key.");
        return Ok(());
    }

    println!("{}", "Current license".bold());
    if let Some(ref key) = config.license_key {
        match license::validate_license_format(key) {
            Ok(info) => {
                println!("  Customer: {}", info.customer.cyan());
                println!("  Days remaining: {}", info.days_remaining);
                println!();
            }
            Err(e) => {
                println!("  {} {}", "!".yellow(), e);
                println!();
            }
        }
    } else {
        println!("  {} No license key configured.", "-".dimmed());
        println!();
    }

    let update = Confirm::new("Update license key?")
        .with_default(false)
        .with_render_config(render_config())
        .prompt()?;

    if !update {
        return Ok(());
    }

    let mut prompt = Text::new("License key")
        .with_help_message("Starts with ENGR_ - get yours at engrammic.ai/self-hosted")
        .with_render_config(render_config());

    if let Some(ref key) = config.license_key {
        prompt = prompt.with_default(key);
    }

    let new_key = prompt.prompt()?;

    println!("{}", "Validating license".bold());
    match license::validate_license_format(&new_key) {
        Ok(info) => {
            println!(
                "  {} Valid - customer: {}, {} days remaining",
                "✓".green(),
                info.customer.cyan(),
                info.days_remaining
            );
        }
        Err(e) => {
            println!("  {} {}", "✗".red(), e);
            return Ok(());
        }
    }
    println!();

    let dir = user_config::UserConfig::dir();
    docker::update_license_key(&dir, &new_key)?;

    let new_config = user_config::UserConfig {
        endpoint: config.endpoint,
        license_key: Some(new_key),
    };
    new_config.save()?;

    println!(
        "{} License key updated. Restart Docker services to apply.",
        "✓".green()
    );
    println!(
        "  Run: {}",
        format!("docker compose -f {}/docker-compose.yml restart", dir.display()).cyan()
    );

    Ok(())
}
