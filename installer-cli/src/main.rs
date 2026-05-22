mod banner;
mod cli;
mod config;
mod skills;
mod tools;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect};

use cli::{Cli, Commands};
use tools::{SkillDest, Tool, ENDPOINT};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Install) {
        Commands::Install => install(cli.yes, cli.tool.as_deref()),
        Commands::Update => update(cli.tool.as_deref()),
        Commands::Uninstall => uninstall(cli.tool.as_deref()),
        Commands::Status => status(),
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

fn update(tool_id: Option<&str>) -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Updater".bold());
    println!();

    // Temporary bridge: select_tools returns Vec<Tool>; use first element.
    // Task 10 will rewrite update() properly.
    let tools = select_tools(false, tool_id)?;
    let tool = match tools.into_iter().next() {
        Some(t) => t,
        None => return Ok(()),
    };

    if !config::is_installed(&tool.config_path, ENDPOINT) {
        println!("{} Engrammic not installed for {}", "!".yellow(), tool.name);
        return Ok(());
    }

    config::install(&tool.config_path, ENDPOINT)?;
    println!("{} Updated engrammic in {}", "✓".green(), tool.config_path.display());

    Ok(())
}

fn uninstall(tool_id: Option<&str>) -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Uninstaller".bold());
    println!();

    // Temporary bridge: select_tools returns Vec<Tool>; use first element.
    // Task 10 will rewrite uninstall() properly.
    let tools = select_tools(false, tool_id)?;
    let tool = match tools.into_iter().next() {
        Some(t) => t,
        None => return Ok(()),
    };

    config::uninstall(&tool.config_path)?;
    println!("{} Removed engrammic from {}", "✓".green(), tool.config_path.display());

    Ok(())
}

fn status() -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Status".bold());
    println!();

    let tools = Tool::all();
    let mut any_installed = false;

    for tool in tools {
        let installed = config::is_installed(&tool.config_path, ENDPOINT);
        let status = if installed {
            any_installed = true;
            "✓ installed".green()
        } else if tool.config_path.parent().map(|p| p.exists()).unwrap_or(false) {
            "- not configured".dimmed()
        } else {
            "- not detected".dimmed()
        };

        println!("  {} {}", status, tool.name);
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
    let items: Vec<&str> = all_tools.iter().map(|t| t.name).collect();
    let detected_ids: Vec<&str> = detected.iter().map(|t| t.id).collect();
    let defaults: Vec<bool> = all_tools
        .iter()
        .map(|t| detected_ids.contains(&t.id))
        .collect();

    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select harnesses to configure (space toggles, enter confirms)")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    Ok(selection.into_iter().map(|i| all_tools[i].clone()).collect())
}

fn install_skills_step(yes: bool) -> Result<()> {
    let proceed = if yes {
        true
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Also install 21 Engrammic skills?")
            .default(true)
            .interact()?
    };

    if !proceed {
        println!("  {} Skipped skills.", "-".dimmed());
        return Ok(());
    }

    let all_dests = SkillDest::all();
    let chosen: Vec<&SkillDest> = if yes {
        all_dests.iter().filter(|d| d.default).collect()
    } else {
        let items: Vec<&str> = all_dests.iter().map(|d| d.name).collect();
        let defaults: Vec<bool> = all_dests.iter().map(|d| d.default).collect();
        let picked = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Install skills to (space toggles, enter confirms)")
            .items(&items)
            .defaults(&defaults)
            .interact()?;
        picked.into_iter().map(|i| &all_dests[i]).collect()
    };

    if chosen.is_empty() {
        println!("  {} No skill destination selected.", "-".dimmed());
        return Ok(());
    }

    let paths: Vec<std::path::PathBuf> =
        chosen.iter().map(|d| d.path.clone()).collect();
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
