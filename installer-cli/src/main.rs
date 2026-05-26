mod banner;
mod cli;
mod config;
mod doctor;
mod docker;
mod license;
mod scale;
mod skills;
mod tools;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use inquire::{
    ui::{Attributes, Color, RenderConfig, StyleSheet, Styled},
    Confirm, MultiSelect, Text,
};

use cli::{Cli, Commands};
use tools::{SkillDest, Tool, ENDPOINT};

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
        Commands::Install => install(cli.yes, cli.tool.as_deref()),
        Commands::Update => update(cli.yes, cli.tool.as_deref()),
        Commands::Uninstall => uninstall(cli.yes, cli.tool.as_deref()),
        Commands::Status => status(),
        Commands::Docker => install_docker(),
        Commands::Scale => scale::show_status(),
        Commands::Doctor => doctor::run_diagnostics(),
    }
}

fn install(yes: bool, tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(yes, tool_id)?;
    if tools.is_empty() {
        println!("{} No harness selected.", "!".yellow());
        return Ok(());
    }

    println!("{}", "Writing MCP config".bold());
    for tool in &tools {
        config::install(&tool.config_path, ENDPOINT)?;
        println!(
            "  {} {}  {}",
            "✓".green(),
            tool.name,
            tool.config_path.display().to_string().dimmed()
        );
    }
    println!();

    install_skills_step(yes)?;

    println!();
    println!(
        "Done. Tools available: {}",
        "remember, recall, learn, believe, trace, link".dimmed()
    );
    Ok(())
}

fn update(yes: bool, tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let tools = select_tools(yes, tool_id)?;
    for tool in &tools {
        if config::is_installed(&tool.config_path, ENDPOINT) {
            config::install(&tool.config_path, ENDPOINT)?;
            println!("{} Updated engrammic in {}", "✓".green(), tool.name);
        } else {
            println!("{} Not installed for {}", "!".yellow(), tool.name);
        }
    }

    // Refresh skills in any destination that already has them.
    let dests_with_skills: Vec<std::path::PathBuf> = SkillDest::all()
        .into_iter()
        .filter(|d| skills::count_skills(&d.path) > 0)
        .map(|d| d.path)
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
        let removed = skills::remove_skills(&dest.path)?;
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

fn status() -> Result<()> {
    banner::print_banner();

    println!("{}", "Harnesses".bold());
    let mut any_installed = false;
    for tool in Tool::all() {
        let installed = config::is_installed(&tool.config_path, ENDPOINT);
        let label = if installed {
            any_installed = true;
            format!("{:<18}", "✓ installed").green()
        } else if tool.config_path.parent().map(|p| p.exists()).unwrap_or(false) {
            format!("{:<18}", "- not configured").dimmed()
        } else {
            format!("{:<18}", "- not detected").dimmed()
        };
        println!("  {} {}", label, tool.name);
    }

    println!();
    println!("{}", "Skills".bold());
    for dest in SkillDest::all() {
        let count = skills::count_skills(&dest.path);
        let label = if count > 0 {
            format!("{:<18}", format!("✓ {} skills", count)).green()
        } else {
            format!("{:<18}", "- none").dimmed()
        };
        println!("  {} {}", label, dest.name);
    }

    if !any_installed {
        println!();
        println!("Run {} to install", "engrammic-install".cyan());
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
    let license_key = Text::new("License key")
        .with_help_message("Starts with ENGR_ — get yours at engrammic.ai/self-hosted")
        .with_render_config(render_config())
        .prompt()?;

    // Validate format client-side (full validation is server-side).
    println!("{}", "Validating license".bold());
    match license::validate_license_format(&license_key) {
        Ok(info) => {
            println!(
                "  {} Valid — customer: {}, {} days remaining",
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
    let install_dir = Text::new("Install directory")
        .with_default("./engrammic")
        .with_help_message("Compose file and .env will be written here")
        .with_render_config(render_config())
        .prompt()?;

    let dir = std::path::Path::new(&install_dir);

    // Write compose bundle.
    println!("{}", "Writing compose bundle".bold());
    docker::write_compose_bundle(dir, &license_key)?;
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
        "http://localhost:8000/mcp".cyan()
    );
    println!();
    println!(
        "Configure your harness to use {} as the MCP endpoint.",
        "http://localhost:8000/mcp".cyan()
    );

    Ok(())
}

fn install_skills_step(yes: bool) -> Result<()> {
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
        let options: Vec<&str> = all_dests.iter().map(|d| d.name).collect();

        let picked = MultiSelect::new("Install skills to", options)
            .with_help_message("↑↓ move · space toggle · enter confirm")
            .with_render_config(render_config())
            .prompt()?;

        all_dests
            .iter()
            .filter(|d| picked.contains(&d.name))
            .collect()
    };

    if chosen.is_empty() {
        println!("  {} No skill destination selected.", "-".dimmed());
        return Ok(());
    }

    let paths: Vec<std::path::PathBuf> = chosen.iter().map(|d| d.path.clone()).collect();
    let results = skills::install_skills(&paths)?;

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
